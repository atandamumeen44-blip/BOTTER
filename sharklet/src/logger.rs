// src/logger.rs
//
//  Async, pooled SQLite trade journal.
//  – r2d2 connection pool for thread safety
//  – All public methods are async (spawn_blocking)
//  – Structured logging for errors
//  – Provides rehydration data for the risk engine on restart
//  – Dashboard metrics: total profit, success rate, recent trades

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Result as SqlResult};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::task;
use tracing::error;
use serde::Serialize;

pub struct Logger {
    pool: Pool<SqliteConnectionManager>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TradeRecord {
    pub pair_label: String,
    pub buy_dex: String,
    pub sell_dex: String,
    pub spread_pct: f64,
    pub size_usd: f64,
    pub predicted_net_usd: f64,
    pub realized_net_usd: Option<f64>,
    pub gas_cost_usd: f64,
    pub status: String,          // "executed", "rejected", "failed", "skipped"
    pub reason: Option<String>,
    pub tx_hash: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TradeSummary {
    pub total_profit: f64,
    pub today_profit: f64,
    pub total_trades: u32,
    pub successful_trades: u32,
    pub failed_trades: u32,
    pub success_rate: f64,
}

impl Logger {
    /// Open (or create) the database and apply migrations.
    pub fn open(path: &str) -> SqlResult<Self> {
        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::new(manager).map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?;

        // Run migrations on a connection from the pool
        let conn = pool.get().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS trades (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts INTEGER NOT NULL,
                pair_label TEXT NOT NULL,
                buy_dex TEXT NOT NULL,
                sell_dex TEXT NOT NULL,
                spread_pct REAL NOT NULL,
                size_usd REAL NOT NULL,
                predicted_net_usd REAL NOT NULL,
                realized_net_usd REAL,
                gas_cost_usd REAL NOT NULL,
                status TEXT NOT NULL,
                reason TEXT,
                tx_hash TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_trades_ts ON trades(ts);
            CREATE INDEX IF NOT EXISTS idx_trades_status ON trades(status);
            CREATE INDEX IF NOT EXISTS idx_trades_pair ON trades(pair_label);",
        )?;

        Ok(Logger { pool })
    }

    /// Record a trade asynchronously.
    pub async fn record(&self, record: TradeRecord) -> SqlResult<()> {
        let pool = self.pool.clone();
        let result = task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            let ts = now_secs() as i64;
            conn.execute(
                "INSERT INTO trades (ts, pair_label, buy_dex, sell_dex, spread_pct, size_usd,
                    predicted_net_usd, realized_net_usd, gas_cost_usd, status, reason, tx_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    ts, record.pair_label, record.buy_dex, record.sell_dex,
                    record.spread_pct, record.size_usd, record.predicted_net_usd,
                    record.realized_net_usd, record.gas_cost_usd, record.status,
                    record.reason, record.tx_hash
                ],
            ).map(|_| ())
        })
        .await;

        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                error!("Failed to record trade: {e}");
                Err(e)
            }
            Err(join_err) => {
                error!("spawn_blocking panic in logger::record: {join_err}");
                Err(rusqlite::Error::ToSqlConversionFailure(Box::new(join_err)))
            }
        }
    }

    /// Update realized net profit for a trade identified by tx_hash.
    pub async fn update_realized(&self, tx_hash: &str, realized_net_usd: f64) -> SqlResult<()> {
        let pool = self.pool.clone();
        let tx_hash = tx_hash.to_string();
        task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            conn.execute(
                "UPDATE trades SET realized_net_usd = ?1 WHERE tx_hash = ?2",
                params![realized_net_usd, tx_hash],
            ).map(|_| ())
        })
        .await
        .map_err(|e| {
            error!("spawn_blocking panic in logger::update_realized: {e}");
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?
    }

    /// Sum of realized losses within the last `seconds` (negative realized_net_usd).
    pub async fn losses_within(&self, seconds: i64) -> SqlResult<f64> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            let cutoff = now_secs() as i64 - seconds;
            let loss: Option<f64> = conn.query_row(
                "SELECT COALESCE(SUM(-realized_net_usd), 0) FROM trades
                 WHERE ts >= ?1 AND realized_net_usd IS NOT NULL AND realized_net_usd < 0",
                params![cutoff],
                |row| row.get(0),
            )?;
            Ok(loss.unwrap_or(0.0))
        })
        .await
        .map_err(|e| {
            error!("spawn_blocking panic in logger::losses_within: {e}");
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?
    }

    /// Count consecutive failures (most recent executed trades that were losses).
    pub async fn consecutive_failures(&self) -> SqlResult<u32> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            let mut stmt = conn.prepare(
                "SELECT realized_net_usd FROM trades WHERE status = 'executed' ORDER BY ts DESC LIMIT 20",
            )?;
            let rows: Vec<Option<f64>> = stmt.query_map([], |row| row.get(0))?
                .filter_map(Result::ok)
                .collect();
            let mut count = 0;
            for r in rows {
                match r {
                    Some(v) if v < 0.0 => count += 1,
                    _ => break,
                }
            }
            Ok(count)
        })
        .await
        .map_err(|e| {
            error!("spawn_blocking panic in logger::consecutive_failures: {e}");
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?
    }

    /// Total profit (sum of realized_net_usd) within the last `seconds`.
    pub async fn total_profit_within(&self, seconds: i64) -> SqlResult<f64> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            let cutoff = now_secs() as i64 - seconds;
            let profit: Option<f64> = conn.query_row(
                "SELECT COALESCE(SUM(realized_net_usd), 0) FROM trades
                 WHERE ts >= ?1 AND realized_net_usd IS NOT NULL",
                params![cutoff],
                |row| row.get(0),
            )?;
            Ok(profit.unwrap_or(0.0))
        })
        .await
        .map_err(|e| {
            error!("spawn_blocking panic in logger::total_profit_within: {e}");
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?
    }

    /// Get a summary of all trading activity (for dashboard).
    pub async fn get_summary(&self) -> SqlResult<TradeSummary> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            let total_profit: Option<f64> = conn.query_row(
                "SELECT SUM(realized_net_usd) FROM trades WHERE status = 'executed'",
                [],
                |row| row.get(0),
            )?;
            let today_cutoff = now_secs() as i64 - 86_400;
            let today_profit: Option<f64> = conn.query_row(
                "SELECT SUM(realized_net_usd) FROM trades WHERE status = 'executed' AND ts >= ?1",
                params![today_cutoff],
                |row| row.get(0),
            )?;
            let total_trades: u32 = conn.query_row(
                "SELECT COUNT(*) FROM trades WHERE status = 'executed'",
                [],
                |row| row.get(0),
            )?;
            let successful_trades: u32 = conn.query_row(
                "SELECT COUNT(*) FROM trades WHERE status = 'executed' AND realized_net_usd > 0",
                [],
                |row| row.get(0),
            )?;
            let failed_trades = total_trades - successful_trades;
            let success_rate = if total_trades > 0 {
                successful_trades as f64 / total_trades as f64 * 100.0
            } else {
                0.0
            };
            Ok(TradeSummary {
                total_profit: total_profit.unwrap_or(0.0),
                today_profit: today_profit.unwrap_or(0.0),
                total_trades,
                successful_trades,
                failed_trades,
                success_rate,
            })
        })
        .await
        .map_err(|e| {
            error!("spawn_blocking panic in logger::get_summary: {e}");
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?
    }

    /// Return the most recent `limit` trades.
    pub async fn recent_trades(&self, limit: usize) -> SqlResult<Vec<TradeRecord>> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            let mut stmt = conn.prepare(
                "SELECT pair_label, buy_dex, sell_dex, spread_pct, size_usd, predicted_net_usd,
                        realized_net_usd, gas_cost_usd, status, reason, tx_hash
                 FROM trades ORDER BY ts DESC LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![limit as i64], |row| {
                Ok(TradeRecord {
                    pair_label: row.get(0)?,
                    buy_dex: row.get(1)?,
                    sell_dex: row.get(2)?,
                    spread_pct: row.get(3)?,
                    size_usd: row.get(4)?,
                    predicted_net_usd: row.get(5)?,
                    realized_net_usd: row.get(6)?,
                    gas_cost_usd: row.get(7)?,
                    status: row.get(8)?,
                    reason: row.get(9)?,
                    tx_hash: row.get(10)?,
                })
            })?.collect::<SqlResult<Vec<_>>>()?;
            Ok(rows)
        })
        .await
        .map_err(|e| {
            error!("spawn_blocking panic in logger::recent_trades: {e}");
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}