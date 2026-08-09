// src/api.rs
use serde_json::json;
use warp::Filter;
use std::sync::Arc;

#[derive(Default)]
pub struct AppState {
    pub health: bool,
}

pub async fn start_server(state: Arc<AppState>) {
    let health = warp::path("health")
        .map(|| warp::reply::json(&json!({"status": "ok"})));
    warp::serve(health).run(([0, 0, 0, 0], 8080)).await;
}
