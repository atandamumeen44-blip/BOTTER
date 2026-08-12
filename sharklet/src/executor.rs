use ethers::prelude::*;
use std::sync::Arc;
use std::future::Future;
use tokio::time::{sleep, Duration};

abigen!(
    FlashArb,
    r#"[
        function executeFlashLoan(uint256 amount) external
        event ArbExecuted(uint256 profit, uint256 amountIn, address initiator)
    ]"#
);

#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[error("simulation reverted: {0}")]
    SimulationReverted(String),
    #[error("send failed: {0}")]
    SendFailed(String),
    #[error("no receipt")]
    NoReceipt,
    #[error("log not found")]
    LogNotFound,
    #[error("gas price too high: {0}")]
    GasPriceTooHigh(String),
    #[error("nonce error: {0}")]
    NonceError(String),
}

pub struct Executor<M: Middleware> {
    contract: FlashArb<SignerMiddleware<M, LocalWallet>>,
    read_provider: Arc<M>,
    private_relay: Option<Provider<Http>>,
    max_retries: u32,
    nonce: Option<U256>,
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub tx_hash: H256,
    pub realized_profit_usdc: f64,
    pub gas_used: U256,
    pub gas_cost_usd: f64,
    pub effective_gas_price_gwei: f64,
}

impl<M: Middleware + 'static> Executor<M> {
    pub fn new(
        contract_address: Address,
        wallet: LocalWallet,
        signer_provider: Arc<SignerMiddleware<M, LocalWallet>>,
        read_provider: Arc<M>,
        private_relay_url: Option<String>,
    ) -> Self {
        let contract = FlashArb::new(contract_address, signer_provider);
        let _ = wallet;
        let private_relay = private_relay_url.and_then(|url| Provider::<Http>::try_from(url.as_str()).ok());
        Executor { contract, read_provider, private_relay, max_retries: 3, nonce: None }
    }

    pub async fn sync_nonce(&mut self) -> Result<(), ExecutorError> {
        let nonce = self
            .read_provider
            .get_transaction_count(self.contract.client().address(), None)
            .await
            .map_err(|e| ExecutorError::NonceError(format!("{e:?}")))?;
        self.nonce = Some(nonce);
        Ok(())
    }

    pub async fn simulate(&self, amount: U256) -> Result<(), ExecutorError> {
        self.contract
            .execute_flash_loan(amount)
            .call()
            .await
            .map_err(|e| ExecutorError::SimulationReverted(format!("{e:?}")))?;
        Ok(())
    }

    pub async fn estimate_gas(&self, amount: U256) -> Result<U256, ExecutorError> {
        self.contract
            .execute_flash_loan(amount)
            .estimate_gas()
            .await
            .map_err(|e| ExecutorError::SimulationReverted(format!("{e:?}")))
    }

    pub async fn simulate_and_execute(
        &mut self,
        amount: U256,
        gas_price_usd_per_gas: f64,
    ) -> Result<ExecutionResult, ExecutorError> {
        let mut last_err = None;
        for attempt in 0..self.max_retries {
            match self.simulate(amount).await {
                Ok(()) => {
                    last_err = None;
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                    if attempt + 1 < self.max_retries {
                        sleep(Duration::from_millis(200 * 2u64.pow(attempt))).await;
                    }
                }
            }
        }
        if let Some(e) = last_err {
            return Err(e);
        }

        if self.nonce.is_none() {
            self.sync_nonce().await?;
        }

        let mut call = self.contract.execute_flash_loan(amount);
        call.tx.set_nonce(self.nonce.unwrap());

        let receipt = if let Some(ref relay) = self.private_relay {
            let nonce = self.nonce.unwrap();
            call.tx.set_nonce(nonce);
            // ===== FIX: store client in a variable =====
            let client = self.contract.client();
            let signer = client.signer();
            let signed = call.tx.rlp_signed(
                &signer
                    .sign_transaction_sync(&call.tx)
                    .map_err(|e| ExecutorError::SendFailed(format!("sign error: {e:?}")))?,
            );
            // ========================================
            let pending = relay
                .send_raw_transaction(signed)
                .await
                .map_err(|e| ExecutorError::SendFailed(format!("relay send: {e:?}")))?;
            self.nonce = Some(nonce + 1);
            pending
                .await
                .map_err(|e| ExecutorError::SendFailed(format!("{e:?}")))?
                .ok_or(ExecutorError::NoReceipt)?
        } else {
            let pending = call
                .send()
                .await
                .map_err(|e| ExecutorError::SendFailed(format!("{e:?}")))?;
            self.nonce = Some(self.nonce.unwrap() + 1);
            pending
                .await
                .map_err(|e| ExecutorError::SendFailed(format!("{e:?}")))?
                .ok_or(ExecutorError::NoReceipt)?
        };

        let gas_used = receipt.gas_used.unwrap_or_default();
        let effective_gas_price = receipt.effective_gas_price.unwrap_or_default();
        let gas_cost_usd =
            gas_used.as_u128() as f64 * effective_gas_price.as_u128() as f64 / 1e18 * gas_price_usd_per_gas;

        let profit_log = receipt
            .logs
            .iter()
            .find_map(|log| {
                self.contract
                    .decode_event::<ArbExecutedFilter>("ArbExecuted", log.topics.clone(), log.data.clone())
                    .ok()
            })
            .ok_or(ExecutorError::LogNotFound)?;
        let realized_profit_usdc = profit_log.profit.as_u128() as f64 / 1e6;

        Ok(ExecutionResult {
            tx_hash: receipt.transaction_hash,
            realized_profit_usdc,
            gas_used,
            gas_cost_usd,
            effective_gas_price_gwei: effective_gas_price.as_u128() as f64 / 1e9,
        })
    }

    pub async fn execute_loop<F, Fut>(
        &mut self,
        initial_amount: U256,
        max_repeats: u32,
        gas_price_usd_per_gas: f64,
        target_profit: Option<f64>,
        max_loss: Option<f64>,
        spread_check: F,
    ) -> Vec<Result<ExecutionResult, ExecutorError>>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = bool>,
    {
        let mut results = Vec::new();
        let mut cum = 0.0;
        for _ in 0..max_repeats {
            if !results.is_empty() && !spread_check().await {
                break;
            }
            if let Some(t) = target_profit {
                if cum >= t {
                    break;
                }
            }
            if let Some(l) = max_loss {
                if cum <= -l {
                    break;
                }
            }
            match self.simulate_and_execute(initial_amount, gas_price_usd_per_gas).await {
                Ok(r) => {
                    cum += r.realized_profit_usdc - r.gas_cost_usd;
                    results.push(Ok(r));
                }
                Err(e) => {
                    results.push(Err(e));
                    break;
                }
            }
        }
        results
    }
}
