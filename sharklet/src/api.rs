// src/api.rs — CORRECTED VERSION
use warp::Filter;
use std::sync::Arc;
use serde_json::json;

pub struct AppState {
    pub health: bool,
}

pub async fn start_server(state: Arc<AppState>, db: Arc<crate::logger::Logger>) {
    let health = warp::path("health").map(|| warp::reply::json(&json!({"status": "ok"})));

    let stats = warp::path!("api" / "stats")
        .and_then(move || async {
            Ok::<_, warp::Rejection>(warp::reply::json(&json!({
                "wallet_usd": 0.0,
                "today_pnl": 0.0,
                "total_pnl": 0.0,
                "trades_today": 0,
                "gas_gwei": 0,
                "queue_size": 0,
                "profit_series": []
            })))
        });

    let routes = health.or(stats);
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}
