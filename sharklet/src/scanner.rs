// src/scanner.rs
//
// High-frequency arbitrage scanner. Fetches pool reserves, caches them
// briefly, and validates the best spreads against actual router quotes
// (round-trip) before emitting an Opportunity.

use ethers::prelude::*;
use ethers::contract::abigen;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout as tokio_timeout;
use tracing::{debug, warn, info};

abigen!(
    IUniswapV2Pair,
    r#"[
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
        function token0() external view returns (address)
        function token1() external view returns (address)
    ]"#
);

abigen!(
    IUniswapV2RouterQuote,
    r#"[
        function getAmountsOut(uint amountIn, address[] calldata path) external view returns (uint[] memory amounts)
    ]"#
);

// ── Data structures ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PoolQuote {
    pub dex_name: String,
    pub pool: Address,
    pub reserve_token0: u128,
    pub reserve_token1: u128,
    pub token0: Address,
    pub token1: Address,
}

#[derive(Debug, Clone)]
pub struct TrackedPair {
    pub label: String,
    pub pools: Vec<(String, Address)>, // (dex_name, pool_address)
    pub token0: Address,
    pub token1: Address,
    pub routers: Vec<(String, Address)>, // (dex_name, router_address)
}

#[derive(Debug, Clone)]
pub struct Opportunity {
    pub label: String,
    pub buy_dex: String,
    pub sell_dex: String,
    pub buy_price: f64,
    pub sell_price: f64,
    pub spread_pct: f64,
    pub buy_pool_depth: f64,
    pub sell_pool_depth: f64,
    pub token0: Address,
    pub token1: Address,
    pub block_number: Option<u64>,
}

pub struct Scanner<M: Middleware> {
    provider: Arc<M>,
    pairs: Vec<TrackedPair>,
    token0_decimals: u32,
    token1_decimals: u32,
    cache: Vec<(String, Address, PoolQuote, Instant)>,
    cache_ttl: Duration,
    min_spread_pct: f64,
}

impl<M: Middleware + 'static> Scanner<M> {
    pub fn new(
        provider: Arc<M>,
        pairs: Vec<TrackedPair>,
        token0_decimals: u32,
        token1_decimals: u32,
    ) -> Self {
        let min_spread = std::env::var("MIN_SPREAD_PCT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.05);

        Self {
            provider,
            pairs,
            token0_decimals,
            token1_decimals,
            cache: Vec::new(),
            cache_ttl: Duration::from_secs(2),
            min_spread_pct: min_spread,
        }
    }

    async fn fetch_pool(&mut self, dex_name: &str, pool_addr: Address) -> Option<PoolQuote> {
        let now = Instant::now();

        if let Some((_, _, quote, fetched_at)) = self
            .cache
            .iter()
            .find(|(d, a, _, _)| d == dex_name && *a == pool_addr)
        {
            if now - *fetched_at < self.cache_ttl {
                debug!("cache hit for {} ({})", dex_name, pool_addr);
                return Some(quote.clone());
            }
        }

        // ✅ FIXED: Increased timeout from 2s to 5s to reduce RPC timeouts
        let timeout_dur = Duration::from_secs(5);
        let pool = IUniswapV2Pair::new(pool_addr, self.provider.clone());

        let fetch_future = async {
            let (reserve0, reserve1, _ts) = pool.get_reserves().call().await?;
            let token0 = pool.token_0().call().await?;
            let token1 = pool.token_1().call().await?;
            Ok::<_, ethers::contract::ContractError<M>>(PoolQuote {
                dex_name: dex_name.to_string(),
                pool: pool_addr,
                reserve_token0: reserve0,
                reserve_token1: reserve1,
                token0,
                token1,
            })
        };

        match tokio_timeout(timeout_dur, fetch_future).await {
            Ok(Ok(quote)) => {
                self.cache.retain(|(d, a, _, _)| !(d == dex_name && *a == pool_addr));
                self.cache.push((dex_name.to_string(), pool_addr, quote.clone(), Instant::now()));
                Some(quote)
            }
            Ok(Err(e)) => {
                warn!("Failed to fetch {} on {}: {:?}", dex_name, pool_addr, e);
                None
            }
            Err(_) => {
                warn!("Timeout fetching {} on {}", dex_name, pool_addr);
                None
            }
        }
    }

    /// Fetch every pool for a pair, one at a time (sequential, not
    /// concurrent — fetch_pool needs &mut self so we can't hold multiple
    /// in-flight futures against it at once).
    async fn fetch_all_pools(&mut self, pair: &TrackedPair) -> Vec<PoolQuote> {
        let mut quotes = Vec::new();
        for (dex_name, addr) in &pair.pools {
            if let Some(q) = self.fetch_pool(dex_name, *addr).await {
                quotes.push(q);
            }
        }
        quotes
    }

    fn implied_price(&self, q: &PoolQuote) -> f64 {
        let r0 = q.reserve_token0 as f64 / 10f64.powi(self.token0_decimals as i32);
        let r1 = q.reserve_token1 as f64 / 10f64.powi(self.token1_decimals as i32);
        if r0 == 0.0 {
            return 0.0;
        }
        r1 / r0
    }

    fn router_for_dex(&self, dex_name: &str) -> Option<Address> {
        self.pairs
            .iter()
            .find_map(|p| p.routers.iter().find(|(name, _)| name == dex_name).map(|(_, addr)| *addr))
    }

    async fn quote(&self, router: Address, amount_in: u128, path: Vec<Address>) -> Option<u128> {
        let contract = IUniswapV2RouterQuote::new(router, self.provider.clone());
        match contract.get_amounts_out(U256::from(amount_in), path).call().await {
            Ok(amounts) => amounts.last().copied().map(|a| a.as_u128()),
            Err(_) => None,
        }
    }

    /// Scan all pairs, filter by spread, validate top candidates with routers.
    pub async fn scan(&mut self) -> Vec<Opportunity> {
        let block_number = self.provider.get_block_number().await.ok().map(|n| n.as_u64());
        let mut raw = Vec::new();

        let pairs = self.pairs.clone();
        for pair in &pairs {
            if pair.pools.len() < 2 {
                continue;
            }

            let quotes = self.fetch_all_pools(pair).await;
            if quotes.len() < 2 {
                continue;
            }

            for i in 0..quotes.len() {
                for j in (i + 1)..quotes.len() {
                    let p_i = self.implied_price(&quotes[i]);
                    let p_j = self.implied_price(&quotes[j]);
                    if p_i <= 0.0 || p_j <= 0.0 {
                        continue;
                    }
                    let (cheap, expensive, cheap_price, exp_price) = if p_i < p_j {
                        (&quotes[i], &quotes[j], p_i, p_j)
                    } else {
                        (&quotes[j], &quotes[i], p_j, p_i)
                    };
                    let spread_pct = (exp_price - cheap_price) / cheap_price * 100.0;
                    if spread_pct < self.min_spread_pct {
                        continue;
                    }
                    raw.push(Opportunity {
                        label: pair.label.clone(),
                        buy_dex: cheap.dex_name.clone(),
                        sell_dex: expensive.dex_name.clone(),
                        buy_price: cheap_price,
                        sell_price: exp_price,
                        spread_pct,
                        buy_pool_depth: cheap.reserve_token0 as f64 / 10f64.powi(self.token0_decimals as i32),
                        sell_pool_depth: expensive.reserve_token0 as f64 / 10f64.powi(self.token0_decimals as i32),
                        token0: pair.token0,
                        token1: pair.token1,
                        block_number,
                    });
                }
            }
        }

        raw.sort_by(|a, b| b.spread_pct.partial_cmp(&a.spread_pct).unwrap());
        let top: Vec<Opportunity> = raw.into_iter().take(5).collect();
        let mut validated = Vec::new();

        for opp in top {
            let buy_router = self.router_for_dex(&opp.buy_dex);
            let sell_router = self.router_for_dex(&opp.sell_dex);
            let (Some(buy_router), Some(sell_router)) = (buy_router, sell_router) else {
                debug!("Missing router for {} / {}, dropping", opp.buy_dex, opp.sell_dex);
                continue;
            };

            let probe = 1_000 * 10u128.pow(self.token0_decimals);
            let path_out = vec![opp.token0, opp.token1];
            let buy_amount = match self.quote(buy_router, probe, path_out).await {
                Some(a) => a,
                None => continue,
            };

            let path_back = vec![opp.token1, opp.token0];
            let sell_amount = match self.quote(sell_router, buy_amount, path_back).await {
                Some(a) => a,
                None => continue,
            };

            if sell_amount <= probe {
                debug!("Round-trip not profitable for {} — dropping", opp.label);
                continue;
            }

            validated.push(opp);
        }

        validated
    }

    /// Re-check a single pair by label. Used for the "reswap" loop: after a
    /// trade executes, quickly confirm the same opportunity is still there
    /// before firing again, instead of blindly repeating on a stale spread.
    pub async fn quick_check(&mut self, label: &str) -> Option<Opportunity> {
        let block_number = self.provider.get_block_number().await.ok().map(|n| n.as_u64());
        let pair = self.pairs.iter().find(|p| p.label == label)?.clone();
        if pair.pools.len() < 2 {
            return None;
        }

        let quotes = self.fetch_all_pools(&pair).await;
        if quotes.len() < 2 {
            return None;
        }

        let mut best: Option<Opportunity> = None;
        for i in 0..quotes.len() {
            for j in (i + 1)..quotes.len() {
                let p_i = self.implied_price(&quotes[i]);
                let p_j = self.implied_price(&quotes[j]);
                if p_i <= 0.0 || p_j <= 0.0 {
                    continue;
                }
                let (cheap, expensive, cheap_price, exp_price) = if p_i < p_j {
                    (&quotes[i], &quotes[j], p_i, p_j)
                } else {
                    (&quotes[j], &quotes[i], p_j, p_i)
                };
                let spread_pct = (exp_price - cheap_price) / cheap_price * 100.0;
                if spread_pct < self.min_spread_pct {
                    continue;
                }
                let candidate = Opportunity {
                    label: pair.label.clone(),
                    buy_dex: cheap.dex_name.clone(),
                    sell_dex: expensive.dex_name.clone(),
                    buy_price: cheap_price,
                    sell_price: exp_price,
                    spread_pct,
                    buy_pool_depth: cheap.reserve_token0 as f64 / 10f64.powi(self.token0_decimals as i32),
                    sell_pool_depth: expensive.reserve_token0 as f64 / 10f64.powi(self.token0_decimals as i32),
                    token0: pair.token0,
                    token1: pair.token1,
                    block_number,
                };
                if best.as_ref().map_or(true, |b| candidate.spread_pct > b.spread_pct) {
                    best = Some(candidate);
                }
            }
        }
        best
    }
}
