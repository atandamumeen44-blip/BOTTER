use warp::Filter;
use std::sync::Arc;
use serde_json::json;

pub struct AppState {
    pub health: bool,
}

pub async fn start_server(_state: Arc<AppState>, _db: Arc<crate::logger::Logger>) {
    // Health check
    let health = warp::path("health").map(|| warp::reply::json(&json!({"status": "ok"})));

    // Stats API
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

    // Trades API
    let trades = warp::path!("api" / "trades").map(|| {
        warp::reply::json(&json!([]))
    });

    // Modern Dashboard HTML
    let dashboard = warp::path::end().map(|| {
        warp::reply::html(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>🦈 Sharklet – Dashboard</title>
    <style>
        * { margin:0; padding:0; box-sizing:border-box; }
        body {
            background: #0a0d10;
            color: #d4d4d4;
            font-family: 'Inter', -apple-system, 'Segoe UI', system-ui, sans-serif;
            min-height: 100vh;
            padding: 2rem;
        }
        .container { max-width: 1200px; margin: 0 auto; }

        /* Header */
        .header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 2rem;
            flex-wrap: wrap;
            gap: 1rem;
        }
        .header h1 {
            font-size: 2rem;
            font-weight: 700;
            background: linear-gradient(135deg, #4facfe, #00f2fe);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }
        .header .sub {
            color: #666;
            font-size: 0.85rem;
        }
        .status-badge {
            display: flex;
            align-items: center;
            gap: 0.5rem;
            background: #0d1117;
            border: 1px solid #1a2028;
            border-radius: 30px;
            padding: 0.4rem 1rem;
            font-size: 0.8rem;
        }
        .status-dot {
            width: 8px;
            height: 8px;
            border-radius: 50%;
            background: #4caf50;
            animation: pulse 1.5s infinite;
        }
        @keyframes pulse { 0%{opacity:1;}50%{opacity:.3;}100%{opacity:1;} }

        /* Stats Grid */
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
            gap: 1rem;
            margin-bottom: 2rem;
        }
        .stat-card {
            background: #0d1117;
            border: 1px solid #1a2028;
            border-radius: 12px;
            padding: 1.2rem;
            text-align: center;
            transition: border-color 0.2s;
        }
        .stat-card:hover { border-color: #2a3a50; }
        .stat-card .label {
            font-size: 0.65rem;
            text-transform: uppercase;
            letter-spacing: 0.8px;
            color: #888;
        }
        .stat-card .value {
            font-size: 1.8rem;
            font-weight: 700;
            margin-top: 0.3rem;
        }
        .stat-card .value.green { color: #4caf50; }
        .stat-card .value.blue { color: #4facfe; }
        .stat-card .value.yellow { color: #ffc107; }
        .stat-card .value.red { color: #ff5c5c; }
        .stat-card .value.white { color: #d4d4d4; }

        /* Chart placeholder */
        .chart-container {
            background: #0d1117;
            border: 1px solid #1a2028;
            border-radius: 12px;
            padding: 1.5rem;
            margin-bottom: 2rem;
        }
        .chart-container h3 {
            font-size: 0.8rem;
            text-transform: uppercase;
            letter-spacing: 0.8px;
            color: #888;
            margin-bottom: 1rem;
        }
        .chart-placeholder {
            height: 120px;
            display: flex;
            align-items: flex-end;
            gap: 4px;
            padding: 0.5rem 0;
        }
        .chart-bar {
            flex: 1;
            background: linear-gradient(180deg, #4facfe, #4facfe40);
            border-radius: 2px 2px 0 0;
            min-height: 4px;
            transition: height 0.3s;
            height: 4px;
        }
        .chart-bar.active { background: linear-gradient(180deg, #ffc107, #ffc10740); }

        /* Trade Log */
        .trade-log {
            background: #0d1117;
            border: 1px solid #1a2028;
            border-radius: 12px;
            padding: 1.2rem;
        }
        .trade-log h3 {
            font-size: 0.8rem;
            text-transform: uppercase;
            letter-spacing: 0.8px;
            color: #888;
            margin-bottom: 1rem;
        }
        .trade-log table {
            width: 100%;
            border-collapse: collapse;
            font-size: 0.8rem;
        }
        .trade-log th {
            text-align: left;
            color: #666;
            font-weight: 500;
            padding-bottom: 0.5rem;
            border-bottom: 1px solid #1a2028;
        }
        .trade-log td {
            padding: 0.5rem 0;
            border-bottom: 1px solid #1a2028;
            color: #888;
        }
        .trade-log tr:last-child td { border-bottom: none; }
        .trade-log .status {
            padding: 0.15rem 0.6rem;
            border-radius: 12px;
            font-size: 0.65rem;
            font-weight: 600;
        }
        .trade-log .status.executed { background: #1a2f1a; color: #4caf50; }
        .trade-log .status.skipped { background: #2a1f00; color: #ffc107; }
        .trade-log .status.failed { background: #2f1a1a; color: #ff5c5c; }
        .trade-log .profit { color: #4caf50; }
        .trade-log .loss { color: #ff5c5c; }
        .empty-state {
            text-align: center;
            color: #555;
            padding: 2rem 0;
        }

        footer {
            text-align: center;
            margin-top: 2rem;
            color: #444;
            font-size: 0.7rem;
            border-top: 1px solid #1a2028;
            padding-top: 1rem;
        }
        @media (max-width: 600px) {
            body { padding: 1rem; }
            .stats-grid { grid-template-columns: repeat(2, 1fr); }
            .trade-log table { font-size: 0.65rem; }
        }
    </style>
</head>
<body>
<div class="container">
    <div class="header">
        <div>
            <h1>🦈 Sharklet</h1>
            <span class="sub">Live Arbitrage Dashboard</span>
        </div>
        <div class="status-badge">
            <span class="status-dot"></span>
            <span id="statusText">Hunting</span>
            <span style="color:#555;margin:0 0.3rem;">·</span>
            <span id="uptime" style="color:#555;">00:00:00</span>
        </div>
    </div>

    <div class="stats-grid" id="stats">
        <div class="stat-card"><div class="label">Today's Profit</div><div class="value green" id="today">$0.00</div></div>
        <div class="stat-card"><div class="label">Total Profit</div><div class="value blue" id="total">$0.00</div></div>
        <div class="stat-card"><div class="label">Trades</div><div class="value white" id="trades">0</div></div>
        <div class="stat-card"><div class="label">Gas (Gwei)</div><div class="value yellow" id="gas">0</div></div>
        <div class="stat-card"><div class="label">Wallet</div><div class="value white" id="wallet">$0.00</div></div>
        <div class="stat-card"><div class="label">Queue</div><div class="value white" id="queue">0</div></div>
    </div>

    <div class="chart-container">
        <h3>📈 24h Profit Curve</h3>
        <div class="chart-placeholder" id="chart"></div>
    </div>

    <div class="trade-log">
        <h3>📋 Recent Trades</h3>
        <table>
            <thead><tr><th>Time</th><th>Pair</th><th>Route</th><th>Spread</th><th>Net</th><th>Status</th></tr></thead>
            <tbody id="tradesBody"></tbody>
        </table>
        <div class="empty-state" id="emptyState">No trades yet — bot is hunting</div>
    </div>

    <footer>🦈 Sharklet v2 · Data refreshes every 3 seconds</footer>
</div>

<script>
(function() {
    const $ = id => document.getElementById(id);
    const start = Date.now();

    // Random bar heights for demo (live data will replace)
    function renderChart(series) {
        const container = document.getElementById('chart');
        container.innerHTML = '';
        const max = Math.max(...series, 0.01);
        series.forEach(val => {
            const bar = document.createElement('div');
            bar.className = 'chart-bar' + (val > 0 ? ' active' : '');
            const height = Math.max(4, (val / max) * 100);
            bar.style.height = height + 'px';
            container.appendChild(bar);
        });
    }

    async function refresh() {
        try {
            const res = await fetch('/api/stats');
            const data = await res.json();
            $('today').textContent = '$' + (data.today_pnl || 0).toFixed(2);
            $('total').textContent = '$' + (data.total_pnl || 0).toFixed(2);
            $('trades').textContent = data.trades_today || 0;
            $('gas').textContent = data.gas_gwei || 0;
            $('wallet').textContent = '$' + (data.wallet_usd || 0).toFixed(2);
            $('queue').textContent = data.queue_size || 0;
            if (data.profit_series && data.profit_series.length) {
                renderChart(data.profit_series);
            }
        } catch(e) {
            $('statusText').textContent = 'Connecting...';
        }

        try {
            const res2 = await fetch('/api/trades');
            const trades = await res2.json();
            const tbody = document.getElementById('tradesBody');
            const empty = document.getElementById('emptyState');
            if (trades.length === 0) {
                empty.style.display = 'block';
                tbody.innerHTML = '';
            } else {
                empty.style.display = 'none';
                tbody.innerHTML = trades.slice(0, 20).map(t => `
                    <tr>
                        <td>${t.time || '--:--'}</td>
                        <td>${t.pair || '—'}</td>
                        <td>${t.buy_dex || '?'} → ${t.sell_dex || '?'}</td>
                        <td>${t.spread_pct ? t.spread_pct.toFixed(2) + '%' : '—'}</td>
                        <td class="${t.net_usd > 0 ? 'profit' : t.net_usd < 0 ? 'loss' : ''}">
                            ${t.net_usd !== null && t.net_usd !== undefined ? (t.net_usd >= 0 ? '+' : '') + '$' + t.net_usd.toFixed(4) : '—'}
                        </td>
                        <td><span class="status ${t.status || 'pending'}">${t.status || '—'}</span></td>
                    </tr>
                `).join('');
            }
        } catch(e) {}

        // Uptime
        const diff = Math.floor((Date.now() - start) / 1000);
        const h = String(Math.floor(diff / 3600)).padStart(2, '0');
        const m = String(Math.floor((diff % 3600) / 60)).padStart(2, '0');
        const s = String(diff % 60).padStart(2, '0');
        $('uptime').textContent = h + ':' + m + ':' + s;
    }

    refresh();
    setInterval(refresh, 3000);
})();
</script>
</body>
</html>
        "#)
    });

    let routes = dashboard.or(health).or(stats).or(trades);
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}
