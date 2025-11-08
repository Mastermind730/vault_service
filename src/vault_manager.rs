use crate::config::Config;
use crate::database::DatabaseManager;
use crate::errors::{Result, VaultServiceError};
use crate::models::*;
use anchor_client::{Client, Program};
use chrono::Utc;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;

pub struct VaultManager {
    config: Arc<Config>,
    rpc_client: Arc<RpcClient>,
    db: Arc<DatabaseManager>,
    program_id: Pubkey,
    usdt_mint: Pubkey,
}

impl VaultManager {
    pub fn new(
        config: Arc<Config>,
        rpc_client: Arc<RpcClient>,
        db: Arc<DatabaseManager>,
    ) -> Result<Self> {
        let program_id = Pubkey::from_str(&config.vault_program.program_id)
            .map_err(|e| VaultServiceError::ConfigError(format!("Invalid program ID: {}", e)))?;

        let usdt_mint = Pubkey::from_str(&config.vault_program.usdt_mint)
            .map_err(|e| VaultServiceError::ConfigError(format!("Invalid USDT mint: {}", e)))?;

        Ok(Self {
            config,
            rpc_client,
            db,
            program_id,
            usdt_mint,
        })
    }

    /// Derive vault PDA for a user
    pub fn derive_vault_pda(&self, user: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vault", user.as_ref()], &self.program_id)
    }

    /// Derive authority PDA
    pub fn derive_authority_pda(&self) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"authority"], &self.program_id)
    }

    /// Initialize a new vault for a user
    pub async fn initialize_vault(&self, user_pubkey: Pubkey) -> Result<String> {
        let (vault_pda, _bump) = self.derive_vault_pda(&user_pubkey);

        // Check if vault already exists
        if let Some(_) = self.db.get_vault(&vault_pda.to_string()).await? {
            return Err(VaultServiceError::InternalError(
                "Vault already exists".to_string(),
            ));
        }

        // Build transaction (simplified - in production, this would use anchor_client)
        // For now, we'll just create the database entry
        let vault_doc = VaultDocument {
            id: vault_pda.to_string(),
            owner: user_pubkey.to_string(),
            token_account: "".to_string(), // Will be set after on-chain initialization
            total_balance: 0,
            locked_balance: 0,
            available_balance: 0,
            total_deposited: 0,
            total_withdrawn: 0,
            created_at: Utc::now(),
            last_updated: Utc::now(),
            bump: 0, // Will be set after on-chain initialization
            status: VaultStatus::Active,
        };

        self.db.insert_vault(vault_doc).await?;

        // Log audit
        self.log_audit(
            Some(vault_pda.to_string()),
            Some(user_pubkey.to_string()),
            "initialize_vault".to_string(),
            serde_json::json!({ "vault": vault_pda.to_string() }),
            true,
        )
        .await?;

        Ok(vault_pda.to_string())
    }

    /// Get vault balance
    pub async fn get_vault_balance(&self, vault_pubkey: &str) -> Result<VaultBalanceResponse> {
        let vault = self
            .db
            .get_vault(vault_pubkey)
            .await?
            .ok_or_else(|| VaultServiceError::VaultNotFound(vault_pubkey.to_string()))?;

        Ok(VaultBalanceResponse {
            vault: vault.id,
            owner: vault.owner,
            total_balance: vault.total_balance,
            locked_balance: vault.locked_balance,
            available_balance: vault.available_balance,
            total_deposited: vault.total_deposited,
            total_withdrawn: vault.total_withdrawn,
        })
    }

    /// Get vault balance by owner
    pub async fn get_vault_by_owner(&self, owner_pubkey: &str) -> Result<VaultBalanceResponse> {
        let vault = self
            .db
            .get_vault_by_owner(owner_pubkey)
            .await?
            .ok_or_else(|| VaultServiceError::VaultNotFound(owner_pubkey.to_string()))?;

        Ok(VaultBalanceResponse {
            vault: vault.id,
            owner: vault.owner,
            total_balance: vault.total_balance,
            locked_balance: vault.locked_balance,
            available_balance: vault.available_balance,
            total_deposited: vault.total_deposited,
            total_withdrawn: vault.total_withdrawn,
        })
    }

    /// Record a deposit (called after on-chain transaction)
    pub async fn record_deposit(
        &self,
        vault_pubkey: &str,
        amount: u64,
        signature: &str,
    ) -> Result<()> {
        let vault = self
            .db
            .get_vault(vault_pubkey)
            .await?
            .ok_or_else(|| VaultServiceError::VaultNotFound(vault_pubkey.to_string()))?;

        // Update vault balances
        let new_total = vault.total_balance + amount;
        let new_available = vault.available_balance + amount;
        let new_deposited = vault.total_deposited + amount;

        self.db
            .update_vault_stats(
                vault_pubkey,
                new_total,
                vault.locked_balance,
                new_available,
                Some(new_deposited),
                None,
            )
            .await?;

        // Record transaction
        let transaction = TransactionDocument {
            id: uuid::Uuid::new_v4().to_string(),
            vault: vault_pubkey.to_string(),
            transaction_type: TransactionType::Deposit,
            amount,
            signature: Some(signature.to_string()),
            timestamp: Utc::now(),
            from_vault: None,
            to_vault: None,
            status: TransactionStatus::Confirmed,
            error_message: None,
        };

        self.db.insert_transaction(transaction).await?;

        // Create snapshot
        self.create_snapshot(vault_pubkey, SnapshotType::OnDemand)
            .await?;

        Ok(())
    }

    /// Record a withdrawal (called after on-chain transaction)
    pub async fn record_withdrawal(
        &self,
        vault_pubkey: &str,
        amount: u64,
        signature: &str,
    ) -> Result<()> {
        let vault = self
            .db
            .get_vault(vault_pubkey)
            .await?
            .ok_or_else(|| VaultServiceError::VaultNotFound(vault_pubkey.to_string()))?;

        // Verify sufficient balance
        if vault.available_balance < amount {
            return Err(VaultServiceError::InsufficientBalance(
                vault.available_balance,
                amount,
            ));
        }

        // Update vault balances
        let new_total = vault.total_balance - amount;
        let new_available = vault.available_balance - amount;
        let new_withdrawn = vault.total_withdrawn + amount;

        self.db
            .update_vault_stats(
                vault_pubkey,
                new_total,
                vault.locked_balance,
                new_available,
                None,
                Some(new_withdrawn),
            )
            .await?;

        // Record transaction
        let transaction = TransactionDocument {
            id: uuid::Uuid::new_v4().to_string(),
            vault: vault_pubkey.to_string(),
            transaction_type: TransactionType::Withdrawal,
            amount,
            signature: Some(signature.to_string()),
            timestamp: Utc::now(),
            from_vault: None,
            to_vault: None,
            status: TransactionStatus::Confirmed,
            error_message: None,
        };

        self.db.insert_transaction(transaction).await?;

        // Create snapshot
        self.create_snapshot(vault_pubkey, SnapshotType::OnDemand)
            .await?;

        Ok(())
    }

    /// Lock collateral (called from position manager)
    pub async fn lock_collateral(&self, vault_pubkey: &str, amount: u64) -> Result<()> {
        let vault = self
            .db
            .get_vault(vault_pubkey)
            .await?
            .ok_or_else(|| VaultServiceError::VaultNotFound(vault_pubkey.to_string()))?;

        // Verify sufficient available balance
        if vault.available_balance < amount {
            return Err(VaultServiceError::InsufficientBalance(
                vault.available_balance,
                amount,
            ));
        }

        // Update balances
        let new_locked = vault.locked_balance + amount;
        let new_available = vault.available_balance - amount;

        self.db
            .update_vault_balance(vault_pubkey, vault.total_balance, new_locked, new_available)
            .await?;

        // Record transaction
        let transaction = TransactionDocument {
            id: uuid::Uuid::new_v4().to_string(),
            vault: vault_pubkey.to_string(),
            transaction_type: TransactionType::Lock,
            amount,
            signature: None,
            timestamp: Utc::now(),
            from_vault: None,
            to_vault: None,
            status: TransactionStatus::Confirmed,
            error_message: None,
        };

        self.db.insert_transaction(transaction).await?;

        Ok(())
    }

    /// Unlock collateral (called when position is closed)
    pub async fn unlock_collateral(&self, vault_pubkey: &str, amount: u64) -> Result<()> {
        let vault = self
            .db
            .get_vault(vault_pubkey)
            .await?
            .ok_or_else(|| VaultServiceError::VaultNotFound(vault_pubkey.to_string()))?;

        // Verify sufficient locked balance
        if vault.locked_balance < amount {
            return Err(VaultServiceError::InternalError(format!(
                "Cannot unlock {} tokens, only {} locked",
                amount, vault.locked_balance
            )));
        }

        // Update balances
        let new_locked = vault.locked_balance - amount;
        let new_available = vault.available_balance + amount;

        self.db
            .update_vault_balance(vault_pubkey, vault.total_balance, new_locked, new_available)
            .await?;

        // Record transaction
        let transaction = TransactionDocument {
            id: uuid::Uuid::new_v4().to_string(),
            vault: vault_pubkey.to_string(),
            transaction_type: TransactionType::Unlock,
            amount,
            signature: None,
            timestamp: Utc::now(),
            from_vault: None,
            to_vault: None,
            status: TransactionStatus::Confirmed,
            error_message: None,
        };

        self.db.insert_transaction(transaction).await?;

        Ok(())
    }

    /// Get transaction history
    pub async fn get_transaction_history(
        &self,
        vault_pubkey: &str,
        limit: i64,
    ) -> Result<Vec<TransactionDocument>> {
        self.db.get_vault_transactions(vault_pubkey, limit).await
    }

    /// Create balance snapshot
    async fn create_snapshot(
        &self,
        vault_pubkey: &str,
        snapshot_type: SnapshotType,
    ) -> Result<()> {
        let vault = self
            .db
            .get_vault(vault_pubkey)
            .await?
            .ok_or_else(|| VaultServiceError::VaultNotFound(vault_pubkey.to_string()))?;

        let snapshot = BalanceSnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            vault: vault_pubkey.to_string(),
            total_balance: vault.total_balance,
            locked_balance: vault.locked_balance,
            available_balance: vault.available_balance,
            timestamp: Utc::now(),
            snapshot_type,
        };

        self.db.insert_snapshot(snapshot).await?;
        Ok(())
    }

    /// Log audit entry
    async fn log_audit(
        &self,
        vault: Option<String>,
        user: Option<String>,
        action: String,
        details: serde_json::Value,
        success: bool,
    ) -> Result<()> {
        let log = AuditLog {
            id: uuid::Uuid::new_v4().to_string(),
            vault,
            user,
            action,
            details,
            ip_address: None,
            timestamp: Utc::now(),
            success,
        };

        self.db.insert_audit_log(log).await?;
        Ok(())
    }
}
