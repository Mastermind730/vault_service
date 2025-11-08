use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Vault model for MongoDB storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultDocument {
    #[serde(rename = "_id")]
    pub id: String, // vault pubkey as string
    pub owner: String, // owner pubkey as string
    pub token_account: String,
    pub total_balance: u64,
    pub locked_balance: u64,
    pub available_balance: u64,
    pub total_deposited: u64,
    pub total_withdrawn: u64,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub bump: u8,
    pub status: VaultStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VaultStatus {
    Active,
    Suspended,
    Closed,
}

/// Transaction record for MongoDB storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDocument {
    #[serde(rename = "_id")]
    pub id: String, // MongoDB ObjectId or UUID
    pub vault: String,
    pub transaction_type: TransactionType,
    pub amount: u64,
    pub signature: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub from_vault: Option<String>,
    pub to_vault: Option<String>,
    pub status: TransactionStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Lock,
    Unlock,
    Transfer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

/// Balance snapshot for historical tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSnapshot {
    #[serde(rename = "_id")]
    pub id: String,
    pub vault: String,
    pub total_balance: u64,
    pub locked_balance: u64,
    pub available_balance: u64,
    pub timestamp: DateTime<Utc>,
    pub snapshot_type: SnapshotType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotType {
    Hourly,
    Daily,
    OnDemand,
}

/// Audit log for security tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    #[serde(rename = "_id")]
    pub id: String,
    pub vault: Option<String>,
    pub user: Option<String>,
    pub action: String,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
}

/// TVL (Total Value Locked) statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvlStats {
    #[serde(rename = "_id")]
    pub id: String,
    pub total_tvl: u64,
    pub total_locked: u64,
    pub total_available: u64,
    pub vault_count: u64,
    pub timestamp: DateTime<Utc>,
}

// ============ API Request/Response Models ============

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeVaultRequest {
    pub user_pubkey: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DepositRequest {
    pub user_pubkey: String,
    pub amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawRequest {
    pub user_pubkey: String,
    pub amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LockCollateralRequest {
    pub vault_pubkey: String,
    pub amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnlockCollateralRequest {
    pub vault_pubkey: String,
    pub amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransferCollateralRequest {
    pub from_vault: String,
    pub to_vault: String,
    pub amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultBalanceResponse {
    pub vault: String,
    pub owner: String,
    pub total_balance: u64,
    pub locked_balance: u64,
    pub available_balance: u64,
    pub total_deposited: u64,
    pub total_withdrawn: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub signature: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TvlResponse {
    pub total_tvl: u64,
    pub total_locked: u64,
    pub total_available: u64,
    pub vault_count: u64,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

// ============ WebSocket Messages ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "balance_update")]
    BalanceUpdate {
        vault: String,
        total_balance: u64,
        locked_balance: u64,
        available_balance: u64,
    },
    #[serde(rename = "deposit")]
    Deposit {
        vault: String,
        amount: u64,
        signature: String,
    },
    #[serde(rename = "withdrawal")]
    Withdrawal {
        vault: String,
        amount: u64,
        signature: String,
    },
    #[serde(rename = "lock")]
    Lock {
        vault: String,
        amount: u64,
    },
    #[serde(rename = "unlock")]
    Unlock {
        vault: String,
        amount: u64,
    },
    #[serde(rename = "tvl_update")]
    TvlUpdate {
        total_tvl: u64,
        vault_count: u64,
    },
    #[serde(rename = "error")]
    Error {
        message: String,
    },
}
