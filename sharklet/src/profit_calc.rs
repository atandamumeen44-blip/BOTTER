// src/profit_calc.rs
//
//  ██████╗ ██████╗  ██████╗ ███████╗██╗████████╗      ██████╗ █████╗ ██╗      ██████╗
//  ██╔══██╗██╔══██╗██╔═══██╗██╔════╝██║╚══██╔══╝     ██╔════╝██╔══██╗██║     ██╔════╝
//  ██████╔╝██████╔╝██║   ██║█████╗  ██║   ██║        ██║     ███████║██║     ██║
//  ██╔═══╝ ██╔══██╗██║   ██║██╔══╝  ██║   ██║        ██║     ██╔══██║██║     ██║
//  ██║     ██║  ██║╚██████╔╝██║     ██║   ██║        ╚██████╗██║  ██║███████╗╚██████╗
//  ╚═╝     ╚═╝  ╚═╝ ╚═════╝ ╚═╝     ╚═╝   ╚═╝         ╚═════╝╚═╝  ╚═╝╚══════╝ ╚═════╝
//
//  Pure‑math profit evaluator. No network calls, only deterministic arithmetic.
//  Adaptive profit tiers are loaded from config/bot.toml (or fall back to defaults).

use crate::scanner::Opportunity;
use tracing::debug;

// ── Cost model & profit tiers ────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CostModel {
    pub flash_fee_bps: f64,     // e.g. 9 = 0.09%
    pub gas_cost_usd: f64,
    pub safety_margin_usd: f64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AdaptiveProfitTier {
    pub wallet_ceiling_usd: f64,
    pub min_profit_usd: f64,
}

/// Load tiers from `config/bot.toml`. Falls back to sensible defaults if
/// the file is missing or the key is absent.
pub fn default_tiers() -> Vec<AdaptiveProfitTier> {
    // Try to read from config file
    if let Ok(tiers) = load_tiers_from_config() {
        debug!("Loaded {} adaptive profit tiers from config/bot.toml", tiers.len());
        return tiers;
    }

    // Sensible defaults that work across testnet and mainnet
    vec![
        AdaptiveProfitTier { wallet_ceiling_usd: 100.0,      min_profit_usd: 1.0 },
        AdaptiveProfitTier { wallet_ceiling_usd: 500.0,      min_profit_usd: 2.0 },
        AdaptiveProfitTier { wallet_ceiling_usd: 2_000.0,    min_profit_usd: 5.0 },
        AdaptiveProfitTier { wallet_ceiling_usd: 10_000.0,   min_profit_usd: 10.0 },
        AdaptiveProfitTier { wallet_ceiling_usd: f64::INFINITY, min_profit_usd: 25.0 },
    ]
}

fn load_tiers_from_config() -> Result<Vec<AdaptiveProfitTier>, Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string("config/bot.toml")?;
    let cfg: toml::Value = toml::from_str(&s)?;
    if let Some(tiers) = cfg.get("adaptive_tiers") {
        let tiers: Vec<AdaptiveProfitTier> = toml::Value::try_into(tiers.clone())?;
        Ok(tiers)
    } else {
        Err("no adaptive_tiers key in config".into())
    }
}

pub fn adaptive_min_profit(wallet_balance_usd: f64, tiers: &[AdaptiveProfitTier]) -> f64 {
    tiers
        .iter()
        .find(|t| wallet_balance_usd <= t.wallet_ceiling_usd)
        .map(|t| t.min_profit_usd)
        .unwrap_or(25.0)
}

// ── Decision structures ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SizedTrade {
    pub size_usd: f64,
    pub gross_profit_usd: f64,
    pub net_profit_usd: f64,
    pub slippage_cost_usd: f64,
}

#[derive(Debug, Clone)]
pub enum ProfitDecision {
    Go(SizedTrade),
    NoGo { reason: String, best_net_profit_usd: f64 },
}

// ── Core evaluator ──────────────────────────────────────────────────────

/// Evaluate an arbitrage opportunity and return a go/no‑go decision with
/// an optimally sized trade.
///
/// The sizing model uses the closed‑form optimum for a constant‑product AMM:
/// `size_opt = spread * shallow_pool_depth`. We then clamp to `max_trade_size_usd`
/// and to 25% of the shallow pool to avoid excessive price impact.
pub fn size_and_evaluate(
    opp: &Opportunity,
    costs: &CostModel,
    max_trade_size_usd: f64,
    wallet_balance_usd: f64,
    tiers: &[AdaptiveProfitTier],
) -> ProfitDecision {
    let spread_frac = opp.spread_pct / 100.0;
    let shallow_depth = opp.buy_pool_depth.min(opp.sell_pool_depth);

    // Quick reject on non‑positive spread
    if spread_frac <= 0.0 {
        return ProfitDecision::NoGo {
            reason: "non‑positive spread".into(),
            best_net_profit_usd: 0.0,
        };
    }

    // Closed‑form optimal size: size = spread * depth (maximises profit)
    let unclamped_size = spread_frac * shallow_depth;
    let size = unclamped_size
        .min(max_trade_size_usd)
        .min(shallow_depth * 0.25) // never take more than 25% of the shallow pool
        .max(0.0);

    if size <= 0.0 {
        return ProfitDecision::NoGo {
            reason: "computed size is zero".into(),
            best_net_profit_usd: 0.0,
        };
    }

    let gross_profit = size * spread_frac;
    let flash_fee_cost = size * (costs.flash_fee_bps / 10_000.0);
    let slippage_cost = (size * size) / (2.0 * shallow_depth.max(1.0));
    let net_profit = gross_profit - flash_fee_cost - costs.gas_cost_usd - slippage_cost - costs.safety_margin_usd;

    let min_required = adaptive_min_profit(wallet_balance_usd, tiers);

    debug!(
        "Profit eval for {}: spread={:.4}% size=${:.0} gross=${:.4} net=${:.4} min=${:.4}",
        opp.label, opp.spread_pct, size, gross_profit, net_profit, min_required
    );

    if net_profit < min_required {
        return ProfitDecision::NoGo {
            reason: format!(
                "net profit ${:.4} < required ${:.4} (tier for wallet <= ${:.0})",
                net_profit, min_required, wallet_balance_usd
            ),
            best_net_profit_usd: net_profit,
        };
    }

    ProfitDecision::Go(SizedTrade {
        size_usd: size,
        gross_profit_usd: gross_profit,
        net_profit_usd: net_profit,
        slippage_cost_usd: slippage_cost,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::Opportunity;
    use ethers::types::Address;

    fn dummy_opp(spread_pct: f64, depth: f64) -> Opportunity {
        Opportunity {
            label: "USDC/WETH".into(),
            buy_dex: "uniswap".into(),
            sell_dex: "sushiswap".into(),
            buy_price: 1.0,
            sell_price: 1.0 + spread_pct / 100.0,
            spread_pct,
            buy_pool_depth: depth,
            sell_pool_depth: depth,
            token0: Address::zero(),
            token1: Address::zero(),
            block_number: None,   // added for the enterprise scanner
        }
    }

    #[test]
    fn rejects_thin_spread() {
        let opp = dummy_opp(0.02, 200_000.0);
        let costs = CostModel { flash_fee_bps: 9.0, gas_cost_usd: 0.05, safety_margin_usd: 0.30 };
        let d = size_and_evaluate(&opp, &costs, 5000.0, 50.0, &default_tiers());
        assert!(matches!(d, ProfitDecision::NoGo { .. }));
    }

    #[test]
    fn approves_healthy_spread() {
        let opp = dummy_opp(1.5, 200_000.0);
        let costs = CostModel { flash_fee_bps: 9.0, gas_cost_usd: 0.05, safety_margin_usd: 0.30 };
        let d = size_and_evaluate(&opp, &costs, 5000.0, 50.0, &default_tiers());
        assert!(matches!(d, ProfitDecision::Go(_)));
    }

    #[test]
    fn respects_max_trade_size() {
        let opp = dummy_opp(0.5, 1_000_000.0);
        let costs = CostModel { flash_fee_bps: 9.0, gas_cost_usd: 0.05, safety_margin_usd: 0.30 };
        let d = size_and_evaluate(&opp, &costs, 1000.0, 50.0, &default_tiers());
        if let ProfitDecision::Go(s) = d {
            assert!(s.size_usd <= 1000.0);
        } else {
            panic!("Expected Go");
        }
    }

    #[test]
    fn adaptive_tiers_work() {
        let opp = dummy_opp(0.5, 1_000_000.0);
        let costs = CostModel { flash_fee_bps: 9.0, gas_cost_usd: 0.05, safety_margin_usd: 0.30 };
        let tiers = vec![
            AdaptiveProfitTier { wallet_ceiling_usd: 10.0, min_profit_usd: 100.0 },
            AdaptiveProfitTier { wallet_ceiling_usd: f64::INFINITY, min_profit_usd: 1.0 },
        ];
        // Low wallet -> high minimum -> reject
        assert!(matches!(size_and_evaluate(&opp, &costs, 5000.0, 5.0, &tiers), ProfitDecision::NoGo{..}));
        // Higher wallet -> low minimum -> approve
        assert!(matches!(size_and_evaluate(&opp, &costs, 5000.0, 100.0, &tiers), ProfitDecision::Go(_)));
    }
}