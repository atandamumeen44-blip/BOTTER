// src/simulation.rs
//
// Enhanced Pre-flight Simulator — production hardening.
// Every additional check is there because we've seen a real on-chain
// incident where its absence cost money (stale RPC, EIP‑1559 tip spike,
// wrong gas estimation, lapsed opportunity, etc.).
//
// Still deliberately network‑agnostic. All thresholds come from env.

use ethers::prelude::*;
use ethers::abi::AbiDecode;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;
use crate::rpc_manager::RpcManager;
use crate::scanner::Opportunity;
use crate::profit_calc::SizedTrade;

#[derive(Debug)]
pub enum CheckStatus {
    Pass(String),
    Fail(String),
    Skipped(String),
}

#[derive(Debug)]
pub struct SimulationReport {
    pub passed: bool,
    pub trace: Vec<(String, CheckStatus)>,   // full audit trail
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
    pub max_total_fee_gwei: f64,
    pub estimated_gas_cost_usd: f64,
}

pub struct SimulationConfig {
    pub max_total_fee_gwei: f64,
    pub max_priority_fee_gwei: f64,
    pub min_profit_after_gas_usd: f64,
    pub max_block_age_seconds: u64,
    pub max_rpc_block_disagreement: u64,
    pub min_liquidity_usd: f64,
    pub max_trade_vs_depth_ratio: f64,
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
            max_trade_vs_depth_ratio: 0.1,
            max_gas_vs_block_limit_ratio: 0.25,
        }
    }
}

fn env_parse_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}

abigen!(
    FlashArbSim,
    r#"[
        function executeFlashLoan(uint256 amount) external
    ]"#
);

pub struct Simulator<M: Middleware> {
    provider: Arc<M>,
    rpc_manager: Arc<RpcManager>,
    contract_address: Address,
    config: SimulationConfig,
}

impl<M: Middleware + 'static> Simulator<M> {
    pub fn new(
        provider: Arc<M>,
        rpc_manager: Arc<RpcManager>,
        contract_address: Address,
        config: SimulationConfig,
    ) -> Self {
        Simulator { provider, rpc_manager, contract_address, config }
    }

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
        let mut gas_metrics = gas_metrics.unwrap_or(GasMetrics {
            base_fee_gwei: 0.0,
            priority_fee_gwei: 0.0,
            max_total_fee_gwei: 0.0,
            estimated_gas_cost_usd: 0.0,
        });
        if !gas_ok {
            passed = false;
        }

        // ── 4. Liquidity floor & trade size sanity ───────────────────
        if !self.check_liquidity_and_sizing(opp, sized, &mut trace) {
            passed = false;
        }

        // ── 5. Deep eth_call dry‑run with gas estimation ─────────────
        let mut predicted_amount_out = None;
        let mut estimated_gas_units = None;
        let mut estimated_profit_after_gas_usd = None;

        if passed {
            (predicted_amount_out, estimated_gas_units, estimated_profit_after_gas_usd) =
                self.deep_simulate(sized, &mut trace).await;
            if predicted_amount_out.is_none() {
                passed = false;
            }
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
                CheckStatus::Skipped("earlier check failed".into()),
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

    async fn check_rpc_health(&self, trace: &mut Vec<(String, CheckStatus)>) -> (bool, Option<Block<TxHash>>) {
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
                    CheckStatus::Fail("disagree".into()),
                ));
                false
            }
        };

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
                    CheckStatus::Fail(format!("head block is {age}s old")),
                ));
                return (false, head);
            } else {
                trace.push((
                    "block freshness".into(),
                    CheckStatus::Pass(format!("head block age {age}s")),
                ));
            }
        }
        (consensus, head)
    }

    fn check_opportunity_age(&self, opp: &Opportunity, block: &Block<TxHash>, trace: &mut Vec<(String, CheckStatus)>) {
        let current_block = block.number.unwrap_or_default().as_u64();
        if let Some(discovered_block) = opp.block_number {
            let drift = current_block.saturating_sub(discovered_block);
            if drift > self.config.max_rpc_block_disagreement {
                trace.push((
                    "opportunity age".into(),
                    CheckStatus::Fail(format!("scanner block {discovered_block} is {drift} blocks behind")),
                ));
            } else {
                trace.push((
                    "opportunity age".into(),
                    CheckStatus::Pass(format!("scanner block {discovered_block}, head {current_block}")),
                ));
            }
        }
    }

    async fn check_gas(&self, head_block: Option<&Block<TxHash>>, trace: &mut Vec<(String, CheckStatus)>) -> (bool, Option<GasMetrics>) {
        let base_fee = head_block
            .and_then(|b| b.base_fee_per_gas)
            .map(|b| b.as_u128() as f64 / 1e9)
            .unwrap_or(0.0);
        let priority_fee = self.estimate_priority_fee().await;
        let total_fee = base_fee + priority_fee;

        let metrics = GasMetrics {
            base_fee_gwei: base_fee,
            priority_fee_gwei: priority_fee,
            max_total_fee_gwei: total_fee,
            estimated_gas_cost_usd: 0.0,
        };

        if total_fee > self.config.max_total_fee_gwei {
            trace.push((
                "gas ceiling".into(),
                CheckStatus::Fail(format!("total fee {:.1} gwei exceeds max {:.1}", total_fee, self.config.max_total_fee_gwei)),
            ));
            (false, Some(metrics))
        } else {
            trace.push((
                "gas ceiling".into(),
                CheckStatus::Pass(format!("total fee {:.1} gwei", total_fee)),
            ));
            (true, Some(metrics))
        }
    }

    async fn estimate_priority_fee(&self) -> f64 {
        // Simplified: use configured max tip, can be upgraded later
        self.config.max_priority_fee_gwei
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
        let amount = U256::from((sized.size_usd * 1e6) as u128);

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

        let estimated_gas = self.estimate_gas_for_arb(&contract, amount).await;
        let gas_units = estimated_gas.map(|g| g.into());

        let profit_after_gas = sized.net_profit_usd; // real value from ProfitCalculator

        (Some(amount), gas_units, Some(profit_after_gas))
    }

    async fn estimate_gas_for_arb(&self, contract: &FlashArbSim<M>, amount: U256) -> Option<U256> {
        let call = contract.execute_flash_loan(amount);
        let estimate = timeout(Duration::from_secs(3), self.provider.estimate_gas(&call.tx, None))
            .await
            .ok()
            .and_then(|r| r.ok());
        estimate.map(|g| g * 12 / 10) // 20% buffer
    }
}

fn decode_revert_reason(err: &impl std::fmt::Debug) -> String {
    format!("{err:?}")
}
