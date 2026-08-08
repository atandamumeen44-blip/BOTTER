// src/api.rs
//
//  Lightweight health and metrics API server.
//  – /health → returns {"status": "ok"}
//  – /metrics → returns basic bot stats
//  – Used by Railway/Kubernetes for health checks

use serde_json::json;
use warp::Filter;
use std::sync::Arc;

pub struct AppState {
    pub health: bool,
    pub uptime_seconds: u64,
}

pub async fn start_server(state: Arc<AppState>) {
    let state_filter = warp::any().map(move || state.clone());

    let health = warp::path("health")
        .map(|| warp::reply::json(&json!({"status": "ok"})));

    let metrics = warp::path("metrics")
        .and(state_filter)
        .and_then(|state: Arc<AppState>| async move {
            Ok::<_, warp::Rejection>(warp::reply::json(&json!({
                "status": "ok",
                "uptime_seconds": state.uptime_seconds,
            })))
        });

    let root = warp::path::end().map(|| {
        warp::reply::html(
            r#"<!DOCTYPE html>
<html>
<head><title>🦈 Sharklet</title></head>
<body style="background:#0a0d10;color:#d4d4d4;font-family:monospace;padding:2rem;">
<h1>🦈 Sharklet v2</h1>
<p>Bot is running.</p>
<p><a href="/health" style="color:#4facfe;">/health</a> | <a href="/metrics" style="color:#4facfe;">/metrics</a></p>
</body>
</html>"#,
        )
    });

    let routes = root.or(health).or(metrics);
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}