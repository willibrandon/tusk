# Storage API Contract

**Feature**: 005-local-storage
**Date**: 2026-01-22

This document defines the Rust API contract for the LocalStorage service extensions.

## Module: `tusk_core::services::storage`

### Struct: LocalStorage

Existing struct extended with new methods.

---

## Connection Group Operations

### `save_connection_group`

```rust
/// Save or update a connection group.
///
/// # Arguments
/// * `group` - The connection group to save
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(TuskError)` if database operation fails
///
/// # Example
/// ```rust
/// let group = ConnectionGroup::new("Production", Some("#FF5733"));
/// storage.save_connection_group(&group)?;
/// ```
pub fn save_connection_group(&self, group: &ConnectionGroup) -> Result<(), TuskError>;
```

### `load_connection_group`

```rust
/// Load a connection group by ID.
///
/// # Arguments
/// * `id` - The group's UUID
///
/// # Returns
/// * `Ok(Some(group))` if found
/// * `Ok(None)` if not found
/// * `Err(TuskError)` if database operation fails
pub fn load_connection_group(&self, id: Uuid) -> Result<Option<ConnectionGroup>, TuskError>;
```

### `load_all_connection_groups`

```rust
/// Load all connection groups ordered by sort_order.
///
/// # Returns
/// * `Ok(Vec<ConnectionGroup>)` - Groups in display order
/// * `Err(TuskError)` if database operation fails
pub fn load_all_connection_groups(&self) -> Result<Vec<ConnectionGroup>, TuskError>;
```

### `delete_connection_group`

```rust
/// Delete a connection group.
///
/// Connections in this group become ungrouped (group_id = NULL).
///
/// # Arguments
/// * `id` - The group's UUID
///
/// # Returns
/// * `Ok(())` on success (idempotent)
/// * `Err(TuskError)` if database operation fails
pub fn delete_connection_group(&self, id: Uuid) -> Result<(), TuskError>;
```

### `reorder_connection_groups`

```rust
/// Update sort order for multiple groups.
///
/// # Arguments
/// * `order` - Vec of (group_id, sort_order) pairs
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(TuskError)` if database operation fails
pub fn reorder_connection_groups(&self, order: &[(Uuid, i32)]) -> Result<(), TuskError>;
```

---

## Query Folder Operations

### `save_query_folder`

```rust
/// Save or update a query folder.
///
/// # Arguments
/// * `folder` - The folder to save
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(TuskError)` if:
///   - Database operation fails
///   - Circular reference detected (parent is descendant)
///   - Max depth exceeded (5 levels)
pub fn save_query_folder(&self, folder: &QueryFolder) -> Result<(), TuskError>;
```

### `load_query_folder`

```rust
/// Load a query folder by ID.
///
/// # Arguments
/// * `id` - The folder's UUID
///
/// # Returns
/// * `Ok(Some(folder))` if found
/// * `Ok(None)` if not found
/// * `Err(TuskError)` if database operation fails
pub fn load_query_folder(&self, id: Uuid) -> Result<Option<QueryFolder>, TuskError>;
```

### `load_query_folders`

```rust
/// Load all query folders with optional parent filter.
///
/// # Arguments
/// * `parent_id` - Filter by parent (None = root folders only)
///
/// # Returns
/// * `Ok(Vec<QueryFolder>)` - Folders in display order
/// * `Err(TuskError)` if database operation fails
pub fn load_query_folders(&self, parent_id: Option<Uuid>) -> Result<Vec<QueryFolder>, TuskError>;
```

### `load_all_query_folders`

```rust
/// Load all query folders regardless of parent.
///
/// # Returns
/// * `Ok(Vec<QueryFolder>)` - All folders
/// * `Err(TuskError)` if database operation fails
pub fn load_all_query_folders(&self) -> Result<Vec<QueryFolder>, TuskError>;
```

### `delete_query_folder`

```rust
/// Delete a query folder and all descendants.
///
/// Queries in deleted folders have folder_id set to NULL.
///
/// # Arguments
/// * `id` - The folder's UUID
///
/// # Returns
/// * `Ok(())` on success (idempotent)
/// * `Err(TuskError)` if database operation fails
pub fn delete_query_folder(&self, id: Uuid) -> Result<(), TuskError>;
```

### `move_query_to_folder`

```rust
/// Move a saved query to a folder.
///
/// # Arguments
/// * `query_id` - The query's UUID
/// * `folder_id` - Target folder (None = root/unfiled)
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(TuskError)` if database operation fails
pub fn move_query_to_folder(&self, query_id: Uuid, folder_id: Option<Uuid>) -> Result<(), TuskError>;
```

---

## Application Settings Operations

### `load_settings`

```rust
/// Load all application settings with defaults.
///
/// # Returns
/// * `Ok(AppSettings)` - Settings with defaults for missing keys
/// * `Err(TuskError)` if database operation fails
pub fn load_settings(&self) -> Result<AppSettings, TuskError>;
```

### `save_setting`

```rust
/// Save a single setting value.
///
/// # Arguments
/// * `key` - Setting key (e.g., "theme", "font_size")
/// * `value` - JSON-serializable value
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(TuskError)` if database operation fails
pub fn save_setting<T: Serialize>(&self, key: &str, value: &T) -> Result<(), TuskError>;
```

### `reset_settings`

```rust
/// Reset all settings to defaults.
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(TuskError)` if database operation fails
pub fn reset_settings(&self) -> Result<(), TuskError>;
```

---

## Editor Tab Operations

### `save_editor_tabs`

```rust
/// Save all editor tabs state.
///
/// # Arguments
/// * `tabs` - Current editor tab states
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(TuskError)` if database operation fails
pub fn save_editor_tabs(&self, tabs: &[EditorTab]) -> Result<(), TuskError>;
```

### `load_editor_tabs`

```rust
/// Load saved editor tabs state.
///
/// # Returns
/// * `Ok(Vec<EditorTab>)` - Saved tabs (empty if none)
/// * `Err(TuskError)` if database operation fails
pub fn load_editor_tabs(&self) -> Result<Vec<EditorTab>, TuskError>;
```

---

## Window State Operations

### `save_window_state`

```rust
/// Save window geometry and panel layout.
///
/// # Arguments
/// * `state` - Current window state
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(TuskError)` if database operation fails
pub fn save_window_state(&self, state: &WindowState) -> Result<(), TuskError>;
```

### `load_window_state`

```rust
/// Load saved window state.
///
/// # Returns
/// * `Ok(Some(state))` if saved
/// * `Ok(None)` if no saved state
/// * `Err(TuskError)` if database operation fails
pub fn load_window_state(&self) -> Result<Option<WindowState>, TuskError>;
```

---

## History Pruning Operations

### `prune_history`

```rust
/// Prune history entries exceeding the limit.
///
/// # Arguments
/// * `limit` - Maximum entries to keep (deletes oldest first)
///
/// # Returns
/// * `Ok(deleted_count)` - Number of entries deleted
/// * `Err(TuskError)` if database operation fails
pub fn prune_history(&self, limit: usize) -> Result<usize, TuskError>;
```

### `prune_history_by_age`

```rust
/// Prune history entries older than specified duration.
///
/// # Arguments
/// * `max_age` - Maximum age of entries to keep
///
/// # Returns
/// * `Ok(deleted_count)` - Number of entries deleted
/// * `Err(TuskError)` if database operation fails
pub fn prune_history_by_age(&self, max_age: chrono::Duration) -> Result<usize, TuskError>;
```

---

## Export/Import Operations

### `export_data`

```rust
/// Export all user data to a portable format.
///
/// Excludes: passwords (in keychain), history IDs, internal timestamps.
///
/// # Returns
/// * `Ok(ExportData)` - Serializable export structure
/// * `Err(TuskError)` if database operation fails
pub fn export_data(&self) -> Result<ExportData, TuskError>;
```

### `import_data`

```rust
/// Import data from an export file.
///
/// # Arguments
/// * `data` - Parsed export data
/// * `conflict_resolution` - How to handle conflicts
///
/// # Returns
/// * `Ok(ImportResult)` - Summary of imported items
/// * `Err(TuskError)` if:
///   - Database operation fails
///   - Version incompatibility detected
pub fn import_data(
    &self,
    data: &ExportData,
    conflict_resolution: ConflictResolution,
) -> Result<ImportResult, TuskError>;
```

---

## Supporting Types

### `ConnectionGroup`

```rust
/// A named group for organizing connections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionGroup {
    pub id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

impl ConnectionGroup {
    pub fn new(name: impl Into<String>, color: Option<String>) -> Self;
    pub fn validate(&self) -> Result<(), String>;
}
```

### `QueryFolder`

```rust
/// A hierarchical folder for organizing saved queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFolder {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

impl QueryFolder {
    pub fn new(name: impl Into<String>, parent_id: Option<Uuid>) -> Self;
    pub fn validate(&self) -> Result<(), String>;
}
```

### `EditorTab`

```rust
/// Persisted state of an editor tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorTab {
    pub tab_id: Uuid,
    pub title: String,
    pub content: String,
    pub cursor_position: usize,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
    pub scroll_offset: f32,
    pub saved_query_id: Option<Uuid>,
    pub connection_id: Option<Uuid>,
    pub is_modified: bool,
    pub created_at: DateTime<Utc>,
}
```

### `WindowState`

```rust
/// Window geometry and panel layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_maximized: bool,
    pub sidebar_width: u32,
    pub results_panel_height: u32,
    pub messages_panel_visible: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 1280,
            height: 800,
            is_maximized: false,
            sidebar_width: 250,
            results_panel_height: 200,
            messages_panel_visible: true,
        }
    }
}
```

### `AppSettings`

```rust
/// Application-wide settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,
    pub font_family: String,
    pub font_size: u32,
    pub tab_size: u32,
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    pub auto_complete: bool,
    pub confirm_destructive: bool,
    pub default_timeout_secs: Option<u32>,
    pub history_limit: u32,
    pub recent_limit: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            font_family: "monospace".to_string(),
            font_size: 14,
            tab_size: 4,
            show_line_numbers: true,
            word_wrap: false,
            auto_complete: true,
            confirm_destructive: true,
            default_timeout_secs: None,
            history_limit: 10000,
            recent_limit: 10,
        }
    }
}
```

### `ExportData`

```rust
/// Portable export format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub version: String,
    pub exported_at: DateTime<Utc>,
    pub connection_groups: Vec<ConnectionGroup>,
    pub connections: Vec<ConnectionConfigExport>,
    pub ssh_tunnels: Vec<SshTunnelConfig>,
    pub query_folders: Vec<QueryFolder>,
    pub saved_queries: Vec<SavedQuery>,
    pub settings: AppSettings,
}
```

### `ConflictResolution`

```rust
/// How to handle import conflicts.
#[derive(Debug, Clone, Copy)]
pub enum ConflictResolution {
    /// Skip conflicting items
    Skip,
    /// Replace existing with imported
    Replace,
    /// Rename imported items
    Rename,
}
```

### `ImportResult`

```rust
/// Summary of import operation.
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub connections_imported: usize,
    pub connections_skipped: usize,
    pub groups_imported: usize,
    pub folders_imported: usize,
    pub queries_imported: usize,
    pub queries_skipped: usize,
    pub settings_applied: bool,
    pub warnings: Vec<String>,
}
```
