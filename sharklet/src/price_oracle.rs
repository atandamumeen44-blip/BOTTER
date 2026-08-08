// src/price_oracle.rs
//
//  ██████╗ ██████╗ ██╗ ██████╗███████╗     ██████╗ ██████╗  █████╗  ██████╗██╗     ███████╗
//  ██╔══██╗██╔══██╗██║██╔════╝██╔════╝    ██╔═══██╗██╔══██╗██╔══██╗██╔════╝██║     ██╔════╝
//  ██████╔╝██████╔╝██║██║     █████╗      ██║   ██║██████╔╝███████║██║     ██║     █████╗
//  ██╔═══╝ ██╔══██╗██║██║     ██╔══╝      ██║   ██║██╔══██╗██╔══██║██║     ██║     ██╔══╝
//  ██║     ██║  ██║██║╚██████╗███████╗    ╚██████╔╝██║  ██║██║  ██║╚██████╗███████╗███████╗
//  ╚═╝     ╚═╝  ╚═╝╚═╝ ╚═════╝╚══════╝     ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚══════╝╚══════╝
//
//  Chainlink price feed with caching and fallback.
//  Reads the feed address from config/addresses.toml (or uses a default).
//  All errors are logged via tracing, never silenced.

use ethers::prelude::*;
use ethers::contract::abigen;
use std::sync::Arc;
use std::cell::RefCell;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn, debug, error};

abigen!(
    IChainlinkAggregator,
    r#"[
        function latestRoundData() external view returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)
        function decimals() external view returns (uint8)
    ]"#
);

/// Default Chainlink MATIC/USD feed on Polygon mainnet.
pub const MATIC_USD_FEED: &str = "0xAB594600376Ec9fD91F8e885dADF0CE036862dE0";

pub struct PriceOracle<M: Middleware> {
    feed: IChainlinkAggregator<M>,
    /// Cached (price, timestamp_secs). RefCell because we only ever use this
    /// in a single‑threaded async context (the main loop).
    cache: RefCell<Option<(f64, u64)>>,
    cache_ttl_secs: u64,
}

impl<M: Middleware + 'static> PriceOracle<M> {
    /// Build an oracle from a concrete feed address.
    pub fn new(provider: Arc<M>, feed_address: Address) -> Self {
        info!("Price oracle initialised, feed = {feed_address:?}");
        PriceOracle {
            feed: IChainlinkAggregator::new(feed_address, provider),
            cache: RefCell::new(None),
            cache_ttl_secs: 5,   // avoid hitting the RPC on every tick
        }
    }

    /// Build an oracle by reading the feed address from config/addresses.toml
    /// for the given `chain_id`. Falls back to `MATIC_USD_FEED` if config is missing.
    pub fn from_chain_config(provider: Arc<M>, chain_id: u64) -> Self {
        let addr = load_feed_for_chain(chain_id);
        Self::new(provider, addr)
    }

    /// Returns the live price in USD.
    ///
    /// Uses a short in‑memory cache to avoid calling the RPC more than
    /// once every few seconds. If the on‑chain read fails, or the latest
    /// answer is older than `max_staleness_secs`, `fallback` is returned.
    pub async fn price_usd(&self, fallback: f64, max_staleness_secs: u64) -> f64 {
        let now = now_secs();

        // Serve from cache if still fresh
        if let Some((price, ts)) = *self.cache.borrow() {
            if now.saturating_sub(ts) < self.cache_ttl_secs {
                debug!("Using cached price: ${price:.4}");
                return price;
            }
        }

        match self.feed.latest_round_data().call().await {
            Ok((_, answer, _, updated_at, _)) => {
                let decimals = self.feed.decimals().call().await.unwrap_or(8);
                let age = now.saturating_sub(updated_at.as_u64());

                if age > max_staleness_secs {
                    warn!(
                        "Chainlink data is {age}s old (max {max_staleness_secs}s) — using fallback ${fallback:.4}"
                    );
                    return fallback;
                }

                let price = answer.as_u128() as f64 / 10f64.powi(decimals as i32);
                self.cache.replace(Some((price, now)));
                debug!("Fresh price from Chainlink: ${price:.4}");
                price
            }
            Err(e) => {
                error!("Chainlink read failed: {e:?} — using fallback ${fallback:.4}");
                fallback
            }
        }
    }

    /// Clear the cache so the next `price_usd` call forces a fresh fetch.
    pub fn invalidate_cache(&self) {
        self.cache.replace(None);
    }
}

// ── Config helper ─────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

use serde::Deserialize;
use std::fs;

fn load_feed_for_chain(chain_id: u64) -> Address {
    #[derive(Debug, Deserialize)]
    struct AddressesConfig {
        amoy: Option<NetworkConfig>,
        polygon: Option<NetworkConfig>,
        arbitrum: Option<NetworkConfig>,
        base: Option<NetworkConfig>,
    }
    #[derive(Debug, Deserialize)]
    struct NetworkConfig {
        chainlink_feed: Option<String>,
    }

    let path = "config/addresses.toml";
    let cfg_str = fs::read_to_string(path).unwrap_or_default();
    let cfg: AddressesConfig = toml::from_str(&cfg_str).unwrap_or(AddressesConfig {
        amoy: None,
        polygon: None,
        arbitrum: None,
        base: None,
    });

    let net = match chain_id {
        80002 => cfg.amoy,
        137   => cfg.polygon,
        42161 => cfg.arbitrum,
        8453  => cfg.base,
        _     => None,
    };

    if let Some(Some(feed_str)) = net.map(|n| n.chainlink_feed) {
        if let Ok(addr) = feed_str.parse::<Address>() {
            info!("Using feed from config for chain {chain_id}: {addr:?}");
            return addr;
        }
    }

    warn!("No feed in config for chain {chain_id}, using default MATIC feed");
    MATIC_USD_FEED.parse().expect("default feed address invalid")
}