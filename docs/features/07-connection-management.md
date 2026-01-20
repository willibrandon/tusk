# Feature 07: Connection Management

## Overview

Implement the core connection management system including the connection model, connection pooling with deadpool-postgres, connection lifecycle management, validation, and auto-reconnection.

## Goals

- Implement ConnectionConfig model with all fields from design doc
- Create connection pool with deadpool-postgres
- Handle connection lifecycle (connect, keepalive, disconnect)
- Implement auto-reconnection with exponential backoff
- Support read-only mode and statement timeout

## Technical Specification

### 1. Connection Pool Implementation

```rust
// services/connection.rs
use std::sync::Arc;
use std::time::Duration;
use deadpool_postgres::{Config, Pool, Runtime, PoolConfig, Timeouts};
use tokio_postgres::{NoTls, Config as PgConfig};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::{Result, TuskError};
use crate::models::connection::{ConnectionConfig, ConnectionOptions, SslMode};
use crate::services::keyring::KeyringService;

pub struct ConnectionPool {
    pool: Pool,
    config: ConnectionConfig,
    status: Arc<RwLock<ConnectionStatus>>,
    keepalive_handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

impl ConnectionPool {
    pub async fn new(config: ConnectionConfig) -> Result<Self> {
        // Get password from keyring if needed
        let password = if config.password_in_keyring {
            KeyringService::get_password(&config.id.to_string())?
                .ok_or_else(|| TuskError::CredentialNotFound(config.id.to_string()))?
        } else {
            String::new() // Empty password (trust auth, etc.)
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

        // Handle SSL (basic - full SSL in Feature 08)
        let pool = match config.ssl_mode {
            SslMode::Disable => {
                Self::create_pool_no_tls(pg_config, &config.options).await?
            }
            _ => {
                // For now, use NoTls - full SSL support in Feature 08
                Self::create_pool_no_tls(pg_config, &config.options).await?
            }
        };

        let status = Arc::new(RwLock::new(ConnectionStatus::Connected));

        let mut conn_pool = Self {
            pool,
            config,
            status,
            keepalive_handle: None,
        };

        // Start keepalive task
        conn_pool.start_keepalive();

        Ok(conn_pool)
    }

    async fn create_pool_no_tls(pg_config: PgConfig, options: &ConnectionOptions) -> Result<Pool> {
        let mut pool_config = Config::new();
        pool_config.host = Some(pg_config.get_hosts()[0].to_string());
        pool_config.port = pg_config.get_ports().first().copied();
        pool_config.dbname = pg_config.get_dbname().map(|s| s.to_string());
        pool_config.user = pg_config.get_user().map(|s| s.to_string());

        let pool_cfg = PoolConfig {
            max_size: 10,
            timeouts: Timeouts {
                wait: Some(Duration::from_secs(options.connect_timeout_sec)),
                create: Some(Duration::from_secs(options.connect_timeout_sec)),
                recycle: Some(Duration::from_secs(30)),
            },
            ..Default::default()
        };

        pool_config.pool = Some(pool_cfg);

        let pool = pool_config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| TuskError::ConnectionFailed {
                message: format!("Failed to create connection pool: {}", e),
                source: Some(Box::new(e)),
            })?;

        // Test the connection
        let client = pool.get().await.map_err(|e| TuskError::ConnectionFailed {
            message: format!("Failed to connect: {}", e),
            source: None,
        })?;

        // Set read-only mode if configured
        if options.readonly {
            client
                .execute("SET default_transaction_read_only = ON", &[])
                .await
                .map_err(|e| TuskError::QueryFailed {
                    message: format!("Failed to set read-only mode: {}", e),
                    detail: None,
                    hint: None,
                    position: None,
                    code: None,
                })?;
        }

        Ok(pool)
    }

    pub async fn get_client(&self) -> Result<deadpool_postgres::Client> {
        self.pool.get().await.map_err(|e| {
            TuskError::ConnectionFailed {
                message: format!("Failed to get connection from pool: {}", e),
                source: None,
            }
        })
    }

    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    pub async fn status(&self) -> ConnectionStatus {
        *self.status.read().await
    }

    fn start_keepalive(&mut self) {
        let pool = self.pool.clone();
        let status = self.status.clone();
        let interval_secs = 60; // Keepalive every 60 seconds

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

            loop {
                interval.tick().await;

                match pool.get().await {
                    Ok(client) => {
                        match client.query_one("SELECT 1", &[]).await {
                            Ok(_) => {
                                let mut s = status.write().await;
                                if *s == ConnectionStatus::Reconnecting {
                                    *s = ConnectionStatus::Connected;
                                    tracing::info!("Reconnected successfully");
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Keepalive query failed: {}", e);
                                *status.write().await = ConnectionStatus::Reconnecting;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Keepalive connection failed: {}", e);
                        *status.write().await = ConnectionStatus::Reconnecting;
                    }
                }
            }
        });

        self.keepalive_handle = Some(handle);
    }

    pub async fn close(&mut self) {
        if let Some(handle) = self.keepalive_handle.take() {
            handle.abort();
        }
        *self.status.write().await = ConnectionStatus::Disconnected;
        self.pool.close();
    }

    /// Execute a query with auto-reconnection
    pub async fn execute_with_retry<F, T, Fut>(
        &self,
        max_retries: u32,
        mut operation: F,
    ) -> Result<T>
    where
        F: FnMut(deadpool_postgres::Client) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;
        let mut delay = Duration::from_millis(100);

        for attempt in 0..=max_retries {
            if attempt > 0 {
                tracing::info!("Retry attempt {} after {:?}", attempt, delay);
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
                                continue;
                            }
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    if Self::is_retryable_error(&e) {
                        last_error = Some(e);
                        continue;
                    }
                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| TuskError::ConnectionFailed {
            message: "Max retries exceeded".to_string(),
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
}

impl Drop for ConnectionPool {
    fn drop(&mut self) {
        if let Some(handle) = self.keepalive_handle.take() {
            handle.abort();
        }
    }
}
```

### 2. Connection Validation

```rust
// services/connection.rs (continued)

impl ConnectionConfig {
    pub fn validate(&self) -> Result<()> {
        // Host validation
        if self.host.is_empty() {
            return Err(TuskError::InvalidInput("Host is required".to_string()));
        }

        // Port validation
        if self.port == 0 || self.port > 65535 {
            return Err(TuskError::InvalidInput(
                "Port must be between 1 and 65535".to_string()
            ));
        }

        // Database validation
        if self.database.is_empty() {
            return Err(TuskError::InvalidInput("Database name is required".to_string()));
        }

        // Username validation
        if self.username.is_empty() {
            return Err(TuskError::InvalidInput("Username is required".to_string()));
        }

        // SSH tunnel validation
        if let Some(ref ssh) = self.ssh_tunnel {
            if ssh.enabled {
                if ssh.host.is_empty() {
                    return Err(TuskError::InvalidInput(
                        "SSH host is required when tunnel is enabled".to_string()
                    ));
                }
                if ssh.username.is_empty() {
                    return Err(TuskError::InvalidInput(
                        "SSH username is required".to_string()
                    ));
                }
                if matches!(ssh.auth, crate::models::connection::SshAuthMethod::Key)
                    && ssh.key_path.is_none()
                {
                    return Err(TuskError::InvalidInput(
                        "SSH key path is required for key authentication".to_string()
                    ));
                }
            }
        }

        // SSL validation
        if matches!(self.ssl_mode, SslMode::VerifyCa | SslMode::VerifyFull)
            && self.ssl_ca_cert.is_none()
        {
            return Err(TuskError::InvalidInput(
                "CA certificate is required for SSL verification".to_string()
            ));
        }

        Ok(())
    }
}
```

### 3. Connection Commands

```rust
// commands/connection.rs
use tauri::{command, State, AppHandle, Emitter};
use uuid::Uuid;
use serde::Serialize;

use crate::state::AppState;
use crate::error::{Result, TuskError};
use crate::models::connection::{ConnectionConfig, ConnectionStatus, ConnectionGroup};
use crate::services::connection::ConnectionPool;

#[derive(Debug, Serialize, Clone)]
pub struct ConnectionStatusEvent {
    pub connection_id: Uuid,
    pub status: ConnectionStatus,
    pub error: Option<String>,
}

#[command]
pub async fn connect(
    app: AppHandle,
    state: State<'_, AppState>,
    config: ConnectionConfig,
) -> Result<ConnectionInfo> {
    // Validate config
    config.validate()?;

    tracing::info!("Connecting to: {}:{}/{}", config.host, config.port, config.database);

    // Emit connecting status
    app.emit("connection:status", ConnectionStatusEvent {
        connection_id: config.id,
        status: ConnectionStatus::Connecting,
        error: None,
    })?;

    // Create connection pool (handles SSH tunnel in Feature 08)
    let pool = match ConnectionPool::new(config.clone()).await {
        Ok(pool) => pool,
        Err(e) => {
            app.emit("connection:status", ConnectionStatusEvent {
                connection_id: config.id,
                status: ConnectionStatus::Error,
                error: Some(e.to_string()),
            })?;
            return Err(e);
        }
    };

    // Get server info
    let client = pool.get_client().await?;
    let row = client.query_one(
        "SELECT version(), current_database(), current_user, pg_backend_pid()",
        &[],
    ).await?;

    let server_version: String = row.get(0);
    let current_database: String = row.get(1);
    let current_user: String = row.get(2);
    let backend_pid: i32 = row.get(3);

    // Store in state
    state.add_connection(config.id, pool).await;

    // Update storage with last connected time
    state.storage.update_connection_last_used(&config.id).await?;

    // Emit connected status
    app.emit("connection:status", ConnectionStatusEvent {
        connection_id: config.id,
        status: ConnectionStatus::Connected,
        error: None,
    })?;

    Ok(ConnectionInfo {
        id: config.id,
        server_version,
        current_database,
        current_user,
        backend_pid,
    })
}

#[derive(Debug, Serialize)]
pub struct ConnectionInfo {
    pub id: Uuid,
    pub server_version: String,
    pub current_database: String,
    pub current_user: String,
    pub backend_pid: i32,
}

#[command]
pub async fn disconnect(
    app: AppHandle,
    state: State<'_, AppState>,
    connection_id: Uuid,
) -> Result<()> {
    tracing::info!("Disconnecting: {}", connection_id);

    if let Some(pool) = state.remove_connection(&connection_id).await {
        // Pool is dropped here, closing all connections
        drop(pool);
    }

    app.emit("connection:status", ConnectionStatusEvent {
        connection_id,
        status: ConnectionStatus::Disconnected,
        error: None,
    })?;

    Ok(())
}

#[command]
pub async fn test_connection(config: ConnectionConfig) -> Result<TestConnectionResult> {
    config.validate()?;

    tracing::info!("Testing connection to: {}:{}", config.host, config.port);

    let start = std::time::Instant::now();

    // Create temporary pool
    let pool = ConnectionPool::new(config).await?;
    let client = pool.get_client().await?;

    // Get server info
    let row = client.query_one(
        "SELECT version(), pg_postmaster_start_time()",
        &[],
    ).await?;

    let version: String = row.get(0);
    let started_at: chrono::DateTime<chrono::Utc> = row.get(1);

    let elapsed_ms = start.elapsed().as_millis() as u64;

    Ok(TestConnectionResult {
        success: true,
        version,
        started_at: started_at.to_rfc3339(),
        latency_ms: elapsed_ms,
    })
}

#[derive(Debug, Serialize)]
pub struct TestConnectionResult {
    pub success: bool,
    pub version: String,
    pub started_at: String,
    pub latency_ms: u64,
}

#[command]
pub async fn list_connections(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectionWithStatus>> {
    let configs = state.storage.get_all_connections().await?;

    let mut result = Vec::new();
    for config in configs {
        let status = if state.get_connection(&config.id).await.is_some() {
            ConnectionStatus::Connected
        } else {
            ConnectionStatus::Disconnected
        };

        result.push(ConnectionWithStatus {
            config,
            status,
        });
    }

    Ok(result)
}

#[derive(Debug, Serialize)]
pub struct ConnectionWithStatus {
    pub config: ConnectionConfig,
    pub status: ConnectionStatus,
}

#[command]
pub async fn save_connection(
    state: State<'_, AppState>,
    config: ConnectionConfig,
    password: Option<String>,
) -> Result<()> {
    config.validate()?;

    // Store password in keyring if provided
    if let Some(pwd) = password {
        if !pwd.is_empty() {
            crate::services::keyring::KeyringService::store_password(
                &config.id.to_string(),
                &pwd,
            )?;
        }
    }

    // Save config to storage
    state.storage.save_connection(&config).await?;

    tracing::info!("Saved connection: {} ({})", config.name, config.id);

    Ok(())
}

#[command]
pub async fn delete_connection(
    state: State<'_, AppState>,
    connection_id: Uuid,
) -> Result<()> {
    // Disconnect if connected
    state.remove_connection(&connection_id).await;

    // Delete credentials
    crate::services::keyring::KeyringService::delete_all_for_connection(
        &connection_id.to_string(),
    )?;

    // Delete from storage
    state.storage.delete_connection(&connection_id).await?;

    tracing::info!("Deleted connection: {}", connection_id);

    Ok(())
}

#[command]
pub async fn get_connection_status(
    state: State<'_, AppState>,
    connection_id: Uuid,
) -> Result<ConnectionStatus> {
    if let Some(pool) = state.get_connection(&connection_id).await {
        Ok(pool.status().await.into())
    } else {
        Ok(ConnectionStatus::Disconnected)
    }
}

// Group management commands
#[command]
pub async fn list_groups(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectionGroup>> {
    state.storage.get_all_groups().await
}

#[command]
pub async fn save_group(
    state: State<'_, AppState>,
    group: ConnectionGroup,
) -> Result<()> {
    state.storage.save_group(&group).await
}

#[command]
pub async fn delete_group(
    state: State<'_, AppState>,
    group_id: Uuid,
) -> Result<()> {
    state.storage.delete_group(&group_id).await
}
```

### 4. Frontend Connection Service

```typescript
// services/connection.ts
import { connectionCommands, credentialCommands } from './ipc';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ConnectionConfig, ConnectionStatus } from '$types/connection';

export interface ConnectionStatusEvent {
	connection_id: string;
	status: ConnectionStatus;
	error?: string;
}

export interface ConnectionInfo {
	id: string;
	server_version: string;
	current_database: string;
	current_user: string;
	backend_pid: number;
}

export interface TestConnectionResult {
	success: boolean;
	version: string;
	started_at: string;
	latency_ms: number;
}

export class ConnectionService {
	private statusListeners: Map<string, ((status: ConnectionStatusEvent) => void)[]> = new Map();
	private globalUnlisten: UnlistenFn | null = null;

	async initialize() {
		this.globalUnlisten = await listen<ConnectionStatusEvent>('connection:status', (event) => {
			const listeners = this.statusListeners.get(event.payload.connection_id);
			if (listeners) {
				listeners.forEach((cb) => cb(event.payload));
			}

			// Also notify global listeners
			const globalListeners = this.statusListeners.get('*');
			if (globalListeners) {
				globalListeners.forEach((cb) => cb(event.payload));
			}
		});
	}

	destroy() {
		if (this.globalUnlisten) {
			this.globalUnlisten();
		}
	}

	onStatusChange(
		connectionId: string | '*',
		callback: (status: ConnectionStatusEvent) => void
	): () => void {
		const listeners = this.statusListeners.get(connectionId) || [];
		listeners.push(callback);
		this.statusListeners.set(connectionId, listeners);

		return () => {
			const current = this.statusListeners.get(connectionId) || [];
			this.statusListeners.set(
				connectionId,
				current.filter((cb) => cb !== callback)
			);
		};
	}

	async connect(config: ConnectionConfig): Promise<ConnectionInfo> {
		return connectionCommands.connect(config);
	}

	async disconnect(connectionId: string): Promise<void> {
		return connectionCommands.disconnect(connectionId);
	}

	async testConnection(config: ConnectionConfig): Promise<TestConnectionResult> {
		return connectionCommands.testConnection(config);
	}

	async listConnections() {
		return connectionCommands.listConnections();
	}

	async saveConnection(config: ConnectionConfig, password?: string): Promise<void> {
		return connectionCommands.saveConnection(config, password);
	}

	async deleteConnection(connectionId: string): Promise<void> {
		return connectionCommands.deleteConnection(connectionId);
	}

	async getStatus(connectionId: string): Promise<ConnectionStatus> {
		return connectionCommands.getConnectionStatus(connectionId);
	}
}

export const connectionService = new ConnectionService();
```

### 5. Connection Store Integration

```typescript
// stores/connections.ts (updated)
import { writable, derived, get } from 'svelte/store';
import { connectionService, type ConnectionStatusEvent } from '$services/connection';
import type { ConnectionConfig, ConnectionStatus } from '$types/connection';

export interface ConnectionState {
	id: string;
	config: ConnectionConfig;
	status: ConnectionStatus;
	serverVersion?: string;
	currentDatabase?: string;
	currentUser?: string;
}

interface ConnectionsStoreState {
	connections: ConnectionState[];
	groups: ConnectionGroup[];
	activeConnectionId: string | null;
	loading: boolean;
	error: string | null;
}

function createConnectionsStore() {
	const { subscribe, update, set } = writable<ConnectionsStoreState>({
		connections: [],
		groups: [],
		activeConnectionId: null,
		loading: false,
		error: null
	});

	// Subscribe to status events
	connectionService.onStatusChange('*', (event: ConnectionStatusEvent) => {
		update((s) => ({
			...s,
			connections: s.connections.map((c) =>
				c.id === event.connection_id ? { ...c, status: event.status } : c
			)
		}));
	});

	return {
		subscribe,

		async load() {
			update((s) => ({ ...s, loading: true, error: null }));

			try {
				const [connections, groups] = await Promise.all([
					connectionService.listConnections(),
					connectionCommands.listGroups()
				]);

				update((s) => ({
					...s,
					connections: connections.map((c) => ({
						id: c.config.id,
						config: c.config,
						status: c.status
					})),
					groups,
					loading: false
				}));
			} catch (error) {
				update((s) => ({
					...s,
					loading: false,
					error: error instanceof Error ? error.message : 'Failed to load connections'
				}));
			}
		},

		async connect(id: string) {
			const state = get({ subscribe });
			const connection = state.connections.find((c) => c.id === id);
			if (!connection) return;

			try {
				const info = await connectionService.connect(connection.config);

				update((s) => ({
					...s,
					activeConnectionId: id,
					connections: s.connections.map((c) =>
						c.id === id
							? {
									...c,
									status: 'connected' as ConnectionStatus,
									serverVersion: info.server_version,
									currentDatabase: info.current_database,
									currentUser: info.current_user
								}
							: c
					)
				}));
			} catch (error) {
				console.error('Connect failed:', error);
				throw error;
			}
		},

		async disconnect(id: string) {
			await connectionService.disconnect(id);

			update((s) => ({
				...s,
				activeConnectionId: s.activeConnectionId === id ? null : s.activeConnectionId
			}));
		},

		async save(config: ConnectionConfig, password?: string) {
			await connectionService.saveConnection(config, password);
			await this.load();
		},

		async delete(id: string) {
			await connectionService.deleteConnection(id);

			update((s) => ({
				...s,
				connections: s.connections.filter((c) => c.id !== id),
				activeConnectionId: s.activeConnectionId === id ? null : s.activeConnectionId
			}));
		},

		setActive(id: string | null) {
			update((s) => ({ ...s, activeConnectionId: id }));
		}
	};
}

export const connectionsStore = createConnectionsStore();

// Derived stores
export const activeConnection = derived(
	connectionsStore,
	($store) => $store.connections.find((c) => c.id === $store.activeConnectionId) || null
);

export const connectedConnections = derived(connectionsStore, ($store) =>
	$store.connections.filter((c) => c.status === 'connected')
);
```

## Acceptance Criteria

1. [ ] Connection pool creates successfully with valid config
2. [ ] Connection validation catches all invalid configurations
3. [ ] Password retrieved from keyring when password_in_keyring is true
4. [ ] Keepalive query runs every 60 seconds
5. [ ] Auto-reconnection works with exponential backoff
6. [ ] Read-only mode prevents write operations
7. [ ] Statement timeout is enforced
8. [ ] Connection status events emitted correctly
9. [ ] Multiple concurrent connections supported
10. [ ] Connection cleanup on disconnect
11. [ ] Pool size limited to 10 connections

## Testing with MCP

```
1. Start app: npm run tauri dev
2. Connect: driver_session action=start
3. Save connection: ipc_execute_command command="save_connection" args={...}
4. Connect: ipc_execute_command command="connect" args={...}
5. Verify connected: ipc_execute_command command="get_connection_status"
6. Monitor keepalive: ipc_monitor action=start, wait 60s
7. Test reconnection: kill postgres, verify auto-reconnect
8. Test disconnect: ipc_execute_command command="disconnect"
```

## Dependencies on Other Features

- 04-ipc-layer.md
- 05-local-storage.md
- 06-settings-theming-credentials.md

## Dependent Features

- 08-ssl-ssh-security.md
- 09-connection-ui.md
- 11-query-execution.md
- All features that require database connectivity
