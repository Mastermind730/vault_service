use crate::config::Config;
use crate::errors::{Result, VaultServiceError};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use std::sync::Arc;

pub struct TransactionBuilder {
    rpc_client: Arc<RpcClient>,
    config: Arc<Config>,
}

impl TransactionBuilder {
    pub fn new(rpc_client: Arc<RpcClient>, config: Arc<Config>) -> Self {
        Self { rpc_client, config }
    }

    /// Build and send a transaction with compute budget
    pub async fn build_and_send(
        &self,
        instructions: Vec<Instruction>,
        signers: &[&Keypair],
    ) -> Result<Signature> {
        // Add compute budget instructions
        let mut all_instructions = vec![
            ComputeBudgetInstruction::set_compute_unit_limit(300_000),
            ComputeBudgetInstruction::set_compute_unit_price(1),
        ];
        all_instructions.extend(instructions);

        // Get recent blockhash
        let recent_blockhash = self
            .rpc_client
            .get_latest_blockhash()
            .map_err(|e| VaultServiceError::SolanaClientError(e))?;

        // Create transaction
        let mut transaction = Transaction::new_with_payer(&all_instructions, Some(&signers[0].pubkey()));
        transaction.sign(signers, recent_blockhash);

        // Send transaction
        let signature = self
            .rpc_client
            .send_and_confirm_transaction_with_spinner(&transaction)
            .map_err(|e| VaultServiceError::TransactionFailed(e.to_string()))?;

        log::info!("Transaction sent: {}", signature);
        Ok(signature)
    }

    /// Simulate transaction before sending
    pub async fn simulate_transaction(
        &self,
        instructions: Vec<Instruction>,
        signers: &[&Keypair],
    ) -> Result<()> {
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;

        let mut transaction = Transaction::new_with_payer(&instructions, Some(&signers[0].pubkey()));
        transaction.sign(signers, recent_blockhash);

        let result = self.rpc_client.simulate_transaction(&transaction)?;

        if let Some(err) = result.value.err {
            return Err(VaultServiceError::TransactionFailed(format!(
                "Simulation failed: {:?}",
                err
            )));
        }

        log::debug!("Transaction simulation successful");
        Ok(())
    }

    /// Get transaction status
    pub async fn get_transaction_status(&self, signature: &Signature) -> Result<bool> {
        match self.rpc_client.get_signature_status(signature)? {
            Some(result) => match result {
                Ok(_) => Ok(true),
                Err(e) => Err(VaultServiceError::TransactionFailed(format!(
                    "Transaction failed: {:?}",
                    e
                ))),
            },
            None => Ok(false),
        }
    }

    /// Wait for transaction confirmation
    pub async fn confirm_transaction(
        &self,
        signature: &Signature,
        max_retries: u32,
    ) -> Result<()> {
        for attempt in 0..max_retries {
            match self.rpc_client.get_signature_status(signature)? {
                Some(result) => match result {
                    Ok(_) => {
                        log::info!("Transaction confirmed: {}", signature);
                        return Ok(());
                    }
                    Err(e) => {
                        return Err(VaultServiceError::TransactionFailed(format!(
                            "Transaction failed: {:?}",
                            e
                        )))
                    }
                },
                None => {
                    if attempt < max_retries - 1 {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    }
                }
            }
        }

        Err(VaultServiceError::TransactionFailed(
            "Transaction confirmation timeout".to_string(),
        ))
    }
}
