# Feature 18: Inline Data Editing

## Overview

Inline editing allows users to modify table data directly in the results grid without writing SQL. Changes are tracked, previewed, and committed in a single transaction. This feature is only available for single-table SELECT queries with a primary key. Built entirely in Rust using GPUI for native performance.

## Goals

- Enable cell editing with double-click or keyboard
- Track all changes (inserts, updates, deletes)
- Show visual indicators for modified cells
- Preview generated SQL before committing
- Execute changes in a transaction with rollback on error
- Support NULL value handling
- Validate against column constraints

## Dependencies

- Feature 14: Results Grid (display layer)
- Feature 17: Table Data Viewer (integration point)
- Feature 11: Query Execution (data modification)
- Feature 10: Schema Introspection (primary key detection)

## Technical Specification

### 18.1 Edit Mode Models

```rust
// src/models/edit_mode.rs

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::models::query::Value;
use crate::models::schema::Column;

/// Type of change made to a row
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    Insert,
    Update,
    Delete,
}

/// State of a cell in edit mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Unchanged,
    Modified,
    New,
    Deleted,
}

/// A change to a single cell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellChange {
    pub column_name: String,
    pub original_value: Value,
    pub new_value: Value,
}

/// A change to a single row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowChange {
    pub change_type: ChangeType,
    pub row_index: i64,  // Negative for new rows
    pub original_row: Option<Vec<Value>>,
    pub cell_changes: HashMap<String, CellChange>,
}

impl RowChange {
    pub fn new_update(row_index: i64) -> Self {
        Self {
            change_type: ChangeType::Update,
            row_index,
            original_row: None,
            cell_changes: HashMap::new(),
        }
    }

    pub fn new_insert(row_index: i64, column_count: usize) -> Self {
        Self {
            change_type: ChangeType::Insert,
            row_index,
            original_row: None,
            cell_changes: HashMap::new(),
        }
    }

    pub fn new_delete(row_index: i64, original_row: Vec<Value>) -> Self {
        Self {
            change_type: ChangeType::Delete,
            row_index,
            original_row: Some(original_row),
            cell_changes: HashMap::new(),
        }
    }

    pub fn add_cell_change(&mut self, column: String, original: Value, new_value: Value) {
        self.cell_changes.insert(column.clone(), CellChange {
            column_name: column,
            original_value: original,
            new_value,
        });
    }

    pub fn remove_cell_change(&mut self, column: &str) {
        self.cell_changes.remove(column);
    }

    pub fn is_empty(&self) -> bool {
        self.cell_changes.is_empty() && self.change_type != ChangeType::Delete
    }
}

/// Information about a table that can be edited
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditableTableInfo {
    pub connection_id: Uuid,
    pub schema: String,
    pub table: String,
    pub primary_key_columns: Vec<String>,
    pub columns: Vec<EditableColumnInfo>,
}

impl EditableTableInfo {
    pub fn is_editable(&self) -> bool {
        !self.primary_key_columns.is_empty()
    }

    pub fn get_column(&self, name: &str) -> Option<&EditableColumnInfo> {
        self.columns.iter().find(|c| c.name == name)
    }

    pub fn get_column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.name == name)
    }

    pub fn primary_key_indices(&self) -> Vec<usize> {
        self.primary_key_columns.iter()
            .filter_map(|pk| self.get_column_index(pk))
            .collect()
    }
}

/// Information about a column for editing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditableColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub has_default: bool,
    pub default_value: Option<String>,
    pub is_generated: bool,
    pub is_identity: bool,
    pub max_length: Option<i32>,
    pub numeric_precision: Option<i32>,
    pub numeric_scale: Option<i32>,
}

impl EditableColumnInfo {
    pub fn can_edit(&self) -> bool {
        !self.is_generated
    }

    pub fn can_insert(&self) -> bool {
        !self.is_generated || self.has_default || self.is_identity
    }

    /// Get the appropriate editor type for this column
    pub fn editor_type(&self) -> EditorType {
        match self.data_type.as_str() {
            "bool" => EditorType::Boolean,
            "int2" | "int4" | "int8" | "smallint" | "integer" | "bigint" => EditorType::Integer,
            "float4" | "float8" | "numeric" | "decimal" | "real" | "double precision" => EditorType::Decimal,
            "text" => EditorType::MultilineText,
            "json" | "jsonb" => EditorType::Json,
            "date" => EditorType::Date,
            "time" | "timetz" => EditorType::Time,
            "timestamp" | "timestamptz" => EditorType::DateTime,
            "uuid" => EditorType::Uuid,
            "bytea" => EditorType::Binary,
            _ if self.data_type.ends_with("[]") => EditorType::Array,
            _ => EditorType::Text,
        }
    }
}

/// Type of editor to use for a cell
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorType {
    Text,
    MultilineText,
    Integer,
    Decimal,
    Boolean,
    Date,
    Time,
    DateTime,
    Json,
    Uuid,
    Binary,
    Array,
    Enum(/* variants loaded dynamically */),
}

/// Result of committing edits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditCommitResult {
    pub success: bool,
    pub affected_rows: u64,
    pub error: Option<String>,
    pub failed_statement: Option<String>,
}

/// Validation error for a cell
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub row_index: i64,
    pub column_name: String,
    pub message: String,
}

/// Cell coordinates for tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellCoord {
    pub row: i64,
    pub column_index: usize,
}

impl CellCoord {
    pub fn new(row: i64, column_index: usize) -> Self {
        Self { row, column_index }
    }
}
```

### 18.2 Edit Mode State

```rust
// src/state/edit_mode.rs

use gpui::Global;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::runtime::Handle;
use uuid::Uuid;

use crate::models::edit_mode::{
    EditableTableInfo, EditableColumnInfo, RowChange, CellChange,
    ChangeType, CellState, CellCoord, EditCommitResult, ValidationError,
};
use crate::models::query::Value;
use crate::services::query::QueryService;
use crate::services::schema::SchemaService;

/// Global state for edit mode
pub struct EditModeState {
    query_service: Arc<QueryService>,
    schema_service: Arc<SchemaService>,
    runtime: Handle,

    // Current edit session
    enabled: RwLock<bool>,
    table_info: RwLock<Option<EditableTableInfo>>,

    // Change tracking
    row_changes: RwLock<HashMap<i64, RowChange>>,
    deleted_rows: RwLock<HashSet<i64>>,
    new_rows: RwLock<HashMap<i64, Vec<Value>>>,
    next_new_row_id: RwLock<i64>,

    // Current edit state
    editing_cell: RwLock<Option<CellCoord>>,
    validation_errors: RwLock<Vec<ValidationError>>,
}

impl Global for EditModeState {}

impl EditModeState {
    pub fn new(
        query_service: Arc<QueryService>,
        schema_service: Arc<SchemaService>,
        runtime: Handle,
    ) -> Self {
        Self {
            query_service,
            schema_service,
            runtime,
            enabled: RwLock::new(false),
            table_info: RwLock::new(None),
            row_changes: RwLock::new(HashMap::new()),
            deleted_rows: RwLock::new(HashSet::new()),
            new_rows: RwLock::new(HashMap::new()),
            next_new_row_id: RwLock::new(-1),
            editing_cell: RwLock::new(None),
            validation_errors: RwLock::new(Vec::new()),
        }
    }

    // ==================== State Queries ====================

    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    pub fn can_edit(&self) -> bool {
        self.is_enabled() &&
        self.table_info.read().as_ref().map(|t| t.is_editable()).unwrap_or(false)
    }

    pub fn table_info(&self) -> Option<EditableTableInfo> {
        self.table_info.read().clone()
    }

    pub fn has_changes(&self) -> bool {
        !self.row_changes.read().is_empty() ||
        !self.deleted_rows.read().is_empty() ||
        !self.new_rows.read().is_empty()
    }

    pub fn change_count(&self) -> usize {
        self.row_changes.read().len() +
        self.deleted_rows.read().len() +
        self.new_rows.read().len()
    }

    pub fn update_count(&self) -> usize {
        self.row_changes.read().values()
            .filter(|c| c.change_type == ChangeType::Update && !c.cell_changes.is_empty())
            .count()
    }

    pub fn insert_count(&self) -> usize {
        self.new_rows.read().len()
    }

    pub fn delete_count(&self) -> usize {
        self.deleted_rows.read().len()
    }

    pub fn editing_cell(&self) -> Option<CellCoord> {
        *self.editing_cell.read()
    }

    pub fn validation_errors(&self) -> Vec<ValidationError> {
        self.validation_errors.read().clone()
    }

    // ==================== Enable/Disable ====================

    /// Enable edit mode for a table
    pub fn enable(&self, connection_id: Uuid, schema: &str, table: &str) -> Result<bool, String> {
        // Get table info including primary key
        let info = self.runtime.block_on(async {
            self.get_editable_info(connection_id, schema, table).await
        })?;

        if !info.is_editable() {
            return Ok(false);
        }

        *self.table_info.write() = Some(info);
        *self.enabled.write() = true;
        self.clear_changes();

        Ok(true)
    }

    /// Disable edit mode
    pub fn disable(&self) {
        *self.enabled.write() = false;
        *self.table_info.write() = None;
        self.clear_changes();
    }

    /// Clear all pending changes
    pub fn clear_changes(&self) {
        self.row_changes.write().clear();
        self.deleted_rows.write().clear();
        self.new_rows.write().clear();
        *self.next_new_row_id.write() = -1;
        *self.editing_cell.write() = None;
        self.validation_errors.write().clear();
    }

    // ==================== Cell State ====================

    /// Get the state of a cell
    pub fn get_cell_state(&self, row_index: i64, column_name: &str) -> CellState {
        // Check for new row
        if row_index < 0 && self.new_rows.read().contains_key(&row_index) {
            return CellState::New;
        }

        // Check for deleted row
        if self.deleted_rows.read().contains(&row_index) {
            return CellState::Deleted;
        }

        // Check for modified cell
        if let Some(change) = self.row_changes.read().get(&row_index) {
            if change.cell_changes.contains_key(column_name) {
                return CellState::Modified;
            }
        }

        CellState::Unchanged
    }

    /// Get the current value of a cell (modified or original)
    pub fn get_cell_value(&self, row_index: i64, column_name: &str, original: &Value) -> Value {
        // Check new rows
        if row_index < 0 {
            if let Some(row) = self.new_rows.read().get(&row_index) {
                if let Some(info) = self.table_info.read().as_ref() {
                    if let Some(idx) = info.get_column_index(column_name) {
                        return row.get(idx).cloned().unwrap_or(Value::Null);
                    }
                }
            }
            return Value::Null;
        }

        // Check for modified value
        if let Some(change) = self.row_changes.read().get(&row_index) {
            if let Some(cell_change) = change.cell_changes.get(column_name) {
                return cell_change.new_value.clone();
            }
        }

        original.clone()
    }

    // ==================== Cell Editing ====================

    /// Start editing a cell
    pub fn start_editing(&self, row_index: i64, column_index: usize) -> bool {
        if !self.can_edit() {
            return false;
        }

        // Can't edit deleted rows
        if self.deleted_rows.read().contains(&row_index) {
            return false;
        }

        // Check if column is editable
        if let Some(info) = self.table_info.read().as_ref() {
            if let Some(col) = info.columns.get(column_index) {
                if !col.can_edit() {
                    return false;
                }
            }
        }

        *self.editing_cell.write() = Some(CellCoord::new(row_index, column_index));
        true
    }

    /// Stop editing current cell
    pub fn stop_editing(&self) {
        *self.editing_cell.write() = None;
    }

    /// Update a cell value
    pub fn update_cell(
        &self,
        row_index: i64,
        column_name: &str,
        original_value: &Value,
        new_value: Value,
    ) -> Result<(), String> {
        if !self.can_edit() {
            return Err("Edit mode not enabled".to_string());
        }

        // Validate the new value
        self.validate_cell_value(column_name, &new_value)?;

        // Handle new rows
        if row_index < 0 {
            let mut new_rows = self.new_rows.write();
            if let Some(row) = new_rows.get_mut(&row_index) {
                if let Some(info) = self.table_info.read().as_ref() {
                    if let Some(idx) = info.get_column_index(column_name) {
                        if idx < row.len() {
                            row[idx] = new_value;
                        }
                    }
                }
            }
            return Ok(());
        }

        // Can't edit deleted rows
        if self.deleted_rows.read().contains(&row_index) {
            return Err("Cannot edit deleted row".to_string());
        }

        // Track the change
        let mut changes = self.row_changes.write();
        let row_change = changes.entry(row_index)
            .or_insert_with(|| RowChange::new_update(row_index));

        // Check if reverting to original
        if values_equal(original_value, &new_value) {
            row_change.remove_cell_change(column_name);

            // Remove row change if no changes left
            if row_change.is_empty() {
                changes.remove(&row_index);
            }
        } else {
            row_change.add_cell_change(
                column_name.to_string(),
                original_value.clone(),
                new_value,
            );
        }

        Ok(())
    }

    /// Set a cell to NULL
    pub fn set_cell_null(&self, row_index: i64, column_name: &str, original_value: &Value) -> Result<(), String> {
        // Check if column is nullable
        if let Some(info) = self.table_info.read().as_ref() {
            if let Some(col) = info.get_column(column_name) {
                if !col.nullable {
                    return Err(format!("Column '{}' does not allow NULL values", column_name));
                }
            }
        }

        self.update_cell(row_index, column_name, original_value, Value::Null)
    }

    // ==================== Row Operations ====================

    /// Add a new row
    pub fn add_row(&self) -> Option<i64> {
        if !self.can_edit() {
            return None;
        }

        let info = self.table_info.read();
        let info = info.as_ref()?;

        // Generate new row ID
        let mut next_id = self.next_new_row_id.write();
        let row_id = *next_id;
        *next_id -= 1;

        // Create new row with default values
        let new_row: Vec<Value> = info.columns.iter().map(|col| {
            if col.has_default || col.is_generated || col.is_identity {
                Value::Default  // Use DEFAULT keyword
            } else {
                Value::Null
            }
        }).collect();

        self.new_rows.write().insert(row_id, new_row);

        Some(row_id)
    }

    /// Delete a row
    pub fn delete_row(&self, row_index: i64, original_row: Option<Vec<Value>>) {
        if !self.can_edit() {
            return;
        }

        if row_index < 0 {
            // Remove new row
            self.new_rows.write().remove(&row_index);
        } else {
            // Mark existing row for deletion
            self.deleted_rows.write().insert(row_index);

            // Store original row for SQL generation
            if let Some(row) = original_row {
                self.row_changes.write().insert(
                    row_index,
                    RowChange::new_delete(row_index, row),
                );
            }

            // Remove any pending updates (they're irrelevant now)
            if let Some(change) = self.row_changes.write().get_mut(&row_index) {
                if change.change_type == ChangeType::Update {
                    change.cell_changes.clear();
                }
            }
        }
    }

    /// Undelete a row
    pub fn undelete_row(&self, row_index: i64) {
        self.deleted_rows.write().remove(&row_index);

        // Convert back to update change if there were pending changes
        if let Some(change) = self.row_changes.write().get_mut(&row_index) {
            if change.change_type == ChangeType::Delete {
                change.change_type = ChangeType::Update;
            }
        }
    }

    /// Duplicate a row (creates a new row with same values)
    pub fn duplicate_row(&self, source_row: &[Value]) -> Option<i64> {
        let row_id = self.add_row()?;

        let mut new_rows = self.new_rows.write();
        if let Some(new_row) = new_rows.get_mut(&row_id) {
            if let Some(info) = self.table_info.read().as_ref() {
                for (i, col) in info.columns.iter().enumerate() {
                    // Skip primary key columns (they need unique values)
                    if info.primary_key_columns.contains(&col.name) {
                        continue;
                    }

                    // Skip generated columns
                    if col.is_generated {
                        continue;
                    }

                    // Copy value from source
                    if i < source_row.len() && i < new_row.len() {
                        new_row[i] = source_row[i].clone();
                    }
                }
            }
        }

        Some(row_id)
    }

    // ==================== Validation ====================

    fn validate_cell_value(&self, column_name: &str, value: &Value) -> Result<(), String> {
        let info = self.table_info.read();
        let info = match info.as_ref() {
            Some(i) => i,
            None => return Ok(()),
        };

        let col = match info.get_column(column_name) {
            Some(c) => c,
            None => return Ok(()),
        };

        // Check nullable
        if matches!(value, Value::Null) && !col.nullable {
            return Err(format!("Column '{}' cannot be NULL", column_name));
        }

        // Check max length for text types
        if let Some(max_len) = col.max_length {
            if let Value::String(s) = value {
                if s.len() > max_len as usize {
                    return Err(format!(
                        "Value exceeds maximum length of {} for column '{}'",
                        max_len, column_name
                    ));
                }
            }
        }

        // Type-specific validation
        match col.data_type.as_str() {
            "uuid" => {
                if let Value::String(s) = value {
                    if uuid::Uuid::parse_str(s).is_err() {
                        return Err("Invalid UUID format".to_string());
                    }
                }
            }
            "json" | "jsonb" => {
                if let Value::String(s) = value {
                    if serde_json::from_str::<serde_json::Value>(s).is_err() {
                        return Err("Invalid JSON format".to_string());
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Validate all changes before commit
    pub fn validate_all(&self, rows: &[Vec<Value>]) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        let info = match self.table_info.read().clone() {
            Some(i) => i,
            None => return errors,
        };

        // Validate new rows
        for (&row_id, new_row) in self.new_rows.read().iter() {
            for (i, col) in info.columns.iter().enumerate() {
                if i >= new_row.len() {
                    continue;
                }

                let value = &new_row[i];

                // Check NOT NULL without default
                if matches!(value, Value::Null | Value::Default) &&
                   !col.nullable && !col.has_default && !col.is_identity
                {
                    errors.push(ValidationError {
                        row_index: row_id,
                        column_name: col.name.clone(),
                        message: format!("Column '{}' requires a value", col.name),
                    });
                }
            }
        }

        // Validate updates
        for (&row_id, change) in self.row_changes.read().iter() {
            if change.change_type != ChangeType::Update {
                continue;
            }

            for (col_name, cell_change) in &change.cell_changes {
                if let Some(col) = info.get_column(col_name) {
                    if matches!(cell_change.new_value, Value::Null) && !col.nullable {
                        errors.push(ValidationError {
                            row_index: row_id,
                            column_name: col_name.clone(),
                            message: format!("Column '{}' cannot be NULL", col_name),
                        });
                    }
                }
            }
        }

        *self.validation_errors.write() = errors.clone();
        errors
    }

    // ==================== SQL Generation ====================

    /// Generate SQL statements for all changes
    pub fn generate_sql(&self, rows: &[Vec<Value>]) -> Vec<String> {
        let info = match self.table_info.read().clone() {
            Some(i) => i,
            None => return Vec::new(),
        };

        let mut statements = Vec::new();
        let table_ref = format!("\"{}\".\"{}\"", info.schema, info.table);

        // DELETE statements first
        for &row_idx in self.deleted_rows.read().iter() {
            if row_idx >= 0 && (row_idx as usize) < rows.len() {
                let row = &rows[row_idx as usize];
                let where_clause = build_pk_where_clause(&info, row);
                statements.push(format!("DELETE FROM {} WHERE {};", table_ref, where_clause));
            }
        }

        // UPDATE statements
        for (row_idx, change) in self.row_changes.read().iter() {
            if change.change_type != ChangeType::Update || change.cell_changes.is_empty() {
                continue;
            }

            if *row_idx >= 0 && (*row_idx as usize) < rows.len() {
                let row = &rows[*row_idx as usize];
                let set_clause: Vec<String> = change.cell_changes.iter()
                    .map(|(col, cell)| {
                        format!("\"{}\" = {}", col, format_value(&cell.new_value))
                    })
                    .collect();

                let where_clause = build_pk_where_clause(&info, row);
                statements.push(format!(
                    "UPDATE {} SET {} WHERE {};",
                    table_ref,
                    set_clause.join(", "),
                    where_clause
                ));
            }
        }

        // INSERT statements
        for (_, new_row) in self.new_rows.read().iter() {
            let non_generated_cols: Vec<_> = info.columns.iter()
                .enumerate()
                .filter(|(_, c)| !c.is_generated)
                .collect();

            let col_names: Vec<_> = non_generated_cols.iter()
                .map(|(_, c)| format!("\"{}\"", c.name))
                .collect();

            let values: Vec<_> = non_generated_cols.iter()
                .map(|(i, _)| {
                    new_row.get(*i)
                        .map(format_value)
                        .unwrap_or_else(|| "DEFAULT".to_string())
                })
                .collect();

            statements.push(format!(
                "INSERT INTO {} ({}) VALUES ({});",
                table_ref,
                col_names.join(", "),
                values.join(", ")
            ));
        }

        statements
    }

    /// Generate formatted SQL for preview
    pub fn generate_preview_sql(&self, rows: &[Vec<Value>]) -> String {
        let statements = self.generate_sql(rows);
        if statements.is_empty() {
            return "-- No changes to commit".to_string();
        }

        format!("BEGIN;\n\n{}\n\nCOMMIT;", statements.join("\n\n"))
    }

    // ==================== Commit ====================

    /// Commit all changes to the database
    pub fn commit(&self, rows: &[Vec<Value>]) -> EditCommitResult {
        if !self.has_changes() {
            return EditCommitResult {
                success: true,
                affected_rows: 0,
                error: None,
                failed_statement: None,
            };
        }

        // Validate first
        let errors = self.validate_all(rows);
        if !errors.is_empty() {
            return EditCommitResult {
                success: false,
                affected_rows: 0,
                error: Some(format!("{} validation error(s)", errors.len())),
                failed_statement: None,
            };
        }

        let statements = self.generate_sql(rows);
        if statements.is_empty() {
            return EditCommitResult {
                success: true,
                affected_rows: 0,
                error: None,
                failed_statement: None,
            };
        }

        // Execute in transaction
        let info = match self.table_info.read().clone() {
            Some(i) => i,
            None => return EditCommitResult {
                success: false,
                affected_rows: 0,
                error: Some("No table info".to_string()),
                failed_statement: None,
            },
        };

        let sql = format!("BEGIN;\n{}\nCOMMIT;", statements.join("\n"));

        let result = self.runtime.block_on(async {
            self.query_service.execute_query(
                info.connection_id,
                &sql,
                Some(60000),  // 60 second timeout for edits
            ).await
        });

        match result {
            Ok(_) => {
                self.clear_changes();
                EditCommitResult {
                    success: true,
                    affected_rows: statements.len() as u64,
                    error: None,
                    failed_statement: None,
                }
            }
            Err(e) => EditCommitResult {
                success: false,
                affected_rows: 0,
                error: Some(e.to_string()),
                failed_statement: None,
            }
        }
    }

    // ==================== Helper ====================

    async fn get_editable_info(
        &self,
        connection_id: Uuid,
        schema: &str,
        table: &str,
    ) -> Result<EditableTableInfo, String> {
        let table_info = self.schema_service
            .get_table(connection_id, schema, table)
            .await
            .map_err(|e| e.to_string())?;

        let primary_key_columns = table_info.primary_key
            .map(|pk| pk.columns)
            .unwrap_or_default();

        let columns = table_info.columns.iter()
            .map(|c| EditableColumnInfo {
                name: c.name.clone(),
                data_type: c.data_type.clone(),
                nullable: c.nullable,
                has_default: c.default.is_some(),
                default_value: c.default.clone(),
                is_generated: c.is_generated,
                is_identity: c.is_identity,
                max_length: c.max_length,
                numeric_precision: c.numeric_precision,
                numeric_scale: c.numeric_scale,
            })
            .collect();

        Ok(EditableTableInfo {
            connection_id,
            schema: schema.to_string(),
            table: table.to_string(),
            primary_key_columns,
            columns,
        })
    }
}

// ==================== Helper Functions ====================

fn build_pk_where_clause(info: &EditableTableInfo, row: &[Value]) -> String {
    info.primary_key_columns.iter()
        .filter_map(|pk_col| {
            info.get_column_index(pk_col).map(|idx| {
                let value = row.get(idx).unwrap_or(&Value::Null);
                format!("\"{}\" = {}", pk_col, format_value(value))
            })
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_string(),
        Value::Default => "DEFAULT".to_string(),
        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => {
            if f.is_nan() {
                "'NaN'::float".to_string()
            } else if f.is_infinite() {
                if *f > 0.0 { "'Infinity'::float" } else { "'-Infinity'::float" }.to_string()
            } else {
                f.to_string()
            }
        }
        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        Value::Bytes(b) => format!("'\\x{}'", hex::encode(b)),
        Value::Json(j) => format!("'{}'::jsonb", j.to_string().replace('\'', "''")),
        Value::Array(arr) => {
            let elements: Vec<_> = arr.iter().map(format_value).collect();
            format!("ARRAY[{}]", elements.join(", "))
        }
        Value::Uuid(u) => format!("'{}'::uuid", u),
        Value::Date(d) => format!("'{}'::date", d),
        Value::Time(t) => format!("'{}'::time", t),
        Value::Timestamp(ts) => format!("'{}'::timestamp", ts),
        Value::Interval(i) => format!("'{}'::interval", i),
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => {
            if a.is_nan() && b.is_nan() { true }
            else { (a - b).abs() < f64::EPSILON }
        }
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Json(a), Value::Json(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| values_equal(x, y))
        }
        _ => false,
    }
}
```

### 18.3 Editable Cell Component

```rust
// src/ui/components/editable_cell.rs

use gpui::*;

use crate::models::edit_mode::{CellState, EditorType};
use crate::models::query::Value;
use crate::state::edit_mode::EditModeState;
use crate::theme::Theme;

/// Events emitted by EditableCell
pub enum EditableCellEvent {
    ValueChanged { row: i64, column: String, value: Value },
    EditStarted { row: i64, column: usize },
    EditEnded,
    NavigateNext,
    NavigatePrevious,
}

impl EventEmitter<EditableCellEvent> for EditableCell {}

/// Editable cell component
pub struct EditableCell {
    row_index: i64,
    column_index: usize,
    column_name: String,
    column_type: String,
    original_value: Value,
    is_editing: bool,
    edit_buffer: String,
    editor_type: EditorType,
}

impl EditableCell {
    pub fn new(
        row_index: i64,
        column_index: usize,
        column_name: String,
        column_type: String,
        value: Value,
    ) -> Self {
        let editor_type = Self::get_editor_type(&column_type);

        Self {
            row_index,
            column_index,
            column_name,
            column_type,
            original_value: value,
            is_editing: false,
            edit_buffer: String::new(),
            editor_type,
        }
    }

    fn get_editor_type(column_type: &str) -> EditorType {
        match column_type {
            "bool" => EditorType::Boolean,
            "int2" | "int4" | "int8" | "smallint" | "integer" | "bigint" => EditorType::Integer,
            "float4" | "float8" | "numeric" | "decimal" => EditorType::Decimal,
            "text" => EditorType::MultilineText,
            "json" | "jsonb" => EditorType::Json,
            "date" => EditorType::Date,
            "time" | "timetz" => EditorType::Time,
            "timestamp" | "timestamptz" => EditorType::DateTime,
            "uuid" => EditorType::Uuid,
            "bytea" => EditorType::Binary,
            _ if column_type.ends_with("[]") => EditorType::Array,
            _ => EditorType::Text,
        }
    }

    pub fn start_editing(&mut self, cx: &mut Context<Self>) {
        let edit_state = cx.global::<EditModeState>();
        if !edit_state.can_edit() {
            return;
        }

        let cell_state = edit_state.get_cell_state(self.row_index, &self.column_name);
        if cell_state == CellState::Deleted {
            return;
        }

        self.is_editing = true;
        self.edit_buffer = self.value_to_edit_string();

        cx.emit(EditableCellEvent::EditStarted {
            row: self.row_index,
            column: self.column_index,
        });
        cx.notify();
    }

    pub fn stop_editing(&mut self, commit: bool, cx: &mut Context<Self>) {
        if !self.is_editing {
            return;
        }

        if commit {
            self.commit_edit(cx);
        }

        self.is_editing = false;
        self.edit_buffer.clear();
        cx.emit(EditableCellEvent::EditEnded);
        cx.notify();
    }

    fn commit_edit(&mut self, cx: &mut Context<Self>) {
        let new_value = self.parse_edit_value();

        let edit_state = cx.global::<EditModeState>();
        if let Err(e) = edit_state.update_cell(
            self.row_index,
            &self.column_name,
            &self.original_value,
            new_value.clone(),
        ) {
            // Show error - could emit an event here
            eprintln!("Edit error: {}", e);
            return;
        }

        cx.emit(EditableCellEvent::ValueChanged {
            row: self.row_index,
            column: self.column_name.clone(),
            value: new_value,
        });
    }

    fn set_null(&mut self, cx: &mut Context<Self>) {
        let edit_state = cx.global::<EditModeState>();
        if let Err(e) = edit_state.set_cell_null(
            self.row_index,
            &self.column_name,
            &self.original_value,
        ) {
            eprintln!("Error setting NULL: {}", e);
            return;
        }

        self.is_editing = false;
        cx.emit(EditableCellEvent::ValueChanged {
            row: self.row_index,
            column: self.column_name.clone(),
            value: Value::Null,
        });
        cx.notify();
    }

    fn value_to_edit_string(&self) -> String {
        let edit_state_guard = unsafe {
            // This is safe because we're in a single-threaded context here
            &*std::ptr::null::<EditModeState>()
        };
        // Actually get from context in real impl

        match &self.original_value {
            Value::Null => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::Json(j) => serde_json::to_string_pretty(j).unwrap_or_default(),
            Value::Array(arr) => {
                // Format as comma-separated for editing
                arr.iter()
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        _ => format!("{:?}", v),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            }
            Value::Uuid(u) => u.to_string(),
            Value::Date(d) => d.clone(),
            Value::Time(t) => t.clone(),
            Value::Timestamp(ts) => ts.clone(),
            Value::Bytes(b) => hex::encode(b),
            _ => format!("{:?}", self.original_value),
        }
    }

    fn parse_edit_value(&self) -> Value {
        let trimmed = self.edit_buffer.trim();

        if trimmed.is_empty() {
            return Value::Null;
        }

        match self.editor_type {
            EditorType::Boolean => {
                let lower = trimmed.to_lowercase();
                if ["true", "t", "yes", "y", "1"].contains(&lower.as_str()) {
                    Value::Bool(true)
                } else if ["false", "f", "no", "n", "0"].contains(&lower.as_str()) {
                    Value::Bool(false)
                } else {
                    Value::Null
                }
            }
            EditorType::Integer => {
                trimmed.parse::<i64>()
                    .map(Value::Int)
                    .unwrap_or(Value::Null)
            }
            EditorType::Decimal => {
                trimmed.parse::<f64>()
                    .map(Value::Float)
                    .unwrap_or(Value::Null)
            }
            EditorType::Json => {
                serde_json::from_str(trimmed)
                    .map(Value::Json)
                    .unwrap_or_else(|_| Value::String(trimmed.to_string()))
            }
            EditorType::Uuid => {
                if uuid::Uuid::parse_str(trimmed).is_ok() {
                    Value::Uuid(trimmed.to_string())
                } else {
                    Value::String(trimmed.to_string())
                }
            }
            EditorType::Array => {
                // Parse comma-separated values
                let elements: Vec<Value> = trimmed
                    .split(',')
                    .map(|s| Value::String(s.trim().to_string()))
                    .collect();
                Value::Array(elements)
            }
            _ => Value::String(trimmed.to_string()),
        }
    }

    fn handle_keydown(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "enter" => {
                if !event.keystroke.modifiers.shift {
                    self.stop_editing(true, cx);
                    cx.emit(EditableCellEvent::NavigateNext);
                }
            }
            "escape" => {
                self.stop_editing(false, cx);
            }
            "tab" => {
                self.stop_editing(true, cx);
                if event.keystroke.modifiers.shift {
                    cx.emit(EditableCellEvent::NavigatePrevious);
                } else {
                    cx.emit(EditableCellEvent::NavigateNext);
                }
            }
            _ => {}
        }
    }

    fn get_display_value(&self, cx: &Context<Self>) -> String {
        let edit_state = cx.global::<EditModeState>();
        let value = edit_state.get_cell_value(
            self.row_index,
            &self.column_name,
            &self.original_value,
        );

        match value {
            Value::Null => "NULL".to_string(),
            Value::Bool(b) => if b { "✓" } else { "✗" }.to_string(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => format!("{:.6}", f).trim_end_matches('0').trim_end_matches('.').to_string(),
            Value::String(s) => s,
            Value::Json(j) => j.to_string(),
            Value::Uuid(u) => u,
            Value::Date(d) => d,
            Value::Time(t) => t,
            Value::Timestamp(ts) => ts,
            Value::Bytes(b) => format!("[{} bytes]", b.len()),
            Value::Array(arr) => format!("[{} items]", arr.len()),
            _ => "...".to_string(),
        }
    }
}

impl Render for EditableCell {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let edit_state = cx.global::<EditModeState>();

        let cell_state = edit_state.get_cell_state(self.row_index, &self.column_name);
        let value = edit_state.get_cell_value(
            self.row_index,
            &self.column_name,
            &self.original_value,
        );
        let is_null = matches!(value, Value::Null);

        // Background color based on state
        let bg_color = match cell_state {
            CellState::Modified => theme.warning_bg,
            CellState::New => theme.success_bg,
            CellState::Deleted => theme.error_bg,
            CellState::Unchanged => theme.transparent,
        };

        let content = if self.is_editing {
            self.render_editor(cx).into_any_element()
        } else {
            self.render_display(cell_state, is_null, cx).into_any_element()
        };

        div()
            .id(SharedString::from(format!("cell-{}-{}", self.row_index, self.column_index)))
            .relative()
            .size_full()
            .flex()
            .items_center()
            .px_2()
            .bg(bg_color)
            .when(cell_state == CellState::Deleted, |el| {
                el.child(
                    div()
                        .absolute()
                        .top_1_2()
                        .left_0()
                        .right_0()
                        .h_px()
                        .bg(theme.text_muted)
                )
            })
            .on_double_click(cx.listener(|this, _, cx| {
                this.start_editing(cx);
            }))
            .child(content)
    }
}

impl EditableCell {
    fn render_display(&self, cell_state: CellState, is_null: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let display_value = self.get_display_value(cx);

        let text_color = match cell_state {
            CellState::Deleted => theme.text_muted,
            _ if is_null => theme.text_muted,
            _ => theme.text,
        };

        span()
            .text_sm()
            .text_color(text_color)
            .when(is_null, |el| el.italic())
            .overflow_hidden()
            .text_ellipsis()
            .whitespace_nowrap()
            .child(display_value)
    }

    fn render_editor(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .bg(theme.surface)
            .child(
                match self.editor_type {
                    EditorType::Boolean => self.render_boolean_editor(cx).into_any_element(),
                    EditorType::MultilineText | EditorType::Json => {
                        self.render_textarea_editor(cx).into_any_element()
                    }
                    _ => self.render_text_editor(cx).into_any_element(),
                }
            )
            .child(
                // NULL button
                div()
                    .id("set-null-btn")
                    .absolute()
                    .right_1()
                    .top_1_2()
                    .neg_translate_y_1_2()
                    .w_5()
                    .h_5()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .bg(theme.surface_secondary)
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.hover))
                    .on_click(cx.listener(|this, _, cx| this.set_null(cx)))
                    .child(
                        span()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child("∅")
                    )
            )
    }

    fn render_text_editor(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        input()
            .w_full()
            .h_full()
            .px_2()
            .border_2()
            .border_color(theme.primary)
            .bg(theme.surface)
            .text_sm()
            .text_color(theme.text)
            .font_family("monospace")
            .value(self.edit_buffer.clone())
            .autofocus()
            .on_input(cx.listener(|this, event: &InputEvent, cx| {
                this.edit_buffer = event.value.clone();
                cx.notify();
            }))
            .on_key_down(cx.listener(Self::handle_keydown))
            .on_blur(cx.listener(|this, _, cx| {
                this.stop_editing(true, cx);
            }))
    }

    fn render_textarea_editor(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        // For multiline, render a popup editor
        div()
            .absolute()
            .top_0()
            .left_0()
            .w_64()
            .min_h_24()
            .p_2()
            .border_2()
            .border_color(theme.primary)
            .rounded_md()
            .bg(theme.surface)
            .shadow_lg()
            .z_index(100)
            .child(
                textarea()
                    .w_full()
                    .min_h_20()
                    .p_2()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_sm()
                    .bg(theme.background)
                    .text_sm()
                    .font_family("monospace")
                    .text_color(theme.text)
                    .value(self.edit_buffer.clone())
                    .autofocus()
                    .on_input(cx.listener(|this, event: &InputEvent, cx| {
                        this.edit_buffer = event.value.clone();
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, cx| {
                        // Only handle Escape for textarea, Enter is newline
                        if event.keystroke.key.as_str() == "escape" {
                            this.stop_editing(false, cx);
                        }
                    }))
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .mt_2()
                    .child(
                        button()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(theme.primary)
                            .text_sm()
                            .text_color(theme.on_primary)
                            .on_click(cx.listener(|this, _, cx| this.stop_editing(true, cx)))
                            .child("Save")
                    )
                    .child(
                        button()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .text_sm()
                            .text_color(theme.text)
                            .on_click(cx.listener(|this, _, cx| this.stop_editing(false, cx)))
                            .child("Cancel")
                    )
            )
    }

    fn render_boolean_editor(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        select()
            .w_full()
            .h_full()
            .px_2()
            .border_2()
            .border_color(theme.primary)
            .bg(theme.surface)
            .text_sm()
            .text_color(theme.text)
            .autofocus()
            .on_change(cx.listener(|this, event: &ChangeEvent, cx| {
                this.edit_buffer = event.value.clone();
                this.stop_editing(true, cx);
            }))
            .on_blur(cx.listener(|this, _, cx| {
                this.stop_editing(true, cx);
            }))
            .child(option().value("").child("NULL"))
            .child(option().value("true").selected(self.edit_buffer == "true").child("true"))
            .child(option().value("false").selected(self.edit_buffer == "false").child("false"))
    }
}
```

### 18.4 Edit Toolbar Component

```rust
// src/ui/components/edit_toolbar.rs

use gpui::*;

use crate::models::query::Value;
use crate::state::edit_mode::EditModeState;
use crate::theme::Theme;

/// Events emitted by EditToolbar
pub enum EditToolbarEvent {
    Commit,
    Discard,
    AddRow,
    Refresh,
}

impl EventEmitter<EditToolbarEvent> for EditToolbar {}

/// Edit mode toolbar component
pub struct EditToolbar {
    show_preview: bool,
    is_committing: bool,
    rows: Vec<Vec<Value>>,
}

impl EditToolbar {
    pub fn new(rows: Vec<Vec<Value>>) -> Self {
        Self {
            show_preview: false,
            is_committing: false,
            rows,
        }
    }

    pub fn set_rows(&mut self, rows: Vec<Vec<Value>>) {
        self.rows = rows;
    }

    fn toggle_preview(&mut self, cx: &mut Context<Self>) {
        self.show_preview = !self.show_preview;
        cx.notify();
    }

    fn add_row(&mut self, cx: &mut Context<Self>) {
        let edit_state = cx.global::<EditModeState>();
        edit_state.add_row();
        cx.emit(EditToolbarEvent::AddRow);
        cx.notify();
    }

    fn discard(&mut self, cx: &mut Context<Self>) {
        let edit_state = cx.global::<EditModeState>();
        if edit_state.has_changes() {
            // Show confirmation dialog
            // For now, just discard
            edit_state.clear_changes();
        }
        cx.emit(EditToolbarEvent::Discard);
        cx.notify();
    }

    fn commit(&mut self, cx: &mut Context<Self>) {
        self.is_committing = true;
        cx.notify();

        let edit_state = cx.global::<EditModeState>();
        let result = edit_state.commit(&self.rows);

        self.is_committing = false;

        if result.success {
            cx.emit(EditToolbarEvent::Commit);
            cx.emit(EditToolbarEvent::Refresh);
        } else if let Some(error) = result.error {
            // Show error notification
            eprintln!("Commit failed: {}", error);
        }

        cx.notify();
    }
}

impl Render for EditToolbar {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let edit_state = cx.global::<EditModeState>();

        let change_count = edit_state.change_count();
        let has_changes = edit_state.has_changes();
        let update_count = edit_state.update_count();
        let insert_count = edit_state.insert_count();
        let delete_count = edit_state.delete_count();

        div()
            .w_full()
            .flex()
            .flex_col()
            .child(
                // Main toolbar
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .bg(theme.warning_bg)
                    .border_b_1()
                    .border_color(theme.warning)
                    .child(
                        // Info section
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(IconName::AlertTriangle)
                                    .size_4()
                                    .color(theme.warning)
                            )
                            .child(
                                span()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.warning_text)
                                    .child(format!("Edit Mode: {} changes pending", change_count))
                            )
                            .when(change_count > 0, |el| {
                                el.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .ml_4()
                                        .text_xs()
                                        .text_color(theme.warning_text)
                                        .when(update_count > 0, |el| {
                                            el.child(format!("{} updates", update_count))
                                        })
                                        .when(insert_count > 0, |el| {
                                            el.child(format!("{} inserts", insert_count))
                                        })
                                        .when(delete_count > 0, |el| {
                                            el.child(format!("{} deletes", delete_count))
                                        })
                                )
                            })
                    )
                    .child(
                        // Actions
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                self.render_button(
                                    "Add Row",
                                    Some(IconName::Plus),
                                    false,
                                    cx.listener(|this, _, cx| this.add_row(cx)),
                                    cx,
                                )
                            )
                            .child(
                                self.render_button(
                                    "Preview SQL",
                                    Some(IconName::Eye),
                                    self.show_preview,
                                    cx.listener(|this, _, cx| this.toggle_preview(cx)),
                                    cx,
                                )
                            )
                            .child(
                                div().w_px().h_5().bg(theme.warning)
                            )
                            .child(
                                self.render_primary_button(
                                    if self.is_committing {
                                        "Saving...".to_string()
                                    } else {
                                        format!("Save Changes ({})", change_count)
                                    },
                                    !has_changes || self.is_committing,
                                    cx.listener(|this, _, cx| this.commit(cx)),
                                    cx,
                                )
                            )
                            .child(
                                self.render_button(
                                    "Discard",
                                    Some(IconName::X),
                                    false,
                                    cx.listener(|this, _, cx| this.discard(cx)),
                                    cx,
                                )
                            )
                    )
            )
            .when(self.show_preview, |el| {
                el.child(self.render_preview(cx))
            })
    }
}

impl EditToolbar {
    fn render_button(
        &self,
        label: &str,
        icon: Option<IconName>,
        active: bool,
        on_click: impl Fn(&ClickEvent, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id(SharedString::from(format!("edit-btn-{}", label)))
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .when(active, |el| el.bg(theme.warning.opacity(0.2)))
            .hover(|s| s.bg(theme.warning.opacity(0.1)))
            .on_click(on_click)
            .when_some(icon, |el, icon| {
                el.child(Icon::new(icon).size_4().color(theme.warning_text))
            })
            .child(
                span()
                    .text_sm()
                    .text_color(theme.warning_text)
                    .child(label.to_string())
            )
    }

    fn render_primary_button(
        &self,
        label: String,
        disabled: bool,
        on_click: impl Fn(&ClickEvent, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id("save-changes-btn")
            .flex()
            .items_center()
            .gap_1()
            .px_3()
            .py_1()
            .rounded_md()
            .when(disabled, |el| el.opacity(0.5).cursor_default())
            .when(!disabled, |el| {
                el.cursor_pointer()
                    .bg(theme.success)
                    .hover(|s| s.bg(theme.success_hover))
                    .on_click(on_click)
            })
            .when(disabled, |el| el.bg(theme.success.opacity(0.5)))
            .child(
                Icon::new(IconName::Check)
                    .size_4()
                    .color(theme.on_success)
            )
            .child(
                span()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.on_success)
                    .child(label)
            )
    }

    fn render_preview(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let edit_state = cx.global::<EditModeState>();
        let preview_sql = edit_state.generate_preview_sql(&self.rows);

        div()
            .w_full()
            .bg(theme.surface)
            .border_b_1()
            .border_color(theme.border)
            .child(
                // Header
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        span()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child("SQL Preview")
                    )
                    .child(
                        div()
                            .id("close-preview")
                            .p_1()
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.hover))
                            .on_click(cx.listener(|this, _, cx| this.toggle_preview(cx)))
                            .child(Icon::new(IconName::X).size_4().color(theme.text_muted))
                    )
            )
            .child(
                // SQL content
                div()
                    .max_h_48()
                    .overflow_y_auto()
                    .p_3()
                    .bg(theme.surface_secondary)
                    .child(
                        pre()
                            .text_sm()
                            .font_family("monospace")
                            .text_color(theme.text)
                            .whitespace_pre_wrap()
                            .child(preview_sql)
                    )
            )
    }
}
```

### 18.5 Row Operations Context Menu

```rust
// src/ui/components/row_context_menu.rs

use gpui::*;

use crate::models::query::Value;
use crate::state::edit_mode::EditModeState;
use crate::theme::Theme;

/// Events from row context menu
pub enum RowContextMenuEvent {
    Delete(i64),
    Undelete(i64),
    Duplicate(i64),
    InsertAbove(i64),
    InsertBelow(i64),
}

impl EventEmitter<RowContextMenuEvent> for RowContextMenu {}

/// Row context menu for edit operations
pub struct RowContextMenu {
    row_index: i64,
    is_deleted: bool,
    is_new_row: bool,
    position: Point<Pixels>,
}

impl RowContextMenu {
    pub fn new(row_index: i64, position: Point<Pixels>, cx: &mut Context<Self>) -> Self {
        let edit_state = cx.global::<EditModeState>();
        let is_deleted = edit_state.get_cell_state(row_index, "") == crate::models::edit_mode::CellState::Deleted;
        let is_new_row = row_index < 0;

        Self {
            row_index,
            is_deleted,
            is_new_row,
            position,
        }
    }
}

impl Render for RowContextMenu {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .absolute()
            .left(self.position.x)
            .top(self.position.y)
            .min_w_40()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .shadow_lg()
            .z_index(1000)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .when(!self.is_deleted, |el| {
                        el.child(self.menu_item("Insert Row Above", IconName::ArrowUp,
                            cx.listener(|this, _, cx| {
                                cx.emit(RowContextMenuEvent::InsertAbove(this.row_index));
                            }), cx))
                        .child(self.menu_item("Insert Row Below", IconName::ArrowDown,
                            cx.listener(|this, _, cx| {
                                cx.emit(RowContextMenuEvent::InsertBelow(this.row_index));
                            }), cx))
                        .child(self.menu_divider(cx))
                        .when(!self.is_new_row, |el| {
                            el.child(self.menu_item("Duplicate Row", IconName::Copy,
                                cx.listener(|this, _, cx| {
                                    cx.emit(RowContextMenuEvent::Duplicate(this.row_index));
                                }), cx))
                        })
                        .child(self.menu_divider(cx))
                        .child(self.menu_item_danger("Delete Row", IconName::Trash,
                            cx.listener(|this, _, cx| {
                                cx.emit(RowContextMenuEvent::Delete(this.row_index));
                            }), cx))
                    })
                    .when(self.is_deleted, |el| {
                        el.child(self.menu_item("Restore Row", IconName::Undo,
                            cx.listener(|this, _, cx| {
                                cx.emit(RowContextMenuEvent::Undelete(this.row_index));
                            }), cx))
                    })
            )
    }
}

impl RowContextMenu {
    fn menu_item(
        &self,
        label: &str,
        icon: IconName,
        on_click: impl Fn(&ClickEvent, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id(SharedString::from(format!("menu-{}", label)))
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .cursor_pointer()
            .hover(|s| s.bg(theme.hover))
            .on_click(on_click)
            .child(Icon::new(icon).size_4().color(theme.text_muted))
            .child(
                span()
                    .text_sm()
                    .text_color(theme.text)
                    .child(label.to_string())
            )
    }

    fn menu_item_danger(
        &self,
        label: &str,
        icon: IconName,
        on_click: impl Fn(&ClickEvent, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id(SharedString::from(format!("menu-{}", label)))
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .cursor_pointer()
            .hover(|s| s.bg(theme.error_bg))
            .on_click(on_click)
            .child(Icon::new(icon).size_4().color(theme.error))
            .child(
                span()
                    .text_sm()
                    .text_color(theme.error)
                    .child(label.to_string())
            )
    }

    fn menu_divider(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .w_full()
            .h_px()
            .my_1()
            .bg(theme.border)
    }
}
```

### 18.6 Keyboard Shortcuts

```rust
// Keyboard shortcuts for edit mode

use gpui::*;

/// Register edit mode keyboard shortcuts
pub fn register_edit_mode_shortcuts(cx: &mut AppContext) {
    // Cell editing
    cx.bind_keys([
        // Start editing
        KeyBinding::new("enter", StartEditing, Some("EditableCell")),
        KeyBinding::new("f2", StartEditing, Some("EditableCell")),

        // Commit edit
        KeyBinding::new("enter", CommitEdit, Some("EditableCell[editing=true]")),
        KeyBinding::new("tab", CommitAndNext, Some("EditableCell[editing=true]")),
        KeyBinding::new("shift-tab", CommitAndPrevious, Some("EditableCell[editing=true]")),

        // Cancel edit
        KeyBinding::new("escape", CancelEdit, Some("EditableCell[editing=true]")),

        // Set NULL
        KeyBinding::new("cmd-0", SetNull, Some("EditableCell")),
        KeyBinding::new("ctrl-0", SetNull, Some("EditableCell")),
    ]);

    // Row operations
    cx.bind_keys([
        // Add row
        KeyBinding::new("cmd-shift-n", AddRow, Some("TableViewer[edit_mode=true]")),
        KeyBinding::new("ctrl-shift-n", AddRow, Some("TableViewer[edit_mode=true]")),

        // Delete row
        KeyBinding::new("cmd-backspace", DeleteRow, Some("TableViewer[edit_mode=true]")),
        KeyBinding::new("ctrl-backspace", DeleteRow, Some("TableViewer[edit_mode=true]")),

        // Duplicate row
        KeyBinding::new("cmd-d", DuplicateRow, Some("TableViewer[edit_mode=true]")),
        KeyBinding::new("ctrl-d", DuplicateRow, Some("TableViewer[edit_mode=true]")),

        // Undo last change (per-cell)
        KeyBinding::new("cmd-z", UndoChange, Some("TableViewer[edit_mode=true]")),
        KeyBinding::new("ctrl-z", UndoChange, Some("TableViewer[edit_mode=true]")),
    ]);

    // Commit/Discard
    cx.bind_keys([
        // Save all changes
        KeyBinding::new("cmd-s", CommitChanges, Some("TableViewer[edit_mode=true]")),
        KeyBinding::new("ctrl-s", CommitChanges, Some("TableViewer[edit_mode=true]")),

        // Discard all changes
        KeyBinding::new("cmd-shift-z", DiscardChanges, Some("TableViewer[edit_mode=true]")),
    ]);

    // Navigation in edit mode
    cx.bind_keys([
        KeyBinding::new("up", NavigateUp, Some("EditableCell")),
        KeyBinding::new("down", NavigateDown, Some("EditableCell")),
        KeyBinding::new("left", NavigateLeft, Some("EditableCell")),
        KeyBinding::new("right", NavigateRight, Some("EditableCell")),
    ]);
}

// Action definitions
actions!(
    edit_mode,
    [
        StartEditing,
        CommitEdit,
        CommitAndNext,
        CommitAndPrevious,
        CancelEdit,
        SetNull,
        AddRow,
        DeleteRow,
        DuplicateRow,
        UndoChange,
        CommitChanges,
        DiscardChanges,
        NavigateUp,
        NavigateDown,
        NavigateLeft,
        NavigateRight,
    ]
);
```

## Acceptance Criteria

1. **Edit Mode Activation**
   - Toggle edit mode button in table viewer
   - Only enable for tables with primary key
   - Show warning banner when active
   - Cannot edit views or queries without identifiable primary key

2. **Cell Editing**
   - Double-click or Enter/F2 to edit cell
   - Tab/Enter to commit, Escape to cancel
   - Type-appropriate input controls (text, number, boolean dropdown, etc.)
   - NULL button (∅) for setting null values
   - Popup editor for multiline text and JSON
   - Validation on commit with error messages

3. **Change Tracking**
   - Yellow/amber highlight for modified cells
   - Green highlight for new rows
   - Red highlight with strikethrough for deleted rows
   - Change count in toolbar (updates, inserts, deletes)
   - Per-cell change tracking (can revert individual cells)

4. **Row Operations**
   - Add new row button and keyboard shortcut
   - Delete row (context menu or Cmd/Ctrl+Backspace)
   - Undelete support for accidentally deleted rows
   - Duplicate row to create copy
   - Insert row above/below

5. **SQL Preview**
   - Preview generated SQL statements
   - Shows DELETE, UPDATE, INSERT in execution order
   - Formatted with BEGIN/COMMIT wrapper
   - Toggle visibility

6. **Commit/Discard**
   - Save Changes button with count
   - Execute all statements in a single transaction
   - Automatic rollback on any error
   - Discard all changes option with confirmation
   - Refresh grid after successful commit

7. **Validation**
   - Respect NOT NULL constraints
   - Type validation on edit
   - Max length validation for varchar
   - UUID format validation
   - JSON syntax validation
   - Show errors inline and in validation summary

8. **Keyboard Navigation**
   - Tab/Shift+Tab to move between cells
   - Arrow keys to navigate
   - Cmd/Ctrl+S to save all changes
   - Cmd/Ctrl+Z to undo last change

## Testing Instructions

### Unit Tests

```rust
#[test]
fn test_cell_change_tracking() {
    let mut change = RowChange::new_update(0);
    change.add_cell_change("name".to_string(), Value::String("old".to_string()), Value::String("new".to_string()));

    assert!(!change.is_empty());
    assert!(change.cell_changes.contains_key("name"));

    // Revert to original
    change.add_cell_change("name".to_string(), Value::String("old".to_string()), Value::String("old".to_string()));
    // Should still contain the change but with same value
}

#[test]
fn test_sql_generation() {
    let mut edit_state = create_test_edit_state();
    edit_state.update_cell(0, "status", &Value::String("active".to_string()), Value::String("inactive".to_string())).unwrap();

    let rows = vec![vec![Value::Int(1), Value::String("active".to_string())]];
    let sql = edit_state.generate_sql(&rows);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("UPDATE"));
    assert!(sql[0].contains("status"));
    assert!(sql[0].contains("inactive"));
}

#[test]
fn test_validation() {
    let edit_state = create_test_edit_state();

    // Should fail for non-nullable column
    let result = edit_state.update_cell(0, "id", &Value::Int(1), Value::Null);
    assert!(result.is_err());
}

#[test]
fn test_delete_and_undelete() {
    let edit_state = create_test_edit_state();
    let original_row = vec![Value::Int(1), Value::String("test".to_string())];

    edit_state.delete_row(0, Some(original_row));
    assert!(edit_state.delete_count() == 1);

    edit_state.undelete_row(0);
    assert!(edit_state.delete_count() == 0);
}
```

### Integration Tests

```rust
#[test]
fn test_edit_mode_flow() {
    // Enable edit mode
    let result = edit_state.enable(conn_id, "public", "users");
    assert!(result.is_ok());
    assert!(edit_state.can_edit());

    // Make some changes
    edit_state.update_cell(0, "email", &old_email, new_email.clone()).unwrap();
    edit_state.add_row();
    edit_state.delete_row(2, Some(row_2.clone()));

    assert_eq!(edit_state.change_count(), 3);
    assert_eq!(edit_state.update_count(), 1);
    assert_eq!(edit_state.insert_count(), 1);
    assert_eq!(edit_state.delete_count(), 1);

    // Generate and verify SQL
    let sql = edit_state.generate_sql(&rows);
    assert_eq!(sql.len(), 3);
    assert!(sql.iter().any(|s| s.starts_with("DELETE")));
    assert!(sql.iter().any(|s| s.starts_with("UPDATE")));
    assert!(sql.iter().any(|s| s.starts_with("INSERT")));
}
```

## Performance Considerations

1. **Change Tracking Efficiency**
   - Use HashMap for O(1) change lookup
   - Only track actual changes (not all cells)
   - Remove changes when reverted to original

2. **Validation Caching**
   - Cache column metadata during edit session
   - Don't re-fetch schema for each validation

3. **SQL Generation**
   - Generate SQL lazily (only for preview/commit)
   - Use parameterized queries where possible for commit

4. **UI Updates**
   - Only re-render changed cells
   - Batch change notifications

## Security Considerations

1. **SQL Injection Prevention**
   - All values properly escaped in generated SQL
   - Use parameterized queries for actual execution
   - Validate input types match column types

2. **Transaction Safety**
   - All changes wrapped in BEGIN/COMMIT
   - Automatic ROLLBACK on any error
   - No partial commits

3. **Audit Trail**
   - Log all edit operations
   - Include connection ID and timestamp

## Dependencies

- Feature 14: Results Grid (display layer)
- Feature 17: Table Data Viewer (integration point)
- Feature 11: Query Execution (data modification)
- Feature 10: Schema Introspection (primary key detection)
