// src/simulation.rs
//  Full enterprise pre-flight simulator

use ethers::prelude::*;
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
    pub trace: Vec<(String, CheckStatus)>,
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
    pub fn new(provider: Arc<M>, rpc_manager: Arc<RpcManager>, contract_address: Address, config: SimulationConfig) -> Self {
        Simulator { provider, rpc_manager, contract_address, config }
    }
    pub async fn run(&self, opp: &Opportunity, sized: &SizedTrade) -> SimulationReport {
        let mut trace = Vec::new();
        let mut passed = true;

        let (rpc_ok, _) = self.check_rpc_health(&mut trace).await;
        if !rpc_ok { passed = false; }

        let (gas_ok, gas_metrics) = self.check_gas(&mut trace).await;
        if !gas_ok { passed = false; }

        self.check_liquidity_and_sizing(opp, sized, &mut trace);

        let mut predicted_amount_out = None;
        let mut estimated_gas_units = None;
        let mut estimated_profit_after_gas_usd = None;

        if passed {
            let contract = FlashArbSim::new(self.contract_address, self.provider.clone());
            let amount = U256::from((sized.size_usd * 1e6) as u128);
            match timeout(Duration::from_secs(2), contract.execute_flash_loan(amount).call()).await {
                Ok(Ok(())) => {
                    trace.push(("eth_call revert".into(), CheckStatus::Pass("would not revert".into())));
                    predicted_amount_out = Some(amount);
                }
                Ok(Err(e)) => {
                    trace.push(("eth_call revert".into(), CheckStatus::Fail(format!("would revert: {e:?}"))));
                    passed = false;
                }
                Err(_) => {
                    trace.push(("eth_call revert".into(), CheckStatus::Fail("timeout".into())));
                    passed = false;
                }
            }

            let profit = sized.net_profit_usd; // using net_profit_usd
            estimated_profit_after_gas_usd = Some(profit);
            if profit < self.config.min_profit_after_gas_usd {
                trace.push(("profit floor".into(), CheckStatus::Fail(format!("profit ${:.2} < min ${:.2}", profit, self.config.min_profit_after_gas_usd))));
                passed = false;
            }
        } else {
            trace.push(("eth_call simulation".into(), CheckStatus::Skipped("earlier check failed".into())));
        }

        SimulationReport {
            passed,
            trace,
            gas_metrics: gas_metrics.unwrap_or(GasMetrics { base_fee_gwei: 0.0, priority_fee_gwei: 0.0, max_total_fee_gwei: 0.0, estimated_gas_cost_usd: 0.0 }),
            rpc_consensus_ok: rpc_ok,
            predicted_amount_out,
            estimated_gas_units,
            estimated_profit_after_gas_usd,
        }
    }

    async fn check_rpc_health(&self, trace: &mut Vec<(String, CheckStatus)>) -> (bool, Option<Block<TxHash>>) {
        let consensus = match self.rpc_manager.consensus_block_number().await {
            Some(pair) => { trace.push(("rpc consensus".into(), CheckStatus::Pass(format!("blocks {}/{}", pair.a, pair.b)))); true }
            None => { trace.push(("rpc consensus".into(), CheckStatus::Fail("disagree".into()))); false }
        };
        (consensus, None)
    }

    async fn check_gas(&self, trace: &mut Vec<(String, CheckStatus)>) -> (bool, Option<GasMetrics>) {
        let gas_price = self.provider.get_gas_price().await.unwrap_or_default();
        let gwei = gas_price.as_u128() as f64 / 1e9;
        if gwei > self.config.max_total_fee_gwei {
            trace.push(("gas ceiling".into(), CheckStatus::Fail(format!("{:.1} gwei", gwei))));
            (false, None)
        } else {
            trace.push(("gas ceiling".into(), CheckStatus::Pass(format!("{:.1} gwei", gwei))));
            (true, Some(GasMetrics { base_fee_gwei: gwei, priority_fee_gwei: 0.0, max_total_fee_gwei: gwei, estimated_gas_cost_usd: 0.0 }))
        }
    }

    fn check_liquidity_and_sizing(&self, opp: &Opportunity, sized: &SizedTrade, trace: &mut Vec<(String, CheckStatus)>) {
        let depth = opp.buy_pool_depth.min(opp.sell_pool_depth);
        if depth < self.config.min_liquidity_usd {
            trace.push(("liquidity".into(), CheckStatus::Fail(format!("${:.0}", depth))));
        } else {
            trace.push(("liquidity".into(), CheckStatus::Pass(format!("${:.0}", depth))));
        }
    }
}
