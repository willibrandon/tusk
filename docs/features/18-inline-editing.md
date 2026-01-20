# Feature 18: Inline Data Editing

## Overview

Inline editing allows users to modify table data directly in the results grid without writing SQL. Changes are tracked, previewed, and committed in a single transaction. This feature is only available for single-table SELECT queries with a primary key.

## Goals

- Enable cell editing with double-click
- Track all changes (inserts, updates, deletes)
- Show visual indicators for modified cells
- Preview generated SQL before committing
- Execute changes in a transaction with rollback on error
- Support NULL value handling

## Dependencies

- Feature 14: Results Grid (display layer)
- Feature 17: Table Data Viewer (integration point)
- Feature 11: Query Execution (data modification)
- Feature 10: Schema Introspection (primary key detection)

## Technical Specification

### 18.1 Edit Mode Store

```typescript
// src/lib/stores/editMode.svelte.ts

import { invoke } from '@tauri-apps/api/core';
import type { ColumnMeta, Value } from '$lib/services/query';

export type ChangeType = 'insert' | 'update' | 'delete';

export interface CellChange {
  rowIndex: number;
  columnName: string;
  originalValue: Value;
  newValue: Value;
}

export interface RowChange {
  type: ChangeType;
  rowIndex: number;
  originalRow?: Value[];
  newRow?: Value[];
  changes?: Map<string, { original: Value; new: Value }>;
}

export interface EditableTableInfo {
  schema: string;
  table: string;
  primaryKeyColumns: string[];
  columns: ColumnMeta[];
}

class EditModeStore {
  isEnabled = $state(false);
  tableInfo = $state<EditableTableInfo | null>(null);
  changes = $state<Map<number, RowChange>>(new Map());
  newRows = $state<Map<number, Value[]>>(new Map());
  deletedRows = $state<Set<number>>(new Set());
  nextNewRowIndex = $state(-1); // Negative indices for new rows

  canEdit = $derived(
    this.isEnabled &&
    this.tableInfo !== null &&
    this.tableInfo.primaryKeyColumns.length > 0
  );

  hasChanges = $derived(
    this.changes.size > 0 ||
    this.newRows.size > 0 ||
    this.deletedRows.size > 0
  );

  changeCount = $derived(
    this.changes.size + this.newRows.size + this.deletedRows.size
  );

  enable(tableInfo: EditableTableInfo) {
    this.tableInfo = tableInfo;
    this.isEnabled = true;
    this.clearChanges();
  }

  disable() {
    this.isEnabled = false;
    this.clearChanges();
  }

  clearChanges() {
    this.changes.clear();
    this.newRows.clear();
    this.deletedRows.clear();
    this.nextNewRowIndex = -1;
  }

  updateCell(
    rowIndex: number,
    columnName: string,
    originalValue: Value,
    newValue: Value
  ) {
    if (!this.canEdit) return;

    // Check if this is a new row
    if (rowIndex < 0) {
      const row = this.newRows.get(rowIndex);
      if (row) {
        const colIndex = this.tableInfo!.columns.findIndex(c => c.name === columnName);
        if (colIndex >= 0) {
          row[colIndex] = newValue;
          this.newRows.set(rowIndex, [...row]); // Trigger reactivity
        }
      }
      return;
    }

    // Check if row is deleted
    if (this.deletedRows.has(rowIndex)) return;

    // Track change
    let rowChange = this.changes.get(rowIndex);

    if (!rowChange) {
      rowChange = {
        type: 'update',
        rowIndex,
        changes: new Map(),
      };
      this.changes.set(rowIndex, rowChange);
    }

    const existingChange = rowChange.changes!.get(columnName);

    if (existingChange) {
      if (this.valuesEqual(existingChange.original, newValue)) {
        // Reverted to original - remove change
        rowChange.changes!.delete(columnName);

        // If no changes left, remove row change
        if (rowChange.changes!.size === 0) {
          this.changes.delete(rowIndex);
        }
      } else {
        existingChange.new = newValue;
      }
    } else {
      rowChange.changes!.set(columnName, {
        original: originalValue,
        new: newValue,
      });
    }

    // Trigger reactivity
    this.changes = new Map(this.changes);
  }

  addRow(): number {
    if (!this.canEdit || !this.tableInfo) return -1;

    const newRowIndex = this.nextNewRowIndex;
    this.nextNewRowIndex--;

    // Create empty row with defaults
    const newRow: Value[] = this.tableInfo.columns.map(col => {
      if (col.default) {
        // Would need to evaluate default expression
        return null;
      }
      return null;
    });

    this.newRows.set(newRowIndex, newRow);
    this.newRows = new Map(this.newRows); // Trigger reactivity

    return newRowIndex;
  }

  deleteRow(rowIndex: number) {
    if (!this.canEdit) return;

    if (rowIndex < 0) {
      // Delete new row
      this.newRows.delete(rowIndex);
      this.newRows = new Map(this.newRows);
    } else {
      // Mark existing row for deletion
      this.deletedRows.add(rowIndex);
      this.deletedRows = new Set(this.deletedRows);

      // Remove any pending updates
      this.changes.delete(rowIndex);
      this.changes = new Map(this.changes);
    }
  }

  undeleteRow(rowIndex: number) {
    this.deletedRows.delete(rowIndex);
    this.deletedRows = new Set(this.deletedRows);
  }

  setNullCell(rowIndex: number, columnName: string, originalValue: Value) {
    this.updateCell(rowIndex, columnName, originalValue, null);
  }

  getCellState(
    rowIndex: number,
    columnName: string
  ): 'unchanged' | 'modified' | 'new' | 'deleted' {
    if (rowIndex < 0 && this.newRows.has(rowIndex)) {
      return 'new';
    }

    if (this.deletedRows.has(rowIndex)) {
      return 'deleted';
    }

    const rowChange = this.changes.get(rowIndex);
    if (rowChange?.changes?.has(columnName)) {
      return 'modified';
    }

    return 'unchanged';
  }

  getModifiedValue(rowIndex: number, columnName: string): Value | undefined {
    if (rowIndex < 0) {
      const row = this.newRows.get(rowIndex);
      if (row) {
        const colIndex = this.tableInfo!.columns.findIndex(c => c.name === columnName);
        return row[colIndex];
      }
      return undefined;
    }

    const rowChange = this.changes.get(rowIndex);
    return rowChange?.changes?.get(columnName)?.new;
  }

  private valuesEqual(a: Value, b: Value): boolean {
    if (a === b) return true;
    if (a === null || b === null) return a === b;
    if (typeof a === 'object' && typeof b === 'object') {
      return JSON.stringify(a) === JSON.stringify(b);
    }
    return a === b;
  }

  generateSql(rows: Value[][]): string[] {
    if (!this.tableInfo) return [];

    const statements: string[] = [];
    const { schema, table, primaryKeyColumns, columns } = this.tableInfo;
    const tableName = `"${schema}"."${table}"`;

    // DELETE statements
    for (const rowIndex of this.deletedRows) {
      const row = rows[rowIndex];
      const whereClause = this.buildWhereClause(row, primaryKeyColumns, columns);
      statements.push(`DELETE FROM ${tableName} WHERE ${whereClause};`);
    }

    // UPDATE statements
    for (const [rowIndex, change] of this.changes) {
      if (change.type !== 'update' || !change.changes?.size) continue;

      const row = rows[rowIndex];
      const setClauses: string[] = [];

      for (const [colName, { new: newValue }] of change.changes) {
        setClauses.push(`"${colName}" = ${this.formatValue(newValue)}`);
      }

      const whereClause = this.buildWhereClause(row, primaryKeyColumns, columns);

      statements.push(
        `UPDATE ${tableName} SET ${setClauses.join(', ')} WHERE ${whereClause};`
      );
    }

    // INSERT statements
    for (const [_, newRow] of this.newRows) {
      const colNames = columns.map(c => `"${c.name}"`).join(', ');
      const values = newRow.map(v => this.formatValue(v)).join(', ');

      statements.push(
        `INSERT INTO ${tableName} (${colNames}) VALUES (${values});`
      );
    }

    return statements;
  }

  private buildWhereClause(
    row: Value[],
    pkColumns: string[],
    columns: ColumnMeta[]
  ): string {
    return pkColumns
      .map(pkCol => {
        const colIndex = columns.findIndex(c => c.name === pkCol);
        const value = row[colIndex];
        return `"${pkCol}" = ${this.formatValue(value)}`;
      })
      .join(' AND ');
  }

  private formatValue(value: Value): string {
    if (value === null) return 'NULL';
    if (typeof value === 'boolean') return value ? 'TRUE' : 'FALSE';
    if (typeof value === 'number') return String(value);
    if (typeof value === 'string') return `'${value.replace(/'/g, "''")}'`;
    if (typeof value === 'object') {
      if ('hex' in value) return `'\\x${value.hex}'`;
      return `'${JSON.stringify(value).replace(/'/g, "''")}'`;
    }
    return `'${String(value).replace(/'/g, "''")}'`;
  }

  async commit(connectionId: string, rows: Value[][]): Promise<{ success: boolean; error?: string }> {
    if (!this.hasChanges) return { success: true };

    const statements = this.generateSql(rows);
    if (statements.length === 0) return { success: true };

    try {
      // Execute all statements in a transaction
      const sql = `BEGIN;\n${statements.join('\n')}\nCOMMIT;`;

      await invoke('execute_query', {
        connId: connectionId,
        sql,
      });

      this.clearChanges();
      return { success: true };
    } catch (err) {
      return { success: false, error: String(err) };
    }
  }
}

export const editModeStore = new EditModeStore();
```

### 18.2 Editable Cell Component

```svelte
<!-- src/lib/components/grid/EditableCell.svelte -->
<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { editModeStore } from '$lib/stores/editMode.svelte';
  import type { Value } from '$lib/services/query';

  interface Props {
    rowIndex: number;
    columnName: string;
    columnType: string;
    value: Value;
    isEditable: boolean;
  }

  let {
    rowIndex,
    columnName,
    columnType,
    value,
    isEditable,
  }: Props = $props();

  let isEditing = $state(false);
  let editValue = $state('');
  let inputRef: HTMLInputElement | HTMLTextAreaElement;

  const cellState = $derived(
    editModeStore.getCellState(rowIndex, columnName)
  );

  const displayValue = $derived(() => {
    const modified = editModeStore.getModifiedValue(rowIndex, columnName);
    return modified !== undefined ? modified : value;
  });

  function formatForDisplay(val: Value): string {
    if (val === null) return 'NULL';
    if (typeof val === 'boolean') return val ? '✓' : '✗';
    if (typeof val === 'object') return JSON.stringify(val);
    return String(val);
  }

  function formatForEdit(val: Value): string {
    if (val === null) return '';
    if (typeof val === 'object') return JSON.stringify(val);
    return String(val);
  }

  function parseEditValue(text: string): Value {
    const trimmed = text.trim();

    // Empty string -> null for most types
    if (trimmed === '') return null;

    // Parse based on type
    switch (columnType) {
      case 'bool':
        const lower = trimmed.toLowerCase();
        if (['true', 't', 'yes', 'y', '1'].includes(lower)) return true;
        if (['false', 'f', 'no', 'n', '0'].includes(lower)) return false;
        return null;

      case 'int2':
      case 'int4':
      case 'int8':
        const intVal = parseInt(trimmed, 10);
        return isNaN(intVal) ? null : intVal;

      case 'float4':
      case 'float8':
      case 'numeric':
        const floatVal = parseFloat(trimmed);
        return isNaN(floatVal) ? null : floatVal;

      case 'json':
      case 'jsonb':
        try {
          return JSON.parse(trimmed);
        } catch {
          return trimmed; // Return as string if invalid JSON
        }

      default:
        return trimmed;
    }
  }

  async function startEditing() {
    if (!isEditable || cellState === 'deleted') return;

    isEditing = true;
    editValue = formatForEdit(displayValue());

    await tick();
    inputRef?.focus();
    inputRef?.select();
  }

  function commitEdit() {
    const newValue = parseEditValue(editValue);
    editModeStore.updateCell(rowIndex, columnName, value, newValue);
    isEditing = false;
  }

  function cancelEdit() {
    isEditing = false;
    editValue = '';
  }

  function handleKeydown(e: KeyboardEvent) {
    switch (e.key) {
      case 'Enter':
        if (!e.shiftKey) {
          e.preventDefault();
          commitEdit();
        }
        break;

      case 'Escape':
        cancelEdit();
        break;

      case 'Tab':
        commitEdit();
        // Let default tab behavior continue
        break;
    }
  }

  function handleBlur() {
    if (isEditing) {
      commitEdit();
    }
  }

  function setNull(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    editModeStore.setNullCell(rowIndex, columnName, value);
    isEditing = false;
  }
</script>

<div
  class="editable-cell"
  class:editing={isEditing}
  class:modified={cellState === 'modified'}
  class:new={cellState === 'new'}
  class:deleted={cellState === 'deleted'}
  class:null={displayValue() === null}
  ondblclick={startEditing}
  role="gridcell"
>
  {#if isEditing}
    {#if columnType === 'text' || columnType === 'json' || columnType === 'jsonb'}
      <textarea
        bind:this={inputRef}
        bind:value={editValue}
        onkeydown={handleKeydown}
        onblur={handleBlur}
        class="cell-input"
        rows="1"
      ></textarea>
    {:else if columnType === 'bool'}
      <select
        bind:this={inputRef}
        bind:value={editValue}
        onchange={commitEdit}
        onblur={handleBlur}
        class="cell-select"
      >
        <option value="">NULL</option>
        <option value="true">true</option>
        <option value="false">false</option>
      </select>
    {:else}
      <input
        bind:this={inputRef}
        bind:value={editValue}
        onkeydown={handleKeydown}
        onblur={handleBlur}
        class="cell-input"
        type={columnType.includes('int') || columnType.includes('float') || columnType === 'numeric' ? 'number' : 'text'}
      />
    {/if}

    <button class="null-btn" onclick={setNull} title="Set to NULL">
      ∅
    </button>
  {:else}
    <span class="cell-value">
      {formatForDisplay(displayValue())}
    </span>
  {/if}
</div>

<style>
  .editable-cell {
    position: relative;
    height: 100%;
    display: flex;
    align-items: center;
    padding: 0 8px;
  }

  .editable-cell.modified {
    background: #fef3c7;
  }

  .editable-cell.new {
    background: #dcfce7;
  }

  .editable-cell.deleted {
    background: #fee2e2;
    text-decoration: line-through;
    color: var(--text-muted);
  }

  .editable-cell.null .cell-value {
    color: var(--text-muted);
    font-style: italic;
  }

  .editing {
    padding: 0;
  }

  .cell-value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cell-input,
  .cell-select {
    width: 100%;
    height: 100%;
    padding: 0 8px;
    border: 2px solid var(--primary-color);
    background: white;
    font-family: inherit;
    font-size: inherit;
    outline: none;
  }

  .cell-input:focus,
  .cell-select:focus {
    border-color: var(--primary-color);
  }

  .null-btn {
    position: absolute;
    right: 2px;
    top: 50%;
    transform: translateY(-50%);
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 2px;
    background: var(--surface-secondary);
    color: var(--text-muted);
    font-size: 10px;
    cursor: pointer;
  }

  .null-btn:hover {
    background: var(--hover-color);
    color: var(--text-color);
  }
</style>
```

### 18.3 Edit Mode Toolbar

```svelte
<!-- src/lib/components/grid/EditToolbar.svelte -->
<script lang="ts">
  import { Check, X, Plus, Eye, AlertTriangle } from 'lucide-svelte';
  import { editModeStore } from '$lib/stores/editMode.svelte';

  interface Props {
    connectionId: string;
    rows: any[][];
    onCommit: () => void;
    onDiscard: () => void;
    onPreview: () => void;
  }

  let { connectionId, rows, onCommit, onDiscard, onPreview }: Props = $props();

  let isCommitting = $state(false);
  let showPreview = $state(false);

  const statements = $derived(
    editModeStore.generateSql(rows)
  );

  async function handleCommit() {
    isCommitting = true;
    try {
      const result = await editModeStore.commit(connectionId, rows);
      if (result.success) {
        onCommit();
      } else {
        // Show error
        alert(result.error);
      }
    } finally {
      isCommitting = false;
    }
  }

  function handleDiscard() {
    if (editModeStore.hasChanges) {
      if (!confirm('Discard all changes?')) return;
    }
    editModeStore.clearChanges();
    onDiscard();
  }

  function handleAddRow() {
    editModeStore.addRow();
  }
</script>

<div class="edit-toolbar">
  <div class="toolbar-info">
    <AlertTriangle size={16} class="warning-icon" />
    <span>Edit Mode: {$editModeStore.changeCount} changes pending</span>
  </div>

  <div class="toolbar-actions">
    <button class="btn btn-ghost" onclick={handleAddRow}>
      <Plus size={16} />
      Add Row
    </button>

    <button class="btn btn-ghost" onclick={() => showPreview = !showPreview}>
      <Eye size={16} />
      Preview SQL
    </button>

    <div class="toolbar-separator"></div>

    <button
      class="btn btn-primary"
      onclick={handleCommit}
      disabled={!$editModeStore.hasChanges || isCommitting}
    >
      <Check size={16} />
      {isCommitting ? 'Saving...' : `Save Changes (${$editModeStore.changeCount})`}
    </button>

    <button class="btn btn-ghost" onclick={handleDiscard}>
      <X size={16} />
      Discard
    </button>
  </div>
</div>

{#if showPreview}
  <div class="sql-preview">
    <div class="preview-header">
      <span>SQL Preview</span>
      <button class="close-btn" onclick={() => showPreview = false}>
        <X size={16} />
      </button>
    </div>
    <pre class="preview-sql">{statements.join('\n\n')}</pre>
  </div>
{/if}

<style>
  .edit-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 0.75rem;
    background: #fef3c7;
    border-bottom: 1px solid #fcd34d;
  }

  .toolbar-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    font-weight: 500;
    color: #92400e;
  }

  .toolbar-info :global(.warning-icon) {
    color: #f59e0b;
  }

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .toolbar-separator {
    width: 1px;
    height: 20px;
    background: #fcd34d;
    margin: 0 0.25rem;
  }

  .btn {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.75rem;
    border: none;
    border-radius: 0.375rem;
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-ghost {
    background: transparent;
    color: #92400e;
  }

  .btn-ghost:hover:not(:disabled) {
    background: rgba(0, 0, 0, 0.05);
  }

  .btn-primary {
    background: #16a34a;
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: #15803d;
  }

  .sql-preview {
    background: var(--surface-color);
    border-bottom: 1px solid var(--border-color);
  }

  .preview-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border-color);
    font-weight: 500;
    font-size: 0.875rem;
  }

  .close-btn {
    display: flex;
    padding: 0.25rem;
    border: none;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: 0.25rem;
  }

  .close-btn:hover {
    background: var(--hover-color);
  }

  .preview-sql {
    padding: 0.75rem;
    margin: 0;
    font-family: var(--font-mono);
    font-size: 0.8125rem;
    white-space: pre-wrap;
    max-height: 200px;
    overflow-y: auto;
    background: var(--surface-secondary);
  }
</style>
```

### 18.4 Backend Edit Commands

```rust
// src-tauri/src/commands/edit.rs

use tauri::State;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::models::query::Value;
use crate::state::AppState;

#[derive(Clone, Debug, serde::Deserialize)]
pub struct CellEdit {
    pub row_pk_values: Vec<Value>,
    pub column_name: String,
    pub new_value: Value,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct RowInsert {
    pub values: Vec<Value>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct RowDelete {
    pub pk_values: Vec<Value>,
}

#[tauri::command]
pub async fn check_table_editable(
    state: State<'_, AppState>,
    conn_id: String,
    schema: String,
    table: String,
) -> Result<EditableInfo> {
    let conn_uuid = Uuid::parse_str(&conn_id)?;

    // Get table info including primary key
    let table_info = state.schema_service
        .get_table(&conn_uuid, &schema, &table)
        .await?;

    let primary_key_columns = table_info.primary_key
        .map(|pk| pk.columns)
        .unwrap_or_default();

    Ok(EditableInfo {
        is_editable: !primary_key_columns.is_empty(),
        primary_key_columns,
        columns: table_info.columns.iter()
            .map(|c| ColumnEditInfo {
                name: c.name.clone(),
                type_name: c.type_name.clone(),
                nullable: c.nullable,
                has_default: c.default.is_some(),
                is_generated: c.is_generated,
            })
            .collect(),
    })
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct EditableInfo {
    pub is_editable: bool,
    pub primary_key_columns: Vec<String>,
    pub columns: Vec<ColumnEditInfo>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ColumnEditInfo {
    pub name: String,
    pub type_name: String,
    pub nullable: bool,
    pub has_default: bool,
    pub is_generated: bool,
}

#[tauri::command]
pub async fn execute_table_edits(
    state: State<'_, AppState>,
    conn_id: String,
    schema: String,
    table: String,
    updates: Vec<CellEdit>,
    inserts: Vec<RowInsert>,
    deletes: Vec<RowDelete>,
    pk_columns: Vec<String>,
) -> Result<EditResult> {
    let conn_uuid = Uuid::parse_str(&conn_id)?;
    let pool = state.connection_manager.get_pool(&conn_uuid).await?;
    let mut client = pool.get().await?;

    let tx = client.transaction().await?;

    let mut affected_rows = 0u64;

    // Process deletes first
    for delete in deletes {
        let sql = build_delete_sql(&schema, &table, &pk_columns, &delete.pk_values);
        affected_rows += tx.execute(&sql, &[]).await? as u64;
    }

    // Process updates
    for update in updates {
        let sql = build_update_sql(
            &schema,
            &table,
            &pk_columns,
            &update.row_pk_values,
            &update.column_name,
            &update.new_value,
        );
        affected_rows += tx.execute(&sql, &[]).await? as u64;
    }

    // Process inserts
    for insert in inserts {
        let sql = build_insert_sql(&schema, &table, &insert.values);
        affected_rows += tx.execute(&sql, &[]).await? as u64;
    }

    tx.commit().await?;

    Ok(EditResult {
        success: true,
        affected_rows,
        error: None,
    })
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct EditResult {
    pub success: bool,
    pub affected_rows: u64,
    pub error: Option<String>,
}

fn build_delete_sql(
    schema: &str,
    table: &str,
    pk_columns: &[String],
    pk_values: &[Value],
) -> String {
    let where_clause = pk_columns.iter()
        .zip(pk_values.iter())
        .map(|(col, val)| format!("\"{}\" = {}", col, format_value(val)))
        .collect::<Vec<_>>()
        .join(" AND ");

    format!("DELETE FROM \"{}\".\"{}\" WHERE {}", schema, table, where_clause)
}

fn build_update_sql(
    schema: &str,
    table: &str,
    pk_columns: &[String],
    pk_values: &[Value],
    column: &str,
    value: &Value,
) -> String {
    let where_clause = pk_columns.iter()
        .zip(pk_values.iter())
        .map(|(col, val)| format!("\"{}\" = {}", col, format_value(val)))
        .collect::<Vec<_>>()
        .join(" AND ");

    format!(
        "UPDATE \"{}\".\"{}\" SET \"{}\" = {} WHERE {}",
        schema, table, column, format_value(value), where_clause
    )
}

fn build_insert_sql(schema: &str, table: &str, values: &[Value]) -> String {
    let value_list = values.iter()
        .map(format_value)
        .collect::<Vec<_>>()
        .join(", ");

    format!("INSERT INTO \"{}\".\"{}\" VALUES ({})", schema, table, value_list)
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_string(),
        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        Value::Json(j) => format!("'{}'", j.to_string().replace('\'', "''")),
        _ => "NULL".to_string(),
    }
}
```

## Acceptance Criteria

1. **Edit Mode Activation**
   - Toggle edit mode button in table viewer
   - Only enable for tables with primary key
   - Show warning banner when active

2. **Cell Editing**
   - Double-click to edit cell
   - Tab/Enter to commit, Escape to cancel
   - Type-appropriate input controls
   - NULL button for setting null values

3. **Change Tracking**
   - Yellow highlight for modified cells
   - Green highlight for new rows
   - Red highlight with strikethrough for deleted rows
   - Change count in toolbar

4. **Row Operations**
   - Add new row button
   - Delete row (context menu or keyboard)
   - Undelete support

5. **SQL Preview**
   - Preview generated SQL statements
   - Shows DELETE, UPDATE, INSERT in order

6. **Commit/Discard**
   - Save Changes button with count
   - Execute in transaction
   - Rollback on any error
   - Discard all changes option

7. **Validation**
   - Respect NOT NULL constraints
   - Type validation on edit
   - Show errors inline

## MCP Testing Instructions

### Using Tauri MCP

```typescript
// Enable edit mode
await mcp.webview_click({
  selector: '[data-testid="edit-mode-btn"]',
  element: 'Edit mode button'
});

// Verify edit toolbar appears
await mcp.browser_wait_for({ text: 'Edit Mode' });

// Double-click a cell to edit
await mcp.webview_interact({
  action: 'double-click',
  selector: '.grid-cell[data-row="0"][data-col="1"]'
});

// Type new value
await mcp.webview_type({
  selector: '.cell-input',
  text: 'New Value'
});

// Press Enter to commit
await mcp.browser_press_key({ key: 'Enter' });

// Verify cell is marked as modified
const snapshot = await mcp.webview_dom_snapshot({ type: 'accessibility' });
assert(snapshot.includes('modified'));

// Preview SQL
await mcp.webview_click({
  selector: '[data-testid="preview-sql-btn"]',
  element: 'Preview SQL button'
});

// Verify SQL preview
await mcp.browser_wait_for({ text: 'UPDATE' });

// Save changes
await mcp.webview_click({
  selector: '[data-testid="save-changes-btn"]',
  element: 'Save changes button'
});

// Verify success
await mcp.browser_wait_for({ textGone: 'changes pending' });
```

## Dependencies

- Feature 14: Results Grid
- Feature 17: Table Data Viewer
- Feature 10: Schema Introspection (primary key detection)
- Feature 11: Query Execution
