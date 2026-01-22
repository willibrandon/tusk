# Feature 05: Local Storage

## Overview

Implement the local SQLite database for storing connection configurations, query history, saved queries, editor state, and application settings. This is a pure Rust service using rusqlite, accessed directly from GPUI components via the TuskState global.

## Goals

- Define complete SQLite schema per design document
- Implement schema migrations
- Create StorageService with all CRUD operations
- Handle concurrent access safely with synchronous API
- Support data export/import
- Integrate with GPUI's global state system

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

-- Window state persistence
CREATE TABLE IF NOT EXISTS window_state (
    id TEXT PRIMARY KEY DEFAULT 'main',
    x INTEGER,
    y INTEGER,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    maximized INTEGER NOT NULL DEFAULT 0,
    fullscreen INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Insert initial version
INSERT OR IGNORE INTO schema_version (version) VALUES (1);
```

### 2. StorageService Implementation

```rust
// src/services/storage.rs

use parking_lot::RwLock;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{Result, TuskError};
use crate::models::connection::{ConnectionConfig, ConnectionGroup};
use crate::models::settings::Settings;

const CURRENT_SCHEMA_VERSION: i32 = 1;

/// Local SQLite storage for application data.
///
/// Uses synchronous rusqlite wrapped in RwLock for thread-safe access.
/// All methods are synchronous and can be called from any thread.
pub struct StorageService {
    conn: RwLock<Connection>,
}

impl StorageService {
    /// Create or open the storage database.
    ///
    /// # Arguments
    /// * `data_dir` - Application data directory (from directories crate)
    pub fn new(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir).map_err(|e| {
            TuskError::Storage(format!("Failed to create data directory: {}", e))
        })?;

        let db_path = data_dir.join("tusk.db");

        let conn = Connection::open(&db_path).map_err(|e| {
            TuskError::Storage(format!("Failed to open database: {}", e))
        })?;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        // WAL mode for better concurrency
        conn.execute("PRAGMA journal_mode = WAL", [])?;

        // Optimize for desktop application usage
        conn.execute("PRAGMA synchronous = NORMAL", [])?;
        conn.execute("PRAGMA cache_size = -64000", [])?; // 64MB cache

        let service = Self {
            conn: RwLock::new(conn),
        };

        // Run migrations
        service.migrate()?;

        Ok(service)
    }

    /// Open an in-memory database for testing.
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(|e| {
            TuskError::Storage(format!("Failed to open in-memory database: {}", e))
        })?;

        conn.execute("PRAGMA foreign_keys = ON", [])?;

        let service = Self {
            conn: RwLock::new(conn),
        };

        service.migrate()?;
        Ok(service)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.write();

        // Create schema_version table if not exists
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )?;

        // Check current version
        let version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if version < CURRENT_SCHEMA_VERSION {
            tracing::info!(
                "Running migrations from version {} to {}",
                version,
                CURRENT_SCHEMA_VERSION
            );

            // Run initial schema
            if version < 1 {
                conn.execute_batch(include_str!("../../migrations/001_initial.sql"))?;
            }

            // Future migrations:
            // if version < 2 {
            //     conn.execute_batch(include_str!("../../migrations/002_feature.sql"))?;
            // }
        }

        Ok(())
    }

    // ==================== Connections ====================

    /// Get all saved connections.
    pub fn get_all_connections(&self) -> Result<Vec<ConnectionConfig>> {
        let conn = self.conn.read();

        let mut stmt = conn.prepare(
            "SELECT id, config_json, group_id, sort_order, last_connected_at
             FROM connections
             ORDER BY sort_order, name"
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

    /// Get a connection by ID.
    pub fn get_connection(&self, id: Uuid) -> Result<Option<ConnectionConfig>> {
        let conn = self.conn.read();

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

    /// Save or update a connection.
    pub fn save_connection(&self, config: &ConnectionConfig) -> Result<()> {
        let conn = self.conn.write();

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
                config.sort_order.unwrap_or(0),
            ],
        )?;

        Ok(())
    }

    /// Delete a connection.
    pub fn delete_connection(&self, id: Uuid) -> Result<()> {
        let conn = self.conn.write();

        conn.execute(
            "DELETE FROM connections WHERE id = ?",
            params![id.to_string()],
        )?;

        Ok(())
    }

    /// Update the last connected timestamp.
    pub fn touch_connection(&self, id: Uuid) -> Result<()> {
        let conn = self.conn.write();

        conn.execute(
            "UPDATE connections SET last_connected_at = datetime('now') WHERE id = ?",
            params![id.to_string()],
        )?;

        Ok(())
    }

    /// Reorder connections within a group.
    pub fn reorder_connections(&self, ids: &[Uuid]) -> Result<()> {
        let conn = self.conn.write();

        let mut stmt = conn.prepare(
            "UPDATE connections SET sort_order = ? WHERE id = ?"
        )?;

        for (order, id) in ids.iter().enumerate() {
            stmt.execute(params![order as i32, id.to_string()])?;
        }

        Ok(())
    }

    // ==================== Groups ====================

    /// Get all connection groups.
    pub fn get_all_groups(&self) -> Result<Vec<ConnectionGroup>> {
        let conn = self.conn.read();

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

    /// Save or update a group.
    pub fn save_group(&self, group: &ConnectionGroup) -> Result<()> {
        let conn = self.conn.write();

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

    /// Delete a group (cascades to child groups).
    pub fn delete_group(&self, id: Uuid) -> Result<()> {
        let conn = self.conn.write();

        conn.execute(
            "DELETE FROM groups WHERE id = ?",
            params![id.to_string()],
        )?;

        Ok(())
    }

    // ==================== Query History ====================

    /// Add a query to history.
    pub fn add_history(
        &self,
        connection_id: Uuid,
        sql: &str,
        duration_ms: u64,
        rows_affected: Option<i64>,
        error: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.write();

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

        let id = conn.last_insert_rowid();

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

        Ok(id)
    }

    /// Get query history for a connection.
    pub fn get_history(
        &self,
        connection_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<QueryHistoryItem>> {
        let conn = self.conn.read();

        let mut stmt = conn.prepare(
            "SELECT id, sql, executed_at, duration_ms, rows_affected, error, favorited
             FROM query_history
             WHERE connection_id = ?
             ORDER BY executed_at DESC
             LIMIT ? OFFSET ?"
        )?;

        let items = stmt
            .query_map(
                params![connection_id.to_string(), limit, offset],
                |row| {
                    Ok(QueryHistoryItem {
                        id: row.get(0)?,
                        sql: row.get(1)?,
                        executed_at: row.get(2)?,
                        duration_ms: row.get(3)?,
                        rows_affected: row.get(4)?,
                        error: row.get(5)?,
                        favorited: row.get::<_, i32>(6)? != 0,
                    })
                },
            )?
            .filter_map(|r| r.ok())
            .collect();

        Ok(items)
    }

    /// Get favorited history items.
    pub fn get_favorites(&self, connection_id: Uuid) -> Result<Vec<QueryHistoryItem>> {
        let conn = self.conn.read();

        let mut stmt = conn.prepare(
            "SELECT id, sql, executed_at, duration_ms, rows_affected, error, favorited
             FROM query_history
             WHERE connection_id = ? AND favorited = 1
             ORDER BY executed_at DESC"
        )?;

        let items = stmt
            .query_map(params![connection_id.to_string()], |row| {
                Ok(QueryHistoryItem {
                    id: row.get(0)?,
                    sql: row.get(1)?,
                    executed_at: row.get(2)?,
                    duration_ms: row.get(3)?,
                    rows_affected: row.get(4)?,
                    error: row.get(5)?,
                    favorited: true,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(items)
    }

    /// Toggle favorite status and return new state.
    pub fn toggle_favorite(&self, id: i64) -> Result<bool> {
        let conn = self.conn.write();

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

    /// Delete a history item.
    pub fn delete_history(&self, id: i64) -> Result<()> {
        let conn = self.conn.write();

        conn.execute("DELETE FROM query_history WHERE id = ?", params![id])?;

        Ok(())
    }

    /// Clear all history for a connection.
    pub fn clear_history(&self, connection_id: Uuid) -> Result<()> {
        let conn = self.conn.write();

        conn.execute(
            "DELETE FROM query_history WHERE connection_id = ? AND favorited = 0",
            params![connection_id.to_string()],
        )?;

        Ok(())
    }

    /// Search history by SQL text.
    pub fn search_history(
        &self,
        connection_id: Uuid,
        query: &str,
        limit: u32,
    ) -> Result<Vec<QueryHistoryItem>> {
        let conn = self.conn.read();

        let pattern = format!("%{}%", query);

        let mut stmt = conn.prepare(
            "SELECT id, sql, executed_at, duration_ms, rows_affected, error, favorited
             FROM query_history
             WHERE connection_id = ? AND sql LIKE ?
             ORDER BY executed_at DESC
             LIMIT ?"
        )?;

        let items = stmt
            .query_map(params![connection_id.to_string(), pattern, limit], |row| {
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

    // ==================== Saved Queries ====================

    /// Get all saved query folders.
    pub fn get_saved_folders(&self) -> Result<Vec<SavedQueryFolder>> {
        let conn = self.conn.read();

        let mut stmt = conn.prepare(
            "SELECT id, name, parent_id, sort_order
             FROM saved_query_folders
             ORDER BY sort_order, name"
        )?;

        let folders = stmt
            .query_map([], |row| {
                Ok(SavedQueryFolder {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    name: row.get(1)?,
                    parent_id: row.get::<_, Option<String>>(2)?
                        .and_then(|s| Uuid::parse_str(&s).ok()),
                    sort_order: row.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(folders)
    }

    /// Get saved queries, optionally filtered by connection.
    pub fn get_saved_queries(&self, connection_id: Option<Uuid>) -> Result<Vec<SavedQuery>> {
        let conn = self.conn.read();

        let (sql, params_vec): (&str, Vec<String>) = match connection_id {
            Some(cid) => (
                "SELECT id, connection_id, name, description, sql, folder_id, tags, created_at, updated_at
                 FROM saved_queries
                 WHERE connection_id = ? OR connection_id IS NULL
                 ORDER BY name",
                vec![cid.to_string()],
            ),
            None => (
                "SELECT id, connection_id, name, description, sql, folder_id, tags, created_at, updated_at
                 FROM saved_queries
                 ORDER BY name",
                vec![],
            ),
        };

        let mut stmt = conn.prepare(sql)?;

        let queries = if params_vec.is_empty() {
            stmt.query_map([], Self::map_saved_query)?
                .filter_map(|r| r.ok())
                .collect()
        } else {
            stmt.query_map(params![params_vec[0]], Self::map_saved_query)?
                .filter_map(|r| r.ok())
                .collect()
        };

        Ok(queries)
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

    /// Save or update a saved query.
    pub fn save_query(&self, query: &SavedQuery) -> Result<()> {
        let conn = self.conn.write();

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

    /// Save or update a saved query folder.
    pub fn save_folder(&self, folder: &SavedQueryFolder) -> Result<()> {
        let conn = self.conn.write();

        conn.execute(
            "INSERT INTO saved_query_folders (id, name, parent_id, sort_order)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
                name = ?2,
                parent_id = ?3,
                sort_order = ?4",
            params![
                folder.id.to_string(),
                folder.name,
                folder.parent_id.map(|p| p.to_string()),
                folder.sort_order,
            ],
        )?;

        Ok(())
    }

    /// Delete a saved query.
    pub fn delete_saved_query(&self, id: Uuid) -> Result<()> {
        let conn = self.conn.write();

        conn.execute(
            "DELETE FROM saved_queries WHERE id = ?",
            params![id.to_string()],
        )?;

        Ok(())
    }

    /// Delete a saved query folder (cascades).
    pub fn delete_folder(&self, id: Uuid) -> Result<()> {
        let conn = self.conn.write();

        conn.execute(
            "DELETE FROM saved_query_folders WHERE id = ?",
            params![id.to_string()],
        )?;

        Ok(())
    }

    // ==================== Editor State ====================

    /// Get saved editor tab state.
    pub fn get_editor_state(&self) -> Result<Vec<EditorTabState>> {
        let conn = self.conn.read();

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
                    tab_type: row.get::<_, String>(2)?.parse().unwrap_or(TabType::Query),
                    tab_title: row.get(3)?,
                    content: serde_json::from_str(&row.get::<_, String>(4)?).ok(),
                    sort_order: row.get(5)?,
                    is_active: row.get::<_, i32>(6)? != 0,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tabs)
    }

    /// Save editor tab state (replaces all).
    pub fn save_editor_state(&self, tabs: &[EditorTabState]) -> Result<()> {
        let conn = self.conn.write();

        // Clear existing state
        conn.execute("DELETE FROM editor_state", [])?;

        // Insert new state
        let mut stmt = conn.prepare(
            "INSERT INTO editor_state (id, connection_id, tab_type, tab_title, content_json, sort_order, is_active)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )?;

        for tab in tabs {
            let content_json = serde_json::to_string(&tab.content).unwrap_or_else(|_| "{}".into());

            stmt.execute(params![
                tab.id.to_string(),
                tab.connection_id.map(|c| c.to_string()),
                tab.tab_type.as_str(),
                tab.tab_title,
                content_json,
                tab.sort_order,
                tab.is_active as i32,
            ])?;
        }

        Ok(())
    }

    // ==================== Window State ====================

    /// Get window state for restoration.
    pub fn get_window_state(&self) -> Result<Option<WindowState>> {
        let conn = self.conn.read();

        conn.query_row(
            "SELECT x, y, width, height, maximized, fullscreen
             FROM window_state
             WHERE id = 'main'",
            [],
            |row| {
                Ok(WindowState {
                    x: row.get(0)?,
                    y: row.get(1)?,
                    width: row.get(2)?,
                    height: row.get(3)?,
                    maximized: row.get::<_, i32>(4)? != 0,
                    fullscreen: row.get::<_, i32>(5)? != 0,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    /// Save window state.
    pub fn save_window_state(&self, state: &WindowState) -> Result<()> {
        let conn = self.conn.write();

        conn.execute(
            "INSERT INTO window_state (id, x, y, width, height, maximized, fullscreen, updated_at)
             VALUES ('main', ?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                x = ?1, y = ?2, width = ?3, height = ?4,
                maximized = ?5, fullscreen = ?6,
                updated_at = datetime('now')",
            params![
                state.x,
                state.y,
                state.width,
                state.height,
                state.maximized as i32,
                state.fullscreen as i32,
            ],
        )?;

        Ok(())
    }

    // ==================== Settings ====================

    /// Get a single setting by key.
    pub fn get_setting<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let conn = self.conn.read();

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

    /// Save a single setting.
    pub fn save_setting<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let conn = self.conn.write();
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

    /// Load all settings with defaults.
    pub fn get_all_settings(&self) -> Result<Settings> {
        Ok(Settings {
            theme: self.get_setting("theme")?.unwrap_or_default(),
            editor: self.get_setting("editor")?.unwrap_or_default(),
            results: self.get_setting("results")?.unwrap_or_default(),
            query_execution: self.get_setting("query_execution")?.unwrap_or_default(),
            connections: self.get_setting("connections_settings")?.unwrap_or_default(),
        })
    }

    /// Save all settings.
    pub fn save_all_settings(&self, settings: &Settings) -> Result<()> {
        self.save_setting("theme", &settings.theme)?;
        self.save_setting("editor", &settings.editor)?;
        self.save_setting("results", &settings.results)?;
        self.save_setting("query_execution", &settings.query_execution)?;
        self.save_setting("connections_settings", &settings.connections)?;
        Ok(())
    }

    // ==================== Export/Import ====================

    /// Export all data to JSON for backup.
    pub fn export_all(&self) -> Result<StorageExport> {
        Ok(StorageExport {
            version: CURRENT_SCHEMA_VERSION,
            exported_at: chrono::Utc::now().to_rfc3339(),
            connections: self.get_all_connections()?,
            groups: self.get_all_groups()?,
            saved_queries: self.get_saved_queries(None)?,
            saved_folders: self.get_saved_folders()?,
            settings: self.get_all_settings()?,
        })
    }

    /// Import data from backup.
    pub fn import_all(&self, data: &StorageExport) -> Result<ImportResult> {
        let mut result = ImportResult::default();

        // Import groups first (connections depend on them)
        for group in &data.groups {
            if let Err(e) = self.save_group(group) {
                tracing::warn!("Failed to import group {}: {}", group.name, e);
                result.failed_groups += 1;
            } else {
                result.imported_groups += 1;
            }
        }

        // Import connections
        for conn in &data.connections {
            if let Err(e) = self.save_connection(conn) {
                tracing::warn!("Failed to import connection {}: {}", conn.name, e);
                result.failed_connections += 1;
            } else {
                result.imported_connections += 1;
            }
        }

        // Import saved query folders
        for folder in &data.saved_folders {
            if let Err(e) = self.save_folder(folder) {
                tracing::warn!("Failed to import folder {}: {}", folder.name, e);
                result.failed_folders += 1;
            } else {
                result.imported_folders += 1;
            }
        }

        // Import saved queries
        for query in &data.saved_queries {
            if let Err(e) = self.save_query(query) {
                tracing::warn!("Failed to import query {}: {}", query.name, e);
                result.failed_queries += 1;
            } else {
                result.imported_queries += 1;
            }
        }

        // Import settings
        if let Err(e) = self.save_all_settings(&data.settings) {
            tracing::warn!("Failed to import settings: {}", e);
        } else {
            result.imported_settings = true;
        }

        Ok(result)
    }
}

// ==================== Data Types ====================

/// Query history item.
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

/// Saved query.
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

impl SavedQuery {
    pub fn new(name: impl Into<String>, sql: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            connection_id: None,
            name: name.into(),
            description: None,
            sql: sql.into(),
            folder_id: None,
            tags: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Saved query folder.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SavedQueryFolder {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
}

impl SavedQueryFolder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            parent_id: None,
            sort_order: 0,
        }
    }
}

/// Editor tab type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabType {
    Query,
    Table,
    View,
    Function,
}

impl TabType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Query => "query",
            Self::Table => "table",
            Self::View => "view",
            Self::Function => "function",
        }
    }
}

impl std::str::FromStr for TabType {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "query" => Ok(Self::Query),
            "table" => Ok(Self::Table),
            "view" => Ok(Self::View),
            "function" => Ok(Self::Function),
            _ => Err(()),
        }
    }
}

/// Editor tab state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EditorTabState {
    pub id: Uuid,
    pub connection_id: Option<Uuid>,
    pub tab_type: TabType,
    pub tab_title: Option<String>,
    pub content: Option<TabContent>,
    pub sort_order: i32,
    pub is_active: bool,
}

/// Tab-specific content.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum TabContent {
    Query {
        sql: String,
        cursor_line: u32,
        cursor_column: u32,
        selection: Option<TextSelection>,
    },
    Table {
        schema: String,
        table: String,
        filter: Option<String>,
        sort_column: Option<String>,
        sort_direction: Option<String>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextSelection {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

/// Window state for restoration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowState {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
    pub fullscreen: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            width: 1280,
            height: 800,
            maximized: false,
            fullscreen: false,
        }
    }
}

/// Export format for backup.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageExport {
    pub version: i32,
    pub exported_at: String,
    pub connections: Vec<ConnectionConfig>,
    pub groups: Vec<ConnectionGroup>,
    pub saved_queries: Vec<SavedQuery>,
    pub saved_folders: Vec<SavedQueryFolder>,
    pub settings: Settings,
}

/// Import result statistics.
#[derive(Debug, Clone, Default)]
pub struct ImportResult {
    pub imported_connections: usize,
    pub failed_connections: usize,
    pub imported_groups: usize,
    pub failed_groups: usize,
    pub imported_queries: usize,
    pub failed_queries: usize,
    pub imported_folders: usize,
    pub failed_folders: usize,
    pub imported_settings: bool,
}
```

### 3. Settings Model

```rust
// src/models/settings.rs

use serde::{Deserialize, Serialize};

/// Complete application settings.
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
    /// Theme mode: "light", "dark", "system"
    #[serde(default = "default_theme_mode")]
    pub mode: ThemeMode,

    /// Custom accent color (hex)
    pub accent_color: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    Light,
    Dark,
    #[default]
    System,
}

fn default_theme_mode() -> ThemeMode {
    ThemeMode::System
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            mode: ThemeMode::System,
            accent_color: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    #[serde(default = "default_font_family")]
    pub font_family: String,

    #[serde(default = "default_font_size")]
    pub font_size: f32,

    #[serde(default = "default_tab_size")]
    pub tab_size: u32,

    #[serde(default = "default_true")]
    pub use_spaces: bool,

    #[serde(default = "default_true")]
    pub line_numbers: bool,

    #[serde(default = "default_true")]
    pub highlight_current_line: bool,

    #[serde(default = "default_true")]
    pub bracket_matching: bool,

    #[serde(default = "default_true")]
    pub auto_indent: bool,

    #[serde(default)]
    pub word_wrap: bool,

    #[serde(default = "default_autocomplete_delay")]
    pub autocomplete_delay_ms: u32,

    #[serde(default = "default_true")]
    pub show_whitespace: bool,
}

fn default_font_family() -> String {
    "Zed Plex Mono".to_string()
}
fn default_font_size() -> f32 {
    13.0
}
fn default_tab_size() -> u32 {
    2
}
fn default_autocomplete_delay() -> u32 {
    100
}
fn default_true() -> bool {
    true
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_family: default_font_family(),
            font_size: default_font_size(),
            tab_size: default_tab_size(),
            use_spaces: true,
            line_numbers: true,
            highlight_current_line: true,
            bracket_matching: true,
            auto_indent: true,
            word_wrap: false,
            autocomplete_delay_ms: default_autocomplete_delay(),
            show_whitespace: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsSettings {
    #[serde(default = "default_row_limit")]
    pub default_row_limit: u32,

    #[serde(default = "default_batch_size")]
    pub batch_size: u32,

    #[serde(default = "default_date_format")]
    pub date_format: String,

    #[serde(default = "default_timestamp_format")]
    pub timestamp_format: String,

    #[serde(default = "default_null_display")]
    pub null_display: String,

    #[serde(default = "default_truncate_at")]
    pub truncate_text_at: u32,

    #[serde(default = "default_copy_format")]
    pub copy_format: CopyFormat,

    #[serde(default = "default_true")]
    pub copy_headers: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CopyFormat {
    #[default]
    Tsv,
    Csv,
    Json,
    Markdown,
}

fn default_row_limit() -> u32 {
    1000
}
fn default_batch_size() -> u32 {
    100
}
fn default_date_format() -> String {
    "YYYY-MM-DD".to_string()
}
fn default_timestamp_format() -> String {
    "YYYY-MM-DD HH:mm:ss".to_string()
}
fn default_null_display() -> String {
    "NULL".to_string()
}
fn default_truncate_at() -> u32 {
    500
}
fn default_copy_format() -> CopyFormat {
    CopyFormat::Tsv
}

impl Default for ResultsSettings {
    fn default() -> Self {
        Self {
            default_row_limit: default_row_limit(),
            batch_size: default_batch_size(),
            date_format: default_date_format(),
            timestamp_format: default_timestamp_format(),
            null_display: default_null_display(),
            truncate_text_at: default_truncate_at(),
            copy_format: default_copy_format(),
            copy_headers: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExecutionSettings {
    /// Default statement timeout in milliseconds (None = no timeout)
    pub default_timeout_ms: Option<u64>,

    /// Require confirmation for DDL statements
    #[serde(default = "default_true")]
    pub confirm_ddl: bool,

    /// Require confirmation for destructive statements (DROP, TRUNCATE, DELETE without WHERE)
    #[serde(default = "default_true")]
    pub confirm_destructive: bool,

    /// Auto-uppercase SQL keywords
    #[serde(default)]
    pub auto_uppercase_keywords: bool,

    /// Auto-add LIMIT to SELECT without LIMIT
    #[serde(default = "default_true")]
    pub auto_limit_select: bool,

    /// Explain output format
    #[serde(default)]
    pub explain_format: ExplainFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExplainFormat {
    #[default]
    Text,
    Json,
    Yaml,
    Xml,
}

impl Default for QueryExecutionSettings {
    fn default() -> Self {
        Self {
            default_timeout_ms: None,
            confirm_ddl: true,
            confirm_destructive: true,
            auto_uppercase_keywords: false,
            auto_limit_select: true,
            explain_format: ExplainFormat::Text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionsSettings {
    #[serde(default = "default_ssl_mode")]
    pub default_ssl_mode: SslMode,

    #[serde(default = "default_connect_timeout")]
    pub default_connect_timeout_sec: u64,

    #[serde(default = "default_reconnect_attempts")]
    pub auto_reconnect_attempts: u32,

    #[serde(default = "default_keepalive")]
    pub keepalive_interval_sec: u64,

    #[serde(default = "default_true")]
    pub auto_reconnect: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SslMode {
    Disable,
    #[default]
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

fn default_ssl_mode() -> SslMode {
    SslMode::Prefer
}
fn default_connect_timeout() -> u64 {
    10
}
fn default_reconnect_attempts() -> u32 {
    3
}
fn default_keepalive() -> u64 {
    60
}

impl Default for ConnectionsSettings {
    fn default() -> Self {
        Self {
            default_ssl_mode: default_ssl_mode(),
            default_connect_timeout_sec: default_connect_timeout(),
            auto_reconnect_attempts: default_reconnect_attempts(),
            keepalive_interval_sec: default_keepalive(),
            auto_reconnect: true,
        }
    }
}
```

### 4. GPUI Integration

```rust
// src/state.rs - Integration with TuskState global

use std::sync::Arc;
use gpui::Global;

use crate::services::storage::StorageService;

/// Application-wide state accessible via cx.global::<TuskState>()
pub struct TuskState {
    pub storage: Arc<StorageService>,
    // ... other services
}

impl Global for TuskState {}

// Example usage in a component
impl Workspace {
    fn save_tabs(&self, cx: &mut Context<Self>) {
        let state = cx.global::<TuskState>();

        let tab_states: Vec<EditorTabState> = self.panes
            .iter()
            .flat_map(|pane| pane.read(cx).tabs())
            .enumerate()
            .map(|(i, tab)| tab.to_state(i as i32))
            .collect();

        // Storage operations are synchronous
        if let Err(e) = state.storage.save_editor_state(&tab_states) {
            tracing::error!("Failed to save editor state: {}", e);
        }
    }

    fn restore_tabs(&mut self, cx: &mut Context<Self>) {
        let state = cx.global::<TuskState>();

        match state.storage.get_editor_state() {
            Ok(tabs) => {
                for tab_state in tabs {
                    self.restore_tab(tab_state, cx);
                }
            }
            Err(e) => {
                tracing::error!("Failed to restore editor state: {}", e);
            }
        }
    }
}
```

### 5. History Panel Component

```rust
// src/components/history_panel.rs

use gpui::*;
use uuid::Uuid;

use crate::state::TuskState;
use crate::services::storage::QueryHistoryItem;
use crate::theme::Theme;

pub struct HistoryPanel {
    connection_id: Option<Uuid>,
    items: Vec<QueryHistoryItem>,
    search_query: String,
    selected_index: Option<usize>,
    focus_handle: FocusHandle,
}

impl HistoryPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            connection_id: None,
            items: Vec::new(),
            search_query: String::new(),
            selected_index: None,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn set_connection(&mut self, connection_id: Option<Uuid>, cx: &mut Context<Self>) {
        self.connection_id = connection_id;
        self.refresh(cx);
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id else {
            self.items.clear();
            cx.notify();
            return;
        };

        let state = cx.global::<TuskState>();

        let items = if self.search_query.is_empty() {
            state.storage.get_history(connection_id, 100, 0)
        } else {
            state.storage.search_history(connection_id, &self.search_query, 100)
        };

        match items {
            Ok(items) => {
                self.items = items;
                self.selected_index = if self.items.is_empty() { None } else { Some(0) };
            }
            Err(e) => {
                tracing::error!("Failed to load history: {}", e);
                self.items.clear();
            }
        }

        cx.notify();
    }

    fn toggle_favorite(&mut self, id: i64, cx: &mut Context<Self>) {
        let state = cx.global::<TuskState>();

        match state.storage.toggle_favorite(id) {
            Ok(favorited) => {
                // Update local state
                if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
                    item.favorited = favorited;
                }
                cx.notify();
            }
            Err(e) => {
                tracing::error!("Failed to toggle favorite: {}", e);
            }
        }
    }

    fn delete_item(&mut self, id: i64, cx: &mut Context<Self>) {
        let state = cx.global::<TuskState>();

        if let Err(e) = state.storage.delete_history(id) {
            tracing::error!("Failed to delete history item: {}", e);
            return;
        }

        self.items.retain(|i| i.id != id);
        cx.notify();
    }

    fn on_search_change(&mut self, query: String, cx: &mut Context<Self>) {
        self.search_query = query;
        self.refresh(cx);
    }

    fn render_item(
        &self,
        item: &QueryHistoryItem,
        index: usize,
        theme: &Theme,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let is_selected = self.selected_index == Some(index);
        let id = item.id;

        div()
            .id(ElementId::Name(format!("history-{}", id).into()))
            .px_2()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .when(is_selected, |this| this.bg(theme.colors.selection))
            .hover(|this| this.bg(theme.colors.hover))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.colors.text)
                                    .truncate()
                                    .child(truncate_sql(&item.sql, 100))
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .text_xs()
                                    .text_color(theme.colors.text_muted)
                                    .child(format_timestamp(&item.executed_at))
                                    .when_some(item.duration_ms, |this, ms| {
                                        this.child(format!("{}ms", ms))
                                    })
                                    .when_some(item.rows_affected, |this, rows| {
                                        this.child(format!("{} rows", rows))
                                    })
                            )
                    )
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(
                                IconButton::new("favorite", if item.favorited { IconName::StarFilled } else { IconName::Star })
                                    .size(ButtonSize::Small)
                                    .on_click(cx.listener(move |this, _, cx| {
                                        this.toggle_favorite(id, cx);
                                    }))
                            )
                            .child(
                                IconButton::new("delete", IconName::Trash)
                                    .size(ButtonSize::Small)
                                    .on_click(cx.listener(move |this, _, cx| {
                                        this.delete_item(id, cx);
                                    }))
                            )
                    )
            )
            .when(item.error.is_some(), |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(theme.colors.error)
                        .child(item.error.as_ref().unwrap().clone())
                )
            })
    }
}

impl Render for HistoryPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id("history-panel")
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.surface)
            .child(
                // Search bar
                div()
                    .p_2()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        TextInput::new("search")
                            .placeholder("Search history...")
                            .value(self.search_query.clone())
                            .on_change(cx.listener(Self::on_search_change))
                    )
            )
            .child(
                // History list
                div()
                    .flex_1()
                    .overflow_y_auto()
                    .p_1()
                    .children(
                        self.items
                            .iter()
                            .enumerate()
                            .map(|(i, item)| self.render_item(item, i, theme, cx))
                    )
                    .when(self.items.is_empty(), |this| {
                        this.child(
                            div()
                                .p_4()
                                .text_center()
                                .text_color(theme.colors.text_muted)
                                .child("No history")
                        )
                    })
            )
    }
}

fn truncate_sql(sql: &str, max_len: usize) -> String {
    let normalized = sql.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.len() <= max_len {
        normalized
    } else {
        format!("{}...", &normalized[..max_len])
    }
}

fn format_timestamp(ts: &str) -> String {
    // Parse ISO timestamp and format as relative or absolute
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.format("%H:%M:%S").to_string())
        .unwrap_or_else(|_| ts.to_string())
}
```

### 6. Data Directory Resolution

```rust
// src/paths.rs

use directories::ProjectDirs;
use std::path::PathBuf;

/// Get the application data directory.
///
/// - macOS: ~/Library/Application Support/dev.tusk.Tusk
/// - Linux: ~/.local/share/tusk
/// - Windows: %APPDATA%\Tusk\Tusk\data
pub fn data_dir() -> PathBuf {
    ProjectDirs::from("dev", "tusk", "Tusk")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| {
            // Fallback to current directory
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(".tusk")
        })
}

/// Get the configuration directory.
///
/// - macOS: ~/Library/Application Support/dev.tusk.Tusk
/// - Linux: ~/.config/tusk
/// - Windows: %APPDATA%\Tusk\Tusk\config
pub fn config_dir() -> PathBuf {
    ProjectDirs::from("dev", "tusk", "Tusk")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| data_dir())
}

/// Get the cache directory.
///
/// - macOS: ~/Library/Caches/dev.tusk.Tusk
/// - Linux: ~/.cache/tusk
/// - Windows: %LOCALAPPDATA%\Tusk\Tusk\cache
pub fn cache_dir() -> PathBuf {
    ProjectDirs::from("dev", "tusk", "Tusk")
        .map(|dirs| dirs.cache_dir().to_path_buf())
        .unwrap_or_else(|| data_dir().join("cache"))
}

/// Get the log directory.
pub fn log_dir() -> PathBuf {
    data_dir().join("logs")
}

/// Get the migrations directory (embedded in binary).
pub const MIGRATIONS_DIR: &str = "migrations";
```

## Acceptance Criteria

1. [ ] SQLite database created in platform-correct data directory
2. [ ] Migrations run automatically on startup
3. [ ] Connection CRUD operations work
4. [ ] Group CRUD operations work with hierarchy
5. [ ] Query history is saved and retrieved
6. [ ] History is automatically pruned to last 1000 entries per connection
7. [ ] History search works
8. [ ] Favorites toggle works
9. [ ] Saved queries CRUD operations work
10. [ ] Saved query folders work with hierarchy
11. [ ] Editor state save/restore works across sessions
12. [ ] Window state save/restore works
13. [ ] Settings save/load works with defaults
14. [ ] Foreign key constraints enforced
15. [ ] WAL mode enabled
16. [ ] Export/import functionality works
17. [ ] In-memory database for testing works
18. [ ] Thread-safe access via RwLock

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_crud() {
        let storage = StorageService::new_in_memory().unwrap();

        // Create
        let config = ConnectionConfig {
            id: Uuid::new_v4(),
            name: "Test".into(),
            host: "localhost".into(),
            port: 5432,
            database: "test".into(),
            username: "user".into(),
            group_id: None,
            sort_order: None,
            ..Default::default()
        };

        storage.save_connection(&config).unwrap();

        // Read
        let loaded = storage.get_connection(config.id).unwrap().unwrap();
        assert_eq!(loaded.name, "Test");

        // Update
        let mut updated = loaded;
        updated.name = "Updated".into();
        storage.save_connection(&updated).unwrap();

        let reloaded = storage.get_connection(config.id).unwrap().unwrap();
        assert_eq!(reloaded.name, "Updated");

        // Delete
        storage.delete_connection(config.id).unwrap();
        assert!(storage.get_connection(config.id).unwrap().is_none());
    }

    #[test]
    fn test_history_pruning() {
        let storage = StorageService::new_in_memory().unwrap();

        // Create connection first
        let conn_id = Uuid::new_v4();
        let config = ConnectionConfig {
            id: conn_id,
            name: "Test".into(),
            ..Default::default()
        };
        storage.save_connection(&config).unwrap();

        // Add 1100 history items
        for i in 0..1100 {
            storage.add_history(
                conn_id,
                &format!("SELECT {}", i),
                100,
                Some(1),
                None,
            ).unwrap();
        }

        // Should only have 1000
        let history = storage.get_history(conn_id, 2000, 0).unwrap();
        assert_eq!(history.len(), 1000);
    }

    #[test]
    fn test_settings_defaults() {
        let storage = StorageService::new_in_memory().unwrap();

        let settings = storage.get_all_settings().unwrap();

        assert_eq!(settings.theme.mode, ThemeMode::System);
        assert_eq!(settings.editor.font_size, 13.0);
        assert_eq!(settings.results.default_row_limit, 1000);
    }

    #[test]
    fn test_export_import() {
        let storage = StorageService::new_in_memory().unwrap();

        // Add some data
        let config = ConnectionConfig {
            id: Uuid::new_v4(),
            name: "Export Test".into(),
            ..Default::default()
        };
        storage.save_connection(&config).unwrap();

        let query = SavedQuery::new("Test Query", "SELECT 1");
        storage.save_query(&query).unwrap();

        // Export
        let export = storage.export_all().unwrap();
        assert_eq!(export.connections.len(), 1);
        assert_eq!(export.saved_queries.len(), 1);

        // Import into new database
        let storage2 = StorageService::new_in_memory().unwrap();
        let result = storage2.import_all(&export).unwrap();

        assert_eq!(result.imported_connections, 1);
        assert_eq!(result.imported_queries, 1);
    }
}
```

## Dependencies on Other Features

- **02-backend-architecture.md**: Error types, module structure
- **04-service-integration.md**: TuskState global integration

## Dependent Features

- **06-settings-theming-credentials.md**: Settings persistence
- **07-connection-management.md**: Connection storage
- **13-tabs-history.md**: History and editor state storage
