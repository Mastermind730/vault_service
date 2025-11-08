use crate::config::MongoDbConfig;
use crate::errors::{Result, VaultServiceError};
use crate::models::*;
use chrono::Utc;
use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    Client, Collection, Database,
};

#[derive(Clone)]
pub struct DatabaseManager {
    client: Client,
    db: Database,
}

impl DatabaseManager {
    pub async fn new(config: &MongoDbConfig) -> Result<Self> {
        let client_options = ClientOptions::parse(&config.uri).await?;
        let client = Client::with_options(client_options)?;
        let db = client.database(&config.database);

        // Create indexes
        let manager = Self { client, db };
        manager.create_indexes().await?;

        Ok(manager)
    }

    async fn create_indexes(&self) -> Result<()> {
        use mongodb::options::IndexOptions;
        use mongodb::IndexModel;

        // Vaults collection indexes
        let vaults: Collection<VaultDocument> = self.db.collection("vaults");
        vaults
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "owner": 1 })
                    .options(IndexOptions::builder().unique(false).build())
                    .build(),
                None,
            )
            .await?;

        // Transactions collection indexes
        let transactions: Collection<TransactionDocument> = self.db.collection("transactions");
        transactions
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "vault": 1, "timestamp": -1 })
                    .build(),
                None,
            )
            .await?;

        transactions
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "signature": 1 })
                    .options(IndexOptions::builder().unique(true).sparse(true).build())
                    .build(),
                None,
            )
            .await?;

        // Balance snapshots indexes
        let snapshots: Collection<BalanceSnapshot> = self.db.collection("balance_snapshots");
        snapshots
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "vault": 1, "timestamp": -1 })
                    .build(),
                None,
            )
            .await?;

        // Audit logs indexes
        let audit: Collection<AuditLog> = self.db.collection("audit_logs");
        audit
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "timestamp": -1 })
                    .build(),
                None,
            )
            .await?;

        Ok(())
    }

    // ============ Vault Operations ============

    pub async fn insert_vault(&self, vault: VaultDocument) -> Result<()> {
        let collection: Collection<VaultDocument> = self.db.collection("vaults");
        collection.insert_one(vault, None).await?;
        Ok(())
    }

    pub async fn get_vault(&self, vault_pubkey: &str) -> Result<Option<VaultDocument>> {
        let collection: Collection<VaultDocument> = self.db.collection("vaults");
        let vault = collection
            .find_one(doc! { "_id": vault_pubkey }, None)
            .await?;
        Ok(vault)
    }

    pub async fn get_vault_by_owner(&self, owner_pubkey: &str) -> Result<Option<VaultDocument>> {
        let collection: Collection<VaultDocument> = self.db.collection("vaults");
        let vault = collection
            .find_one(doc! { "owner": owner_pubkey }, None)
            .await?;
        Ok(vault)
    }

    pub async fn update_vault_balance(
        &self,
        vault_pubkey: &str,
        total_balance: u64,
        locked_balance: u64,
        available_balance: u64,
    ) -> Result<()> {
        let collection: Collection<VaultDocument> = self.db.collection("vaults");
        collection
            .update_one(
                doc! { "_id": vault_pubkey },
                doc! {
                    "$set": {
                        "total_balance": total_balance as i64,
                        "locked_balance": locked_balance as i64,
                        "available_balance": available_balance as i64,
                        "last_updated": Utc::now(),
                    }
                },
                None,
            )
            .await?;
        Ok(())
    }

    pub async fn update_vault_stats(
        &self,
        vault_pubkey: &str,
        total_balance: u64,
        locked_balance: u64,
        available_balance: u64,
        total_deposited: Option<u64>,
        total_withdrawn: Option<u64>,
    ) -> Result<()> {
        let collection: Collection<VaultDocument> = self.db.collection("vaults");
        
        let mut update_doc = doc! {
            "total_balance": total_balance as i64,
            "locked_balance": locked_balance as i64,
            "available_balance": available_balance as i64,
            "last_updated": Utc::now(),
        };

        if let Some(deposited) = total_deposited {
            update_doc.insert("total_deposited", deposited as i64);
        }
        if let Some(withdrawn) = total_withdrawn {
            update_doc.insert("total_withdrawn", withdrawn as i64);
        }

        collection
            .update_one(
                doc! { "_id": vault_pubkey },
                doc! { "$set": update_doc },
                None,
            )
            .await?;
        Ok(())
    }

    pub async fn get_all_vaults(&self) -> Result<Vec<VaultDocument>> {
        use futures::stream::TryStreamExt;

        let collection: Collection<VaultDocument> = self.db.collection("vaults");
        let cursor = collection.find(None, None).await?;
        let vaults: Vec<VaultDocument> = cursor.try_collect().await?;
        Ok(vaults)
    }

    // ============ Transaction Operations ============

    pub async fn insert_transaction(&self, transaction: TransactionDocument) -> Result<()> {
        let collection: Collection<TransactionDocument> = self.db.collection("transactions");
        collection.insert_one(transaction, None).await?;
        Ok(())
    }

    pub async fn update_transaction_status(
        &self,
        transaction_id: &str,
        status: TransactionStatus,
        signature: Option<String>,
        error_message: Option<String>,
    ) -> Result<()> {
        let collection: Collection<TransactionDocument> = self.db.collection("transactions");
        
        let mut update_doc = doc! {
            "status": bson::to_bson(&status)?,
        };

        if let Some(sig) = signature {
            update_doc.insert("signature", sig);
        }
        if let Some(err) = error_message {
            update_doc.insert("error_message", err);
        }

        collection
            .update_one(
                doc! { "_id": transaction_id },
                doc! { "$set": update_doc },
                None,
            )
            .await?;
        Ok(())
    }

    pub async fn get_vault_transactions(
        &self,
        vault_pubkey: &str,
        limit: i64,
    ) -> Result<Vec<TransactionDocument>> {
        use futures::stream::TryStreamExt;
        use mongodb::options::FindOptions;

        let collection: Collection<TransactionDocument> = self.db.collection("transactions");
        let options = FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .limit(limit)
            .build();

        let cursor = collection
            .find(doc! { "vault": vault_pubkey }, options)
            .await?;
        let transactions: Vec<TransactionDocument> = cursor.try_collect().await?;
        Ok(transactions)
    }

    // ============ Balance Snapshot Operations ============

    pub async fn insert_snapshot(&self, snapshot: BalanceSnapshot) -> Result<()> {
        let collection: Collection<BalanceSnapshot> = self.db.collection("balance_snapshots");
        collection.insert_one(snapshot, None).await?;
        Ok(())
    }

    pub async fn get_vault_snapshots(
        &self,
        vault_pubkey: &str,
        limit: i64,
    ) -> Result<Vec<BalanceSnapshot>> {
        use futures::stream::TryStreamExt;
        use mongodb::options::FindOptions;

        let collection: Collection<BalanceSnapshot> = self.db.collection("balance_snapshots");
        let options = FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .limit(limit)
            .build();

        let cursor = collection
            .find(doc! { "vault": vault_pubkey }, options)
            .await?;
        let snapshots: Vec<BalanceSnapshot> = cursor.try_collect().await?;
        Ok(snapshots)
    }

    // ============ Audit Log Operations ============

    pub async fn insert_audit_log(&self, log: AuditLog) -> Result<()> {
        let collection: Collection<AuditLog> = self.db.collection("audit_logs");
        collection.insert_one(log, None).await?;
        Ok(())
    }

    pub async fn get_recent_audit_logs(&self, limit: i64) -> Result<Vec<AuditLog>> {
        use futures::stream::TryStreamExt;
        use mongodb::options::FindOptions;

        let collection: Collection<AuditLog> = self.db.collection("audit_logs");
        let options = FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .limit(limit)
            .build();

        let cursor = collection.find(None, options).await?;
        let logs: Vec<AuditLog> = cursor.try_collect().await?;
        Ok(logs)
    }

    // ============ TVL Operations ============

    pub async fn calculate_tvl(&self) -> Result<TvlStats> {
        use futures::stream::TryStreamExt;

        let collection: Collection<VaultDocument> = self.db.collection("vaults");
        let vaults: Vec<VaultDocument> = collection.find(None, None).await?.try_collect().await?;

        let mut total_tvl = 0u64;
        let mut total_locked = 0u64;
        let mut total_available = 0u64;

        for vault in &vaults {
            total_tvl += vault.total_balance;
            total_locked += vault.locked_balance;
            total_available += vault.available_balance;
        }

        Ok(TvlStats {
            id: uuid::Uuid::new_v4().to_string(),
            total_tvl,
            total_locked,
            total_available,
            vault_count: vaults.len() as u64,
            timestamp: Utc::now(),
        })
    }

    pub async fn save_tvl_stats(&self, stats: TvlStats) -> Result<()> {
        let collection: Collection<TvlStats> = self.db.collection("tvl_stats");
        collection.insert_one(stats, None).await?;
        Ok(())
    }
}
