use crate::error::TuskResult;
use crate::services::connection::ConnectionPoolEntry;
use crate::services::AppDirs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

/// Application-level state container.
/// This struct holds all runtime state for the Tusk backend.
pub struct AppState {
    /// Active connection pools keyed by connection config ID.
    pub connections: Arc<RwLock<HashMap<Uuid, ConnectionPoolEntry>>>,

    /// Currently executing queries keyed by query ID.
    pub active_queries: Arc<RwLock<HashMap<Uuid, QueryHandle>>>,

    /// Local SQLite storage service.
    pub storage: Option<Arc<crate::services::StorageService>>,

    /// OS keychain credential service.
    pub credentials: Option<Arc<crate::services::CredentialService>>,

    /// Path to the log directory.
    pub log_dir: PathBuf,

    /// Path to the data directory.
    pub data_dir: PathBuf,
}

/// Handle for a running query, used for cancellation.
pub struct QueryHandle {
    /// Query ID
    pub query_id: Uuid,
    /// Connection this query is running on
    pub connection_id: Uuid,
    /// SQL text (truncated for logging)
    pub sql: String,
    /// When the query started
    pub started_at: std::time::Instant,
    /// Cancel token for stopping the query
    pub cancel_token: tokio_util::sync::CancellationToken,
}

impl AppState {
    /// Create a new AppState from application directories.
    ///
    /// # Arguments
    ///
    /// * `app_dirs` - Application directory paths
    ///
    /// # Returns
    ///
    /// Returns a new `AppState` with initialized fields.
    pub fn new(app_dirs: &AppDirs) -> TuskResult<Self> {
        // Initialize SQLite storage service
        let storage = crate::services::StorageService::new(&app_dirs.data_dir)?;
        tracing::info!("Storage service initialized");

        // Initialize credential service for OS keychain access
        let credentials = crate::services::CredentialService::new("tusk");
        tracing::info!("Credential service initialized");

        Ok(Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            active_queries: Arc::new(RwLock::new(HashMap::new())),
            storage: Some(Arc::new(storage)),
            credentials: Some(Arc::new(credentials)),
            log_dir: app_dirs.log_dir.clone(),
            data_dir: app_dirs.data_dir.clone(),
        })
    }
}
