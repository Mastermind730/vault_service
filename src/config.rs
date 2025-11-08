use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub solana: SolanaConfig,
    pub mongodb: MongoDbConfig,
    pub server: ServerConfig,
    pub vault_program: VaultProgramConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaConfig {
    pub rpc_url: String,
    pub ws_url: String,
    pub commitment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoDbConfig {
    pub uri: String,
    pub database: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultProgramConfig {
    pub program_id: String,
    pub usdt_mint: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv::dotenv().ok();

        Ok(Config {
            solana: SolanaConfig {
                rpc_url: env::var("SOLANA_RPC_URL")
                    .unwrap_or_else(|_| "http://localhost:8899".to_string()),
                ws_url: env::var("SOLANA_WS_URL")
                    .unwrap_or_else(|_| "ws://localhost:8900".to_string()),
                commitment: env::var("SOLANA_COMMITMENT")
                    .unwrap_or_else(|_| "confirmed".to_string()),
            },
            mongodb: MongoDbConfig {
                uri: env::var("MONGODB_URI")
                    .unwrap_or_else(|_| "mongodb://localhost:27017".to_string()),
                database: env::var("MONGODB_DATABASE")
                    .unwrap_or_else(|_| "vault_manager".to_string()),
            },
            server: ServerConfig {
                host: env::var("SERVER_HOST")
                    .unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()
                    .unwrap_or(8080),
            },
            vault_program: VaultProgramConfig {
                program_id: env::var("VAULT_PROGRAM_ID")
                    .unwrap_or_else(|_| "VAULTmngr11111111111111111111111111111111".to_string()),
                usdt_mint: env::var("USDT_MINT")
                    .expect("USDT_MINT must be set"),
            },
        })
    }
}
