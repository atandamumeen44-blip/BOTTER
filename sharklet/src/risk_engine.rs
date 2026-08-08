// src/risk_engine.rs
// Sits between Simulator and Executor. Every check must pass or the trade
// is dropped BEFORE it reaches the executor. This module owns no chain
// state of its own — it's fed live numbers by the caller each loop tick,
// which keeps it trivially unit-testable without a live RPC.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct RiskLimits {
    pub max_daily_loss_usd: f64,
    pub max_hourly_loss_usd: f64,
    pub max_trade_size_usd: f64,
    pub max_flash_loan_usd: f64,
    pub max_gas_spent_today_usd: f64,
    pub max_consecutive_failures: u32,
    pub wallet_min_balance_usd: f64,
    pub min_rpc_confidence: f64, // 0.0-1.0, from RPC scoreboard success rate
}

impl Default for RiskLimits {
    fn default() -> Self {
        RiskLimits {
            max_daily_loss_usd: 20.0,
            max_hourly_loss_usd: 8.0,
            max_trade_size_usd: 5000.0,
            max_flash_loan_usd: 5000.0,
            max_gas_spent_today_usd: 10.0,
            max_consecutive_failures: 3,
            wallet_min_balance_usd: 5.0, // must always have gas money left
            min_rpc_confidence: 0.90,
        }
    }
}

#[derive(Debug, Default)]
struct RollingWindow {
    entries: Vec<(u64, f64)>, // (unix_ts, amount)
}

impl RollingWindow {
    fn push(&mut self, amount: f64) {
        self.entries.push((now(), amount));
    }
    fn sum_within(&mut self, seconds: u64) -> f64 {
        let cutoff = now().saturating_sub(seconds);
        self.entries.retain(|(ts, _)| *ts >= cutoff);
        self.entries.iter().map(|(_, a)| a).sum()
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

pub struct RiskEngine {
    limits: RiskLimits,
    losses: RollingWindow,
    gas_spent: RollingWindow,
    consecutive_failures: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RiskDecision {
    Approve,
    Reject(&'static str),
}

impl RiskEngine {
    pub fn new(limits: RiskLimits) -> Self {
        RiskEngine {
            limits,
            losses: RollingWindow::default(),
            gas_spent: RollingWindow::default(),
            consecutive_failures: 0,
        }
    }

    /// Call this on every trade attempt BEFORE sending to the executor.
    pub fn evaluate(
        &mut self,
        trade_size_usd: f64,
        flash_loan_usd: f64,
        wallet_balance_usd: f64,
        rpc_confidence: f64,
    ) -> RiskDecision {
        if self.consecutive_failures >= self.limits.max_consecutive_failures {
            return RiskDecision::Reject("consecutive failure limit hit — circuit breaker should be open");
        }
        if trade_size_usd > self.limits.max_trade_size_usd {
            return RiskDecision::Reject("trade size exceeds max_trade_size_usd");
        }
        if flash_loan_usd > self.limits.max_flash_loan_usd {
            return RiskDecision::Reject("flash loan amount exceeds max_flash_loan_usd");
        }
        if wallet_balance_usd < self.limits.wallet_min_balance_usd {
            return RiskDecision::Reject("wallet balance below minimum — would risk stranding gas");
        }
        if rpc_confidence < self.limits.min_rpc_confidence {
            return RiskDecision::Reject("RPC confidence too low — stale reserve data risk");
        }
        if self.losses.sum_within(3600) >= self.limits.max_hourly_loss_usd {
            return RiskDecision::Reject("hourly loss cap hit");
        }
        if self.losses.sum_within(86400) >= self.limits.max_daily_loss_usd {
            return RiskDecision::Reject("daily loss cap hit");
        }
        if self.gas_spent.sum_within(86400) >= self.limits.max_gas_spent_today_usd {
            return RiskDecision::Reject("daily gas budget exhausted");
        }
        RiskDecision::Approve
    }

    pub fn record_result(&mut self, realized_pnl_usd: f64, gas_cost_usd: f64) {
        self.gas_spent.push(gas_cost_usd);
        if realized_pnl_usd < 0.0 {
            self.losses.push(-realized_pnl_usd);
            self.consecutive_failures += 1;
        } else {
            self.consecutive_failures = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_oversized_trade() {
        let mut re = RiskEngine::new(RiskLimits::default());
        let d = re.evaluate(10_000.0, 10_000.0, 50.0, 0.99);
        assert_eq!(d, RiskDecision::Reject("trade size exceeds max_trade_size_usd"));
    }

    #[test]
    fn trips_after_consecutive_failures() {
        let mut re = RiskEngine::new(RiskLimits::default());
        for _ in 0..3 {
            re.record_result(-1.0, 0.05);
        }
        let d = re.evaluate(100.0, 100.0, 50.0, 0.99);
        assert!(matches!(d, RiskDecision::Reject(_)));
    }
}// src/risk_engine.rs
//
//  ██████╗ ██╗███████╗██╗  ██╗   ███████╗███╗   ██╗ ██████╗ ██╗███╗   ██╗███████╗
//  ██╔══██╗██║██╔════╝██║ ██╔╝   ██╔════╝████╗  ██║██╔════╝ ██║████╗  ██║██╔════╝
//  ██████╔╝██║███████╗█████╔╝    █████╗  ██╔██╗ ██║██║  ███╗██║██╔██╗ ██║█████╗
//  ██╔══██╗██║╚════██║██╔═██╗    ██╔══╝  ██║╚██╗██║██║   ██║██║██║╚██╗██║██╔══╝
//  ██║  ██║██║███████║██║  ██╗██╗███████╗██║ ╚████║╚██████╔╝██║██║ ╚████║███████╗
//  ╚═╝  ╚═╝╚═╝╚══════╝╚═╝  ╚═╝╚═╝╚══════╝╚═╝  ╚═══╝ ╚═════╝ ╚═╝╚═╝  ╚═══╝╚══════╝
//
//  PRODUCTION-GRADE RISK ENGINE — SITS BETWEEN SIMULATOR AND EXECUTOR.
//
//  Every check must pass or the trade is dropped BEFORE it reaches the
//  executor.  This module owns no chain state of its own — it's fed live
//  numbers each loop tick, which keeps it trivially unit-testable without
//  a live RPC.
//
//  Loads limits from environment variables by default.  If a `bot.toml`
//  file is present, its values override the env (TOML wins).  Call
//  `RiskEngine::from_toml("bot.toml")` at startup to load from file.
//  Or just use `RiskEngine::default()` for quick env-only setup.
//
//  Adaptive limits scale trade size, flash loan, and gas budget based on
//  wallet balance.  Define tiers in config and the engine picks the right
//  one automatically.

use std::time::{SystemTime, UNIX_EPOCH};

// ─── CONFIGURATION STRUCTURES ──────────────────────────────────────────────

/// All limits, loaded from env or TOML.  Every field has a default so
/// you can omit any key and the bot still runs safely.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct RiskConfig {
    pub max_daily_loss_usd: f64,
    pub max_hourly_loss_usd: f64,
    pub max_trade_size_usd: f64,
    pub max_flash_loan_usd: f64,
    pub max_gas_spent_today_usd: f64,
    pub max_consecutive_failures: u32,
    pub wallet_min_balance_usd: f64,
    pub min_rpc_confidence: f64,       // 0.0–1.0

    /// Seconds to pause trading when the circuit breaker trips.
    pub circuit_breaker_pause_secs: u64,
    /// Failures older than this many seconds are ignored.
    pub failure_reset_window_secs: u64,

    /// Enable adaptive limits (trade/flash/gas scale with wallet).
    pub adaptive_limits: bool,
    /// Tiers: (wallet_balance_ceiling, max_trade, max_flash_loan, max_gas_spent)
    /// Sorted ascending by wallet ceiling.  If balance <= ceiling, that tier
    /// is used; otherwise the next one is tried.
    pub adaptive_tiers: Vec<(f64, f64, f64, f64)>,
}

impl Default for RiskConfig {
    fn default() -> Self {
        RiskConfig {
            max_daily_loss_usd:        env_or("RISK_MAX_DAILY_LOSS_USD", 20.0),
            max_hourly_loss_usd:       env_or("RISK_MAX_HOURLY_LOSS_USD", 8.0),
            max_trade_size_usd:        env_or("RISK_MAX_TRADE_SIZE_USD", 5_000.0),
            max_flash_loan_usd:        env_or("RISK_MAX_FLASH_LOAN_USD", 5_000.0),
            max_gas_spent_today_usd:   env_or("RISK_MAX_GAS_SPENT_TODAY_USD", 10.0),
            max_consecutive_failures:  env_or("RISK_MAX_CONSECUTIVE_FAILURES", 3),
            wallet_min_balance_usd:    env_or("RISK_WALLET_MIN_BALANCE_USD", 5.0),
            min_rpc_confidence:        env_or("RISK_MIN_RPC_CONFIDENCE", 0.90),

            circuit_breaker_pause_secs: env_or("RISK_CB_PAUSE_SECS", 300),
            failure_reset_window_secs:  env_or("RISK_FAILURE_RESET_SECS", 3_600),

            adaptive_limits: false,
            adaptive_tiers: vec![],
        }
    }
}

/// Helper to read an env var and parse it, falling back to `default`.
fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}

// ─── ROLLING WINDOW (memory‑safe) ──────────────────────────────────────────

#[derive(Debug)]
struct RollingWindow {
    entries: Vec<(u64, f64)>, // (timestamp, value)
}

impl RollingWindow {
    fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Record a new value and immediately prune entries older than 25h.
    fn push(&mut self, amount: f64) {
        let now = now_secs();
        self.entries.push((now, amount));
        self.prune(now.saturating_sub(90_000)); // 25h
    }

    /// Sum of all values within the last `seconds`.
    fn sum_within(&self, seconds: u64) -> f64 {
        let cutoff = now_secs().saturating_sub(seconds);
        self.entries
            .iter()
            .filter(|(ts, _)| *ts >= cutoff)
            .map(|(_, v)| v)
            .sum()
    }

    fn prune(&mut self, cutoff: u64) {
        self.entries.retain(|(ts, _)| *ts >= cutoff);
    }
}

// ─── FAILURE TRACKER WITH RESET WINDOW ─────────────────────────────────────

#[derive(Debug)]
struct FailureTracker {
    timestamps: Vec<u64>,
}

impl FailureTracker {
    fn new() -> Self {
        Self { timestamps: Vec::new() }
    }

    fn record_failure(&mut self) {
        self.timestamps.push(now_secs());
        self.prune_old();
    }

    fn record_success(&mut self) {
        self.timestamps.clear();
    }

    /// Number of failures within the configured reset window.
    fn consecutive(&self, window_secs: u64) -> u32 {
        let cutoff = now_secs().saturating_sub(window_secs);
        self.timestamps.iter().filter(|&&ts| ts >= cutoff).count() as u32
    }

    fn prune_old(&mut self) {
        let cutoff = now_secs().saturating_sub(90_000); // 25h
        self.timestamps.retain(|&ts| ts >= cutoff);
    }
}

/// Current Unix timestamp (seconds).
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ─── THE RISK ENGINE ───────────────────────────────────────────────────────

pub struct RiskEngine {
    limits: RiskConfig,
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
    // ── constructors ──────────────────────────────────────────────────

    /// Create from a `RiskConfig` (e.g., loaded from TOML).
    pub fn new(cfg: RiskConfig) -> Self {
        Self {
            limits: cfg,
            losses: RollingWindow::new(),
            gas_spent: RollingWindow::new(),
            failures: FailureTracker::new(),
            paused_until: 0,
        }
    }

    /// Quick initialisation using only environment variables.
    pub fn default() -> Self {
        Self::new(RiskConfig::default())
    }

    /// Load from a TOML file.  Falls back to env defaults if the file
    /// doesn't exist or can't be parsed.
    pub fn from_toml(path: &str) -> Self {
        let cfg = std::fs::read_to_string(path)
            .ok()
            .and_then(|contents| toml::from_str::<RiskConfig>(&contents).ok())
            .unwrap_or_default();
        Self::new(cfg)
    }

    // ── core evaluate method ──────────────────────────────────────────

    /// Call this on every trade attempt BEFORE sending to the executor.
    /// Returns `Approve`, `Reject(reason)`, or `Paused`.
    pub fn evaluate(
        &mut self,
        trade_size_usd: f64,
        flash_loan_usd: f64,
        wallet_balance_usd: f64,
        rpc_confidence: f64,
    ) -> RiskDecision {
        // 1. Circuit breaker pause
        if self.is_paused() {
            return RiskDecision::Paused;
        }

        // 2. Consecutive failures (with reset window)
        let failures = self.failures.consecutive(self.limits.failure_reset_window_secs);
        if failures >= self.limits.max_consecutive_failures {
            self.activate_pause();
            return RiskDecision::Reject(format!(
                "circuit breaker tripped — {} failures in {} secs",
                failures, self.limits.failure_reset_window_secs
            ));
        }

        // 3. Compute adaptive limits if enabled
        let (max_trade, max_loan, max_gas_today) = if self.limits.adaptive_limits {
            self.adaptive_limits(wallet_balance_usd)
        } else {
            (
                self.limits.max_trade_size_usd,
                self.limits.max_flash_loan_usd,
                self.limits.max_gas_spent_today_usd,
            )
        };

        // 4. Static checks with detailed rejection reasons
        if trade_size_usd > max_trade {
            return RiskDecision::Reject(format!(
                "trade size ${:.0} > max ${:.0}",
                trade_size_usd, max_trade
            ));
        }
        if flash_loan_usd > max_loan {
            return RiskDecision::Reject(format!(
                "flash loan ${:.0} > max ${:.0}",
                flash_loan_usd, max_loan
            ));
        }
        if wallet_balance_usd < self.limits.wallet_min_balance_usd {
            return RiskDecision::Reject(format!(
                "wallet ${:.2} < min ${:.2}",
                wallet_balance_usd, self.limits.wallet_min_balance_usd
            ));
        }
        if rpc_confidence < self.limits.min_rpc_confidence {
            return RiskDecision::Reject(format!(
                "RPC confidence {:.2} < min {:.2}",
                rpc_confidence, self.limits.min_rpc_confidence
            ));
        }

        // 5. Loss limits (rolling windows)
        let hourly_loss = self.losses.sum_within(3_600);
        if hourly_loss >= self.limits.max_hourly_loss_usd {
            return RiskDecision::Reject(format!(
                "hourly loss ${:.2} >= limit ${:.2}",
                hourly_loss, self.limits.max_hourly_loss_usd
            ));
        }
        let daily_loss = self.losses.sum_within(86_400);
        if daily_loss >= self.limits.max_daily_loss_usd {
            return RiskDecision::Reject(format!(
                "daily loss ${:.2} >= limit ${:.2}",
                daily_loss, self.limits.max_daily_loss_usd
            ));
        }
        let gas_today = self.gas_spent.sum_within(86_400);
        if gas_today >= max_gas_today {
            return RiskDecision::Reject(format!(
                "daily gas ${:.2} >= limit ${:.2}",
                gas_today, max_gas_today
            ));
        }

        RiskDecision::Approve
    }

    // ── result recording ──────────────────────────────────────────────

    /// Call after each trade (success or failure) to update running totals.
    pub fn record_result(&mut self, realized_pnl_usd: f64, gas_cost_usd: f64) {
        self.gas_spent.push(gas_cost_usd);
        if realized_pnl_usd < 0.0 {
            self.losses.push(-realized_pnl_usd);
            self.failures.record_failure();
        } else {
            // Profit resets the failure counter (you can remove this if you
            // prefer the streak to persist, but usually a win means the
            // strategy isn't broken).
            self.failures.record_success();
        }
    }

    // ── circuit breaker controls ──────────────────────────────────────

    fn is_paused(&self) -> bool {
        now_secs() < self.paused_until
    }

    fn activate_pause(&mut self) {
        self.paused_until = now_secs() + self.limits.circuit_breaker_pause_secs;
        eprintln!(
            "⏸️  CIRCUIT BREAKER PAUSED for {} secs",
            self.limits.circuit_breaker_pause_secs
        );
    }

    /// Manually unpause the engine (e.g., after you've fixed a bug).
    pub fn resume(&mut self) {
        self.paused_until = 0;
        self.failures.timestamps.clear();
        eprintln!("▶️  Circuit breaker manually resumed");
    }

    /// Fully reset all counters and pause state.
    pub fn full_reset(&mut self) {
        *self = Self::new(self.limits.clone());
        eprintln!("🔄 Risk engine fully reset");
    }

    // ── adaptive limits helper ────────────────────────────────────────

    fn adaptive_limits(&self, wallet: f64) -> (f64, f64, f64) {
        for &(ceiling, trade, loan, gas) in &self.limits.adaptive_tiers {
            if wallet <= ceiling {
                return (trade, loan, gas);
            }
        }
        // fallback to base limits
        (
            self.limits.max_trade_size_usd,
            self.limits.max_flash_loan_usd,
            self.limits.max_gas_spent_today_usd,
        )
    }

    // ── live limit reloading ──────────────────────────────────────────

    /// Reload limits from a fresh `RiskConfig` without losing runtime state.
    /// This allows hot‑reloading your `bot.toml` without restarting the bot.
    pub fn update_limits(&mut self, new_cfg: RiskConfig) {
        self.limits = new_cfg;
    }
}

// ─── TESTS ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_trade_rejected() {
        let mut re = RiskEngine::default();
        let d = re.evaluate(10_000.0, 10_000.0, 50.0, 0.99);
        assert!(matches!(d, RiskDecision::Reject(_)));
    }

    #[test]
    fn circuit_breaker_trips_after_failures() {
        let mut re = RiskEngine::default();
        for _ in 0..3 {
            re.record_result(-1.0, 0.05);
        }
        let d = re.evaluate(100.0, 100.0, 50.0, 0.99);
        // Should either reject or pause
        assert!(d != RiskDecision::Approve);
    }

    #[test]
    fn success_resets_failures() {
        let mut re = RiskEngine::default();
        for _ in 0..3 {
            re.record_result(-1.0, 0.05);
        }
        re.record_result(1.0, 0.05); // win resets
        let d = re.evaluate(100.0, 100.0, 50.0, 0.99);
        assert_eq!(d, RiskDecision::Approve);
    }

    #[test]
    fn adaptive_tiers_work() {
        let mut cfg = RiskConfig::default();
        cfg.adaptive_limits = true;
        cfg.adaptive_tiers = vec![
            (100.0, 1_000.0, 1_000.0, 5.0),
            (1_000.0, 5_000.0, 5_000.0, 10.0),
        ];
        let mut re = RiskEngine::new(cfg);

        // wallet $50 → first tier
        assert!(matches!(
            re.evaluate(2_000.0, 2_000.0, 50.0, 0.99),
            RiskDecision::Reject(_)
        ));
        assert_eq!(
            re.evaluate(800.0, 800.0, 50.0, 0.99),
            RiskDecision::Approve
        );

        // wallet $500 → second tier
        assert_eq!(
            re.evaluate(4_500.0, 4_500.0, 500.0, 0.99),
            RiskDecision::Approve
        );
    }
}