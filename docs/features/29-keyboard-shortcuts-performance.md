# Feature 29: Keyboard Shortcuts, Settings, and Performance

## Overview

This feature covers the comprehensive keyboard shortcuts system using GPUI's native action and keybinding system, the complete settings UI built with GPUI components, and performance optimizations required to meet the application's targets. It includes result streaming, schema caching, UI virtualization, and memory management to ensure cold start under 1 second, idle memory under 100MB, and smooth handling of 1M+ row result sets.

## Goals

1. Full keyboard shortcut system using GPUI actions with customization
2. Complete settings UI with all configuration categories
3. Result streaming with batch rendering
4. Schema caching and incremental refresh
5. Virtual scrolling for results and schema tree
6. Memory-efficient data handling
7. Meet all performance targets from design spec

## Dependencies

- Feature 01: Project Setup (GPUI application)
- Feature 05: Local Storage (settings persistence)
- Feature 06: Settings System (base implementation)
- Feature 12: Query Editor (keybindings)
- Feature 14: Results Grid (virtualization)
- Feature 27: Platform Integration (platform-specific shortcuts)

## Technical Specification

### 29.1 GPUI Action System

**File: `src/actions.rs`**

```rust
use gpui::*;

/// Define all application actions
actions!(
    tusk,
    [
        // General
        OpenSettings,
        OpenCommandPalette,
        NewQueryTab,
        CloseTab,
        NextTab,
        PreviousTab,
        ToggleSidebar,

        // Editor
        ExecuteStatement,
        ExecuteAll,
        CancelQuery,
        FormatSql,
        SaveQuery,
        ToggleComment,
        Find,
        Replace,
        GotoLine,
        DuplicateLine,
        MoveLineUp,
        MoveLineDown,

        // Results
        Copy,
        SelectAll,
        Export,
        ToggleEditMode,

        // Navigation
        FocusEditor,
        FocusResults,
        FocusSidebar,
        SearchObjects,
    ]
);

/// Actions with parameters
#[derive(Clone, PartialEq, Deserialize)]
pub struct GotoTab {
    pub index: usize,
}

impl_actions!(tusk, [GotoTab]);

/// Register all default keybindings
pub fn register_keybindings(cx: &mut App) {
    let is_mac = cfg!(target_os = "macos");

    // Context: global
    cx.bind_keys([
        // General
        KeyBinding::new(shortcut(is_mac, "comma"), OpenSettings, None),
        KeyBinding::new(shortcut(is_mac, "shift-p"), OpenCommandPalette, None),
        KeyBinding::new(shortcut(is_mac, "n"), NewQueryTab, None),
        KeyBinding::new(shortcut(is_mac, "w"), CloseTab, None),
        KeyBinding::new(shortcut(is_mac, "shift-]"), NextTab, None),
        KeyBinding::new(shortcut(is_mac, "shift-["), PreviousTab, None),
        KeyBinding::new(shortcut(is_mac, "b"), ToggleSidebar, None),
        KeyBinding::new(shortcut(is_mac, "p"), SearchObjects, None),

        // Tab switching
        KeyBinding::new(shortcut(is_mac, "1"), GotoTab { index: 0 }, Some("Workspace")),
        KeyBinding::new(shortcut(is_mac, "2"), GotoTab { index: 1 }, Some("Workspace")),
        KeyBinding::new(shortcut(is_mac, "3"), GotoTab { index: 2 }, Some("Workspace")),
        KeyBinding::new(shortcut(is_mac, "4"), GotoTab { index: 3 }, Some("Workspace")),
        KeyBinding::new(shortcut(is_mac, "5"), GotoTab { index: 4 }, Some("Workspace")),
        KeyBinding::new(shortcut(is_mac, "6"), GotoTab { index: 5 }, Some("Workspace")),
        KeyBinding::new(shortcut(is_mac, "7"), GotoTab { index: 6 }, Some("Workspace")),
        KeyBinding::new(shortcut(is_mac, "8"), GotoTab { index: 7 }, Some("Workspace")),
        KeyBinding::new(shortcut(is_mac, "9"), GotoTab { index: 8 }, Some("Workspace")),
    ]);

    // Context: editor
    cx.bind_keys([
        KeyBinding::new(shortcut(is_mac, "enter"), ExecuteStatement, Some("QueryEditor")),
        KeyBinding::new(shortcut(is_mac, "shift-enter"), ExecuteAll, Some("QueryEditor")),
        KeyBinding::new(shortcut(is_mac, "."), CancelQuery, Some("QueryEditor")),
        KeyBinding::new(shortcut(is_mac, "shift-f"), FormatSql, Some("QueryEditor")),
        KeyBinding::new(shortcut(is_mac, "s"), SaveQuery, Some("QueryEditor")),
        KeyBinding::new(shortcut(is_mac, "/"), ToggleComment, Some("QueryEditor")),
        KeyBinding::new(shortcut(is_mac, "f"), Find, Some("QueryEditor")),
        KeyBinding::new(shortcut(is_mac, "h"), Replace, Some("QueryEditor")),
        KeyBinding::new(shortcut(is_mac, "g"), GotoLine, Some("QueryEditor")),
        KeyBinding::new(shortcut(is_mac, "shift-d"), DuplicateLine, Some("QueryEditor")),
        KeyBinding::new("alt-up", MoveLineUp, Some("QueryEditor")),
        KeyBinding::new("alt-down", MoveLineDown, Some("QueryEditor")),
    ]);

    // Context: results grid
    cx.bind_keys([
        KeyBinding::new(shortcut(is_mac, "c"), Copy, Some("ResultsGrid")),
        KeyBinding::new(shortcut(is_mac, "a"), SelectAll, Some("ResultsGrid")),
        KeyBinding::new(shortcut(is_mac, "e"), Export, Some("ResultsGrid")),
        KeyBinding::new(shortcut(is_mac, "shift-e"), ToggleEditMode, Some("ResultsGrid")),
    ]);

    // Context: navigation
    cx.bind_keys([
        KeyBinding::new(shortcut(is_mac, "alt-1"), FocusEditor, None),
        KeyBinding::new(shortcut(is_mac, "alt-2"), FocusResults, None),
        KeyBinding::new(shortcut(is_mac, "alt-0"), FocusSidebar, None),
    ]);
}

/// Helper to create platform-specific shortcuts
fn shortcut(is_mac: bool, keys: &str) -> &str {
    // The actual key string - GPUI handles platform differences
    // through its keystroke parsing
    if is_mac {
        // On macOS, cmd is primary
        keys
    } else {
        // On other platforms, ctrl is primary
        // GPUI's Keystroke parsing handles "cmd" -> "ctrl" mapping
        keys
    }
}
```

### 29.2 Shortcut Configuration Model

**File: `src/models/shortcuts.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete keyboard shortcut configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutConfig {
    pub version: u32,
    pub shortcuts: HashMap<String, Shortcut>,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            version: 1,
            shortcuts: Self::default_shortcuts(),
        }
    }
}

impl ShortcutConfig {
    fn default_shortcuts() -> HashMap<String, Shortcut> {
        let mut map = HashMap::new();

        // General
        map.insert("settings".into(), Shortcut::new("General", "Settings", "cmd-,", "ctrl-,"));
        map.insert("command_palette".into(), Shortcut::new("General", "Command Palette", "cmd-shift-p", "ctrl-shift-p"));
        map.insert("new_query".into(), Shortcut::new("General", "New Query Tab", "cmd-n", "ctrl-n"));
        map.insert("close_tab".into(), Shortcut::new("General", "Close Tab", "cmd-w", "ctrl-w"));
        map.insert("next_tab".into(), Shortcut::new("General", "Next Tab", "cmd-shift-]", "ctrl-tab"));
        map.insert("prev_tab".into(), Shortcut::new("General", "Previous Tab", "cmd-shift-[", "ctrl-shift-tab"));
        map.insert("toggle_sidebar".into(), Shortcut::new("General", "Toggle Sidebar", "cmd-b", "ctrl-b"));

        // Editor
        map.insert("execute".into(), Shortcut::new("Editor", "Execute Statement", "cmd-enter", "ctrl-enter"));
        map.insert("execute_all".into(), Shortcut::new("Editor", "Execute All", "cmd-shift-enter", "ctrl-shift-enter"));
        map.insert("cancel".into(), Shortcut::new("Editor", "Cancel Query", "cmd-.", "ctrl-."));
        map.insert("format".into(), Shortcut::new("Editor", "Format SQL", "cmd-shift-f", "ctrl-shift-f"));
        map.insert("save".into(), Shortcut::new("Editor", "Save", "cmd-s", "ctrl-s"));
        map.insert("comment".into(), Shortcut::new("Editor", "Toggle Comment", "cmd-/", "ctrl-/"));
        map.insert("find".into(), Shortcut::new("Editor", "Find", "cmd-f", "ctrl-f"));
        map.insert("replace".into(), Shortcut::new("Editor", "Replace", "cmd-h", "ctrl-h"));
        map.insert("goto_line".into(), Shortcut::new("Editor", "Go to Line", "cmd-g", "ctrl-g"));
        map.insert("duplicate_line".into(), Shortcut::new("Editor", "Duplicate Line", "cmd-shift-d", "ctrl-shift-d"));
        map.insert("move_line_up".into(), Shortcut::new("Editor", "Move Line Up", "alt-up", "alt-up"));
        map.insert("move_line_down".into(), Shortcut::new("Editor", "Move Line Down", "alt-down", "alt-down"));

        // Results
        map.insert("copy".into(), Shortcut::new("Results", "Copy", "cmd-c", "ctrl-c"));
        map.insert("select_all".into(), Shortcut::new("Results", "Select All", "cmd-a", "ctrl-a"));
        map.insert("export".into(), Shortcut::new("Results", "Export", "cmd-e", "ctrl-e"));
        map.insert("edit_mode".into(), Shortcut::new("Results", "Toggle Edit Mode", "cmd-shift-e", "ctrl-shift-e"));

        // Navigation
        map.insert("focus_editor".into(), Shortcut::new("Navigation", "Focus Editor", "cmd-1", "alt-1"));
        map.insert("focus_results".into(), Shortcut::new("Navigation", "Focus Results", "cmd-2", "alt-2"));
        map.insert("focus_sidebar".into(), Shortcut::new("Navigation", "Focus Sidebar", "cmd-0", "alt-0"));
        map.insert("search_objects".into(), Shortcut::new("Navigation", "Search Objects", "cmd-p", "ctrl-p"));

        map
    }

    pub fn get_binding(&self, action: &str) -> Option<&Shortcut> {
        self.shortcuts.get(action)
    }
}

/// Individual shortcut definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shortcut {
    pub category: String,
    pub label: String,
    pub mac_binding: String,
    pub other_binding: String,
    pub custom_binding: Option<String>,
    pub enabled: bool,
}

impl Shortcut {
    pub fn new(category: &str, label: &str, mac: &str, other: &str) -> Self {
        Self {
            category: category.into(),
            label: label.into(),
            mac_binding: mac.into(),
            other_binding: other.into(),
            custom_binding: None,
            enabled: true,
        }
    }

    /// Get the effective binding for current platform
    pub fn effective_binding(&self) -> &str {
        if let Some(custom) = &self.custom_binding {
            return custom;
        }

        if cfg!(target_os = "macos") {
            &self.mac_binding
        } else {
            &self.other_binding
        }
    }

    /// Get the default binding for current platform
    pub fn default_binding(&self) -> &str {
        if cfg!(target_os = "macos") {
            &self.mac_binding
        } else {
            &self.other_binding
        }
    }

    pub fn is_customized(&self) -> bool {
        self.custom_binding.is_some()
    }
}
```

### 29.3 Shortcut Service

**File: `src/services/shortcuts.rs`**

```rust
use crate::models::shortcuts::{ShortcutConfig, Shortcut};
use crate::services::storage::StorageService;
use crate::error::Result;
use gpui::*;
use std::sync::Arc;

const SHORTCUTS_KEY: &str = "shortcuts_config";

pub struct ShortcutService {
    storage: Arc<StorageService>,
    config: parking_lot::RwLock<ShortcutConfig>,
}

impl ShortcutService {
    pub fn new(storage: Arc<StorageService>) -> Self {
        Self {
            storage,
            config: parking_lot::RwLock::new(ShortcutConfig::default()),
        }
    }

    /// Load shortcut configuration from storage
    pub async fn load(&self) -> Result<()> {
        let config = match self.storage.get(SHORTCUTS_KEY).await? {
            Some(json) => serde_json::from_str(&json)?,
            None => ShortcutConfig::default(),
        };

        *self.config.write() = config;
        Ok(())
    }

    /// Get current configuration
    pub fn config(&self) -> ShortcutConfig {
        self.config.read().clone()
    }

    /// Save current configuration
    pub async fn save(&self) -> Result<()> {
        let config = self.config.read().clone();
        let json = serde_json::to_string(&config)?;
        self.storage.set(SHORTCUTS_KEY, &json).await
    }

    /// Update a single shortcut binding
    pub async fn update_binding(&self, action: &str, binding: &str) -> Result<()> {
        {
            let mut config = self.config.write();
            if let Some(shortcut) = config.shortcuts.get_mut(action) {
                shortcut.custom_binding = Some(binding.to_string());
            }
        }
        self.save().await
    }

    /// Reset a shortcut to default
    pub async fn reset_binding(&self, action: &str) -> Result<()> {
        {
            let mut config = self.config.write();
            if let Some(shortcut) = config.shortcuts.get_mut(action) {
                shortcut.custom_binding = None;
            }
        }
        self.save().await
    }

    /// Reset all shortcuts to defaults
    pub async fn reset_all(&self) -> Result<()> {
        *self.config.write() = ShortcutConfig::default();
        self.save().await
    }

    /// Export shortcuts to JSON string
    pub fn export(&self) -> Result<String> {
        let config = self.config.read();
        Ok(serde_json::to_string_pretty(&*config)?)
    }

    /// Import shortcuts from JSON string
    pub async fn import(&self, json: &str) -> Result<()> {
        let config: ShortcutConfig = serde_json::from_str(json)?;
        *self.config.write() = config;
        self.save().await
    }

    /// Apply keybindings to GPUI context
    pub fn apply_keybindings(&self, cx: &mut App) {
        let config = self.config.read();

        // Clear existing and rebind all shortcuts
        // Note: In production, this would need to track and clear old bindings
        for (action, shortcut) in config.shortcuts.iter() {
            if shortcut.enabled {
                let binding = shortcut.effective_binding();
                // Apply binding based on action name
                self.bind_action(cx, action, binding);
            }
        }
    }

    fn bind_action(&self, cx: &mut App, action: &str, binding: &str) {
        // Map action names to GPUI actions
        match action {
            "settings" => {
                cx.bind_keys([KeyBinding::new(binding, crate::actions::OpenSettings, None)]);
            }
            "new_query" => {
                cx.bind_keys([KeyBinding::new(binding, crate::actions::NewQueryTab, None)]);
            }
            "execute" => {
                cx.bind_keys([KeyBinding::new(binding, crate::actions::ExecuteStatement, Some("QueryEditor"))]);
            }
            // ... other actions
            _ => {}
        }
    }
}
```

### 29.4 Settings Models

**File: `src/models/settings.rs`**

```rust
use serde::{Deserialize, Serialize};
use gpui::Hsla;

/// Complete application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub editor: EditorSettings,
    pub results: ResultsSettings,
    pub query: QuerySettings,
    pub connections: ConnectionSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            general: GeneralSettings::default(),
            editor: EditorSettings::default(),
            results: ResultsSettings::default(),
            query: QuerySettings::default(),
            connections: ConnectionSettings::default(),
        }
    }
}

/// General settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub theme: ThemePreference,
    pub language: String,
    pub startup_behavior: StartupBehavior,
    pub auto_save_interval_secs: u32,
    pub confirm_close_unsaved: bool,
    pub show_welcome_on_startup: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::System,
            language: "en".into(),
            startup_behavior: StartupBehavior::RestorePrevious,
            auto_save_interval_secs: 30,
            confirm_close_unsaved: true,
            show_welcome_on_startup: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThemePreference {
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StartupBehavior {
    RestorePrevious,
    StartFresh,
}

/// Editor settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    pub font_family: String,
    pub font_size: f32,
    pub line_height: f32,
    pub tab_size: u32,
    pub use_spaces: bool,
    pub show_line_numbers: bool,
    pub show_gutter: bool,
    pub show_indent_guides: bool,
    pub word_wrap: bool,
    pub autocomplete_delay_ms: u32,
    pub bracket_matching: bool,
    pub highlight_current_line: bool,
    pub cursor_blink_rate_ms: u32,
    pub scroll_beyond_last_line: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_family: "JetBrains Mono, Menlo, Monaco, Consolas, monospace".into(),
            font_size: 14.0,
            line_height: 1.5,
            tab_size: 2,
            use_spaces: true,
            show_line_numbers: true,
            show_gutter: true,
            show_indent_guides: true,
            word_wrap: false,
            autocomplete_delay_ms: 100,
            bracket_matching: true,
            highlight_current_line: true,
            cursor_blink_rate_ms: 500,
            scroll_beyond_last_line: true,
        }
    }
}

/// Results display settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsSettings {
    pub default_row_limit: u32,
    pub date_format: String,
    pub time_format: String,
    pub timestamp_format: String,
    pub number_locale: String,
    pub null_display: String,
    pub boolean_display: BooleanDisplay,
    pub truncate_text_at: u32,
    pub copy_format: CopyFormat,
    pub row_height: f32,
    pub alternate_row_colors: bool,
    pub show_row_numbers: bool,
}

impl Default for ResultsSettings {
    fn default() -> Self {
        Self {
            default_row_limit: 1000,
            date_format: "yyyy-MM-dd".into(),
            time_format: "HH:mm:ss".into(),
            timestamp_format: "yyyy-MM-dd HH:mm:ss".into(),
            number_locale: "en-US".into(),
            null_display: "NULL".into(),
            boolean_display: BooleanDisplay::TrueFalse,
            truncate_text_at: 500,
            copy_format: CopyFormat::Tsv,
            row_height: 28.0,
            alternate_row_colors: true,
            show_row_numbers: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BooleanDisplay {
    TrueFalse,
    YesNo,
    OneZero,
    Checkmark,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CopyFormat {
    Tsv,
    Csv,
    Json,
    Markdown,
}

/// Query execution settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuerySettings {
    pub default_statement_timeout_secs: u32,
    pub confirm_ddl: bool,
    pub confirm_destructive: bool,
    pub auto_uppercase_keywords: bool,
    pub auto_limit: bool,
    pub auto_limit_rows: u32,
    pub streaming_batch_size: u32,
    pub max_results_in_memory: u32,
}

impl Default for QuerySettings {
    fn default() -> Self {
        Self {
            default_statement_timeout_secs: 300, // 5 minutes
            confirm_ddl: true,
            confirm_destructive: true,
            auto_uppercase_keywords: false,
            auto_limit: true,
            auto_limit_rows: 1000,
            streaming_batch_size: 1000,
            max_results_in_memory: 100_000,
        }
    }
}

/// Connection default settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSettings {
    pub default_ssl_mode: String,
    pub connection_timeout_secs: u32,
    pub auto_reconnect_attempts: u32,
    pub keepalive_interval_secs: u32,
    pub default_statement_timeout_secs: u32,
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            default_ssl_mode: "prefer".into(),
            connection_timeout_secs: 30,
            auto_reconnect_attempts: 5,
            keepalive_interval_secs: 60,
            default_statement_timeout_secs: 300,
        }
    }
}
```

### 29.5 Settings State Management

**File: `src/state/settings_state.rs`**

```rust
use crate::models::settings::AppSettings;
use crate::services::storage::StorageService;
use gpui::Global;
use parking_lot::RwLock;
use std::sync::Arc;

const SETTINGS_KEY: &str = "app_settings";

/// Global settings state
pub struct SettingsState {
    settings: RwLock<AppSettings>,
    storage: Arc<StorageService>,
}

impl Global for SettingsState {}

impl SettingsState {
    pub fn new(storage: Arc<StorageService>) -> Self {
        Self {
            settings: RwLock::new(AppSettings::default()),
            storage,
        }
    }

    /// Load settings from storage
    pub async fn load(&self) -> crate::error::Result<()> {
        let settings = match self.storage.get(SETTINGS_KEY).await? {
            Some(json) => serde_json::from_str(&json)?,
            None => AppSettings::default(),
        };
        *self.settings.write() = settings;
        Ok(())
    }

    /// Get current settings
    pub fn get(&self) -> AppSettings {
        self.settings.read().clone()
    }

    /// Update settings
    pub async fn update(&self, settings: AppSettings) -> crate::error::Result<()> {
        *self.settings.write() = settings.clone();
        let json = serde_json::to_string(&settings)?;
        self.storage.set(SETTINGS_KEY, &json).await
    }

    /// Update a specific section
    pub async fn update_general(&self, general: crate::models::settings::GeneralSettings) -> crate::error::Result<()> {
        let mut settings = self.settings.write();
        settings.general = general;
        let json = serde_json::to_string(&*settings)?;
        drop(settings);
        self.storage.set(SETTINGS_KEY, &json).await
    }

    pub async fn update_editor(&self, editor: crate::models::settings::EditorSettings) -> crate::error::Result<()> {
        let mut settings = self.settings.write();
        settings.editor = editor;
        let json = serde_json::to_string(&*settings)?;
        drop(settings);
        self.storage.set(SETTINGS_KEY, &json).await
    }

    pub async fn update_results(&self, results: crate::models::settings::ResultsSettings) -> crate::error::Result<()> {
        let mut settings = self.settings.write();
        settings.results = results;
        let json = serde_json::to_string(&*settings)?;
        drop(settings);
        self.storage.set(SETTINGS_KEY, &json).await
    }

    pub async fn update_query(&self, query: crate::models::settings::QuerySettings) -> crate::error::Result<()> {
        let mut settings = self.settings.write();
        settings.query = query;
        let json = serde_json::to_string(&*settings)?;
        drop(settings);
        self.storage.set(SETTINGS_KEY, &json).await
    }

    pub async fn update_connections(&self, connections: crate::models::settings::ConnectionSettings) -> crate::error::Result<()> {
        let mut settings = self.settings.write();
        settings.connections = connections;
        let json = serde_json::to_string(&*settings)?;
        drop(settings);
        self.storage.set(SETTINGS_KEY, &json).await
    }

    /// Reset to defaults
    pub async fn reset(&self) -> crate::error::Result<()> {
        self.update(AppSettings::default()).await
    }
}
```

### 29.6 Performance: Result Streaming

**File: `src/services/query_streaming.rs`**

```rust
use crate::models::query::{QueryResult, Row, Column, CellValue};
use crate::error::Result;
use tokio_postgres::{Client, Row as PgRow};
use std::sync::Arc;
use tokio::sync::mpsc;
use std::time::Instant;

const DEFAULT_BATCH_SIZE: usize = 1000;

/// Streaming query result batch
#[derive(Debug, Clone)]
pub struct RowBatch {
    pub batch_num: u32,
    pub rows: Vec<Row>,
}

/// Query completion event
#[derive(Debug, Clone)]
pub struct QueryComplete {
    pub total_rows: u64,
    pub elapsed_ms: u64,
    pub batches: u32,
}

/// Events emitted during streaming
#[derive(Debug, Clone)]
pub enum StreamEvent {
    Columns(Vec<Column>),
    Batch(RowBatch),
    Complete(QueryComplete),
    Error(String),
}

pub struct QueryStreaming {
    batch_size: usize,
}

impl QueryStreaming {
    pub fn new(batch_size: Option<usize>) -> Self {
        Self {
            batch_size: batch_size.unwrap_or(DEFAULT_BATCH_SIZE),
        }
    }

    /// Execute query with streaming results via channel
    pub async fn execute_streaming(
        &self,
        client: &Client,
        sql: &str,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<()> {
        let start = Instant::now();

        // Prepare and execute
        let statement = client.prepare(sql).await?;
        let columns: Vec<Column> = statement.columns().iter().map(|c| Column {
            name: c.name().to_string(),
            data_type: c.type_().name().to_string(),
            type_oid: c.type_().oid(),
            nullable: true,
        }).collect();

        // Emit schema immediately
        let _ = tx.send(StreamEvent::Columns(columns.clone())).await;

        // Stream rows
        let row_stream = client.query_raw(&statement, &[] as &[&str]).await?;
        tokio::pin!(row_stream);

        let mut batch: Vec<Row> = Vec::with_capacity(self.batch_size);
        let mut total_rows = 0u64;
        let mut batch_num = 0u32;

        use futures::StreamExt;
        while let Some(result) = row_stream.next().await {
            match result {
                Ok(pg_row) => {
                    let row = self.convert_row(&pg_row, &columns)?;
                    batch.push(row);
                    total_rows += 1;

                    if batch.len() >= self.batch_size {
                        let _ = tx.send(StreamEvent::Batch(RowBatch {
                            batch_num,
                            rows: std::mem::take(&mut batch),
                        })).await;
                        batch_num += 1;
                        batch = Vec::with_capacity(self.batch_size);
                    }
                }
                Err(e) => {
                    let _ = tx.send(StreamEvent::Error(e.to_string())).await;
                    return Err(e.into());
                }
            }
        }

        // Emit final partial batch
        if !batch.is_empty() {
            let _ = tx.send(StreamEvent::Batch(RowBatch {
                batch_num,
                rows: batch,
            })).await;
            batch_num += 1;
        }

        // Emit completion
        let elapsed_ms = start.elapsed().as_millis() as u64;
        let _ = tx.send(StreamEvent::Complete(QueryComplete {
            total_rows,
            elapsed_ms,
            batches: batch_num,
        })).await;

        Ok(())
    }

    fn convert_row(&self, pg_row: &PgRow, columns: &[Column]) -> Result<Row> {
        let mut values = Vec::with_capacity(columns.len());

        for (i, col) in columns.iter().enumerate() {
            let value = self.extract_value(pg_row, i, col.type_oid)?;
            values.push(value);
        }

        Ok(Row { values })
    }

    fn extract_value(&self, row: &PgRow, idx: usize, type_oid: u32) -> Result<CellValue> {
        use tokio_postgres::types::Type;

        // Check for NULL first
        match type_oid {
            oid if oid == Type::INT2.oid() => {
                Ok(row.try_get::<_, Option<i16>>(idx)?
                    .map(|v| CellValue::Integer(v as i64))
                    .unwrap_or(CellValue::Null))
            }
            oid if oid == Type::INT4.oid() => {
                Ok(row.try_get::<_, Option<i32>>(idx)?
                    .map(|v| CellValue::Integer(v as i64))
                    .unwrap_or(CellValue::Null))
            }
            oid if oid == Type::INT8.oid() => {
                Ok(row.try_get::<_, Option<i64>>(idx)?
                    .map(CellValue::Integer)
                    .unwrap_or(CellValue::Null))
            }
            oid if oid == Type::FLOAT4.oid() => {
                Ok(row.try_get::<_, Option<f32>>(idx)?
                    .map(|v| CellValue::Float(v as f64))
                    .unwrap_or(CellValue::Null))
            }
            oid if oid == Type::FLOAT8.oid() => {
                Ok(row.try_get::<_, Option<f64>>(idx)?
                    .map(CellValue::Float)
                    .unwrap_or(CellValue::Null))
            }
            oid if oid == Type::BOOL.oid() => {
                Ok(row.try_get::<_, Option<bool>>(idx)?
                    .map(CellValue::Boolean)
                    .unwrap_or(CellValue::Null))
            }
            oid if oid == Type::JSON.oid() || oid == Type::JSONB.oid() => {
                Ok(row.try_get::<_, Option<serde_json::Value>>(idx)?
                    .map(CellValue::Json)
                    .unwrap_or(CellValue::Null))
            }
            oid if oid == Type::BYTEA.oid() => {
                Ok(row.try_get::<_, Option<Vec<u8>>>(idx)?
                    .map(CellValue::Binary)
                    .unwrap_or(CellValue::Null))
            }
            _ => {
                // Default to string representation
                Ok(row.try_get::<_, Option<String>>(idx)?
                    .map(CellValue::Text)
                    .unwrap_or(CellValue::Null))
            }
        }
    }
}
```

### 29.7 Performance: Schema Caching

**File: `src/services/schema_cache.rs`**

```rust
use crate::models::schema::*;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_TTL_SECS: u64 = 300; // 5 minutes

/// Cache entry with TTL
struct CacheEntry<T> {
    data: T,
    fetched_at: Instant,
    ttl: Duration,
}

impl<T: Clone> CacheEntry<T> {
    fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            fetched_at: Instant::now(),
            ttl,
        }
    }

    fn is_valid(&self) -> bool {
        self.fetched_at.elapsed() < self.ttl
    }

    fn get(&self) -> Option<T> {
        if self.is_valid() {
            Some(self.data.clone())
        } else {
            None
        }
    }
}

/// Schema cache for a single connection
pub struct ConnectionSchemaCache {
    ttl: Duration,
    schemas: RwLock<Option<CacheEntry<Vec<SchemaInfo>>>>,
    tables: RwLock<HashMap<String, CacheEntry<Vec<TableInfo>>>>,
    columns: RwLock<HashMap<String, CacheEntry<Vec<ColumnInfo>>>>,
    indexes: RwLock<HashMap<String, CacheEntry<Vec<IndexInfo>>>>,
    constraints: RwLock<HashMap<String, CacheEntry<Vec<ConstraintInfo>>>>,
    functions: RwLock<HashMap<String, CacheEntry<Vec<FunctionInfo>>>>,
}

impl ConnectionSchemaCache {
    pub fn new() -> Self {
        Self::with_ttl(Duration::from_secs(DEFAULT_TTL_SECS))
    }

    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            ttl,
            schemas: RwLock::new(None),
            tables: RwLock::new(HashMap::new()),
            columns: RwLock::new(HashMap::new()),
            indexes: RwLock::new(HashMap::new()),
            constraints: RwLock::new(HashMap::new()),
            functions: RwLock::new(HashMap::new()),
        }
    }

    // Schemas
    pub fn get_schemas(&self) -> Option<Vec<SchemaInfo>> {
        self.schemas.read().as_ref().and_then(|e| e.get())
    }

    pub fn set_schemas(&self, schemas: Vec<SchemaInfo>) {
        *self.schemas.write() = Some(CacheEntry::new(schemas, self.ttl));
    }

    // Tables
    pub fn get_tables(&self, schema: &str) -> Option<Vec<TableInfo>> {
        self.tables.read().get(schema).and_then(|e| e.get())
    }

    pub fn set_tables(&self, schema: &str, tables: Vec<TableInfo>) {
        self.tables.write().insert(
            schema.to_string(),
            CacheEntry::new(tables, self.ttl),
        );
    }

    // Columns
    pub fn get_columns(&self, table_key: &str) -> Option<Vec<ColumnInfo>> {
        self.columns.read().get(table_key).and_then(|e| e.get())
    }

    pub fn set_columns(&self, table_key: &str, columns: Vec<ColumnInfo>) {
        self.columns.write().insert(
            table_key.to_string(),
            CacheEntry::new(columns, self.ttl),
        );
    }

    // Indexes
    pub fn get_indexes(&self, table_key: &str) -> Option<Vec<IndexInfo>> {
        self.indexes.read().get(table_key).and_then(|e| e.get())
    }

    pub fn set_indexes(&self, table_key: &str, indexes: Vec<IndexInfo>) {
        self.indexes.write().insert(
            table_key.to_string(),
            CacheEntry::new(indexes, self.ttl),
        );
    }

    // Constraints
    pub fn get_constraints(&self, table_key: &str) -> Option<Vec<ConstraintInfo>> {
        self.constraints.read().get(table_key).and_then(|e| e.get())
    }

    pub fn set_constraints(&self, table_key: &str, constraints: Vec<ConstraintInfo>) {
        self.constraints.write().insert(
            table_key.to_string(),
            CacheEntry::new(constraints, self.ttl),
        );
    }

    // Functions
    pub fn get_functions(&self, schema: &str) -> Option<Vec<FunctionInfo>> {
        self.functions.read().get(schema).and_then(|e| e.get())
    }

    pub fn set_functions(&self, schema: &str, functions: Vec<FunctionInfo>) {
        self.functions.write().insert(
            schema.to_string(),
            CacheEntry::new(functions, self.ttl),
        );
    }

    /// Invalidate all caches
    pub fn invalidate_all(&self) {
        *self.schemas.write() = None;
        self.tables.write().clear();
        self.columns.write().clear();
        self.indexes.write().clear();
        self.constraints.write().clear();
        self.functions.write().clear();
    }

    /// Invalidate cache for a specific table
    pub fn invalidate_table(&self, table_key: &str) {
        self.columns.write().remove(table_key);
        self.indexes.write().remove(table_key);
        self.constraints.write().remove(table_key);
    }

    /// Invalidate cache for a specific schema
    pub fn invalidate_schema(&self, schema: &str) {
        self.tables.write().remove(schema);
        self.functions.write().remove(schema);

        // Also invalidate all tables in this schema
        let prefix = format!("{}.", schema);
        self.columns.write().retain(|k, _| !k.starts_with(&prefix));
        self.indexes.write().retain(|k, _| !k.starts_with(&prefix));
        self.constraints.write().retain(|k, _| !k.starts_with(&prefix));
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            schemas_cached: self.schemas.read().is_some(),
            tables_entries: self.tables.read().len(),
            columns_entries: self.columns.read().len(),
            indexes_entries: self.indexes.read().len(),
            functions_entries: self.functions.read().len(),
        }
    }
}

impl Default for ConnectionSchemaCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub schemas_cached: bool,
    pub tables_entries: usize,
    pub columns_entries: usize,
    pub indexes_entries: usize,
    pub functions_entries: usize,
}

/// Global schema cache manager for all connections
pub struct SchemaCacheManager {
    caches: RwLock<HashMap<uuid::Uuid, Arc<ConnectionSchemaCache>>>,
    ttl: Duration,
}

impl SchemaCacheManager {
    pub fn new() -> Self {
        Self::with_ttl(Duration::from_secs(DEFAULT_TTL_SECS))
    }

    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            caches: RwLock::new(HashMap::new()),
            ttl,
        }
    }

    /// Get or create cache for a connection
    pub fn get_cache(&self, connection_id: uuid::Uuid) -> Arc<ConnectionSchemaCache> {
        let caches = self.caches.read();
        if let Some(cache) = caches.get(&connection_id) {
            return cache.clone();
        }
        drop(caches);

        let mut caches = self.caches.write();
        caches.entry(connection_id)
            .or_insert_with(|| Arc::new(ConnectionSchemaCache::with_ttl(self.ttl)))
            .clone()
    }

    /// Remove cache for a connection
    pub fn remove_cache(&self, connection_id: uuid::Uuid) {
        self.caches.write().remove(&connection_id);
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.caches.write().clear();
    }
}

impl Default for SchemaCacheManager {
    fn default() -> Self {
        Self::new()
    }
}
```

### 29.8 Settings UI Components

**File: `src/ui/settings/settings_page.rs`**

```rust
use crate::models::settings::*;
use crate::state::settings_state::SettingsState;
use crate::ui::theme::Theme;
use gpui::*;
use std::sync::Arc;

/// Settings page tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Editor,
    Results,
    Query,
    Connections,
    Shortcuts,
}

impl SettingsTab {
    fn label(&self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Editor => "Editor",
            Self::Results => "Results",
            Self::Query => "Query Execution",
            Self::Connections => "Connections",
            Self::Shortcuts => "Shortcuts",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            Self::General => "⚙",
            Self::Editor => "✎",
            Self::Results => "☰",
            Self::Query => "▶",
            Self::Connections => "🔗",
            Self::Shortcuts => "⌨",
        }
    }
}

/// Settings page component
pub struct SettingsPage {
    active_tab: SettingsTab,
    settings: AppSettings,
    settings_state: Arc<SettingsState>,
    focus_handle: FocusHandle,
}

impl SettingsPage {
    pub fn new(settings_state: Arc<SettingsState>, cx: &mut Context<Self>) -> Self {
        Self {
            active_tab: SettingsTab::General,
            settings: settings_state.get(),
            settings_state,
            focus_handle: cx.focus_handle(),
        }
    }

    fn set_tab(&mut self, tab: SettingsTab, cx: &mut Context<Self>) {
        self.active_tab = tab;
        cx.notify();
    }

    fn save_settings(&self, cx: &mut Context<Self>) {
        let settings = self.settings.clone();
        let state = self.settings_state.clone();
        cx.spawn(|_, _| async move {
            let _ = state.update(settings).await;
        }).detach();
    }
}

impl FocusableView for SettingsPage {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SettingsPage {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_row()
            .size_full()
            .bg(theme.bg_primary)
            .track_focus(&self.focus_handle)
            .child(self.render_sidebar(cx))
            .child(self.render_content(cx))
    }
}

impl SettingsPage {
    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let active_tab = self.active_tab;

        let tabs = [
            SettingsTab::General,
            SettingsTab::Editor,
            SettingsTab::Results,
            SettingsTab::Query,
            SettingsTab::Connections,
            SettingsTab::Shortcuts,
        ];

        div()
            .w(px(240.0))
            .border_r_1()
            .border_color(theme.border)
            .p_6()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .mb_6()
                    .child("Settings")
            )
            .children(tabs.iter().map(|tab| {
                let is_active = *tab == active_tab;
                let tab_val = *tab;

                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .when(is_active, |el| el.bg(theme.primary).text_color(theme.text_on_primary))
                    .when(!is_active, |el| {
                        el.text_color(theme.text_secondary)
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                    })
                    .on_click(cx.listener(move |this, _, cx| {
                        this.set_tab(tab_val, cx);
                    }))
                    .child(tab.icon())
                    .child(tab.label())
            }))
    }

    fn render_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex_1()
            .p_8()
            .overflow_y_scroll()
            .child(match self.active_tab {
                SettingsTab::General => self.render_general_settings(cx).into_any_element(),
                SettingsTab::Editor => self.render_editor_settings(cx).into_any_element(),
                SettingsTab::Results => self.render_results_settings(cx).into_any_element(),
                SettingsTab::Query => self.render_query_settings(cx).into_any_element(),
                SettingsTab::Connections => self.render_connection_settings(cx).into_any_element(),
                SettingsTab::Shortcuts => self.render_shortcuts_settings(cx).into_any_element(),
            })
    }

    fn render_general_settings(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_col()
            .gap_6()
            .max_w(px(600.0))
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .mb_4()
                    .child("General")
            )
            .child(
                SettingGroup::new("Appearance")
                    .child(
                        SettingRow::new("Theme")
                            .description("Choose the color theme")
                            .child(self.render_theme_selector(cx))
                    )
            )
            .child(
                SettingGroup::new("Startup")
                    .child(
                        SettingRow::new("On startup")
                            .description("What to do when the app starts")
                            .child(self.render_startup_selector(cx))
                    )
                    .child(
                        SettingRow::new("Show welcome")
                            .description("Show welcome screen on startup")
                            .child(self.render_checkbox(
                                self.settings.general.show_welcome_on_startup,
                                |this, value, cx| {
                                    this.settings.general.show_welcome_on_startup = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
            )
            .child(
                SettingGroup::new("Auto-save")
                    .child(
                        SettingRow::new("Interval")
                            .description("Auto-save interval in seconds")
                            .child(self.render_number_input(
                                self.settings.general.auto_save_interval_secs,
                                1,
                                300,
                                |this, value, cx| {
                                    this.settings.general.auto_save_interval_secs = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
            )
    }

    fn render_editor_settings(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .max_w(px(600.0))
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .mb_4()
                    .child("Editor")
            )
            .child(
                SettingGroup::new("Font")
                    .child(
                        SettingRow::new("Font family")
                            .description("Monospace font for the editor")
                            .child(self.render_text_input(
                                &self.settings.editor.font_family,
                                |this, value, cx| {
                                    this.settings.editor.font_family = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
                    .child(
                        SettingRow::new("Font size")
                            .description("Font size in pixels")
                            .child(self.render_number_input(
                                self.settings.editor.font_size as u32,
                                8,
                                32,
                                |this, value, cx| {
                                    this.settings.editor.font_size = value as f32;
                                    this.save_settings(cx);
                                },
                            ))
                    )
            )
            .child(
                SettingGroup::new("Indentation")
                    .child(
                        SettingRow::new("Tab size")
                            .description("Number of spaces per tab")
                            .child(self.render_number_input(
                                self.settings.editor.tab_size,
                                1,
                                8,
                                |this, value, cx| {
                                    this.settings.editor.tab_size = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
                    .child(
                        SettingRow::new("Use spaces")
                            .description("Insert spaces when pressing tab")
                            .child(self.render_checkbox(
                                self.settings.editor.use_spaces,
                                |this, value, cx| {
                                    this.settings.editor.use_spaces = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
            )
            .child(
                SettingGroup::new("Display")
                    .child(
                        SettingRow::new("Line numbers")
                            .description("Show line numbers in the gutter")
                            .child(self.render_checkbox(
                                self.settings.editor.show_line_numbers,
                                |this, value, cx| {
                                    this.settings.editor.show_line_numbers = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
                    .child(
                        SettingRow::new("Word wrap")
                            .description("Wrap long lines")
                            .child(self.render_checkbox(
                                self.settings.editor.word_wrap,
                                |this, value, cx| {
                                    this.settings.editor.word_wrap = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
                    .child(
                        SettingRow::new("Highlight current line")
                            .description("Highlight the line with the cursor")
                            .child(self.render_checkbox(
                                self.settings.editor.highlight_current_line,
                                |this, value, cx| {
                                    this.settings.editor.highlight_current_line = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
            )
    }

    fn render_results_settings(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .max_w(px(600.0))
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .mb_4()
                    .child("Results")
            )
            .child(
                SettingGroup::new("Display")
                    .child(
                        SettingRow::new("Row limit")
                            .description("Default maximum rows to fetch")
                            .child(self.render_number_input(
                                self.settings.results.default_row_limit,
                                100,
                                100_000,
                                |this, value, cx| {
                                    this.settings.results.default_row_limit = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
                    .child(
                        SettingRow::new("NULL display")
                            .description("How to display NULL values")
                            .child(self.render_text_input(
                                &self.settings.results.null_display,
                                |this, value, cx| {
                                    this.settings.results.null_display = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
                    .child(
                        SettingRow::new("Alternate row colors")
                            .description("Alternate background colors for rows")
                            .child(self.render_checkbox(
                                self.settings.results.alternate_row_colors,
                                |this, value, cx| {
                                    this.settings.results.alternate_row_colors = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
            )
            .child(
                SettingGroup::new("Formatting")
                    .child(
                        SettingRow::new("Date format")
                            .description("Format for date values")
                            .child(self.render_text_input(
                                &self.settings.results.date_format,
                                |this, value, cx| {
                                    this.settings.results.date_format = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
                    .child(
                        SettingRow::new("Time format")
                            .description("Format for time values")
                            .child(self.render_text_input(
                                &self.settings.results.time_format,
                                |this, value, cx| {
                                    this.settings.results.time_format = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
            )
    }

    fn render_query_settings(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .max_w(px(600.0))
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .mb_4()
                    .child("Query Execution")
            )
            .child(
                SettingGroup::new("Timeouts")
                    .child(
                        SettingRow::new("Statement timeout")
                            .description("Default timeout in seconds (0 = no limit)")
                            .child(self.render_number_input(
                                self.settings.query.default_statement_timeout_secs,
                                0,
                                3600,
                                |this, value, cx| {
                                    this.settings.query.default_statement_timeout_secs = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
            )
            .child(
                SettingGroup::new("Confirmations")
                    .child(
                        SettingRow::new("Confirm DDL")
                            .description("Confirm before executing DDL statements")
                            .child(self.render_checkbox(
                                self.settings.query.confirm_ddl,
                                |this, value, cx| {
                                    this.settings.query.confirm_ddl = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
                    .child(
                        SettingRow::new("Confirm destructive")
                            .description("Confirm DELETE/TRUNCATE without WHERE")
                            .child(self.render_checkbox(
                                self.settings.query.confirm_destructive,
                                |this, value, cx| {
                                    this.settings.query.confirm_destructive = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
            )
            .child(
                SettingGroup::new("Auto-limit")
                    .child(
                        SettingRow::new("Enable auto-limit")
                            .description("Automatically add LIMIT to SELECT queries")
                            .child(self.render_checkbox(
                                self.settings.query.auto_limit,
                                |this, value, cx| {
                                    this.settings.query.auto_limit = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
                    .child(
                        SettingRow::new("Auto-limit rows")
                            .description("Number of rows for auto-limit")
                            .child(self.render_number_input(
                                self.settings.query.auto_limit_rows,
                                100,
                                100_000,
                                |this, value, cx| {
                                    this.settings.query.auto_limit_rows = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
            )
    }

    fn render_connection_settings(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .max_w(px(600.0))
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .mb_4()
                    .child("Connection Defaults")
            )
            .child(
                SettingGroup::new("Timeouts")
                    .child(
                        SettingRow::new("Connection timeout")
                            .description("Timeout for establishing connection (seconds)")
                            .child(self.render_number_input(
                                self.settings.connections.connection_timeout_secs,
                                5,
                                120,
                                |this, value, cx| {
                                    this.settings.connections.connection_timeout_secs = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
            )
            .child(
                SettingGroup::new("Reconnection")
                    .child(
                        SettingRow::new("Auto-reconnect attempts")
                            .description("Number of reconnection attempts")
                            .child(self.render_number_input(
                                self.settings.connections.auto_reconnect_attempts,
                                0,
                                10,
                                |this, value, cx| {
                                    this.settings.connections.auto_reconnect_attempts = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
                    .child(
                        SettingRow::new("Keepalive interval")
                            .description("Send keepalive every N seconds (0 = disabled)")
                            .child(self.render_number_input(
                                self.settings.connections.keepalive_interval_secs,
                                0,
                                300,
                                |this, value, cx| {
                                    this.settings.connections.keepalive_interval_secs = value;
                                    this.save_settings(cx);
                                },
                            ))
                    )
            )
    }

    fn render_shortcuts_settings(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        // This would delegate to the ShortcutsSettings component
        ShortcutsSettings::new().into_any_element()
    }

    // Helper rendering methods
    fn render_theme_selector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Theme dropdown implementation
        div().child("Theme selector placeholder")
    }

    fn render_startup_selector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Startup behavior dropdown implementation
        div().child("Startup selector placeholder")
    }

    fn render_checkbox(
        &self,
        value: bool,
        on_change: impl Fn(&mut Self, bool, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .w_5()
            .h_5()
            .rounded_sm()
            .border_1()
            .border_color(if value { theme.primary } else { theme.border })
            .bg(if value { theme.primary } else { theme.bg_primary })
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(move |this, _, cx| {
                on_change(this, !value, cx);
            }))
            .when(value, |el| el.child("✓").text_color(theme.text_on_primary))
    }

    fn render_number_input(
        &self,
        value: u32,
        min: u32,
        max: u32,
        on_change: impl Fn(&mut Self, u32, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .child(
                div()
                    .w(px(80.0))
                    .px_2()
                    .py_1()
                    .bg(theme.bg_secondary)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .text_right()
                    .child(format!("{}", value))
            )
    }

    fn render_text_input(
        &self,
        value: &str,
        on_change: impl Fn(&mut Self, String, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .w(px(200.0))
            .px_2()
            .py_1()
            .bg(theme.bg_secondary)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .child(value.to_string())
    }
}

/// Setting group container
struct SettingGroup {
    title: String,
    children: Vec<AnyElement>,
}

impl SettingGroup {
    fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            children: Vec::new(),
        }
    }

    fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl Render for SettingGroup {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .text_base()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text_secondary)
                    .pb_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(self.title.clone())
            )
            .children(std::mem::take(&mut self.children))
    }
}

/// Individual setting row
struct SettingRow {
    label: String,
    description: Option<String>,
    control: Option<AnyElement>,
}

impl SettingRow {
    fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            description: None,
            control: None,
        }
    }

    fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    fn child(mut self, control: impl IntoElement) -> Self {
        self.control = Some(control.into_any_element());
        self
    }
}

impl Render for SettingRow {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_row()
            .justify_between()
            .items_center()
            .py_2()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_primary)
                            .child(self.label.clone())
                    )
                    .when_some(self.description.clone(), |el, desc| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(theme.text_tertiary)
                                .child(desc)
                        )
                    })
            )
            .when_some(self.control.take(), |el, control| el.child(control))
    }
}
```

### 29.9 Shortcuts Settings Component

**File: `src/ui/settings/shortcuts_settings.rs`**

```rust
use crate::models::shortcuts::{ShortcutConfig, Shortcut};
use crate::services::shortcuts::ShortcutService;
use crate::ui::theme::Theme;
use gpui::*;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ShortcutsSettings {
    config: ShortcutConfig,
    search_query: String,
    editing_action: Option<String>,
    new_binding: String,
    shortcut_service: Option<Arc<ShortcutService>>,
}

impl ShortcutsSettings {
    pub fn new() -> Self {
        Self {
            config: ShortcutConfig::default(),
            search_query: String::new(),
            editing_action: None,
            new_binding: String::new(),
            shortcut_service: None,
        }
    }

    pub fn with_service(mut self, service: Arc<ShortcutService>) -> Self {
        self.config = service.config();
        self.shortcut_service = Some(service);
        self
    }

    fn grouped_shortcuts(&self) -> HashMap<String, Vec<(&String, &Shortcut)>> {
        let mut groups: HashMap<String, Vec<(&String, &Shortcut)>> = HashMap::new();

        for (action, shortcut) in &self.config.shortcuts {
            if !self.search_query.is_empty() {
                let query = self.search_query.to_lowercase();
                if !shortcut.label.to_lowercase().contains(&query)
                    && !action.to_lowercase().contains(&query)
                    && !shortcut.effective_binding().to_lowercase().contains(&query)
                {
                    continue;
                }
            }

            groups
                .entry(shortcut.category.clone())
                .or_default()
                .push((action, shortcut));
        }

        groups
    }

    fn start_editing(&mut self, action: String, cx: &mut Context<Self>) {
        self.editing_action = Some(action);
        self.new_binding = String::new();
        cx.notify();
    }

    fn cancel_editing(&mut self, cx: &mut Context<Self>) {
        self.editing_action = None;
        self.new_binding = String::new();
        cx.notify();
    }

    fn save_binding(&mut self, cx: &mut Context<Self>) {
        if let Some(action) = self.editing_action.take() {
            if !self.new_binding.is_empty() {
                if let Some(shortcut) = self.config.shortcuts.get_mut(&action) {
                    shortcut.custom_binding = Some(self.new_binding.clone());
                }

                // Save to service
                if let Some(service) = &self.shortcut_service {
                    let service = service.clone();
                    let binding = self.new_binding.clone();
                    cx.spawn(|_, _| async move {
                        let _ = service.update_binding(&action, &binding).await;
                    }).detach();
                }
            }
        }
        self.new_binding = String::new();
        cx.notify();
    }

    fn reset_binding(&mut self, action: &str, cx: &mut Context<Self>) {
        if let Some(shortcut) = self.config.shortcuts.get_mut(action) {
            shortcut.custom_binding = None;
        }

        if let Some(service) = &self.shortcut_service {
            let service = service.clone();
            let action = action.to_string();
            cx.spawn(|_, _| async move {
                let _ = service.reset_binding(&action).await;
            }).detach();
        }

        cx.notify();
    }

    fn reset_all(&mut self, cx: &mut Context<Self>) {
        self.config = ShortcutConfig::default();

        if let Some(service) = &self.shortcut_service {
            let service = service.clone();
            cx.spawn(|_, _| async move {
                let _ = service.reset_all().await;
            }).detach();
        }

        cx.notify();
    }

    fn handle_key_event(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if self.editing_action.is_none() {
            return;
        }

        let mut parts = Vec::new();

        if event.keystroke.modifiers.command {
            parts.push("cmd");
        }
        if event.keystroke.modifiers.control {
            parts.push("ctrl");
        }
        if event.keystroke.modifiers.alt {
            parts.push("alt");
        }
        if event.keystroke.modifiers.shift {
            parts.push("shift");
        }

        let key = &event.keystroke.key;
        if !["Control", "Alt", "Shift", "Meta", "Command"].contains(&key.as_str()) {
            parts.push(key);
        }

        if !parts.is_empty() {
            self.new_binding = parts.join("-");
            cx.notify();
        }
    }
}

impl Render for ShortcutsSettings {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let grouped = self.grouped_shortcuts();

        div()
            .flex()
            .flex_col()
            .gap_6()
            .max_w(px(700.0))
            .on_key_down(cx.listener(|this, event, cx| {
                this.handle_key_event(event, cx);
            }))
            // Header
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Keyboard Shortcuts")
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(self.render_button("Export", cx))
                            .child(self.render_button("Import", cx))
                            .child(self.render_button_action("Reset All", cx.listener(|this, _, cx| {
                                this.reset_all(cx);
                            })))
                    )
            )
            // Search
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .bg(theme.bg_secondary)
                    .rounded_md()
                    .child("🔍")
                    .child(
                        div()
                            .flex_1()
                            .text_color(if self.search_query.is_empty() {
                                theme.text_tertiary
                            } else {
                                theme.text_primary
                            })
                            .child(if self.search_query.is_empty() {
                                "Search shortcuts...".to_string()
                            } else {
                                self.search_query.clone()
                            })
                    )
            )
            // Categories
            .children(grouped.into_iter().map(|(category, shortcuts)| {
                self.render_category(&category, shortcuts, cx)
            }))
    }
}

impl ShortcutsSettings {
    fn render_category(
        &mut self,
        category: &str,
        shortcuts: Vec<(&String, &Shortcut)>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text_secondary)
                    .pb_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(category.to_string())
            )
            .children(shortcuts.into_iter().map(|(action, shortcut)| {
                self.render_shortcut_row(action, shortcut, cx)
            }))
    }

    fn render_shortcut_row(
        &mut self,
        action: &str,
        shortcut: &Shortcut,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let is_editing = self.editing_action.as_ref() == Some(&action.to_string());
        let is_custom = shortcut.is_customized();
        let action_clone = action.to_string();

        div()
            .flex()
            .flex_row()
            .justify_between()
            .items_center()
            .px_3()
            .py_2()
            .bg(theme.bg_secondary)
            .rounded_md()
            .child(
                div()
                    .text_sm()
                    .child(shortcut.label.clone())
            )
            .child(
                if is_editing {
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .min_w(px(120.0))
                                .px_2()
                                .py_1()
                                .bg(theme.bg_primary)
                                .border_2()
                                .border_color(theme.primary)
                                .rounded_md()
                                .font_family("monospace")
                                .text_xs()
                                .text_center()
                                .text_color(if self.new_binding.is_empty() {
                                    theme.text_tertiary
                                } else {
                                    theme.text_primary
                                })
                                .child(if self.new_binding.is_empty() {
                                    "Press keys...".to_string()
                                } else {
                                    self.format_binding(&self.new_binding)
                                })
                        )
                        .child(self.render_button_action("Save", cx.listener(|this, _, cx| {
                            this.save_binding(cx);
                        })))
                        .child(self.render_button_action("Cancel", cx.listener(|this, _, cx| {
                            this.cancel_editing(cx);
                        })))
                        .into_any_element()
                } else {
                    let action_for_click = action_clone.clone();
                    let action_for_reset = action_clone.clone();

                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .bg(theme.bg_tertiary)
                                .border_1()
                                .border_color(if is_custom { theme.primary } else { theme.border })
                                .when(is_custom, |el| el.bg(theme.primary.opacity(0.1)))
                                .rounded_md()
                                .font_family("monospace")
                                .text_xs()
                                .cursor_pointer()
                                .hover(|s| s.border_color(theme.primary))
                                .on_click(cx.listener(move |this, _, cx| {
                                    this.start_editing(action_for_click.clone(), cx);
                                }))
                                .child(self.format_binding(shortcut.effective_binding()))
                        )
                        .when(is_custom, |el| {
                            el.child(
                                div()
                                    .p_1()
                                    .rounded_sm()
                                    .cursor_pointer()
                                    .text_color(theme.text_tertiary)
                                    .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                                    .on_click(cx.listener(move |this, _, cx| {
                                        this.reset_binding(&action_for_reset, cx);
                                    }))
                                    .child("↺")
                            )
                        })
                        .into_any_element()
                }
            )
    }

    fn format_binding(&self, binding: &str) -> String {
        // Format binding for display with platform-specific symbols
        let is_mac = cfg!(target_os = "macos");

        binding
            .split('-')
            .map(|part| match part.to_lowercase().as_str() {
                "cmd" | "command" => if is_mac { "⌘" } else { "Ctrl" },
                "ctrl" | "control" => if is_mac { "⌃" } else { "Ctrl" },
                "alt" | "option" => if is_mac { "⌥" } else { "Alt" },
                "shift" => if is_mac { "⇧" } else { "Shift" },
                "enter" | "return" => "↵",
                "backspace" => "⌫",
                "delete" => "⌦",
                "tab" => "⇥",
                "escape" | "esc" => "⎋",
                "up" => "↑",
                "down" => "↓",
                "left" => "←",
                "right" => "→",
                other => other,
            })
            .collect::<Vec<_>>()
            .join(if is_mac { "" } else { "+" })
    }

    fn render_button(&self, label: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .px_3()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .text_sm()
            .text_color(theme.text_secondary)
            .hover(|s| s.bg(theme.bg_hover))
            .child(label)
    }

    fn render_button_action(
        &self,
        label: &str,
        on_click: impl Fn(&ClickEvent, &mut App) + 'static,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .px_3()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .text_sm()
            .text_color(theme.text_secondary)
            .hover(|s| s.bg(theme.bg_hover))
            .on_click(on_click)
            .child(label)
    }
}
```

### 29.10 Performance Targets Verification

| Metric                            | Target     | Implementation                                         |
| --------------------------------- | ---------- | ------------------------------------------------------ |
| Cold start                        | < 1 second | Lazy loading, minimal initial setup, GPUI efficiency   |
| Memory (idle)                     | < 100 MB   | Efficient Rust memory, no JS runtime overhead          |
| Memory (1M rows)                  | < 500 MB   | Streaming + virtual scrolling, rows not held in memory |
| Query result render (1000 rows)   | < 100ms    | Virtual scrolling renders ~30 visible rows             |
| Schema browser load (1000 tables) | < 500ms    | Cached introspection, virtual tree                     |
| Autocomplete response             | < 50ms     | In-memory trie index from schema cache                 |

## Acceptance Criteria

1. **Keyboard Shortcuts**
   - [ ] All shortcuts from Appendix B implemented using GPUI actions
   - [ ] Platform-specific modifiers (Cmd on macOS, Ctrl elsewhere)
   - [ ] Customizable keybindings persisted to storage
   - [ ] Search/filter shortcuts
   - [ ] Reset to defaults
   - [ ] Import/export configuration

2. **Settings UI**
   - [ ] General settings (theme, language, startup)
   - [ ] Editor settings (font, tab size, display options)
   - [ ] Results settings (limits, formats, display)
   - [ ] Query settings (timeout, confirmations)
   - [ ] Connection defaults (SSL, timeout)
   - [ ] All settings persisted to SQLite

3. **Performance: Streaming**
   - [ ] Results streamed in batches of 1000
   - [ ] First batch rendered immediately
   - [ ] Background streaming continues via channel
   - [ ] Progress indicator for large results

4. **Performance: Caching**
   - [ ] Schema cached with 5-minute TTL
   - [ ] Incremental refresh on changes
   - [ ] Cache invalidation on DDL
   - [ ] Autocomplete from cache

5. **Performance: Virtualization**
   - [ ] Results grid uses virtual scrolling
   - [ ] Schema tree uses virtual list
   - [ ] Only visible rows rendered
   - [ ] Smooth scrolling at 60fps

6. **Performance Targets Met**
   - [ ] Cold start < 1 second
   - [ ] Idle memory < 100 MB
   - [ ] 1M row memory < 500 MB
   - [ ] 1000 row render < 100ms
   - [ ] Schema load < 500ms
   - [ ] Autocomplete < 50ms

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortcut_effective_binding() {
        let shortcut = Shortcut::new("Test", "Test Action", "cmd-t", "ctrl-t");

        // Default binding
        let binding = shortcut.effective_binding();
        if cfg!(target_os = "macos") {
            assert_eq!(binding, "cmd-t");
        } else {
            assert_eq!(binding, "ctrl-t");
        }

        // Custom binding
        let mut shortcut = shortcut;
        shortcut.custom_binding = Some("cmd-shift-t".to_string());
        assert_eq!(shortcut.effective_binding(), "cmd-shift-t");
    }

    #[test]
    fn test_schema_cache_ttl() {
        let cache = ConnectionSchemaCache::with_ttl(Duration::from_millis(100));

        cache.set_schemas(vec![SchemaInfo {
            name: "public".to_string(),
            owner: "postgres".to_string(),
        }]);

        // Should be valid immediately
        assert!(cache.get_schemas().is_some());

        // Should be invalid after TTL
        std::thread::sleep(Duration::from_millis(150));
        assert!(cache.get_schemas().is_none());
    }

    #[test]
    fn test_settings_defaults() {
        let settings = AppSettings::default();

        assert_eq!(settings.editor.font_size, 14.0);
        assert_eq!(settings.editor.tab_size, 2);
        assert!(settings.editor.show_line_numbers);
        assert_eq!(settings.query.default_statement_timeout_secs, 300);
        assert!(settings.query.confirm_ddl);
    }
}
```

### Performance Tests

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_streaming_batch_performance() {
        // Generate 10,000 mock rows
        let rows: Vec<Row> = (0..10_000)
            .map(|i| Row {
                values: vec![
                    CellValue::Integer(i),
                    CellValue::Text(format!("row_{}", i)),
                ],
            })
            .collect();

        let start = Instant::now();

        // Simulate batching
        let batch_size = 1000;
        let batches: Vec<&[Row]> = rows.chunks(batch_size).collect();

        let elapsed = start.elapsed();

        assert_eq!(batches.len(), 10);
        assert!(elapsed.as_millis() < 100, "Batching should be < 100ms");
    }

    #[test]
    fn test_cache_lookup_performance() {
        let cache = ConnectionSchemaCache::new();

        // Populate with 1000 tables
        for i in 0..1000 {
            cache.set_columns(
                &format!("public.table_{}", i),
                vec![ColumnInfo {
                    name: "id".to_string(),
                    data_type: "integer".to_string(),
                    nullable: false,
                    default: None,
                }],
            );
        }

        let start = Instant::now();

        // Perform 1000 lookups
        for i in 0..1000 {
            let _ = cache.get_columns(&format!("public.table_{}", i));
        }

        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() < 10, "1000 cache lookups should be < 10ms");
    }
}
```
