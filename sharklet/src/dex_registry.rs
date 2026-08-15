use ethers::prelude::*;
use ethers::contract::abigen;
use std::sync::Arc;
use serde::Deserialize;
use tracing::{info, warn, error};

abigen!(
    IUniswapV2Factory,
    r#"[
        function getPair(address tokenA, address tokenB) external view returns (address pair)
    ]"#
);

#[derive(Debug, Clone)]
pub struct DexDef {
    pub name: &'static str,
    pub factory: Address,
    pub router: Address,
}

fn addr(s: &str) -> Address {
    s.parse().expect("Invalid address")
}

pub fn known_dexes() -> Vec<DexDef> {
    let chain_id: u64 = std::env::var("CHAIN_ID")
        .unwrap_or_else(|_| "80002".into())
        .parse()
        .unwrap_or(80002);

    if chain_id == 80002 {
        // Amoy testnet
        vec![
            DexDef {
                name: "quickswap",
                factory: addr("0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff"),
                router: addr("0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506"),
            },
            DexDef {
                name: "sushiswap",
                factory: addr("0xc35DADB65012eC5796536bD9864eD8773aBc74C4"),
                router: addr("0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506"),
            },
        ]
    } else {
        // Polygon Mainnet — verified against PolygonScan
        vec![
            DexDef {
                name: "quickswap",
                factory: addr("0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32"), // QuickSwap V2 Factory
                router: addr("0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff"),  // QuickSwap V2 Router
            },
            DexDef {
                name: "sushiswap",
                factory: addr("0xc35DADB65012eC5796536bD9864eD8773aBc74C4"), // SushiSwap V2 Factory
                router: addr("0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506"),  // SushiSwap Router
            },
        ]
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenPairDef {
    pub label: String,
    pub token0: Address,
    pub token1: Address,
}

pub fn token_pairs() -> Vec<TokenPairDef> {
    let chain_id: u64 = std::env::var("CHAIN_ID")
        .unwrap_or_else(|_| "80002".into())
        .parse()
        .unwrap_or(80002);

    if chain_id == 80002 {
        vec![TokenPairDef {
            label: "USDC/WMATIC".to_string(),
            token0: addr("0x41E94Eb019C0762f9Bfcf9Fb1E58725BfB0e7582"),
            token1: addr("0x360ad4f9a9A8EFe9A8DCB5f461c4Cc1047E1Dcf9"),
        }]
    } else {
        vec![TokenPairDef {
            label: "USDC/WETH".to_string(),
            token0: addr("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"),
            token1: addr("0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619"),
        }]
    }
}

pub async fn resolve_all_pools<M: Middleware + 'static>(
    provider: Arc<M>,
    dexes: &[DexDef],
    pairs: &[TokenPairDef],
) -> Vec<crate::scanner::TrackedPair> {
    let mut resolved = Vec::new();

    for pair_def in pairs {
        let mut pools = Vec::new();
        let mut routers = Vec::new();

        for dex in dexes {
            let factory = IUniswapV2Factory::new(dex.factory, provider.clone());
            let call = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                factory.get_pair(pair_def.token0, pair_def.token1).call(),
            )
            .await;

            match call {
                Ok(Ok(pool_addr)) if pool_addr != Address::zero() => {
                    info!("dex_registry: found {} pool for {} at {:?}", dex.name, pair_def.label, pool_addr);
                    pools.push((dex.name.to_string(), pool_addr));
                    routers.push((dex.name.to_string(), dex.router));
                }
                Ok(Ok(_)) => {
                    warn!("dex_registry: {} has no pool for {}", dex.name, pair_def.label);
                }
                Ok(Err(e)) => {
                    error!("dex_registry: getPair call failed on {} for {}: {:?}", dex.name, pair_def.label, e);
                }
                Err(_) => {
                    error!("dex_registry: getPair TIMED OUT on {} for {}", dex.name, pair_def.label);
                }
            }
        }

        if pools.len() >= 2 {
            info!("dex_registry: {} resolved on {} DEX(es) — tracking", pair_def.label, pools.len());
            resolved.push(crate::scanner::TrackedPair {
                label: pair_def.label.clone(),
                pools,
                token0: pair_def.token0,
                token1: pair_def.token1,
                routers,
            });
        } else {
            warn!(
                "dex_registry: {} only resolved on {} DEX(es) — need 2+ to scan for spread, skipping",
                pair_def.label, pools.len()
            );
        }
    }

    resolved
}
