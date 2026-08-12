use warp::Filter;
use std::sync::Arc;
use serde_json::json;

pub struct AppState {
    pub health: bool,
}

pub async fn start_server(_state: Arc<AppState>, _db: Arc<crate::logger::Logger>) {
    let health = warp::path("health").map(|| warp::reply::json(&json!({"status": "ok"})));

    // Stats endpoint (mock data for now)
    let stats = warp::path!("api" / "stats").map(|| {
        warp::reply::json(&json!({
            "wallet_usd": 0.0,
            "today_pnl": 0.0,
            "total_pnl": 0.0,
            "trades_today": 0,
            "gas_gwei": 31,
            "queue_size": 0,
            "profit_series": [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        }))
    });

    // Trades endpoint
    let trades = warp::path!("api" / "trades").map(|| {
        warp::reply::json(&json!([]))
    });

    // Dashboard HTML
    let dashboard = warp::path::end().map(|| {
        warp::reply::html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>🦈 Sharklet Dashboard</title>
    <meta charset="UTF-8">
    <style>
        * { margin:0; padding:0; box-sizing:border-box; }
        body { background:#0a0d10; color:#d4d4d4; font-family:monospace; padding:2rem; }
        .container { max-width:1000px; margin:0 auto; }
        h1 { color:#4facfe; font-size:2rem; margin-bottom:1rem; }
        .sub { color:#666; margin-bottom:2rem; }
        .grid { display:grid; grid-template-columns:repeat(auto-fit,minmax(180px,1fr)); gap:1rem; margin-bottom:2rem; }
        .card { background:#0d1117; border:1px solid #1a2028; border-radius:10px; padding:1.2rem; text-align:center; }
        .card .label { font-size:0.7rem; color:#888; text-transform:uppercase; }
        .card .value { font-size:1.8rem; font-weight:700; margin-top:0.3rem; }
        .green { color:#4caf50; }
        .blue { color:#4facfe; }
        .yellow { color:#ffc107; }
        .live { display:inline-block; width:10px; height:10px; background:#4caf50; border-radius:50%; margin-left:10px; animation:pulse 1.5s infinite; }
        @keyframes pulse { 0%{opacity:1;}50%{opacity:.3;}100%{opacity:1;} }
        .footer { margin-top:2rem; color:#444; font-size:0.7rem; text-align:center; border-top:1px solid #1a2028; padding-top:1rem; }
    </style>
</head>
<body>
<div class="container">
    <h1>🦈 Sharklet <span class="live"></span></h1>
    <div class="sub">Live arbitrage dashboard – data refreshes every 3 seconds</div>
    <div class="grid">
        <div class="card"><div class="label">Today's Profit</div><div class="value green" id="today">$0.00</div></div>
        <div class="card"><div class="label">Total Profit</div><div class="value blue" id="total">$0.00</div></div>
        <div class="card"><div class="label">Trades</div><div class="value" id="trades">0</div></div>
        <div class="card"><div class="label">Status</div><div class="value yellow" id="status">Hunting</div></div>
    </div>
    <div class="footer">🦈 Sharklet v2 · Built for the hunt</div>
</div>
<script>
async function refresh() {
    try {
        const res = await fetch('/api/stats');
        const data = await res.json();
        document.getElementById('today').textContent = '$' + data.today_pnl.toFixed(2);
        document.getElementById('total').textContent = '$' + data.total_pnl.toFixed(2);
        document.getElementById('trades').textContent = data.trades_today;
    } catch(e) {}
}
refresh();
setInterval(refresh, 3000);
</script>
</body>
</html>
        "#)
    });

    let routes = dashboard.or(health).or(stats).or(trades);
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}
