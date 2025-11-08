use crate::balance_tracker::BalanceTracker;
use crate::database::DatabaseManager;
use crate::errors::VaultServiceError;
use crate::models::*;
use crate::vault_manager::VaultManager;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;

pub struct AppState {
    pub vault_manager: Arc<VaultManager>,
    pub balance_tracker: Arc<BalanceTracker>,
    pub db: Arc<DatabaseManager>,
}

// Error response helper
impl IntoResponse for VaultServiceError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            VaultServiceError::VaultNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            VaultServiceError::InsufficientBalance(_, _) => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            VaultServiceError::InvalidAmount(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            VaultServiceError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = Json(ErrorResponse {
            error: status.to_string(),
            message,
        });

        (status, body).into_response()
    }
}

// ============ API Handlers ============

/// Initialize a new vault
pub async fn initialize_vault(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<InitializeVaultRequest>,
) -> Result<Json<TransactionResponse>, VaultServiceError> {
    let user_pubkey = Pubkey::from_str(&payload.user_pubkey)
        .map_err(|e| VaultServiceError::InvalidPublicKey(e))?;

    let vault_pubkey = state.vault_manager.initialize_vault(user_pubkey).await?;

    Ok(Json(TransactionResponse {
        signature: vault_pubkey,
        status: "success".to_string(),
    }))
}

/// Get vault balance by vault pubkey
pub async fn get_vault_balance(
    State(state): State<Arc<AppState>>,
    Path(vault_pubkey): Path<String>,
) -> Result<Json<VaultBalanceResponse>, VaultServiceError> {
    let balance = state.vault_manager.get_vault_balance(&vault_pubkey).await?;
    Ok(Json(balance))
}

/// Get vault balance by owner pubkey
pub async fn get_vault_by_owner(
    State(state): State<Arc<AppState>>,
    Path(owner_pubkey): Path<String>,
) -> Result<Json<VaultBalanceResponse>, VaultServiceError> {
    let balance = state
        .vault_manager
        .get_vault_by_owner(&owner_pubkey)
        .await?;
    Ok(Json(balance))
}

/// Record a deposit (called after on-chain transaction)
pub async fn record_deposit(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DepositRequest>,
) -> Result<Json<TransactionResponse>, VaultServiceError> {
    // In production, verify the transaction signature on-chain
    let user_pubkey = Pubkey::from_str(&payload.user_pubkey)
        .map_err(|e| VaultServiceError::InvalidPublicKey(e))?;

    let (vault_pda, _) = state.vault_manager.derive_vault_pda(&user_pubkey);

    // Simulated signature - in production, get from actual transaction
    let signature = format!("sim_{}", uuid::Uuid::new_v4());

    state
        .vault_manager
        .record_deposit(&vault_pda.to_string(), payload.amount, &signature)
        .await?;

    // Trigger balance update notification
    state
        .balance_tracker
        .monitor_vault(&vault_pda.to_string())
        .await?;

    Ok(Json(TransactionResponse {
        signature,
        status: "confirmed".to_string(),
    }))
}

/// Record a withdrawal (called after on-chain transaction)
pub async fn record_withdrawal(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<WithdrawRequest>,
) -> Result<Json<TransactionResponse>, VaultServiceError> {
    let user_pubkey = Pubkey::from_str(&payload.user_pubkey)
        .map_err(|e| VaultServiceError::InvalidPublicKey(e))?;

    let (vault_pda, _) = state.vault_manager.derive_vault_pda(&user_pubkey);

    // Simulated signature
    let signature = format!("sim_{}", uuid::Uuid::new_v4());

    state
        .vault_manager
        .record_withdrawal(&vault_pda.to_string(), payload.amount, &signature)
        .await?;

    // Trigger balance update notification
    state
        .balance_tracker
        .monitor_vault(&vault_pda.to_string())
        .await?;

    Ok(Json(TransactionResponse {
        signature,
        status: "confirmed".to_string(),
    }))
}

/// Lock collateral (internal API for position manager)
pub async fn lock_collateral(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LockCollateralRequest>,
) -> Result<Json<TransactionResponse>, VaultServiceError> {
    state
        .vault_manager
        .lock_collateral(&payload.vault_pubkey, payload.amount)
        .await?;

    // Trigger balance update notification
    state
        .balance_tracker
        .monitor_vault(&payload.vault_pubkey)
        .await?;

    Ok(Json(TransactionResponse {
        signature: "lock_success".to_string(),
        status: "confirmed".to_string(),
    }))
}

/// Unlock collateral (internal API for position manager)
pub async fn unlock_collateral(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UnlockCollateralRequest>,
) -> Result<Json<TransactionResponse>, VaultServiceError> {
    state
        .vault_manager
        .unlock_collateral(&payload.vault_pubkey, payload.amount)
        .await?;

    // Trigger balance update notification
    state
        .balance_tracker
        .monitor_vault(&payload.vault_pubkey)
        .await?;

    Ok(Json(TransactionResponse {
        signature: "unlock_success".to_string(),
        status: "confirmed".to_string(),
    }))
}

#[derive(Deserialize)]
pub struct TransactionHistoryQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    50
}

/// Get transaction history for a vault
pub async fn get_transaction_history(
    State(state): State<Arc<AppState>>,
    Path(vault_pubkey): Path<String>,
    Query(query): Query<TransactionHistoryQuery>,
) -> Result<Json<Vec<TransactionDocument>>, VaultServiceError> {
    let transactions = state
        .vault_manager
        .get_transaction_history(&vault_pubkey, query.limit)
        .await?;

    Ok(Json(transactions))
}

/// Get TVL statistics
pub async fn get_tvl(
    State(state): State<Arc<AppState>>,
) -> Result<Json<TvlResponse>, VaultServiceError> {
    let stats = state.db.calculate_tvl().await?;

    Ok(Json(TvlResponse {
        total_tvl: stats.total_tvl,
        total_locked: stats.total_locked,
        total_available: stats.total_available,
        vault_count: stats.vault_count,
        timestamp: stats.timestamp.to_rfc3339(),
    }))
}

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
