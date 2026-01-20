# Feature 29: Keyboard Shortcuts, Settings, and Performance

## Overview

This feature covers the comprehensive keyboard shortcuts system with customizable keybindings, the complete settings UI, and performance optimizations required to meet the application's targets. It includes result streaming, schema caching, UI virtualization, and memory management to ensure cold start under 1 second, idle memory under 100MB, and smooth handling of 1M+ row result sets.

## Goals

1. Full keyboard shortcut system with customization
2. Complete settings UI with all configuration categories
3. Result streaming with batch rendering
4. Schema caching and incremental refresh
5. Virtual scrolling for results and schema tree
6. Memory-efficient data handling
7. Meet all performance targets from design spec

## Dependencies

- Feature 01: Project Setup (Tauri + Svelte)
- Feature 02: Local Storage (settings persistence)
- Feature 06: Settings System (base implementation)
- Feature 11: Query Editor (Monaco keybindings)
- Feature 14: Results Grid (virtualization)
- Feature 27: Platform Integration (platform-specific shortcuts)

## Technical Specification

### 29.1 Keyboard Shortcuts System

**File: `src-tauri/src/models/shortcuts.rs`**

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
        map.insert("settings".into(), Shortcut::new("General", "Settings", "Mod+,"));
        map.insert("command_palette".into(), Shortcut::new("General", "Command Palette", "Mod+Shift+P"));
        map.insert("new_query".into(), Shortcut::new("General", "New Query Tab", "Mod+N"));
        map.insert("close_tab".into(), Shortcut::new("General", "Close Tab", "Mod+W"));
        map.insert("next_tab".into(), Shortcut::new("General", "Next Tab", "Mod+Shift+]|Ctrl+Tab"));
        map.insert("prev_tab".into(), Shortcut::new("General", "Previous Tab", "Mod+Shift+[|Ctrl+Shift+Tab"));
        map.insert("toggle_sidebar".into(), Shortcut::new("General", "Toggle Sidebar", "Mod+B"));

        // Editor
        map.insert("execute".into(), Shortcut::new("Editor", "Execute Statement", "Mod+Enter"));
        map.insert("execute_all".into(), Shortcut::new("Editor", "Execute All", "Mod+Shift+Enter"));
        map.insert("cancel".into(), Shortcut::new("Editor", "Cancel Query", "Mod+."));
        map.insert("format".into(), Shortcut::new("Editor", "Format SQL", "Mod+Shift+F"));
        map.insert("save".into(), Shortcut::new("Editor", "Save", "Mod+S"));
        map.insert("comment".into(), Shortcut::new("Editor", "Toggle Comment", "Mod+/"));
        map.insert("find".into(), Shortcut::new("Editor", "Find", "Mod+F"));
        map.insert("replace".into(), Shortcut::new("Editor", "Replace", "Mod+Alt+F|Mod+H"));
        map.insert("goto_line".into(), Shortcut::new("Editor", "Go to Line", "Mod+G"));
        map.insert("duplicate_line".into(), Shortcut::new("Editor", "Duplicate Line", "Mod+Shift+D"));
        map.insert("move_line_up".into(), Shortcut::new("Editor", "Move Line Up", "Alt+Up"));
        map.insert("move_line_down".into(), Shortcut::new("Editor", "Move Line Down", "Alt+Down"));

        // Results
        map.insert("copy".into(), Shortcut::new("Results", "Copy", "Mod+C"));
        map.insert("select_all".into(), Shortcut::new("Results", "Select All", "Mod+A"));
        map.insert("export".into(), Shortcut::new("Results", "Export", "Mod+E"));
        map.insert("edit_mode".into(), Shortcut::new("Results", "Toggle Edit Mode", "Mod+Shift+E"));

        // Navigation
        map.insert("focus_editor".into(), Shortcut::new("Navigation", "Focus Editor", "Mod+1"));
        map.insert("focus_results".into(), Shortcut::new("Navigation", "Focus Results", "Mod+2"));
        map.insert("focus_sidebar".into(), Shortcut::new("Navigation", "Focus Sidebar", "Mod+0"));
        map.insert("search_objects".into(), Shortcut::new("Navigation", "Search Objects", "Mod+P"));

        map
    }

    pub fn get_binding(&self, action: &str) -> Option<&str> {
        self.shortcuts.get(action).map(|s| s.binding.as_str())
    }
}

/// Individual shortcut definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shortcut {
    pub category: String,
    pub label: String,
    pub binding: String, // Mod+Key format, | for alternatives
    pub custom_binding: Option<String>,
    pub enabled: bool,
}

impl Shortcut {
    pub fn new(category: &str, label: &str, binding: &str) -> Self {
        Self {
            category: category.into(),
            label: label.into(),
            binding: binding.into(),
            custom_binding: None,
            enabled: true,
        }
    }

    /// Get the effective binding (custom or default)
    pub fn effective_binding(&self) -> &str {
        self.custom_binding.as_deref().unwrap_or(&self.binding)
    }
}
```

**File: `src-tauri/src/services/shortcuts.rs`**

```rust
use crate::models::shortcuts::{ShortcutConfig, Shortcut};
use crate::services::storage::StorageService;
use crate::error::Result;

const SHORTCUTS_KEY: &str = "shortcuts_config";

pub struct ShortcutService {
    storage: StorageService,
}

impl ShortcutService {
    pub fn new(storage: StorageService) -> Self {
        Self { storage }
    }

    /// Load shortcut configuration
    pub async fn load_config(&self) -> Result<ShortcutConfig> {
        match self.storage.get(SHORTCUTS_KEY).await? {
            Some(json) => {
                let config: ShortcutConfig = serde_json::from_str(&json)?;
                Ok(config)
            }
            None => Ok(ShortcutConfig::default()),
        }
    }

    /// Save shortcut configuration
    pub async fn save_config(&self, config: &ShortcutConfig) -> Result<()> {
        let json = serde_json::to_string(config)?;
        self.storage.set(SHORTCUTS_KEY, &json).await
    }

    /// Update a single shortcut binding
    pub async fn update_binding(&self, action: &str, binding: &str) -> Result<()> {
        let mut config = self.load_config().await?;

        if let Some(shortcut) = config.shortcuts.get_mut(action) {
            shortcut.custom_binding = Some(binding.to_string());
        }

        self.save_config(&config).await
    }

    /// Reset a shortcut to default
    pub async fn reset_binding(&self, action: &str) -> Result<()> {
        let mut config = self.load_config().await?;

        if let Some(shortcut) = config.shortcuts.get_mut(action) {
            shortcut.custom_binding = None;
        }

        self.save_config(&config).await
    }

    /// Reset all shortcuts to defaults
    pub async fn reset_all(&self) -> Result<()> {
        self.save_config(&ShortcutConfig::default()).await
    }

    /// Export shortcuts to JSON
    pub async fn export_config(&self) -> Result<String> {
        let config = self.load_config().await?;
        Ok(serde_json::to_string_pretty(&config)?)
    }

    /// Import shortcuts from JSON
    pub async fn import_config(&self, json: &str) -> Result<()> {
        let config: ShortcutConfig = serde_json::from_str(json)?;
        self.save_config(&config).await
    }
}
```

### 29.2 Settings Categories

**File: `src-tauri/src/models/settings.rs`**

```rust
use serde::{Deserialize, Serialize};

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
    pub theme: Theme,
    pub language: String,
    pub startup_behavior: StartupBehavior,
    pub auto_save_interval_secs: u32,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            language: "en".into(),
            startup_behavior: StartupBehavior::RestorePrevious,
            auto_save_interval_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StartupBehavior {
    RestorePrevious,
    StartFresh,
}

/// Editor settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    pub font_family: String,
    pub font_size: u32,
    pub tab_size: u32,
    pub use_spaces: bool,
    pub show_line_numbers: bool,
    pub show_minimap: bool,
    pub word_wrap: bool,
    pub autocomplete_delay_ms: u32,
    pub bracket_matching: bool,
    pub highlight_current_line: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_family: "JetBrains Mono, Menlo, Monaco, Consolas, monospace".into(),
            font_size: 14,
            tab_size: 2,
            use_spaces: true,
            show_line_numbers: true,
            show_minimap: false,
            word_wrap: false,
            autocomplete_delay_ms: 100,
            bracket_matching: true,
            highlight_current_line: true,
        }
    }
}

/// Results settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsSettings {
    pub default_row_limit: u32,
    pub date_format: String,
    pub time_format: String,
    pub number_locale: String,
    pub null_display: String,
    pub truncate_text_at: u32,
    pub copy_format: CopyFormat,
    pub row_height: u32,
}

impl Default for ResultsSettings {
    fn default() -> Self {
        Self {
            default_row_limit: 1000,
            date_format: "yyyy-MM-dd".into(),
            time_format: "HH:mm:ss".into(),
            number_locale: "en-US".into(),
            null_display: "NULL".into(),
            truncate_text_at: 500,
            copy_format: CopyFormat::Tsv,
            row_height: 32,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CopyFormat {
    Tsv,
    Csv,
    Json,
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
        }
    }
}

/// Connection defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSettings {
    pub default_ssl_mode: String,
    pub connection_timeout_secs: u32,
    pub auto_reconnect_attempts: u32,
    pub keepalive_interval_secs: u32,
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            default_ssl_mode: "prefer".into(),
            connection_timeout_secs: 30,
            auto_reconnect_attempts: 5,
            keepalive_interval_secs: 60,
        }
    }
}
```

### 29.3 Performance: Result Streaming

**File: `src-tauri/src/services/query_streaming.rs`**

```rust
use crate::models::query::{QueryResult, Row, Column};
use crate::error::Result;
use tokio_postgres::{Client, Row as PgRow, types::Type};
use tauri::AppHandle;
use std::sync::Arc;
use tokio::sync::Mutex;

const DEFAULT_BATCH_SIZE: usize = 1000;

pub struct QueryStreaming {
    app: AppHandle,
    batch_size: usize,
}

impl QueryStreaming {
    pub fn new(app: AppHandle, batch_size: Option<usize>) -> Self {
        Self {
            app,
            batch_size: batch_size.unwrap_or(DEFAULT_BATCH_SIZE),
        }
    }

    /// Execute query with streaming results
    pub async fn execute_streaming(
        &self,
        client: &Client,
        query_id: &str,
        sql: &str,
    ) -> Result<()> {
        let start = std::time::Instant::now();

        // Prepare and execute
        let statement = client.prepare(sql).await?;
        let columns: Vec<Column> = statement.columns().iter().map(|c| Column {
            name: c.name().to_string(),
            data_type: c.type_().name().to_string(),
            nullable: true, // Not available from statement
        }).collect();

        // Emit schema immediately
        self.app.emit(&format!("query:{}:columns", query_id), &columns)?;

        // Stream rows
        let row_stream = client.query_raw(&statement, &[] as &[&str]).await?;
        tokio::pin!(row_stream);

        let mut batch: Vec<Row> = Vec::with_capacity(self.batch_size);
        let mut total_rows = 0u64;
        let mut batch_num = 0u32;

        use futures::StreamExt;
        while let Some(result) = row_stream.next().await {
            let pg_row = result?;
            let row = self.convert_row(&pg_row, &columns)?;
            batch.push(row);
            total_rows += 1;

            if batch.len() >= self.batch_size {
                self.emit_batch(query_id, &batch, batch_num)?;
                batch_num += 1;
                batch.clear();
            }
        }

        // Emit final partial batch
        if !batch.is_empty() {
            self.emit_batch(query_id, &batch, batch_num)?;
        }

        // Emit completion
        let elapsed_ms = start.elapsed().as_millis() as u64;
        self.app.emit(&format!("query:{}:complete", query_id), serde_json::json!({
            "total_rows": total_rows,
            "elapsed_ms": elapsed_ms,
            "batches": batch_num + 1,
        }))?;

        Ok(())
    }

    fn emit_batch(&self, query_id: &str, rows: &[Row], batch_num: u32) -> Result<()> {
        self.app.emit(&format!("query:{}:rows", query_id), serde_json::json!({
            "batch_num": batch_num,
            "rows": rows,
        }))?;
        Ok(())
    }

    fn convert_row(&self, pg_row: &PgRow, columns: &[Column]) -> Result<Row> {
        let mut values = Vec::with_capacity(columns.len());

        for (i, col) in columns.iter().enumerate() {
            let value = self.extract_value(pg_row, i, &col.data_type)?;
            values.push(value);
        }

        Ok(Row { values })
    }

    fn extract_value(&self, row: &PgRow, idx: usize, type_name: &str) -> Result<serde_json::Value> {
        use serde_json::Value;

        // Check for NULL first
        if row.try_get::<_, Option<String>>(idx).ok().flatten().is_none()
            && row.try_get::<_, Option<i32>>(idx).ok().flatten().is_none()
        {
            return Ok(Value::Null);
        }

        // Type-specific extraction
        match type_name {
            "int2" => Ok(row.try_get::<_, i16>(idx).map(Value::from).unwrap_or(Value::Null)),
            "int4" => Ok(row.try_get::<_, i32>(idx).map(Value::from).unwrap_or(Value::Null)),
            "int8" => Ok(row.try_get::<_, i64>(idx).map(Value::from).unwrap_or(Value::Null)),
            "float4" => Ok(row.try_get::<_, f32>(idx).map(Value::from).unwrap_or(Value::Null)),
            "float8" => Ok(row.try_get::<_, f64>(idx).map(Value::from).unwrap_or(Value::Null)),
            "bool" => Ok(row.try_get::<_, bool>(idx).map(Value::from).unwrap_or(Value::Null)),
            "json" | "jsonb" => Ok(row.try_get::<_, Value>(idx).unwrap_or(Value::Null)),
            _ => Ok(row.try_get::<_, String>(idx).map(Value::from).unwrap_or(Value::Null)),
        }
    }
}
```

### 29.4 Performance: Schema Caching

**File: `src-tauri/src/services/schema_cache.rs`**

```rust
use crate::models::schema::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

const CACHE_TTL_SECS: u64 = 300; // 5 minutes

pub struct SchemaCache {
    tables: Arc<RwLock<CacheEntry<Vec<Table>>>>,
    columns: Arc<RwLock<HashMap<String, CacheEntry<Vec<Column>>>>>,
    indexes: Arc<RwLock<HashMap<String, CacheEntry<Vec<Index>>>>>,
    functions: Arc<RwLock<CacheEntry<Vec<Function>>>>,
}

struct CacheEntry<T> {
    data: T,
    fetched_at: Instant,
    ttl: Duration,
}

impl<T> CacheEntry<T> {
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
}

impl SchemaCache {
    pub fn new() -> Self {
        Self {
            tables: Arc::new(RwLock::new(CacheEntry::new(vec![], Duration::ZERO))),
            columns: Arc::new(RwLock::new(HashMap::new())),
            indexes: Arc::new(RwLock::new(HashMap::new())),
            functions: Arc::new(RwLock::new(CacheEntry::new(vec![], Duration::ZERO))),
        }
    }

    pub async fn get_tables(&self) -> Option<Vec<Table>> {
        let cache = self.tables.read().await;
        if cache.is_valid() {
            Some(cache.data.clone())
        } else {
            None
        }
    }

    pub async fn set_tables(&self, tables: Vec<Table>) {
        let mut cache = self.tables.write().await;
        *cache = CacheEntry::new(tables, Duration::from_secs(CACHE_TTL_SECS));
    }

    pub async fn get_columns(&self, table_key: &str) -> Option<Vec<Column>> {
        let cache = self.columns.read().await;
        cache.get(table_key).and_then(|entry| {
            if entry.is_valid() {
                Some(entry.data.clone())
            } else {
                None
            }
        })
    }

    pub async fn set_columns(&self, table_key: &str, columns: Vec<Column>) {
        let mut cache = self.columns.write().await;
        cache.insert(
            table_key.to_string(),
            CacheEntry::new(columns, Duration::from_secs(CACHE_TTL_SECS)),
        );
    }

    pub async fn invalidate(&self) {
        let mut tables = self.tables.write().await;
        *tables = CacheEntry::new(vec![], Duration::ZERO);

        let mut columns = self.columns.write().await;
        columns.clear();

        let mut indexes = self.indexes.write().await;
        indexes.clear();

        let mut functions = self.functions.write().await;
        *functions = CacheEntry::new(vec![], Duration::ZERO);
    }

    pub async fn invalidate_table(&self, table_key: &str) {
        let mut columns = self.columns.write().await;
        columns.remove(table_key);

        let mut indexes = self.indexes.write().await;
        indexes.remove(table_key);
    }
}

impl Default for SchemaCache {
    fn default() -> Self {
        Self::new()
    }
}
```

### 29.5 Svelte Frontend Components

#### Settings UI

**File: `src/lib/components/settings/SettingsPage.svelte`**

```svelte
<script lang="ts">
	import { settingsStore, type AppSettings } from '$lib/stores/settings';
	import GeneralSettings from './GeneralSettings.svelte';
	import EditorSettings from './EditorSettings.svelte';
	import ResultsSettings from './ResultsSettings.svelte';
	import QuerySettings from './QuerySettings.svelte';
	import ConnectionSettings from './ConnectionSettings.svelte';
	import ShortcutsSettings from './ShortcutsSettings.svelte';
	import { Settings, Code, Table, Play, Link, Keyboard } from 'lucide-svelte';

	let activeTab = 'general';

	const tabs = [
		{ id: 'general', label: 'General', icon: Settings },
		{ id: 'editor', label: 'Editor', icon: Code },
		{ id: 'results', label: 'Results', icon: Table },
		{ id: 'query', label: 'Query Execution', icon: Play },
		{ id: 'connections', label: 'Connections', icon: Link },
		{ id: 'shortcuts', label: 'Shortcuts', icon: Keyboard }
	];
</script>

<div class="settings-page">
	<div class="settings-sidebar">
		<h2>Settings</h2>
		<nav>
			{#each tabs as tab}
				<button
					class="tab-button"
					class:active={activeTab === tab.id}
					on:click={() => (activeTab = tab.id)}
				>
					<svelte:component this={tab.icon} size={18} />
					{tab.label}
				</button>
			{/each}
		</nav>
	</div>

	<div class="settings-content">
		{#if activeTab === 'general'}
			<GeneralSettings />
		{:else if activeTab === 'editor'}
			<EditorSettings />
		{:else if activeTab === 'results'}
			<ResultsSettings />
		{:else if activeTab === 'query'}
			<QuerySettings />
		{:else if activeTab === 'connections'}
			<ConnectionSettings />
		{:else if activeTab === 'shortcuts'}
			<ShortcutsSettings />
		{/if}
	</div>
</div>

<style>
	.settings-page {
		display: flex;
		height: 100%;
		background: var(--bg-primary);
	}

	.settings-sidebar {
		width: 240px;
		border-right: 1px solid var(--border-color);
		padding: 24px;
	}

	.settings-sidebar h2 {
		font-size: 18px;
		font-weight: 600;
		margin-bottom: 24px;
	}

	nav {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.tab-button {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 12px;
		background: none;
		border: none;
		border-radius: 6px;
		cursor: pointer;
		font-size: 14px;
		color: var(--text-secondary);
		text-align: left;
		transition: all 0.15s ease;
	}

	.tab-button:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}

	.tab-button.active {
		background: var(--primary-color);
		color: white;
	}

	.settings-content {
		flex: 1;
		padding: 24px 32px;
		overflow-y: auto;
	}
</style>
```

#### Keyboard Shortcuts Settings

**File: `src/lib/components/settings/ShortcutsSettings.svelte`**

```svelte
<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { platformStore, formatShortcut } from '$lib/stores/platform';
	import Button from '$lib/components/common/Button.svelte';
	import Input from '$lib/components/common/Input.svelte';
	import { Search, RotateCcw, Download, Upload } from 'lucide-svelte';

	interface Shortcut {
		category: string;
		label: string;
		binding: string;
		custom_binding: string | null;
		enabled: boolean;
	}

	interface ShortcutConfig {
		version: number;
		shortcuts: Record<string, Shortcut>;
	}

	let config: ShortcutConfig | null = null;
	let searchQuery = '';
	let editingAction: string | null = null;
	let newBinding = '';

	onMount(async () => {
		config = await invoke<ShortcutConfig>('load_shortcuts');
	});

	$: groupedShortcuts = config ? groupByCategory(Object.entries(config.shortcuts)) : {};
	$: filteredGroups = filterShortcuts(groupedShortcuts, searchQuery);

	function groupByCategory(entries: [string, Shortcut][]) {
		const groups: Record<string, [string, Shortcut][]> = {};
		for (const [action, shortcut] of entries) {
			if (!groups[shortcut.category]) {
				groups[shortcut.category] = [];
			}
			groups[shortcut.category].push([action, shortcut]);
		}
		return groups;
	}

	function filterShortcuts(groups: Record<string, [string, Shortcut][]>, query: string) {
		if (!query) return groups;
		const q = query.toLowerCase();

		const filtered: Record<string, [string, Shortcut][]> = {};
		for (const [category, shortcuts] of Object.entries(groups)) {
			const matches = shortcuts.filter(
				([action, s]) =>
					s.label.toLowerCase().includes(q) ||
					action.toLowerCase().includes(q) ||
					s.binding.toLowerCase().includes(q)
			);
			if (matches.length > 0) {
				filtered[category] = matches;
			}
		}
		return filtered;
	}

	function startEditing(action: string, currentBinding: string) {
		editingAction = action;
		newBinding = '';
	}

	function handleKeyDown(event: KeyboardEvent) {
		if (!editingAction) return;

		event.preventDefault();

		const parts: string[] = [];
		if (event.metaKey || event.ctrlKey) parts.push('Mod');
		if (event.altKey) parts.push('Alt');
		if (event.shiftKey) parts.push('Shift');

		const key = event.key;
		if (!['Meta', 'Control', 'Alt', 'Shift'].includes(key)) {
			parts.push(key.length === 1 ? key.toUpperCase() : key);
		}

		newBinding = parts.join('+');
	}

	async function saveBinding() {
		if (!editingAction || !newBinding) return;

		await invoke('update_shortcut', { action: editingAction, binding: newBinding });
		config = await invoke<ShortcutConfig>('load_shortcuts');
		editingAction = null;
		newBinding = '';
	}

	async function resetBinding(action: string) {
		await invoke('reset_shortcut', { action });
		config = await invoke<ShortcutConfig>('load_shortcuts');
	}

	async function resetAll() {
		await invoke('reset_all_shortcuts');
		config = await invoke<ShortcutConfig>('load_shortcuts');
	}

	async function exportConfig() {
		const json = await invoke<string>('export_shortcuts');
		const blob = new Blob([json], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = 'tusk-shortcuts.json';
		a.click();
		URL.revokeObjectURL(url);
	}

	async function importConfig() {
		const input = document.createElement('input');
		input.type = 'file';
		input.accept = '.json';
		input.onchange = async () => {
			const file = input.files?.[0];
			if (!file) return;

			const json = await file.text();
			await invoke('import_shortcuts', { json });
			config = await invoke<ShortcutConfig>('load_shortcuts');
		};
		input.click();
	}

	function cancelEditing() {
		editingAction = null;
		newBinding = '';
	}
</script>

<svelte:window on:keydown={handleKeyDown} />

<div class="shortcuts-settings">
	<div class="header">
		<h3>Keyboard Shortcuts</h3>
		<div class="actions">
			<Button variant="ghost" size="sm" on:click={exportConfig}>
				<Download size={14} />
				Export
			</Button>
			<Button variant="ghost" size="sm" on:click={importConfig}>
				<Upload size={14} />
				Import
			</Button>
			<Button variant="ghost" size="sm" on:click={resetAll}>
				<RotateCcw size={14} />
				Reset All
			</Button>
		</div>
	</div>

	<div class="search">
		<Search size={16} />
		<Input bind:value={searchQuery} placeholder="Search shortcuts..." />
	</div>

	<div class="shortcuts-list">
		{#each Object.entries(filteredGroups) as [category, shortcuts]}
			<div class="category">
				<h4>{category}</h4>
				<div class="shortcuts">
					{#each shortcuts as [action, shortcut]}
						<div class="shortcut-row">
							<span class="label">{shortcut.label}</span>

							{#if editingAction === action}
								<div class="editing">
									<kbd class="binding-input" class:empty={!newBinding}>
										{newBinding || 'Press keys...'}
									</kbd>
									<Button variant="primary" size="xs" on:click={saveBinding} disabled={!newBinding}>
										Save
									</Button>
									<Button variant="ghost" size="xs" on:click={cancelEditing}>Cancel</Button>
								</div>
							{:else}
								<div class="binding-display">
									<kbd
										class="binding"
										class:custom={shortcut.custom_binding}
										on:click={() =>
											startEditing(action, shortcut.custom_binding || shortcut.binding)}
									>
										{formatShortcut(shortcut.custom_binding || shortcut.binding)}
									</kbd>

									{#if shortcut.custom_binding}
										<button
											class="reset-btn"
											on:click={() => resetBinding(action)}
											title="Reset to default"
										>
											<RotateCcw size={12} />
										</button>
									{/if}
								</div>
							{/if}
						</div>
					{/each}
				</div>
			</div>
		{/each}
	</div>
</div>

<style>
	.shortcuts-settings {
		max-width: 700px;
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 24px;
	}

	.header h3 {
		font-size: 20px;
		font-weight: 600;
	}

	.actions {
		display: flex;
		gap: 8px;
	}

	.search {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 24px;
		padding: 8px 12px;
		background: var(--bg-secondary);
		border-radius: 6px;
	}

	.search :global(input) {
		flex: 1;
		background: none;
		border: none;
	}

	.category {
		margin-bottom: 24px;
	}

	.category h4 {
		font-size: 14px;
		font-weight: 600;
		color: var(--text-secondary);
		margin-bottom: 12px;
		padding-bottom: 8px;
		border-bottom: 1px solid var(--border-color);
	}

	.shortcuts {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.shortcut-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px 12px;
		background: var(--bg-secondary);
		border-radius: 6px;
	}

	.label {
		font-size: 14px;
	}

	.binding-display {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	kbd.binding {
		padding: 4px 8px;
		background: var(--bg-tertiary);
		border: 1px solid var(--border-color);
		border-radius: 4px;
		font-family: var(--font-mono);
		font-size: 12px;
		cursor: pointer;
		transition: all 0.15s ease;
	}

	kbd.binding:hover {
		background: var(--bg-hover);
		border-color: var(--primary-color);
	}

	kbd.binding.custom {
		border-color: var(--primary-color);
		background: color-mix(in srgb, var(--primary-color) 10%, var(--bg-tertiary));
	}

	.editing {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.binding-input {
		min-width: 120px;
		padding: 4px 8px;
		background: var(--bg-primary);
		border: 2px solid var(--primary-color);
		border-radius: 4px;
		font-family: var(--font-mono);
		font-size: 12px;
		text-align: center;
	}

	.binding-input.empty {
		color: var(--text-tertiary);
	}

	.reset-btn {
		padding: 4px;
		background: none;
		border: none;
		cursor: pointer;
		color: var(--text-tertiary);
		border-radius: 4px;
	}

	.reset-btn:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}
</style>
```

### 29.6 Tauri Commands

**File: `src-tauri/src/commands/shortcuts.rs`**

```rust
use crate::models::shortcuts::ShortcutConfig;
use crate::services::shortcuts::ShortcutService;
use crate::state::AppState;
use crate::error::Result;
use tauri::State;

/// Load shortcuts configuration
#[tauri::command]
pub async fn load_shortcuts(
    state: State<'_, AppState>,
) -> Result<ShortcutConfig> {
    let service = state.shortcut_service.lock().await;
    service.load_config().await
}

/// Update a single shortcut binding
#[tauri::command]
pub async fn update_shortcut(
    state: State<'_, AppState>,
    action: String,
    binding: String,
) -> Result<()> {
    let service = state.shortcut_service.lock().await;
    service.update_binding(&action, &binding).await
}

/// Reset a shortcut to default
#[tauri::command]
pub async fn reset_shortcut(
    state: State<'_, AppState>,
    action: String,
) -> Result<()> {
    let service = state.shortcut_service.lock().await;
    service.reset_binding(&action).await
}

/// Reset all shortcuts to defaults
#[tauri::command]
pub async fn reset_all_shortcuts(
    state: State<'_, AppState>,
) -> Result<()> {
    let service = state.shortcut_service.lock().await;
    service.reset_all().await
}

/// Export shortcuts to JSON
#[tauri::command]
pub async fn export_shortcuts(
    state: State<'_, AppState>,
) -> Result<String> {
    let service = state.shortcut_service.lock().await;
    service.export_config().await
}

/// Import shortcuts from JSON
#[tauri::command]
pub async fn import_shortcuts(
    state: State<'_, AppState>,
    json: String,
) -> Result<()> {
    let service = state.shortcut_service.lock().await;
    service.import_config(&json).await
}
```

**File: `src-tauri/src/commands/settings.rs`**

```rust
use crate::models::settings::AppSettings;
use crate::services::storage::StorageService;
use crate::state::AppState;
use crate::error::Result;
use tauri::State;

const SETTINGS_KEY: &str = "app_settings";

/// Load application settings
#[tauri::command]
pub async fn load_settings(
    state: State<'_, AppState>,
) -> Result<AppSettings> {
    let storage = state.storage.lock().await;

    match storage.get(SETTINGS_KEY).await? {
        Some(json) => Ok(serde_json::from_str(&json)?),
        None => Ok(AppSettings::default()),
    }
}

/// Save application settings
#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<()> {
    let storage = state.storage.lock().await;
    let json = serde_json::to_string(&settings)?;
    storage.set(SETTINGS_KEY, &json).await
}

/// Reset settings to defaults
#[tauri::command]
pub async fn reset_settings(
    state: State<'_, AppState>,
) -> Result<AppSettings> {
    let storage = state.storage.lock().await;
    let settings = AppSettings::default();
    let json = serde_json::to_string(&settings)?;
    storage.set(SETTINGS_KEY, &json).await?;
    Ok(settings)
}
```

### 29.7 Performance Targets Verification

| Metric                            | Target     | Implementation                                         |
| --------------------------------- | ---------- | ------------------------------------------------------ |
| Cold start                        | < 1 second | Lazy loading, minimal initial bundle                   |
| Memory (idle)                     | < 100 MB   | Efficient Rust memory, no retained data                |
| Memory (1M rows)                  | < 500 MB   | Streaming + virtual scrolling, rows not held in memory |
| Query result render (1000 rows)   | < 100ms    | Virtual scrolling renders ~50 visible rows             |
| Schema browser load (1000 tables) | < 500ms    | Cached introspection, virtual tree                     |
| Autocomplete response             | < 50ms     | In-memory trie index from schema cache                 |

## Acceptance Criteria

1. **Keyboard Shortcuts**
   - [ ] All shortcuts from Appendix B implemented
   - [ ] Platform-specific modifiers (Cmd/Ctrl)
   - [ ] Customizable keybindings
   - [ ] Search/filter shortcuts
   - [ ] Reset to defaults
   - [ ] Import/export configuration

2. **Settings UI**
   - [ ] General settings (theme, language, startup)
   - [ ] Editor settings (font, tab size, minimap)
   - [ ] Results settings (limits, formats, display)
   - [ ] Query settings (timeout, confirmations)
   - [ ] Connection defaults (SSL, timeout)

3. **Performance: Streaming**
   - [ ] Results streamed in batches of 1000
   - [ ] First batch rendered immediately
   - [ ] Background streaming continues
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

## MCP Testing Instructions

### Using Tauri MCP

```typescript
// Test keyboard shortcuts and settings
await driver_session({ action: 'start', port: 9223 });

// Open settings with keyboard
await webview_keyboard({ action: 'press', key: 'Meta+,' });
await webview_wait_for({ type: 'text', value: 'Settings' });

// Navigate to shortcuts
await webview_click({ selector: '[data-tab="shortcuts"]' });
await webview_wait_for({ type: 'text', value: 'Keyboard Shortcuts' });

// Screenshot settings page
await webview_screenshot({ filePath: 'settings-shortcuts.png' });

// Test shortcut customization
await webview_click({ selector: '[data-action="execute"] kbd' });

// Press new shortcut
await webview_keyboard({ action: 'down', key: 'Meta' });
await webview_keyboard({ action: 'down', key: 'Shift' });
await webview_keyboard({ action: 'press', key: 'e' });
await webview_keyboard({ action: 'up', key: 'Shift' });
await webview_keyboard({ action: 'up', key: 'Meta' });

// Save
await webview_click({ selector: '[data-testid="save-shortcut"]' });

await driver_session({ action: 'stop' });
```

### Performance Testing

```typescript
// Test streaming performance
await driver_session({ action: 'start', port: 9223 });

// Execute large query
await webview_keyboard({
	action: 'type',
	selector: '.monaco-editor textarea',
	text: 'SELECT * FROM generate_series(1, 100000)'
});

const start = Date.now();
await webview_keyboard({ action: 'press', key: 'Meta+Enter' });

// Wait for first batch
await webview_wait_for({ type: 'selector', value: '.results-row' });
const firstBatch = Date.now() - start;
console.log('First batch rendered in:', firstBatch, 'ms');

// Wait for completion
await webview_wait_for({ type: 'text', value: '100,000 rows' });
const total = Date.now() - start;
console.log('Total time:', total, 'ms');

await driver_session({ action: 'stop' });
```
