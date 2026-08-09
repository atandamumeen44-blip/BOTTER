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
        let best = &opportunities[0];   // <-- FIXED indexing

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
