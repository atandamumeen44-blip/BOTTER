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
