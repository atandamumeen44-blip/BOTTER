// src/gas_manager.rs
//
//  Tracks native gas balance and reinvests a portion of profits back into gas.

use ethers::prelude::*;
use std::sync::Arc;

pub struct GasManager<M: Middleware> {
    wallet: Address,
    provider: Arc<M>,
    pub min_gas_balance_usd: f64,
    pub reinvestment_rate: f64,
    pub max_gas_reinvest_usd: f64,
    pub gas_balance_usd: f64,
}

impl<M: Middleware + 'static> GasManager<M> {
    pub fn new(wallet: Address, provider: Arc<M>) -> Self {
        Self {
            wallet,
            provider,
            min_gas_balance_usd: 1.0,
            reinvestment_rate: 0.20,
            max_gas_reinvest_usd: 100.0,
            gas_balance_usd: 5.0,
        }
    }

    pub async fn update_balance(&mut self, matic_price: f64) -> f64 {
        let balance_wei = self.provider.get_balance(self.wallet, None).await.unwrap_or_default();
        let balance_pol = balance_wei.as_u128() as f64 / 1e18;
        self.gas_balance_usd = balance_pol * matic_price;
        self.gas_balance_usd
    }

    pub fn calculate_gas_to_reinvest(&self, profit_usd: f64) -> f64 {
        if profit_usd <= 0.0 { return 0.0; }
        (profit_usd * self.reinvestment_rate).min(self.max_gas_reinvest_usd)
    }

    pub fn spend_gas(&mut self, amount_usd: f64) { self.gas_balance_usd -= amount_usd; }
    pub fn add_gas(&mut self, amount_usd: f64) { self.gas_balance_usd += amount_usd; }
}