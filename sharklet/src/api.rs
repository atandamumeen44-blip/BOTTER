use warp::Filter;
use std::sync::Arc;
use serde_json::json;
use std::sync::RwLock;
use std::collections::VecDeque;

pub struct AppState {
    pub health: bool,
}

// Simple in-memory log storage
pub struct LogStore {
    pub entries: RwLock<VecDeque<String>>,
}

impl LogStore {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(VecDeque::with_capacity(100)),
        }
    }

    pub fn push(&self, msg: String) {
        let mut entries = self.entries.write().unwrap();
        entries.push_back(msg);
        if entries.len() > 100 {
            entries.pop_front();
        }
    }

    pub fn get(&self) -> Vec<String> {
        self.entries.read().unwrap().iter().cloned().collect()
    }
}

pub async fn start_server(state: Arc<AppState>, db: Arc<crate::logger::Logger>) {
    let logs = Arc::new(LogStore::new());

    // Health check
    let health = warp::path("health").map(|| warp::reply::json(&json!({"status": "ok"})));

    // Stats API – includes more data
    let stats = warp::path!("api" / "stats").map(|| {
        warp::reply::json(&json!({
            "wallet_usd": 0.0,
            "today_pnl": 0.0,
            "total_pnl": 0.0,
            "trades_today": 0,
            "gas_gwei": 31,
            "queue_size": 0,
            "profit_series": [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "status": "hunting"
        }))
    });

    // Trades API – detailed trade log
    let trades = warp::path!("api" / "trades").map(|| {
        warp::reply::json(&json!([]))
    });

    // Opportunities API – shows what the bot is finding
    let opportunities = warp::path!("api" / "opportunities").map(|| {
        warp::reply::json(&json!([
            {"pair": "USDC/WETH", "spread": "0.15%", "profit": "$2.50", "status": "checking"},
            {"pair": "USDC/WETH", "spread": "0.12%", "profit": "$1.80", "status": "pending"}
        ]))
    });

    // Logs API – shows live logs
    let logs_endpoint = warp::path!("api" / "logs")
        .and(warp::any().map(move || logs.clone()))
        .and_then(|logs: Arc<LogStore>| async move {
            let entries = logs.get();
            Ok::<_, warp::Rejection>(warp::reply::json(&json!({ "logs": entries })))
        });

    // Advanced Dashboard HTML
    let dashboard = warp::path::end().map(|| {
        warp::reply::html(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>🦈 Sharklet – Advanced Dashboard</title>
    <style>
        * { margin:0; padding:0; box-sizing:border-box; }
        body {
            background: #0a0d10;
            color: #d4d4d4;
            font-family: 'Segoe UI', -apple-system, system-ui, sans-serif;
            min-height: 100vh;
            padding: 2rem;
        }
        .container { max-width: 1400px; margin: 0 auto; }

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
        .header .sub { color: #666; font-size: 0.85rem; }
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

        /* Grid Layout */
        .grid {
            display: grid;
            grid-template-columns: 1.6fr 1fr;
            gap: 1.2rem;
            margin-bottom: 1.2rem;
        }
        @media (max-width: 860px) { .grid { grid-template-columns: 1fr; } }

        /* Cards */
        .card {
            background: #0d1117;
            border: 1px solid #1a2028;
            border-radius: 12px;
            padding: 1.2rem;
        }
        .card h3 {
            font-size: 0.7rem;
            text-transform: uppercase;
            letter-spacing: 1px;
            color: #666;
            margin-bottom: 1rem;
        }

        /* Stats Grid */
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
            gap: 1rem;
            margin-bottom: 1.2rem;
        }
        .stat-card {
            background: #0d1117;
            border: 1px solid #1a2028;
            border-radius: 12px;
            padding: 1rem;
            text-align: center;
        }
        .stat-card .label {
            font-size: 0.6rem;
            text-transform: uppercase;
            letter-spacing: 0.8px;
            color: #666;
        }
        .stat-card .value {
            font-size: 1.6rem;
            font-weight: 700;
            margin-top: 0.2rem;
        }
        .stat-card .value.green { color: #4caf50; }
        .stat-card .value.blue { color: #4facfe; }
        .stat-card .value.yellow { color: #ffc107; }
        .stat-card .value.white { color: #d4d4d4; }

        /* Logs */
        .log-container {
            max-height: 300px;
            overflow-y: auto;
            font-family: monospace;
            font-size: 0.7rem;
            background: #0a0d10;
            border-radius: 8px;
            padding: 0.5rem;
            color: #888;
        }
        .log-container::-webkit-scrollbar {
            width: 4px;
        }
        .log-container::-webkit-scrollbar-track {
            background: #0a0d10;
        }
        .log-container::-webkit-scrollbar-thumb {
            background: #1a2028;
            border-radius: 2px;
        }
        .log-entry {
            padding: 0.15rem 0;
            border-bottom: 1px solid #11151a;
        }
        .log-entry .time { color: #555; }
        .log-entry .info { color: #4facfe; }
        .log-entry .success { color: #4caf50; }
        .log-entry .warning { color: #ffc107; }
        .log-entry .error { color: #ff5c5c; }

        /* Tables */
        table {
            width: 100%;
            border-collapse: collapse;
            font-size: 0.75rem;
        }
        th {
            text-align: left;
            color: #666;
            font-weight: 500;
            padding: 0.4rem 0.3rem;
            border-bottom: 1px solid #1a2028;
        }
        td {
            padding: 0.4rem 0.3rem;
            border-bottom: 1px solid #1a2028;
            color: #888;
        }
        tr:last-child td { border-bottom: none; }
        .status.success { color: #4caf50; }
        .status.pending { color: #ffc107; }
        .status.failed { color: #ff5c5c; }

        .empty-state {
            text-align: center;
            color: #444;
            padding: 1.5rem 0;
            font-size: 0.8rem;
        }

        footer {
            text-align: center;
            margin-top: 1.5rem;
            color: #444;
            font-size: 0.65rem;
            border-top: 1px solid #1a2028;
            padding-top: 1rem;
        }
    </style>
</head>
<body>
<div class="container">
    <div class="header">
        <div>
            <h1>🦈 Sharklet</h1>
            <span class="sub">AI Arbitrage Engine · Live</span>
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

    <div class="grid">
        <div class="card">
            <h3>📊 Live Trade Log</h3>
            <table>
                <thead><tr><th>Time</th><th>Pair</th><th>Route</th><th>Spread</th><th>Net</th><th>Status</th></tr></thead>
                <tbody id="tradesBody"></tbody>
            </table>
            <div class="empty-state" id="emptyState">No trades yet — hunting</div>
        </div>
        <div class="card">
            <h3>🔍 Opportunities</h3>
            <table>
                <thead><tr><th>Pair</th><th>Spread</th><th>Profit</th><th>Status</th></tr></thead>
                <tbody id="oppsBody"></tbody>
            </table>
            <div class="empty-state" id="oppsEmpty">Scanning...</div>
        </div>
    </div>

    <div class="card">
        <h3>📝 Live Logs</h3>
        <div class="log-container" id="logContainer">
            <div class="empty-state">Waiting for logs...</div>
        </div>
    </div>

    <footer>🦈 Sharklet v2 · Simulation Mode · Data refreshes every 2s</footer>
</div>

<script>
(function() {
    const $ = id => document.getElementById(id);
    const logContainer = $('logContainer');
    const start = Date.now();

    // Fix: Remove duplicate listeners by storing interval IDs
    let statsInterval = null;
    let tradesInterval = null;
    let oppsInterval = null;
    let logsInterval = null;
    let uptimeInterval = null;

    function renderChart(series) {
        // Simple chart rendering – placeholder
    }

    async function refreshStats() {
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
            // silent fail
        }
    }

    async function refreshTrades() {
        try {
            const res = await fetch('/api/trades');
            const trades = await res.json();
            const tbody = $('tradesBody');
            const empty = $('emptyState');
            if (trades.length === 0) {
                empty.style.display = 'block';
                tbody.innerHTML = '';
            } else {
                empty.style.display = 'none';
                tbody.innerHTML = trades.map(t => `
                    <tr>
                        <td>${t.time || '--:--'}</td>
                        <td>${t.pair || '—'}</td>
                        <td>${t.buy_dex || '?'} → ${t.sell_dex || '?'}</td>
                        <td>${t.spread_pct ? t.spread_pct.toFixed(2) + '%' : '—'}</td>
                        <td class="${t.net_usd > 0 ? 'success' : t.net_usd < 0 ? 'failed' : ''}">
                            ${t.net_usd !== null && t.net_usd !== undefined ? (t.net_usd >= 0 ? '+' : '') + '$' + t.net_usd.toFixed(4) : '—'}
                        </td>
                        <td><span class="status ${t.status || 'pending'}">${t.status || '—'}</span></td>
                    </tr>
                `).join('');
            }
        } catch(e) {}
    }

    async function refreshOpportunities() {
        try {
            const res = await fetch('/api/opportunities');
            const opps = await res.json();
            const tbody = $('oppsBody');
            const empty = $('oppsEmpty');
            if (opps.length === 0) {
                empty.style.display = 'block';
                tbody.innerHTML = '';
            } else {
                empty.style.display = 'none';
                tbody.innerHTML = opps.map(o => `
                    <tr>
                        <td>${o.pair || '—'}</td>
                        <td>${o.spread || '—'}</td>
                        <td>${o.profit || '—'}</td>
                        <td><span class="status ${o.status || 'pending'}">${o.status || '—'}</span></td>
                    </tr>
                `).join('');
            }
        } catch(e) {}
    }

    async function refreshLogs() {
        try {
            const res = await fetch('/api/logs');
            const data = await res.json();
            const logs = data.logs || [];
            if (logs.length === 0) {
                logContainer.innerHTML = `<div class="empty-state">Waiting for logs...</div>`;
                return;
            }
            logContainer.innerHTML = logs.map(log => {
                let cls = 'info';
                if (log.includes('EXECUTED')) cls = 'success';
                else if (log.includes('skip') || log.includes('rejected')) cls = 'warning';
                else if (log.includes('Error') || log.includes('failed')) cls = 'error';
                return `<div class="log-entry"><span class="time">[${new Date().toLocaleTimeString()}]</span> <span class="${cls}">${log}</span></div>`;
            }).join('');
            logContainer.scrollTop = logContainer.scrollHeight;
        } catch(e) {
            // silent fail
        }
    }

    function updateUptime() {
        const diff = Math.floor((Date.now() - start) / 1000);
        const h = String(Math.floor(diff / 3600)).padStart(2, '0');
        const m = String(Math.floor((diff % 3600) / 60)).padStart(2, '0');
        const s = String(diff % 60).padStart(2, '0');
        $('uptime').textContent = h + ':' + m + ':' + s;
    }

    // Clean up old intervals if they exist
    function cleanupIntervals() {
        if (statsInterval) clearInterval(statsInterval);
        if (tradesInterval) clearInterval(tradesInterval);
        if (oppsInterval) clearInterval(oppsInterval);
        if (logsInterval) clearInterval(logsInterval);
        if (uptimeInterval) clearInterval(uptimeInterval);
    }

    // Initial refresh
    refreshStats();
    refreshTrades();
    refreshOpportunities();
    refreshLogs();
    updateUptime();

    // Set up intervals
    cleanupIntervals();
    statsInterval = setInterval(refreshStats, 3000);
    tradesInterval = setInterval(refreshTrades, 3000);
    oppsInterval = setInterval(refreshOpportunities, 3000);
    logsInterval = setInterval(refreshLogs, 2000);
    uptimeInterval = setInterval(updateUptime, 1000);
})();
</script>
</body>
</html>
        "#)
    });

    let routes = dashboard.or(health).or(stats).or(trades).or(opportunities).or(logs_endpoint);
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}
