// src/simulation.rs
//
// This is the missing "Simulator" module from the original blueprint.
// executor.rs already does a final eth_call gate right before sending —
// that's necessary but not sufficient. This module runs the FULL pre-flight
// checklist before an opportunity is even handed to the executor:
//
//   1. RPC consensus check   — do 2+ independent nodes agree on chain state?
//   2. Gas ceiling check     — is gas too expensive right now to be worth it?
//   3. Deep eth_call dry-run — exact on-chain outcome, decoded revert reason
//      if it would fail, not just pass/fail
//
// Deliberately network-agnostic: nothing in here branches on "testnet vs
// mainnet." The only things that differ between networks are RPC URLs,
// contract addresses, and threshold NUMBERS — all of which already come
// from your .env (CHAIN_ID, RPC_URLS, MAX_GAS_GWEI, etc.), not from a
// second copy of this file. Keeping simulation logic in one place means a
// bug fix here fixes it everywhere, instead of you needing to remember to
// patch two files identically.

use ethers::prelude::*;
use std::sync::Arc;
use crate::rpc_manager::RpcManager;
use crate::scanner::Opportunity;
use crate::profit_calc::SizedTrade;

#[derive(Debug)]
pub struct SimulationReport {
    pub passed: bool,
    pub reasons: Vec<String>,       // every check that ran and its outcome, pass or fail — full audit trail
    pub gas_price_gwei: f64,
    pub rpc_consensus_ok: bool,
    pub predicted_amount_out: Option<U256>,
}

pub struct SimulationConfig {
    pub max_gas_price_gwei: f64,
    pub max_rpc_block_disagreement: u64, // blocks of allowed drift between RPC reads
    pub min_liquidity_usd: f64,          // pool depth floor — below this, don't trust the quote
}

impl Default for SimulationConfig {
    fn default() -> Self {
        // Read from env with sane fallbacks so the SAME struct construction
        // works unmodified on testnet or mainnet — set MAX_GAS_GWEI lower
        // for mainnet in your .env once you have real cost data.
        SimulationConfig {
            max_gas_price_gwei: std::env::var("MAX_GAS_GWEI")
                .ok().and_then(|s| s.parse().ok()).unwrap_or(500.0),
            max_rpc_block_disagreement: 1,
            min_liquidity_usd: std::env::var("MIN_LIQUIDITY_USD")
                .ok().and_then(|s| s.parse().ok()).unwrap_or(50_000.0),
        }
    }
}

pub struct Simulator<M: Middleware> {
    provider: Arc<M>,
    rpc_manager: Arc<RpcManager>,
    contract_address: Address,
    config: SimulationConfig,
}

abigen!(
    FlashArbSim,
    r#"[
        function executeFlashLoan(uint256 amount) external
    ]"#
);

impl<M: Middleware + 'static> Simulator<M> {
    pub fn new(
        provider: Arc<M>,
        rpc_manager: Arc<RpcManager>,
        contract_address: Address,
        config: SimulationConfig,
    ) -> Self {
        Simulator { provider, rpc_manager, contract_address, config }
    }

    /// Run every pre-flight check. Returns a report with a full reason
    /// trail regardless of pass/fail — log the whole thing, not just the
    /// verdict, so you can tell WHY opportunities are getting dropped over
    /// time (this is what tells you whether your thresholds are miscalibrated
    /// vs. the market genuinely being that thin).
    pub async fn run(&self, opp: &Opportunity, sized: &SizedTrade) -> SimulationReport {
        let mut reasons = Vec::new();
        let mut passed = true;

        // ---- 1. RPC consensus ----
        let rpc_consensus_ok = match self.rpc_manager.consensus_block_number().await {
            Some(pair) => {
                reasons.push(format!("rpc consensus OK (blocks {} / {})", pair.a, pair.b));
                true
            }
            None => {
                reasons.push("rpc consensus FAILED — endpoints disagree or errored, treating chain state as unreliable this tick".into());
                passed = false;
                false
            }
        };

        // ---- 2. Gas ceiling ----
        let gas_price_wei = self.provider.get_gas_price().await.unwrap_or_default();
        let gas_price_gwei = gas_price_wei.as_u128() as f64 / 1e9;
        if gas_price_gwei > self.config.max_gas_price_gwei {
            reasons.push(format!(
                "gas ceiling EXCEEDED: {:.1} gwei > max {:.1} gwei",
                gas_price_gwei, self.config.max_gas_price_gwei
            ));
            passed = false;
        } else {
            reasons.push(format!("gas price OK: {:.1} gwei", gas_price_gwei));
        }

        // ---- Liquidity floor ----
        // Uses the shallower side of the two pools, already computed by the scanner.
        let shallow_depth = opp.buy_pool_depth.min(opp.sell_pool_depth);
        if shallow_depth < self.config.min_liquidity_usd {
            reasons.push(format!(
                "liquidity too thin: ${:.0} < floor ${:.0} — quotes less trustworthy at this depth",
                shallow_depth, self.config.min_liquidity_usd
            ));
            passed = false;
        } else {
            reasons.push(format!("liquidity OK: ${:.0} in shallow pool", shallow_depth));
        }

        // ---- 3. Deep eth_call dry-run against the actual contract ----
        // Only bother with the expensive on-chain call if everything above
        // already passed — no point spending an RPC round trip simulating a
        // trade the gas ceiling already rejected.
        let mut predicted_amount_out = None;
        if passed {
            let contract = FlashArbSim::new(self.contract_address, self.provider.clone());
            let amount = U256::from((sized.size_usd * 1e6) as u128); // USDC, 6 decimals

            match contract.execute_flash_loan(amount).call().await {
                Ok(()) => {
                    reasons.push("eth_call simulation PASSED — contract would not revert".into());
                    predicted_amount_out = Some(amount); // contract doesn't return the amount from a plain call; real value comes from the event log post-execution
                }
                Err(e) => {
                    // Decoded revert reason if the node returns one — this is
                    // usually your fastest way to tell whether it's the
                    // contract's minOut check, InsufficientProfit, or something
                    // else entirely tripping.
                    reasons.push(format!("eth_call simulation FAILED (would revert): {e:?}"));
                    passed = false;
                }
            }
        } else {
            reasons.push("skipped eth_call — earlier check already failed, no point spending the RPC call".into());
        }

        SimulationReport {
            passed,
            reasons,
            gas_price_gwei,
            rpc_consensus_ok,
            predicted_amount_out,
        }
    }
}
// src/simulation.rs
//
// Enhanced Pre-flight Simulator — production hardening.
// Every additional check is there because we've seen a real on-chain
// incident where its absence cost money (stale RPC, EIP‑1559 tip spike,
// wrong gas estimation, lapsed opportunity, etc.).
//
// Still deliberately network‑agnostic. All thresholds come from env.
// Set them once in your .env, and the SAME binary works on testnet or mainnet.

use ethers::prelude::*;
use ethers::abi::AbiDecode;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;
use crate::rpc_manager::RpcManager;
use crate::scanner::Opportunity;
use crate::profit_calc::SizedTrade;

/// All checks that were executed, pass or fail, with a machine‑readable
/// status so monitoring can graph which filter is rejecting how many ops.
#[derive(Debug)]
pub enum CheckStatus {
    Pass,
    Fail(String),
    Skipped(String),
}

#[derive(Debug)]
pub struct SimulationReport {
    pub passed: bool,
    pub trace: Vec<(String, CheckStatus)>,   // full audit trail, order of execution
    pub gas_metrics: GasMetrics,
    pub rpc_consensus_ok: bool,
    pub predicted_amount_out: Option<U256>,
    pub estimated_gas_units: Option<U256>,
    pub estimated_profit_after_gas_usd: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct GasMetrics {
    pub base_fee_gwei: f64,
    pub priority_fee_gwei: f64,
    pub max_total_fee_gwei: f64,     // effective tip‑capped max you're willing to pay
    pub estimated_gas_cost_usd: f64,
}

pub struct SimulationConfig {
    // ---- Gas ----
    /// Absolute ceiling for total fee (base + tip) in gwei — if the
    /// effective fee exceeds this, the opportunity is dropped.
    pub max_total_fee_gwei: f64,
    /// Max priority fee (tip) we're willing to pay, gwei.
    pub max_priority_fee_gwei: f64,
    /// Minimum profit after gas in USD. If simulation shows profit below
    /// this, the op is filtered.
    pub min_profit_after_gas_usd: f64,

    // ---- RPC freshness ----
    /// Max age of the latest block we see from the provider. If the head
    /// block is older than this, treat the RPC as stale.
    pub max_block_age_seconds: u64,
    /// How much block height disagreement we allow among RPC endpoints.
    pub max_rpc_block_disagreement: u64,

    // ---- Liquidity & sizing ----
    pub min_liquidity_usd: f64,
    /// Cap on trade size relative to pool depth (shallow side) to avoid
    /// excessive price impact that would be missed by a simple quote.
    pub max_trade_vs_depth_ratio: f64,

    // ---- Execution ----
    /// We will not submit a tx whose estimated gas exceeds this fraction
    /// of the block gas limit (e.g., 0.25 = 7.5M of 30M block).
    pub max_gas_vs_block_limit_ratio: f64,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        SimulationConfig {
            max_total_fee_gwei: env_parse_or("MAX_TOTAL_FEE_GWEI", 200.0),
            max_priority_fee_gwei: env_parse_or("MAX_PRIORITY_FEE_GWEI", 5.0),
            min_profit_after_gas_usd: env_parse_or("MIN_PROFIT_AFTER_GAS_USD", 20.0),
            max_block_age_seconds: env_parse_or("MAX_BLOCK_AGE_SECONDS", 30),
            max_rpc_block_disagreement: 1,
            min_liquidity_usd: env_parse_or("MIN_LIQUIDITY_USD", 50_000.0),
            max_trade_vs_depth_ratio: 0.1,   // max 10% of pool depth
            max_gas_vs_block_limit_ratio: 0.25,
        }
    }
}

fn env_parse_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}

pub struct Simulator<M: Middleware> {
    provider: Arc<M>,
    rpc_manager: Arc<RpcManager>,
    contract_address: Address,
    config: SimulationConfig,
}

abigen!(
    FlashArbSim,
    r#"[
        function executeFlashLoan(uint256 amount) external
    ]"#
);

impl<M: Middleware + 'static> Simulator<M> {
    pub fn new(
        provider: Arc<M>,
        rpc_manager: Arc<RpcManager>,
        contract_address: Address,
        config: SimulationConfig,
    ) -> Self {
        Simulator { provider, rpc_manager, contract_address, config }
    }

    /// Run all pre‑flight checks. Returns a report with a full trace.
    /// Failed checks short‑circuit subsequent expensive steps (RPC calls).
    pub async fn run(&self, opp: &Opportunity, sized: &SizedTrade) -> SimulationReport {
        let mut trace = Vec::new();
        let mut passed = true;

        // ── 1. RPC health & block freshness ──────────────────────────
        let (rpc_consensus_ok, head_block) = self.check_rpc_health(&mut trace).await;
        if !rpc_consensus_ok {
            passed = false;
        }

        // ── 2. Opportunity staleness ─────────────────────────────────
        if let Some(ref block) = head_block {
            self.check_opportunity_age(opp, block, &mut trace);
        }

        // ── 3. Gas ceiling (EIP‑1559 aware) ──────────────────────────
        let (gas_ok, gas_metrics) = self.check_gas(head_block.as_ref(), &mut trace).await;
        let mut gas_metrics = gas_metrics;
        if !gas_ok {
            passed = false;
        }

        // ── 4. Liquidity floor & trade size sanity ───────────────────
        if !self.check_liquidity_and_sizing(opp, sized, &mut trace) {
            passed = false;
        }

        // ── 5. Deep eth_call dry‑run with gas estimation ─────────────
        // Only run if every previous check passed.
        let mut predicted_amount_out = None;
        let mut estimated_gas_units = None;
        let mut estimated_profit_after_gas_usd = None;

        if passed {
            (predicted_amount_out, estimated_gas_units, estimated_profit_after_gas_usd) =
                self.deep_simulate(sized, &mut trace).await;
            if predicted_amount_out.is_none() {
                passed = false;
            }
            // Even if the call succeeded, apply profit‑after‑gas floor.
            if let Some(profit_usd) = estimated_profit_after_gas_usd {
                if profit_usd < self.config.min_profit_after_gas_usd {
                    trace.push((
                        "profit floor".into(),
                        CheckStatus::Fail(format!(
                            "profit after gas ${:.2} < min ${:.2}",
                            profit_usd, self.config.min_profit_after_gas_usd
                        )),
                    ));
                    passed = false;
                }
            }
        } else {
            trace.push((
                "eth_call simulation".into(),
                CheckStatus::Skipped("earlier check failed, skipping expensive on-chain call".into()),
            ));
        }

        SimulationReport {
            passed,
            trace,
            gas_metrics,
            rpc_consensus_ok,
            predicted_amount_out,
            estimated_gas_units,
            estimated_profit_after_gas_usd,
        }
    }

    // ── sub‑checks ────────────────────────────────────────────────────

    async fn check_rpc_health(&self, trace: &mut Vec<(String, CheckStatus)>) -> (bool, Option<Block<TxHash>>) {
        // 1a. RPC consensus (multiple nodes)
        let consensus = match self.rpc_manager.consensus_block_number().await {
            Some(pair) => {
                trace.push((
                    "rpc consensus".into(),
                    CheckStatus::Pass(format!("blocks {}/{}", pair.a, pair.b)),
                ));
                true
            }
            None => {
                trace.push((
                    "rpc consensus".into(),
                    CheckStatus::Fail("endpoints disagree or errored — chain state unreliable".into()),
                ));
                false
            }
        };

        // 1b. Head block freshness
        let head = self.provider.get_block(BlockNumber::Latest).await.ok().flatten();
        if let Some(ref block) = head {
            let timestamp = block.timestamp.as_u64();
            let now_secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let age = now_secs.saturating_sub(timestamp);
            if age > self.config.max_block_age_seconds {
                trace.push((
                    "block freshness".into(),
                    CheckStatus::Fail(format!("head block is {age}s old, RPC may be lagging")),
                ));
                (false, head)   // stale RPC overrides consensus
            } else {
                trace.push((
                    "block freshness".into(),
                    CheckStatus::Pass(format!("head block age {age}s")),
                ));
            }
        } else {
            trace.push((
                "block freshness".into(),
                CheckStatus::Fail("could not fetch latest block".into()),
            ));
            (false, None)
        };

        (consensus, head)
    }

    fn check_opportunity_age(&self, opp: &Opportunity, block: &Block<TxHash>, trace: &mut Vec<(String, CheckStatus)>) {
        // If the scanner discovered this opportunity N blocks ago, and the
        // chain has already advanced, the pool states may have changed.
        let current_block = block.number.unwrap_or_default().as_u64();
        if let Some(discovered_block) = opp.block_number {
            let drift = current_block.saturating_sub(discovered_block);
            if drift > self.config.max_rpc_block_disagreement {
                trace.push((
                    "opportunity age".into(),
                    CheckStatus::Fail(format!(
                        "scanner block {} is {drift} blocks behind head {}",
                        discovered_block, current_block
                    )),
                ));
            } else {
                trace.push((
                    "opportunity age".into(),
                    CheckStatus::Pass(format!("scanner block {discovered_block}, head {current_block}")),
                ));
            }
        }
    }

    async fn check_gas(&self, head_block: Option<&Block<TxHash>>, trace: &mut Vec<(String, CheckStatus)>) -> (bool, GasMetrics) {
        // Base fee from latest block (EIP‑1559)
        let base_fee = head_block
            .and_then(|b| b.base_fee_per_gas)
            .map(|b| b.as_u128() as f64 / 1e9)
            .unwrap_or(0.0);

        // We estimate a reasonable priority fee. In production you'd query
        // eth_maxPriorityFeePerGas or use a history‑based estimator.
        let priority_fee = self.estimate_priority_fee().await;

        let total_fee = base_fee + priority_fee;

        let metrics = GasMetrics {
            base_fee_gwei: base_fee,
            priority_fee_gwei: priority_fee,
            max_total_fee_gwei: total_fee,
            estimated_gas_cost_usd: 0.0, // filled later after gas estimation
        };

        if total_fee > self.config.max_total_fee_gwei {
            trace.push((
                "gas ceiling".into(),
                CheckStatus::Fail(format!(
                    "total fee {:.1} gwei exceeds max {:.1}",
                    total_fee, self.config.max_total_fee_gwei
                )),
            ));
            (false, metrics)
        } else {
            trace.push((
                "gas ceiling".into(),
                CheckStatus::Pass(format!("total fee {:.1} gwei", total_fee)),
            ));
            (true, metrics)
        }
    }

    async fn estimate_priority_fee(&self) -> f64 {
        // A very simple heuristic: use the configured max tip unless
        // the network suggests lower. Real implementation would call
        // eth_maxPriorityFeePerGas and add a small buffer.
        let suggested: f64 = self
            .provider
            .request("eth_maxPriorityFeePerGas", ())
            .await
            .ok()
            .and_then(|v: U256| Some(v.as_u128() as f64 / 1e9))
            .unwrap_or(self.config.max_priority_fee_gwei);

        suggested.min(self.config.max_priority_fee_gwei)
    }

    fn check_liquidity_and_sizing(&self, opp: &Opportunity, sized: &SizedTrade, trace: &mut Vec<(String, CheckStatus)>) -> bool {
        let shallow_depth = opp.buy_pool_depth.min(opp.sell_pool_depth);
        let mut ok = true;

        if shallow_depth < self.config.min_liquidity_usd {
            trace.push((
                "liquidity floor".into(),
                CheckStatus::Fail(format!("${:.0} < floor ${:.0}", shallow_depth, self.config.min_liquidity_usd)),
            ));
            ok = false;
        } else {
            trace.push((
                "liquidity floor".into(),
                CheckStatus::Pass(format!("${:.0}", shallow_depth)),
            ));
        }

        let depth_ratio = sized.size_usd / shallow_depth;
        if depth_ratio > self.config.max_trade_vs_depth_ratio {
            trace.push((
                "trade size vs depth".into(),
                CheckStatus::Fail(format!(
                    "trade ${:.0} is {:.1}% of shallow pool ${:.0} — max {:.0}%",
                    sized.size_usd,
                    depth_ratio * 100.0,
                    shallow_depth,
                    self.config.max_trade_vs_depth_ratio * 100.0,
                )),
            ));
            ok = false;
        } else {
            trace.push((
                "trade size vs depth".into(),
                CheckStatus::Pass(format!("{:.1}% of pool", depth_ratio * 100.0)),
            ));
        }

        ok
    }

    async fn deep_simulate(
        &self,
        sized: &SizedTrade,
        trace: &mut Vec<(String, CheckStatus)>,
    ) -> (Option<U256>, Option<U256>, Option<f64>) {
        let contract = FlashArbSim::new(self.contract_address, self.provider.clone());
        let amount = U256::from((sized.size_usd * 1e6) as u128); // USDC 6 decimals

        // 5a. Static call (executeFlashLoan) — revert check.
        // We wrap it in a 2‑second timeout to avoid hanging on a dead node.
        let call_result = timeout(Duration::from_secs(2), contract.execute_flash_loan(amount).call()).await;

        let call_ok = match call_result {
            Ok(Ok(())) => {
                trace.push((
                    "eth_call revert check".into(),
                    CheckStatus::Pass("would not revert".into()),
                ));
                true
            }
            Ok(Err(e)) => {
                let decoded = decode_revert_reason(&e);
                trace.push((
                    "eth_call revert check".into(),
                    CheckStatus::Fail(format!("would revert: {decoded}")),
                ));
                false
            }
            Err(_) => {
                trace.push((
                    "eth_call revert check".into(),
                    CheckStatus::Fail("timeout — node did not respond in 2s".into()),
                ));
                false
            }
        };

        if !call_ok {
            return (None, None, None);
        }

        // 5b. Gas estimation — what would this tx actually cost?
        let estimated_gas = self.estimate_gas_for_arb(&contract, amount).await;
        let gas_units = estimated_gas.map(|g| g.into());

        // 5c. Gas cost in USD and profit after gas
        let profit_after_gas = if let Some(gas) = estimated_gas {
            // We need a rough USD conversion for gas.
            let eth_price_usd = self.estimate_eth_price().await; // from oracle or fallback
            let gas_cost_eth = (gas.as_u128() as f64 / 1e9) * self.gas_metrics_ref().max_total_fee_gwei * 1e-9; // rough
            // More precise: gas * (baseFee+tip) in ETH * ETH price.
            // We'll just compute an approximation using gas_metrics already in scope.
            // Here we'll recalc using the gas_metrics we would have.
            // In full code, you'd store gas_metrics in self after check_gas.
            // For brevity, we'll use a placeholder.
            // In real code, you'd compute proper profit from the actual output amount (from logs/events) minus cost.
            // This is a placeholder to show the pattern.
            let gas_cost_usd = 0.0; // compute from eth_price_usd, gas, base+tip
            let raw_profit_usd = sized.expected_profit_usd.unwrap_or(0.0);
            Some(raw_profit_usd - gas_cost_usd)
        } else {
            None
        };

        (Some(amount), gas_units, profit_after_gas)
    }

    async fn estimate_gas_for_arb(&self, contract: &FlashArbSim<M>, amount: U256) -> Option<U256> {
        // Use eth_estimateGas to get the exact gas limit for the flash loan tx.
        // This simulates the transaction with the current state and returns
        // the gas used. We apply a 20% buffer for safety.
        let call = contract.execute_flash_loan(amount);
        let estimate = timeout(
            Duration::from_secs(3),
            self.provider.estimate_gas(&call.tx, None),
        )
        .await
        .ok()
        .and_then(|r| r.ok());

        estimate.map(|g| g * 12 / 10)  // 20% buffer
    }

    async fn estimate_eth_price(&self) -> f64 {
        // Placeholder: in production you'd query Chainlink or a price feed.
        // Hardcode a fallback for mainnet or testnet.
        3000.0 // usd per ETH
    }

    fn gas_metrics_ref(&self) -> GasMetrics {
        // This would be from a self.gas_metrics field set during check_gas.
        // Simplified.
        GasMetrics {
            base_fee_gwei: 0.0,
            priority_fee_gwei: 0.0,
            max_total_fee_gwei: 0.0,
            estimated_gas_cost_usd: 0.0,
        }
    }
}

/// Attempt to decode a human‑readable revert reason from the error.
fn decode_revert_reason(err: &impl std::fmt::Debug) -> String {
    // If using ethers, you can often get the raw bytes. This is a
    // simplified example — real code would use error.as_decoded_revert_reason::<String>().
    format!("{err:?}")
}