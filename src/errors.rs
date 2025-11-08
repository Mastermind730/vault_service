use thiserror::Error;

#[derive(Error, Debug)]
pub enum VaultServiceError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] mongodb::error::Error),

    #[error("Solana client error: {0}")]
    SolanaClientError(#[from] solana_client::client_error::ClientError),

    #[error("Solana program error: {0}")]
    SolanaProgramError(String),

    #[error("Anchor error: {0}")]
    AnchorError(#[from] anchor_client::ClientError),

    #[error("Invalid public key: {0}")]
    InvalidPublicKey(#[from] solana_sdk::pubkey::ParsePubkeyError),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("BSON error: {0}")]
    BsonError(#[from] bson::ser::Error),

    #[error("Vault not found: {0}")]
    VaultNotFound(String),

    #[error("Insufficient balance: available={0}, required={1}")]
    InsufficientBalance(u64, u64),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Unauthorized access")]
    Unauthorized,

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, VaultServiceError>;
