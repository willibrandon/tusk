//! Local SQLite storage for application metadata.
//!
//! Stores saved connections, query history, saved queries, and UI state.
//! Credentials are NOT stored here—they use the OS keychain via CredentialService.
//!
//! # Data Directory Locations
//!
//! - **macOS**: `~/Library/Application Support/dev.tusk.Tusk`
//! - **Windows**: `%APPDATA%\tusk\Tusk`
//! - **Linux**: `~/.local/share/tusk`
//! - **Debug builds**: `./tusk_data` in current directory

use crate::error::TuskError;
use crate::models::{
    ConnectionConfig, ConnectionOptions, QueryHistoryEntry, SshAuthMethod, SshTunnelConfig, SslMode,
};

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use uuid::Uuid;

/// Get the default data directory for the application.
///
/// # Paths by Platform (FR-026)
/// - **macOS**: `~/Library/Application Support/dev.tusk.Tusk`
/// - **Windows**: `%APPDATA%\tusk\Tusk`
/// - **Linux**: `~/.local/share/tusk`
///
/// # Debug Builds (FR-027)
/// Returns `./tusk_data` in the current directory.
pub fn default_data_dir() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        PathBuf::from("./tusk_data")
    }

    #[cfg(not(debug_assertions))]
    {
        dirs::data_dir()
            .map(|d| {
                #[cfg(target_os = "macos")]
                {
                    d.join("dev.tusk.Tusk")
                }
                #[cfg(target_os = "windows")]
                {
                    d.join("tusk").join("Tusk")
                }
                #[cfg(target_os = "linux")]
                {
                    d.join("tusk")
                }
                #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
                {
                    d.join("tusk")
                }
            })
            .unwrap_or_else(|| PathBuf::from("./tusk_data"))
    }
}

/// Initialize the data directory, creating it if needed (FR-025, FR-027a).
///
/// Returns an error with actionable options if the directory cannot be created.
pub fn init_data_dir(path: &PathBuf) -> Result<(), TuskError> {
    if path.exists() {
        if !path.is_dir() {
            return Err(TuskError::storage(
                format!("Data path exists but is not a directory: {}", path.display()),
                Some("Select a different location or remove the existing file"),
            ));
        }
        return Ok(());
    }

    std::fs::create_dir_all(path).map_err(|e| {
        TuskError::storage(
            format!("Failed to create data directory '{}': {}", path.display(), e),
            Some("Check permissions or select a different location"),
        )
    })?;

    tracing::info!(path = %path.display(), "Created data directory");
    Ok(())
}

/// SQLite-based local storage for application data.
///
/// Thread-safe via internal Mutex. Uses WAL mode for concurrent reads.
pub struct LocalStorage {
    /// Thread-safe SQLite connection
    connection: Mutex<Connection>,
    /// Data directory path
    data_dir: PathBuf,
}

impl LocalStorage {
    /// Open or create local storage in the given data directory.
    ///
    /// Creates the directory and database if they don't exist.
    pub fn open(data_dir: PathBuf) -> Result<Self, TuskError> {
        init_data_dir(&data_dir)?;
        let db_path = data_dir.join("tusk.db");
        Self::open_with_path(db_path, data_dir)
    }

    /// Open storage with a specific database path (for testing).
    pub fn open_with_path(db_path: PathBuf, data_dir: PathBuf) -> Result<Self, TuskError> {
        let connection = Connection::open(&db_path).map_err(|e| {
            TuskError::storage(
                format!("Failed to open database '{}': {}", db_path.display(), e),
                Some("The database file may be corrupted. Try deleting it to start fresh."),
            )
        })?;

        // Configure SQLite for optimal performance
        Self::configure_connection(&connection)?;

        let storage = Self { connection: Mutex::new(connection), data_dir };

        // Run migrations
        storage.run_migrations()?;

        tracing::info!(path = %db_path.display(), "Local storage opened");
        Ok(storage)
    }

    /// Configure SQLite connection with optimal pragmas.
    fn configure_connection(conn: &Connection) -> Result<(), TuskError> {
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA busy_timeout = 5000;
            PRAGMA cache_size = -64000;
            PRAGMA foreign_keys = ON;
            PRAGMA temp_store = MEMORY;
            ",
        )
        .map_err(|e| TuskError::storage(format!("Failed to configure database: {e}"), None))
    }

    /// Run database migrations.
    fn run_migrations(&self) -> Result<(), TuskError> {
        let conn = self.connection.lock();

        // Create migrations tracking table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS migrations (
                domain TEXT NOT NULL,
                step INTEGER NOT NULL,
                migration TEXT NOT NULL,
                PRIMARY KEY(domain, step)
            ) STRICT",
            [],
        )
        .map_err(|e| TuskError::storage(format!("Failed to create migrations table: {e}"), None))?;

        // Run schema migrations
        self.migrate_schema(&conn)?;

        Ok(())
    }

    /// Run schema migrations for the main domain.
    fn migrate_schema(&self, conn: &Connection) -> Result<(), TuskError> {
        const DOMAIN: &str = "core";

        // Check current migration level
        let current_step: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(step), 0) FROM migrations WHERE domain = ?",
                [DOMAIN],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Migration 1: Initial schema
        if current_step < 1 {
            conn.execute_batch(
                "
                -- SSH Tunnel Configurations
                CREATE TABLE ssh_tunnels (
                    tunnel_id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    host TEXT NOT NULL,
                    port INTEGER NOT NULL DEFAULT 22,
                    username TEXT NOT NULL,
                    auth_method TEXT NOT NULL,
                    key_path TEXT,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                ) STRICT;

                -- Saved Connections (credentials in OS keychain, NOT here)
                CREATE TABLE connections (
                    connection_id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    host TEXT NOT NULL,
                    port INTEGER NOT NULL DEFAULT 5432,
                    database_name TEXT NOT NULL,
                    username TEXT NOT NULL,
                    ssl_mode TEXT NOT NULL DEFAULT 'prefer',
                    ssh_tunnel_id TEXT,
                    color TEXT,
                    read_only INTEGER NOT NULL DEFAULT 0,
                    connect_timeout_secs INTEGER NOT NULL DEFAULT 10,
                    statement_timeout_secs INTEGER,
                    application_name TEXT NOT NULL DEFAULT 'Tusk',
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    last_connected_at TEXT,
                    FOREIGN KEY(ssh_tunnel_id) REFERENCES ssh_tunnels(tunnel_id) ON DELETE SET NULL
                ) STRICT;

                -- Query History
                CREATE TABLE query_history (
                    history_id INTEGER PRIMARY KEY,
                    connection_id TEXT NOT NULL,
                    sql_text TEXT NOT NULL,
                    execution_time_ms INTEGER,
                    row_count INTEGER,
                    error_message TEXT,
                    executed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(connection_id) REFERENCES connections(connection_id) ON DELETE CASCADE
                ) STRICT;

                -- Saved Queries
                CREATE TABLE saved_queries (
                    query_id TEXT PRIMARY KEY,
                    connection_id TEXT,
                    name TEXT NOT NULL,
                    description TEXT,
                    sql_text TEXT NOT NULL,
                    folder_path TEXT,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(connection_id) REFERENCES connections(connection_id) ON DELETE SET NULL
                ) STRICT;

                -- UI State Persistence
                CREATE TABLE ui_state (
                    key TEXT PRIMARY KEY,
                    value_json TEXT NOT NULL,
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                ) STRICT;

                -- Indexes
                CREATE INDEX idx_connections_last_connected ON connections(last_connected_at DESC);
                CREATE INDEX idx_query_history_connection ON query_history(connection_id, executed_at DESC);
                CREATE INDEX idx_query_history_executed ON query_history(executed_at DESC);
                CREATE INDEX idx_saved_queries_folder ON saved_queries(folder_path);
                ",
            )
            .map_err(|e| TuskError::storage(format!("Migration 1 failed: {e}"), None))?;

            conn.execute(
                "INSERT INTO migrations (domain, step, migration) VALUES (?, 1, 'initial_schema')",
                [DOMAIN],
            )
            .map_err(|e| TuskError::storage(format!("Failed to record migration: {e}"), None))?;

            tracing::info!("Applied migration 1: initial_schema");
        }

        Ok(())
    }

    /// Get the data directory path.
    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    // ========== Connection Operations ==========

    /// Save a connection configuration.
    pub fn save_connection(&self, config: &ConnectionConfig) -> Result<(), TuskError> {
        let conn = self.connection.lock();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO connections (
                connection_id, name, host, port, database_name, username,
                ssl_mode, ssh_tunnel_id, color, read_only,
                connect_timeout_secs, statement_timeout_secs, application_name,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)
            ON CONFLICT(connection_id) DO UPDATE SET
                name = excluded.name,
                host = excluded.host,
                port = excluded.port,
                database_name = excluded.database_name,
                username = excluded.username,
                ssl_mode = excluded.ssl_mode,
                ssh_tunnel_id = excluded.ssh_tunnel_id,
                color = excluded.color,
                read_only = excluded.read_only,
                connect_timeout_secs = excluded.connect_timeout_secs,
                statement_timeout_secs = excluded.statement_timeout_secs,
                application_name = excluded.application_name,
                updated_at = excluded.updated_at",
            params![
                config.id.to_string(),
                config.name,
                config.host,
                config.port,
                config.database,
                config.username,
                config.ssl_mode.as_str(),
                config.ssh_tunnel.as_ref().map(|t| t.id.to_string()),
                config.color,
                config.options.read_only,
                config.options.connect_timeout_secs,
                config.options.statement_timeout_secs,
                config.options.application_name,
                now,
            ],
        )
        .map_err(|e| TuskError::storage(format!("Failed to save connection: {e}"), None))?;

        tracing::debug!(connection_id = %config.id, name = %config.name, "Connection saved");
        Ok(())
    }

    /// Load a connection configuration by ID.
    pub fn load_connection(&self, id: Uuid) -> Result<Option<ConnectionConfig>, TuskError> {
        let conn = self.connection.lock();

        let result = conn
            .query_row(
                "SELECT connection_id, name, host, port, database_name, username,
                        ssl_mode, ssh_tunnel_id, color, read_only,
                        connect_timeout_secs, statement_timeout_secs, application_name
                 FROM connections WHERE connection_id = ?",
                [id.to_string()],
                |row| {
                    Ok(ConnectionConfigRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        host: row.get(2)?,
                        port: row.get(3)?,
                        database: row.get(4)?,
                        username: row.get(5)?,
                        ssl_mode: row.get(6)?,
                        ssh_tunnel_id: row.get(7)?,
                        color: row.get(8)?,
                        read_only: row.get(9)?,
                        connect_timeout_secs: row.get(10)?,
                        statement_timeout_secs: row.get(11)?,
                        application_name: row.get(12)?,
                    })
                },
            )
            .optional()
            .map_err(|e| TuskError::storage(format!("Failed to load connection: {e}"), None))?;

        match result {
            Some(row) => {
                let ssh_tunnel = if let Some(ref tunnel_id) = row.ssh_tunnel_id {
                    let tunnel_uuid = Uuid::parse_str(tunnel_id).map_err(|e| {
                        TuskError::storage(format!("Invalid SSH tunnel ID: {e}"), None)
                    })?;
                    self.load_ssh_tunnel_internal(&conn, tunnel_uuid)?
                } else {
                    None
                };

                Ok(Some(self.row_to_connection_config(row, ssh_tunnel)?))
            }
            None => Ok(None),
        }
    }

    /// Load all saved connections.
    pub fn load_all_connections(&self) -> Result<Vec<ConnectionConfig>, TuskError> {
        let conn = self.connection.lock();

        let mut stmt = conn
            .prepare(
                "SELECT connection_id, name, host, port, database_name, username,
                        ssl_mode, ssh_tunnel_id, color, read_only,
                        connect_timeout_secs, statement_timeout_secs, application_name
                 FROM connections ORDER BY last_connected_at DESC NULLS LAST, name",
            )
            .map_err(|e| TuskError::storage(format!("Failed to prepare query: {e}"), None))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ConnectionConfigRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    host: row.get(2)?,
                    port: row.get(3)?,
                    database: row.get(4)?,
                    username: row.get(5)?,
                    ssl_mode: row.get(6)?,
                    ssh_tunnel_id: row.get(7)?,
                    color: row.get(8)?,
                    read_only: row.get(9)?,
                    connect_timeout_secs: row.get(10)?,
                    statement_timeout_secs: row.get(11)?,
                    application_name: row.get(12)?,
                })
            })
            .map_err(|e| TuskError::storage(format!("Failed to query connections: {e}"), None))?;

        let mut configs = Vec::new();
        for row_result in rows {
            let row = row_result
                .map_err(|e| TuskError::storage(format!("Failed to read row: {e}"), None))?;

            let ssh_tunnel = if let Some(tunnel_id) = &row.ssh_tunnel_id {
                let tunnel_uuid = Uuid::parse_str(tunnel_id)
                    .map_err(|e| TuskError::storage(format!("Invalid SSH tunnel ID: {e}"), None))?;
                self.load_ssh_tunnel_internal(&conn, tunnel_uuid)?
            } else {
                None
            };

            configs.push(self.row_to_connection_config(row, ssh_tunnel)?);
        }

        Ok(configs)
    }

    /// Delete a connection configuration.
    pub fn delete_connection(&self, id: Uuid) -> Result<(), TuskError> {
        let conn = self.connection.lock();

        conn.execute("DELETE FROM connections WHERE connection_id = ?", [id.to_string()])
            .map_err(|e| TuskError::storage(format!("Failed to delete connection: {e}"), None))?;

        tracing::debug!(connection_id = %id, "Connection deleted");
        Ok(())
    }

    /// Update the last connected timestamp.
    pub fn update_last_connected(&self, id: Uuid) -> Result<(), TuskError> {
        let conn = self.connection.lock();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE connections SET last_connected_at = ? WHERE connection_id = ?",
            params![now, id.to_string()],
        )
        .map_err(|e| TuskError::storage(format!("Failed to update last_connected: {e}"), None))?;

        Ok(())
    }

    // ========== SSH Tunnel Operations ==========

    /// Save an SSH tunnel configuration.
    pub fn save_ssh_tunnel(&self, tunnel: &SshTunnelConfig) -> Result<(), TuskError> {
        let conn = self.connection.lock();

        conn.execute(
            "INSERT INTO ssh_tunnels (tunnel_id, name, host, port, username, auth_method, key_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(tunnel_id) DO UPDATE SET
                name = excluded.name,
                host = excluded.host,
                port = excluded.port,
                username = excluded.username,
                auth_method = excluded.auth_method,
                key_path = excluded.key_path",
            params![
                tunnel.id.to_string(),
                tunnel.name,
                tunnel.host,
                tunnel.port,
                tunnel.username,
                tunnel.auth_method.as_str(),
                tunnel.key_path.as_ref().map(|p| p.display().to_string()),
            ],
        )
        .map_err(|e| TuskError::storage(format!("Failed to save SSH tunnel: {e}"), None))?;

        tracing::debug!(tunnel_id = %tunnel.id, name = %tunnel.name, "SSH tunnel saved");
        Ok(())
    }

    /// Load an SSH tunnel configuration by ID.
    pub fn load_ssh_tunnel(&self, id: Uuid) -> Result<Option<SshTunnelConfig>, TuskError> {
        let conn = self.connection.lock();
        self.load_ssh_tunnel_internal(&conn, id)
    }

    fn load_ssh_tunnel_internal(
        &self,
        conn: &Connection,
        id: Uuid,
    ) -> Result<Option<SshTunnelConfig>, TuskError> {
        conn.query_row(
            "SELECT tunnel_id, name, host, port, username, auth_method, key_path
             FROM ssh_tunnels WHERE tunnel_id = ?",
            [id.to_string()],
            |row| {
                let id_str: String = row.get(0)?;
                let auth_method_str: String = row.get(5)?;
                let key_path_str: Option<String> = row.get(6)?;

                Ok(SshTunnelConfig {
                    id: Uuid::parse_str(&id_str).unwrap_or_default(),
                    name: row.get(1)?,
                    host: row.get(2)?,
                    port: row.get(3)?,
                    username: row.get(4)?,
                    auth_method: SshAuthMethod::parse(&auth_method_str),
                    key_path: key_path_str.map(PathBuf::from),
                })
            },
        )
        .optional()
        .map_err(|e| TuskError::storage(format!("Failed to load SSH tunnel: {e}"), None))
    }

    /// Load all SSH tunnel configurations.
    pub fn load_all_ssh_tunnels(&self) -> Result<Vec<SshTunnelConfig>, TuskError> {
        let conn = self.connection.lock();

        let mut stmt = conn
            .prepare(
                "SELECT tunnel_id, name, host, port, username, auth_method, key_path
                 FROM ssh_tunnels ORDER BY name",
            )
            .map_err(|e| TuskError::storage(format!("Failed to prepare query: {e}"), None))?;

        let rows = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let auth_method_str: String = row.get(5)?;
                let key_path_str: Option<String> = row.get(6)?;

                Ok(SshTunnelConfig {
                    id: Uuid::parse_str(&id_str).unwrap_or_default(),
                    name: row.get(1)?,
                    host: row.get(2)?,
                    port: row.get(3)?,
                    username: row.get(4)?,
                    auth_method: SshAuthMethod::parse(&auth_method_str),
                    key_path: key_path_str.map(PathBuf::from),
                })
            })
            .map_err(|e| TuskError::storage(format!("Failed to query SSH tunnels: {e}"), None))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| TuskError::storage(format!("Failed to read SSH tunnels: {e}"), None))
    }

    /// Delete an SSH tunnel configuration.
    pub fn delete_ssh_tunnel(&self, id: Uuid) -> Result<(), TuskError> {
        let conn = self.connection.lock();

        conn.execute("DELETE FROM ssh_tunnels WHERE tunnel_id = ?", [id.to_string()])
            .map_err(|e| TuskError::storage(format!("Failed to delete SSH tunnel: {e}"), None))?;

        tracing::debug!(tunnel_id = %id, "SSH tunnel deleted");
        Ok(())
    }

    // ========== Query History Operations ==========

    /// Add a query to history.
    pub fn add_to_history(&self, entry: &QueryHistoryEntry) -> Result<i64, TuskError> {
        let conn = self.connection.lock();

        conn.execute(
            "INSERT INTO query_history (connection_id, sql_text, execution_time_ms, row_count, error_message, executed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                entry.connection_id.to_string(),
                entry.sql,
                entry.execution_time_ms,
                entry.row_count,
                entry.error_message,
                entry.executed_at.to_rfc3339(),
            ],
        )
        .map_err(|e| TuskError::storage(format!("Failed to add to history: {e}"), None))?;

        let id = conn.last_insert_rowid();
        tracing::trace!(history_id = id, connection_id = %entry.connection_id, "Query added to history");
        Ok(id)
    }

    /// Load recent history for a connection.
    pub fn load_history(
        &self,
        connection_id: Uuid,
        limit: usize,
    ) -> Result<Vec<QueryHistoryEntry>, TuskError> {
        let conn = self.connection.lock();

        let mut stmt = conn
            .prepare(
                "SELECT history_id, connection_id, sql_text, execution_time_ms, row_count, error_message, executed_at
                 FROM query_history
                 WHERE connection_id = ?
                 ORDER BY executed_at DESC
                 LIMIT ?",
            )
            .map_err(|e| TuskError::storage(format!("Failed to prepare query: {e}"), None))?;

        self.query_history_entries(&mut stmt, params![connection_id.to_string(), limit as i64])
    }

    /// Load all recent history across connections.
    pub fn load_all_history(&self, limit: usize) -> Result<Vec<QueryHistoryEntry>, TuskError> {
        let conn = self.connection.lock();

        let mut stmt = conn
            .prepare(
                "SELECT history_id, connection_id, sql_text, execution_time_ms, row_count, error_message, executed_at
                 FROM query_history
                 ORDER BY executed_at DESC
                 LIMIT ?",
            )
            .map_err(|e| TuskError::storage(format!("Failed to prepare query: {e}"), None))?;

        self.query_history_entries(&mut stmt, params![limit as i64])
    }

    /// Search history by SQL content.
    pub fn search_history(
        &self,
        query: &str,
        connection_id: Option<Uuid>,
        limit: usize,
    ) -> Result<Vec<QueryHistoryEntry>, TuskError> {
        let conn = self.connection.lock();
        let search_pattern = format!("%{query}%");

        let mut stmt = if connection_id.is_some() {
            conn.prepare(
                "SELECT history_id, connection_id, sql_text, execution_time_ms, row_count, error_message, executed_at
                 FROM query_history
                 WHERE sql_text LIKE ? AND connection_id = ?
                 ORDER BY executed_at DESC
                 LIMIT ?",
            )
            .map_err(|e| TuskError::storage(format!("Failed to prepare query: {e}"), None))?
        } else {
            conn.prepare(
                "SELECT history_id, connection_id, sql_text, execution_time_ms, row_count, error_message, executed_at
                 FROM query_history
                 WHERE sql_text LIKE ?
                 ORDER BY executed_at DESC
                 LIMIT ?",
            )
            .map_err(|e| TuskError::storage(format!("Failed to prepare query: {e}"), None))?
        };

        if let Some(conn_id) = connection_id {
            self.query_history_entries(
                &mut stmt,
                params![search_pattern, conn_id.to_string(), limit as i64],
            )
        } else {
            self.query_history_entries(&mut stmt, params![search_pattern, limit as i64])
        }
    }

    fn query_history_entries(
        &self,
        stmt: &mut rusqlite::Statement,
        params: impl rusqlite::Params,
    ) -> Result<Vec<QueryHistoryEntry>, TuskError> {
        let rows = stmt
            .query_map(params, |row| {
                let id: i64 = row.get(0)?;
                let connection_id_str: String = row.get(1)?;
                let executed_at_str: String = row.get(6)?;

                Ok(QueryHistoryEntry {
                    id,
                    connection_id: Uuid::parse_str(&connection_id_str).unwrap_or_default(),
                    sql: row.get(2)?,
                    execution_time_ms: row.get(3)?,
                    row_count: row.get(4)?,
                    error_message: row.get(5)?,
                    executed_at: DateTime::parse_from_rfc3339(&executed_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| TuskError::storage(format!("Failed to query history: {e}"), None))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| TuskError::storage(format!("Failed to read history: {e}"), None))
    }

    /// Clear history for a connection.
    pub fn clear_history(&self, connection_id: Uuid) -> Result<(), TuskError> {
        let conn = self.connection.lock();

        conn.execute(
            "DELETE FROM query_history WHERE connection_id = ?",
            [connection_id.to_string()],
        )
        .map_err(|e| TuskError::storage(format!("Failed to clear history: {e}"), None))?;

        tracing::debug!(connection_id = %connection_id, "Query history cleared");
        Ok(())
    }

    /// Clear all history.
    pub fn clear_all_history(&self) -> Result<(), TuskError> {
        let conn = self.connection.lock();

        conn.execute("DELETE FROM query_history", [])
            .map_err(|e| TuskError::storage(format!("Failed to clear all history: {e}"), None))?;

        tracing::debug!("All query history cleared");
        Ok(())
    }

    // ========== Saved Queries Operations ==========

    /// Save a query.
    pub fn save_query(&self, query: &SavedQuery) -> Result<(), TuskError> {
        let conn = self.connection.lock();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO saved_queries (query_id, connection_id, name, description, sql_text, folder_path, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT(query_id) DO UPDATE SET
                connection_id = excluded.connection_id,
                name = excluded.name,
                description = excluded.description,
                sql_text = excluded.sql_text,
                folder_path = excluded.folder_path,
                updated_at = excluded.updated_at",
            params![
                query.id.to_string(),
                query.connection_id.map(|id| id.to_string()),
                query.name,
                query.description,
                query.sql,
                query.folder_path,
                now,
            ],
        )
        .map_err(|e| TuskError::storage(format!("Failed to save query: {e}"), None))?;

        tracing::debug!(query_id = %query.id, name = %query.name, "Query saved");
        Ok(())
    }

    /// Load a saved query by ID.
    pub fn load_saved_query(&self, id: Uuid) -> Result<Option<SavedQuery>, TuskError> {
        let conn = self.connection.lock();

        conn.query_row(
            "SELECT query_id, connection_id, name, description, sql_text, folder_path, created_at, updated_at
             FROM saved_queries WHERE query_id = ?",
            [id.to_string()],
            |row| self.row_to_saved_query(row),
        )
        .optional()
        .map_err(|e| TuskError::storage(format!("Failed to load saved query: {e}"), None))
    }

    /// Load all saved queries.
    pub fn load_all_saved_queries(&self) -> Result<Vec<SavedQuery>, TuskError> {
        let conn = self.connection.lock();

        let mut stmt = conn
            .prepare(
                "SELECT query_id, connection_id, name, description, sql_text, folder_path, created_at, updated_at
                 FROM saved_queries ORDER BY folder_path, name",
            )
            .map_err(|e| TuskError::storage(format!("Failed to prepare query: {e}"), None))?;

        let rows = stmt
            .query_map([], |row| self.row_to_saved_query(row))
            .map_err(|e| TuskError::storage(format!("Failed to query saved queries: {e}"), None))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| TuskError::storage(format!("Failed to read saved queries: {e}"), None))
    }

    /// Load saved queries in a folder.
    pub fn load_saved_queries_in_folder(
        &self,
        folder_path: &str,
    ) -> Result<Vec<SavedQuery>, TuskError> {
        let conn = self.connection.lock();

        let mut stmt = conn
            .prepare(
                "SELECT query_id, connection_id, name, description, sql_text, folder_path, created_at, updated_at
                 FROM saved_queries WHERE folder_path = ? ORDER BY name",
            )
            .map_err(|e| TuskError::storage(format!("Failed to prepare query: {e}"), None))?;

        let rows = stmt
            .query_map([folder_path], |row| self.row_to_saved_query(row))
            .map_err(|e| TuskError::storage(format!("Failed to query saved queries: {e}"), None))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| TuskError::storage(format!("Failed to read saved queries: {e}"), None))
    }

    fn row_to_saved_query(&self, row: &rusqlite::Row) -> rusqlite::Result<SavedQuery> {
        let id_str: String = row.get(0)?;
        let connection_id_str: Option<String> = row.get(1)?;
        let created_at_str: String = row.get(6)?;
        let updated_at_str: String = row.get(7)?;

        Ok(SavedQuery {
            id: Uuid::parse_str(&id_str).unwrap_or_default(),
            connection_id: connection_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
            name: row.get(2)?,
            description: row.get(3)?,
            sql: row.get(4)?,
            folder_path: row.get(5)?,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }

    /// Delete a saved query.
    pub fn delete_saved_query(&self, id: Uuid) -> Result<(), TuskError> {
        let conn = self.connection.lock();

        conn.execute("DELETE FROM saved_queries WHERE query_id = ?", [id.to_string()])
            .map_err(|e| TuskError::storage(format!("Failed to delete saved query: {e}"), None))?;

        tracing::debug!(query_id = %id, "Saved query deleted");
        Ok(())
    }

    // ========== UI State Operations ==========

    /// Save UI state.
    pub fn save_ui_state(&self, key: &str, value: &serde_json::Value) -> Result<(), TuskError> {
        let conn = self.connection.lock();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO ui_state (key, value_json, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET
                value_json = excluded.value_json,
                updated_at = excluded.updated_at",
            params![key, serde_json::to_string(value).unwrap_or_default(), now],
        )
        .map_err(|e| TuskError::storage(format!("Failed to save UI state: {e}"), None))?;

        Ok(())
    }

    /// Load UI state.
    pub fn load_ui_state(&self, key: &str) -> Result<Option<serde_json::Value>, TuskError> {
        let conn = self.connection.lock();

        let result: Option<String> = conn
            .query_row("SELECT value_json FROM ui_state WHERE key = ?", [key], |row| row.get(0))
            .optional()
            .map_err(|e| TuskError::storage(format!("Failed to load UI state: {e}"), None))?;

        match result {
            Some(json_str) => {
                let value = serde_json::from_str(&json_str)
                    .map_err(|e| TuskError::storage(format!("Invalid UI state JSON: {e}"), None))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Delete UI state.
    pub fn delete_ui_state(&self, key: &str) -> Result<(), TuskError> {
        let conn = self.connection.lock();

        conn.execute("DELETE FROM ui_state WHERE key = ?", [key])
            .map_err(|e| TuskError::storage(format!("Failed to delete UI state: {e}"), None))?;

        Ok(())
    }

    // ========== Helper Methods ==========

    fn row_to_connection_config(
        &self,
        row: ConnectionConfigRow,
        ssh_tunnel: Option<SshTunnelConfig>,
    ) -> Result<ConnectionConfig, TuskError> {
        let id = Uuid::parse_str(&row.id)
            .map_err(|e| TuskError::storage(format!("Invalid connection ID: {e}"), None))?;

        Ok(ConnectionConfig {
            id,
            name: row.name,
            host: row.host,
            port: row.port,
            database: row.database,
            username: row.username,
            ssl_mode: SslMode::parse(&row.ssl_mode),
            ssh_tunnel,
            options: ConnectionOptions {
                connect_timeout_secs: row.connect_timeout_secs,
                statement_timeout_secs: row.statement_timeout_secs,
                read_only: row.read_only,
                application_name: row.application_name,
            },
            color: row.color,
        })
    }
}

/// Internal struct for reading connection rows.
struct ConnectionConfigRow {
    id: String,
    name: String,
    host: String,
    port: u16,
    database: String,
    username: String,
    ssl_mode: String,
    ssh_tunnel_id: Option<String>,
    color: Option<String>,
    read_only: bool,
    connect_timeout_secs: u32,
    statement_timeout_secs: Option<u32>,
    application_name: String,
}

/// A saved query in the user's query library.
#[derive(Debug, Clone)]
pub struct SavedQuery {
    /// Unique identifier
    pub id: Uuid,
    /// Associated connection (None = any connection)
    pub connection_id: Option<Uuid>,
    /// Display name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// The SQL text
    pub sql: String,
    /// Folder path (e.g., "/Reports/Monthly")
    pub folder_path: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl SavedQuery {
    /// Create a new saved query.
    pub fn new(name: impl Into<String>, sql: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            connection_id: None,
            name: name.into(),
            description: None,
            sql: sql.into(),
            folder_path: None,
            created_at: now,
            updated_at: now,
        }
    }
}
