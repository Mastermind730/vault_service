use std::sync::Arc;
use solana_client::rpc_client::RpcClient;
use tokio::net::TcpListener;

mod api;
mod balance_tracker;
mod config;
mod database;
mod errors;
mod models;
mod transaction_builder;
mod vault_manager;
mod websocket;

use api::handlers::AppState;
use balance_tracker::BalanceTracker;
use config::Config;
use database::DatabaseManager;
use vault_manager::VaultManager;
use websocket::WebSocketManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    log::info!("Starting Vault Manager Service");

    // Load configuration
    let config = Arc::new(Config::from_env()?);
    log::info!("Configuration loaded");
    log::info!("MongoDB URI: {}", config.mongodb.uri);
    log::info!("Solana RPC: {}", config.solana.rpc_url);

    // Initialize database
    let db = Arc::new(DatabaseManager::new(&config.mongodb).await?);
    log::info!("Database connection established");

    // Initialize Solana RPC client
    let rpc_client = Arc::new(RpcClient::new(config.solana.rpc_url.clone()));
    log::info!("Solana RPC client initialized");

    // Initialize WebSocket manager
    let (ws_manager, ws_sender) = WebSocketManager::new();
    let ws_sender = Arc::new(ws_sender);
    log::info!("WebSocket manager initialized");

    // Initialize vault manager
    let vault_manager = Arc::new(VaultManager::new(
        Arc::clone(&config),
        Arc::clone(&rpc_client),
        Arc::clone(&db),
    )?);
    log::info!("Vault manager initialized");

    // Initialize balance tracker
    let balance_tracker = Arc::new(BalanceTracker::new(
        Arc::clone(&db),
        Arc::clone(&rpc_client),
        ws_sender.clone(),
    ));
    log::info!("Balance tracker initialized");

    // Start balance monitoring
    balance_tracker.start_monitoring().await?;
    log::info!("Balance monitoring started");

    // Start periodic TVL updates
    let balance_tracker_clone = Arc::clone(&balance_tracker);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(e) = balance_tracker_clone.update_tvl().await {
                log::error!("Failed to update TVL: {}", e);
            }
        }
    });

    // Create application state
    let app_state = Arc::new(AppState {
        vault_manager: Arc::clone(&vault_manager),
        balance_tracker: Arc::clone(&balance_tracker),
        db: Arc::clone(&db),
    });

    // Create router with WebSocket support
    let app = api::create_router(app_state)
        .route("/ws", axum::routing::get(websocket::ws_handler))
        .with_state(ws_sender);

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    log::info!("Starting server on {}", addr);
    
    let listener = TcpListener::bind(&addr).await?;
    log::info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

