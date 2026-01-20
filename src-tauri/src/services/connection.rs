// Connection service - Phase 6 (User Story 4)

use crate::error::{TuskError, TuskResult};
use crate::models::{ActiveConnection, ConnectionConfig, ConnectionTestResult, SslMode};
use crate::services::ssh_tunnel::{SshTunnelHandle, SshTunnelService};
use crate::state::AppState;
use chrono::Utc;
use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod, Runtime};
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use std::time::Instant;
use tauri::State;
use tokio_postgres::NoTls;
use uuid::Uuid;

/// Connection pool entry stored in AppState.
pub struct ConnectionPoolEntry {
    /// Pool ID (same as connection config ID)
    pub id: Uuid,
    /// Connection configuration name
    pub config_name: String,
    /// The actual connection pool
    pub pool: Pool,
    /// When the pool was created
    pub connected_at: chrono::DateTime<Utc>,
    /// SSL mode for the connection (needed for query cancellation)
    pub ssl_mode: SslMode,
    /// SSH tunnel handle if connection uses SSH tunneling
    pub ssh_tunnel: Option<SshTunnelHandle>,
}

/// Connection service for managing PostgreSQL connection pools.
pub struct ConnectionService;

impl ConnectionService {
    /// Create a new deadpool-postgres pool from a connection configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The connection configuration
    /// * `password` - The password to use for authentication
    /// * `tunnel_override` - Optional (host, port) override for SSH tunnel connections
    ///
    /// # Returns
    ///
    /// Returns a configured connection pool ready for use.
    pub fn create_pool(
        config: &ConnectionConfig,
        password: &str,
        tunnel_override: Option<(&str, u16)>,
    ) -> TuskResult<Pool> {
        let mut pg_config = Config::new();

        // Use tunnel endpoint if provided, otherwise use config host/port
        let (host, port) = tunnel_override.unwrap_or((&config.host, config.port));
        pg_config.host = Some(host.to_string());
        pg_config.port = Some(port);
        pg_config.dbname = Some(config.database.clone());
        pg_config.user = Some(config.username.clone());
        pg_config.password = Some(password.to_string());

        // Set connection pool configuration
        pg_config.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        // Set statement timeout if configured
        if let Some(timeout_ms) = config.statement_timeout_ms {
            let timeout_str = format!("{}ms", timeout_ms);
            pg_config.options = Some(format!("-c statement_timeout={}", timeout_str));
        }

        // Create pool based on SSL mode
        match config.ssl_mode {
            SslMode::Disable => {
                pg_config
                    .create_pool(Some(Runtime::Tokio1), NoTls)
                    .map_err(|e| TuskError::connection_with_hint(
                        format!("Failed to create connection pool: {}", e),
                        "Check connection settings and try again",
                    ))
            }
            SslMode::Prefer | SslMode::Require => {
                // Both Prefer and Require use TLS without certificate verification.
                // Only VerifyCa and VerifyFull perform certificate verification.
                let connector = TlsConnector::builder()
                    .danger_accept_invalid_certs(true)
                    .build()
                    .map_err(|e| TuskError::connection_with_hint(
                        format!("Failed to create TLS connector: {}", e),
                        "Check your system's TLS configuration",
                    ))?;
                let tls = MakeTlsConnector::new(connector);

                pg_config
                    .create_pool(Some(Runtime::Tokio1), tls)
                    .map_err(|e| TuskError::connection_with_hint(
                        format!("Failed to create connection pool: {}", e),
                        "Check connection settings and try again",
                    ))
            }
            SslMode::VerifyCa | SslMode::VerifyFull => {
                // Use native TLS with certificate verification
                let mut builder = TlsConnector::builder();

                // Add CA certificate if provided
                if let Some(ca_cert_path) = &config.ssl_ca_cert {
                    let cert_data = std::fs::read(ca_cert_path).map_err(|e| {
                        TuskError::connection_with_hint(
                            format!("Failed to read CA certificate: {}", e),
                            "Check that the certificate file exists and is readable",
                        )
                    })?;
                    let cert = native_tls::Certificate::from_pem(&cert_data).map_err(|e| {
                        TuskError::connection_with_hint(
                            format!("Failed to parse CA certificate: {}", e),
                            "Ensure the certificate is in PEM format",
                        )
                    })?;
                    builder.add_root_certificate(cert);
                }

                let connector = builder.build().map_err(|e| {
                    TuskError::connection_with_hint(
                        format!("Failed to create TLS connector: {}", e),
                        "Check your SSL configuration",
                    )
                })?;
                let tls = MakeTlsConnector::new(connector);

                pg_config
                    .create_pool(Some(Runtime::Tokio1), tls)
                    .map_err(|e| TuskError::connection_with_hint(
                        format!("Failed to create connection pool: {}", e),
                        "Check connection settings and try again",
                    ))
            }
        }
    }

    /// Test a connection without saving it.
    ///
    /// # Arguments
    ///
    /// * `config` - The connection configuration to test
    /// * `password` - The PostgreSQL password
    /// * `ssh_password` - SSH password if using password authentication (from keychain)
    /// * `key_passphrase` - SSH key passphrase if using key file authentication (from keychain)
    ///
    /// # Returns
    ///
    /// Returns a `ConnectionTestResult` indicating success or failure.
    pub async fn test_connection(
        config: &ConnectionConfig,
        password: &str,
        ssh_password: Option<&str>,
        key_passphrase: Option<&str>,
    ) -> ConnectionTestResult {
        let start = Instant::now();

        // Validate configuration first
        if let Err(e) = config.validate() {
            return ConnectionTestResult {
                success: false,
                server_version: None,
                latency_ms: start.elapsed().as_millis() as u64,
                error: Some(crate::models::ErrorResponse::from(e)),
            };
        }

        // Establish SSH tunnel if configured
        let tunnel_handle = if config.ssh_tunnel.is_some() {
            match SshTunnelService::establish(config, ssh_password, key_passphrase).await {
                Ok(handle) => Some(handle),
                Err(e) => {
                    return ConnectionTestResult {
                        success: false,
                        server_version: None,
                        latency_ms: start.elapsed().as_millis() as u64,
                        error: Some(crate::models::ErrorResponse::from(e)),
                    };
                }
            }
        } else {
            None
        };

        // Get tunnel override if we have a tunnel
        let tunnel_override = tunnel_handle
            .as_ref()
            .map(|h| ("127.0.0.1", h.local_port));

        // Try to create and test the pool
        let result = match Self::create_pool(config, password, tunnel_override) {
            Ok(pool) => {
                // Try to get a connection and run a simple query
                match pool.get().await {
                    Ok(client) => {
                        match client.query_one("SELECT version()", &[]).await {
                            Ok(row) => {
                                let version: String = row.get(0);
                                ConnectionTestResult {
                                    success: true,
                                    server_version: Some(version),
                                    latency_ms: start.elapsed().as_millis() as u64,
                                    error: None,
                                }
                            }
                            Err(e) => {
                                let error = TuskError::from(e);
                                ConnectionTestResult {
                                    success: false,
                                    server_version: None,
                                    latency_ms: start.elapsed().as_millis() as u64,
                                    error: Some(crate::models::ErrorResponse::from(error)),
                                }
                            }
                        }
                    }
                    Err(e) => {
                        ConnectionTestResult {
                            success: false,
                            server_version: None,
                            latency_ms: start.elapsed().as_millis() as u64,
                            error: Some(crate::models::ErrorResponse::from(
                                TuskError::connection(format!("Failed to get connection: {}", e)),
                            )),
                        }
                    }
                }
            }
            Err(e) => {
                ConnectionTestResult {
                    success: false,
                    server_version: None,
                    latency_ms: start.elapsed().as_millis() as u64,
                    error: Some(crate::models::ErrorResponse::from(e)),
                }
            }
        };

        // Stop the tunnel (it will be re-established when actually connecting)
        if let Some(handle) = tunnel_handle {
            handle.stop();
        }

        result
    }

    /// Connect to a database using a saved configuration.
    ///
    /// # Arguments
    ///
    /// * `state` - Application state
    /// * `config` - The connection configuration
    /// * `password` - The PostgreSQL password (retrieved from keychain)
    /// * `ssh_password` - SSH password if using password authentication (from keychain)
    /// * `key_passphrase` - SSH key passphrase if using key file authentication (from keychain)
    ///
    /// # Returns
    ///
    /// Returns the pool ID (same as config ID) on success.
    pub async fn connect(
        state: &State<'_, AppState>,
        config: &ConnectionConfig,
        password: &str,
        ssh_password: Option<&str>,
        key_passphrase: Option<&str>,
    ) -> TuskResult<Uuid> {
        // Validate configuration
        config.validate()?;

        // Check if already connected
        {
            let connections = state.connections.read().await;
            if connections.contains_key(&config.id) {
                tracing::info!("Already connected to: {} ({})", config.name, config.id);
                return Ok(config.id);
            }
        }

        // Establish SSH tunnel if configured
        let tunnel_handle = if config.ssh_tunnel.is_some() {
            let handle =
                SshTunnelService::establish(config, ssh_password, key_passphrase).await?;
            tracing::info!(
                "SSH tunnel established on local port {} for connection {}",
                handle.local_port,
                config.id
            );
            Some(handle)
        } else {
            None
        };

        // Get tunnel override if we have a tunnel
        let tunnel_override = tunnel_handle
            .as_ref()
            .map(|h| ("127.0.0.1", h.local_port));

        // Create the pool
        let pool = match Self::create_pool(config, password, tunnel_override) {
            Ok(p) => p,
            Err(e) => {
                // Stop the tunnel if pool creation fails
                if let Some(handle) = tunnel_handle {
                    handle.stop();
                }
                return Err(e);
            }
        };

        // Test the connection
        let client = match pool.get().await {
            Ok(c) => c,
            Err(e) => {
                // Stop the tunnel if connection fails
                if let Some(handle) = tunnel_handle {
                    handle.stop();
                }
                return Err(TuskError::connection_with_hint(
                    format!("Failed to establish connection: {}", e),
                    "Verify the database server is running and credentials are correct",
                ));
            }
        };

        // Run a simple query to verify
        if let Err(e) = client.query_one("SELECT 1", &[]).await {
            // Stop the tunnel if query fails
            if let Some(handle) = tunnel_handle {
                handle.stop();
            }
            return Err(TuskError::from(e));
        }

        // Store in state
        let entry = ConnectionPoolEntry {
            id: config.id,
            config_name: config.name.clone(),
            pool,
            connected_at: Utc::now(),
            ssl_mode: config.ssl_mode,
            ssh_tunnel: tunnel_handle,
        };

        {
            let mut connections = state.connections.write().await;
            connections.insert(config.id, entry);
        }

        tracing::info!("Connected to: {} ({})", config.name, config.id);
        Ok(config.id)
    }

    /// Disconnect from a database.
    ///
    /// # Arguments
    ///
    /// * `state` - Application state
    /// * `id` - The pool ID to disconnect
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    pub async fn disconnect(state: &State<'_, AppState>, id: &Uuid) -> TuskResult<()> {
        let removed = {
            let mut connections = state.connections.write().await;
            connections.remove(id)
        };

        if let Some(entry) = removed {
            // Stop the SSH tunnel if one exists
            if let Some(tunnel) = entry.ssh_tunnel {
                tracing::info!(
                    "Stopping SSH tunnel for connection {} (local port {})",
                    id,
                    tunnel.local_port
                );
                tunnel.stop();
            }

            // The pool will be dropped and connections closed
            tracing::info!("Disconnected from: {} ({})", entry.config_name, id);
        } else {
            tracing::warn!("Attempted to disconnect non-existent pool: {}", id);
        }

        Ok(())
    }

    /// Get all active connections.
    ///
    /// # Arguments
    ///
    /// * `state` - Application state
    ///
    /// # Returns
    ///
    /// Returns a list of active connections.
    pub async fn get_active_connections(state: &State<'_, AppState>) -> Vec<ActiveConnection> {
        let connections = state.connections.read().await;
        let queries = state.active_queries.read().await;

        connections
            .values()
            .map(|entry| {
                // Count active queries for this connection
                let active_queries = queries
                    .values()
                    .filter(|q| q.connection_id == entry.id)
                    .count();

                ActiveConnection {
                    id: entry.id,
                    config_name: entry.config_name.clone(),
                    connected_at: entry.connected_at,
                    active_queries,
                }
            })
            .collect()
    }

    /// Get a connection pool by ID.
    ///
    /// # Arguments
    ///
    /// * `state` - Application state
    /// * `id` - The pool ID
    ///
    /// # Returns
    ///
    /// Returns the pool if found.
    pub async fn get_pool(state: &State<'_, AppState>, id: &Uuid) -> TuskResult<Pool> {
        let connections = state.connections.read().await;
        connections
            .get(id)
            .map(|e| e.pool.clone())
            .ok_or_else(|| TuskError::connection_with_hint(
                format!("Connection pool not found: {}", id),
                "The connection may have been closed. Try reconnecting.",
            ))
    }

    /// Get the SSL mode for a connection by ID.
    ///
    /// # Arguments
    ///
    /// * `state` - Application state
    /// * `id` - The pool ID
    ///
    /// # Returns
    ///
    /// Returns the SSL mode if the connection exists.
    pub async fn get_ssl_mode(state: &State<'_, AppState>, id: &Uuid) -> TuskResult<SslMode> {
        let connections = state.connections.read().await;
        connections
            .get(id)
            .map(|e| e.ssl_mode)
            .ok_or_else(|| TuskError::connection_with_hint(
                format!("Connection not found: {}", id),
                "The connection may have been closed. Try reconnecting.",
            ))
    }
}
