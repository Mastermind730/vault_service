use crate::api::handlers::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub mod handlers;

pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health check
        .route("/health", get(handlers::health_check))
        // Vault operations
        .route("/vault/initialize", post(handlers::initialize_vault))
        .route("/vault/balance/:vault", get(handlers::get_vault_balance))
        .route("/vault/owner/:owner", get(handlers::get_vault_by_owner))
        .route("/vault/deposit", post(handlers::record_deposit))
        .route("/vault/withdraw", post(handlers::record_withdrawal))
        .route(
            "/vault/transactions/:vault",
            get(handlers::get_transaction_history),
        )
        // Internal operations (for position manager)
        .route("/internal/lock", post(handlers::lock_collateral))
        .route("/internal/unlock", post(handlers::unlock_collateral))
        // Analytics
        .route("/analytics/tvl", get(handlers::get_tvl))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
