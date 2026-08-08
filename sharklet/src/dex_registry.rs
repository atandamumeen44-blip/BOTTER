// src/dex_registry.rs
//
//  Network‑agnostic DEX pool resolver.
//  – Reads token pairs from config/addresses.toml (based on CHAIN_ID)
//  – Resolves pool addresses on‑chain via factory.getPair()
//  – Caches results to disk so restarts are instant
//  – Structured logging (tracing) for observability

use ethers::prelude::*;
use ethers::contract::abigen;
use std::sync::Arc;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use serde::Deserialize;
use tracing::{info, warn, debug};

abigen!(
    IUniswapV2Factory,
    r#"[
        function getPair(address tokenA, address tokenB) external view returns (address pair)
    ]"#
);

// ── DEX definitions (factory + router) ──────────────────────────────────
#[derive(Debug, Clone)]
pub struct DexDef {
    pub name: &'static str,
    pub factory: Address,
    pub router: Address,
}

pub fn known_dexes() -> Vec<DexDef> {
    vec![
        DexDef {
            name: "quickswap",
            factory: addr("0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32"),
            router: addr("0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff"),
        },
        DexDef {
            name: "sushiswap",
            factory: addr("0xc35DADB65012eC5796536bD9864eD8773aBc74C4"),
            router: addr("0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506"),
        },
    ]
}

fn addr(s: &str) -> Address {
    s.parse()
        .expect("Hardcoded address is invalid – check dex_registry.rs constants")
}

// ── Token pair configuration (loaded from config/addresses.toml) ────────

#[derive(Debug, Deserialize)]
struct AddressesConfig {
    amoy: Option<NetworkAddresses>,
    polygon: Option<NetworkAddresses>,
    arbitrum: Option<NetworkAddresses>,
    base: Option<NetworkAddresses>,
}

#[derive(Debug, Deserialize)]
struct NetworkAddresses {
    usdc: Option<String>,
    wmatic: Option<String>,
    weth: Option<String>,
    usdt: Option<String>,
    dai: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TokenPairDef {
    pub label: String,
    pub token0: Address,
    pub token1: Address,
}

/// Returns token pairs for the active chain (determined by CHAIN_ID env var).
pub fn token_pairs() -> Vec<TokenPairDef> {
    let chain_id: u64 = std::env::var("CHAIN_ID")
        .unwrap_or_else(|_| "137".into())
        .parse()
        .unwrap_or(137);

    let config_path = "config/addresses.toml";
    let config_str = fs::read_to_string(config_path).unwrap_or_default();
    let config: AddressesConfig = toml::from_str(&config_str).unwrap_or(AddressesConfig {
        amoy: None,
        polygon: None,
        arbitrum: None,
        base: None,
    });

    let network = match chain_id {
        80002 => config.amoy,
        137 => config.polygon,
        42161 => config.arbitrum,
        8453 => config.base,
        _ => {
            warn!("Unknown chain_id {chain_id} – falling back to Polygon mainnet addresses");
            config.polygon
        }
    };

    let Some(net) = network else {
        warn!("No network config found for chain {chain_id} in {config_path}");
        return vec![];
    };

    let mut pairs = Vec::new();
    // USDC/WETH
    if let (Some(usdc), Some(weth)) = (net.usdc.as_ref(), net.weth.as_ref()) {
        pairs.push(TokenPairDef {
            label: "USDC/WETH".into(),
            token0: usdc.parse().expect("Invalid USDC address in config"),
            token1: weth.parse().expect("Invalid WETH address in config"),
        });
    }
    // USDC/WMATIC
    if let (Some(usdc), Some(wmatic)) = (net.usdc.as_ref(), net.wmatic.as_ref()) {
        pairs.push(TokenPairDef {
            label: "USDC/WMATIC".into(),
            token0: usdc.parse().expect("Invalid USDC address in config"),
            token1: wmatic.parse().expect("Invalid WMATIC address in config"),
        });
    }
    // Add more pairs as needed (e.g., USDT/USDC, DAI/USDC)

    info!("Loaded {} token pairs for chain {chain_id}", pairs.len());
    pairs
}

// ── Pool resolution with disk cache ─────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct CachedPool {
    dex_name: String,
    pool_addr: String,
    token0: String,
    token1: String,
}

pub async fn resolve_all_pools<M: Middleware + 'static>(
    provider: Arc<M>,
    dexes: &[DexDef],
    pairs: &[TokenPairDef],
) -> Vec<crate::scanner::TrackedPair> {
    let mut new_cache: Vec<CachedPool> = Vec::new();
    let mut resolved = Vec::new();

    // Load disk cache
    let cache_path = PathBuf::from("resolved_pools.json");
    let cached: HashMap<String, Vec<(String, Address, Address)>> = if cache_path.exists() {
        fs::read_to_string(&cache_path)
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<CachedPool>>(&s).ok())
            .map(|cached_pools| {
                let mut map: HashMap<String, Vec<(String, Address, Address)>> = HashMap::new();
                for cp in cached_pools {
                    let pair_key = format!("{}/{}", cp.token0, cp.token1);
                    map.entry(pair_key)
                        .or_default()
                        .push((
                            cp.dex_name.clone(),
                            cp.pool_addr.parse().unwrap(),
                            cp.token0.parse().unwrap(), // token0 address
                        ));
                }
                map
            })
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    for pair_def in pairs {
        let pair_key = format!("{:?}/{:?}", pair_def.token0, pair_def.token1);
        let pools = if let Some(cached_pools) = cached.get(&pair_key) {
            debug!("Using cached pools for {}", pair_def.label);
            cached_pools.clone()
        } else {
            debug!("Resolving pools on-chain for {}", pair_def.label);
            let mut pools = Vec::new();
            for dex in dexes {
                let factory = IUniswapV2Factory::new(dex.factory, provider.clone());
                match factory.get_pair(pair_def.token0, pair_def.token1).call().await {
                    Ok(pool_addr) if pool_addr != Address::zero() => {
                        pools.push((dex.name.to_string(), pool_addr, dex.router));
                        new_cache.push(CachedPool {
                            dex_name: dex.name.to_string(),
                            pool_addr: format!("{pool_addr:?}"),
                            token0: format!("{:?}", pair_def.token0),
                            token1: format!("{:?}", pair_def.token1),
                        });
                    }
                    Ok(_) => {
                        debug!("{} has no pool for {}", dex.name, pair_def.label);
                    }
                    Err(e) => {
                        warn!("getPair failed on {} for {}: {:?}", dex.name, pair_def.label, e);
                    }
                }
            }
            pools
        };

        if pools.len() >= 2 {
            resolved.push(crate::scanner::TrackedPair {
                label: pair_def.label.clone(),
                pools: pools.iter().map(|(name, pool, _)| (name.clone(), *pool)).collect(),
                token0: pair_def.token0,
                token1: pair_def.token1,
                routers: pools.iter().map(|(name, _, router)| (name.clone(), *router)).collect(),
            });
        } else {
            warn!(
                "{} only resolved on {} DEX(es) – need 2+ to scan for spread, skipping",
                pair_def.label,
                pools.len()
            );
        }
    }

    // Persist any newly fetched pools to cache
    if !new_cache.is_empty() {
        if let Ok(json) = serde_json::to_string(&new_cache) {
            let _ = fs::write(&cache_path, json);
            info!("Updated pool cache with {} new entries", new_cache.len());
        }
    }

    info!("Resolved {} tradable pairs", resolved.len());
    resolved
}