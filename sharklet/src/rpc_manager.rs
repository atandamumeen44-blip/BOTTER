// src/rpc_manager.rs
//
//  ██████╗ ██████╗  ██████╗    ███╗   ███╗ █████╗ ███╗   ██╗ █████╗  ██████╗ ███████╗██████╗
//  ██╔══██╗██╔══██╗██╔════╝    ████╗ ████║██╔══██╗████╗  ██║██╔══██╗██╔════╝ ██╔════╝██╔══██╗
//  ██████╔╝██████╔╝██║         ██╔████╔██║███████║██╔██╗ ██║███████║██║  ███╗█████╗  ██████╔╝
//  ██╔══██╗██╔═══╝ ██║         ██║╚██╔╝██║██╔══██║██║╚██╗██║██╔══██║██║   ██║██╔══╝  ██╔══██╗
//  ██║  ██║██║     ╚██████╗    ██║ ╚═╝ ██║██║  ██║██║ ╚████║██║  ██║╚██████╔╝███████╗██║  ██║
//  ╚═╝  ╚═╝╚═╝      ╚═════╝    ╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═╝
//
//  Multi‑RPC endpoint manager with latency tracking, health checks, and consensus.
//  Uses exponentially weighted moving averages for latency and success rate.
//  Endpoints that fail 3 consecutive calls are automatically benched.

use ethers::providers::{Http, Provider, Middleware};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::timeout as tokio_timeout;
use tracing::{info, warn, debug, error};

// ── Scoring constants ─────────────────────────────────────────────────────
const LATENCY_WEIGHT: f64 = 0.3;
const SUCCESS_WEIGHT: f64 = 0.7;
const MAX_CONSECUTIVE_FAILURES: u32 = 3;

#[derive(Debug, Clone)]
pub struct EndpointStats {
    pub url: String,
    pub ewma_latency_ms: f64,
    pub success_rate: f64,      // 0.0–1.0, EWMA
    pub consecutive_failures: u32,
}

impl EndpointStats {
    fn new(url: String) -> Self {
        Self {
            url,
            ewma_latency_ms: 200.0,
            success_rate: 1.0,
            consecutive_failures: 0,
        }
    }

    fn record(&mut self, ok: bool, latency_ms: f64) {
        const ALPHA: f64 = 0.3;
        self.ewma_latency_ms = ALPHA * latency_ms + (1.0 - ALPHA) * self.ewma_latency_ms;
        let outcome = if ok { 1.0 } else { 0.0 };
        self.success_rate = ALPHA * outcome + (1.0 - ALPHA) * self.success_rate;
        self.consecutive_failures = if ok { 0 } else { self.consecutive_failures + 1 };
    }

    /// Lower is better. Penalised heavily if consecutive failures exceed limit.
    fn score(&self) -> f64 {
        if self.consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
            return f64::INFINITY;
        }
        let latency_score = self.ewma_latency_ms / 1000.0;
        let fail_score = 1.0 - self.success_rate;
        LATENCY_WEIGHT * latency_score + SUCCESS_WEIGHT * fail_score
    }
}

pub struct RpcManager {
    providers: Vec<(EndpointStats, Arc<Provider<Http>>)>,
    stats: RwLock<Vec<EndpointStats>>,
    /// Cached index of the best provider + when it was selected.
    best_cache: RwLock<Option<(usize, Instant)>>,
    cache_ttl: Duration,
    health_check_interval: Duration,
}

impl RpcManager {
    pub fn new(urls: Vec<String>) -> Result<Self, url::ParseError> {
        let mut providers = Vec::new();
        let mut stats = Vec::new();
        for url in urls {
            let provider = Provider::<Http>::try_from(url.as_str())
                .map_err(|_| url::ParseError::EmptyHost)?;
            let es = EndpointStats::new(url.clone());
            stats.push(es.clone());
            providers.push((es, Arc::new(provider)));
        }
        info!(count = providers.len(), "RpcManager initialised");
        Ok(Self {
            providers,
            stats: RwLock::new(stats),
            best_cache: RwLock::new(None),
            cache_ttl: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(120),
        })
    }

    /// Start periodic background health checks. Call this once after wrapping in Arc.
    pub fn start_health_checks(self: &Arc<Self>) {
        let mgr = self.clone();
        tokio::spawn(async move { mgr.background_health_check().await });
    }

    async fn background_health_check(&self) {
        loop {
            tokio::time::sleep(self.health_check_interval).await;
            for (_, provider) in &self.providers {
                let start = Instant::now();
                let result = provider.get_block_number().await;
                let elapsed = start.elapsed();
                let ok = result.is_ok();
                if let Err(e) = &result {
                    debug!("health check failed for {}: {e}", provider.url());
                }
                self.record(provider, ok, elapsed).await;
            }
        }
    }

    /// Returns the current best‑scored provider. Uses caching to avoid
    /// recomputing on every call.
    pub async fn best(&self) -> Arc<Provider<Http>> {
        let now = Instant::now();
        {
            let cache = self.best_cache.read().await;
            if let Some((idx, cached_at)) = *cache {
                if now - cached_at < self.cache_ttl {
                    debug!("Using cached best provider: {}", self.providers[idx].0.url);
                    return self.providers[idx].1.clone();
                }
            }
        }

        // Re‑score
        let stats = self.stats.read().await;
        let mut best_idx = 0;
        let mut best_score = f64::INFINITY;
        for (i, s) in stats.iter().enumerate() {
            let sc = s.score();
            if sc < best_score {
                best_score = sc;
                best_idx = i;
            }
        }
        drop(stats);

        let mut cache = self.best_cache.write().await;
        *cache = Some((best_idx, Instant::now()));
        debug!("Updated best provider: {}", self.providers[best_idx].0.url);
        self.providers[best_idx].1.clone()
    }

    /// Fetch the latest block number from two independently scored endpoints
    /// and only return `Some` if they agree within 1 block.
    pub async fn consensus_block_number(&self) -> Option<U64Pair> {
        if self.providers.len() < 2 {
            warn!("Not enough providers for consensus check");
            return None;
        }

        let a = &self.providers[0].1;
        let b = &self.providers[1].1;
        let timeout_dur = Duration::from_secs(3);

        let (ra, rb) = tokio::join!(
            self.timed_call(a, timeout_dur),
            self.timed_call(b, timeout_dur),
        );

        match (ra, rb) {
            (Some(x), Some(y)) => {
                let diff = if x > y { x - y } else { y - x };
                if diff <= 1 {
                    debug!("RPC consensus OK: blocks {} / {}", x, y);
                    Some(U64Pair { a: x, b: y })
                } else {
                    warn!("RPC consensus FAILED: blocks {} vs {}", x, y);
                    None
                }
            }
            (Some(x), None) | (None, Some(x)) => {
                warn!("RPC consensus: only one endpoint responded (block {})", x);
                None
            }
            _ => {
                warn!("RPC consensus: neither endpoint responded");
                None
            }
        }
    }

    async fn timed_call(&self, provider: &Arc<Provider<Http>>, timeout_dur: Duration) -> Option<u64> {
        let start = Instant::now();
        let result = tokio_timeout(timeout_dur, provider.get_block_number()).await;
        let elapsed = start.elapsed();
        match result {
            Ok(Ok(n)) => {
                self.record(provider, true, elapsed).await;
                Some(n.as_u64())
            }
            Ok(Err(e)) => {
                error!("RPC block number error: {e}");
                self.record(provider, false, elapsed).await;
                None
            }
            Err(_) => {
                warn!("RPC call timed out after {:?}", timeout_dur);
                self.record(provider, false, elapsed).await;
                None
            }
        }
    }

    async fn record(&self, provider: &Arc<Provider<Http>>, ok: bool, elapsed: Duration) {
        let url = provider.url().to_string();
        let mut stats = self.stats.write().await;
        if let Some(s) = stats.iter_mut().find(|s| s.url == url) {
            s.record(ok, elapsed.as_millis() as f64);
        }
    }

    /// Snapshot of all endpoint stats (for dashboard).
    pub async fn scoreboard(&self) -> Vec<EndpointStats> {
        self.stats.read().await.clone()
    }
}

#[derive(Debug)]
pub struct U64Pair {
    pub a: u64,
    pub b: u64,
}