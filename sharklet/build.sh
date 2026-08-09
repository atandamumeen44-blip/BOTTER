#!/bin/bash
set -e

# Overwrite risk_engine.rs
cat > src/risk_engine.rs << 'EOF'
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Deserialize;
use tracing::warn;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RiskConfig {
    pub max_daily_loss_usd: f64,
    pub max_hourly_loss_usd: f64,
    pub max_trade_size_usd: f64,
    pub max_flash_loan_usd: f64,
    pub max_gas_spent_today_usd: f64,
    pub max_consecutive_failures: u32,
    pub wallet_min_balance_usd: f64,
    pub min_rpc_confidence: f64,
    pub circuit_breaker_pause_secs: u64,
    pub failure_reset_window_secs: u64,
    pub adaptive_limits: bool,
    pub adaptive_tiers: Vec<(f64, f64, f64, f64)>,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_daily_loss_usd: env_or("RISK_MAX_DAILY_LOSS_USD", 20.0),
            max_hourly_loss_usd: env_or("RISK_MAX_HOURLY_LOSS_USD", 8.0),
            max_trade_size_usd: env_or("RISK_MAX_TRADE_SIZE_USD", 5_000.0),
            max_flash_loan_usd: env_or("RISK_MAX_FLASH_LOAN_USD", 5_000.0),
            max_gas_spent_today_usd: env_or("RISK_MAX_GAS_SPENT_TODAY_USD", 10.0),
            max_consecutive_failures: env_or("RISK_MAX_CONSECUTIVE_FAILURES", 3),
            wallet_min_balance_usd: env_or("RISK_WALLET_MIN_BALANCE_USD", 5.0),
            min_rpc_confidence: env_or("RISK_MIN_RPC_CONFIDENCE", 0.90),
            circuit_breaker_pause_secs: env_or("RISK_CB_PAUSE_SECS", 300),
            failure_reset_window_secs: env_or("RISK_FAILURE_RESET_SECS", 3_600),
            adaptive_limits: false,
            adaptive_tiers: vec![],
        }
    }
}

fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}

#[derive(Debug)]
struct RollingWindow { entries: Vec<(u64, f64)> }
impl RollingWindow {
    fn new() -> Self { Self { entries: Vec::new() } }
    fn push(&mut self, amount: f64) {
        let now = now_secs();
        self.entries.push((now, amount));
        self.prune(now.saturating_sub(90_000));
    }
    fn sum_within(&self, seconds: u64) -> f64 {
        let cutoff = now_secs().saturating_sub(seconds);
        self.entries.iter().filter(|(ts,_)| *ts >= cutoff).map(|(_,v)| v).sum()
    }
    fn prune(&mut self, cutoff: u64) { self.entries.retain(|(ts,_)| *ts >= cutoff); }
}

#[derive(Debug)]
struct FailureTracker { timestamps: Vec<u64> }
impl FailureTracker {
    fn new() -> Self { Self { timestamps: Vec::new() } }
    fn record_failure(&mut self) { self.timestamps.push(now_secs()); self.prune_old(); }
    fn record_success(&mut self) { self.timestamps.clear(); }
    fn consecutive(&self, window_secs: u64) -> u32 {
        let cutoff = now_secs().saturating_sub(window_secs);
        self.timestamps.iter().filter(|&&ts| ts >= cutoff).count() as u32
    }
    fn prune_old(&mut self) {
        let cutoff = now_secs().saturating_sub(90_000);
        self.timestamps.retain(|&ts| ts >= cutoff);
    }
}

fn now_secs() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() }

pub struct RiskEngine {
    pub limits: RiskConfig,
    losses: RollingWindow,
    gas_spent: RollingWindow,
    failures: FailureTracker,
    paused_until: u64,
}

#[derive(Debug, PartialEq)]
pub enum RiskDecision {
    Approve,
    Reject(String),
    Paused,
}

impl RiskEngine {
    pub fn new(cfg: RiskConfig) -> Self {
        Self { limits: cfg, losses: RollingWindow::new(), gas_spent: RollingWindow::new(), failures: FailureTracker::new(), paused_until: 0 }
    }
    pub fn from_toml(path: &str) -> Self {
        let cfg = std::fs::read_to_string(path).ok().and_then(|c| toml::from_str::<RiskConfig>(&c).ok()).unwrap_or_default();
        Self::new(cfg)
    }
    pub fn evaluate(&mut self, trade_size_usd: f64, flash_loan_usd: f64, wallet_balance_usd: f64, rpc_confidence: f64) -> RiskDecision {
        if self.is_paused() { return RiskDecision::Paused; }
        let failures = self.failures.consecutive(self.limits.failure_reset_window_secs);
        if failures >= self.limits.max_consecutive_failures {
            self.activate_pause();
            return RiskDecision::Reject(format!("circuit breaker tripped — {} failures", failures));
        }
        let (max_trade, max_loan, max_gas_today) = if self.limits.adaptive_limits {
            self.adaptive_limits(wallet_balance_usd)
        } else {
            (self.limits.max_trade_size_usd, self.limits.max_flash_loan_usd, self.limits.max_gas_spent_today_usd)
        };
        if trade_size_usd > max_trade { return RiskDecision::Reject(format!("trade size ${:.0} > max ${:.0}", trade_size_usd, max_trade)); }
        if flash_loan_usd > max_loan { return RiskDecision::Reject(format!("flash loan ${:.0} > max ${:.0}", flash_loan_usd, max_loan)); }
        if wallet_balance_usd < self.limits.wallet_min_balance_usd { return RiskDecision::Reject(format!("wallet ${:.2} < min ${:.2}", wallet_balance_usd, self.limits.wallet_min_balance_usd)); }
        if rpc_confidence < self.limits.min_rpc_confidence { return RiskDecision::Reject(format!("RPC confidence {:.2} < min {:.2}", rpc_confidence, self.limits.min_rpc_confidence)); }
        let hourly = self.losses.sum_within(3_600);
        if hourly >= self.limits.max_hourly_loss_usd { return RiskDecision::Reject(format!("hourly loss ${:.2} >= limit ${:.2}", hourly, self.limits.max_hourly_loss_usd)); }
        let daily = self.losses.sum_within(86_400);
        if daily >= self.limits.max_daily_loss_usd { return RiskDecision::Reject(format!("daily loss ${:.2} >= limit ${:.2}", daily, self.limits.max_daily_loss_usd)); }
        let gas_today = self.gas_spent.sum_within(86_400);
        if gas_today >= max_gas_today { return RiskDecision::Reject(format!("daily gas ${:.2} >= limit ${:.2}", gas_today, max_gas_today)); }
        RiskDecision::Approve
    }
    pub fn record_result(&mut self, realized_pnl_usd: f64, gas_cost_usd: f64) {
        self.gas_spent.push(gas_cost_usd);
        if realized_pnl_usd < 0.0 { self.losses.push(-realized_pnl_usd); self.failures.record_failure(); }
        else { self.failures.record_success(); }
    }
    fn is_paused(&self) -> bool { now_secs() < self.paused_until }
    fn activate_pause(&mut self) {
        self.paused_until = now_secs() + self.limits.circuit_breaker_pause_secs;
        warn!("Circuit breaker paused for {}s", self.limits.circuit_breaker_pause_secs);
    }
    pub fn resume(&mut self) { self.paused_until = 0; self.failures.timestamps.clear(); }
    fn adaptive_limits(&self, wallet: f64) -> (f64, f64, f64) {
        for &(ceiling, trade, loan, gas) in &self.limits.adaptive_tiers {
            if wallet <= ceiling { return (trade, loan, gas); }
        }
        (self.limits.max_trade_size_usd, self.limits.max_flash_loan_usd, self.limits.max_gas_spent_today_usd)
    }
}
EOF

# Overwrite simulation.rs
cat > src/simulation.rs << 'EOF'
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
        let (rpc_ok, head_block) = self.check_rpc_health(&mut trace).await;
        if !rpc_ok { passed = false; }
        if let Some(ref block) = head_block {
            self.check_opportunity_age(opp, block, &mut trace);
        }
        let (gas_ok, gas_metrics) = self.check_gas(head_block.as_ref(), &mut trace).await;
        let mut gas_metrics = gas_metrics;
        if !gas_ok { passed = false; }
        if !self.check_liquidity_and_sizing(opp, sized, &mut trace) { passed = false; }
        let mut predicted_amount_out = None;
        let mut estimated_gas_units = None;
        let mut estimated_profit_after_gas_usd = None;
        if passed {
            (predicted_amount_out, estimated_gas_units, estimated_profit_after_gas_usd) =
                self.deep_simulate(sized, &mut trace).await;
            if predicted_amount_out.is_none() { passed = false; }
            if let Some(profit_usd) = estimated_profit_after_gas_usd {
                if profit_usd < self.config.min_profit_after_gas_usd {
                    trace.push(("profit floor".into(), CheckStatus::Fail(format!("profit after gas ${:.2} < min ${:.2}", profit_usd, self.config.min_profit_after_gas_usd))));
                    passed = false;
                }
            }
        } else {
            trace.push(("eth_call simulation".into(), CheckStatus::Skipped("earlier check failed".into())));
        }
        SimulationReport { passed, trace, gas_metrics, rpc_consensus_ok: rpc_ok, predicted_amount_out, estimated_gas_units, estimated_profit_after_gas_usd }
    }

    async fn check_rpc_health(&self, trace: &mut Vec<(String, CheckStatus)>) -> (bool, Option<Block<TxHash>>) {
        let consensus = match self.rpc_manager.consensus_block_number().await {
            Some(pair) => { trace.push(("rpc consensus".into(), CheckStatus::Pass(format!("blocks {}/{}", pair.a, pair.b)))); true }
            None => { trace.push(("rpc consensus".into(), CheckStatus::Fail("disagree".into()))); false }
        };
        let head = self.provider.get_block(BlockNumber::Latest).await.ok().flatten();
        if let Some(ref block) = head {
            let timestamp = block.timestamp.as_u64();
            let now_secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            let age = now_secs.saturating_sub(timestamp);
            if age > self.config.max_block_age_seconds {
                trace.push(("block freshness".into(), CheckStatus::Fail(format!("head block is {age}s old"))));
                return (false, head);
            } else {
                trace.push(("block freshness".into(), CheckStatus::Pass(format!("head block age {age}s"))));
            }
        }
        (consensus, head)
    }

    fn check_opportunity_age(&self, opp: &Opportunity, block: &Block<TxHash>, trace: &mut Vec<(String, CheckStatus)>) {
        let current_block = block.number.unwrap_or_default().as_u64();
        if let Some(discovered_block) = opp.block_number {
            let drift = current_block.saturating_sub(discovered_block);
            if drift > self.config.max_rpc_block_disagreement {
                trace.push(("opportunity age".into(), CheckStatus::Fail(format!("scanner block {discovered_block} is {drift} blocks behind"))));
            } else {
                trace.push(("opportunity age".into(), CheckStatus::Pass(format!("scanner block {discovered_block}, head {current_block}"))));
            }
        }
    }

    async fn check_gas(&self, head_block: Option<&Block<TxHash>>, trace: &mut Vec<(String, CheckStatus)>) -> (bool, GasMetrics) {
        let base_fee = head_block.and_then(|b| b.base_fee_per_gas).map(|b| b.as_u128() as f64 / 1e9).unwrap_or(0.0);
        let priority_fee = self.estimate_priority_fee().await;
        let total_fee = base_fee + priority_fee;
        let metrics = GasMetrics { base_fee_gwei: base_fee, priority_fee_gwei: priority_fee, max_total_fee_gwei: total_fee, estimated_gas_cost_usd: 0.0 };
        if total_fee > self.config.max_total_fee_gwei {
            trace.push(("gas ceiling".into(), CheckStatus::Fail(format!("total fee {:.1} gwei exceeds max {:.1}", total_fee, self.config.max_total_fee_gwei))));
            (false, metrics)
        } else {
            trace.push(("gas ceiling".into(), CheckStatus::Pass(format!("total fee {:.1} gwei", total_fee))));
            (true, metrics)
        }
    }

    async fn estimate_priority_fee(&self) -> f64 { self.config.max_priority_fee_gwei }

    fn check_liquidity_and_sizing(&self, opp: &Opportunity, sized: &SizedTrade, trace: &mut Vec<(String, CheckStatus)>) -> bool {
        let shallow_depth = opp.buy_pool_depth.min(opp.sell_pool_depth);
        let mut ok = true;
        if shallow_depth < self.config.min_liquidity_usd {
            trace.push(("liquidity floor".into(), CheckStatus::Fail(format!("${:.0} < floor ${:.0}", shallow_depth, self.config.min_liquidity_usd))));
            ok = false;
        } else {
            trace.push(("liquidity floor".into(), CheckStatus::Pass(format!("${:.0}", shallow_depth))));
        }
        let depth_ratio = sized.size_usd / shallow_depth;
        if depth_ratio > self.config.max_trade_vs_depth_ratio {
            trace.push(("trade size vs depth".into(), CheckStatus::Fail(format!("trade ${:.0} is {:.1}% of shallow pool ${:.0} — max {:.0}%", sized.size_usd, depth_ratio*100.0, shallow_depth, self.config.max_trade_vs_depth_ratio*100.0))));
            ok = false;
        } else {
            trace.push(("trade size vs depth".into(), CheckStatus::Pass(format!("{:.1}% of pool", depth_ratio*100.0))));
        }
        ok
    }

    async fn deep_simulate(&self, sized: &SizedTrade, trace: &mut Vec<(String, CheckStatus)>) -> (Option<U256>, Option<U256>, Option<f64>) {
        let contract = FlashArbSim::new(self.contract_address, self.provider.clone());
        let amount = U256::from((sized.size_usd * 1e6) as u128);
        let call_result = timeout(Duration::from_secs(2), contract.execute_flash_loan(amount).call()).await;
        let call_ok = match call_result {
            Ok(Ok(())) => { trace.push(("eth_call revert check".into(), CheckStatus::Pass("would not revert".into()))); true }
            Ok(Err(e)) => { let decoded = decode_revert_reason(&e); trace.push(("eth_call revert check".into(), CheckStatus::Fail(format!("would revert: {decoded}")))); false }
            Err(_) => { trace.push(("eth_call revert check".into(), CheckStatus::Fail("timeout".into()))); false }
        };
        if !call_ok { return (None, None, None); }
        let estimated_gas = self.estimate_gas_for_arb(&contract, amount).await;
        let gas_units = estimated_gas.map(|g| g.into());
        let profit_after_gas = sized.net_profit_usd;
        (Some(amount), gas_units, Some(profit_after_gas))
    }

    async fn estimate_gas_for_arb(&self, contract: &FlashArbSim<M>, amount: U256) -> Option<U256> {
        let call = contract.execute_flash_loan(amount);
        let estimate = timeout(Duration::from_secs(3), self.provider.estimate_gas(&call.tx, None)).await.ok().and_then(|r| r.ok());
        estimate.map(|g| g * 12 / 10)
    }
}

fn decode_revert_reason(err: &impl std::fmt::Debug) -> String { format!("{err:?}") }
EOF

# Overwrite executor.rs
cat > src/executor.rs << 'EOF'
use ethers::prelude::*;
use std::sync::Arc;
use std::future::Future;
use tokio::time::{sleep, Duration};

abigen!(
    FlashArb,
    r#"[
        function executeFlashLoan(uint256 amount) external
        event ArbExecuted(uint256 profit, uint256 amountIn, address initiator)
    ]"#
);

#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[error("simulation reverted: {0}")] SimulationReverted(String),
    #[error("send failed: {0}")] SendFailed(String),
    #[error("no receipt")] NoReceipt,
    #[error("log not found")] LogNotFound,
    #[error("gas price too high: {0}")] GasPriceTooHigh(String),
    #[error("nonce error: {0}")] NonceError(String),
}

pub struct Executor<M: Middleware> {
    contract: FlashArb<SignerMiddleware<M, LocalWallet>>,
    read_provider: Arc<M>,
    private_relay: Option<Provider<Http>>,
    max_retries: u32,
    nonce: Option<U256>,
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub tx_hash: H256,
    pub realized_profit_usdc: f64,
    pub gas_used: U256,
    pub gas_cost_usd: f64,
    pub effective_gas_price_gwei: f64,
}

impl<M: Middleware + 'static> Executor<M> {
    pub fn new(contract_address: Address, wallet: LocalWallet, signer_provider: Arc<SignerMiddleware<M, LocalWallet>>, read_provider: Arc<M>, private_relay_url: Option<String>) -> Self {
        let contract = FlashArb::new(contract_address, signer_provider);
        let _ = wallet;
        let private_relay = private_relay_url.and_then(|url| Provider::<Http>::try_from(url.as_str()).ok());
        Executor { contract, read_provider, private_relay, max_retries: 3, nonce: None }
    }
    pub async fn sync_nonce(&mut self) -> Result<(), ExecutorError> {
        let nonce = self.read_provider.get_transaction_count(self.contract.client().address(), None).await.map_err(|e| ExecutorError::NonceError(format!("{e:?}")))?;
        self.nonce = Some(nonce);
        Ok(())
    }
    pub async fn simulate(&self, amount: U256) -> Result<(), ExecutorError> {
        self.contract.execute_flash_loan(amount).call().await.map_err(|e| ExecutorError::SimulationReverted(format!("{e:?}")))?;
        Ok(())
    }
    pub async fn estimate_gas(&self, amount: U256) -> Result<U256, ExecutorError> {
        self.contract.execute_flash_loan(amount).estimate_gas().await.map_err(|e| ExecutorError::SimulationReverted(format!("{e:?}")))
    }
    pub async fn simulate_and_execute(&mut self, amount: U256, gas_price_usd_per_gas: f64) -> Result<ExecutionResult, ExecutorError> {
        let mut last_err = None;
        for attempt in 0..self.max_retries {
            match self.simulate(amount).await {
                Ok(()) => { last_err = None; break; }
                Err(e) => { last_err = Some(e); if attempt + 1 < self.max_retries { sleep(Duration::from_millis(200 * 2u64.pow(attempt))).await; } }
            }
        }
        if let Some(e) = last_err { return Err(e); }
        if self.nonce.is_none() { self.sync_nonce().await?; }
        let mut call = self.contract.execute_flash_loan(amount);
        call.tx.set_nonce(self.nonce.unwrap());
        let receipt = if let Some(ref relay) = self.private_relay {
            let nonce = self.nonce.unwrap();
            call.tx.set_nonce(nonce);
            let signed = call.tx.rlp_signed(&call.client().signer().sign_transaction_sync(&call.tx).map_err(|e| ExecutorError::SendFailed(format!("sign error: {e:?}")))?);
            let pending = relay.send_raw_transaction(signed).await.map_err(|e| ExecutorError::SendFailed(format!("relay send: {e:?}")))?;
            self.nonce = Some(nonce + 1);
            pending.await.map_err(|e| ExecutorError::SendFailed(format!("{e:?}")))?.ok_or(ExecutorError::NoReceipt)?
        } else {
            let pending = call.send().await.map_err(|e| ExecutorError::SendFailed(format!("{e:?}")))?;
            self.nonce = Some(self.nonce.unwrap() + 1);
            pending.await.map_err(|e| ExecutorError::SendFailed(format!("{e:?}")))?.ok_or(ExecutorError::NoReceipt)?
        };
        let gas_used = receipt.gas_used.unwrap_or_default();
        let effective_gas_price = receipt.effective_gas_price.unwrap_or_default();
        let gas_cost_usd = gas_used.as_u128() as f64 * effective_gas_price.as_u128() as f64 / 1e18 * gas_price_usd_per_gas;
        let profit_log = receipt.logs.iter().find_map(|log| self.contract.decode_event::<ArbExecutedFilter>("ArbExecuted", log.topics.clone(), log.data.clone()).ok()).ok_or(ExecutorError::LogNotFound)?;
        let realized_profit_usdc = profit_log.profit.as_u128() as f64 / 1e6;
        Ok(ExecutionResult { tx_hash: receipt.transaction_hash, realized_profit_usdc, gas_used, gas_cost_usd, effective_gas_price_gwei: effective_gas_price.as_u128() as f64 / 1e9 })
    }
    pub async fn execute_loop<F, Fut>(&mut self, initial_amount: U256, max_repeats: u32, gas_price_usd_per_gas: f64, target_profit: Option<f64>, max_loss: Option<f64>, spread_check: F) -> Vec<Result<ExecutionResult, ExecutorError>>
    where F: Fn() -> Fut, Fut: Future<Output = bool> {
        let mut results = Vec::new();
        let mut cum = 0.0;
        for _ in 0..max_repeats {
            if !results.is_empty() && !spread_check().await { break; }
            if let Some(t) = target_profit { if cum >= t { break; } }
            if let Some(l) = max_loss { if cum <= -l { break; } }
            match self.simulate_and_execute(initial_amount, gas_price_usd_per_gas).await {
                Ok(r) => { cum += r.realized_profit_usdc - r.gas_cost_usd; results.push(Ok(r)); }
                Err(e) => { results.push(Err(e)); break; }
            }
        }
        results
    }
}
EOF

# Overwrite main.rs
cat > src/main.rs << 'EOF'
mod scanner;
mod risk_engine;
mod profit_calc;
mod rpc_manager;
mod executor;
mod logger;
mod simulation;
mod dex_registry;
mod price_oracle;
mod gas_manager;
mod api;

use ethers::prelude::*;
use ethers::signers::LocalWallet;
use risk_engine::{RiskEngine, RiskDecision};
use profit_calc::{CostModel, ProfitDecision, default_tiers, size_and_evaluate};
use rpc_manager::RpcManager;
use scanner::Scanner;
use executor::Executor;
use gas_manager::GasManager;

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{info, warn, error};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("sharklet=info".parse()?))
        .with_target(true)
        .json()
        .init();
    info!("🦈 Sharklet starting");

    dotenvy::dotenv().ok();
    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY");
    let rpc_urls: Vec<String> = std::env::var("RPC_URLS").expect("RPC_URLS").split(',').map(|s| s.trim().to_string()).collect();
    let contract_address: Address = std::env::var("CONTRACT_ADDRESS").expect("CONTRACT_ADDRESS").parse()?;
    let chain_id: u64 = std::env::var("CHAIN_ID").unwrap_or_else(|_| "137".into()).parse()?;
    let wallet: LocalWallet = private_key.parse::<LocalWallet>()?.with_chain_id(chain_id);

    let rpc_manager = Arc::new(RpcManager::new(rpc_urls)?);
    rpc_manager.start_health_checks();
    let read_provider = rpc_manager.best().await;
    let signer_provider = Arc::new(SignerMiddleware::new((*read_provider).clone(), wallet.clone()));

    let risk_engine = RiskEngine::from_toml("config/bot.toml");
    let risk_engine = RwLock::new(risk_engine);

    let private_relay_url = std::env::var("PRIVATE_RELAY_URL").ok().filter(|s| !s.is_empty());
    let mut executor = Executor::new(contract_address, wallet.clone(), signer_provider.clone(), read_provider.clone(), private_relay_url);
    executor.sync_nonce().await?;
    let executor = RwLock::new(executor);

    let simulator = Arc::new(simulation::Simulator::new(read_provider.clone(), rpc_manager.clone(), contract_address, simulation::SimulationConfig::default()));

    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "sharklet.db".into());
    let db = Arc::new(logger::Logger::open(&db_path)?);

    let gas_manager = GasManager::new(wallet.address(), read_provider.clone());
    let gas_manager = RwLock::new(gas_manager);

    let oracle = Arc::new(price_oracle::PriceOracle::from_chain_config(read_provider.clone(), chain_id).await);
    let fallback_price = std::env::var("MATIC_USD_PRICE_HINT").ok().and_then(|s| s.parse().ok()).unwrap_or(0.60);

    let pairs = dex_registry::resolve_all_pools(read_provider.clone(), &dex_registry::known_dexes(), &dex_registry::token_pairs()).await;
    let scanner = RwLock::new(Scanner::new(read_provider.clone(), pairs, 6, 18));

    // Start dashboard API with DB access
    let app_state = Arc::new(api::AppState { health: true });
    let db_clone = db.clone();
    tokio::spawn(api::start_server(app_state, db_clone));

    loop {
        let matic_usd = oracle.price_usd(fallback_price, 3600).await;
        let wallet_balance_usd;
        {
            let mut gas = gas_manager.write().await;
            wallet_balance_usd = gas.update_balance(matic_usd).await;
        }

        let opportunities;
        {
            let mut scan = scanner.write().await;
            opportunities = scan.scan().await;
        }
        if opportunities.is_empty() { sleep(Duration::from_millis(2000)).await; continue; }
        let best = &opportunities[0];

        let gas_price_wei = read_provider.get_gas_price().await.unwrap_or_default();
        let rough = U256::from(1000 * 1e6 as u128);
        let gas_units = executor.read().await.estimate_gas(rough).await.map(|g| g.as_u128()).unwrap_or(300_000);
        let gas_cost_matic = (gas_price_wei.as_u128() * gas_units) as f64 / 1e18;
        let live_gas_cost_usd = gas_cost_matic * matic_usd;

        let mut cost_model = CostModel { flash_fee_bps: 9.0, gas_cost_usd: live_gas_cost_usd.max(0.001), safety_margin_usd: 0.30 };
        let max_trade = risk_engine.read().await.limits.max_trade_size_usd;
        let decision = size_and_evaluate(best, &cost_model, max_trade, wallet_balance_usd, &default_tiers());
        let sized = match decision {
            ProfitDecision::Go(s) => s,
            ProfitDecision::NoGo { reason, best_net_profit_usd } => {
                info!(pair = %best.label, reason, net = best_net_profit_usd, "trade skipped");
                log_trade(&db, best, None, live_gas_cost_usd, "skipped", Some(&reason)).await;
                sleep(Duration::from_millis(500)).await;
                continue;
            }
        };

        let rpc_confidence = rpc_manager.scoreboard().await.iter().map(|s| s.success_rate).fold(0.0, f64::max);
        let mut risk = risk_engine.write().await;
        match risk.evaluate(sized.size_usd, sized.size_usd, wallet_balance_usd, rpc_confidence) {
            RiskDecision::Reject(reason) => {
                warn!(pair = %best.label, reason, "risk engine rejected");
                log_trade(&db, best, Some(sized.size_usd), live_gas_cost_usd, "rejected", Some(&reason)).await;
                sleep(Duration::from_millis(500)).await;
                continue;
            }
            RiskDecision::Paused => {
                warn!("circuit breaker paused");
                sleep(Duration::from_secs(30)).await;
                continue;
            }
            RiskDecision::Approve => {}
        }
        drop(risk);

        let sim_report = simulator.run(best, &sized).await;
        for (label, status) in &sim_report.trace { info!("[sim] {}: {:?}", label, status); }
        if !sim_report.passed {
            warn!(pair = %best.label, "simulation rejected");
            log_trade(&db, best, Some(sized.size_usd), live_gas_cost_usd, "sim_rejected", Some("sim")).await;
            sleep(Duration::from_millis(500)).await;
            continue;
        }

        let amount = U256::from((sized.size_usd * 1e6) as u128);
        let gas_price_usd_per_gas = matic_usd / 1e9;
        let results;
        {
            let mut exec = executor.write().await;
            results = exec.execute_loop(amount, 20, gas_price_usd_per_gas, Some(1000.0), Some(100.0), || async { true }).await;
        }

        let mut risk = risk_engine.write().await;
        let mut gas = gas_manager.write().await;
        for res in &results {
            match res {
                Ok(r) => {
                    let net = r.realized_profit_usdc - r.gas_cost_usd;
                    risk.record_result(net, r.gas_cost_usd);
                    gas.spend_gas(r.gas_cost_usd);
                    gas.add_gas(gas.calculate_gas_to_reinvest(net));
                    info!(pair = %best.label, net, tx = %r.tx_hash, "trade executed");
                    log_trade(&db, best, Some(sized.size_usd), r.gas_cost_usd, "executed", None).await;
                }
                Err(e) => {
                    risk.record_result(-live_gas_cost_usd, live_gas_cost_usd);
                    gas.spend_gas(live_gas_cost_usd);
                    error!(pair = %best.label, error = %e, "execution failed");
                    log_trade(&db, best, Some(sized.size_usd), live_gas_cost_usd, "failed", Some("execution error")).await;
                }
            }
        }
        sleep(Duration::from_millis(500)).await;
    }
}

async fn log_trade(db: &logger::Logger, opp: &scanner::Opportunity, size: Option<f64>, gas_cost: f64, status: &str, reason: Option<&str>) {
    let _ = db.record(logger::TradeRecord {
        pair_label: opp.label.clone(),
        buy_dex: opp.buy_dex.clone(),
        sell_dex: opp.sell_dex.clone(),
        spread_pct: opp.spread_pct,
        size_usd: size.unwrap_or(0.0),
        predicted_net_usd: 0.0,
        realized_net_usd: None,
        gas_cost_usd: gas_cost,
        status: status.to_string(),
        reason: reason.map(|s| s.to_string()),
        tx_hash: None,
    }).await;
}
EOF

# Overwrite api.rs (with embedded dashboard)
cat > src/api.rs << 'EOF'
use warp::Filter;
use std::sync::Arc;
use serde_json::json;

pub struct AppState {
    pub health: bool,
}

pub async fn start_server(state: Arc<AppState>, db: Arc<crate::logger::Logger>) {
    let db_filter = warp::any().map(move || db.clone());
    let health = warp::path("health").map(|| warp::reply::json(&json!({"status": "ok"})));

    let stats = warp::path("api").and(warp::path("stats"))
        .and(db_filter.clone())
        .and_then(|db: Arc<crate::logger::Logger>| async move {
            let total_profit = db.total_profit_within(86400*365).await.unwrap_or(0.0);
            let today_profit = db.total_profit_within(86400).await.unwrap_or(0.0);
            Ok::<_, warp::Rejection>(warp::reply::json(&json!({
                "wallet_usd": 0.0,
                "today_pnl": today_profit,
                "total_pnl": total_profit,
                "trades_today": 0,
                "gas_gwei": 0,
                "queue_size": 0,
                "profit_series": []
            })))
        });

    let trades = warp::path("api").and(warp::path("trades"))
        .and(db_filter)
        .and_then(|db: Arc<crate::logger::Logger>| async move {
            let recent = db.recent_trades(50).await.unwrap_or_default();
            let data: Vec<_> = recent.into_iter().map(|t| json!({
                "time": "-",
                "pair": t.pair_label,
                "buy_dex": t.buy_dex,
                "sell_dex": t.sell_dex,
                "spread_pct": t.spread_pct,
                "net_usd": t.realized_net_usd,
                "status": t.status,
            })).collect();
            Ok::<_, warp::Rejection>(warp::reply::json(&data))
        });

    let dashboard = warp::path::end().map(|| {
        warp::reply::html(r###"
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Sharklet – Control Panel</title>
<style>
  :root{
    --abyss:#07090c; --panel:#0d1218; --panel-2:#121924; --line:#1c2733;
    --amber:#ffb454; --amber-dim:#7a5a2e; --teal:#3ddad0; --red:#ff5c5c; --violet:#9d8cff;
    --text:#e6ecf1; --text-dim:#8b96a3; --text-faint:#4d5763;
    --mono: 'SF Mono', 'Consolas', 'Monaco', monospace;
    --sans: -apple-system, 'Segoe UI', system-ui, sans-serif;
    --radius: 12px;
  }
  *{margin:0;padding:0;box-sizing:border-box;}
  html{scrollbar-color:#2a3543 transparent;}
  body{
    background:
      radial-gradient(ellipse 900px 500px at 15% -10%, rgba(255,180,84,.06), transparent 60%),
      radial-gradient(ellipse 900px 600px at 85% 105%, rgba(61,218,208,.05), transparent 60%),
      var(--abyss);
    color:var(--text); font-family:var(--sans); min-height:100vh; letter-spacing:.1px;
  }
  header{
    display:flex; align-items:center; justify-content:space-between;
    padding:1rem 1.6rem; border-bottom:1px solid var(--line);
    background:rgba(13,18,24,.75); backdrop-filter:blur(14px);
    position:sticky; top:0; z-index:10;
  }
  .brand{display:flex; align-items:center; gap:.7rem;}
  .brand .mark{font-size:1.35rem;}
  .brand h1{font-size:1.05rem; font-weight:800; letter-spacing:.4px;}
  .brand .sub{font-size:.66rem; color:var(--text-faint); font-family:var(--mono); letter-spacing:.6px;}

  .ping{display:flex; align-items:center; gap:.6rem; font-family:var(--mono); font-size:.72rem;}
  .ping-dot{width:9px; height:9px; border-radius:50%; background:var(--text-faint); position:relative; flex-shrink:0;}
  .ping-dot.live{background:var(--teal);}
  .ping-dot.live::after{content:''; position:absolute; inset:-6px; border-radius:50%; border:1px solid var(--teal); animation:sonar 2.2s ease-out infinite;}
  @keyframes sonar{ 0%{transform:scale(.5); opacity:.9;} 100%{transform:scale(2.2); opacity:0;} }
  .mode-label{text-transform:uppercase; letter-spacing:1px; font-weight:700;}
  .mode-label.live{color:var(--teal);}

  main{max-width:1180px; margin:0 auto; padding:1.6rem;}

  .stat-rail{display:grid; grid-template-columns:repeat(auto-fit,minmax(150px,1fr)); gap:1px;
    background:var(--line); border:1px solid var(--line); border-radius:var(--radius); overflow:hidden; margin-bottom:1.6rem;}
  .stat{background:var(--panel); padding:1.15rem 1.25rem; transition:background .2s;}
  .stat .k{font-size:.65rem; text-transform:uppercase; letter-spacing:.8px; color:var(--text-faint); margin-bottom:.4rem;}
  .stat .v{font-family:var(--mono); font-size:1.45rem; font-weight:700;}

  .grid2{display:grid; grid-template-columns:1.6fr 1fr; gap:1.2rem; margin-bottom:1.2rem;}
  @media(max-width:860px){ .grid2{grid-template-columns:1fr;} }

  .card{
    background:linear-gradient(180deg, var(--panel), rgba(13,18,24,.85));
    border:1px solid var(--line); border-radius:var(--radius); padding:1.25rem;
  }
  .card h2{font-size:.74rem; text-transform:uppercase; letter-spacing:1px; color:var(--text-dim); margin-bottom:1rem;}

  canvas{width:100%; display:block;}

  table{width:100%; border-collapse:collapse; font-size:.78rem;}
  th{text-align:left; font-size:.65rem; text-transform:uppercase; letter-spacing:.6px; color:var(--text-faint);
     padding:.5rem .6rem; border-bottom:1px solid var(--line); font-weight:600;}
  td{padding:.55rem .6rem; border-bottom:1px solid var(--line); font-family:var(--mono); color:var(--text-dim);}
  .status-pill{font-size:.65rem; padding:.15rem .5rem; border-radius:4px; font-weight:700; text-transform:uppercase;}
  .status-pill.executed{background:rgba(61,218,208,.12); color:var(--teal);}
  .status-pill.skipped, .status-pill.rejected, .status-pill.sim_rejected{background:rgba(255,180,84,.1); color:var(--amber);}
  .status-pill.failed{background:rgba(255,92,92,.12); color:var(--red);}
  .net-pos{color:var(--teal);} .net-neg{color:var(--red);}
</style>
</head>
<body>
<header>
  <div class="brand">
    <span class="mark">🦈</span>
    <div>
      <h1>SHARKLET</h1>
      <div class="sub">CONTROL PANEL</div>
    </div>
  </div>
  <div class="ping">
    <span class="ping-dot live" id="pingDot"></span>
    <span class="mode-label live" id="modeLabel">LIVE</span>
    <span id="clock">--:--:--</span>
  </div>
</header>
<main>
  <div class="stat-rail" id="statRail"></div>
  <div class="grid2">
    <div class="card">
      <h2>Profit Curve</h2>
      <canvas id="profitChart" height="140"></canvas>
    </div>
    <div class="card">
      <h2>Trade Log</h2>
      <table>
        <thead><tr><th>Time</th><th>Pair</th><th>Route</th><th>Spread</th><th>Net</th><th>Status</th></tr></thead>
        <tbody id="tradeBody"></tbody>
      </table>
      <div id="tradeEmpty">No trades yet</div>
    </div>
  </div>
</main>
<footer>Sharklet · polling /api/stats, /api/trades every 3s</footer>
<script>
(function(){
  const $ = id => document.getElementById(id);
  async function refresh(){
    try {
      const [stats, trades] = await Promise.all([
        fetch('/api/stats').then(r=>r.json()),
        fetch('/api/trades').then(r=>r.json())
      ]);
      $('statRail').innerHTML = [
        ['Wallet', '$'+stats.wallet_usd.toFixed(2)],
        ["Today's P&L", (stats.today_pnl>=0?'+':'')+'$'+stats.today_pnl.toFixed(2)],
        ['Total P&L', (stats.total_pnl>=0?'+':'')+'$'+stats.total_pnl.toFixed(2)],
        ['Trades Today', stats.trades_today],
        ['Gas', stats.gas_gwei+' gwei'],
      ].map(([k,v]) => `<div class="stat"><div class="k">${k}</div><div class="v">${v}</div></div>`).join('');
      const tbody = $('tradeBody');
      tbody.innerHTML = trades.map(t => `
        <tr>
          <td>${t.time}</td>
          <td>${t.pair}</td>
          <td>${t.buy_dex} → ${t.sell_dex}</td>
          <td>${t.spread_pct.toFixed(2)}%</td>
          <td class="${t.net_usd>0?'net-pos':t.net_usd<0?'net-neg':''}">${t.net_usd!=null ? (t.net_usd>=0?'+':'')+'$'+t.net_usd.toFixed(4) : '—'}</td>
          <td><span class="status-pill ${t.status}">${t.status}</span></td>
        </tr>
      `).join('');
      $('tradeEmpty').style.display = trades.length ? 'none' : 'block';
      $('clock').textContent = new Date().toLocaleTimeString('en-US', { hour12:false });
    } catch(e) { /* fallback to demo data */ }
  }
  refresh();
  setInterval(refresh, 3000);
})();
</script>
</body>
</html>
        "###)
    });

    let routes = dashboard.or(health).or(stats).or(trades);
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}
EOF

echo "All source files replaced successfully"