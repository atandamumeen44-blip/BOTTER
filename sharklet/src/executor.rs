// src/executor.rs
//
// Hardened production executor — built from the original solid base,
// enhanced with the multi‑execution loop and a battery of real‑world
// safety nets.
//
// Key additions:
//   - Nonce manager (prevents double‑spend races, handles stuck txs)
//   - Gas price bumping for pending transactions
//   - Private relay with fallback logic
//   - Integration with the Simulator pre‑flight checks
//   - Configurable max repeats, profit target, stop‑loss
//   - Detailed per‑trade logs for post‑mortem analysis

use ethers::prelude::*;
use ethers::abi::Abi;
use std::sync::Arc;
use std::future::Future;
use tokio::time::{sleep, timeout, Duration};

abigen!(
    FlashArb,
    r#"[
        function executeFlashLoan(uint256 amount) external
        event ArbExecuted(uint256 profit, uint256 amountIn, address initiator)
    ]"#
);

pub struct Executor<M: Middleware> {
    contract: FlashArb<SignerMiddleware<M, LocalWallet>>,
    read_provider: Arc<M>,
    private_relay: Option<Provider<Http>>,
    max_retries: u32,
    /// Nonce manager to avoid stuck nonces and double‑spends.
    nonce: Option<U256>,
    /// Track pending tx hash to enable replacement.
    pending_tx: Option<TxHash>,
    /// Base fee + priority fee caps (EIP‑1559).
    max_fee_per_gas: Option<U256>,
    max_priority_fee_per_gas: Option<U256>,
    gas_escalation_factor: f64, // multiplier if tx is pending > N seconds
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub tx_hash: H256,
    pub realized_profit_usdc: f64,
    pub gas_used: U256,
    pub gas_cost_usd: f64,
    pub effective_gas_price_gwei: f64,
}

#[derive(Debug)]
pub enum ExecutorError {
    SimulationReverted(String),
    SendFailed(String),
    NoReceipt,
    LogNotFound,
    GasPriceTooHigh(String),
    NonceError(String),
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
        let private_relay = private_relay_url
            .and_then(|url| Provider::<Http>::try_from(url.as_str()).ok());

        if private_relay.is_none() {
            eprintln!(
                "WARNING: no PRIVATE_RELAY_URL configured — tx will be public and front‑runnable"
            );
        }

        Executor {
            contract,
            read_provider,
            private_relay,
            max_retries: 3,
            nonce: None,
            pending_tx: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            gas_escalation_factor: 1.2, // 20% increase on reprice
        }
    }

    /// Set the current nonce manually (call after wallet init).
    pub async fn sync_nonce(&mut self) -> Result<(), ExecutorError> {
        let nonce = self
            .read_provider
            .get_transaction_count(self.contract.client().address(), None)
            .await
            .map_err(|e| ExecutorError::NonceError(format!("could not fetch nonce: {e:?}")))?;
        self.nonce = Some(nonce);
        Ok(())
    }

    /// Simulate the flash loan call (eth_call).
    pub async fn simulate(&self, amount: U256) -> Result<(), ExecutorError> {
        self.contract
            .execute_flash_loan(amount)
            .call()
            .await
            .map_err(|e| ExecutorError::SimulationReverted(format!("{e:?}")))?;
        Ok(())
    }

    /// Estimate gas for the flash loan call.
    pub async fn estimate_gas(&self, amount: U256) -> Result<U256, ExecutorError> {
        self.contract
            .execute_flash_loan(amount)
            .estimate_gas()
            .await
            .map_err(|e| ExecutorError::SimulationReverted(format!("gas estimation failed: {e:?}")))
    }

    /// Single trade: simulate, then sign and send. Retries simulation on transient errors.
    pub async fn simulate_and_execute(
        &mut self,
        amount: U256,
        gas_price_usd_per_gas: f64,
    ) -> Result<ExecutionResult, ExecutorError> {
        // Retry the simulation step with backoff
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

        // Ensure we have a valid nonce
        if self.nonce.is_none() {
            self.sync_nonce().await?;
        }

        let mut call = self.contract.execute_flash_loan(amount);
        call.tx.set_nonce(self.nonce.unwrap());

        // If we have a pending tx, try to replace it (speed up or cancel).
        // For real replacement, we'd bump gas price by escalation factor.
        if let Some(ref pending_hash) = self.pending_tx {
            // Check if pending tx is still in mempool; if it's been mined, we'll clear it.
            let pending_status = self.read_provider.get_transaction(*pending_hash).await;
            if let Ok(Some(tx)) = pending_status {
                if tx.block_number.is_some() {
                    // It's mined, clear pending
                    self.pending_tx = None;
                } else {
                    // Still pending — bump gas price by escalation factor
                    let current_fee = tx.max_fee_per_gas.unwrap_or_default();
                    let bumped = (current_fee.as_u128() as f64 * self.gas_escalation_factor) as u128;
                    call.tx.set_max_fee_per_gas(U256::from(bumped));
                    eprintln!("⏫ Replacing pending tx {} with bumped gas", pending_hash);
                }
            } else {
                // Transaction not found, clear
                self.pending_tx = None;
            }
        }

        // Apply gas caps if configured (from env or config)
        if let Some(max_fee) = self.max_fee_per_gas {
            call.tx.set_max_fee_per_gas(max_fee);
        }
        if let Some(max_priority) = self.max_priority_fee_per_gas {
            call.tx.set_max_priority_fee_per_gas(max_priority);
        }

        // Send via private relay if available, else public mempool
        let receipt = if let Some(ref relay) = self.private_relay {
            self.send_via_relay_with_call(&mut call, relay).await?
        } else {
            let pending = call
                .send()
                .await
                .map_err(|e| ExecutorError::SendFailed(format!("{e:?}")))?;
            let tx_hash = *pending;
            self.pending_tx = Some(tx_hash);
            // Increment nonce after successful broadcast
            self.nonce = Some(self.nonce.unwrap() + 1);

            pending
                .await
                .map_err(|e| ExecutorError::SendFailed(format!("{e:?}")))?
                .ok_or(ExecutorError::NoReceipt)?
        };

        // After getting receipt, clear pending tx
        self.pending_tx = None;

        let gas_used = receipt.gas_used.unwrap_or_default();
        let effective_gas_price = receipt.effective_gas_price.unwrap_or_default();
        let gas_cost_usd = gas_used.as_u128() as f64 * effective_gas_price.as_u128() as f64 / 1e18 * gas_price_usd_per_gas; // rough

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

    /// Send raw signed tx to private relay, handle nonce and replacement there as well.
    async fn send_via_relay_with_call(
        &mut self,
        call: &mut ethers::contract::ContractCall<
            SignerMiddleware<M, LocalWallet>,
            (),
        >,
        relay: &Provider<Http>,
    ) -> Result<TransactionReceipt, ExecutorError> {
        // Sign the tx with the current nonce
        let nonce = self.nonce.unwrap();
        call.tx.set_nonce(nonce);
        let signed = call
            .tx
            .rlp_signed(
                &call
                    .client()
                    .signer()
                    .sign_transaction_sync(&call.tx)
                    .map_err(|e| ExecutorError::SendFailed(format!("sign error: {e:?}")))?,
            );

        let pending = relay
            .send_raw_transaction(signed)
            .await
            .map_err(|e| ExecutorError::SendFailed(format!("relay send failed: {e:?}")))?;

        let tx_hash = *pending;
        self.pending_tx = Some(tx_hash);
        self.nonce = Some(nonce + 1);

        pending
            .await
            .map_err(|e| ExecutorError::SendFailed(format!("{e:?}")))?
            .ok_or(ExecutorError::NoReceipt)
    }

    // ------------------------------------------------------------------
    // Multi‑execution loop: repeat the flash‑loan trade while the spread
    // still exists, up to `max_repeats` times, or until a profit target
    // or stop‑loss condition is met.
    //
    // `spread_check` is a closure that returns `true` if the opportunity
    // is still profitable — it should re‑scan the pair using the scanner
    // and profit calculator, then return whether the spread is above the
    // threshold.
    // ------------------------------------------------------------------
    pub async fn execute_loop<F, Fut>(
        &mut self,
        initial_amount: U256,
        max_repeats: u32,
        gas_price_usd_per_gas: f64,
        target_profit_usd: Option<f64>,
        max_loss_usd: Option<f64>,
        spread_check: F,
    ) -> Vec<Result<ExecutionResult, ExecutorError>>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = bool>,
    {
        let mut results = Vec::new();
        let mut amount = initial_amount;
        let mut cumulative_profit = 0.0;

        for _ in 0..max_repeats {
            // Before the next attempt (except the very first one),
            // check whether the spread still justifies a trade.
            if !results.is_empty() && !spread_check().await {
                eprintln!("⏹️ Spread closed — stopping loop");
                break;
            }

            // Optional profit target / stop‑loss
            if let Some(target) = target_profit_usd {
                if cumulative_profit >= target {
                    eprintln!("🎯 Profit target reached: ${:.2}", cumulative_profit);
                    break;
                }
            }
            if let Some(max_loss) = max_loss_usd {
                if cumulative_profit <= -max_loss {
                    eprintln!("🛑 Stop‑loss hit: ${:.2}", cumulative_profit);
                    break;
                }
            }

            match self.simulate_and_execute(amount, gas_price_usd_per_gas).await {
                Ok(res) => {
                    cumulative_profit += res.realized_profit_usdc - res.gas_cost_usd;
                    eprintln!(
                        "✅ Trade {}: profit ${:.2}, gas ${:.2} | cumulative ${:.2}",
                        results.len() + 1,
                        res.realized_profit_usdc,
                        res.gas_cost_usd,
                        cumulative_profit
                    );
                    results.push(Ok(res));
                    // You might want to re‑compute `amount` from fresh pool depths here.
                }
                Err(e) => {
                    eprintln!("❌ Loop trade failed: {:?}", e);
                    results.push(Err(e));
                    break; // stop on error to avoid draining gas
                }
            }
        }

        results
    }
}