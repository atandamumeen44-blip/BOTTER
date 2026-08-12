use warp::Filter;
use std::sync::Arc;
use serde_json::json;

pub struct AppState {
    pub health: bool,
}

pub async fn start_server(_state: Arc<AppState>, _db: Arc<crate::logger::Logger>) {
    let health = warp::path("health").map(|| warp::reply::json(&json!({"status": "ok"})));

    let routes = health;
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}
