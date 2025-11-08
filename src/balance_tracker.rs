use crate::database::DatabaseManager;
use crate::errors::Result;
use crate::models::{BalanceSnapshot, SnapshotType, VaultDocument, WsMessage};
use chrono::Utc;
use solana_client::rpc_client::RpcClient;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};

pub struct BalanceTracker {
    db: Arc<DatabaseManager>,
    rpc_client: Arc<RpcClient>,
    ws_sender: broadcast::Sender<WsMessage>,
}

impl BalanceTracker {
    pub fn new(
        db: Arc<DatabaseManager>,
        rpc_client: Arc<RpcClient>,
        ws_sender: broadcast::Sender<WsMessage>,
    ) -> Self {
        Self {
            db,
            rpc_client,
            ws_sender,
        }
    }

    /// Start balance monitoring service
    pub async fn start_monitoring(&self) -> Result<()> {
        let tracker = self.clone();
        
        tokio::spawn(async move {
            let mut hourly_interval = interval(Duration::from_secs(3600)); // 1 hour
            let mut daily_interval = interval(Duration::from_secs(86400)); // 24 hours
            let mut reconcile_interval = interval(Duration::from_secs(300)); // 5 minutes

            loop {
                tokio::select! {
                    _ = hourly_interval.tick() => {
                        if let Err(e) = tracker.create_hourly_snapshots().await {
                            log::error!("Failed to create hourly snapshots: {}", e);
                        }
                    }
                    _ = daily_interval.tick() => {
                        if let Err(e) = tracker.create_daily_snapshots().await {
                            log::error!("Failed to create daily snapshots: {}", e);
                        }
                    }
                    _ = reconcile_interval.tick() => {
                        if let Err(e) = tracker.reconcile_balances().await {
                            log::error!("Failed to reconcile balances: {}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Create hourly balance snapshots for all vaults
    async fn create_hourly_snapshots(&self) -> Result<()> {
        log::info!("Creating hourly balance snapshots");
        
        let vaults = self.db.get_all_vaults().await?;
        
        for vault in vaults {
            let snapshot = BalanceSnapshot {
                id: uuid::Uuid::new_v4().to_string(),
                vault: vault.id.clone(),
                total_balance: vault.total_balance,
                locked_balance: vault.locked_balance,
                available_balance: vault.available_balance,
                timestamp: Utc::now(),
                snapshot_type: SnapshotType::Hourly,
            };
            
            self.db.insert_snapshot(snapshot).await?;
        }
        
        log::info!("Created {} hourly snapshots", vaults.len());
        Ok(())
    }

    /// Create daily balance snapshots for all vaults
    async fn create_daily_snapshots(&self) -> Result<()> {
        log::info!("Creating daily balance snapshots");
        
        let vaults = self.db.get_all_vaults().await?;
        
        for vault in vaults {
            let snapshot = BalanceSnapshot {
                id: uuid::Uuid::new_v4().to_string(),
                vault: vault.id.clone(),
                total_balance: vault.total_balance,
                locked_balance: vault.locked_balance,
                available_balance: vault.available_balance,
                timestamp: Utc::now(),
                snapshot_type: SnapshotType::Daily,
            };
            
            self.db.insert_snapshot(snapshot).await?;
        }
        
        log::info!("Created {} daily snapshots", vaults.len());
        Ok(())
    }

    /// Reconcile off-chain balances with on-chain state
    async fn reconcile_balances(&self) -> Result<()> {
        log::debug!("Starting balance reconciliation");
        
        let vaults = self.db.get_all_vaults().await?;
        let mut discrepancies = 0;
        
        for vault in vaults {
            // In production, fetch actual on-chain balance
            // For now, we'll just validate internal consistency
            if !self.validate_vault_balance(&vault) {
                log::warn!("Balance discrepancy detected for vault: {}", vault.id);
                discrepancies += 1;
                
                // Emit alert via WebSocket
                let _ = self.ws_sender.send(WsMessage::Error {
                    message: format!("Balance discrepancy in vault {}", vault.id),
                });
            }
        }
        
        if discrepancies > 0 {
            log::warn!("Found {} vaults with balance discrepancies", discrepancies);
        } else {
            log::debug!("Balance reconciliation completed successfully");
        }
        
        Ok(())
    }

    /// Validate vault balance consistency
    fn validate_vault_balance(&self, vault: &VaultDocument) -> bool {
        // Check that total = locked + available
        if vault.total_balance != vault.locked_balance + vault.available_balance {
            log::error!(
                "Vault {} balance mismatch: total={}, locked={}, available={}",
                vault.id,
                vault.total_balance,
                vault.locked_balance,
                vault.available_balance
            );
            return false;
        }
        
        true
    }

    /// Monitor a specific vault and broadcast updates
    pub async fn monitor_vault(&self, vault_pubkey: &str) -> Result<()> {
        let vault = self
            .db
            .get_vault(vault_pubkey)
            .await?
            .ok_or_else(|| crate::errors::VaultServiceError::VaultNotFound(vault_pubkey.to_string()))?;

        // Broadcast current balance
        let _ = self.ws_sender.send(WsMessage::BalanceUpdate {
            vault: vault.id.clone(),
            total_balance: vault.total_balance,
            locked_balance: vault.locked_balance,
            available_balance: vault.available_balance,
        });

        Ok(())
    }

    /// Get balance statistics for a vault
    pub async fn get_balance_stats(
        &self,
        vault_pubkey: &str,
        days: i64,
    ) -> Result<Vec<BalanceSnapshot>> {
        let snapshots = self.db.get_vault_snapshots(vault_pubkey, days * 24).await?;
        Ok(snapshots)
    }

    /// Detect unusual activity
    pub async fn detect_unusual_activity(&self, vault_pubkey: &str) -> Result<bool> {
        let transactions = self.db.get_vault_transactions(vault_pubkey, 100).await?;
        
        // Simple heuristic: check for rapid succession of large transactions
        if transactions.len() > 10 {
            let recent = &transactions[0..10];
            let avg_amount: u64 = recent.iter().map(|t| t.amount).sum::<u64>() / 10;
            
            // Check if any transaction is 10x the average
            for tx in recent {
                if tx.amount > avg_amount * 10 {
                    log::warn!(
                        "Unusual transaction detected in vault {}: {} tokens",
                        vault_pubkey,
                        tx.amount
                    );
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }

    /// Calculate TVL and broadcast update
    pub async fn update_tvl(&self) -> Result<()> {
        let stats = self.db.calculate_tvl().await?;
        
        // Save to database
        self.db.save_tvl_stats(stats.clone()).await?;
        
        // Broadcast to WebSocket clients
        let _ = self.ws_sender.send(WsMessage::TvlUpdate {
            total_tvl: stats.total_tvl,
            vault_count: stats.vault_count,
        });
        
        log::info!(
            "TVL updated: {} USDT across {} vaults",
            stats.total_tvl,
            stats.vault_count
        );
        
        Ok(())
    }
}

impl Clone for BalanceTracker {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            rpc_client: Arc::clone(&self.rpc_client),
            ws_sender: self.ws_sender.clone(),
        }
    }
}
