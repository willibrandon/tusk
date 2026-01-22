//! Application state management.
//!
//! Provides centralized state accessible from any component (FR-005 through FR-009).
//! Implements `gpui::Global` for use with GPUI's context system.

use crate::error::TuskError;
use crate::models::{
    ConnectionConfig, ConnectionStatus, PoolStatus, QueryEvent, QueryHandle, SchemaCache,
};
use crate::services::{ConnectionPool, CredentialService, LocalStorage, QueryService};

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Wrapper for connection pool with status tracking (FR-006).
///
/// Stores a connection pool along with its current status and metadata.
/// This enables tracking connection lifecycle and providing status to UI.
#[derive(Debug)]
pub struct ConnectionEntry {
    /// Connection configuration (no password)
    config: ConnectionConfig,
    /// The actual connection pool
    pool: Arc<ConnectionPool>,
    /// Current connection status
    status: ConnectionStatus,
    /// When connection was established
    connected_at: DateTime<Utc>,
}

impl ConnectionEntry {
    /// Create a new connection entry with Connected status.
    pub fn new(config: ConnectionConfig, pool: Arc<ConnectionPool>) -> Self {
        Self {
            config,
            pool,
            status: ConnectionStatus::Connected,
            connected_at: Utc::now(),
        }
    }

    /// Get the connection configuration.
    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    /// Get the connection pool.
    pub fn pool(&self) -> &Arc<ConnectionPool> {
        &self.pool
    }

    /// Get the current connection status.
    pub fn status(&self) -> &ConnectionStatus {
        &self.status
    }

    /// Update the connection status.
    pub fn set_status(&mut self, status: ConnectionStatus) {
        self.status = status;
    }

    /// Get when the connection was established.
    pub fn connected_at(&self) -> DateTime<Utc> {
        self.connected_at
    }

    /// Get the connection ID.
    pub fn id(&self) -> Uuid {
        self.config.id
    }

    /// Get pool status.
    pub fn pool_status(&self) -> PoolStatus {
        self.pool.status()
    }
}

/// Central application state (FR-005).
///
/// Holds all runtime state including connections, caches, and services.
/// Thread-safe via `parking_lot::RwLock` (FR-009).
pub struct TuskState {
    /// Active connection entries with status tracking (FR-006)
    connections: RwLock<HashMap<Uuid, ConnectionEntry>>,
    /// Schema caches per connection (FR-007)
    schema_caches: RwLock<HashMap<Uuid, SchemaCache>>,
    /// Active queries with cancellation support (FR-008)
    active_queries: RwLock<HashMap<Uuid, Arc<QueryHandle>>>,
    /// Local SQLite storage
    storage: LocalStorage,
    /// Application data directory
    data_dir: PathBuf,
    /// Credential service for OS keychain
    credential_service: CredentialService,
    /// Tokio runtime for async database operations
    tokio_runtime: tokio::runtime::Runtime,
}

impl TuskState {
    /// Create new application state (SC-002: <100ms initialization).
    ///
    /// Uses the default data directory based on OS and build type.
    pub fn new() -> Result<Self, TuskError> {
        let data_dir = crate::services::storage::default_data_dir();
        Self::with_data_dir(data_dir)
    }

    /// Create application state with a custom data directory (for testing).
    pub fn with_data_dir(data_dir: PathBuf) -> Result<Self, TuskError> {
        // Initialize data directory
        crate::services::storage::init_data_dir(&data_dir)?;

        // Create tokio runtime for async operations (FR-020)
        let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|e| TuskError::internal(format!("Failed to create tokio runtime: {e}")))?;

        // Open local storage
        let storage = LocalStorage::open(data_dir.clone())?;

        // Initialize credential service
        let credential_service = CredentialService::new();

        tracing::info!(data_dir = %data_dir.display(), "TuskState initialized");

        Ok(Self {
            connections: RwLock::new(HashMap::new()),
            schema_caches: RwLock::new(HashMap::new()),
            active_queries: RwLock::new(HashMap::new()),
            storage,
            data_dir,
            credential_service,
            tokio_runtime,
        })
    }

    // ========== Connection Management (FR-006) ==========

    /// Add a connection entry to state.
    pub fn add_connection_entry(&self, entry: ConnectionEntry) {
        let id = entry.id();
        tracing::debug!(connection_id = %id, "Adding connection to state");
        self.connections.write().insert(id, entry);
    }

    /// Add a connection pool to state (convenience method).
    pub fn add_connection(&self, config: ConnectionConfig, pool: ConnectionPool) {
        let id = config.id;
        let entry = ConnectionEntry::new(config, Arc::new(pool));
        tracing::debug!(connection_id = %id, "Adding connection to state");
        self.connections.write().insert(id, entry);
    }

    /// Get a connection pool by ID.
    pub fn get_connection(&self, id: &Uuid) -> Option<Arc<ConnectionPool>> {
        self.connections.read().get(id).map(|entry| entry.pool().clone())
    }

    /// Get a connection entry by ID.
    pub fn get_connection_entry(&self, id: &Uuid) -> Option<(ConnectionConfig, Arc<ConnectionPool>, ConnectionStatus)> {
        self.connections.read().get(id).map(|entry| {
            (entry.config().clone(), entry.pool().clone(), entry.status().clone())
        })
    }

    /// Remove a connection from state.
    ///
    /// Also removes the associated schema cache (invariant from spec).
    pub fn remove_connection(&self, id: &Uuid) -> Option<Arc<ConnectionPool>> {
        // Remove schema cache for this connection
        self.schema_caches.write().remove(id);

        let entry = self.connections.write().remove(id);
        if let Some(ref e) = entry {
            tracing::debug!(connection_id = %id, "Removed connection from state");
            Some(e.pool().clone())
        } else {
            None
        }
    }

    /// Update connection status.
    pub fn set_connection_status(&self, id: &Uuid, status: ConnectionStatus) {
        if let Some(entry) = self.connections.write().get_mut(id) {
            entry.set_status(status);
            tracing::debug!(connection_id = %id, "Updated connection status");
        }
    }

    /// Get all connection IDs.
    pub fn connection_ids(&self) -> Vec<Uuid> {
        self.connections.read().keys().copied().collect()
    }

    /// Get status of all connection pools (SC-010).
    pub fn all_pool_statuses(&self) -> HashMap<Uuid, PoolStatus> {
        self.connections.read().iter().map(|(id, entry)| (*id, entry.pool_status())).collect()
    }

    /// Get all connection entries (ID, name, status).
    pub fn all_connections(&self) -> Vec<(Uuid, String, ConnectionStatus)> {
        self.connections
            .read()
            .values()
            .map(|entry| (entry.id(), entry.config().name.clone(), entry.status().clone()))
            .collect()
    }

    // ========== Schema Cache Management (FR-016, FR-017, FR-018) ==========

    /// Get schema cache for a connection if it exists and is valid.
    pub fn get_schema_cache(&self, connection_id: &Uuid) -> Option<SchemaCache> {
        let caches = self.schema_caches.read();
        caches.get(connection_id).filter(|cache| cache.is_valid()).cloned()
    }

    /// Get schema cache for a connection even if expired.
    pub fn get_schema_cache_any(&self, connection_id: &Uuid) -> Option<SchemaCache> {
        self.schema_caches.read().get(connection_id).cloned()
    }

    /// Set schema cache for a connection.
    pub fn set_schema_cache(&self, cache: SchemaCache) {
        let connection_id = cache.connection_id();
        self.schema_caches.write().insert(connection_id, cache);
        tracing::debug!(connection_id = %connection_id, "Schema cache updated");
    }

    /// Remove schema cache for a connection.
    pub fn remove_schema_cache(&self, connection_id: &Uuid) -> Option<SchemaCache> {
        let cache = self.schema_caches.write().remove(connection_id);
        if cache.is_some() {
            tracing::debug!(connection_id = %connection_id, "Schema cache removed");
        }
        cache
    }

    /// Check if schema cache exists and is valid for a connection.
    pub fn has_valid_schema_cache(&self, connection_id: &Uuid) -> bool {
        self.schema_caches
            .read()
            .get(connection_id)
            .map(|cache| cache.is_valid())
            .unwrap_or(false)
    }

    // ========== Query Tracking (FR-008) ==========

    /// Register a query for tracking.
    pub fn register_query(&self, handle: QueryHandle) -> Arc<QueryHandle> {
        let id = handle.id();
        let handle = Arc::new(handle);
        self.active_queries.write().insert(id, handle.clone());
        tracing::trace!(query_id = %id, "Query registered");
        handle
    }

    /// Get a query handle by ID.
    pub fn get_query(&self, id: &Uuid) -> Option<Arc<QueryHandle>> {
        self.active_queries.read().get(id).cloned()
    }

    /// Unregister a completed or cancelled query (FR-016).
    pub fn unregister_query(&self, id: &Uuid) -> Option<Arc<QueryHandle>> {
        let handle = self.active_queries.write().remove(id);
        if handle.is_some() {
            tracing::trace!(query_id = %id, "Query unregistered");
        }
        handle
    }

    /// Cancel a running query.
    ///
    /// Returns true if the query was found and cancellation was requested.
    pub fn cancel_query(&self, id: &Uuid) -> bool {
        if let Some(handle) = self.active_queries.read().get(id) {
            handle.cancel();
            true
        } else {
            false
        }
    }

    /// Get all active query IDs.
    pub fn active_query_ids(&self) -> Vec<Uuid> {
        self.active_queries.read().keys().copied().collect()
    }

    // ========== Service Accessors ==========

    /// Get the local storage service.
    pub fn storage(&self) -> &LocalStorage {
        &self.storage
    }

    /// Get the credential service.
    pub fn credentials(&self) -> &CredentialService {
        &self.credential_service
    }

    /// Get the data directory path.
    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    /// Get a handle to the tokio runtime.
    pub fn runtime(&self) -> &tokio::runtime::Runtime {
        &self.tokio_runtime
    }

    /// Spawn a future on the tokio runtime (FR-020, FR-021).
    ///
    /// Use this for database operations to avoid blocking the UI thread.
    pub fn spawn<F, T>(&self, future: F) -> tokio::task::JoinHandle<T>
    where
        F: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.tokio_runtime.spawn(future)
    }

    /// Block on a future using the tokio runtime.
    ///
    /// Note: Avoid using this from the main thread as it will block.
    /// Prefer `spawn` for non-blocking execution.
    pub fn block_on<F, T>(&self, future: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        self.tokio_runtime.block_on(future)
    }

    // ========== Convenience Methods ==========

    /// Load all saved connections from storage.
    pub fn load_saved_connections(&self) -> Result<Vec<ConnectionConfig>, TuskError> {
        self.storage.load_all_connections()
    }

    /// Save a connection configuration to storage.
    pub fn save_connection(&self, config: &ConnectionConfig) -> Result<(), TuskError> {
        self.storage.save_connection(config)
    }

    // ========== High-Level Connection API (FR-005, FR-006, FR-007, FR-008) ==========

    /// Establish a new database connection (FR-005, FR-006, FR-007).
    ///
    /// Creates a connection pool, validates connectivity, and adds to state.
    /// Optionally stores password in OS keychain for future use.
    ///
    /// # Arguments
    /// * `config` - Connection configuration
    /// * `password` - Database password (not stored in config)
    ///
    /// # Returns
    /// Connection ID on success, or TuskError on failure.
    pub async fn connect(
        &self,
        config: &ConnectionConfig,
        password: &str,
    ) -> Result<Uuid, TuskError> {
        let connection_id = config.id;

        tracing::debug!(
            connection_id = %connection_id,
            host = %config.host,
            database = %config.database,
            "Establishing connection"
        );

        // Create and validate connection pool
        let pool = ConnectionPool::new(config.clone(), password).await?;

        // Store password in credential service (FR-009, FR-028)
        // Note: password is intentionally NOT logged (FR-026)
        if let Err(e) = self.credential_service.store_password(connection_id, password) {
            tracing::warn!(
                connection_id = %connection_id,
                error = %e,
                "Failed to store password in keychain, using session storage"
            );
            // Continue anyway - password is already in session store as fallback
        }

        // Add connection entry with Connected status
        let entry = ConnectionEntry::new(config.clone(), Arc::new(pool));
        self.connections.write().insert(connection_id, entry);

        tracing::info!(
            connection_id = %connection_id,
            host = %config.host,
            database = %config.database,
            "Connection established"
        );

        Ok(connection_id)
    }

    /// Close a database connection (FR-008).
    ///
    /// Cancels all active queries on this connection, closes the pool,
    /// and removes from state. Also removes the associated schema cache.
    ///
    /// # Arguments
    /// * `connection_id` - ID of connection to close
    pub async fn disconnect(&self, connection_id: Uuid) -> Result<(), TuskError> {
        tracing::debug!(connection_id = %connection_id, "Disconnecting");

        // Cancel all active queries on this connection
        let query_ids: Vec<Uuid> = self
            .active_queries
            .read()
            .iter()
            .filter(|(_, handle)| handle.connection_id() == connection_id)
            .map(|(id, _)| *id)
            .collect();

        for query_id in query_ids {
            self.cancel_query(&query_id);
            self.unregister_query(&query_id);
        }

        // Remove schema cache
        self.schema_caches.write().remove(&connection_id);

        // Remove and close connection pool
        let entry = self.connections.write().remove(&connection_id);
        if let Some(entry) = entry {
            entry.pool().close();
            tracing::info!(connection_id = %connection_id, "Connection closed");
            Ok(())
        } else {
            Err(TuskError::internal(format!("Connection not found: {connection_id}")))
        }
    }

    /// Get the current status of a connection (FR-006).
    ///
    /// Returns ConnectionStatus for a connection ID.
    /// Returns Disconnected if connection not found.
    pub fn get_connection_status(&self, connection_id: Uuid) -> ConnectionStatus {
        self.connections
            .read()
            .get(&connection_id)
            .map(|entry| entry.status().clone())
            .unwrap_or(ConnectionStatus::Disconnected)
    }

    /// Test connection without establishing a persistent session.
    ///
    /// Validates connectivity and authentication without adding to state.
    /// Useful for the "Test Connection" button in connection dialog.
    ///
    /// # Arguments
    /// * `config` - Connection configuration to test
    /// * `password` - Password for authentication
    pub async fn test_connection(
        &self,
        config: &ConnectionConfig,
        password: &str,
    ) -> Result<(), TuskError> {
        tracing::debug!(
            host = %config.host,
            database = %config.database,
            "Testing connection"
        );

        // Create a temporary pool to validate connectivity
        // The pool will be dropped after this function returns
        let pool = ConnectionPool::new(config.clone(), password).await?;

        // Get a connection to fully validate
        let _conn = pool.get().await?;

        tracing::debug!(
            host = %config.host,
            database = %config.database,
            "Connection test successful"
        );

        // Pool is dropped here, closing connections
        Ok(())
    }

    // ========== Query Execution API (FR-010, FR-011, FR-012, FR-013) ==========

    /// Execute a query and return a handle for tracking (FR-010, FR-013).
    ///
    /// Creates a query handle, registers it for tracking, and executes.
    /// Use cancel_query() with the returned handle's ID to cancel.
    ///
    /// # Arguments
    /// * `connection_id` - Connection to execute on
    /// * `sql` - SQL query to execute
    ///
    /// # Returns
    /// Query handle for tracking and cancellation.
    pub async fn execute_query(
        &self,
        connection_id: Uuid,
        sql: &str,
    ) -> Result<Arc<QueryHandle>, TuskError> {
        // Validate connection exists (pool returned but not used here - execution is separate)
        let _pool = self.get_connection(&connection_id).ok_or_else(|| {
            TuskError::internal(format!("No active connection: {connection_id}"))
        })?;

        // Create and register query handle
        let handle = QueryHandle::new(connection_id, sql);
        let handle = self.register_query(handle);

        Ok(handle)
    }

    /// Execute a query with streaming results via channel (FR-010, FR-011, FR-012).
    ///
    /// Sends QueryEvent messages through the provided channel as results arrive.
    /// Returns immediately after starting; results stream asynchronously.
    ///
    /// # Arguments
    /// * `connection_id` - Connection to execute on
    /// * `sql` - SQL query to execute
    /// * `tx` - Channel sender for QueryEvent stream
    ///
    /// # Returns
    /// Query handle for tracking and cancellation.
    pub async fn execute_query_streaming(
        &self,
        connection_id: Uuid,
        sql: &str,
        tx: mpsc::Sender<QueryEvent>,
    ) -> Result<Arc<QueryHandle>, TuskError> {
        // Get connection pool
        let pool = self.get_connection(&connection_id).ok_or_else(|| {
            TuskError::internal(format!("No active connection: {connection_id}"))
        })?;

        // Create and register query handle
        let handle = QueryHandle::new(connection_id, sql.to_string());
        let handle = self.register_query(handle);

        // Get a connection from the pool
        let conn = pool.get().await?;

        // Execute streaming query
        let sql_owned = sql.to_string();
        let handle_ref = handle.clone();

        // Spawn the streaming execution
        let query_id = handle.id();
        self.spawn(async move {
            let result =
                QueryService::execute_streaming(&conn, &sql_owned, &handle_ref, tx).await;

            if let Err(e) = result {
                tracing::warn!(
                    query_id = %query_id,
                    error = %e,
                    "Streaming query failed"
                );
            }
        });

        Ok(handle)
    }
}

// Implement GPUI's Global trait for application-wide state access (FR-005)
#[cfg(feature = "gpui")]
impl gpui::Global for TuskState {}
