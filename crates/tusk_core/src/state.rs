//! Application state management.
//!
//! Provides centralized state accessible from any component (FR-005 through FR-009).
//! Implements `gpui::Global` for use with GPUI's context system.

use crate::error::TuskError;
use crate::models::{ConnectionConfig, PoolStatus, QueryHandle};
use crate::services::{ConnectionPool, CredentialService, LocalStorage};

use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// Placeholder for schema cache (implemented in future feature).
#[derive(Debug, Clone, Default)]
pub struct SchemaCache {
    /// Connection this cache belongs to
    pub connection_id: Uuid,
    // Future: tables, views, functions, etc.
}

/// Central application state (FR-005).
///
/// Holds all runtime state including connections, caches, and services.
/// Thread-safe via `parking_lot::RwLock` (FR-009).
pub struct TuskState {
    /// Active connection pools (FR-006)
    connections: RwLock<HashMap<Uuid, Arc<ConnectionPool>>>,
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

    /// Add a connection pool to state.
    pub fn add_connection(&self, pool: ConnectionPool) {
        let id = pool.id();
        tracing::debug!(connection_id = %id, "Adding connection to state");
        self.connections.write().insert(id, Arc::new(pool));
    }

    /// Get a connection pool by ID.
    pub fn get_connection(&self, id: &Uuid) -> Option<Arc<ConnectionPool>> {
        self.connections.read().get(id).cloned()
    }

    /// Remove a connection pool from state.
    ///
    /// Also removes the associated schema cache (invariant from spec).
    pub fn remove_connection(&self, id: &Uuid) -> Option<Arc<ConnectionPool>> {
        // Remove schema cache for this connection
        self.schema_caches.write().remove(id);

        let pool = self.connections.write().remove(id);
        if pool.is_some() {
            tracing::debug!(connection_id = %id, "Removed connection from state");
        }
        pool
    }

    /// Get all connection IDs.
    pub fn connection_ids(&self) -> Vec<Uuid> {
        self.connections.read().keys().copied().collect()
    }

    /// Get status of all connection pools (SC-010).
    pub fn all_pool_statuses(&self) -> HashMap<Uuid, PoolStatus> {
        self.connections
            .read()
            .iter()
            .map(|(id, pool)| (*id, pool.status()))
            .collect()
    }

    // ========== Schema Cache Management (FR-007) ==========

    /// Get schema cache for a connection.
    pub fn get_schema_cache(&self, connection_id: &Uuid) -> Option<SchemaCache> {
        self.schema_caches.read().get(connection_id).cloned()
    }

    /// Set schema cache for a connection.
    pub fn set_schema_cache(&self, connection_id: Uuid, cache: SchemaCache) {
        self.schema_caches.write().insert(connection_id, cache);
    }

    /// Remove schema cache for a connection.
    pub fn remove_schema_cache(&self, connection_id: &Uuid) -> Option<SchemaCache> {
        self.schema_caches.write().remove(connection_id)
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
}

// Implement GPUI's Global trait for application-wide state access
// Note: This requires the gpui crate, which may not be available in tusk_core
// The actual implementation will be in the main tusk crate
// impl gpui::Global for TuskState {}
