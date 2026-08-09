
#!/bin/bash
# Ensure Unix line endings (LF), not CRLF

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

cat > src/simulation.rs << 'EOF'
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
            Ok(Err(e)) => { let decoded = format!("{e:?}"); trace.push(("eth_call revert check".into(), CheckStatus::Fail(format!("would revert: {decoded}")))); false }
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
EOF

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
            let signer = self.contract.client().signer();
            let signed = call.tx.rlp_signed(&signer.sign_transaction_sync(&call.tx).map_err(|e| ExecutorError::SendFailed(format!("sign error: {e:?}")))?);
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

    let oracle = Arc::new(price_oracle::PriceOracle::from_chain_config(read_provider.clone(), chain_id));
    let fallback_price = std::env::var("MATIC_USD_PRICE_HINT").ok().and_then(|s| s.parse().ok()).unwrap_or(0.60);

    let pairs = dex_registry::resolve_all_pools(read_provider.clone(), &dex_registry::known_dexes(), &dex_registry::token_pairs()).await;
    let scanner = RwLock::new(Scanner::new(read_provider.clone(), pairs, 6, 18));

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

cat > src/scanner.rs << 'EOF'
use ethers::prelude::*;
use ethers::contract::abigen;
use std::sync::Arc;
use std::cell::RefCell;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, warn};

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
    pub pools: Vec<(String, Address)>,
    pub token0: Address,
    pub token1: Address,
    pub routers: Vec<(String, Address)>,
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
    cache: RefCell<Vec<(String, Address, PoolQuote, Instant)>>,
    cache_ttl: Duration,
    min_spread_pct: f64,
}

impl<M: Middleware + 'static> Scanner<M> {
    pub fn new(provider: Arc<M>, pairs: Vec<TrackedPair>, token0_decimals: u32, token1_decimals: u32) -> Self {
        let min_spread = std::env::var("MIN_SPREAD_PCT").ok().and_then(|s| s.parse().ok()).unwrap_or(0.05);
        Self {
            provider,
            pairs,
            token0_decimals,
            token1_decimals,
            cache: RefCell::new(Vec::new()),
            cache_ttl: Duration::from_secs(2),
            min_spread_pct: min_spread,
        }
    }

    async fn fetch_pool_async(&self, dex_name: &str, pool_addr: Address) -> Option<PoolQuote> {
        let now = Instant::now();
        if let Some((_, _, quote, fetched)) = self.cache.borrow().iter().find(|(d,a,_,_)| d == dex_name && *a == pool_addr) {
            if now - *fetched < self.cache_ttl { return Some(quote.clone()); }
        }

        let pool = IUniswapV2Pair::new(pool_addr, self.provider.clone());
        let fetch_future = async {
            let (reserve0, reserve1, _) = pool.get_reserves().call().await?;
            let token0 = pool.token_0().call().await?;
            let token1 = pool.token_1().call().await?;
            Ok::<_, ethers::contract::ContractError<M>>(PoolQuote {
                dex_name: dex_name.to_string(), pool: pool_addr,
                reserve_token0: reserve0, reserve_token1: reserve1,
                token0, token1,
            })
        };

        match timeout(Duration::from_secs(2), fetch_future).await {
            Ok(Ok(quote)) => {
                self.cache.borrow_mut().push((dex_name.to_string(), pool_addr, quote.clone(), Instant::now()));
                Some(quote)
            }
            _ => None,
        }
    }

    fn implied_price(&self, q: &PoolQuote) -> f64 {
        let r0 = q.reserve_token0 as f64 / 10f64.powi(self.token0_decimals as i32);
        let r1 = q.reserve_token1 as f64 / 10f64.powi(self.token1_decimals as i32);
        if r0 == 0.0 { return 0.0; }
        r1 / r0
    }

    fn router_for_dex(&self, dex_name: &str) -> Option<Address> {
        self.pairs.iter().find_map(|p| p.routers.iter().find(|(n,_)| n == dex_name).map(|(_, a)| *a))
    }

    async fn quote(&self, router: Address, amount_in: u128, path: Vec<Address>) -> Option<u128> {
        let contract = IUniswapV2RouterQuote::new(router, self.provider.clone());
        contract.get_amounts_out(U256::from(amount_in), path).call().await
            .ok().and_then(|amounts| amounts.last().copied().map(|a| a.as_u128()))
    }

    pub async fn scan(&mut self) -> Vec<Opportunity> {
        let block_number = self.provider.get_block_number().await.ok().map(|n| n.as_u64());
        let mut raw = Vec::new();

        for pair in &self.pairs {
            if pair.pools.len() < 2 { continue; }

            let futures = pair.pools.iter().map(|(dex_name, addr)| {
                let dex = dex_name.clone();
                let addr = *addr;
                self.fetch_pool_async(&dex, addr)
            });
            let quotes: Vec<PoolQuote> = futures::future::join_all(futures).await.into_iter().filter_map(|q| q).collect();

            if quotes.len() < 2 { continue; }

            for i in 0..quotes.len() {
                for j in (i+1)..quotes.len() {
                    let p_i = self.implied_price(&quotes[i]);
                    let p_j = self.implied_price(&quotes[j]);
                    if p_i <= 0.0 || p_j <= 0.0 { continue; }
                    let (cheap, expensive, cheap_price, exp_price) = if p_i < p_j {
                        (&quotes[i], &quotes[j], p_i, p_j)
                    } else {
                        (&quotes[j], &quotes[i], p_j, p_i)
                    };
                    let spread_pct = (exp_price - cheap_price) / cheap_price * 100.0;
                    if spread_pct < self.min_spread_pct { continue; }
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

        raw.sort_by(|a,b| b.spread_pct.partial_cmp(&a.spread_pct).unwrap());
        let top = raw.into_iter().take(5);

        let mut validated = Vec::new();
        for opp in top {
            let buy_router = self.router_for_dex(&opp.buy_dex);
            let sell_router = self.router_for_dex(&opp.sell_dex);
            let (Some(buy_router), Some(sell_router)) = (buy_router, sell_router) else { continue; };
            let probe = 1_000 * 10u128.pow(self.token0_decimals);
            let path_out = vec![opp.token0, opp.token1];
            let buy_amount = match self.quote(buy_router, probe, path_out).await { Some(a) => a, None => continue; };
            let path_back = vec![opp.token1, opp.token0];
            let sell_amount = match self.quote(sell_router, buy_amount, path_back).await { Some(a) => a, None => continue; };
            if sell_amount <= probe { continue; }
            validated.push(opp);
        }
        validated
    }
}
EOF

cargo build --release
