# Feature 13: Tab Management and Query History

## Overview

Tab management provides a multi-document interface for working with multiple queries and database objects simultaneously. Query history tracks all executed queries for easy retrieval and re-execution. Both features integrate with local storage for persistence across sessions.

## Goals

- Support multiple query tabs with independent state
- Persist tab state across application restarts
- Track all executed queries with timing and results
- Enable query history search, filtering, and favorites
- Save queries as snippets for reuse
- Provide tab context menu and keyboard navigation

## Dependencies

- Feature 05: Local Storage (SQLite persistence)
- Feature 11: Query Execution (query results and timing)
- Feature 12: Monaco Editor (editor content)

## Technical Specification

### 13.1 Tab State Models

```rust
// src-tauri/src/models/tabs.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorTab {
    pub id: Uuid,
    pub connection_id: Option<Uuid>,
    pub tab_type: TabType,
    pub title: String,
    pub content: TabContent,
    pub is_modified: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TabType {
    Query,
    TableData,
    ViewData,
    FunctionEditor,
    QueryPlan,
    SchemaViewer,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TabContent {
    #[serde(rename = "query")]
    Query {
        sql: String,
        cursor_position: Option<CursorPosition>,
        selection: Option<SelectionRange>,
    },
    #[serde(rename = "table")]
    TableData {
        schema: String,
        table: String,
        filters: Vec<TableFilter>,
        sort: Option<TableSort>,
        page: u32,
    },
    #[serde(rename = "function")]
    FunctionEditor {
        schema: String,
        function_name: String,
        source: String,
    },
    #[serde(rename = "plan")]
    QueryPlan {
        sql: String,
        plan_json: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelectionRange {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableFilter {
    pub column: String,
    pub operator: FilterOperator,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Like,
    ILike,
    In,
    IsNull,
    IsNotNull,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableSort {
    pub column: String,
    pub direction: SortDirection,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    Desc,
}
```

### 13.2 Query History Models

```rust
// src-tauri/src/models/history.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryHistoryEntry {
    pub id: i64,
    pub connection_id: Uuid,
    pub connection_name: String,
    pub sql: String,
    pub executed_at: DateTime<Utc>,
    pub duration_ms: Option<u64>,
    pub rows_affected: Option<u64>,
    pub error: Option<String>,
    pub favorited: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedQuery {
    pub id: Uuid,
    pub connection_id: Option<Uuid>,  // None = global
    pub name: String,
    pub description: Option<String>,
    pub sql: String,
    pub folder_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedQueryFolder {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistorySearchParams {
    pub connection_id: Option<Uuid>,
    pub search_text: Option<String>,
    pub favorites_only: bool,
    pub errors_only: bool,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub limit: u32,
    pub offset: u32,
}
```

### 13.3 Tab Service (Rust)

```rust
// src-tauri/src/services/tabs.rs

use uuid::Uuid;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{Error, Result};
use crate::models::tabs::{EditorTab, TabType, TabContent};
use crate::services::storage::StorageService;

pub struct TabService {
    storage: Arc<StorageService>,
    active_tab_id: RwLock<Option<Uuid>>,
}

impl TabService {
    pub fn new(storage: Arc<StorageService>) -> Self {
        Self {
            storage,
            active_tab_id: RwLock::new(None),
        }
    }

    /// Create a new query tab
    pub async fn create_query_tab(
        &self,
        connection_id: Option<Uuid>,
        title: Option<String>,
        sql: Option<String>,
    ) -> Result<EditorTab> {
        let tabs = self.storage.get_all_tabs().await?;
        let next_num = tabs.iter()
            .filter(|t| t.tab_type == TabType::Query)
            .filter(|t| t.title.starts_with("Query "))
            .filter_map(|t| t.title.strip_prefix("Query ").and_then(|n| n.parse::<u32>().ok()))
            .max()
            .unwrap_or(0) + 1;

        let tab = EditorTab {
            id: Uuid::new_v4(),
            connection_id,
            tab_type: TabType::Query,
            title: title.unwrap_or_else(|| format!("Query {}", next_num)),
            content: TabContent::Query {
                sql: sql.unwrap_or_default(),
                cursor_position: None,
                selection: None,
            },
            is_modified: false,
            sort_order: tabs.len() as i32,
            created_at: Utc::now(),
            last_accessed_at: Utc::now(),
        };

        self.storage.save_tab(&tab).await?;
        self.set_active_tab(tab.id).await?;

        Ok(tab)
    }

    /// Create a table data viewer tab
    pub async fn create_table_tab(
        &self,
        connection_id: Uuid,
        schema: String,
        table: String,
    ) -> Result<EditorTab> {
        // Check if tab already exists for this table
        let tabs = self.storage.get_all_tabs().await?;
        for tab in &tabs {
            if let TabContent::TableData { schema: s, table: t, .. } = &tab.content {
                if s == &schema && t == &table && tab.connection_id == Some(connection_id) {
                    // Tab exists, just activate it
                    self.set_active_tab(tab.id).await?;
                    return Ok(tab.clone());
                }
            }
        }

        let tab = EditorTab {
            id: Uuid::new_v4(),
            connection_id: Some(connection_id),
            tab_type: TabType::TableData,
            title: format!("{}.{}", schema, table),
            content: TabContent::TableData {
                schema,
                table,
                filters: Vec::new(),
                sort: None,
                page: 1,
            },
            is_modified: false,
            sort_order: tabs.len() as i32,
            created_at: Utc::now(),
            last_accessed_at: Utc::now(),
        };

        self.storage.save_tab(&tab).await?;
        self.set_active_tab(tab.id).await?;

        Ok(tab)
    }

    /// Get all tabs
    pub async fn get_all_tabs(&self) -> Result<Vec<EditorTab>> {
        self.storage.get_all_tabs().await
    }

    /// Get active tab ID
    pub async fn get_active_tab_id(&self) -> Option<Uuid> {
        *self.active_tab_id.read().await
    }

    /// Set active tab
    pub async fn set_active_tab(&self, tab_id: Uuid) -> Result<()> {
        let mut active = self.active_tab_id.write().await;
        *active = Some(tab_id);

        // Update last_accessed_at
        self.storage.update_tab_accessed(tab_id).await?;
        self.storage.set_active_tab(tab_id).await?;

        Ok(())
    }

    /// Update tab content
    pub async fn update_tab_content(
        &self,
        tab_id: Uuid,
        content: TabContent,
        is_modified: bool,
    ) -> Result<()> {
        self.storage.update_tab_content(tab_id, content, is_modified).await
    }

    /// Update tab title
    pub async fn rename_tab(&self, tab_id: Uuid, new_title: String) -> Result<()> {
        self.storage.rename_tab(tab_id, new_title).await
    }

    /// Close a tab
    pub async fn close_tab(&self, tab_id: Uuid) -> Result<Option<Uuid>> {
        let tabs = self.storage.get_all_tabs().await?;
        let tab_index = tabs.iter().position(|t| t.id == tab_id);

        self.storage.delete_tab(tab_id).await?;

        // If this was the active tab, activate an adjacent tab
        let active = *self.active_tab_id.read().await;
        if active == Some(tab_id) {
            if let Some(idx) = tab_index {
                let remaining_tabs: Vec<_> = tabs.iter()
                    .filter(|t| t.id != tab_id)
                    .collect();

                if !remaining_tabs.is_empty() {
                    let new_active_idx = idx.min(remaining_tabs.len() - 1);
                    let new_active_id = remaining_tabs[new_active_idx].id;
                    self.set_active_tab(new_active_id).await?;
                    return Ok(Some(new_active_id));
                } else {
                    let mut active = self.active_tab_id.write().await;
                    *active = None;
                }
            }
        }

        Ok(None)
    }

    /// Close all tabs except the specified one
    pub async fn close_other_tabs(&self, keep_tab_id: Uuid) -> Result<()> {
        let tabs = self.storage.get_all_tabs().await?;

        for tab in tabs {
            if tab.id != keep_tab_id {
                self.storage.delete_tab(tab.id).await?;
            }
        }

        self.set_active_tab(keep_tab_id).await?;
        Ok(())
    }

    /// Close tabs to the right of the specified tab
    pub async fn close_tabs_to_right(&self, tab_id: Uuid) -> Result<()> {
        let tabs = self.storage.get_all_tabs().await?;
        let tab_index = tabs.iter().position(|t| t.id == tab_id);

        if let Some(idx) = tab_index {
            for tab in tabs.iter().skip(idx + 1) {
                self.storage.delete_tab(tab.id).await?;
            }
        }

        Ok(())
    }

    /// Reorder tabs
    pub async fn reorder_tabs(&self, tab_ids: Vec<Uuid>) -> Result<()> {
        for (order, tab_id) in tab_ids.iter().enumerate() {
            self.storage.update_tab_order(*tab_id, order as i32).await?;
        }
        Ok(())
    }

    /// Restore tabs from previous session
    pub async fn restore_session(&self) -> Result<(Vec<EditorTab>, Option<Uuid>)> {
        let tabs = self.storage.get_all_tabs().await?;
        let active_id = self.storage.get_active_tab_id().await?;

        if let Some(id) = active_id {
            let mut active = self.active_tab_id.write().await;
            *active = Some(id);
        }

        Ok((tabs, active_id))
    }
}
```

### 13.4 History Service (Rust)

```rust
// src-tauri/src/services/history.rs

use uuid::Uuid;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::models::history::{QueryHistoryEntry, SavedQuery, SavedQueryFolder, HistorySearchParams};
use crate::services::storage::StorageService;

pub struct HistoryService {
    storage: Arc<StorageService>,
}

impl HistoryService {
    pub fn new(storage: Arc<StorageService>) -> Self {
        Self { storage }
    }

    /// Record a query execution in history
    pub async fn record_query(
        &self,
        connection_id: Uuid,
        sql: &str,
        duration_ms: u64,
        rows_affected: Option<u64>,
        error: Option<String>,
    ) -> Result<i64> {
        self.storage.insert_query_history(
            connection_id,
            sql,
            duration_ms,
            rows_affected,
            error,
        ).await
    }

    /// Search query history
    pub async fn search_history(
        &self,
        params: HistorySearchParams,
    ) -> Result<Vec<QueryHistoryEntry>> {
        self.storage.search_query_history(params).await
    }

    /// Get recent queries for a connection
    pub async fn get_recent_queries(
        &self,
        connection_id: Uuid,
        limit: u32,
    ) -> Result<Vec<QueryHistoryEntry>> {
        self.search_history(HistorySearchParams {
            connection_id: Some(connection_id),
            search_text: None,
            favorites_only: false,
            errors_only: false,
            from_date: None,
            to_date: None,
            limit,
            offset: 0,
        }).await
    }

    /// Toggle favorite status for a history entry
    pub async fn toggle_favorite(&self, history_id: i64) -> Result<bool> {
        self.storage.toggle_history_favorite(history_id).await
    }

    /// Delete history entries
    pub async fn delete_history(&self, ids: Vec<i64>) -> Result<()> {
        for id in ids {
            self.storage.delete_history_entry(id).await?;
        }
        Ok(())
    }

    /// Clear all history for a connection
    pub async fn clear_connection_history(&self, connection_id: Uuid) -> Result<()> {
        self.storage.clear_connection_history(connection_id).await
    }

    /// Save a query as a snippet
    pub async fn save_query(&self, query: SavedQuery) -> Result<Uuid> {
        self.storage.insert_saved_query(&query).await?;
        Ok(query.id)
    }

    /// Get all saved queries
    pub async fn get_saved_queries(
        &self,
        connection_id: Option<Uuid>,
    ) -> Result<Vec<SavedQuery>> {
        self.storage.get_saved_queries(connection_id).await
    }

    /// Update a saved query
    pub async fn update_saved_query(&self, query: SavedQuery) -> Result<()> {
        self.storage.update_saved_query(&query).await
    }

    /// Delete a saved query
    pub async fn delete_saved_query(&self, query_id: Uuid) -> Result<()> {
        self.storage.delete_saved_query(query_id).await
    }

    /// Create a folder for saved queries
    pub async fn create_folder(&self, folder: SavedQueryFolder) -> Result<Uuid> {
        self.storage.insert_saved_query_folder(&folder).await?;
        Ok(folder.id)
    }

    /// Get all folders
    pub async fn get_folders(&self) -> Result<Vec<SavedQueryFolder>> {
        self.storage.get_saved_query_folders().await
    }
}
```

### 13.5 IPC Commands

```rust
// src-tauri/src/commands/tabs.rs

use tauri::State;
use uuid::Uuid;

use crate::error::Result;
use crate::models::tabs::{EditorTab, TabContent};
use crate::state::AppState;

#[tauri::command]
pub async fn create_query_tab(
    state: State<'_, AppState>,
    connection_id: Option<String>,
    title: Option<String>,
    sql: Option<String>,
) -> Result<EditorTab> {
    let conn_id = connection_id
        .map(|id| Uuid::parse_str(&id))
        .transpose()?;

    state.tab_service.create_query_tab(conn_id, title, sql).await
}

#[tauri::command]
pub async fn create_table_tab(
    state: State<'_, AppState>,
    connection_id: String,
    schema: String,
    table: String,
) -> Result<EditorTab> {
    let conn_id = Uuid::parse_str(&connection_id)?;
    state.tab_service.create_table_tab(conn_id, schema, table).await
}

#[tauri::command]
pub async fn get_all_tabs(state: State<'_, AppState>) -> Result<Vec<EditorTab>> {
    state.tab_service.get_all_tabs().await
}

#[tauri::command]
pub async fn set_active_tab(
    state: State<'_, AppState>,
    tab_id: String,
) -> Result<()> {
    let id = Uuid::parse_str(&tab_id)?;
    state.tab_service.set_active_tab(id).await
}

#[tauri::command]
pub async fn update_tab_content(
    state: State<'_, AppState>,
    tab_id: String,
    content: TabContent,
    is_modified: bool,
) -> Result<()> {
    let id = Uuid::parse_str(&tab_id)?;
    state.tab_service.update_tab_content(id, content, is_modified).await
}

#[tauri::command]
pub async fn rename_tab(
    state: State<'_, AppState>,
    tab_id: String,
    new_title: String,
) -> Result<()> {
    let id = Uuid::parse_str(&tab_id)?;
    state.tab_service.rename_tab(id, new_title).await
}

#[tauri::command]
pub async fn close_tab(
    state: State<'_, AppState>,
    tab_id: String,
) -> Result<Option<String>> {
    let id = Uuid::parse_str(&tab_id)?;
    let new_active = state.tab_service.close_tab(id).await?;
    Ok(new_active.map(|id| id.to_string()))
}

#[tauri::command]
pub async fn close_other_tabs(
    state: State<'_, AppState>,
    keep_tab_id: String,
) -> Result<()> {
    let id = Uuid::parse_str(&keep_tab_id)?;
    state.tab_service.close_other_tabs(id).await
}

#[tauri::command]
pub async fn reorder_tabs(
    state: State<'_, AppState>,
    tab_ids: Vec<String>,
) -> Result<()> {
    let ids: Vec<Uuid> = tab_ids
        .iter()
        .map(|id| Uuid::parse_str(id))
        .collect::<std::result::Result<Vec<_>, _>>()?;

    state.tab_service.reorder_tabs(ids).await
}

#[tauri::command]
pub async fn restore_session(
    state: State<'_, AppState>,
) -> Result<(Vec<EditorTab>, Option<String>)> {
    let (tabs, active_id) = state.tab_service.restore_session().await?;
    Ok((tabs, active_id.map(|id| id.to_string())))
}
```

```rust
// src-tauri/src/commands/history.rs

use tauri::State;
use uuid::Uuid;

use crate::error::Result;
use crate::models::history::{QueryHistoryEntry, SavedQuery, SavedQueryFolder, HistorySearchParams};
use crate::state::AppState;

#[tauri::command]
pub async fn search_history(
    state: State<'_, AppState>,
    params: HistorySearchParams,
) -> Result<Vec<QueryHistoryEntry>> {
    state.history_service.search_history(params).await
}

#[tauri::command]
pub async fn get_recent_queries(
    state: State<'_, AppState>,
    connection_id: String,
    limit: u32,
) -> Result<Vec<QueryHistoryEntry>> {
    let conn_id = Uuid::parse_str(&connection_id)?;
    state.history_service.get_recent_queries(conn_id, limit).await
}

#[tauri::command]
pub async fn toggle_history_favorite(
    state: State<'_, AppState>,
    history_id: i64,
) -> Result<bool> {
    state.history_service.toggle_favorite(history_id).await
}

#[tauri::command]
pub async fn delete_history_entries(
    state: State<'_, AppState>,
    ids: Vec<i64>,
) -> Result<()> {
    state.history_service.delete_history(ids).await
}

#[tauri::command]
pub async fn save_query_snippet(
    state: State<'_, AppState>,
    query: SavedQuery,
) -> Result<String> {
    let id = state.history_service.save_query(query).await?;
    Ok(id.to_string())
}

#[tauri::command]
pub async fn get_saved_queries(
    state: State<'_, AppState>,
    connection_id: Option<String>,
) -> Result<Vec<SavedQuery>> {
    let conn_id = connection_id
        .map(|id| Uuid::parse_str(&id))
        .transpose()?;

    state.history_service.get_saved_queries(conn_id).await
}

#[tauri::command]
pub async fn delete_saved_query(
    state: State<'_, AppState>,
    query_id: String,
) -> Result<()> {
    let id = Uuid::parse_str(&query_id)?;
    state.history_service.delete_saved_query(id).await
}
```

### 13.6 Frontend Tab Store

```typescript
// src/lib/stores/tabs.svelte.ts

import { invoke } from '@tauri-apps/api/core';

export interface EditorTab {
	id: string;
	connection_id: string | null;
	tab_type: 'query' | 'table_data' | 'view_data' | 'function_editor' | 'query_plan';
	title: string;
	content: TabContent;
	is_modified: boolean;
	sort_order: number;
}

export type TabContent =
	| { type: 'query'; sql: string; cursor_position?: CursorPosition; selection?: SelectionRange }
	| {
			type: 'table';
			schema: string;
			table: string;
			filters: TableFilter[];
			sort?: TableSort;
			page: number;
	  }
	| { type: 'function'; schema: string; function_name: string; source: string }
	| { type: 'plan'; sql: string; plan_json?: string };

export interface CursorPosition {
	line: number;
	column: number;
}

export interface SelectionRange {
	start_line: number;
	start_column: number;
	end_line: number;
	end_column: number;
}

export interface TableFilter {
	column: string;
	operator: string;
	value: string;
}

export interface TableSort {
	column: string;
	direction: 'asc' | 'desc';
}

class TabStore {
	tabs = $state<EditorTab[]>([]);
	activeTabId = $state<string | null>(null);
	isLoading = $state(false);

	get activeTab(): EditorTab | undefined {
		return this.tabs.find((t) => t.id === this.activeTabId);
	}

	async init() {
		this.isLoading = true;
		try {
			const [tabs, activeId] = await invoke<[EditorTab[], string | null]>('restore_session');
			this.tabs = tabs.sort((a, b) => a.sort_order - b.sort_order);
			this.activeTabId = activeId;

			// Create default tab if none exist
			if (this.tabs.length === 0) {
				await this.createQueryTab();
			}
		} finally {
			this.isLoading = false;
		}
	}

	async createQueryTab(connectionId?: string, title?: string, sql?: string): Promise<EditorTab> {
		const tab = await invoke<EditorTab>('create_query_tab', {
			connectionId,
			title,
			sql
		});

		this.tabs = [...this.tabs, tab];
		this.activeTabId = tab.id;
		return tab;
	}

	async createTableTab(connectionId: string, schema: string, table: string): Promise<EditorTab> {
		const tab = await invoke<EditorTab>('create_table_tab', {
			connectionId,
			schema,
			table
		});

		// Check if tab was already open (backend returns existing)
		const existingIndex = this.tabs.findIndex((t) => t.id === tab.id);
		if (existingIndex === -1) {
			this.tabs = [...this.tabs, tab];
		}

		this.activeTabId = tab.id;
		return tab;
	}

	async setActiveTab(tabId: string) {
		await invoke('set_active_tab', { tabId });
		this.activeTabId = tabId;
	}

	async updateTabContent(tabId: string, content: TabContent, isModified: boolean = true) {
		await invoke('update_tab_content', { tabId, content, isModified });

		const index = this.tabs.findIndex((t) => t.id === tabId);
		if (index !== -1) {
			this.tabs[index] = {
				...this.tabs[index],
				content,
				is_modified: isModified
			};
		}
	}

	async renameTab(tabId: string, newTitle: string) {
		await invoke('rename_tab', { tabId, newTitle });

		const index = this.tabs.findIndex((t) => t.id === tabId);
		if (index !== -1) {
			this.tabs[index] = { ...this.tabs[index], title: newTitle };
		}
	}

	async closeTab(tabId: string): Promise<boolean> {
		const tab = this.tabs.find((t) => t.id === tabId);

		// Check for unsaved changes
		if (tab?.is_modified) {
			// Return false to indicate confirmation needed
			return false;
		}

		const newActiveId = await invoke<string | null>('close_tab', { tabId });
		this.tabs = this.tabs.filter((t) => t.id !== tabId);

		if (newActiveId) {
			this.activeTabId = newActiveId;
		} else if (this.tabs.length === 0) {
			this.activeTabId = null;
		}

		return true;
	}

	async forceCloseTab(tabId: string) {
		const newActiveId = await invoke<string | null>('close_tab', { tabId });
		this.tabs = this.tabs.filter((t) => t.id !== tabId);

		if (newActiveId) {
			this.activeTabId = newActiveId;
		} else if (this.tabs.length === 0) {
			this.activeTabId = null;
		}
	}

	async closeOtherTabs(keepTabId: string) {
		await invoke('close_other_tabs', { keepTabId });
		this.tabs = this.tabs.filter((t) => t.id === keepTabId);
		this.activeTabId = keepTabId;
	}

	async closeTabsToRight(tabId: string) {
		const index = this.tabs.findIndex((t) => t.id === tabId);
		if (index === -1) return;

		const tabsToClose = this.tabs.slice(index + 1);
		for (const tab of tabsToClose) {
			await invoke('close_tab', { tabId: tab.id });
		}

		this.tabs = this.tabs.slice(0, index + 1);

		// If active tab was closed, activate the rightmost remaining tab
		if (!this.tabs.find((t) => t.id === this.activeTabId)) {
			this.activeTabId = this.tabs[this.tabs.length - 1]?.id ?? null;
		}
	}

	async reorderTabs(fromIndex: number, toIndex: number) {
		const newTabs = [...this.tabs];
		const [moved] = newTabs.splice(fromIndex, 1);
		newTabs.splice(toIndex, 0, moved);

		this.tabs = newTabs;

		const tabIds = newTabs.map((t) => t.id);
		await invoke('reorder_tabs', { tabIds });
	}

	markTabSaved(tabId: string) {
		const index = this.tabs.findIndex((t) => t.id === tabId);
		if (index !== -1) {
			this.tabs[index] = { ...this.tabs[index], is_modified: false };
		}
	}
}

export const tabStore = new TabStore();
```

### 13.7 Frontend History Store

```typescript
// src/lib/stores/history.svelte.ts

import { invoke } from '@tauri-apps/api/core';

export interface QueryHistoryEntry {
	id: number;
	connection_id: string;
	connection_name: string;
	sql: string;
	executed_at: string;
	duration_ms: number | null;
	rows_affected: number | null;
	error: string | null;
	favorited: boolean;
}

export interface SavedQuery {
	id: string;
	connection_id: string | null;
	name: string;
	description: string | null;
	sql: string;
	folder_id: string | null;
	tags: string[];
	created_at: string;
	updated_at: string;
}

export interface HistorySearchParams {
	connection_id?: string;
	search_text?: string;
	favorites_only?: boolean;
	errors_only?: boolean;
	from_date?: string;
	to_date?: string;
	limit?: number;
	offset?: number;
}

class HistoryStore {
	entries = $state<QueryHistoryEntry[]>([]);
	savedQueries = $state<SavedQuery[]>([]);
	isLoading = $state(false);
	hasMore = $state(true);
	searchParams = $state<HistorySearchParams>({
		limit: 50,
		offset: 0
	});

	async search(params: HistorySearchParams) {
		this.isLoading = true;
		this.searchParams = { ...params, limit: params.limit ?? 50, offset: 0 };

		try {
			const entries = await invoke<QueryHistoryEntry[]>('search_history', {
				params: this.searchParams
			});

			this.entries = entries;
			this.hasMore = entries.length === (this.searchParams.limit ?? 50);
		} finally {
			this.isLoading = false;
		}
	}

	async loadMore() {
		if (!this.hasMore || this.isLoading) return;

		this.isLoading = true;
		const offset = (this.searchParams.offset ?? 0) + (this.searchParams.limit ?? 50);

		try {
			const entries = await invoke<QueryHistoryEntry[]>('search_history', {
				params: { ...this.searchParams, offset }
			});

			this.entries = [...this.entries, ...entries];
			this.searchParams.offset = offset;
			this.hasMore = entries.length === (this.searchParams.limit ?? 50);
		} finally {
			this.isLoading = false;
		}
	}

	async getRecentForConnection(connectionId: string, limit: number = 20) {
		return invoke<QueryHistoryEntry[]>('get_recent_queries', {
			connectionId,
			limit
		});
	}

	async toggleFavorite(historyId: number) {
		const isFavorited = await invoke<boolean>('toggle_history_favorite', {
			historyId
		});

		const index = this.entries.findIndex((e) => e.id === historyId);
		if (index !== -1) {
			this.entries[index] = { ...this.entries[index], favorited: isFavorited };
		}

		return isFavorited;
	}

	async deleteEntries(ids: number[]) {
		await invoke('delete_history_entries', { ids });
		this.entries = this.entries.filter((e) => !ids.includes(e.id));
	}

	async loadSavedQueries(connectionId?: string) {
		this.savedQueries = await invoke<SavedQuery[]>('get_saved_queries', {
			connectionId
		});
	}

	async saveQuery(query: Omit<SavedQuery, 'id' | 'created_at' | 'updated_at'>) {
		const id = crypto.randomUUID();
		const now = new Date().toISOString();

		const savedQuery: SavedQuery = {
			...query,
			id,
			created_at: now,
			updated_at: now
		};

		await invoke('save_query_snippet', { query: savedQuery });
		this.savedQueries = [...this.savedQueries, savedQuery];

		return id;
	}

	async deleteSavedQuery(queryId: string) {
		await invoke('delete_saved_query', { queryId });
		this.savedQueries = this.savedQueries.filter((q) => q.id !== queryId);
	}
}

export const historyStore = new HistoryStore();
```

### 13.8 Tab Bar Component

```svelte
<!-- src/lib/components/shell/TabBar.svelte -->
<script lang="ts">
	import { X, Circle, Plus, ChevronDown } from 'lucide-svelte';
	import { tabStore, type EditorTab } from '$lib/stores/tabs.svelte';
	import { connectionsStore } from '$lib/stores/connections.svelte';
	import ContextMenu from '$lib/components/common/ContextMenu.svelte';

	let draggedTab: EditorTab | null = null;
	let dragOverIndex: number | null = null;
	let contextMenu: { x: number; y: number; tab: EditorTab } | null = null;
	let renamingTabId: string | null = null;
	let renameInput: HTMLInputElement;

	function getConnectionColor(connectionId: string | null): string | undefined {
		if (!connectionId) return undefined;
		const conn = $connectionsStore.connections.find((c) => c.id === connectionId);
		return conn?.color;
	}

	function handleTabClick(tab: EditorTab) {
		tabStore.setActiveTab(tab.id);
	}

	function handleMiddleClick(e: MouseEvent, tab: EditorTab) {
		if (e.button === 1) {
			e.preventDefault();
			closeTab(tab);
		}
	}

	function handleContextMenu(e: MouseEvent, tab: EditorTab) {
		e.preventDefault();
		contextMenu = { x: e.clientX, y: e.clientY, tab };
	}

	function closeContextMenu() {
		contextMenu = null;
	}

	async function closeTab(tab: EditorTab) {
		const closed = await tabStore.closeTab(tab.id);
		if (!closed) {
			// Show unsaved changes dialog
			const confirmed = await showUnsavedDialog(tab);
			if (confirmed) {
				await tabStore.forceCloseTab(tab.id);
			}
		}
	}

	async function showUnsavedDialog(tab: EditorTab): Promise<boolean> {
		// This would be replaced with a proper dialog component
		return confirm(`"${tab.title}" has unsaved changes. Close anyway?`);
	}

	function handleDragStart(e: DragEvent, tab: EditorTab) {
		draggedTab = tab;
		e.dataTransfer!.effectAllowed = 'move';
		e.dataTransfer!.setData('text/plain', tab.id);
	}

	function handleDragOver(e: DragEvent, index: number) {
		e.preventDefault();
		e.dataTransfer!.dropEffect = 'move';
		dragOverIndex = index;
	}

	function handleDragLeave() {
		dragOverIndex = null;
	}

	function handleDrop(e: DragEvent, toIndex: number) {
		e.preventDefault();

		if (draggedTab) {
			const fromIndex = $tabStore.tabs.findIndex((t) => t.id === draggedTab!.id);
			if (fromIndex !== -1 && fromIndex !== toIndex) {
				tabStore.reorderTabs(fromIndex, toIndex);
			}
		}

		draggedTab = null;
		dragOverIndex = null;
	}

	function handleDragEnd() {
		draggedTab = null;
		dragOverIndex = null;
	}

	function startRename(tab: EditorTab) {
		renamingTabId = tab.id;
		closeContextMenu();
		// Focus input after render
		setTimeout(() => renameInput?.focus(), 0);
	}

	function finishRename(tab: EditorTab, newTitle: string) {
		if (newTitle.trim() && newTitle !== tab.title) {
			tabStore.renameTab(tab.id, newTitle.trim());
		}
		renamingTabId = null;
	}

	function handleRenameKeydown(e: KeyboardEvent, tab: EditorTab) {
		if (e.key === 'Enter') {
			finishRename(tab, (e.target as HTMLInputElement).value);
		} else if (e.key === 'Escape') {
			renamingTabId = null;
		}
	}
</script>

<div class="tab-bar">
	<div class="tabs-container">
		{#each $tabStore.tabs as tab, index (tab.id)}
			<div
				class="tab"
				class:active={tab.id === $tabStore.activeTabId}
				class:modified={tab.is_modified}
				class:drag-over={dragOverIndex === index}
				draggable="true"
				onclick={() => handleTabClick(tab)}
				onmousedown={(e) => handleMiddleClick(e, tab)}
				oncontextmenu={(e) => handleContextMenu(e, tab)}
				ondragstart={(e) => handleDragStart(e, tab)}
				ondragover={(e) => handleDragOver(e, index)}
				ondragleave={handleDragLeave}
				ondrop={(e) => handleDrop(e, index)}
				ondragend={handleDragEnd}
				role="tab"
				tabindex="0"
				aria-selected={tab.id === $tabStore.activeTabId}
			>
				{#if getConnectionColor(tab.connection_id)}
					<span
						class="connection-dot"
						style:background-color={getConnectionColor(tab.connection_id)}
					></span>
				{/if}

				{#if renamingTabId === tab.id}
					<input
						bind:this={renameInput}
						class="rename-input"
						type="text"
						value={tab.title}
						onkeydown={(e) => handleRenameKeydown(e, tab)}
						onblur={(e) => finishRename(tab, e.currentTarget.value)}
					/>
				{:else}
					<span class="tab-title" ondblclick={() => startRename(tab)}>
						{tab.title}
					</span>
				{/if}

				{#if tab.is_modified}
					<Circle size={8} class="modified-indicator" fill="currentColor" />
				{/if}

				<button
					class="close-btn"
					onclick={(e) => {
						e.stopPropagation();
						closeTab(tab);
					}}
					title="Close tab"
				>
					<X size={14} />
				</button>
			</div>
		{/each}
	</div>

	<button class="new-tab-btn" onclick={() => tabStore.createQueryTab()} title="New Query (Cmd+N)">
		<Plus size={16} />
	</button>
</div>

{#if contextMenu}
	<ContextMenu
		x={contextMenu.x}
		y={contextMenu.y}
		onClose={closeContextMenu}
		items={[
			{ label: 'Close', action: () => closeTab(contextMenu!.tab) },
			{ label: 'Close Others', action: () => tabStore.closeOtherTabs(contextMenu!.tab.id) },
			{ label: 'Close to the Right', action: () => tabStore.closeTabsToRight(contextMenu!.tab.id) },
			{ type: 'separator' },
			{ label: 'Rename', action: () => startRename(contextMenu!.tab) }
		]}
	/>
{/if}

<style>
	.tab-bar {
		display: flex;
		align-items: center;
		background: var(--surface-color);
		border-bottom: 1px solid var(--border-color);
		height: 36px;
		overflow: hidden;
	}

	.tabs-container {
		display: flex;
		flex: 1;
		overflow-x: auto;
		scrollbar-width: none;
	}

	.tabs-container::-webkit-scrollbar {
		display: none;
	}

	.tab {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0 0.75rem;
		height: 36px;
		border-right: 1px solid var(--border-color);
		cursor: pointer;
		user-select: none;
		white-space: nowrap;
		min-width: 0;
		max-width: 200px;
		background: var(--surface-secondary);
		transition: background 0.15s;
	}

	.tab:hover {
		background: var(--hover-color);
	}

	.tab.active {
		background: var(--background-color);
		border-bottom: 2px solid var(--primary-color);
	}

	.tab.drag-over {
		border-left: 2px solid var(--primary-color);
	}

	.connection-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.tab-title {
		overflow: hidden;
		text-overflow: ellipsis;
		font-size: 0.8125rem;
	}

	.rename-input {
		width: 100px;
		padding: 0.125rem 0.25rem;
		border: 1px solid var(--primary-color);
		border-radius: 0.25rem;
		font-size: 0.8125rem;
		outline: none;
	}

	.modified-indicator {
		color: var(--primary-color);
		flex-shrink: 0;
	}

	.close-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0.125rem;
		border: none;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
		border-radius: 0.25rem;
		opacity: 0;
		transition:
			opacity 0.15s,
			background 0.15s;
	}

	.tab:hover .close-btn {
		opacity: 1;
	}

	.close-btn:hover {
		background: var(--hover-color);
		color: var(--text-color);
	}

	.new-tab-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		border: none;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
		transition: color 0.15s;
	}

	.new-tab-btn:hover {
		color: var(--text-color);
	}
</style>
```

### 13.9 History Panel Component

```svelte
<!-- src/lib/components/shell/HistoryPanel.svelte -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { Search, Star, Clock, AlertCircle, Trash2, Copy, Play } from 'lucide-svelte';
	import { historyStore, type QueryHistoryEntry } from '$lib/stores/history.svelte';
	import { tabStore } from '$lib/stores/tabs.svelte';

	interface Props {
		connectionId?: string;
	}

	let { connectionId }: Props = $props();

	let searchText = $state('');
	let showFavoritesOnly = $state(false);
	let showErrorsOnly = $state(false);
	let selectedIds = $state<Set<number>>(new Set());

	onMount(() => {
		loadHistory();
	});

	$effect(() => {
		// Reload when connection changes
		if (connectionId) {
			loadHistory();
		}
	});

	function loadHistory() {
		historyStore.search({
			connection_id: connectionId,
			search_text: searchText || undefined,
			favorites_only: showFavoritesOnly,
			errors_only: showErrorsOnly
		});
	}

	function formatTime(isoString: string): string {
		const date = new Date(isoString);
		const now = new Date();
		const diff = now.getTime() - date.getTime();

		if (diff < 60000) return 'Just now';
		if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
		if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
		if (diff < 604800000) return `${Math.floor(diff / 86400000)}d ago`;

		return date.toLocaleDateString();
	}

	function formatDuration(ms: number | null): string {
		if (ms === null) return '-';
		if (ms < 1000) return `${ms}ms`;
		return `${(ms / 1000).toFixed(2)}s`;
	}

	function truncateSql(sql: string, maxLength: number = 100): string {
		const singleLine = sql.replace(/\s+/g, ' ').trim();
		if (singleLine.length <= maxLength) return singleLine;
		return singleLine.substring(0, maxLength) + '...';
	}

	async function toggleFavorite(entry: QueryHistoryEntry) {
		await historyStore.toggleFavorite(entry.id);
	}

	function openInNewTab(entry: QueryHistoryEntry) {
		tabStore.createQueryTab(entry.connection_id, undefined, entry.sql);
	}

	function copyToClipboard(sql: string) {
		navigator.clipboard.writeText(sql);
	}

	async function deleteSelected() {
		if (selectedIds.size === 0) return;
		await historyStore.deleteEntries(Array.from(selectedIds));
		selectedIds = new Set();
	}

	function toggleSelection(id: number, e: MouseEvent) {
		if (e.ctrlKey || e.metaKey) {
			const newSet = new Set(selectedIds);
			if (newSet.has(id)) {
				newSet.delete(id);
			} else {
				newSet.add(id);
			}
			selectedIds = newSet;
		} else {
			selectedIds = new Set([id]);
		}
	}
</script>

<div class="history-panel">
	<div class="history-header">
		<div class="search-box">
			<Search size={14} class="search-icon" />
			<input
				type="text"
				placeholder="Search history..."
				bind:value={searchText}
				oninput={() => loadHistory()}
			/>
		</div>

		<div class="filters">
			<button
				class="filter-btn"
				class:active={showFavoritesOnly}
				onclick={() => {
					showFavoritesOnly = !showFavoritesOnly;
					loadHistory();
				}}
				title="Show favorites only"
			>
				<Star size={14} />
			</button>

			<button
				class="filter-btn"
				class:active={showErrorsOnly}
				onclick={() => {
					showErrorsOnly = !showErrorsOnly;
					loadHistory();
				}}
				title="Show errors only"
			>
				<AlertCircle size={14} />
			</button>

			{#if selectedIds.size > 0}
				<button class="filter-btn danger" onclick={deleteSelected} title="Delete selected">
					<Trash2 size={14} />
				</button>
			{/if}
		</div>
	</div>

	<div class="history-list">
		{#if $historyStore.isLoading && $historyStore.entries.length === 0}
			<div class="loading">Loading history...</div>
		{:else if $historyStore.entries.length === 0}
			<div class="empty">No history found</div>
		{:else}
			{#each $historyStore.entries as entry (entry.id)}
				<div
					class="history-entry"
					class:selected={selectedIds.has(entry.id)}
					class:error={entry.error !== null}
					onclick={(e) => toggleSelection(entry.id, e)}
					ondblclick={() => openInNewTab(entry)}
					role="button"
					tabindex="0"
				>
					<div class="entry-header">
						<span class="entry-time">
							<Clock size={12} />
							{formatTime(entry.executed_at)}
						</span>

						<span class="entry-duration">
							{formatDuration(entry.duration_ms)}
						</span>

						{#if entry.rows_affected !== null}
							<span class="entry-rows">
								{entry.rows_affected.toLocaleString()} rows
							</span>
						{/if}
					</div>

					<div class="entry-sql">
						{truncateSql(entry.sql)}
					</div>

					{#if entry.error}
						<div class="entry-error">
							<AlertCircle size={12} />
							{truncateSql(entry.error, 50)}
						</div>
					{/if}

					<div class="entry-actions">
						<button
							class="action-btn"
							class:favorited={entry.favorited}
							onclick={(e) => {
								e.stopPropagation();
								toggleFavorite(entry);
							}}
							title={entry.favorited ? 'Remove from favorites' : 'Add to favorites'}
						>
							<Star size={14} fill={entry.favorited ? 'currentColor' : 'none'} />
						</button>

						<button
							class="action-btn"
							onclick={(e) => {
								e.stopPropagation();
								copyToClipboard(entry.sql);
							}}
							title="Copy SQL"
						>
							<Copy size={14} />
						</button>

						<button
							class="action-btn"
							onclick={(e) => {
								e.stopPropagation();
								openInNewTab(entry);
							}}
							title="Open in new tab"
						>
							<Play size={14} />
						</button>
					</div>
				</div>
			{/each}

			{#if $historyStore.hasMore}
				<button class="load-more" onclick={() => historyStore.loadMore()}>
					{$historyStore.isLoading ? 'Loading...' : 'Load more'}
				</button>
			{/if}
		{/if}
	</div>
</div>

<style>
	.history-panel {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--surface-color);
	}

	.history-header {
		display: flex;
		gap: 0.5rem;
		padding: 0.5rem;
		border-bottom: 1px solid var(--border-color);
	}

	.search-box {
		flex: 1;
		position: relative;
		display: flex;
		align-items: center;
	}

	.search-box :global(.search-icon) {
		position: absolute;
		left: 0.5rem;
		color: var(--text-muted);
	}

	.search-box input {
		width: 100%;
		padding: 0.375rem 0.5rem 0.375rem 1.75rem;
		border: 1px solid var(--border-color);
		border-radius: 0.375rem;
		background: var(--background-color);
		color: var(--text-color);
		font-size: 0.8125rem;
	}

	.search-box input:focus {
		outline: none;
		border-color: var(--primary-color);
	}

	.filters {
		display: flex;
		gap: 0.25rem;
	}

	.filter-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border: 1px solid var(--border-color);
		border-radius: 0.375rem;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
		transition: all 0.15s;
	}

	.filter-btn:hover {
		background: var(--hover-color);
		color: var(--text-color);
	}

	.filter-btn.active {
		background: var(--primary-color);
		border-color: var(--primary-color);
		color: white;
	}

	.filter-btn.danger:hover {
		background: #ef4444;
		border-color: #ef4444;
		color: white;
	}

	.history-list {
		flex: 1;
		overflow-y: auto;
	}

	.loading,
	.empty {
		padding: 2rem;
		text-align: center;
		color: var(--text-muted);
	}

	.history-entry {
		padding: 0.75rem;
		border-bottom: 1px solid var(--border-color);
		cursor: pointer;
		transition: background 0.15s;
	}

	.history-entry:hover {
		background: var(--hover-color);
	}

	.history-entry.selected {
		background: var(--selected-color);
	}

	.history-entry.error {
		border-left: 3px solid #ef4444;
	}

	.entry-header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.375rem;
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.entry-time {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}

	.entry-sql {
		font-family: var(--font-mono);
		font-size: 0.8125rem;
		color: var(--text-color);
		word-break: break-word;
	}

	.entry-error {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		margin-top: 0.375rem;
		font-size: 0.75rem;
		color: #ef4444;
	}

	.entry-actions {
		display: flex;
		gap: 0.25rem;
		margin-top: 0.5rem;
		opacity: 0;
		transition: opacity 0.15s;
	}

	.history-entry:hover .entry-actions {
		opacity: 1;
	}

	.action-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 24px;
		height: 24px;
		border: none;
		border-radius: 0.25rem;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
		transition: all 0.15s;
	}

	.action-btn:hover {
		background: var(--hover-color);
		color: var(--text-color);
	}

	.action-btn.favorited {
		color: #f59e0b;
	}

	.load-more {
		width: 100%;
		padding: 0.75rem;
		border: none;
		background: none;
		color: var(--primary-color);
		font-size: 0.875rem;
		cursor: pointer;
		transition: background 0.15s;
	}

	.load-more:hover {
		background: var(--hover-color);
	}
</style>
```

## Acceptance Criteria

1. **Tab Management**
   - Create new query tabs with unique names
   - Switch between tabs with click or keyboard
   - Close tabs with X button or middle-click
   - Prompt for unsaved changes before closing
   - Drag and drop to reorder tabs
   - Context menu with Close, Close Others, Close to Right

2. **Tab Persistence**
   - Tabs persist across application restarts
   - Active tab restored on startup
   - Tab content (SQL, cursor position) preserved

3. **Tab Renaming**
   - Double-click tab to rename
   - Enter to confirm, Escape to cancel
   - Names persist to storage

4. **Query History**
   - All executed queries recorded with timing
   - Search history by SQL text
   - Filter by favorites, errors
   - Infinite scroll pagination

5. **History Actions**
   - Toggle favorite status
   - Copy SQL to clipboard
   - Open in new tab
   - Delete entries

6. **Saved Queries**
   - Save queries as named snippets
   - Organize in folders
   - Search and filter saved queries
   - Open saved query in new tab

## MCP Testing Instructions

### Using Tauri MCP

```typescript
// Start session
await mcp.driver_session({ action: 'start' });

// Create a new tab
await mcp.ipc_execute_command({
	command: 'create_query_tab',
	args: { title: 'Test Query' }
});

// Verify tab appears
const snapshot = await mcp.webview_dom_snapshot({ type: 'accessibility' });
assert(snapshot.includes('Test Query'));

// Test tab switching
await mcp.webview_click({
	selector: '.tab:first-child',
	element: 'First tab'
});

// Test history recording
await mcp.ipc_execute_command({
	command: 'execute_query',
	args: {
		connId: connectionId,
		sql: 'SELECT 1'
	}
});

// Verify history entry
const history = await mcp.ipc_execute_command({
	command: 'search_history',
	args: { params: { limit: 10 } }
});
assert(history.length > 0);
assert(history[0].sql === 'SELECT 1');
```

## Dependencies

- Feature 05: Local Storage (SQLite for tab/history persistence)
- Feature 11: Query Execution (history recording)
