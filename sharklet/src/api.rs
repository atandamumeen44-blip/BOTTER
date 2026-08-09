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
        warp::reply::html(include_str!("../dashboard/index.html"))
    });

    let routes = dashboard.or(health).or(stats).or(trades);
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}
