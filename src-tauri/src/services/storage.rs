use crate::error::{TuskError, TuskResult};
use crate::models::{ConnectionConfig, DatabaseHealth};
use chrono::{DateTime, Local, Utc};
use rusqlite::{params, Connection};
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

/// SQLite-based local storage service for persisting connections and preferences.
pub struct StorageService {
    /// SQLite connection (wrapped in Mutex for thread safety)
    conn: Mutex<Connection>,
    /// Path to the database file
    db_path: PathBuf,
}

impl StorageService {
    /// Create a new StorageService, opening or repairing the database.
    ///
    /// # Arguments
    ///
    /// * `data_dir` - Directory where the database file will be stored
    ///
    /// # Returns
    ///
    /// Returns a new `StorageService` with an initialized database.
    pub fn new(data_dir: &Path) -> TuskResult<Self> {
        let db_path = data_dir.join("tusk.db");
        let conn = Self::open_or_repair(&db_path)?;

        let service = Self {
            conn: Mutex::new(conn),
            db_path,
        };

        service.run_migrations()?;
        Ok(service)
    }

    /// Open an existing database or repair/create if corrupted.
    fn open_or_repair(db_path: &Path) -> TuskResult<Connection> {
        // Try to open existing database
        if db_path.exists() {
            match Connection::open(db_path) {
                Ok(conn) => {
                    // Check integrity
                    let integrity: String = conn
                        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
                        .unwrap_or_else(|_| "error".to_string());

                    if integrity == "ok" {
                        // Set busy timeout to handle concurrent access
                        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
                        return Ok(conn);
                    }

                    tracing::warn!("Database integrity check failed: {}", integrity);

                    // Attempt repair via VACUUM and REINDEX
                    if conn.execute_batch("VACUUM; REINDEX;").is_ok() {
                        let recheck: String = conn
                            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
                            .unwrap_or_else(|_| "error".to_string());

                        if recheck == "ok" {
                            tracing::info!("Database repaired successfully via VACUUM/REINDEX");
                            conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
                            return Ok(conn);
                        }
                    }

                    // Repair failed, need to backup and reset
                    drop(conn);
                    Self::backup_corrupted(db_path)?;
                }
                Err(e) => {
                    tracing::error!("Failed to open database: {}", e);
                    Self::backup_corrupted(db_path)?;
                }
            }
        }

        // Create fresh database
        Self::create_fresh(db_path)
    }

    /// Backup a corrupted database file.
    fn backup_corrupted(db_path: &Path) -> TuskResult<PathBuf> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let backup_path = db_path.with_file_name(format!(
            "{}.{}.corrupt",
            db_path.file_stem().unwrap().to_string_lossy(),
            timestamp
        ));

        std::fs::copy(db_path, &backup_path).map_err(|e| {
            TuskError::storage_with_hint(
                format!("Failed to backup corrupted database: {}", e),
                "Check disk space and permissions",
            )
        })?;

        tracing::warn!(
            "Corrupted database backed up to: {:?}",
            backup_path
        );

        Ok(backup_path)
    }

    /// Create a fresh database with the schema.
    fn create_fresh(db_path: &Path) -> TuskResult<Connection> {
        // Remove existing file if present
        if db_path.exists() {
            std::fs::remove_file(db_path).map_err(|e| {
                TuskError::storage_with_hint(
                    format!("Failed to remove corrupted database: {}", e),
                    "Check file permissions",
                )
            })?;
        }

        let conn = Connection::open(db_path)?;

        // Set pragmas for safety and performance
        conn.execute_batch(
            "
            PRAGMA busy_timeout = 5000;
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            PRAGMA synchronous = NORMAL;
            ",
        )?;

        // Create schema
        conn.execute_batch(
            "
            -- Connections table
            CREATE TABLE IF NOT EXISTS connections (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL UNIQUE,
                host TEXT NOT NULL,
                port INTEGER NOT NULL DEFAULT 5432,
                database TEXT NOT NULL,
                username TEXT NOT NULL,
                ssl_mode TEXT NOT NULL DEFAULT 'prefer',
                ssl_ca_cert TEXT,
                ssh_tunnel_json TEXT,
                read_only INTEGER NOT NULL DEFAULT 0,
                statement_timeout_ms INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_connections_name ON connections(name);

            -- Preferences table
            CREATE TABLE IF NOT EXISTS preferences (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- Migrations table
            CREATE TABLE IF NOT EXISTS migrations (
                version INTEGER PRIMARY KEY NOT NULL,
                applied_at TEXT NOT NULL
            );
            ",
        )?;

        tracing::info!("Created fresh database at: {:?}", db_path);
        Ok(conn)
    }

    /// Run any pending migrations.
    fn run_migrations(&self) -> TuskResult<()> {
        let conn = self.conn.lock().map_err(|e| TuskError::Internal(e.to_string()))?;

        // Get current version
        let current_version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Define migrations
        let migrations: Vec<(i64, &str)> = vec![
            // Migration 1: Initial schema (already created in create_fresh)
            (1, "SELECT 1"), // No-op, schema already exists
        ];

        for (version, sql) in migrations {
            if version > current_version {
                tracing::info!("Running migration {}", version);
                conn.execute_batch(sql)?;
                conn.execute(
                    "INSERT INTO migrations (version, applied_at) VALUES (?, ?)",
                    params![version, Utc::now().to_rfc3339()],
                )?;
            }
        }

        Ok(())
    }

    /// List all saved connection configurations.
    pub fn list_connections(&self) -> TuskResult<Vec<ConnectionConfig>> {
        let conn = self.conn.lock().map_err(|e| TuskError::Internal(e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, host, port, database, username, ssl_mode, ssl_ca_cert,
                    ssh_tunnel_json, read_only, statement_timeout_ms, created_at, updated_at
             FROM connections
             ORDER BY name",
        )?;

        let rows = stmt.query_map([], |row| {
            Self::row_to_connection_config(row)
        })?;

        let mut configs = Vec::new();
        for row in rows {
            configs.push(row?);
        }

        Ok(configs)
    }

    /// Get a single connection configuration by ID.
    pub fn get_connection(&self, id: &Uuid) -> TuskResult<Option<ConnectionConfig>> {
        let conn = self.conn.lock().map_err(|e| TuskError::Internal(e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, host, port, database, username, ssl_mode, ssl_ca_cert,
                    ssh_tunnel_json, read_only, statement_timeout_ms, created_at, updated_at
             FROM connections
             WHERE id = ?",
        )?;

        let result = stmt.query_row([id.to_string()], |row| {
            Self::row_to_connection_config(row)
        });

        match result {
            Ok(config) => Ok(Some(config)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Save a connection configuration (insert or update).
    pub fn save_connection(&self, config: &ConnectionConfig) -> TuskResult<()> {
        let conn = self.conn.lock().map_err(|e| TuskError::Internal(e.to_string()))?;

        let ssh_tunnel_json = config
            .ssh_tunnel
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| TuskError::Internal(e.to_string()))?;

        conn.execute(
            "INSERT INTO connections
             (id, name, host, port, database, username, ssl_mode, ssl_ca_cert,
              ssh_tunnel_json, read_only, statement_timeout_ms, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                host = excluded.host,
                port = excluded.port,
                database = excluded.database,
                username = excluded.username,
                ssl_mode = excluded.ssl_mode,
                ssl_ca_cert = excluded.ssl_ca_cert,
                ssh_tunnel_json = excluded.ssh_tunnel_json,
                read_only = excluded.read_only,
                statement_timeout_ms = excluded.statement_timeout_ms,
                updated_at = excluded.updated_at",
            params![
                config.id.to_string(),
                config.name,
                config.host,
                config.port as i64,
                config.database,
                config.username,
                config.ssl_mode.as_str(),
                config.ssl_ca_cert,
                ssh_tunnel_json,
                config.read_only as i64,
                config.statement_timeout_ms.map(|v| v as i64),
                config.created_at.to_rfc3339(),
                config.updated_at.to_rfc3339(),
            ],
        )?;

        tracing::info!("Saved connection: {} ({})", config.name, config.id);
        Ok(())
    }

    /// Delete a connection configuration.
    pub fn delete_connection(&self, id: &Uuid) -> TuskResult<()> {
        let conn = self.conn.lock().map_err(|e| TuskError::Internal(e.to_string()))?;

        conn.execute("DELETE FROM connections WHERE id = ?", [id.to_string()])?;

        tracing::info!("Deleted connection: {}", id);
        Ok(())
    }

    /// Get a preference value.
    pub fn get_preference(&self, key: &str) -> TuskResult<Option<JsonValue>> {
        let conn = self.conn.lock().map_err(|e| TuskError::Internal(e.to_string()))?;

        let result = conn.query_row(
            "SELECT value FROM preferences WHERE key = ?",
            [key],
            |row| row.get::<_, String>(0),
        );

        match result {
            Ok(value_str) => {
                let value: JsonValue = serde_json::from_str(&value_str)
                    .map_err(|e| TuskError::Internal(e.to_string()))?;
                Ok(Some(value))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Set a preference value.
    pub fn set_preference(&self, key: &str, value: &JsonValue) -> TuskResult<()> {
        let conn = self.conn.lock().map_err(|e| TuskError::Internal(e.to_string()))?;

        let value_str = serde_json::to_string(value)
            .map_err(|e| TuskError::Internal(e.to_string()))?;

        conn.execute(
            "INSERT INTO preferences (key, value, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value_str, Utc::now().to_rfc3339()],
        )?;

        Ok(())
    }

    /// Get the database file path.
    pub fn database_path(&self) -> &std::path::Path {
        &self.db_path
    }

    /// Check database integrity and return health status.
    pub fn check_integrity(&self) -> TuskResult<DatabaseHealth> {
        let conn = self.conn.lock().map_err(|e| TuskError::Internal(e.to_string()))?;

        let mut stmt = conn.prepare("PRAGMA integrity_check")?;
        let results: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        if results.len() == 1 && results[0] == "ok" {
            Ok(DatabaseHealth::healthy())
        } else {
            Ok(DatabaseHealth::unhealthy(results))
        }
    }

    /// Convert a row to a ConnectionConfig.
    fn row_to_connection_config(row: &rusqlite::Row) -> rusqlite::Result<ConnectionConfig> {
        let id_str: String = row.get(0)?;
        let ssl_mode_str: String = row.get(6)?;
        let ssh_tunnel_json: Option<String> = row.get(8)?;
        let created_at_str: String = row.get(11)?;
        let updated_at_str: String = row.get(12)?;

        let id = Uuid::parse_str(&id_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;

        let ssl_mode = match ssl_mode_str.as_str() {
            "disable" => crate::models::SslMode::Disable,
            "prefer" => crate::models::SslMode::Prefer,
            "require" => crate::models::SslMode::Require,
            "verify-ca" => crate::models::SslMode::VerifyCa,
            "verify-full" => crate::models::SslMode::VerifyFull,
            _ => crate::models::SslMode::Prefer,
        };

        let ssh_tunnel = ssh_tunnel_json
            .and_then(|json| serde_json::from_str(&json).ok());

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(ConnectionConfig {
            id,
            name: row.get(1)?,
            host: row.get(2)?,
            port: row.get::<_, i64>(3)? as u16,
            database: row.get(4)?,
            username: row.get(5)?,
            ssl_mode,
            ssl_ca_cert: row.get(7)?,
            ssh_tunnel,
            read_only: row.get::<_, i64>(9)? != 0,
            statement_timeout_ms: row.get::<_, Option<i64>>(10)?.map(|v| v as u64),
            created_at,
            updated_at,
        })
    }
}
