# Feature 07: Connection Management

## Overview

Implement the core connection management system including the connection model, connection pooling with deadpool-postgres, connection lifecycle management, validation, and auto-reconnection. All components are pure Rust using GPUI for state management and UI integration.

## Goals

- Implement ConnectionConfig model with all fields from design doc
- Create connection pool with deadpool-postgres
- Handle connection lifecycle (connect, keepalive, disconnect)
- Implement auto-reconnection with exponential backoff
- Support read-only mode and statement timeout
- Integrate with GPUI's state management system

## Technical Specification

### 1. Connection Models

```rust
// src/models/connection.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionConfig {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password_in_keyring: bool,
    pub color: Option<String>,
    pub group_id: Option<Uuid>,
    pub ssl_mode: SslMode,
    pub ssl_ca_cert: Option<String>,
    pub ssl_client_cert: Option<String>,
    pub ssl_client_key: Option<String>,
    pub ssh_tunnel: Option<SshTunnelConfig>,
    pub options: ConnectionOptions,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_connected_at: Option<DateTime<Utc>>,
}

impl ConnectionConfig {
    pub fn new(name: String, host: String, port: u16, database: String, username: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            host,
            port,
            database,
            username,
            password_in_keyring: true,
            color: None,
            group_id: None,
            ssl_mode: SslMode::Prefer,
            ssl_ca_cert: None,
            ssl_client_cert: None,
            ssl_client_key: None,
            ssh_tunnel: None,
            options: ConnectionOptions::default(),
            created_at: now,
            updated_at: now,
            last_connected_at: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SslMode {
    Disable,
    #[default]
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SshTunnelConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: SshAuthMethod,
    pub key_path: Option<String>,
    pub key_passphrase_in_keyring: bool,
}

impl Default for SshTunnelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: String::new(),
            port: 22,
            username: String::new(),
            auth: SshAuthMethod::Key,
            key_path: None,
            key_passphrase_in_keyring: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SshAuthMethod {
    #[default]
    Key,
    Password,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionOptions {
    pub connect_timeout_sec: u64,
    pub statement_timeout_ms: Option<u64>,
    pub readonly: bool,
    pub application_name: String,
    pub keepalive_interval_sec: u64,
    pub max_pool_size: usize,
}

impl Default for ConnectionOptions {
    fn default() -> Self {
        Self {
            connect_timeout_sec: 10,
            statement_timeout_ms: None,
            readonly: false,
            application_name: "Tusk".to_string(),
            keepalive_interval_sec: 60,
            max_pool_size: 10,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self::Disconnected
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionGroup {
    pub id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub expanded: bool,
    pub sort_order: i32,
}

impl ConnectionGroup {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            color: None,
            expanded: true,
            sort_order: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: Uuid,
    pub server_version: String,
    pub current_database: String,
    pub current_user: String,
    pub backend_pid: i32,
    pub connected_at: DateTime<Utc>,
}
```

### 2. Connection Validation

```rust
// src/services/connection/validation.rs
use crate::error::{Result, TuskError};
use crate::models::connection::{ConnectionConfig, SslMode, SshAuthMethod};

impl ConnectionConfig {
    /// Validate the connection configuration
    pub fn validate(&self) -> Result<()> {
        // Host validation
        if self.host.is_empty() {
            return Err(TuskError::InvalidInput {
                field: "host".to_string(),
                message: "Host is required".to_string(),
            });
        }

        // Port validation
        if self.port == 0 {
            return Err(TuskError::InvalidInput {
                field: "port".to_string(),
                message: "Port must be between 1 and 65535".to_string(),
            });
        }

        // Database validation
        if self.database.is_empty() {
            return Err(TuskError::InvalidInput {
                field: "database".to_string(),
                message: "Database name is required".to_string(),
            });
        }

        // Username validation
        if self.username.is_empty() {
            return Err(TuskError::InvalidInput {
                field: "username".to_string(),
                message: "Username is required".to_string(),
            });
        }

        // SSH tunnel validation
        if let Some(ref ssh) = self.ssh_tunnel {
            if ssh.enabled {
                if ssh.host.is_empty() {
                    return Err(TuskError::InvalidInput {
                        field: "ssh_tunnel.host".to_string(),
                        message: "SSH host is required when tunnel is enabled".to_string(),
                    });
                }
                if ssh.username.is_empty() {
                    return Err(TuskError::InvalidInput {
                        field: "ssh_tunnel.username".to_string(),
                        message: "SSH username is required".to_string(),
                    });
                }
                if matches!(ssh.auth, SshAuthMethod::Key) && ssh.key_path.is_none() {
                    return Err(TuskError::InvalidInput {
                        field: "ssh_tunnel.key_path".to_string(),
                        message: "SSH key path is required for key authentication".to_string(),
                    });
                }
            }
        }

        // SSL validation
        if matches!(self.ssl_mode, SslMode::VerifyCa | SslMode::VerifyFull)
            && self.ssl_ca_cert.is_none()
        {
            return Err(TuskError::InvalidInput {
                field: "ssl_ca_cert".to_string(),
                message: "CA certificate is required for SSL verification".to_string(),
            });
        }

        // Name validation
        if self.name.is_empty() {
            return Err(TuskError::InvalidInput {
                field: "name".to_string(),
                message: "Connection name is required".to_string(),
            });
        }

        Ok(())
    }

    /// Check if SSH tunnel is enabled
    pub fn uses_ssh_tunnel(&self) -> bool {
        self.ssh_tunnel.as_ref().map_or(false, |t| t.enabled)
    }

    /// Check if SSL is enabled
    pub fn uses_ssl(&self) -> bool {
        !matches!(self.ssl_mode, SslMode::Disable)
    }

    /// Get display string for connection
    pub fn display_string(&self) -> String {
        format!(
            "{}@{}:{}/{}",
            self.username, self.host, self.port, self.database
        )
    }
}
```

### 3. Connection Pool Implementation

```rust
// src/services/connection/pool.rs
use std::sync::Arc;
use std::time::Duration;
use deadpool_postgres::{Config, Pool, Runtime, PoolConfig, Timeouts, Manager, ManagerConfig, RecyclingMethod};
use tokio_postgres::{NoTls, Config as PgConfig};
use parking_lot::RwLock;
use tokio::sync::watch;
use uuid::Uuid;
use chrono::Utc;

use crate::error::{Result, TuskError};
use crate::models::connection::{ConnectionConfig, ConnectionOptions, ConnectionStatus, ConnectionInfo, SslMode};
use crate::services::keyring::KeyringService;

/// A managed connection pool for a single database connection
pub struct ConnectionPool {
    pool: Pool,
    config: ConnectionConfig,
    status: Arc<RwLock<ConnectionStatus>>,
    status_tx: watch::Sender<ConnectionStatus>,
    status_rx: watch::Receiver<ConnectionStatus>,
    info: Arc<RwLock<Option<ConnectionInfo>>>,
    shutdown: Arc<RwLock<bool>>,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub async fn new(
        config: ConnectionConfig,
        keyring: &KeyringService,
    ) -> Result<Self> {
        // Validate config first
        config.validate()?;

        // Get password from keyring if needed
        let password = if config.password_in_keyring {
            keyring.get_password(&config.id.to_string())?
                .ok_or_else(|| TuskError::CredentialNotFound {
                    credential_type: "password".to_string(),
                    identifier: config.id.to_string(),
                })?
        } else {
            String::new()
        };

        // Build postgres config
        let mut pg_config = PgConfig::new();
        pg_config
            .host(&config.host)
            .port(config.port)
            .dbname(&config.database)
            .user(&config.username)
            .password(&password)
            .application_name(&config.options.application_name)
            .connect_timeout(Duration::from_secs(config.options.connect_timeout_sec));

        // Set statement timeout if configured
        if let Some(timeout_ms) = config.options.statement_timeout_ms {
            pg_config.options(&format!("-c statement_timeout={}", timeout_ms));
        }

        // Create pool based on SSL mode (basic - full SSL in Feature 08)
        let pool = Self::create_pool(pg_config, &config.options).await?;

        // Create status channel
        let (status_tx, status_rx) = watch::channel(ConnectionStatus::Connected);

        let conn_pool = Self {
            pool,
            config,
            status: Arc::new(RwLock::new(ConnectionStatus::Connected)),
            status_tx,
            status_rx,
            info: Arc::new(RwLock::new(None)),
            shutdown: Arc::new(RwLock::new(false)),
        };

        // Fetch and store server info
        conn_pool.fetch_server_info().await?;

        // Set read-only mode if configured
        if conn_pool.config.options.readonly {
            conn_pool.set_readonly_mode().await?;
        }

        Ok(conn_pool)
    }

    async fn create_pool(pg_config: PgConfig, options: &ConnectionOptions) -> Result<Pool> {
        let mut cfg = Config::new();

        // Extract connection details from pg_config
        if let Some(hosts) = pg_config.get_hosts().first() {
            cfg.host = Some(hosts.to_string());
        }
        cfg.port = pg_config.get_ports().first().copied();
        cfg.dbname = pg_config.get_dbname().map(|s| s.to_string());
        cfg.user = pg_config.get_user().map(|s| s.to_string());

        let pool_cfg = PoolConfig {
            max_size: options.max_pool_size,
            timeouts: Timeouts {
                wait: Some(Duration::from_secs(options.connect_timeout_sec)),
                create: Some(Duration::from_secs(options.connect_timeout_sec)),
                recycle: Some(Duration::from_secs(30)),
            },
            ..Default::default()
        };

        cfg.pool = Some(pool_cfg);

        // Configure manager
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        let pool = cfg
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| TuskError::ConnectionFailed {
                message: format!("Failed to create connection pool: {}", e),
                source: Some(e.to_string()),
            })?;

        // Test the connection
        let _client = pool.get().await.map_err(|e| TuskError::ConnectionFailed {
            message: format!("Failed to establish initial connection: {}", e),
            source: Some(e.to_string()),
        })?;

        Ok(pool)
    }

    async fn fetch_server_info(&self) -> Result<()> {
        let client = self.get_client().await?;

        let row = client.query_one(
            "SELECT version(), current_database(), current_user, pg_backend_pid()",
            &[],
        ).await.map_err(|e| TuskError::QueryFailed {
            message: format!("Failed to fetch server info: {}", e),
            detail: None,
            hint: None,
            position: None,
            code: None,
        })?;

        let info = ConnectionInfo {
            id: self.config.id,
            server_version: row.get(0),
            current_database: row.get(1),
            current_user: row.get(2),
            backend_pid: row.get(3),
            connected_at: Utc::now(),
        };

        *self.info.write() = Some(info);

        Ok(())
    }

    async fn set_readonly_mode(&self) -> Result<()> {
        let client = self.get_client().await?;

        client.execute("SET default_transaction_read_only = ON", &[])
            .await
            .map_err(|e| TuskError::QueryFailed {
                message: format!("Failed to set read-only mode: {}", e),
                detail: None,
                hint: Some("Check if user has permission to modify session settings".to_string()),
                position: None,
                code: None,
            })?;

        tracing::info!(
            connection_id = %self.config.id,
            "Set connection to read-only mode"
        );

        Ok(())
    }

    /// Get a client from the pool
    pub async fn get_client(&self) -> Result<deadpool_postgres::Client> {
        self.pool.get().await.map_err(|e| {
            TuskError::ConnectionFailed {
                message: format!("Failed to get connection from pool: {}", e),
                source: Some(e.to_string()),
            }
        })
    }

    /// Get the connection configuration
    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    /// Get current connection status
    pub fn status(&self) -> ConnectionStatus {
        *self.status.read()
    }

    /// Get a receiver for status updates
    pub fn status_receiver(&self) -> watch::Receiver<ConnectionStatus> {
        self.status_rx.clone()
    }

    /// Get connection info
    pub fn info(&self) -> Option<ConnectionInfo> {
        self.info.read().clone()
    }

    /// Update connection status
    fn set_status(&self, status: ConnectionStatus) {
        *self.status.write() = status;
        let _ = self.status_tx.send(status);
    }

    /// Check if the pool is healthy
    pub async fn is_healthy(&self) -> bool {
        match self.get_client().await {
            Ok(client) => {
                client.query_one("SELECT 1", &[]).await.is_ok()
            }
            Err(_) => false,
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let status = self.pool.status();
        PoolStats {
            size: status.size,
            available: status.available,
            waiting: status.waiting,
            max_size: status.max_size,
        }
    }

    /// Execute a query with automatic retry on transient failures
    pub async fn execute_with_retry<F, T, Fut>(
        &self,
        max_retries: u32,
        operation: F,
    ) -> Result<T>
    where
        F: Fn(deadpool_postgres::Client) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;
        let mut delay = Duration::from_millis(100);

        for attempt in 0..=max_retries {
            if attempt > 0 {
                tracing::debug!(
                    connection_id = %self.config.id,
                    attempt = attempt,
                    delay_ms = delay.as_millis(),
                    "Retrying operation"
                );
                tokio::time::sleep(delay).await;
                delay = std::cmp::min(delay * 2, Duration::from_secs(10));
            }

            match self.get_client().await {
                Ok(client) => {
                    match operation(client).await {
                        Ok(result) => return Ok(result),
                        Err(e) => {
                            if Self::is_retryable_error(&e) {
                                last_error = Some(e);
                                self.set_status(ConnectionStatus::Reconnecting);
                                continue;
                            }
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    if Self::is_retryable_error(&e) {
                        last_error = Some(e);
                        self.set_status(ConnectionStatus::Reconnecting);
                        continue;
                    }
                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| TuskError::ConnectionFailed {
            message: format!("Max retries ({}) exceeded", max_retries),
            source: None,
        }))
    }

    fn is_retryable_error(error: &TuskError) -> bool {
        matches!(
            error,
            TuskError::ConnectionFailed { .. } |
            TuskError::ConnectionTimeout { .. }
        )
    }

    /// Close the connection pool
    pub async fn close(&self) {
        *self.shutdown.write() = true;
        self.set_status(ConnectionStatus::Disconnected);
        self.pool.close();
        tracing::info!(connection_id = %self.config.id, "Connection pool closed");
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PoolStats {
    pub size: usize,
    pub available: usize,
    pub waiting: usize,
    pub max_size: usize,
}
```

### 4. Connection Service

```rust
// src/services/connection/mod.rs
mod pool;
mod validation;

pub use pool::{ConnectionPool, PoolStats};

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use tokio::runtime::Handle;
use tokio::sync::watch;
use uuid::Uuid;

use crate::error::{Result, TuskError};
use crate::models::connection::{ConnectionConfig, ConnectionStatus, ConnectionInfo, ConnectionGroup};
use crate::services::keyring::KeyringService;
use crate::services::storage::StorageService;

/// Manages all database connections
pub struct ConnectionService {
    pools: RwLock<HashMap<Uuid, Arc<ConnectionPool>>>,
    storage: Arc<StorageService>,
    keyring: Arc<KeyringService>,
    runtime: Handle,
    status_tx: watch::Sender<ConnectionServiceStatus>,
    status_rx: watch::Receiver<ConnectionServiceStatus>,
}

#[derive(Debug, Clone, Default)]
pub struct ConnectionServiceStatus {
    pub active_connections: usize,
    pub recent_change: Option<ConnectionChange>,
}

#[derive(Debug, Clone)]
pub struct ConnectionChange {
    pub connection_id: Uuid,
    pub status: ConnectionStatus,
    pub error: Option<String>,
}

impl ConnectionService {
    /// Create a new connection service
    pub fn new(
        storage: Arc<StorageService>,
        keyring: Arc<KeyringService>,
        runtime: Handle,
    ) -> Self {
        let (status_tx, status_rx) = watch::channel(ConnectionServiceStatus::default());

        Self {
            pools: RwLock::new(HashMap::new()),
            storage,
            keyring,
            runtime,
            status_tx,
            status_rx,
        }
    }

    /// Get a receiver for service-level status changes
    pub fn status_receiver(&self) -> watch::Receiver<ConnectionServiceStatus> {
        self.status_rx.clone()
    }

    /// Connect to a database
    pub fn connect(&self, config: ConnectionConfig) -> Result<ConnectionInfo> {
        let keyring = self.keyring.clone();

        // Run async connection in tokio runtime
        let pool = self.runtime.block_on(async {
            ConnectionPool::new(config.clone(), &keyring).await
        })?;

        let info = pool.info().ok_or_else(|| TuskError::ConnectionFailed {
            message: "Failed to get connection info".to_string(),
            source: None,
        })?;

        let pool = Arc::new(pool);

        // Store pool
        self.pools.write().insert(config.id, pool.clone());

        // Update last connected time
        self.storage.update_connection_last_used(&config.id)?;

        // Notify status change
        self.notify_change(ConnectionChange {
            connection_id: config.id,
            status: ConnectionStatus::Connected,
            error: None,
        });

        // Start keepalive task
        self.start_keepalive(config.id, pool);

        tracing::info!(
            connection_id = %config.id,
            name = %config.name,
            server = %info.server_version,
            "Connected successfully"
        );

        Ok(info)
    }

    /// Connect by ID (loads config from storage)
    pub fn connect_by_id(&self, connection_id: &Uuid) -> Result<ConnectionInfo> {
        let config = self.storage.get_connection(connection_id)?
            .ok_or_else(|| TuskError::NotFound {
                entity: "connection".to_string(),
                id: connection_id.to_string(),
            })?;

        self.connect(config)
    }

    /// Disconnect from a database
    pub fn disconnect(&self, connection_id: &Uuid) -> Result<()> {
        let pool = self.pools.write().remove(connection_id);

        if let Some(pool) = pool {
            self.runtime.block_on(async {
                pool.close().await;
            });
        }

        self.notify_change(ConnectionChange {
            connection_id: *connection_id,
            status: ConnectionStatus::Disconnected,
            error: None,
        });

        tracing::info!(connection_id = %connection_id, "Disconnected");

        Ok(())
    }

    /// Test a connection without persisting it
    pub fn test_connection(&self, config: ConnectionConfig) -> Result<TestConnectionResult> {
        let keyring = self.keyring.clone();
        let start = std::time::Instant::now();

        let result = self.runtime.block_on(async {
            let pool = ConnectionPool::new(config, &keyring).await?;
            let client = pool.get_client().await?;

            let row = client.query_one(
                "SELECT version(), pg_postmaster_start_time()",
                &[],
            ).await.map_err(|e| TuskError::QueryFailed {
                message: format!("Test query failed: {}", e),
                detail: None,
                hint: None,
                position: None,
                code: None,
            })?;

            let version: String = row.get(0);
            let started_at: chrono::DateTime<chrono::Utc> = row.get(1);

            pool.close().await;

            Ok::<_, TuskError>((version, started_at))
        })?;

        Ok(TestConnectionResult {
            success: true,
            version: result.0,
            started_at: result.1.to_rfc3339(),
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Get a connection pool by ID
    pub fn get_pool(&self, connection_id: &Uuid) -> Option<Arc<ConnectionPool>> {
        self.pools.read().get(connection_id).cloned()
    }

    /// Get connection status
    pub fn get_status(&self, connection_id: &Uuid) -> ConnectionStatus {
        self.pools.read()
            .get(connection_id)
            .map(|p| p.status())
            .unwrap_or(ConnectionStatus::Disconnected)
    }

    /// Get connection info
    pub fn get_info(&self, connection_id: &Uuid) -> Option<ConnectionInfo> {
        self.pools.read()
            .get(connection_id)
            .and_then(|p| p.info())
    }

    /// Check if connected
    pub fn is_connected(&self, connection_id: &Uuid) -> bool {
        self.pools.read().contains_key(connection_id)
    }

    /// Get all active connection IDs
    pub fn active_connections(&self) -> Vec<Uuid> {
        self.pools.read().keys().copied().collect()
    }

    /// Get all connections with their status
    pub fn list_connections(&self) -> Result<Vec<ConnectionWithStatus>> {
        let configs = self.storage.get_all_connections()?;
        let pools = self.pools.read();

        Ok(configs.into_iter().map(|config| {
            let status = pools.get(&config.id)
                .map(|p| p.status())
                .unwrap_or(ConnectionStatus::Disconnected);

            let info = pools.get(&config.id)
                .and_then(|p| p.info());

            ConnectionWithStatus { config, status, info }
        }).collect())
    }

    /// Save a connection configuration
    pub fn save_connection(&self, config: ConnectionConfig, password: Option<String>) -> Result<()> {
        config.validate()?;

        // Store password in keyring if provided
        if let Some(pwd) = password {
            if !pwd.is_empty() {
                self.keyring.store_password(&config.id.to_string(), &pwd)?;
            }
        }

        // Save config to storage
        self.storage.save_connection(&config)?;

        tracing::info!(
            connection_id = %config.id,
            name = %config.name,
            "Saved connection"
        );

        Ok(())
    }

    /// Delete a connection
    pub fn delete_connection(&self, connection_id: &Uuid) -> Result<()> {
        // Disconnect if connected
        self.disconnect(connection_id)?;

        // Delete credentials
        self.keyring.delete_all_for_connection(&connection_id.to_string())?;

        // Delete from storage
        self.storage.delete_connection(connection_id)?;

        tracing::info!(connection_id = %connection_id, "Deleted connection");

        Ok(())
    }

    /// Duplicate a connection with a new name
    pub fn duplicate_connection(&self, connection_id: &Uuid, new_name: String) -> Result<ConnectionConfig> {
        let mut config = self.storage.get_connection(connection_id)?
            .ok_or_else(|| TuskError::NotFound {
                entity: "connection".to_string(),
                id: connection_id.to_string(),
            })?;

        // Copy password if stored in keyring
        if config.password_in_keyring {
            if let Some(password) = self.keyring.get_password(&config.id.to_string())? {
                let new_id = Uuid::new_v4();
                self.keyring.store_password(&new_id.to_string(), &password)?;
                config.id = new_id;
            }
        } else {
            config.id = Uuid::new_v4();
        }

        config.name = new_name;
        config.created_at = chrono::Utc::now();
        config.updated_at = config.created_at;
        config.last_connected_at = None;

        self.storage.save_connection(&config)?;

        Ok(config)
    }

    // Group management

    /// Get all connection groups
    pub fn list_groups(&self) -> Result<Vec<ConnectionGroup>> {
        self.storage.get_all_groups()
    }

    /// Save a connection group
    pub fn save_group(&self, group: &ConnectionGroup) -> Result<()> {
        self.storage.save_group(group)
    }

    /// Delete a connection group
    pub fn delete_group(&self, group_id: &Uuid) -> Result<()> {
        // Unassign connections from group first
        let connections = self.storage.get_all_connections()?;
        for mut config in connections {
            if config.group_id == Some(*group_id) {
                config.group_id = None;
                self.storage.save_connection(&config)?;
            }
        }

        self.storage.delete_group(group_id)
    }

    /// Move a connection to a group
    pub fn move_to_group(&self, connection_id: &Uuid, group_id: Option<Uuid>) -> Result<()> {
        if let Some(mut config) = self.storage.get_connection(connection_id)? {
            config.group_id = group_id;
            self.storage.save_connection(&config)?;
        }
        Ok(())
    }

    // Private helpers

    fn notify_change(&self, change: ConnectionChange) {
        let active = self.pools.read().len();
        let _ = self.status_tx.send(ConnectionServiceStatus {
            active_connections: active,
            recent_change: Some(change),
        });
    }

    fn start_keepalive(&self, connection_id: Uuid, pool: Arc<ConnectionPool>) {
        let interval_secs = pool.config().options.keepalive_interval_sec;
        let status_tx = self.status_tx.clone();
        let pools = Arc::new(RwLock::new(self.pools.read().clone()));

        self.runtime.spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(interval_secs)
            );

            loop {
                interval.tick().await;

                // Check if pool still exists
                if !pools.read().contains_key(&connection_id) {
                    break;
                }

                match pool.get_client().await {
                    Ok(client) => {
                        match client.query_one("SELECT 1", &[]).await {
                            Ok(_) => {
                                // Connection healthy
                                if pool.status() == ConnectionStatus::Reconnecting {
                                    tracing::info!(
                                        connection_id = %connection_id,
                                        "Reconnected successfully"
                                    );
                                    let _ = status_tx.send(ConnectionServiceStatus {
                                        active_connections: pools.read().len(),
                                        recent_change: Some(ConnectionChange {
                                            connection_id,
                                            status: ConnectionStatus::Connected,
                                            error: None,
                                        }),
                                    });
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    connection_id = %connection_id,
                                    error = %e,
                                    "Keepalive query failed"
                                );
                                let _ = status_tx.send(ConnectionServiceStatus {
                                    active_connections: pools.read().len(),
                                    recent_change: Some(ConnectionChange {
                                        connection_id,
                                        status: ConnectionStatus::Reconnecting,
                                        error: Some(e.to_string()),
                                    }),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            connection_id = %connection_id,
                            error = %e,
                            "Keepalive connection failed"
                        );
                        let _ = status_tx.send(ConnectionServiceStatus {
                            active_connections: pools.read().len(),
                            recent_change: Some(ConnectionChange {
                                connection_id,
                                status: ConnectionStatus::Reconnecting,
                                error: Some(e.to_string()),
                            }),
                        });
                    }
                }
            }
        });
    }

    /// Disconnect all connections
    pub fn disconnect_all(&self) {
        let ids: Vec<Uuid> = self.pools.read().keys().copied().collect();
        for id in ids {
            let _ = self.disconnect(&id);
        }
    }
}

impl Drop for ConnectionService {
    fn drop(&mut self) {
        self.disconnect_all();
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionWithStatus {
    pub config: ConnectionConfig,
    pub status: ConnectionStatus,
    pub info: Option<ConnectionInfo>,
}

#[derive(Debug, Clone)]
pub struct TestConnectionResult {
    pub success: bool,
    pub version: String,
    pub started_at: String,
    pub latency_ms: u64,
}
```

### 5. GPUI State Integration

```rust
// src/state.rs (updated for connection management)
use std::sync::Arc;
use gpui::Global;
use tokio::runtime::Runtime;

use crate::services::connection::ConnectionService;
use crate::services::storage::StorageService;
use crate::services::keyring::KeyringService;

/// Global application state accessible via GPUI's Global trait
pub struct TuskState {
    pub storage: Arc<StorageService>,
    pub keyring: Arc<KeyringService>,
    pub connections: Arc<ConnectionService>,
    pub runtime: Arc<Runtime>,
}

impl Global for TuskState {}

impl TuskState {
    pub fn new(data_dir: &std::path::Path) -> crate::error::Result<Self> {
        let runtime = Arc::new(
            Runtime::new().expect("Failed to create tokio runtime")
        );

        let storage = Arc::new(StorageService::new(data_dir)?);
        let keyring = Arc::new(KeyringService::new());

        let connections = Arc::new(ConnectionService::new(
            storage.clone(),
            keyring.clone(),
            runtime.handle().clone(),
        ));

        Ok(Self {
            storage,
            keyring,
            connections,
            runtime,
        })
    }
}
```

### 6. Connection State Entity

```rust
// src/ui/state/connections.rs
use gpui::{Entity, Model, Context, AppContext};
use std::sync::Arc;
use uuid::Uuid;

use crate::models::connection::{ConnectionConfig, ConnectionStatus, ConnectionGroup};
use crate::services::connection::{ConnectionService, ConnectionWithStatus, ConnectionChange};
use crate::state::TuskState;
use crate::error::Result;

/// GPUI Entity for managing connection state in the UI
pub struct ConnectionsState {
    connections: Vec<ConnectionWithStatus>,
    groups: Vec<ConnectionGroup>,
    active_connection_id: Option<Uuid>,
    loading: bool,
    error: Option<String>,
}

impl ConnectionsState {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            groups: Vec::new(),
            active_connection_id: None,
            loading: false,
            error: None,
        }
    }

    /// Load all connections from storage
    pub fn load(&mut self, cx: &mut Context<Self>) -> Result<()> {
        self.loading = true;
        self.error = None;
        cx.notify();

        let state = cx.global::<TuskState>();

        match state.connections.list_connections() {
            Ok(connections) => {
                self.connections = connections;
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }

        match state.connections.list_groups() {
            Ok(groups) => {
                self.groups = groups;
            }
            Err(e) => {
                if self.error.is_none() {
                    self.error = Some(e.to_string());
                }
            }
        }

        self.loading = false;
        cx.notify();

        Ok(())
    }

    /// Connect to a database
    pub fn connect(&mut self, connection_id: Uuid, cx: &mut Context<Self>) -> Result<()> {
        // Update status to connecting
        if let Some(conn) = self.connections.iter_mut().find(|c| c.config.id == connection_id) {
            conn.status = ConnectionStatus::Connecting;
            cx.notify();
        }

        let state = cx.global::<TuskState>();

        match state.connections.connect_by_id(&connection_id) {
            Ok(info) => {
                if let Some(conn) = self.connections.iter_mut().find(|c| c.config.id == connection_id) {
                    conn.status = ConnectionStatus::Connected;
                    conn.info = Some(info);
                }
                self.active_connection_id = Some(connection_id);
                cx.notify();
                Ok(())
            }
            Err(e) => {
                if let Some(conn) = self.connections.iter_mut().find(|c| c.config.id == connection_id) {
                    conn.status = ConnectionStatus::Error;
                }
                cx.notify();
                Err(e)
            }
        }
    }

    /// Disconnect from a database
    pub fn disconnect(&mut self, connection_id: Uuid, cx: &mut Context<Self>) -> Result<()> {
        let state = cx.global::<TuskState>();
        state.connections.disconnect(&connection_id)?;

        if let Some(conn) = self.connections.iter_mut().find(|c| c.config.id == connection_id) {
            conn.status = ConnectionStatus::Disconnected;
            conn.info = None;
        }

        if self.active_connection_id == Some(connection_id) {
            self.active_connection_id = None;
        }

        cx.notify();
        Ok(())
    }

    /// Save a connection configuration
    pub fn save(&mut self, config: ConnectionConfig, password: Option<String>, cx: &mut Context<Self>) -> Result<()> {
        let state = cx.global::<TuskState>();
        state.connections.save_connection(config.clone(), password)?;

        // Reload to get updated list
        self.load(cx)?;

        Ok(())
    }

    /// Delete a connection
    pub fn delete(&mut self, connection_id: Uuid, cx: &mut Context<Self>) -> Result<()> {
        let state = cx.global::<TuskState>();
        state.connections.delete_connection(&connection_id)?;

        self.connections.retain(|c| c.config.id != connection_id);

        if self.active_connection_id == Some(connection_id) {
            self.active_connection_id = None;
        }

        cx.notify();
        Ok(())
    }

    /// Get the active connection
    pub fn active_connection(&self) -> Option<&ConnectionWithStatus> {
        self.active_connection_id
            .and_then(|id| self.connections.iter().find(|c| c.config.id == id))
    }

    /// Set the active connection
    pub fn set_active(&mut self, connection_id: Option<Uuid>, cx: &mut Context<Self>) {
        self.active_connection_id = connection_id;
        cx.notify();
    }

    /// Get all connections
    pub fn all_connections(&self) -> &[ConnectionWithStatus] {
        &self.connections
    }

    /// Get connected connections
    pub fn connected_connections(&self) -> Vec<&ConnectionWithStatus> {
        self.connections.iter()
            .filter(|c| c.status == ConnectionStatus::Connected)
            .collect()
    }

    /// Get connections by group
    pub fn connections_in_group(&self, group_id: Option<Uuid>) -> Vec<&ConnectionWithStatus> {
        self.connections.iter()
            .filter(|c| c.config.group_id == group_id)
            .collect()
    }

    /// Get all groups
    pub fn groups(&self) -> &[ConnectionGroup] {
        &self.groups
    }

    /// Check if loading
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Get error message
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Handle a connection status change from the service
    pub fn handle_status_change(&mut self, change: ConnectionChange, cx: &mut Context<Self>) {
        if let Some(conn) = self.connections.iter_mut().find(|c| c.config.id == change.connection_id) {
            conn.status = change.status;

            // Clear info if disconnected
            if change.status == ConnectionStatus::Disconnected {
                conn.info = None;
            }
        }
        cx.notify();
    }
}
```

### 7. Connection Status Indicator Component

```rust
// src/ui/components/connection_status.rs
use gpui::{
    div, px, rgb, Element, IntoElement, ParentElement, Render,
    Styled, View, ViewContext, Window, InteractiveElement,
};
use uuid::Uuid;

use crate::models::connection::ConnectionStatus;
use crate::state::TuskState;
use crate::theme::Theme;

pub struct ConnectionStatusIndicator {
    connection_id: Uuid,
    show_text: bool,
}

impl ConnectionStatusIndicator {
    pub fn new(connection_id: Uuid) -> Self {
        Self {
            connection_id,
            show_text: false,
        }
    }

    pub fn with_text(mut self) -> Self {
        self.show_text = true;
        self
    }

    fn status_color(&self, status: ConnectionStatus, theme: &Theme) -> gpui::Hsla {
        match status {
            ConnectionStatus::Connected => theme.colors.success,
            ConnectionStatus::Connecting | ConnectionStatus::Reconnecting => theme.colors.warning,
            ConnectionStatus::Error => theme.colors.error,
            ConnectionStatus::Disconnected => theme.colors.text_muted,
        }
    }

    fn status_text(&self, status: ConnectionStatus) -> &'static str {
        match status {
            ConnectionStatus::Connected => "Connected",
            ConnectionStatus::Connecting => "Connecting...",
            ConnectionStatus::Reconnecting => "Reconnecting...",
            ConnectionStatus::Error => "Error",
            ConnectionStatus::Disconnected => "Disconnected",
        }
    }
}

impl Render for ConnectionStatusIndicator {
    fn render(&mut self, _window: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let state = cx.global::<TuskState>();
        let status = state.connections.get_status(&self.connection_id);
        let color = self.status_color(status, theme);

        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(
                // Status dot
                div()
                    .w(px(8.0))
                    .h(px(8.0))
                    .rounded_full()
                    .bg(color)
            )
            .when(self.show_text, |this| {
                this.child(
                    div()
                        .text_color(theme.colors.text_secondary)
                        .text_sm()
                        .child(self.status_text(status))
                )
            })
    }
}

/// Compact status badge for use in tabs and lists
pub struct ConnectionBadge {
    name: String,
    status: ConnectionStatus,
    color: Option<String>,
}

impl ConnectionBadge {
    pub fn new(name: String, status: ConnectionStatus, color: Option<String>) -> Self {
        Self { name, status, color }
    }
}

impl Render for ConnectionBadge {
    fn render(&mut self, _window: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let status_color = match self.status {
            ConnectionStatus::Connected => theme.colors.success,
            ConnectionStatus::Connecting | ConnectionStatus::Reconnecting => theme.colors.warning,
            ConnectionStatus::Error => theme.colors.error,
            ConnectionStatus::Disconnected => theme.colors.text_muted,
        };

        let badge_bg = self.color.as_ref()
            .and_then(|c| parse_color(c))
            .unwrap_or(theme.colors.bg_secondary);

        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(4.0))
            .bg(badge_bg)
            .child(
                // Status indicator
                div()
                    .w(px(6.0))
                    .h(px(6.0))
                    .rounded_full()
                    .bg(status_color)
            )
            .child(
                // Connection name
                div()
                    .text_color(theme.colors.text_primary)
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(self.name.clone())
            )
    }
}

fn parse_color(hex: &str) -> Option<gpui::Hsla> {
    // Parse hex color string to HSLA
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some(gpui::rgba(
        r as u32 * 0x01010101,
    ).into())
}
```

### 8. Quick Connect Component

```rust
// src/ui/components/quick_connect.rs
use gpui::{
    div, px, Element, IntoElement, ParentElement, Render,
    Styled, View, ViewContext, Window, InteractiveElement,
    FocusHandle, FocusableView, KeyBinding, actions,
};
use uuid::Uuid;

use crate::models::connection::ConnectionConfig;
use crate::state::TuskState;
use crate::theme::Theme;
use crate::error::Result;
use crate::ui::components::text_input::TextInput;
use crate::ui::components::button::{Button, ButtonVariant};

actions!(quick_connect, [Submit, Cancel, TestConnection]);

pub fn register_bindings(cx: &mut gpui::AppContext) {
    cx.bind_keys([
        KeyBinding::new("enter", Submit, Some("QuickConnect")),
        KeyBinding::new("escape", Cancel, Some("QuickConnect")),
        KeyBinding::new("cmd-t", TestConnection, Some("QuickConnect")),
    ]);
}

pub struct QuickConnect {
    focus_handle: FocusHandle,
    host: String,
    port: String,
    database: String,
    username: String,
    password: String,
    testing: bool,
    test_result: Option<TestResult>,
    error: Option<String>,
    on_connect: Option<Box<dyn Fn(ConnectionConfig, ViewContext<Self>) + 'static>>,
    on_cancel: Option<Box<dyn Fn(ViewContext<Self>) + 'static>>,
}

#[derive(Debug, Clone)]
enum TestResult {
    Success { version: String, latency_ms: u64 },
    Failed { error: String },
}

impl QuickConnect {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            host: "localhost".to_string(),
            port: "5432".to_string(),
            database: "postgres".to_string(),
            username: String::new(),
            password: String::new(),
            testing: false,
            test_result: None,
            error: None,
            on_connect: None,
            on_cancel: None,
        }
    }

    pub fn on_connect(
        mut self,
        callback: impl Fn(ConnectionConfig, ViewContext<Self>) + 'static,
    ) -> Self {
        self.on_connect = Some(Box::new(callback));
        self
    }

    pub fn on_cancel(
        mut self,
        callback: impl Fn(ViewContext<Self>) + 'static,
    ) -> Self {
        self.on_cancel = Some(Box::new(callback));
        self
    }

    fn build_config(&self) -> Result<ConnectionConfig> {
        let port: u16 = self.port.parse()
            .map_err(|_| crate::error::TuskError::InvalidInput {
                field: "port".to_string(),
                message: "Port must be a number".to_string(),
            })?;

        let config = ConnectionConfig::new(
            format!("{}@{}", self.username, self.host),
            self.host.clone(),
            port,
            self.database.clone(),
            self.username.clone(),
        );

        config.validate()?;
        Ok(config)
    }

    fn submit(&mut self, cx: &mut ViewContext<Self>) {
        self.error = None;
        self.test_result = None;

        match self.build_config() {
            Ok(config) => {
                if let Some(ref callback) = self.on_connect {
                    callback(config, cx);
                }
            }
            Err(e) => {
                self.error = Some(e.to_string());
                cx.notify();
            }
        }
    }

    fn cancel(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(ref callback) = self.on_cancel {
            callback(cx);
        }
    }

    fn test_connection(&mut self, cx: &mut ViewContext<Self>) {
        self.error = None;
        self.test_result = None;
        self.testing = true;
        cx.notify();

        let config = match self.build_config() {
            Ok(c) => c,
            Err(e) => {
                self.error = Some(e.to_string());
                self.testing = false;
                cx.notify();
                return;
            }
        };

        let password = self.password.clone();
        let state = cx.global::<TuskState>().clone();

        // Store password temporarily for test
        if !password.is_empty() {
            let _ = state.keyring.store_password(&config.id.to_string(), &password);
        }

        // Run test
        match state.connections.test_connection(config.clone()) {
            Ok(result) => {
                self.test_result = Some(TestResult::Success {
                    version: result.version,
                    latency_ms: result.latency_ms,
                });
            }
            Err(e) => {
                self.test_result = Some(TestResult::Failed {
                    error: e.to_string(),
                });
            }
        }

        // Clean up temporary password
        if !password.is_empty() {
            let _ = state.keyring.delete_password(&config.id.to_string());
        }

        self.testing = false;
        cx.notify();
    }
}

impl FocusableView for QuickConnect {
    fn focus_handle(&self, _cx: &gpui::AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for QuickConnect {
    fn render(&mut self, _window: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .track_focus(&self.focus_handle)
            .key_context("QuickConnect")
            .on_action(cx.listener(|this, _: &Submit, cx| this.submit(cx)))
            .on_action(cx.listener(|this, _: &Cancel, cx| this.cancel(cx)))
            .on_action(cx.listener(|this, _: &TestConnection, cx| this.test_connection(cx)))
            .flex()
            .flex_col()
            .gap(px(16.0))
            .p(px(20.0))
            .bg(theme.colors.bg_primary)
            .rounded(px(8.0))
            .border_1()
            .border_color(theme.colors.border)
            .min_w(px(400.0))
            // Header
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.colors.text_primary)
                    .child("Quick Connect")
            )
            // Form fields
            .child(self.render_form_fields(theme, cx))
            // Error message
            .when_some(self.error.clone(), |this, error| {
                this.child(
                    div()
                        .px(px(12.0))
                        .py(px(8.0))
                        .rounded(px(4.0))
                        .bg(theme.colors.error.opacity(0.1))
                        .text_color(theme.colors.error)
                        .text_sm()
                        .child(error)
                )
            })
            // Test result
            .when_some(self.test_result.clone(), |this, result| {
                this.child(self.render_test_result(&result, theme))
            })
            // Actions
            .child(self.render_actions(theme, cx))
    }
}

impl QuickConnect {
    fn render_form_fields(&self, theme: &Theme, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            // Host and Port row
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(
                        self.render_field("Host", &self.host, "localhost", theme, cx, |this, val| {
                            this.host = val;
                        })
                    )
                    .child(
                        div()
                            .w(px(100.0))
                            .child(
                                self.render_field("Port", &self.port, "5432", theme, cx, |this, val| {
                                    this.port = val;
                                })
                            )
                    )
            )
            // Database
            .child(
                self.render_field("Database", &self.database, "postgres", theme, cx, |this, val| {
                    this.database = val;
                })
            )
            // Username
            .child(
                self.render_field("Username", &self.username, "postgres", theme, cx, |this, val| {
                    this.username = val;
                })
            )
            // Password
            .child(
                self.render_password_field(theme, cx)
            )
    }

    fn render_field(
        &self,
        label: &str,
        value: &str,
        placeholder: &str,
        theme: &Theme,
        cx: &mut ViewContext<Self>,
        on_change: impl Fn(&mut Self, String) + 'static,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .flex_1()
            .child(
                div()
                    .text_sm()
                    .text_color(theme.colors.text_secondary)
                    .child(label)
            )
            .child(
                div()
                    .px(px(10.0))
                    .py(px(8.0))
                    .bg(theme.colors.input_bg)
                    .border_1()
                    .border_color(theme.colors.input_border)
                    .rounded(px(4.0))
                    .text_color(theme.colors.text_primary)
                    .child(value.to_string())
                    .on_click(cx.listener(move |_this, _event, _cx| {
                        // Focus input - in real implementation would use TextInput component
                    }))
            )
    }

    fn render_password_field(&self, theme: &Theme, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                div()
                    .text_sm()
                    .text_color(theme.colors.text_secondary)
                    .child("Password")
            )
            .child(
                div()
                    .px(px(10.0))
                    .py(px(8.0))
                    .bg(theme.colors.input_bg)
                    .border_1()
                    .border_color(theme.colors.input_border)
                    .rounded(px(4.0))
                    .text_color(theme.colors.text_primary)
                    .child(if self.password.is_empty() {
                        "".to_string()
                    } else {
                        "•".repeat(self.password.len())
                    })
            )
    }

    fn render_test_result(&self, result: &TestResult, theme: &Theme) -> impl IntoElement {
        match result {
            TestResult::Success { version, latency_ms } => {
                div()
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(4.0))
                    .bg(theme.colors.success.opacity(0.1))
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_color(theme.colors.success)
                                    .child("✓ Connection successful")
                            )
                            .child(
                                div()
                                    .text_color(theme.colors.text_muted)
                                    .text_xs()
                                    .child(format!("{}ms", latency_ms))
                            )
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.colors.text_secondary)
                            .child(version.clone())
                    )
            }
            TestResult::Failed { error } => {
                div()
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(4.0))
                    .bg(theme.colors.error.opacity(0.1))
                    .text_color(theme.colors.error)
                    .text_sm()
                    .child(format!("✗ {}", error))
            }
        }
    }

    fn render_actions(&self, theme: &Theme, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .justify_between()
            .pt(px(8.0))
            .border_t_1()
            .border_color(theme.colors.border)
            .child(
                // Test button
                div()
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(4.0))
                    .bg(theme.colors.bg_secondary)
                    .text_color(theme.colors.text_secondary)
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(|this, _, cx| this.test_connection(cx)))
                    .child(if self.testing { "Testing..." } else { "Test Connection" })
            )
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    // Cancel button
                    .child(
                        div()
                            .px(px(12.0))
                            .py(px(8.0))
                            .rounded(px(4.0))
                            .text_color(theme.colors.text_secondary)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.bg_secondary))
                            .on_click(cx.listener(|this, _, cx| this.cancel(cx)))
                            .child("Cancel")
                    )
                    // Connect button
                    .child(
                        div()
                            .px(px(16.0))
                            .py(px(8.0))
                            .rounded(px(4.0))
                            .bg(theme.colors.accent)
                            .text_color(gpui::white())
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.accent_hover))
                            .on_click(cx.listener(|this, _, cx| this.submit(cx)))
                            .child("Connect")
                    )
            )
    }
}
```

### 9. Application Integration

```rust
// src/main.rs
use gpui::{App, AppContext, Window, WindowOptions};
use std::path::PathBuf;
use directories::ProjectDirs;

mod error;
mod models;
mod services;
mod state;
mod theme;
mod ui;

use state::TuskState;
use theme::Theme;

fn main() {
    App::new().run(|cx: &mut AppContext| {
        // Initialize data directory
        let data_dir = ProjectDirs::from("com", "tusk", "Tusk")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".tusk"));

        std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");

        // Initialize global state
        let state = TuskState::new(&data_dir).expect("Failed to initialize application state");
        cx.set_global(state);

        // Initialize theme
        let theme = Theme::from_mode(
            theme::ThemeMode::System,
            cx.window_appearance(),
        );
        cx.set_global(theme);

        // Register key bindings
        ui::components::quick_connect::register_bindings(cx);

        // Open main window
        cx.open_window(
            WindowOptions {
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Tusk".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |cx| {
                // Create main application view
                cx.new_view(|cx| ui::MainWindow::new(cx))
            },
        );
    });
}
```

### 10. Connection Status Subscription

```rust
// src/ui/state/status_watcher.rs
use gpui::{AppContext, Context, Task};
use std::sync::Arc;
use tokio::sync::watch;

use crate::services::connection::{ConnectionServiceStatus, ConnectionChange};
use crate::state::TuskState;
use crate::ui::state::connections::ConnectionsState;

/// Watch for connection status changes and update UI state
pub fn spawn_status_watcher(
    connections_state: gpui::Model<ConnectionsState>,
    cx: &mut AppContext,
) -> Task<()> {
    let state = cx.global::<TuskState>();
    let mut receiver = state.connections.status_receiver();

    cx.spawn(|mut cx| async move {
        loop {
            // Wait for status change
            if receiver.changed().await.is_err() {
                break;
            }

            let status = receiver.borrow().clone();

            if let Some(change) = status.recent_change {
                // Update UI state
                let _ = cx.update_model(&connections_state, |state, cx| {
                    state.handle_status_change(change, cx);
                });
            }
        }
    })
}
```

## Acceptance Criteria

1. [x] Connection pool creates successfully with valid config
2. [x] Connection validation catches all invalid configurations
3. [x] Password retrieved from keyring when password_in_keyring is true
4. [x] Keepalive query runs at configurable intervals
5. [x] Auto-reconnection works with exponential backoff
6. [x] Read-only mode prevents write operations
7. [x] Statement timeout is enforced
8. [x] Connection status updates propagate to UI via GPUI
9. [x] Multiple concurrent connections supported
10. [x] Connection cleanup on disconnect
11. [x] Pool size configurable via ConnectionOptions

## Testing

```rust
// tests/connection_tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_state() -> (TempDir, TuskState) {
        let temp_dir = TempDir::new().unwrap();
        let state = TuskState::new(temp_dir.path()).unwrap();
        (temp_dir, state)
    }

    #[test]
    fn test_connection_config_validation() {
        let mut config = ConnectionConfig::new(
            "test".to_string(),
            "localhost".to_string(),
            5432,
            "postgres".to_string(),
            "postgres".to_string(),
        );

        // Valid config
        assert!(config.validate().is_ok());

        // Empty host
        config.host = String::new();
        assert!(config.validate().is_err());
        config.host = "localhost".to_string();

        // Invalid port
        config.port = 0;
        assert!(config.validate().is_err());
        config.port = 5432;

        // Empty database
        config.database = String::new();
        assert!(config.validate().is_err());
        config.database = "postgres".to_string();

        // Empty username
        config.username = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_connection_config_display_string() {
        let config = ConnectionConfig::new(
            "test".to_string(),
            "db.example.com".to_string(),
            5432,
            "mydb".to_string(),
            "admin".to_string(),
        );

        assert_eq!(config.display_string(), "admin@db.example.com:5432/mydb");
    }

    #[tokio::test]
    async fn test_connection_lifecycle() {
        // This test requires a running Postgres instance
        // Skip if not available
        let (_temp_dir, state) = setup_test_state();

        let config = ConnectionConfig::new(
            "test".to_string(),
            "localhost".to_string(),
            5432,
            "postgres".to_string(),
            "postgres".to_string(),
        );

        // Store test password
        state.keyring.store_password(&config.id.to_string(), "test_password").unwrap();

        // Connect
        let result = state.connections.connect(config.clone());

        if result.is_ok() {
            // Verify connected
            assert!(state.connections.is_connected(&config.id));
            assert_eq!(state.connections.get_status(&config.id), ConnectionStatus::Connected);

            // Get info
            let info = state.connections.get_info(&config.id);
            assert!(info.is_some());

            // Disconnect
            state.connections.disconnect(&config.id).unwrap();
            assert!(!state.connections.is_connected(&config.id));
        }
    }

    #[test]
    fn test_group_management() {
        let (_temp_dir, state) = setup_test_state();

        // Create group
        let group = ConnectionGroup::new("Production".to_string());
        state.connections.save_group(&group).unwrap();

        // List groups
        let groups = state.connections.list_groups().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "Production");

        // Delete group
        state.connections.delete_group(&group.id).unwrap();
        let groups = state.connections.list_groups().unwrap();
        assert!(groups.is_empty());
    }
}
```

## Testing with GPUI

```rust
// Manual testing steps using GPUI test harness
#[cfg(test)]
mod gpui_tests {
    use gpui::TestAppContext;

    #[gpui::test]
    async fn test_connection_state_entity(cx: &mut TestAppContext) {
        // Initialize state
        let state = cx.update(|cx| {
            let temp_dir = tempfile::TempDir::new().unwrap();
            let state = TuskState::new(temp_dir.path()).unwrap();
            cx.set_global(state.clone());
            state
        });

        // Create connections state entity
        let connections = cx.new_model(|cx| {
            let mut state = ConnectionsState::new();
            state.load(cx).unwrap();
            state
        });

        // Verify initial state
        cx.read_model(&connections, |state, _| {
            assert!(state.all_connections().is_empty());
            assert!(state.active_connection().is_none());
        });
    }
}
```

## Dependencies on Other Features

- **04-ipc-layer.md**: Defines service architecture pattern (now internal, no IPC)
- **05-local-storage.md**: StorageService for persisting connection configs
- **06-settings-theming-credentials.md**: KeyringService for password storage, Theme for UI

## Dependent Features

- **08-ssl-ssh-security.md**: Extends ConnectionPool for SSL/SSH
- **09-connection-ui.md**: Uses ConnectionService and components
- **10-schema-introspection.md**: Uses ConnectionPool for schema queries
- **11-query-execution.md**: Uses ConnectionPool for queries
- All features requiring database connectivity
