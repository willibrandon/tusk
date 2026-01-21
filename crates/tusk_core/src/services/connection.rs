//! Database connection pooling with deadpool-postgres.
//!
//! Provides efficient connection management with:
//! - Connection validation on pool creation (FR-011)
//! - Pool status reporting (FR-013)
//! - Configurable timeout on pool exhaustion (FR-013a)

use crate::error::TuskError;
use crate::models::{ConnectionConfig, PoolStatus};

use chrono::{DateTime, Utc};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use std::sync::Arc;
use std::time::Duration;
use tokio_postgres::NoTls;
use uuid::Uuid;

/// A managed pool of database connections for a single ConnectionConfig.
///
/// Wraps deadpool-postgres to provide connection reuse, health checking,
/// and pool status monitoring.
pub struct ConnectionPool {
    /// Unique identifier matching ConnectionConfig.id
    id: Uuid,
    /// Original connection configuration
    config: Arc<ConnectionConfig>,
    /// The actual connection pool
    pool: Pool,
    /// When this pool was created
    created_at: DateTime<Utc>,
}

impl ConnectionPool {
    /// Create a new connection pool with the given configuration.
    ///
    /// This validates connectivity by establishing a test connection (FR-011).
    /// Pool creation completes within the configured connection timeout (SC-003).
    pub async fn new(config: ConnectionConfig, password: &str) -> Result<Self, TuskError> {
        Self::with_pool_config(config, password, 4, Duration::from_secs(30)).await
    }

    /// Create a connection pool with custom pool settings.
    ///
    /// # Arguments
    /// * `config` - Connection configuration
    /// * `password` - Database password (from credential service)
    /// * `max_size` - Maximum number of connections in the pool
    /// * `wait_timeout` - How long to wait when pool is exhausted (FR-013a)
    pub async fn with_pool_config(
        config: ConnectionConfig,
        password: &str,
        max_size: usize,
        wait_timeout: Duration,
    ) -> Result<Self, TuskError> {
        let connect_timeout = Duration::from_secs(config.options.connect_timeout_secs as u64);

        // Build tokio-postgres config
        let mut pg_config = tokio_postgres::Config::new();
        pg_config.host(&config.host);
        pg_config.port(config.port);
        pg_config.dbname(&config.database);
        pg_config.user(&config.username);
        pg_config.password(password);
        pg_config.application_name(&config.options.application_name);
        pg_config.connect_timeout(connect_timeout);
        pg_config.keepalives(true);
        pg_config.keepalives_idle(Duration::from_secs(60));

        // Create manager with recycling for connection health
        let manager = Manager::from_config(
            pg_config,
            NoTls,
            ManagerConfig {
                recycling_method: RecyclingMethod::Fast,
            },
        );

        // Build the pool
        let pool = Pool::builder(manager)
            .max_size(max_size)
            .wait_timeout(Some(wait_timeout))
            .create_timeout(Some(connect_timeout))
            .runtime(Runtime::Tokio1)
            .build()
            .map_err(|e| TuskError::connection(format!("Failed to create pool: {e}")))?;

        // Validate connection by establishing a test connection (FR-011)
        let client = pool.get().await.map_err(|e| {
            TuskError::connection(format!("Failed to establish connection: {e}"))
        })?;

        // Execute a simple query to verify the connection is working
        client
            .execute("SELECT 1", &[])
            .await
            .map_err(|e| TuskError::connection(format!("Connection validation failed: {e}")))?;

        tracing::info!(
            connection_id = %config.id,
            host = %config.host,
            database = %config.database,
            "Connection pool created successfully"
        );

        Ok(Self {
            id: config.id,
            config: Arc::new(config),
            pool,
            created_at: Utc::now(),
        })
    }

    /// Get the pool's unique identifier.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the connection configuration.
    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    /// Get when this pool was created.
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Acquire a connection from the pool.
    ///
    /// Waits up to the configured timeout if the pool is exhausted (FR-013a).
    pub async fn get(&self) -> Result<PooledConnection, TuskError> {
        let client = self.pool.get().await.map_err(|e| {
            let status = self.status();
            if status.waiting > 0 {
                TuskError::pool_timeout(
                    format!("Pool exhausted after timeout: {e}"),
                    status.waiting,
                )
            } else {
                TuskError::connection(format!("Failed to acquire connection: {e}"))
            }
        })?;

        Ok(PooledConnection {
            client,
            connection_id: self.id,
        })
    }

    /// Get current pool status (FR-013, SC-010).
    pub fn status(&self) -> PoolStatus {
        let status = self.pool.status();
        PoolStatus {
            max_size: status.max_size,
            size: status.size,
            available: status.available as isize,
            waiting: status.waiting,
        }
    }

    /// Close the pool, dropping all connections.
    pub fn close(&self) {
        self.pool.close();
        tracing::info!(connection_id = %self.id, "Connection pool closed");
    }

    /// Check if the pool is closed.
    pub fn is_closed(&self) -> bool {
        self.pool.is_closed()
    }
}

/// A connection acquired from the pool.
///
/// Automatically returns to the pool when dropped.
pub struct PooledConnection {
    client: deadpool_postgres::Client,
    connection_id: Uuid,
}

impl PooledConnection {
    /// Get the connection ID this pooled connection belongs to.
    pub fn connection_id(&self) -> Uuid {
        self.connection_id
    }

    /// Execute a query that returns rows.
    pub async fn query(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<Vec<tokio_postgres::Row>, TuskError> {
        self.client
            .query(sql, params)
            .await
            .map_err(TuskError::from)
    }

    /// Execute a query that doesn't return rows.
    pub async fn execute(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<u64, TuskError> {
        self.client
            .execute(sql, params)
            .await
            .map_err(TuskError::from)
    }

    /// Prepare a statement for repeated execution.
    pub async fn prepare(&self, sql: &str) -> Result<tokio_postgres::Statement, TuskError> {
        self.client.prepare(sql).await.map_err(TuskError::from)
    }

    /// Begin a transaction.
    pub async fn transaction(&mut self) -> Result<Transaction<'_>, TuskError> {
        let txn = self.client.transaction().await.map_err(TuskError::from)?;
        Ok(Transaction { txn })
    }
}

/// A database transaction.
///
/// Automatically rolls back on drop unless committed.
pub struct Transaction<'a> {
    txn: deadpool_postgres::Transaction<'a>,
}

impl<'a> Transaction<'a> {
    /// Execute a query within the transaction.
    pub async fn query(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<Vec<tokio_postgres::Row>, TuskError> {
        self.txn.query(sql, params).await.map_err(TuskError::from)
    }

    /// Execute a statement within the transaction.
    pub async fn execute(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<u64, TuskError> {
        self.txn
            .execute(sql, params)
            .await
            .map_err(TuskError::from)
    }

    /// Commit the transaction.
    pub async fn commit(self) -> Result<(), TuskError> {
        self.txn.commit().await.map_err(TuskError::from)
    }

    /// Rollback the transaction explicitly.
    pub async fn rollback(self) -> Result<(), TuskError> {
        self.txn.rollback().await.map_err(TuskError::from)
    }
}
