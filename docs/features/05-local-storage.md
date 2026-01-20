# Feature 05: Local Storage

## Overview

Implement the local SQLite database for storing connection configurations, query history, saved queries, editor state, and application settings. Includes schema definition, migrations, and CRUD operations.

## Goals

- Define complete SQLite schema per design document
- Implement schema migrations
- Create StorageService with all CRUD operations
- Handle concurrent access safely
- Support data export/import

## Technical Specification

### 1. SQLite Schema

```sql
-- migrations/001_initial.sql

-- Connection groups (folders)
CREATE TABLE IF NOT EXISTS groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    parent_id TEXT REFERENCES groups(id) ON DELETE CASCADE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    color TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_groups_parent ON groups(parent_id);

-- Connection definitions (passwords stored in OS keyring)
CREATE TABLE IF NOT EXISTS connections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    config_json TEXT NOT NULL,
    group_id TEXT REFERENCES groups(id) ON DELETE SET NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_connected_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_connections_group ON connections(group_id);
CREATE INDEX IF NOT EXISTS idx_connections_last_connected ON connections(last_connected_at DESC);

-- Query history per connection
CREATE TABLE IF NOT EXISTS query_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    connection_id TEXT NOT NULL REFERENCES connections(id) ON DELETE CASCADE,
    sql TEXT NOT NULL,
    executed_at TEXT NOT NULL DEFAULT (datetime('now')),
    duration_ms INTEGER,
    rows_affected INTEGER,
    error TEXT,
    favorited INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_history_conn_time ON query_history(connection_id, executed_at DESC);
CREATE INDEX IF NOT EXISTS idx_history_favorited ON query_history(connection_id, favorited) WHERE favorited = 1;

-- Saved query folders
CREATE TABLE IF NOT EXISTS saved_query_folders (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    parent_id TEXT REFERENCES saved_query_folders(id) ON DELETE CASCADE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_saved_folders_parent ON saved_query_folders(parent_id);

-- Saved queries (snippets)
CREATE TABLE IF NOT EXISTS saved_queries (
    id TEXT PRIMARY KEY,
    connection_id TEXT REFERENCES connections(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    description TEXT,
    sql TEXT NOT NULL,
    folder_id TEXT REFERENCES saved_query_folders(id) ON DELETE SET NULL,
    tags TEXT, -- JSON array
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_saved_queries_conn ON saved_queries(connection_id);
CREATE INDEX IF NOT EXISTS idx_saved_queries_folder ON saved_queries(folder_id);

-- Editor tabs state (restore on reopen)
CREATE TABLE IF NOT EXISTS editor_state (
    id TEXT PRIMARY KEY,
    connection_id TEXT REFERENCES connections(id) ON DELETE SET NULL,
    tab_type TEXT NOT NULL CHECK (tab_type IN ('query', 'table', 'view', 'function')),
    tab_title TEXT,
    content_json TEXT NOT NULL, -- Tab-specific state
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_editor_state_conn ON editor_state(connection_id);

-- Application settings
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Insert initial version
INSERT OR IGNORE INTO schema_version (version) VALUES (1);
```

### 2. StorageService Implementation

```rust
// services/storage.rs
use rusqlite::{Connection, params, OptionalExtension};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::error::{Result, TuskError};
use crate::models::connection::{ConnectionConfig, ConnectionGroup};
use crate::models::settings::Settings;

const CURRENT_SCHEMA_VERSION: i32 = 1;

pub struct StorageService {
    conn: Arc<Mutex<Connection>>,
}

impl StorageService {
    pub async fn new(data_dir: &Path) -> Result<Self> {
        let db_path = data_dir.join("tusk.db");

        let conn = Connection::open(&db_path).map_err(|e| {
            TuskError::StorageError(format!("Failed to open database: {}", e))
        })?;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        // WAL mode for better concurrency
        conn.execute("PRAGMA journal_mode = WAL", [])?;

        let service = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        // Run migrations
        service.migrate().await?;

        Ok(service)
    }

    async fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().await;

        // Check current version
        let version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if version < CURRENT_SCHEMA_VERSION {
            tracing::info!("Running migrations from version {} to {}", version, CURRENT_SCHEMA_VERSION);

            // Run initial schema
            if version < 1 {
                conn.execute_batch(include_str!("../../migrations/001_initial.sql"))?;
            }

            // Add more migrations here as needed
            // if version < 2 { ... }
        }

        Ok(())
    }

    // ==================== Connections ====================

    pub async fn get_all_connections(&self) -> Result<Vec<ConnectionConfig>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT id, config_json FROM connections ORDER BY sort_order, name"
        )?;

        let configs = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let json: String = row.get(1)?;
                Ok((id, json))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(id, json)| {
                serde_json::from_str::<ConnectionConfig>(&json)
                    .map_err(|e| tracing::warn!("Failed to parse connection {}: {}", id, e))
                    .ok()
            })
            .collect();

        Ok(configs)
    }

    pub async fn get_connection(&self, id: &Uuid) -> Result<Option<ConnectionConfig>> {
        let conn = self.conn.lock().await;

        let json: Option<String> = conn
            .query_row(
                "SELECT config_json FROM connections WHERE id = ?",
                params![id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        match json {
            Some(j) => Ok(Some(serde_json::from_str(&j)?)),
            None => Ok(None),
        }
    }

    pub async fn save_connection(&self, config: &ConnectionConfig) -> Result<()> {
        let conn = self.conn.lock().await;

        let json = serde_json::to_string(config)?;

        conn.execute(
            "INSERT INTO connections (id, name, config_json, group_id, sort_order, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                name = ?2,
                config_json = ?3,
                group_id = ?4,
                sort_order = ?5,
                updated_at = datetime('now')",
            params![
                config.id.to_string(),
                config.name,
                json,
                config.group_id.map(|g| g.to_string()),
                0, // TODO: handle sort_order
            ],
        )?;

        Ok(())
    }

    pub async fn delete_connection(&self, id: &Uuid) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM connections WHERE id = ?",
            params![id.to_string()],
        )?;

        Ok(())
    }

    pub async fn update_connection_last_used(&self, id: &Uuid) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE connections SET last_connected_at = datetime('now') WHERE id = ?",
            params![id.to_string()],
        )?;

        Ok(())
    }

    // ==================== Groups ====================

    pub async fn get_all_groups(&self) -> Result<Vec<ConnectionGroup>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT id, name, parent_id, sort_order, color
             FROM groups
             ORDER BY sort_order, name"
        )?;

        let groups = stmt
            .query_map([], |row| {
                Ok(ConnectionGroup {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    name: row.get(1)?,
                    parent_id: row.get::<_, Option<String>>(2)?
                        .and_then(|s| Uuid::parse_str(&s).ok()),
                    sort_order: row.get(3)?,
                    color: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(groups)
    }

    pub async fn save_group(&self, group: &ConnectionGroup) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO groups (id, name, parent_id, sort_order, color, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                name = ?2,
                parent_id = ?3,
                sort_order = ?4,
                color = ?5,
                updated_at = datetime('now')",
            params![
                group.id.to_string(),
                group.name,
                group.parent_id.map(|g| g.to_string()),
                group.sort_order,
                group.color,
            ],
        )?;

        Ok(())
    }

    pub async fn delete_group(&self, id: &Uuid) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM groups WHERE id = ?",
            params![id.to_string()],
        )?;

        Ok(())
    }

    // ==================== Query History ====================

    pub async fn add_query_history(
        &self,
        connection_id: &Uuid,
        sql: &str,
        duration_ms: u64,
        rows_affected: Option<i64>,
        error: Option<String>,
    ) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO query_history (connection_id, sql, duration_ms, rows_affected, error)
             VALUES (?, ?, ?, ?, ?)",
            params![
                connection_id.to_string(),
                sql,
                duration_ms as i64,
                rows_affected,
                error,
            ],
        )?;

        // Prune old history (keep last 1000 per connection)
        conn.execute(
            "DELETE FROM query_history
             WHERE connection_id = ?
               AND id NOT IN (
                 SELECT id FROM query_history
                 WHERE connection_id = ?
                 ORDER BY executed_at DESC
                 LIMIT 1000
               )",
            params![connection_id.to_string(), connection_id.to_string()],
        )?;

        Ok(())
    }

    pub async fn get_query_history(
        &self,
        connection_id: &Uuid,
        limit: Option<u32>,
    ) -> Result<Vec<QueryHistoryItem>> {
        let conn = self.conn.lock().await;
        let limit = limit.unwrap_or(100);

        let mut stmt = conn.prepare(
            "SELECT id, sql, executed_at, duration_ms, rows_affected, error, favorited
             FROM query_history
             WHERE connection_id = ?
             ORDER BY executed_at DESC
             LIMIT ?"
        )?;

        let items = stmt
            .query_map(params![connection_id.to_string(), limit], |row| {
                Ok(QueryHistoryItem {
                    id: row.get(0)?,
                    sql: row.get(1)?,
                    executed_at: row.get(2)?,
                    duration_ms: row.get(3)?,
                    rows_affected: row.get(4)?,
                    error: row.get(5)?,
                    favorited: row.get::<_, i32>(6)? != 0,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(items)
    }

    pub async fn toggle_history_favorite(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE query_history SET favorited = NOT favorited WHERE id = ?",
            params![id],
        )?;

        let favorited: bool = conn.query_row(
            "SELECT favorited FROM query_history WHERE id = ?",
            params![id],
            |row| Ok(row.get::<_, i32>(0)? != 0),
        )?;

        Ok(favorited)
    }

    // ==================== Saved Queries ====================

    pub async fn get_saved_queries(&self, connection_id: Option<&Uuid>) -> Result<Vec<SavedQuery>> {
        let conn = self.conn.lock().await;

        let sql = match connection_id {
            Some(_) => "SELECT id, connection_id, name, description, sql, folder_id, tags, created_at, updated_at
                       FROM saved_queries
                       WHERE connection_id = ? OR connection_id IS NULL
                       ORDER BY name",
            None => "SELECT id, connection_id, name, description, sql, folder_id, tags, created_at, updated_at
                    FROM saved_queries
                    ORDER BY name",
        };

        let mut stmt = conn.prepare(sql)?;

        let rows = if let Some(cid) = connection_id {
            stmt.query_map(params![cid.to_string()], map_saved_query)?
        } else {
            stmt.query_map([], map_saved_query)?
        };

        let queries = rows.filter_map(|r| r.ok()).collect();
        Ok(queries)
    }

    pub async fn save_query(&self, query: &SavedQuery) -> Result<()> {
        let conn = self.conn.lock().await;

        let tags_json = serde_json::to_string(&query.tags)?;

        conn.execute(
            "INSERT INTO saved_queries (id, connection_id, name, description, sql, folder_id, tags, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                name = ?3,
                description = ?4,
                sql = ?5,
                folder_id = ?6,
                tags = ?7,
                updated_at = datetime('now')",
            params![
                query.id.to_string(),
                query.connection_id.map(|c| c.to_string()),
                query.name,
                query.description,
                query.sql,
                query.folder_id.map(|f| f.to_string()),
                tags_json,
            ],
        )?;

        Ok(())
    }

    pub async fn delete_saved_query(&self, id: &Uuid) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM saved_queries WHERE id = ?",
            params![id.to_string()],
        )?;

        Ok(())
    }

    // ==================== Editor State ====================

    pub async fn get_editor_state(&self) -> Result<Vec<EditorTabState>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT id, connection_id, tab_type, tab_title, content_json, sort_order, is_active
             FROM editor_state
             ORDER BY sort_order"
        )?;

        let tabs = stmt
            .query_map([], |row| {
                Ok(EditorTabState {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    connection_id: row.get::<_, Option<String>>(1)?
                        .and_then(|s| Uuid::parse_str(&s).ok()),
                    tab_type: row.get(2)?,
                    tab_title: row.get(3)?,
                    content_json: row.get(4)?,
                    sort_order: row.get(5)?,
                    is_active: row.get::<_, i32>(6)? != 0,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tabs)
    }

    pub async fn save_editor_state(&self, tabs: &[EditorTabState]) -> Result<()> {
        let conn = self.conn.lock().await;

        // Clear existing state
        conn.execute("DELETE FROM editor_state", [])?;

        // Insert new state
        let mut stmt = conn.prepare(
            "INSERT INTO editor_state (id, connection_id, tab_type, tab_title, content_json, sort_order, is_active)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )?;

        for tab in tabs {
            stmt.execute(params![
                tab.id.to_string(),
                tab.connection_id.map(|c| c.to_string()),
                tab.tab_type,
                tab.tab_title,
                tab.content_json,
                tab.sort_order,
                tab.is_active as i32,
            ])?;
        }

        Ok(())
    }

    // ==================== Settings ====================

    pub async fn get_setting<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let conn = self.conn.lock().await;

        let json: Option<String> = conn
            .query_row(
                "SELECT value_json FROM settings WHERE key = ?",
                params![key],
                |row| row.get(0),
            )
            .optional()?;

        match json {
            Some(j) => Ok(Some(serde_json::from_str(&j)?)),
            None => Ok(None),
        }
    }

    pub async fn save_setting<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let conn = self.conn.lock().await;
        let json = serde_json::to_string(value)?;

        conn.execute(
            "INSERT INTO settings (key, value_json, updated_at)
             VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET
                value_json = ?2,
                updated_at = datetime('now')",
            params![key, json],
        )?;

        Ok(())
    }

    pub async fn get_all_settings(&self) -> Result<Settings> {
        // Load all settings with defaults
        let theme = self.get_setting("theme").await?.unwrap_or_default();
        let editor = self.get_setting("editor").await?.unwrap_or_default();
        let results = self.get_setting("results").await?.unwrap_or_default();
        let query_execution = self.get_setting("query_execution").await?.unwrap_or_default();
        let connections = self.get_setting("connections_settings").await?.unwrap_or_default();

        Ok(Settings {
            theme,
            editor,
            results,
            query_execution,
            connections,
        })
    }

    pub async fn save_all_settings(&self, settings: &Settings) -> Result<()> {
        self.save_setting("theme", &settings.theme).await?;
        self.save_setting("editor", &settings.editor).await?;
        self.save_setting("results", &settings.results).await?;
        self.save_setting("query_execution", &settings.query_execution).await?;
        self.save_setting("connections_settings", &settings.connections).await?;
        Ok(())
    }
}

// Helper structs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueryHistoryItem {
    pub id: i64,
    pub sql: String,
    pub executed_at: String,
    pub duration_ms: Option<i64>,
    pub rows_affected: Option<i64>,
    pub error: Option<String>,
    pub favorited: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SavedQuery {
    pub id: Uuid,
    pub connection_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub sql: String,
    pub folder_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EditorTabState {
    pub id: Uuid,
    pub connection_id: Option<Uuid>,
    pub tab_type: String,
    pub tab_title: Option<String>,
    pub content_json: String,
    pub sort_order: i32,
    pub is_active: bool,
}

fn map_saved_query(row: &rusqlite::Row) -> rusqlite::Result<SavedQuery> {
    let tags_json: String = row.get(6)?;
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

    Ok(SavedQuery {
        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
        connection_id: row.get::<_, Option<String>>(1)?
            .and_then(|s| Uuid::parse_str(&s).ok()),
        name: row.get(2)?,
        description: row.get(3)?,
        sql: row.get(4)?,
        folder_id: row.get::<_, Option<String>>(5)?
            .and_then(|s| Uuid::parse_str(&s).ok()),
        tags,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}
```

### 3. Settings Model

```rust
// models/settings.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    pub theme: ThemeSettings,
    pub editor: EditorSettings,
    pub results: ResultsSettings,
    pub query_execution: QueryExecutionSettings,
    pub connections: ConnectionsSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSettings {
    #[serde(default = "default_theme")]
    pub mode: String, // "light", "dark", "system"
}

fn default_theme() -> String { "system".to_string() }

impl Default for ThemeSettings {
    fn default() -> Self {
        Self { mode: default_theme() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: u32,
    #[serde(default = "default_tab_size")]
    pub tab_size: u32,
    #[serde(default)]
    pub use_spaces: bool,
    #[serde(default = "default_true")]
    pub line_numbers: bool,
    #[serde(default)]
    pub minimap: bool,
    #[serde(default)]
    pub word_wrap: bool,
    #[serde(default = "default_autocomplete_delay")]
    pub autocomplete_delay_ms: u32,
    #[serde(default = "default_true")]
    pub bracket_matching: bool,
}

fn default_font_family() -> String { "JetBrains Mono".to_string() }
fn default_font_size() -> u32 { 13 }
fn default_tab_size() -> u32 { 2 }
fn default_autocomplete_delay() -> u32 { 100 }
fn default_true() -> bool { true }

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_family: default_font_family(),
            font_size: default_font_size(),
            tab_size: default_tab_size(),
            use_spaces: true,
            line_numbers: true,
            minimap: false,
            word_wrap: false,
            autocomplete_delay_ms: default_autocomplete_delay(),
            bracket_matching: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsSettings {
    #[serde(default = "default_row_limit")]
    pub default_row_limit: u32,
    #[serde(default = "default_date_format")]
    pub date_format: String,
    #[serde(default = "default_null_display")]
    pub null_display: String,
    #[serde(default = "default_truncate_at")]
    pub truncate_text_at: u32,
    #[serde(default = "default_copy_format")]
    pub copy_format: String, // "tsv", "csv", "json"
}

fn default_row_limit() -> u32 { 1000 }
fn default_date_format() -> String { "YYYY-MM-DD HH:mm:ss".to_string() }
fn default_null_display() -> String { "NULL".to_string() }
fn default_truncate_at() -> u32 { 500 }
fn default_copy_format() -> String { "tsv".to_string() }

impl Default for ResultsSettings {
    fn default() -> Self {
        Self {
            default_row_limit: default_row_limit(),
            date_format: default_date_format(),
            null_display: default_null_display(),
            truncate_text_at: default_truncate_at(),
            copy_format: default_copy_format(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExecutionSettings {
    pub default_statement_timeout_ms: Option<u64>,
    #[serde(default = "default_true")]
    pub confirm_ddl: bool,
    #[serde(default = "default_true")]
    pub confirm_destructive: bool,
    #[serde(default)]
    pub auto_uppercase_keywords: bool,
    #[serde(default = "default_true")]
    pub auto_limit_select: bool,
}

impl Default for QueryExecutionSettings {
    fn default() -> Self {
        Self {
            default_statement_timeout_ms: None,
            confirm_ddl: true,
            confirm_destructive: true,
            auto_uppercase_keywords: false,
            auto_limit_select: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionsSettings {
    #[serde(default = "default_ssl_mode")]
    pub default_ssl_mode: String,
    #[serde(default = "default_connect_timeout")]
    pub default_connect_timeout_sec: u64,
    #[serde(default = "default_reconnect_attempts")]
    pub auto_reconnect_attempts: u32,
    #[serde(default = "default_keepalive")]
    pub keepalive_interval_sec: u64,
}

fn default_ssl_mode() -> String { "prefer".to_string() }
fn default_connect_timeout() -> u64 { 10 }
fn default_reconnect_attempts() -> u32 { 3 }
fn default_keepalive() -> u64 { 60 }

impl Default for ConnectionsSettings {
    fn default() -> Self {
        Self {
            default_ssl_mode: default_ssl_mode(),
            default_connect_timeout_sec: default_connect_timeout(),
            auto_reconnect_attempts: default_reconnect_attempts(),
            keepalive_interval_sec: default_keepalive(),
        }
    }
}
```

### 4. Storage Commands

```rust
// commands/storage.rs
use tauri::{command, State};
use uuid::Uuid;

use crate::state::AppState;
use crate::error::Result;
use crate::services::storage::{QueryHistoryItem, SavedQuery, EditorTabState};
use crate::models::settings::Settings;

#[command]
pub async fn get_query_history(
    state: State<'_, AppState>,
    connection_id: Uuid,
    limit: Option<u32>,
) -> Result<Vec<QueryHistoryItem>> {
    state.storage.get_query_history(&connection_id, limit).await
}

#[command]
pub async fn toggle_history_favorite(
    state: State<'_, AppState>,
    id: i64,
) -> Result<bool> {
    state.storage.toggle_history_favorite(id).await
}

#[command]
pub async fn get_saved_queries(
    state: State<'_, AppState>,
    connection_id: Option<Uuid>,
) -> Result<Vec<SavedQuery>> {
    state.storage.get_saved_queries(connection_id.as_ref()).await
}

#[command]
pub async fn save_query(
    state: State<'_, AppState>,
    query: SavedQuery,
) -> Result<()> {
    state.storage.save_query(&query).await
}

#[command]
pub async fn delete_saved_query(
    state: State<'_, AppState>,
    query_id: Uuid,
) -> Result<()> {
    state.storage.delete_saved_query(&query_id).await
}

#[command]
pub async fn get_editor_state(
    state: State<'_, AppState>,
) -> Result<Vec<EditorTabState>> {
    state.storage.get_editor_state().await
}

#[command]
pub async fn save_editor_state(
    state: State<'_, AppState>,
    tabs: Vec<EditorTabState>,
) -> Result<()> {
    state.storage.save_editor_state(&tabs).await
}

#[command]
pub async fn get_settings(
    state: State<'_, AppState>,
) -> Result<Settings> {
    state.storage.get_all_settings().await
}

#[command]
pub async fn save_settings(
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<()> {
    state.storage.save_all_settings(&settings).await
}
```

## Acceptance Criteria

1. [ ] SQLite database created in correct location
2. [ ] Migrations run automatically on startup
3. [ ] Connection CRUD operations work
4. [ ] Group CRUD operations work
5. [ ] Query history is saved and retrieved
6. [ ] History is automatically pruned to last 1000 entries
7. [ ] Saved queries CRUD operations work
8. [ ] Editor state save/restore works
9. [ ] Settings save/load works with defaults
10. [ ] Foreign key constraints enforced
11. [ ] WAL mode enabled for better concurrency

## Testing with MCP

```
1. Start app: npm run tauri dev
2. Connect: driver_session action=start
3. Save connection: ipc_execute_command command="save_connection" args={...}
4. List connections: ipc_execute_command command="list_connections"
5. Execute query to generate history
6. Get history: ipc_execute_command command="get_query_history"
7. Save settings: ipc_execute_command command="save_settings" args={...}
8. Restart app and verify data persisted
```

## Dependencies on Other Features

- 02-backend-architecture.md
- 04-ipc-layer.md

## Dependent Features

- 06-settings-theming-credentials.md
- 07-connection-management.md
- 13-tabs-history.md
