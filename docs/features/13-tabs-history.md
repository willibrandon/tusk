# Feature 13: Tab Management and Query History

## Overview

Tab management provides a multi-document interface for working with multiple queries and database objects simultaneously. Query history tracks all executed queries for easy retrieval and re-execution. Both features use GPUI's native component system and integrate with local SQLite storage for persistence across sessions.

## Goals

- Support multiple query tabs with independent state
- Persist tab state across application restarts
- Track all executed queries with timing and results
- Enable query history search, filtering, and favorites
- Save queries as snippets for reuse
- Provide tab context menu and keyboard navigation
- Native GPUI rendering with GPU acceleration

## Dependencies

- Feature 05: Local Storage (SQLite persistence)
- Feature 11: Query Execution (query results and timing)
- Feature 12: SQL Editor (editor content and buffer management)

## Technical Specification

### 13.1 Tab State Models

```rust
// src/models/tabs.rs

use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents an editor tab in the application
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorTab {
    pub id: Uuid,
    pub connection_id: Option<Uuid>,
    pub tab_type: TabType,
    pub title: String,
    pub content: TabContent,
    pub is_modified: bool,
    pub is_pinned: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
}

impl EditorTab {
    /// Create a new query tab
    pub fn new_query(
        connection_id: Option<Uuid>,
        title: String,
        sql: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            connection_id,
            tab_type: TabType::Query,
            title,
            content: TabContent::Query(QueryTabContent {
                sql,
                cursor_position: None,
                selections: Vec::new(),
                scroll_position: ScrollPosition::default(),
            }),
            is_modified: false,
            is_pinned: false,
            sort_order: 0,
            created_at: Utc::now(),
            last_accessed_at: Utc::now(),
        }
    }

    /// Create a new table data tab
    pub fn new_table_data(
        connection_id: Uuid,
        schema: String,
        table: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            connection_id: Some(connection_id),
            tab_type: TabType::TableData,
            title: format!("{}.{}", schema, table),
            content: TabContent::TableData(TableTabContent {
                schema,
                table,
                filters: Vec::new(),
                sort: None,
                page: 1,
                page_size: 100,
            }),
            is_modified: false,
            is_pinned: false,
            sort_order: 0,
            created_at: Utc::now(),
            last_accessed_at: Utc::now(),
        }
    }

    /// Create a function editor tab
    pub fn new_function_editor(
        connection_id: Uuid,
        schema: String,
        function_name: String,
        source: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            connection_id: Some(connection_id),
            tab_type: TabType::FunctionEditor,
            title: format!("{}.{}", schema, function_name),
            content: TabContent::FunctionEditor(FunctionTabContent {
                schema,
                function_name,
                original_source: source.clone(),
                current_source: source,
            }),
            is_modified: false,
            is_pinned: false,
            sort_order: 0,
            created_at: Utc::now(),
            last_accessed_at: Utc::now(),
        }
    }

    /// Create a query plan tab
    pub fn new_query_plan(
        connection_id: Uuid,
        sql: String,
        plan_json: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            connection_id: Some(connection_id),
            tab_type: TabType::QueryPlan,
            title: "Query Plan".to_string(),
            content: TabContent::QueryPlan(QueryPlanTabContent {
                sql,
                plan_json,
                plan_text: None,
            }),
            is_modified: false,
            is_pinned: false,
            sort_order: 0,
            created_at: Utc::now(),
            last_accessed_at: Utc::now(),
        }
    }

    /// Check if this tab matches a table data request
    pub fn matches_table(&self, connection_id: Uuid, schema: &str, table: &str) -> bool {
        if self.connection_id != Some(connection_id) {
            return false;
        }

        match &self.content {
            TabContent::TableData(content) => {
                content.schema == schema && content.table == table
            }
            _ => false,
        }
    }

    /// Update the last accessed timestamp
    pub fn touch(&mut self) {
        self.last_accessed_at = Utc::now();
    }
}

/// Type of editor tab
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TabType {
    Query,
    TableData,
    ViewData,
    FunctionEditor,
    QueryPlan,
    SchemaViewer,
}

impl TabType {
    /// Get the icon name for this tab type
    pub fn icon_name(&self) -> &'static str {
        match self {
            TabType::Query => "file-code",
            TabType::TableData => "table",
            TabType::ViewData => "eye",
            TabType::FunctionEditor => "function",
            TabType::QueryPlan => "git-branch",
            TabType::SchemaViewer => "database",
        }
    }
}

/// Content specific to each tab type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TabContent {
    Query(QueryTabContent),
    TableData(TableTabContent),
    FunctionEditor(FunctionTabContent),
    QueryPlan(QueryPlanTabContent),
    SchemaViewer(SchemaViewerTabContent),
}

/// Content for query tabs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryTabContent {
    pub sql: String,
    pub cursor_position: Option<CursorPosition>,
    pub selections: Vec<SelectionRange>,
    pub scroll_position: ScrollPosition,
}

/// Content for table data tabs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableTabContent {
    pub schema: String,
    pub table: String,
    pub filters: Vec<TableFilter>,
    pub sort: Option<TableSort>,
    pub page: u32,
    pub page_size: u32,
}

/// Content for function editor tabs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionTabContent {
    pub schema: String,
    pub function_name: String,
    pub original_source: String,
    pub current_source: String,
}

/// Content for query plan tabs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryPlanTabContent {
    pub sql: String,
    pub plan_json: Option<String>,
    pub plan_text: Option<String>,
}

/// Content for schema viewer tabs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchemaViewerTabContent {
    pub schema: String,
    pub object_type: String,
    pub object_name: String,
}

/// Cursor position in an editor
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
}

/// Selection range in an editor
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelectionRange {
    pub start: CursorPosition,
    pub end: CursorPosition,
    pub is_reversed: bool,
}

/// Scroll position in an editor
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ScrollPosition {
    pub x: f32,
    pub y: f32,
}

/// Filter for table data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableFilter {
    pub column: String,
    pub operator: FilterOperator,
    pub value: String,
    pub is_enabled: bool,
}

/// Filter operators
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    NotIn,
    IsNull,
    IsNotNull,
    Between,
    Contains,
    StartsWith,
    EndsWith,
}

impl FilterOperator {
    /// Get the SQL representation
    pub fn to_sql(&self) -> &'static str {
        match self {
            FilterOperator::Equals => "=",
            FilterOperator::NotEquals => "<>",
            FilterOperator::GreaterThan => ">",
            FilterOperator::LessThan => "<",
            FilterOperator::GreaterOrEqual => ">=",
            FilterOperator::LessOrEqual => "<=",
            FilterOperator::Like => "LIKE",
            FilterOperator::ILike => "ILIKE",
            FilterOperator::In => "IN",
            FilterOperator::NotIn => "NOT IN",
            FilterOperator::IsNull => "IS NULL",
            FilterOperator::IsNotNull => "IS NOT NULL",
            FilterOperator::Between => "BETWEEN",
            FilterOperator::Contains => "ILIKE",
            FilterOperator::StartsWith => "ILIKE",
            FilterOperator::EndsWith => "ILIKE",
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            FilterOperator::Equals => "equals",
            FilterOperator::NotEquals => "not equals",
            FilterOperator::GreaterThan => "greater than",
            FilterOperator::LessThan => "less than",
            FilterOperator::GreaterOrEqual => "greater or equal",
            FilterOperator::LessOrEqual => "less or equal",
            FilterOperator::Like => "like",
            FilterOperator::ILike => "ilike (case insensitive)",
            FilterOperator::In => "in",
            FilterOperator::NotIn => "not in",
            FilterOperator::IsNull => "is null",
            FilterOperator::IsNotNull => "is not null",
            FilterOperator::Between => "between",
            FilterOperator::Contains => "contains",
            FilterOperator::StartsWith => "starts with",
            FilterOperator::EndsWith => "ends with",
        }
    }

    /// Check if this operator requires a value
    pub fn requires_value(&self) -> bool {
        !matches!(self, FilterOperator::IsNull | FilterOperator::IsNotNull)
    }
}

/// Sort configuration for table data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableSort {
    pub column: String,
    pub direction: SortDirection,
}

/// Sort direction
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    /// Toggle the direction
    pub fn toggle(&self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }

    /// Get SQL representation
    pub fn to_sql(&self) -> &'static str {
        match self {
            SortDirection::Ascending => "ASC",
            SortDirection::Descending => "DESC",
        }
    }
}
```

### 13.2 Query History Models

```rust
// src/models/history.rs

use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single query history entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryHistoryEntry {
    pub id: i64,
    pub connection_id: Uuid,
    pub connection_name: String,
    pub database_name: String,
    pub sql: String,
    pub executed_at: DateTime<Utc>,
    pub duration_ms: Option<u64>,
    pub rows_affected: Option<u64>,
    pub error: Option<String>,
    pub favorited: bool,
    pub tags: Vec<String>,
}

impl QueryHistoryEntry {
    /// Check if this entry was successful
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    /// Get a truncated version of the SQL for display
    pub fn truncated_sql(&self, max_length: usize) -> String {
        let single_line = self.sql
            .chars()
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect::<String>();

        let trimmed = single_line.split_whitespace().collect::<Vec<_>>().join(" ");

        if trimmed.len() <= max_length {
            trimmed
        } else {
            format!("{}...", &trimmed[..max_length])
        }
    }

    /// Format duration for display
    pub fn formatted_duration(&self) -> String {
        match self.duration_ms {
            None => "-".to_string(),
            Some(ms) if ms < 1000 => format!("{}ms", ms),
            Some(ms) if ms < 60000 => format!("{:.2}s", ms as f64 / 1000.0),
            Some(ms) => {
                let seconds = ms / 1000;
                let minutes = seconds / 60;
                let remaining_seconds = seconds % 60;
                format!("{}m {}s", minutes, remaining_seconds)
            }
        }
    }

    /// Format relative time for display
    pub fn relative_time(&self) -> String {
        let now = Utc::now();
        let diff = now.signed_duration_since(self.executed_at);

        let seconds = diff.num_seconds();

        if seconds < 60 {
            "Just now".to_string()
        } else if seconds < 3600 {
            let minutes = seconds / 60;
            if minutes == 1 {
                "1 minute ago".to_string()
            } else {
                format!("{} minutes ago", minutes)
            }
        } else if seconds < 86400 {
            let hours = seconds / 3600;
            if hours == 1 {
                "1 hour ago".to_string()
            } else {
                format!("{} hours ago", hours)
            }
        } else if seconds < 604800 {
            let days = seconds / 86400;
            if days == 1 {
                "Yesterday".to_string()
            } else {
                format!("{} days ago", days)
            }
        } else {
            self.executed_at.format("%Y-%m-%d %H:%M").to_string()
        }
    }
}

/// A saved query snippet
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedQuery {
    pub id: Uuid,
    pub connection_id: Option<Uuid>,  // None = global
    pub name: String,
    pub description: Option<String>,
    pub sql: String,
    pub folder_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub keyboard_shortcut: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SavedQuery {
    /// Create a new saved query
    pub fn new(
        name: String,
        sql: String,
        connection_id: Option<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            connection_id,
            name,
            description: None,
            sql,
            folder_id: None,
            tags: Vec::new(),
            keyboard_shortcut: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the SQL content
    pub fn update_sql(&mut self, sql: String) {
        self.sql = sql;
        self.updated_at = Utc::now();
    }
}

/// A folder for organizing saved queries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedQueryFolder {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
    pub icon: Option<String>,
    pub color: Option<String>,
}

impl SavedQueryFolder {
    /// Create a new folder
    pub fn new(name: String, parent_id: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            parent_id,
            sort_order: 0,
            icon: None,
            color: None,
        }
    }
}

/// Search parameters for history
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HistorySearchParams {
    pub connection_id: Option<Uuid>,
    pub search_text: Option<String>,
    pub favorites_only: bool,
    pub errors_only: bool,
    pub success_only: bool,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub min_duration_ms: Option<u64>,
    pub max_duration_ms: Option<u64>,
    pub limit: u32,
    pub offset: u32,
}

impl HistorySearchParams {
    /// Create with default pagination
    pub fn with_limit(limit: u32) -> Self {
        Self {
            limit,
            ..Default::default()
        }
    }

    /// Filter by connection
    pub fn for_connection(mut self, connection_id: Uuid) -> Self {
        self.connection_id = Some(connection_id);
        self
    }

    /// Filter favorites only
    pub fn favorites(mut self) -> Self {
        self.favorites_only = true;
        self
    }

    /// Filter errors only
    pub fn errors(mut self) -> Self {
        self.errors_only = true;
        self
    }
}

/// Result of a history search including total count
#[derive(Clone, Debug)]
pub struct HistorySearchResult {
    pub entries: Vec<QueryHistoryEntry>,
    pub total_count: u64,
    pub has_more: bool,
}
```

### 13.3 Tab Service

```rust
// src/services/tabs.rs

use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use parking_lot::RwLock;
use tokio::runtime::Handle;

use crate::error::{Error, Result};
use crate::models::tabs::{EditorTab, TabType, TabContent, QueryTabContent};
use crate::services::storage::StorageService;

/// Service for managing editor tabs
pub struct TabService {
    storage: Arc<StorageService>,
    tabs: RwLock<Vec<EditorTab>>,
    active_tab_id: RwLock<Option<Uuid>>,
    runtime: Handle,
}

impl TabService {
    /// Create a new tab service
    pub fn new(storage: Arc<StorageService>, runtime: Handle) -> Self {
        Self {
            storage,
            tabs: RwLock::new(Vec::new()),
            active_tab_id: RwLock::new(None),
            runtime,
        }
    }

    /// Initialize and restore tabs from storage
    pub fn initialize(&self) -> Result<()> {
        let storage = self.storage.clone();

        let (tabs, active_id) = self.runtime.block_on(async move {
            let tabs = storage.get_all_tabs().await?;
            let active_id = storage.get_active_tab_id().await?;
            Ok::<_, Error>((tabs, active_id))
        })?;

        {
            let mut tabs_guard = self.tabs.write();
            *tabs_guard = tabs;
        }

        {
            let mut active_guard = self.active_tab_id.write();
            *active_guard = active_id;
        }

        Ok(())
    }

    /// Get all tabs
    pub fn get_all_tabs(&self) -> Vec<EditorTab> {
        self.tabs.read().clone()
    }

    /// Get the active tab ID
    pub fn get_active_tab_id(&self) -> Option<Uuid> {
        *self.active_tab_id.read()
    }

    /// Get the active tab
    pub fn get_active_tab(&self) -> Option<EditorTab> {
        let active_id = *self.active_tab_id.read();
        active_id.and_then(|id| {
            self.tabs.read().iter().find(|t| t.id == id).cloned()
        })
    }

    /// Get a tab by ID
    pub fn get_tab(&self, tab_id: Uuid) -> Option<EditorTab> {
        self.tabs.read().iter().find(|t| t.id == tab_id).cloned()
    }

    /// Create a new query tab
    pub fn create_query_tab(
        &self,
        connection_id: Option<Uuid>,
        title: Option<String>,
        sql: Option<String>,
    ) -> Result<EditorTab> {
        let next_num = self.get_next_query_number();
        let title = title.unwrap_or_else(|| format!("Query {}", next_num));

        let mut tab = EditorTab::new_query(
            connection_id,
            title,
            sql.unwrap_or_default(),
        );

        // Set sort order to end
        {
            let tabs = self.tabs.read();
            tab.sort_order = tabs.len() as i32;
        }

        // Persist to storage
        let storage = self.storage.clone();
        let tab_clone = tab.clone();
        self.runtime.block_on(async move {
            storage.save_tab(&tab_clone).await
        })?;

        // Add to memory
        {
            let mut tabs = self.tabs.write();
            tabs.push(tab.clone());
        }

        // Set as active
        self.set_active_tab(tab.id)?;

        Ok(tab)
    }

    /// Create a table data viewer tab
    pub fn create_table_tab(
        &self,
        connection_id: Uuid,
        schema: String,
        table: String,
    ) -> Result<EditorTab> {
        // Check if tab already exists
        {
            let tabs = self.tabs.read();
            if let Some(existing) = tabs.iter().find(|t| t.matches_table(connection_id, &schema, &table)) {
                let tab_id = existing.id;
                drop(tabs);
                self.set_active_tab(tab_id)?;
                return Ok(self.get_tab(tab_id).unwrap());
            }
        }

        let mut tab = EditorTab::new_table_data(connection_id, schema, table);

        // Set sort order
        {
            let tabs = self.tabs.read();
            tab.sort_order = tabs.len() as i32;
        }

        // Persist
        let storage = self.storage.clone();
        let tab_clone = tab.clone();
        self.runtime.block_on(async move {
            storage.save_tab(&tab_clone).await
        })?;

        // Add to memory
        {
            let mut tabs = self.tabs.write();
            tabs.push(tab.clone());
        }

        // Set as active
        self.set_active_tab(tab.id)?;

        Ok(tab)
    }

    /// Create a function editor tab
    pub fn create_function_tab(
        &self,
        connection_id: Uuid,
        schema: String,
        function_name: String,
        source: String,
    ) -> Result<EditorTab> {
        let mut tab = EditorTab::new_function_editor(
            connection_id,
            schema,
            function_name,
            source,
        );

        {
            let tabs = self.tabs.read();
            tab.sort_order = tabs.len() as i32;
        }

        let storage = self.storage.clone();
        let tab_clone = tab.clone();
        self.runtime.block_on(async move {
            storage.save_tab(&tab_clone).await
        })?;

        {
            let mut tabs = self.tabs.write();
            tabs.push(tab.clone());
        }

        self.set_active_tab(tab.id)?;

        Ok(tab)
    }

    /// Create a query plan tab
    pub fn create_query_plan_tab(
        &self,
        connection_id: Uuid,
        sql: String,
        plan_json: Option<String>,
    ) -> Result<EditorTab> {
        let mut tab = EditorTab::new_query_plan(connection_id, sql, plan_json);

        {
            let tabs = self.tabs.read();
            tab.sort_order = tabs.len() as i32;
        }

        let storage = self.storage.clone();
        let tab_clone = tab.clone();
        self.runtime.block_on(async move {
            storage.save_tab(&tab_clone).await
        })?;

        {
            let mut tabs = self.tabs.write();
            tabs.push(tab.clone());
        }

        self.set_active_tab(tab.id)?;

        Ok(tab)
    }

    /// Set the active tab
    pub fn set_active_tab(&self, tab_id: Uuid) -> Result<()> {
        // Update last accessed time
        {
            let mut tabs = self.tabs.write();
            if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
                tab.touch();
            }
        }

        // Update active ID
        {
            let mut active = self.active_tab_id.write();
            *active = Some(tab_id);
        }

        // Persist
        let storage = self.storage.clone();
        self.runtime.block_on(async move {
            storage.set_active_tab(tab_id).await?;
            storage.update_tab_accessed(tab_id).await
        })?;

        Ok(())
    }

    /// Update tab content
    pub fn update_tab_content(
        &self,
        tab_id: Uuid,
        content: TabContent,
        is_modified: bool,
    ) -> Result<()> {
        {
            let mut tabs = self.tabs.write();
            if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
                tab.content = content.clone();
                tab.is_modified = is_modified;
            }
        }

        let storage = self.storage.clone();
        self.runtime.block_on(async move {
            storage.update_tab_content(tab_id, content, is_modified).await
        })?;

        Ok(())
    }

    /// Update just the SQL content of a query tab
    pub fn update_query_sql(&self, tab_id: Uuid, sql: String) -> Result<()> {
        let mut tabs = self.tabs.write();
        if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
            if let TabContent::Query(ref mut content) = tab.content {
                content.sql = sql;
                tab.is_modified = true;
            }
        }
        Ok(())
    }

    /// Mark tab as saved (not modified)
    pub fn mark_tab_saved(&self, tab_id: Uuid) -> Result<()> {
        {
            let mut tabs = self.tabs.write();
            if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
                tab.is_modified = false;
            }
        }

        let storage = self.storage.clone();
        self.runtime.block_on(async move {
            storage.mark_tab_saved(tab_id).await
        })?;

        Ok(())
    }

    /// Rename a tab
    pub fn rename_tab(&self, tab_id: Uuid, new_title: String) -> Result<()> {
        {
            let mut tabs = self.tabs.write();
            if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
                tab.title = new_title.clone();
            }
        }

        let storage = self.storage.clone();
        self.runtime.block_on(async move {
            storage.rename_tab(tab_id, new_title).await
        })?;

        Ok(())
    }

    /// Toggle pin status of a tab
    pub fn toggle_pin(&self, tab_id: Uuid) -> Result<bool> {
        let is_pinned;
        {
            let mut tabs = self.tabs.write();
            if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
                tab.is_pinned = !tab.is_pinned;
                is_pinned = tab.is_pinned;
            } else {
                return Err(Error::NotFound("Tab not found".to_string()));
            }
        }

        let storage = self.storage.clone();
        self.runtime.block_on(async move {
            storage.update_tab_pinned(tab_id, is_pinned).await
        })?;

        Ok(is_pinned)
    }

    /// Close a tab, returns the new active tab ID if changed
    pub fn close_tab(&self, tab_id: Uuid) -> Result<Option<Uuid>> {
        let (tab_index, active_id) = {
            let tabs = self.tabs.read();
            let index = tabs.iter().position(|t| t.id == tab_id);
            let active = *self.active_tab_id.read();
            (index, active)
        };

        // Remove from storage
        let storage = self.storage.clone();
        self.runtime.block_on(async move {
            storage.delete_tab(tab_id).await
        })?;

        // Remove from memory and determine new active
        let new_active = {
            let mut tabs = self.tabs.write();
            tabs.retain(|t| t.id != tab_id);

            if active_id == Some(tab_id) && !tabs.is_empty() {
                // Activate adjacent tab
                let new_idx = tab_index
                    .map(|i| i.min(tabs.len().saturating_sub(1)))
                    .unwrap_or(0);

                Some(tabs[new_idx].id)
            } else if tabs.is_empty() {
                None
            } else {
                active_id
            }
        };

        // Update active tab
        if let Some(new_id) = new_active {
            if active_id == Some(tab_id) {
                self.set_active_tab(new_id)?;
            }
        } else {
            let mut active = self.active_tab_id.write();
            *active = None;
        }

        Ok(new_active)
    }

    /// Close all tabs except the specified one
    pub fn close_other_tabs(&self, keep_tab_id: Uuid) -> Result<()> {
        let tabs_to_close: Vec<Uuid> = {
            let tabs = self.tabs.read();
            tabs.iter()
                .filter(|t| t.id != keep_tab_id && !t.is_pinned)
                .map(|t| t.id)
                .collect()
        };

        for tab_id in tabs_to_close {
            let storage = self.storage.clone();
            self.runtime.block_on(async move {
                storage.delete_tab(tab_id).await
            })?;
        }

        {
            let mut tabs = self.tabs.write();
            tabs.retain(|t| t.id == keep_tab_id || t.is_pinned);
        }

        self.set_active_tab(keep_tab_id)?;

        Ok(())
    }

    /// Close tabs to the right of the specified tab
    pub fn close_tabs_to_right(&self, tab_id: Uuid) -> Result<()> {
        let (start_index, tabs_to_close): (Option<usize>, Vec<Uuid>) = {
            let tabs = self.tabs.read();
            let index = tabs.iter().position(|t| t.id == tab_id);

            let to_close = index
                .map(|i| {
                    tabs.iter()
                        .skip(i + 1)
                        .filter(|t| !t.is_pinned)
                        .map(|t| t.id)
                        .collect()
                })
                .unwrap_or_default();

            (index, to_close)
        };

        for tab_id in tabs_to_close {
            let storage = self.storage.clone();
            self.runtime.block_on(async move {
                storage.delete_tab(tab_id).await
            })?;
        }

        if let Some(idx) = start_index {
            let mut tabs = self.tabs.write();
            let keep_ids: Vec<Uuid> = tabs.iter()
                .take(idx + 1)
                .chain(tabs.iter().skip(idx + 1).filter(|t| t.is_pinned))
                .map(|t| t.id)
                .collect();

            tabs.retain(|t| keep_ids.contains(&t.id));
        }

        // If active tab was closed, activate the rightmost remaining
        let active = *self.active_tab_id.read();
        let tabs = self.tabs.read();
        if active.is_some() && !tabs.iter().any(|t| Some(t.id) == active) {
            if let Some(last) = tabs.last() {
                drop(tabs);
                self.set_active_tab(last.id)?;
            }
        }

        Ok(())
    }

    /// Close all tabs
    pub fn close_all_tabs(&self) -> Result<()> {
        let tabs_to_close: Vec<Uuid> = {
            let tabs = self.tabs.read();
            tabs.iter()
                .filter(|t| !t.is_pinned)
                .map(|t| t.id)
                .collect()
        };

        for tab_id in tabs_to_close {
            let storage = self.storage.clone();
            self.runtime.block_on(async move {
                storage.delete_tab(tab_id).await
            })?;
        }

        {
            let mut tabs = self.tabs.write();
            tabs.retain(|t| t.is_pinned);
        }

        // Update active to first pinned or none
        let new_active = self.tabs.read().first().map(|t| t.id);
        let mut active = self.active_tab_id.write();
        *active = new_active;

        Ok(())
    }

    /// Reorder tabs
    pub fn reorder_tabs(&self, tab_ids: Vec<Uuid>) -> Result<()> {
        {
            let mut tabs = self.tabs.write();

            // Create a map of id -> tab
            let tab_map: std::collections::HashMap<Uuid, EditorTab> =
                tabs.drain(..).map(|t| (t.id, t)).collect();

            // Rebuild in new order
            for (order, id) in tab_ids.iter().enumerate() {
                if let Some(mut tab) = tab_map.get(id).cloned() {
                    tab.sort_order = order as i32;
                    tabs.push(tab);
                }
            }
        }

        // Persist new order
        let storage = self.storage.clone();
        self.runtime.block_on(async move {
            for (order, tab_id) in tab_ids.iter().enumerate() {
                storage.update_tab_order(*tab_id, order as i32).await?;
            }
            Ok::<_, Error>(())
        })?;

        Ok(())
    }

    /// Move to next tab
    pub fn next_tab(&self) -> Result<()> {
        let tabs = self.tabs.read();
        if tabs.len() <= 1 {
            return Ok(());
        }

        let active = *self.active_tab_id.read();
        let current_idx = active
            .and_then(|id| tabs.iter().position(|t| t.id == id))
            .unwrap_or(0);

        let next_idx = (current_idx + 1) % tabs.len();
        let next_id = tabs[next_idx].id;

        drop(tabs);
        self.set_active_tab(next_id)
    }

    /// Move to previous tab
    pub fn previous_tab(&self) -> Result<()> {
        let tabs = self.tabs.read();
        if tabs.len() <= 1 {
            return Ok(());
        }

        let active = *self.active_tab_id.read();
        let current_idx = active
            .and_then(|id| tabs.iter().position(|t| t.id == id))
            .unwrap_or(0);

        let prev_idx = if current_idx == 0 {
            tabs.len() - 1
        } else {
            current_idx - 1
        };
        let prev_id = tabs[prev_idx].id;

        drop(tabs);
        self.set_active_tab(prev_id)
    }

    /// Get tabs with unsaved changes
    pub fn get_modified_tabs(&self) -> Vec<EditorTab> {
        self.tabs.read()
            .iter()
            .filter(|t| t.is_modified)
            .cloned()
            .collect()
    }

    /// Check if any tabs have unsaved changes
    pub fn has_unsaved_changes(&self) -> bool {
        self.tabs.read().iter().any(|t| t.is_modified)
    }

    /// Get next query number for naming
    fn get_next_query_number(&self) -> u32 {
        let tabs = self.tabs.read();
        tabs.iter()
            .filter(|t| t.tab_type == TabType::Query)
            .filter_map(|t| {
                t.title
                    .strip_prefix("Query ")
                    .and_then(|n| n.parse::<u32>().ok())
            })
            .max()
            .unwrap_or(0) + 1
    }
}
```

### 13.4 History Service

```rust
// src/services/history.rs

use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use parking_lot::RwLock;
use tokio::runtime::Handle;

use crate::error::{Error, Result};
use crate::models::history::{
    QueryHistoryEntry, SavedQuery, SavedQueryFolder,
    HistorySearchParams, HistorySearchResult
};
use crate::services::storage::StorageService;

/// Service for managing query history and saved queries
pub struct HistoryService {
    storage: Arc<StorageService>,
    runtime: Handle,
    /// In-memory cache of recent history entries
    recent_cache: RwLock<Vec<QueryHistoryEntry>>,
    /// Maximum cache size
    cache_size: usize,
}

impl HistoryService {
    /// Create a new history service
    pub fn new(storage: Arc<StorageService>, runtime: Handle) -> Self {
        Self {
            storage,
            runtime,
            recent_cache: RwLock::new(Vec::new()),
            cache_size: 100,
        }
    }

    /// Initialize the service and warm the cache
    pub fn initialize(&self) -> Result<()> {
        let entries = self.search_history(HistorySearchParams::with_limit(self.cache_size as u32))?;

        let mut cache = self.recent_cache.write();
        *cache = entries.entries;

        Ok(())
    }

    /// Record a query execution
    pub fn record_query(
        &self,
        connection_id: Uuid,
        connection_name: &str,
        database_name: &str,
        sql: &str,
        duration_ms: u64,
        rows_affected: Option<u64>,
        error: Option<String>,
    ) -> Result<i64> {
        let storage = self.storage.clone();
        let sql = sql.to_string();
        let connection_name = connection_name.to_string();
        let database_name = database_name.to_string();

        let entry_id = self.runtime.block_on(async move {
            storage.insert_query_history(
                connection_id,
                &connection_name,
                &database_name,
                &sql,
                duration_ms,
                rows_affected,
                error,
            ).await
        })?;

        // Invalidate cache (will be refreshed on next query)
        self.recent_cache.write().clear();

        Ok(entry_id)
    }

    /// Search query history
    pub fn search_history(&self, params: HistorySearchParams) -> Result<HistorySearchResult> {
        let storage = self.storage.clone();

        self.runtime.block_on(async move {
            storage.search_query_history(params).await
        })
    }

    /// Get recent queries for a connection
    pub fn get_recent_queries(
        &self,
        connection_id: Uuid,
        limit: u32,
    ) -> Result<Vec<QueryHistoryEntry>> {
        let result = self.search_history(
            HistorySearchParams::with_limit(limit).for_connection(connection_id)
        )?;

        Ok(result.entries)
    }

    /// Get cached recent history (fast, may be stale)
    pub fn get_cached_recent(&self) -> Vec<QueryHistoryEntry> {
        self.recent_cache.read().clone()
    }

    /// Toggle favorite status
    pub fn toggle_favorite(&self, history_id: i64) -> Result<bool> {
        let storage = self.storage.clone();

        let is_favorited = self.runtime.block_on(async move {
            storage.toggle_history_favorite(history_id).await
        })?;

        // Update cache if entry exists
        {
            let mut cache = self.recent_cache.write();
            if let Some(entry) = cache.iter_mut().find(|e| e.id == history_id) {
                entry.favorited = is_favorited;
            }
        }

        Ok(is_favorited)
    }

    /// Delete history entries
    pub fn delete_entries(&self, ids: Vec<i64>) -> Result<()> {
        let storage = self.storage.clone();

        self.runtime.block_on(async move {
            for id in &ids {
                storage.delete_history_entry(*id).await?;
            }
            Ok::<_, Error>(())
        })?;

        // Update cache
        {
            let mut cache = self.recent_cache.write();
            cache.retain(|e| !ids.contains(&e.id));
        }

        Ok(())
    }

    /// Clear all history for a connection
    pub fn clear_connection_history(&self, connection_id: Uuid) -> Result<()> {
        let storage = self.storage.clone();

        self.runtime.block_on(async move {
            storage.clear_connection_history(connection_id).await
        })?;

        // Update cache
        {
            let mut cache = self.recent_cache.write();
            cache.retain(|e| e.connection_id != connection_id);
        }

        Ok(())
    }

    /// Clear all history
    pub fn clear_all_history(&self) -> Result<()> {
        let storage = self.storage.clone();

        self.runtime.block_on(async move {
            storage.clear_all_history().await
        })?;

        self.recent_cache.write().clear();

        Ok(())
    }

    /// Add or update tags for a history entry
    pub fn update_tags(&self, history_id: i64, tags: Vec<String>) -> Result<()> {
        let storage = self.storage.clone();

        self.runtime.block_on(async move {
            storage.update_history_tags(history_id, tags).await
        })?;

        Ok(())
    }

    // === Saved Queries ===

    /// Save a query as a snippet
    pub fn save_query(&self, query: SavedQuery) -> Result<Uuid> {
        let storage = self.storage.clone();
        let id = query.id;

        self.runtime.block_on(async move {
            storage.insert_saved_query(&query).await
        })?;

        Ok(id)
    }

    /// Get all saved queries
    pub fn get_saved_queries(&self, connection_id: Option<Uuid>) -> Result<Vec<SavedQuery>> {
        let storage = self.storage.clone();

        self.runtime.block_on(async move {
            storage.get_saved_queries(connection_id).await
        })
    }

    /// Get a single saved query
    pub fn get_saved_query(&self, query_id: Uuid) -> Result<Option<SavedQuery>> {
        let storage = self.storage.clone();

        self.runtime.block_on(async move {
            storage.get_saved_query(query_id).await
        })
    }

    /// Update a saved query
    pub fn update_saved_query(&self, query: &SavedQuery) -> Result<()> {
        let storage = self.storage.clone();
        let query = query.clone();

        self.runtime.block_on(async move {
            storage.update_saved_query(&query).await
        })
    }

    /// Delete a saved query
    pub fn delete_saved_query(&self, query_id: Uuid) -> Result<()> {
        let storage = self.storage.clone();

        self.runtime.block_on(async move {
            storage.delete_saved_query(query_id).await
        })
    }

    /// Search saved queries
    pub fn search_saved_queries(
        &self,
        search_text: &str,
        connection_id: Option<Uuid>,
    ) -> Result<Vec<SavedQuery>> {
        let storage = self.storage.clone();
        let search = search_text.to_string();

        self.runtime.block_on(async move {
            storage.search_saved_queries(&search, connection_id).await
        })
    }

    // === Folders ===

    /// Create a folder for saved queries
    pub fn create_folder(&self, folder: SavedQueryFolder) -> Result<Uuid> {
        let storage = self.storage.clone();
        let id = folder.id;

        self.runtime.block_on(async move {
            storage.insert_saved_query_folder(&folder).await
        })?;

        Ok(id)
    }

    /// Get all folders
    pub fn get_folders(&self) -> Result<Vec<SavedQueryFolder>> {
        let storage = self.storage.clone();

        self.runtime.block_on(async move {
            storage.get_saved_query_folders().await
        })
    }

    /// Update a folder
    pub fn update_folder(&self, folder: &SavedQueryFolder) -> Result<()> {
        let storage = self.storage.clone();
        let folder = folder.clone();

        self.runtime.block_on(async move {
            storage.update_saved_query_folder(&folder).await
        })
    }

    /// Delete a folder (moves queries to root)
    pub fn delete_folder(&self, folder_id: Uuid) -> Result<()> {
        let storage = self.storage.clone();

        self.runtime.block_on(async move {
            storage.delete_saved_query_folder(folder_id).await
        })
    }

    /// Move query to folder
    pub fn move_query_to_folder(
        &self,
        query_id: Uuid,
        folder_id: Option<Uuid>,
    ) -> Result<()> {
        let storage = self.storage.clone();

        self.runtime.block_on(async move {
            storage.move_query_to_folder(query_id, folder_id).await
        })
    }
}
```

### 13.5 GPUI Tab State

```rust
// src/state/tabs_state.rs

use std::sync::Arc;
use gpui::Global;
use uuid::Uuid;

use crate::models::tabs::{EditorTab, TabContent};
use crate::services::tabs::TabService;
use crate::error::Result;

/// Global tab state for GPUI
pub struct TabState {
    service: Arc<TabService>,
}

impl Global for TabState {}

impl TabState {
    /// Create a new tab state
    pub fn new(service: Arc<TabService>) -> Self {
        Self { service }
    }

    /// Get the tab service
    pub fn service(&self) -> &TabService {
        &self.service
    }

    /// Get all tabs
    pub fn tabs(&self) -> Vec<EditorTab> {
        self.service.get_all_tabs()
    }

    /// Get active tab ID
    pub fn active_tab_id(&self) -> Option<Uuid> {
        self.service.get_active_tab_id()
    }

    /// Get active tab
    pub fn active_tab(&self) -> Option<EditorTab> {
        self.service.get_active_tab()
    }

    /// Create a new query tab
    pub fn create_query_tab(
        &self,
        connection_id: Option<Uuid>,
        title: Option<String>,
        sql: Option<String>,
    ) -> Result<EditorTab> {
        self.service.create_query_tab(connection_id, title, sql)
    }

    /// Set active tab
    pub fn set_active(&self, tab_id: Uuid) -> Result<()> {
        self.service.set_active_tab(tab_id)
    }

    /// Close a tab
    pub fn close(&self, tab_id: Uuid) -> Result<Option<Uuid>> {
        self.service.close_tab(tab_id)
    }

    /// Update tab content
    pub fn update_content(
        &self,
        tab_id: Uuid,
        content: TabContent,
        is_modified: bool,
    ) -> Result<()> {
        self.service.update_tab_content(tab_id, content, is_modified)
    }

    /// Rename a tab
    pub fn rename(&self, tab_id: Uuid, new_title: String) -> Result<()> {
        self.service.rename_tab(tab_id, new_title)
    }

    /// Check for unsaved changes
    pub fn has_unsaved_changes(&self) -> bool {
        self.service.has_unsaved_changes()
    }

    /// Get modified tabs
    pub fn modified_tabs(&self) -> Vec<EditorTab> {
        self.service.get_modified_tabs()
    }
}

/// Global history state for GPUI
pub struct HistoryState {
    service: Arc<crate::services::history::HistoryService>,
}

impl Global for HistoryState {}

impl HistoryState {
    /// Create a new history state
    pub fn new(service: Arc<crate::services::history::HistoryService>) -> Self {
        Self { service }
    }

    /// Get the history service
    pub fn service(&self) -> &crate::services::history::HistoryService {
        &self.service
    }
}
```

### 13.6 Tab Bar Component

```rust
// src/ui/components/tab_bar.rs

use gpui::*;
use uuid::Uuid;
use std::sync::Arc;

use crate::models::tabs::{EditorTab, TabType};
use crate::state::tabs_state::TabState;
use crate::ui::theme::Theme;

/// Tab bar events
pub enum TabBarEvent {
    TabSelected(Uuid),
    TabClosed(Uuid),
    TabRenamed { id: Uuid, new_title: String },
    NewTabRequested,
    TabsReordered(Vec<Uuid>),
}

/// Tab bar component
pub struct TabBar {
    /// Dragged tab (if any)
    dragged_tab: Option<Uuid>,
    /// Drop target index
    drop_target: Option<usize>,
    /// Tab being renamed
    renaming_tab: Option<Uuid>,
    /// Rename buffer
    rename_buffer: String,
    /// Scroll offset for overflow tabs
    scroll_offset: f32,
    /// Context menu state
    context_menu: Option<ContextMenuState>,
}

struct ContextMenuState {
    tab_id: Uuid,
    position: Point<Pixels>,
}

impl TabBar {
    pub fn new() -> Self {
        Self {
            dragged_tab: None,
            drop_target: None,
            renaming_tab: None,
            rename_buffer: String::new(),
            scroll_offset: 0.0,
            context_menu: None,
        }
    }

    /// Start renaming a tab
    fn start_rename(&mut self, tab: &EditorTab) {
        self.renaming_tab = Some(tab.id);
        self.rename_buffer = tab.title.clone();
    }

    /// Finish renaming
    fn finish_rename(&mut self, cx: &mut Context<Self>) {
        if let Some(tab_id) = self.renaming_tab.take() {
            if !self.rename_buffer.trim().is_empty() {
                cx.emit(TabBarEvent::TabRenamed {
                    id: tab_id,
                    new_title: self.rename_buffer.clone(),
                });
            }
        }
        self.rename_buffer.clear();
    }

    /// Cancel renaming
    fn cancel_rename(&mut self) {
        self.renaming_tab = None;
        self.rename_buffer.clear();
    }

    /// Render a single tab
    fn render_tab(
        &self,
        tab: &EditorTab,
        is_active: bool,
        index: usize,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tab_id = tab.id;
        let is_modified = tab.is_modified;
        let is_pinned = tab.is_pinned;
        let is_renaming = self.renaming_tab == Some(tab_id);
        let is_drag_target = self.drop_target == Some(index);

        div()
            .id(ElementId::Name(format!("tab-{}", tab_id).into()))
            .flex()
            .items_center()
            .gap_1()
            .px_3()
            .h_9()
            .border_r_1()
            .border_color(theme.border)
            .bg(if is_active {
                theme.background
            } else {
                theme.surface_secondary
            })
            .when(is_active, |el| {
                el.border_b_2().border_color(theme.primary)
            })
            .when(is_drag_target, |el| {
                el.border_l_2().border_color(theme.primary)
            })
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            // Click handler
            .on_click(cx.listener(move |this, _event, cx| {
                cx.emit(TabBarEvent::TabSelected(tab_id));
            }))
            // Middle click to close
            .on_mouse_down(MouseButton::Middle, cx.listener(move |this, _event, cx| {
                cx.emit(TabBarEvent::TabClosed(tab_id));
            }))
            // Context menu
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &MouseDownEvent, _cx| {
                this.context_menu = Some(ContextMenuState {
                    tab_id,
                    position: event.position,
                });
            }))
            // Double click to rename
            .on_double_click(cx.listener({
                let tab_title = tab.title.clone();
                move |this, _event, _cx| {
                    this.renaming_tab = Some(tab_id);
                    this.rename_buffer = tab_title.clone();
                }
            }))
            // Drag handlers
            .draggable(tab_id)
            .on_drag_start(cx.listener(move |this, _event, _cx| {
                this.dragged_tab = Some(tab_id);
            }))
            .on_drag_over(cx.listener(move |this, _event, _cx| {
                this.drop_target = Some(index);
            }))
            .on_drag_leave(cx.listener(|this, _event, _cx| {
                this.drop_target = None;
            }))
            .on_drop(cx.listener(move |this, _event, cx| {
                if let Some(dragged_id) = this.dragged_tab.take() {
                    // Reorder tabs
                    let tab_state = cx.global::<TabState>();
                    let mut tabs = tab_state.tabs();

                    let from_idx = tabs.iter().position(|t| t.id == dragged_id);
                    let to_idx = index;

                    if let Some(from) = from_idx {
                        let tab_ids: Vec<Uuid> = {
                            let removed = tabs.remove(from);
                            tabs.insert(to_idx.min(tabs.len()), removed);
                            tabs.iter().map(|t| t.id).collect()
                        };
                        cx.emit(TabBarEvent::TabsReordered(tab_ids));
                    }
                }
                this.drop_target = None;
            }))
            .child(
                // Connection color indicator
                if let Some(_conn_id) = tab.connection_id {
                    // TODO: Get connection color from connection state
                    div()
                        .w_2()
                        .h_2()
                        .rounded_full()
                        .bg(theme.primary)
                        .flex_shrink_0()
                        .into_any_element()
                } else {
                    div().into_any_element()
                }
            )
            .child(
                // Pin indicator
                if is_pinned {
                    svg()
                        .path("icons/pin.svg")
                        .size_3()
                        .text_color(theme.text_muted)
                        .flex_shrink_0()
                        .into_any_element()
                } else {
                    div().into_any_element()
                }
            )
            .child(
                // Title or rename input
                if is_renaming {
                    // Render rename input
                    div()
                        .child(
                            // Text input would go here
                            // Using a simple div for now
                            div()
                                .px_1()
                                .border_1()
                                .border_color(theme.primary)
                                .rounded_sm()
                                .bg(theme.background)
                                .text_sm()
                                .child(self.rename_buffer.clone())
                        )
                        .into_any_element()
                } else {
                    div()
                        .text_sm()
                        .text_ellipsis()
                        .overflow_hidden()
                        .max_w_40()
                        .child(tab.title.clone())
                        .into_any_element()
                }
            )
            .child(
                // Modified indicator
                if is_modified && !is_pinned {
                    div()
                        .w_2()
                        .h_2()
                        .rounded_full()
                        .bg(theme.primary)
                        .flex_shrink_0()
                        .into_any_element()
                } else {
                    div().into_any_element()
                }
            )
            .child(
                // Close button (hidden when pinned)
                if !is_pinned {
                    div()
                        .id(ElementId::Name(format!("close-{}", tab_id).into()))
                        .flex()
                        .items_center()
                        .justify_center()
                        .w_5()
                        .h_5()
                        .rounded_sm()
                        .cursor_pointer()
                        .text_color(theme.text_muted)
                        .opacity(0.0)
                        .group_hover("tab", |style| style.opacity(1.0))
                        .hover(|style| style.bg(theme.hover).text_color(theme.text))
                        .on_click(cx.listener(move |_this, event: &ClickEvent, cx| {
                            event.stop_propagation();
                            cx.emit(TabBarEvent::TabClosed(tab_id));
                        }))
                        .child(
                            svg()
                                .path("icons/x.svg")
                                .size_3p5()
                        )
                        .into_any_element()
                } else {
                    div().into_any_element()
                }
            )
    }

    /// Render context menu
    fn render_context_menu(
        &self,
        state: &ContextMenuState,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tab_id = state.tab_id;

        div()
            .absolute()
            .left(state.position.x)
            .top(state.position.y)
            .z_index(100)
            .min_w_48()
            .py_1()
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .shadow_lg()
            .child(
                self.context_menu_item("Close", theme, cx.listener(move |this, _event, cx| {
                    this.context_menu = None;
                    cx.emit(TabBarEvent::TabClosed(tab_id));
                }))
            )
            .child(
                self.context_menu_item("Close Others", theme, cx.listener(move |this, _event, cx| {
                    this.context_menu = None;
                    let tab_state = cx.global::<TabState>();
                    let _ = tab_state.service().close_other_tabs(tab_id);
                    cx.notify();
                }))
            )
            .child(
                self.context_menu_item("Close to the Right", theme, cx.listener(move |this, _event, cx| {
                    this.context_menu = None;
                    let tab_state = cx.global::<TabState>();
                    let _ = tab_state.service().close_tabs_to_right(tab_id);
                    cx.notify();
                }))
            )
            .child(div().h_px().bg(theme.border).my_1()) // Separator
            .child(
                self.context_menu_item("Rename", theme, cx.listener(move |this, _event, cx| {
                    this.context_menu = None;
                    let tab_state = cx.global::<TabState>();
                    if let Some(tab) = tab_state.service().get_tab(tab_id) {
                        this.start_rename(&tab);
                    }
                    cx.notify();
                }))
            )
            .child(
                self.context_menu_item("Pin/Unpin", theme, cx.listener(move |this, _event, cx| {
                    this.context_menu = None;
                    let tab_state = cx.global::<TabState>();
                    let _ = tab_state.service().toggle_pin(tab_id);
                    cx.notify();
                }))
            )
    }

    fn context_menu_item(
        &self,
        label: &str,
        theme: &Theme,
        handler: impl Fn(&mut Self, &ClickEvent, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        div()
            .px_3()
            .py_1p5()
            .text_sm()
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            .on_click(handler)
            .child(label.to_string())
    }
}

impl EventEmitter<TabBarEvent> for TabBar {}

impl Render for TabBar {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let tab_state = cx.global::<TabState>();
        let tabs = tab_state.tabs();
        let active_id = tab_state.active_tab_id();

        div()
            .flex()
            .items_center()
            .h_9()
            .bg(theme.surface)
            .border_b_1()
            .border_color(theme.border)
            .overflow_hidden()
            // Tabs container
            .child(
                div()
                    .id("tabs-container")
                    .flex()
                    .flex_1()
                    .overflow_x_auto()
                    .children(
                        tabs.iter().enumerate().map(|(index, tab)| {
                            self.render_tab(
                                tab,
                                Some(tab.id) == active_id,
                                index,
                                &theme,
                                cx,
                            )
                        })
                    )
            )
            // New tab button
            .child(
                div()
                    .id("new-tab-btn")
                    .flex()
                    .items_center()
                    .justify_center()
                    .w_9()
                    .h_9()
                    .cursor_pointer()
                    .text_color(theme.text_muted)
                    .hover(|style| style.text_color(theme.text))
                    .on_click(cx.listener(|_this, _event, cx| {
                        cx.emit(TabBarEvent::NewTabRequested);
                    }))
                    .child(
                        svg()
                            .path("icons/plus.svg")
                            .size_4()
                    )
            )
            // Context menu (if open)
            .when_some(self.context_menu.as_ref(), |el, state| {
                el.child(self.render_context_menu(state, &theme, cx))
            })
    }
}
```

### 13.7 History Panel Component

```rust
// src/ui/components/history_panel.rs

use gpui::*;
use uuid::Uuid;
use std::sync::Arc;

use crate::models::history::{QueryHistoryEntry, HistorySearchParams};
use crate::state::tabs_state::HistoryState;
use crate::ui::theme::Theme;

/// Events emitted by history panel
pub enum HistoryPanelEvent {
    OpenInNewTab { sql: String, connection_id: Uuid },
    CopyToClipboard(String),
    ExecuteQuery { sql: String, connection_id: Uuid },
}

/// History panel component
pub struct HistoryPanel {
    /// Current search text
    search_text: String,
    /// Filter settings
    show_favorites_only: bool,
    show_errors_only: bool,
    /// Selected entry IDs
    selected_ids: Vec<i64>,
    /// Loaded entries
    entries: Vec<QueryHistoryEntry>,
    /// Whether more entries are available
    has_more: bool,
    /// Current page offset
    offset: u32,
    /// Items per page
    page_size: u32,
    /// Is loading
    is_loading: bool,
    /// Connection filter
    connection_id: Option<Uuid>,
}

impl HistoryPanel {
    pub fn new(connection_id: Option<Uuid>) -> Self {
        Self {
            search_text: String::new(),
            show_favorites_only: false,
            show_errors_only: false,
            selected_ids: Vec::new(),
            entries: Vec::new(),
            has_more: false,
            offset: 0,
            page_size: 50,
            is_loading: false,
            connection_id,
        }
    }

    /// Load history entries
    pub fn load(&mut self, cx: &mut Context<Self>) {
        self.is_loading = true;
        self.offset = 0;

        let history_state = cx.global::<HistoryState>();

        let params = HistorySearchParams {
            connection_id: self.connection_id,
            search_text: if self.search_text.is_empty() {
                None
            } else {
                Some(self.search_text.clone())
            },
            favorites_only: self.show_favorites_only,
            errors_only: self.show_errors_only,
            limit: self.page_size,
            offset: 0,
            ..Default::default()
        };

        match history_state.service().search_history(params) {
            Ok(result) => {
                self.entries = result.entries;
                self.has_more = result.has_more;
            }
            Err(e) => {
                tracing::error!("Failed to load history: {}", e);
                self.entries.clear();
                self.has_more = false;
            }
        }

        self.is_loading = false;
        cx.notify();
    }

    /// Load more entries
    pub fn load_more(&mut self, cx: &mut Context<Self>) {
        if !self.has_more || self.is_loading {
            return;
        }

        self.is_loading = true;
        self.offset += self.page_size;

        let history_state = cx.global::<HistoryState>();

        let params = HistorySearchParams {
            connection_id: self.connection_id,
            search_text: if self.search_text.is_empty() {
                None
            } else {
                Some(self.search_text.clone())
            },
            favorites_only: self.show_favorites_only,
            errors_only: self.show_errors_only,
            limit: self.page_size,
            offset: self.offset,
            ..Default::default()
        };

        match history_state.service().search_history(params) {
            Ok(result) => {
                self.entries.extend(result.entries);
                self.has_more = result.has_more;
            }
            Err(e) => {
                tracing::error!("Failed to load more history: {}", e);
            }
        }

        self.is_loading = false;
        cx.notify();
    }

    /// Toggle favorite status
    pub fn toggle_favorite(&mut self, entry_id: i64, cx: &mut Context<Self>) {
        let history_state = cx.global::<HistoryState>();

        match history_state.service().toggle_favorite(entry_id) {
            Ok(is_favorited) => {
                if let Some(entry) = self.entries.iter_mut().find(|e| e.id == entry_id) {
                    entry.favorited = is_favorited;
                }
            }
            Err(e) => {
                tracing::error!("Failed to toggle favorite: {}", e);
            }
        }

        cx.notify();
    }

    /// Delete selected entries
    pub fn delete_selected(&mut self, cx: &mut Context<Self>) {
        if self.selected_ids.is_empty() {
            return;
        }

        let history_state = cx.global::<HistoryState>();
        let ids = self.selected_ids.clone();

        match history_state.service().delete_entries(ids.clone()) {
            Ok(_) => {
                self.entries.retain(|e| !ids.contains(&e.id));
                self.selected_ids.clear();
            }
            Err(e) => {
                tracing::error!("Failed to delete entries: {}", e);
            }
        }

        cx.notify();
    }

    /// Toggle selection
    fn toggle_selection(&mut self, entry_id: i64, multi_select: bool) {
        if multi_select {
            if let Some(pos) = self.selected_ids.iter().position(|&id| id == entry_id) {
                self.selected_ids.remove(pos);
            } else {
                self.selected_ids.push(entry_id);
            }
        } else {
            self.selected_ids = vec![entry_id];
        }
    }

    /// Render the search header
    fn render_header(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .gap_2()
            .p_2()
            .border_b_1()
            .border_color(theme.border)
            // Search box
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .px_2()
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .child(
                        svg()
                            .path("icons/search.svg")
                            .size_3p5()
                            .text_color(theme.text_muted)
                    )
                    .child(
                        // Text input placeholder
                        div()
                            .flex_1()
                            .px_2()
                            .py_1p5()
                            .text_sm()
                            .child(
                                if self.search_text.is_empty() {
                                    "Search history...".to_string()
                                } else {
                                    self.search_text.clone()
                                }
                            )
                    )
            )
            // Filter buttons
            .child(
                div()
                    .flex()
                    .gap_1()
                    // Favorites filter
                    .child(
                        self.render_filter_button(
                            "icons/star.svg",
                            self.show_favorites_only,
                            "Show favorites only",
                            theme,
                            cx.listener(|this, _event, cx| {
                                this.show_favorites_only = !this.show_favorites_only;
                                this.load(cx);
                            }),
                        )
                    )
                    // Errors filter
                    .child(
                        self.render_filter_button(
                            "icons/alert-circle.svg",
                            self.show_errors_only,
                            "Show errors only",
                            theme,
                            cx.listener(|this, _event, cx| {
                                this.show_errors_only = !this.show_errors_only;
                                this.load(cx);
                            }),
                        )
                    )
                    // Delete selected
                    .when(!self.selected_ids.is_empty(), |el| {
                        el.child(
                            self.render_filter_button(
                                "icons/trash-2.svg",
                                false,
                                "Delete selected",
                                theme,
                                cx.listener(|this, _event, cx| {
                                    this.delete_selected(cx);
                                }),
                            )
                        )
                    })
            )
    }

    fn render_filter_button(
        &self,
        icon: &str,
        is_active: bool,
        tooltip: &str,
        theme: &Theme,
        handler: impl Fn(&mut Self, &ClickEvent, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .w_7()
            .h_7()
            .border_1()
            .border_color(if is_active { theme.primary } else { theme.border })
            .rounded_md()
            .cursor_pointer()
            .bg(if is_active { theme.primary } else { Hsla::transparent_black() })
            .text_color(if is_active {
                Hsla::white()
            } else {
                theme.text_muted
            })
            .hover(|style| {
                style
                    .bg(if is_active { theme.primary } else { theme.hover })
                    .text_color(if is_active { Hsla::white() } else { theme.text })
            })
            .on_click(handler)
            .child(
                svg()
                    .path(icon)
                    .size_3p5()
            )
    }

    /// Render a history entry
    fn render_entry(
        &self,
        entry: &QueryHistoryEntry,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let entry_id = entry.id;
        let connection_id = entry.connection_id;
        let sql = entry.sql.clone();
        let is_selected = self.selected_ids.contains(&entry_id);
        let is_error = entry.error.is_some();

        div()
            .id(ElementId::Name(format!("history-{}", entry_id).into()))
            .py_3()
            .px_3()
            .border_b_1()
            .border_color(theme.border)
            .cursor_pointer()
            .bg(if is_selected { theme.selected } else { Hsla::transparent_black() })
            .when(is_error, |el| {
                el.border_l_3().border_color(hsla(0.0, 0.84, 0.6, 1.0))
            })
            .hover(|style| style.bg(theme.hover))
            // Click to select
            .on_click(cx.listener(move |this, event: &ClickEvent, cx| {
                let multi = event.modifiers.command || event.modifiers.control;
                this.toggle_selection(entry_id, multi);
                cx.notify();
            }))
            // Double click to open
            .on_double_click(cx.listener({
                let sql = sql.clone();
                move |_this, _event, cx| {
                    cx.emit(HistoryPanelEvent::OpenInNewTab {
                        sql: sql.clone(),
                        connection_id,
                    });
                }
            }))
            // Header row
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .mb_1()
                    .text_xs()
                    .text_color(theme.text_muted)
                    // Time
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(svg().path("icons/clock.svg").size_3())
                            .child(entry.relative_time())
                    )
                    // Duration
                    .child(entry.formatted_duration())
                    // Rows affected
                    .when_some(entry.rows_affected, |el, rows| {
                        el.child(format!("{} rows", rows))
                    })
            )
            // SQL content
            .child(
                div()
                    .font_family("monospace")
                    .text_sm()
                    .text_color(theme.text)
                    .line_clamp(2)
                    .child(entry.truncated_sql(200))
            )
            // Error message (if any)
            .when_some(entry.error.as_ref(), |el, error| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .mt_1()
                        .text_xs()
                        .text_color(hsla(0.0, 0.84, 0.6, 1.0))
                        .child(svg().path("icons/alert-circle.svg").size_3())
                        .child(error.chars().take(100).collect::<String>())
                )
            })
            // Action buttons (show on hover)
            .child(
                div()
                    .flex()
                    .gap_1()
                    .mt_2()
                    .opacity(0.0)
                    .group_hover("entry", |style| style.opacity(1.0))
                    // Favorite button
                    .child(
                        self.render_action_button(
                            if entry.favorited { "icons/star-filled.svg" } else { "icons/star.svg" },
                            if entry.favorited { "Remove from favorites" } else { "Add to favorites" },
                            if entry.favorited { Some(hsla(0.13, 0.93, 0.66, 1.0)) } else { None },
                            theme,
                            cx.listener(move |this, _event, cx| {
                                this.toggle_favorite(entry_id, cx);
                            }),
                        )
                    )
                    // Copy button
                    .child({
                        let sql = sql.clone();
                        self.render_action_button(
                            "icons/copy.svg",
                            "Copy SQL",
                            None,
                            theme,
                            cx.listener(move |_this, _event, cx| {
                                cx.emit(HistoryPanelEvent::CopyToClipboard(sql.clone()));
                            }),
                        )
                    })
                    // Open in new tab
                    .child({
                        let sql = sql.clone();
                        self.render_action_button(
                            "icons/external-link.svg",
                            "Open in new tab",
                            None,
                            theme,
                            cx.listener(move |_this, _event, cx| {
                                cx.emit(HistoryPanelEvent::OpenInNewTab {
                                    sql: sql.clone(),
                                    connection_id,
                                });
                            }),
                        )
                    })
                    // Execute
                    .child({
                        let sql = sql.clone();
                        self.render_action_button(
                            "icons/play.svg",
                            "Execute",
                            None,
                            theme,
                            cx.listener(move |_this, _event, cx| {
                                cx.emit(HistoryPanelEvent::ExecuteQuery {
                                    sql: sql.clone(),
                                    connection_id,
                                });
                            }),
                        )
                    })
            )
    }

    fn render_action_button(
        &self,
        icon: &str,
        tooltip: &str,
        color: Option<Hsla>,
        theme: &Theme,
        handler: impl Fn(&mut Self, &ClickEvent, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .w_6()
            .h_6()
            .rounded_sm()
            .cursor_pointer()
            .text_color(color.unwrap_or(theme.text_muted))
            .hover(|style| style.bg(theme.hover).text_color(color.unwrap_or(theme.text)))
            .on_click(handler)
            .child(
                svg().path(icon).size_3p5()
            )
    }

    /// Render load more button
    fn render_load_more(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .py_3()
            .text_center()
            .text_sm()
            .text_color(theme.primary)
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            .on_click(cx.listener(|this, _event, cx| {
                this.load_more(cx);
            }))
            .child(
                if self.is_loading {
                    "Loading..."
                } else {
                    "Load more"
                }
            )
    }
}

impl EventEmitter<HistoryPanelEvent> for HistoryPanel {}

impl Render for HistoryPanel {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();

        div()
            .flex()
            .flex_col()
            .h_full()
            .bg(theme.surface)
            // Header with search and filters
            .child(self.render_header(&theme, cx))
            // History list
            .child(
                div()
                    .flex_1()
                    .overflow_y_auto()
                    .when(self.is_loading && self.entries.is_empty(), |el| {
                        el.child(
                            div()
                                .p_8()
                                .text_center()
                                .text_color(theme.text_muted)
                                .child("Loading history...")
                        )
                    })
                    .when(!self.is_loading && self.entries.is_empty(), |el| {
                        el.child(
                            div()
                                .p_8()
                                .text_center()
                                .text_color(theme.text_muted)
                                .child("No history found")
                        )
                    })
                    .when(!self.entries.is_empty(), |el| {
                        el.children(
                            self.entries.iter().map(|entry| {
                                self.render_entry(entry, &theme, cx)
                            })
                        )
                        .when(self.has_more, |el| {
                            el.child(self.render_load_more(&theme, cx))
                        })
                    })
            )
    }
}
```

### 13.8 Saved Queries Panel

```rust
// src/ui/components/saved_queries_panel.rs

use gpui::*;
use uuid::Uuid;

use crate::models::history::{SavedQuery, SavedQueryFolder};
use crate::state::tabs_state::HistoryState;
use crate::ui::theme::Theme;

/// Events emitted by saved queries panel
pub enum SavedQueriesEvent {
    OpenQuery(SavedQuery),
    ExecuteQuery(SavedQuery),
    DeleteQuery(Uuid),
    RenameQuery { id: Uuid, new_name: String },
    CreateFolder(String),
    MoveToFolder { query_id: Uuid, folder_id: Option<Uuid> },
}

/// Saved queries panel component
pub struct SavedQueriesPanel {
    /// Search text
    search_text: String,
    /// Loaded queries
    queries: Vec<SavedQuery>,
    /// Loaded folders
    folders: Vec<SavedQueryFolder>,
    /// Expanded folder IDs
    expanded_folders: Vec<Uuid>,
    /// Selected query ID
    selected_query: Option<Uuid>,
    /// Query being renamed
    renaming_query: Option<Uuid>,
    /// Rename buffer
    rename_buffer: String,
    /// Connection filter
    connection_id: Option<Uuid>,
    /// Creating new folder
    creating_folder: bool,
    /// New folder name
    new_folder_name: String,
}

impl SavedQueriesPanel {
    pub fn new(connection_id: Option<Uuid>) -> Self {
        Self {
            search_text: String::new(),
            queries: Vec::new(),
            folders: Vec::new(),
            expanded_folders: Vec::new(),
            selected_query: None,
            renaming_query: None,
            rename_buffer: String::new(),
            connection_id,
            creating_folder: false,
            new_folder_name: String::new(),
        }
    }

    /// Load saved queries and folders
    pub fn load(&mut self, cx: &mut Context<Self>) {
        let history_state = cx.global::<HistoryState>();

        match history_state.service().get_folders() {
            Ok(folders) => self.folders = folders,
            Err(e) => tracing::error!("Failed to load folders: {}", e),
        }

        match history_state.service().get_saved_queries(self.connection_id) {
            Ok(queries) => self.queries = queries,
            Err(e) => tracing::error!("Failed to load saved queries: {}", e),
        }

        cx.notify();
    }

    /// Filter queries by search text
    fn filtered_queries(&self) -> Vec<&SavedQuery> {
        if self.search_text.is_empty() {
            self.queries.iter().collect()
        } else {
            let search = self.search_text.to_lowercase();
            self.queries.iter()
                .filter(|q| {
                    q.name.to_lowercase().contains(&search) ||
                    q.sql.to_lowercase().contains(&search) ||
                    q.tags.iter().any(|t| t.to_lowercase().contains(&search))
                })
                .collect()
        }
    }

    /// Get queries in a folder
    fn queries_in_folder(&self, folder_id: Option<Uuid>) -> Vec<&SavedQuery> {
        self.filtered_queries()
            .into_iter()
            .filter(|q| q.folder_id == folder_id)
            .collect()
    }

    /// Toggle folder expansion
    fn toggle_folder(&mut self, folder_id: Uuid) {
        if let Some(pos) = self.expanded_folders.iter().position(|&id| id == folder_id) {
            self.expanded_folders.remove(pos);
        } else {
            self.expanded_folders.push(folder_id);
        }
    }

    /// Start creating a new folder
    fn start_create_folder(&mut self) {
        self.creating_folder = true;
        self.new_folder_name.clear();
    }

    /// Finish creating folder
    fn finish_create_folder(&mut self, cx: &mut Context<Self>) {
        if !self.new_folder_name.trim().is_empty() {
            cx.emit(SavedQueriesEvent::CreateFolder(self.new_folder_name.clone()));
        }
        self.creating_folder = false;
        self.new_folder_name.clear();
    }

    /// Render a folder
    fn render_folder(
        &self,
        folder: &SavedQueryFolder,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let folder_id = folder.id;
        let is_expanded = self.expanded_folders.contains(&folder_id);
        let queries = self.queries_in_folder(Some(folder_id));
        let query_count = queries.len();

        div()
            // Folder header
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1p5()
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .on_click(cx.listener(move |this, _event, _cx| {
                        this.toggle_folder(folder_id);
                    }))
                    // Expand/collapse icon
                    .child(
                        svg()
                            .path(if is_expanded { "icons/chevron-down.svg" } else { "icons/chevron-right.svg" })
                            .size_4()
                            .text_color(theme.text_muted)
                    )
                    // Folder icon
                    .child(
                        svg()
                            .path(if is_expanded { "icons/folder-open.svg" } else { "icons/folder.svg" })
                            .size_4()
                            .text_color(theme.text_muted)
                    )
                    // Folder name
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .child(folder.name.clone())
                    )
                    // Query count
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(format!("{}", query_count))
                    )
            )
            // Folder contents (if expanded)
            .when(is_expanded, |el| {
                el.child(
                    div()
                        .pl_6()
                        .children(
                            queries.iter().map(|query| {
                                self.render_query(query, theme, cx)
                            })
                        )
                )
            })
    }

    /// Render a query item
    fn render_query(
        &self,
        query: &SavedQuery,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let query_id = query.id;
        let is_selected = self.selected_query == Some(query_id);
        let is_renaming = self.renaming_query == Some(query_id);
        let query_clone = query.clone();

        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1p5()
            .cursor_pointer()
            .bg(if is_selected { theme.selected } else { Hsla::transparent_black() })
            .hover(|style| style.bg(theme.hover))
            .on_click(cx.listener(move |this, _event, _cx| {
                this.selected_query = Some(query_id);
            }))
            .on_double_click(cx.listener({
                let query = query_clone.clone();
                move |_this, _event, cx| {
                    cx.emit(SavedQueriesEvent::OpenQuery(query.clone()));
                }
            }))
            // Query icon
            .child(
                svg()
                    .path("icons/file-code.svg")
                    .size_4()
                    .text_color(theme.text_muted)
            )
            // Query name or rename input
            .child(
                if is_renaming {
                    div()
                        .flex_1()
                        .px_1()
                        .border_1()
                        .border_color(theme.primary)
                        .rounded_sm()
                        .bg(theme.background)
                        .text_sm()
                        .child(self.rename_buffer.clone())
                        .into_any_element()
                } else {
                    div()
                        .flex_1()
                        .text_sm()
                        .text_ellipsis()
                        .overflow_hidden()
                        .child(query.name.clone())
                        .into_any_element()
                }
            )
            // Tags
            .when(!query.tags.is_empty(), |el| {
                el.child(
                    div()
                        .flex()
                        .gap_1()
                        .children(
                            query.tags.iter().take(2).map(|tag| {
                                div()
                                    .px_1()
                                    .py_px()
                                    .text_xs()
                                    .bg(theme.surface_secondary)
                                    .rounded_sm()
                                    .child(tag.clone())
                            })
                        )
                )
            })
            // Keyboard shortcut
            .when_some(query.keyboard_shortcut.as_ref(), |el, shortcut| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(theme.text_muted)
                        .child(shortcut.clone())
                )
            })
    }

    /// Render toolbar
    fn render_toolbar(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .p_2()
            .border_b_1()
            .border_color(theme.border)
            // Search box
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .px_2()
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .child(
                        svg()
                            .path("icons/search.svg")
                            .size_3p5()
                            .text_color(theme.text_muted)
                    )
                    .child(
                        div()
                            .flex_1()
                            .px_2()
                            .py_1p5()
                            .text_sm()
                            .child(
                                if self.search_text.is_empty() {
                                    "Search saved queries...".to_string()
                                } else {
                                    self.search_text.clone()
                                }
                            )
                    )
            )
            // New folder button
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w_7()
                    .h_7()
                    .rounded_md()
                    .cursor_pointer()
                    .text_color(theme.text_muted)
                    .hover(|style| style.bg(theme.hover).text_color(theme.text))
                    .on_click(cx.listener(|this, _event, _cx| {
                        this.start_create_folder();
                    }))
                    .child(
                        svg()
                            .path("icons/folder-plus.svg")
                            .size_4()
                    )
            )
    }
}

impl EventEmitter<SavedQueriesEvent> for SavedQueriesPanel {}

impl Render for SavedQueriesPanel {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let root_queries = self.queries_in_folder(None);

        div()
            .flex()
            .flex_col()
            .h_full()
            .bg(theme.surface)
            // Toolbar
            .child(self.render_toolbar(&theme, cx))
            // New folder input (if creating)
            .when(self.creating_folder, |el| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py_1p5()
                        .border_b_1()
                        .border_color(theme.border)
                        .child(
                            svg()
                                .path("icons/folder.svg")
                                .size_4()
                                .text_color(theme.text_muted)
                        )
                        .child(
                            div()
                                .flex_1()
                                .px_1()
                                .border_1()
                                .border_color(theme.primary)
                                .rounded_sm()
                                .bg(theme.background)
                                .text_sm()
                                .child(self.new_folder_name.clone())
                        )
                )
            })
            // Content
            .child(
                div()
                    .flex_1()
                    .overflow_y_auto()
                    // Folders
                    .children(
                        self.folders.iter().map(|folder| {
                            self.render_folder(folder, &theme, cx)
                        })
                    )
                    // Root queries (no folder)
                    .children(
                        root_queries.iter().map(|query| {
                            self.render_query(query, &theme, cx)
                        })
                    )
                    // Empty state
                    .when(self.folders.is_empty() && root_queries.is_empty(), |el| {
                        el.child(
                            div()
                                .p_8()
                                .text_center()
                                .text_color(theme.text_muted)
                                .child("No saved queries")
                                .child(
                                    div()
                                        .mt_2()
                                        .text_sm()
                                        .child("Save queries from the editor for quick access")
                                )
                        )
                    })
            )
    }
}
```

### 13.9 Unsaved Changes Dialog

```rust
// src/ui/dialogs/unsaved_changes_dialog.rs

use gpui::*;
use uuid::Uuid;

use crate::models::tabs::EditorTab;
use crate::ui::theme::Theme;

/// Result of unsaved changes dialog
pub enum UnsavedChangesResult {
    Save,
    DontSave,
    Cancel,
}

/// Events from unsaved changes dialog
pub enum UnsavedChangesEvent {
    Completed(UnsavedChangesResult),
}

/// Dialog for confirming unsaved changes
pub struct UnsavedChangesDialog {
    /// Tabs with unsaved changes
    tabs: Vec<EditorTab>,
    /// Whether to show "Save All" option
    show_save_all: bool,
}

impl UnsavedChangesDialog {
    /// Create dialog for a single tab
    pub fn for_tab(tab: EditorTab) -> Self {
        Self {
            tabs: vec![tab],
            show_save_all: false,
        }
    }

    /// Create dialog for multiple tabs
    pub fn for_tabs(tabs: Vec<EditorTab>) -> Self {
        Self {
            show_save_all: tabs.len() > 1,
            tabs,
        }
    }
}

impl EventEmitter<UnsavedChangesEvent> for UnsavedChangesDialog {}

impl Render for UnsavedChangesDialog {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();

        let title = if self.tabs.len() == 1 {
            format!("\"{}\" has unsaved changes", self.tabs[0].title)
        } else {
            format!("{} files have unsaved changes", self.tabs.len())
        };

        div()
            .flex()
            .flex_col()
            .w_96()
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .shadow_xl()
            // Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .p_4()
                    .border_b_1()
                    .border_color(theme.border)
                    // Warning icon
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w_10()
                            .h_10()
                            .rounded_full()
                            .bg(hsla(0.13, 0.93, 0.66, 0.1))
                            .child(
                                svg()
                                    .path("icons/alert-triangle.svg")
                                    .size_5()
                                    .text_color(hsla(0.13, 0.93, 0.66, 1.0))
                            )
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(
                                div()
                                    .text_base()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(title)
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.text_muted)
                                    .child("Do you want to save your changes?")
                            )
                    )
            )
            // File list (if multiple)
            .when(self.tabs.len() > 1, |el| {
                el.child(
                    div()
                        .max_h_48()
                        .overflow_y_auto()
                        .p_2()
                        .children(
                            self.tabs.iter().map(|tab| {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_2()
                                    .py_1()
                                    .child(
                                        svg()
                                            .path("icons/file.svg")
                                            .size_4()
                                            .text_color(theme.text_muted)
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .child(tab.title.clone())
                                    )
                            })
                        )
                )
            })
            // Actions
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .p_4()
                    .bg(theme.surface_secondary)
                    // Don't Save button
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .cursor_pointer()
                            .text_sm()
                            .hover(|style| style.bg(theme.hover))
                            .on_click(cx.listener(|_this, _event, cx| {
                                cx.emit(UnsavedChangesEvent::Completed(UnsavedChangesResult::DontSave));
                            }))
                            .child("Don't Save")
                    )
                    // Cancel button
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .cursor_pointer()
                            .text_sm()
                            .hover(|style| style.bg(theme.hover))
                            .on_click(cx.listener(|_this, _event, cx| {
                                cx.emit(UnsavedChangesEvent::Completed(UnsavedChangesResult::Cancel));
                            }))
                            .child("Cancel")
                    )
                    // Save button
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .rounded_md()
                            .bg(theme.primary)
                            .text_color(Hsla::white())
                            .cursor_pointer()
                            .text_sm()
                            .hover(|style| style.bg(theme.primary_hover))
                            .on_click(cx.listener(|_this, _event, cx| {
                                cx.emit(UnsavedChangesEvent::Completed(UnsavedChangesResult::Save));
                            }))
                            .child(if self.show_save_all { "Save All" } else { "Save" })
                    )
            )
    }
}
```

### 13.10 Keyboard Shortcuts

```rust
// src/input/tab_actions.rs

use gpui::*;

use crate::state::tabs_state::TabState;

/// Tab-related actions
pub struct TabActions;

impl TabActions {
    /// Register tab keyboard shortcuts
    pub fn register(cx: &mut App) {
        // Cmd/Ctrl+T: New tab
        cx.bind_keys([
            KeyBinding::new("cmd-t", NewTab, None),
            KeyBinding::new("ctrl-t", NewTab, None),
        ]);

        // Cmd/Ctrl+W: Close tab
        cx.bind_keys([
            KeyBinding::new("cmd-w", CloseTab, None),
            KeyBinding::new("ctrl-w", CloseTab, None),
        ]);

        // Cmd/Ctrl+Shift+T: Reopen closed tab
        cx.bind_keys([
            KeyBinding::new("cmd-shift-t", ReopenClosedTab, None),
            KeyBinding::new("ctrl-shift-t", ReopenClosedTab, None),
        ]);

        // Cmd/Ctrl+Tab: Next tab
        cx.bind_keys([
            KeyBinding::new("ctrl-tab", NextTab, None),
        ]);

        // Cmd/Ctrl+Shift+Tab: Previous tab
        cx.bind_keys([
            KeyBinding::new("ctrl-shift-tab", PreviousTab, None),
        ]);

        // Cmd/Ctrl+1-9: Go to tab N
        for i in 1..=9 {
            cx.bind_keys([
                KeyBinding::new(&format!("cmd-{}", i), GoToTab(i), None),
                KeyBinding::new(&format!("ctrl-{}", i), GoToTab(i), None),
            ]);
        }

        // Cmd/Ctrl+S: Save current tab
        cx.bind_keys([
            KeyBinding::new("cmd-s", SaveTab, None),
            KeyBinding::new("ctrl-s", SaveTab, None),
        ]);

        // Cmd/Ctrl+Shift+S: Save all tabs
        cx.bind_keys([
            KeyBinding::new("cmd-shift-s", SaveAllTabs, None),
            KeyBinding::new("ctrl-shift-s", SaveAllTabs, None),
        ]);
    }
}

/// Action: Create new tab
#[derive(Clone, PartialEq)]
pub struct NewTab;

impl_actions!(tab_actions, [NewTab]);

impl NewTab {
    pub fn handle(cx: &mut App) {
        let tab_state = cx.global::<TabState>();
        if let Err(e) = tab_state.create_query_tab(None, None, None) {
            tracing::error!("Failed to create new tab: {}", e);
        }
    }
}

/// Action: Close current tab
#[derive(Clone, PartialEq)]
pub struct CloseTab;

impl_actions!(tab_actions, [CloseTab]);

/// Action: Reopen last closed tab
#[derive(Clone, PartialEq)]
pub struct ReopenClosedTab;

impl_actions!(tab_actions, [ReopenClosedTab]);

/// Action: Next tab
#[derive(Clone, PartialEq)]
pub struct NextTab;

impl_actions!(tab_actions, [NextTab]);

impl NextTab {
    pub fn handle(cx: &mut App) {
        let tab_state = cx.global::<TabState>();
        if let Err(e) = tab_state.service().next_tab() {
            tracing::error!("Failed to switch to next tab: {}", e);
        }
    }
}

/// Action: Previous tab
#[derive(Clone, PartialEq)]
pub struct PreviousTab;

impl_actions!(tab_actions, [PreviousTab]);

impl PreviousTab {
    pub fn handle(cx: &mut App) {
        let tab_state = cx.global::<TabState>();
        if let Err(e) = tab_state.service().previous_tab() {
            tracing::error!("Failed to switch to previous tab: {}", e);
        }
    }
}

/// Action: Go to specific tab
#[derive(Clone, PartialEq)]
pub struct GoToTab(pub usize);

impl_actions!(tab_actions, [GoToTab]);

impl GoToTab {
    pub fn handle(&self, cx: &mut App) {
        let tab_state = cx.global::<TabState>();
        let tabs = tab_state.tabs();

        if self.0 > 0 && self.0 <= tabs.len() {
            let tab_id = tabs[self.0 - 1].id;
            if let Err(e) = tab_state.set_active(tab_id) {
                tracing::error!("Failed to switch to tab: {}", e);
            }
        }
    }
}

/// Action: Save current tab
#[derive(Clone, PartialEq)]
pub struct SaveTab;

impl_actions!(tab_actions, [SaveTab]);

/// Action: Save all tabs
#[derive(Clone, PartialEq)]
pub struct SaveAllTabs;

impl_actions!(tab_actions, [SaveAllTabs]);
```

### 13.11 Storage Schema Updates

```sql
-- Additional tables for tabs and history

-- Tabs table
CREATE TABLE IF NOT EXISTS tabs (
    id TEXT PRIMARY KEY,
    connection_id TEXT,
    tab_type TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,  -- JSON serialized TabContent
    is_modified INTEGER NOT NULL DEFAULT 0,
    is_pinned INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    last_accessed_at TEXT NOT NULL,
    FOREIGN KEY (connection_id) REFERENCES connections(id)
);

-- Active tab tracking
CREATE TABLE IF NOT EXISTS app_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Query history table
CREATE TABLE IF NOT EXISTS query_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    connection_id TEXT NOT NULL,
    connection_name TEXT NOT NULL,
    database_name TEXT NOT NULL,
    sql TEXT NOT NULL,
    executed_at TEXT NOT NULL,
    duration_ms INTEGER,
    rows_affected INTEGER,
    error TEXT,
    favorited INTEGER NOT NULL DEFAULT 0,
    tags TEXT,  -- JSON array
    FOREIGN KEY (connection_id) REFERENCES connections(id)
);

CREATE INDEX IF NOT EXISTS idx_history_connection ON query_history(connection_id);
CREATE INDEX IF NOT EXISTS idx_history_executed_at ON query_history(executed_at DESC);
CREATE INDEX IF NOT EXISTS idx_history_favorited ON query_history(favorited) WHERE favorited = 1;

-- Saved queries table
CREATE TABLE IF NOT EXISTS saved_queries (
    id TEXT PRIMARY KEY,
    connection_id TEXT,
    name TEXT NOT NULL,
    description TEXT,
    sql TEXT NOT NULL,
    folder_id TEXT,
    tags TEXT,  -- JSON array
    keyboard_shortcut TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (connection_id) REFERENCES connections(id),
    FOREIGN KEY (folder_id) REFERENCES saved_query_folders(id)
);

CREATE INDEX IF NOT EXISTS idx_saved_queries_connection ON saved_queries(connection_id);
CREATE INDEX IF NOT EXISTS idx_saved_queries_folder ON saved_queries(folder_id);

-- Saved query folders table
CREATE TABLE IF NOT EXISTS saved_query_folders (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    parent_id TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    icon TEXT,
    color TEXT,
    FOREIGN KEY (parent_id) REFERENCES saved_query_folders(id)
);

-- Recently closed tabs (for reopening)
CREATE TABLE IF NOT EXISTS recently_closed_tabs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tab_data TEXT NOT NULL,  -- JSON serialized EditorTab
    closed_at TEXT NOT NULL
);

-- Limit recently closed tabs
CREATE TRIGGER IF NOT EXISTS limit_recently_closed
AFTER INSERT ON recently_closed_tabs
BEGIN
    DELETE FROM recently_closed_tabs
    WHERE id NOT IN (
        SELECT id FROM recently_closed_tabs
        ORDER BY closed_at DESC
        LIMIT 20
    );
END;
```

## Acceptance Criteria

1. **Tab Management**
   - Create new query tabs with unique names
   - Switch between tabs with click or keyboard (Ctrl+Tab, Ctrl+1-9)
   - Close tabs with X button, middle-click, or Ctrl+W
   - Prompt for unsaved changes before closing
   - Drag and drop to reorder tabs
   - Context menu with Close, Close Others, Close to Right, Pin/Unpin
   - Pinned tabs cannot be closed with mass close operations

2. **Tab Persistence**
   - Tabs persist across application restarts
   - Active tab restored on startup
   - Tab content (SQL, cursor position, scroll) preserved
   - Recently closed tabs can be reopened (Ctrl+Shift+T)

3. **Tab Renaming**
   - Double-click tab to rename
   - Enter to confirm, Escape to cancel
   - Names persist to storage

4. **Query History**
   - All executed queries recorded with timing
   - Search history by SQL text
   - Filter by favorites, errors, date range
   - Infinite scroll pagination
   - Relative time display ("5 minutes ago")

5. **History Actions**
   - Toggle favorite status
   - Copy SQL to clipboard
   - Open in new tab (double-click or button)
   - Execute directly
   - Delete entries
   - Tag management

6. **Saved Queries**
   - Save queries as named snippets
   - Organize in folders
   - Search and filter saved queries
   - Open saved query in new tab
   - Assign keyboard shortcuts
   - Tag support

## Testing Instructions

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_create_query_tab() {
        let (service, _storage) = create_test_tab_service();

        let tab = service.create_query_tab(None, None, None).unwrap();

        assert!(tab.title.starts_with("Query "));
        assert_eq!(tab.tab_type, TabType::Query);
        assert!(!tab.is_modified);
    }

    #[test]
    fn test_tab_numbering() {
        let (service, _storage) = create_test_tab_service();

        let tab1 = service.create_query_tab(None, None, None).unwrap();
        let tab2 = service.create_query_tab(None, None, None).unwrap();
        let tab3 = service.create_query_tab(None, None, None).unwrap();

        assert_eq!(tab1.title, "Query 1");
        assert_eq!(tab2.title, "Query 2");
        assert_eq!(tab3.title, "Query 3");
    }

    #[test]
    fn test_close_tab_activates_adjacent() {
        let (service, _storage) = create_test_tab_service();

        let tab1 = service.create_query_tab(None, None, None).unwrap();
        let tab2 = service.create_query_tab(None, None, None).unwrap();
        let tab3 = service.create_query_tab(None, None, None).unwrap();

        // Close middle tab
        service.set_active_tab(tab2.id).unwrap();
        let new_active = service.close_tab(tab2.id).unwrap();

        // Should activate tab3 (the next one)
        assert_eq!(new_active, Some(tab3.id));
    }

    #[test]
    fn test_history_recording() {
        let (service, _storage) = create_test_history_service();

        let conn_id = Uuid::new_v4();
        let entry_id = service.record_query(
            conn_id,
            "Test Connection",
            "test_db",
            "SELECT 1",
            100,
            Some(1),
            None,
        ).unwrap();

        let entries = service.get_recent_queries(conn_id, 10).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, entry_id);
        assert_eq!(entries[0].sql, "SELECT 1");
    }

    #[test]
    fn test_history_search() {
        let (service, _storage) = create_test_history_service();

        let conn_id = Uuid::new_v4();
        service.record_query(conn_id, "Conn", "db", "SELECT * FROM users", 100, Some(10), None).unwrap();
        service.record_query(conn_id, "Conn", "db", "SELECT * FROM orders", 100, Some(5), None).unwrap();
        service.record_query(conn_id, "Conn", "db", "INSERT INTO logs", 50, Some(1), None).unwrap();

        let params = HistorySearchParams {
            search_text: Some("SELECT".to_string()),
            limit: 50,
            ..Default::default()
        };

        let result = service.search_history(params).unwrap();
        assert_eq!(result.entries.len(), 2);
    }

    #[test]
    fn test_favorite_toggle() {
        let (service, _storage) = create_test_history_service();

        let conn_id = Uuid::new_v4();
        let entry_id = service.record_query(
            conn_id, "Conn", "db", "SELECT 1", 100, Some(1), None
        ).unwrap();

        // Initially not favorited
        let entries = service.get_recent_queries(conn_id, 10).unwrap();
        assert!(!entries[0].favorited);

        // Toggle favorite
        let is_fav = service.toggle_favorite(entry_id).unwrap();
        assert!(is_fav);

        // Verify
        let entries = service.get_recent_queries(conn_id, 10).unwrap();
        assert!(entries[0].favorited);
    }
}
```

## Dependencies

- Feature 05: Local Storage (SQLite for tab/history persistence)
- Feature 11: Query Execution (history recording trigger)
- Feature 12: SQL Editor (buffer integration for tab content)
