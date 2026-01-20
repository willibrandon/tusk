# Feature 17: Table Data Viewer

## Overview

The table data viewer provides a dedicated interface for browsing and filtering table data. It combines the results grid with a visual filter builder, sortable columns, and pagination. This is the primary way users explore table contents without writing SQL.

## Goals

- Display table data with all grid features
- Provide visual filter builder with type-aware operators
- Support multi-column sorting
- Paginate large tables efficiently
- Show table metadata (columns, constraints, indexes)
- Enable transition to SQL query for complex filters

## Dependencies

- Feature 14: Results Grid (data display)
- Feature 11: Query Execution (data fetching)
- Feature 10: Schema Introspection (column metadata)

## Technical Specification

### 17.1 Table Viewer Component

```svelte
<!-- src/lib/components/viewer/TableViewer.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { Filter, Plus, X, ArrowUpDown, ChevronLeft, ChevronRight, Edit, Database } from 'lucide-svelte';
  import ResultsGrid from '$lib/components/grid/ResultsGrid.svelte';
  import FilterBuilder from './FilterBuilder.svelte';
  import { tableViewerStore, type TableFilter, type TableSort } from '$lib/stores/tableViewer.svelte';
  import { schemaStore } from '$lib/stores/schema.svelte';
  import { tabStore } from '$lib/stores/tabs.svelte';

  interface Props {
    connectionId: string;
    schema: string;
    table: string;
  }

  let { connectionId, schema, table }: Props = $props();

  let showFilterBuilder = $state(false);
  let editMode = $state(false);

  const tableInfo = $derived(
    schemaStore.getTable(connectionId, schema, table)
  );

  const viewerState = $derived(
    tableViewerStore.getState(connectionId, schema, table)
  );

  onMount(() => {
    tableViewerStore.init(connectionId, schema, table);
    return () => tableViewerStore.cleanup(connectionId, schema, table);
  });

  function addFilter(filter: TableFilter) {
    tableViewerStore.addFilter(connectionId, schema, table, filter);
  }

  function removeFilter(index: number) {
    tableViewerStore.removeFilter(connectionId, schema, table, index);
  }

  function clearFilters() {
    tableViewerStore.clearFilters(connectionId, schema, table);
  }

  function toggleSort(columnName: string) {
    tableViewerStore.toggleSort(connectionId, schema, table, columnName);
  }

  function setPage(page: number) {
    tableViewerStore.setPage(connectionId, schema, table, page);
  }

  function refresh() {
    tableViewerStore.refresh(connectionId, schema, table);
  }

  function openAsSql() {
    const sql = tableViewerStore.generateSql(connectionId, schema, table);
    tabStore.createQueryTab(connectionId, undefined, sql);
  }
</script>

<div class="table-viewer">
  <div class="viewer-header">
    <div class="table-info">
      <Database size={16} />
      <span class="schema-name">{schema}</span>
      <span class="separator">.</span>
      <span class="table-name">{table}</span>
      {#if tableInfo}
        <span class="row-estimate">
          ~{tableInfo.row_count_estimate?.toLocaleString() ?? '?'} rows
        </span>
      {/if}
    </div>

    <div class="header-actions">
      <button
        class="btn btn-ghost"
        onclick={() => showFilterBuilder = !showFilterBuilder}
        class:active={showFilterBuilder || viewerState?.filters.length > 0}
      >
        <Filter size={16} />
        Filter
        {#if viewerState?.filters.length}
          <span class="badge">{viewerState.filters.length}</span>
        {/if}
      </button>

      <button class="btn btn-ghost" onclick={openAsSql}>
        Open as SQL
      </button>

      <button
        class="btn btn-ghost"
        class:active={editMode}
        onclick={() => editMode = !editMode}
      >
        <Edit size={16} />
        Edit Mode
      </button>
    </div>
  </div>

  {#if showFilterBuilder || viewerState?.filters.length > 0}
    <div class="filter-section">
      {#if viewerState?.filters.length > 0}
        <div class="active-filters">
          {#each viewerState.filters as filter, i}
            <div class="filter-chip">
              <span class="filter-column">{filter.column}</span>
              <span class="filter-operator">{filter.operator}</span>
              <span class="filter-value">{filter.value || 'NULL'}</span>
              <button class="filter-remove" onclick={() => removeFilter(i)}>
                <X size={12} />
              </button>
            </div>
          {/each}
          <button class="clear-filters" onclick={clearFilters}>
            Clear all
          </button>
        </div>
      {/if}

      {#if showFilterBuilder && tableInfo}
        <FilterBuilder
          columns={tableInfo.columns}
          onAdd={addFilter}
          onClose={() => showFilterBuilder = false}
        />
      {/if}
    </div>
  {/if}

  <div class="viewer-content">
    {#if viewerState?.isLoading && !viewerState.rows.length}
      <div class="loading">Loading data...</div>
    {:else if viewerState?.error}
      <div class="error">{viewerState.error}</div>
    {:else}
      <ResultsGrid
        {editMode}
        onColumnSort={toggleSort}
      />
    {/if}
  </div>

  <div class="viewer-footer">
    <div class="pagination">
      <button
        class="page-btn"
        disabled={viewerState?.currentPage === 1}
        onclick={() => setPage(1)}
      >
        First
      </button>
      <button
        class="page-btn"
        disabled={viewerState?.currentPage === 1}
        onclick={() => setPage(viewerState!.currentPage - 1)}
      >
        <ChevronLeft size={16} />
      </button>

      <span class="page-info">
        Page {viewerState?.currentPage ?? 1} of {viewerState?.totalPages ?? 1}
      </span>

      <button
        class="page-btn"
        disabled={viewerState?.currentPage === viewerState?.totalPages}
        onclick={() => setPage(viewerState!.currentPage + 1)}
      >
        <ChevronRight size={16} />
      </button>
      <button
        class="page-btn"
        disabled={viewerState?.currentPage === viewerState?.totalPages}
        onclick={() => setPage(viewerState!.totalPages)}
      >
        Last
      </button>
    </div>

    <div class="row-info">
      {#if viewerState}
        Showing {((viewerState.currentPage - 1) * viewerState.pageSize + 1).toLocaleString()}
        - {Math.min(viewerState.currentPage * viewerState.pageSize, viewerState.totalRows).toLocaleString()}
        of {viewerState.totalRows.toLocaleString()} rows
      {/if}
    </div>

    <div class="sort-info">
      {#if viewerState?.sorts.length}
        Sort: {viewerState.sorts.map(s => `${s.column} ${s.direction.toUpperCase()}`).join(', ')}
      {/if}
    </div>
  </div>
</div>

<style>
  .table-viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--background-color);
  }

  .viewer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem;
    border-bottom: 1px solid var(--border-color);
    background: var(--surface-color);
  }

  .table-info {
    display: flex;
    align-items: center;
    gap: 0.375rem;
  }

  .schema-name {
    color: var(--text-muted);
  }

  .separator {
    color: var(--text-muted);
  }

  .table-name {
    font-weight: 600;
  }

  .row-estimate {
    margin-left: 0.5rem;
    padding: 0.125rem 0.5rem;
    background: var(--surface-secondary);
    border-radius: 1rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
  }

  .btn {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 0.375rem;
    background: none;
    color: var(--text-color);
    font-size: 0.8125rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn:hover {
    background: var(--hover-color);
  }

  .btn.active {
    background: var(--primary-color);
    border-color: var(--primary-color);
    color: white;
  }

  .badge {
    padding: 0.0625rem 0.375rem;
    background: white;
    color: var(--primary-color);
    border-radius: 1rem;
    font-size: 0.6875rem;
    font-weight: 600;
  }

  .filter-section {
    padding: 0.75rem;
    border-bottom: 1px solid var(--border-color);
    background: var(--surface-secondary);
  }

  .active-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .filter-chip {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    background: var(--surface-color);
    border: 1px solid var(--border-color);
    border-radius: 0.25rem;
    font-size: 0.8125rem;
  }

  .filter-column {
    font-weight: 500;
  }

  .filter-operator {
    color: var(--primary-color);
  }

  .filter-value {
    font-family: var(--font-mono);
  }

  .filter-remove {
    display: flex;
    margin-left: 0.25rem;
    padding: 0.125rem;
    border: none;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: 0.25rem;
  }

  .filter-remove:hover {
    background: var(--hover-color);
    color: var(--text-color);
  }

  .clear-filters {
    padding: 0.25rem 0.5rem;
    border: none;
    background: none;
    color: var(--text-muted);
    font-size: 0.8125rem;
    cursor: pointer;
  }

  .clear-filters:hover {
    color: var(--primary-color);
  }

  .viewer-content {
    flex: 1;
    overflow: hidden;
  }

  .loading, .error {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
  }

  .error {
    color: #ef4444;
  }

  .viewer-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 0.75rem;
    border-top: 1px solid var(--border-color);
    background: var(--surface-color);
    font-size: 0.8125rem;
  }

  .pagination {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .page-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 28px;
    height: 28px;
    padding: 0 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 0.25rem;
    background: none;
    color: var(--text-color);
    font-size: 0.8125rem;
    cursor: pointer;
  }

  .page-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .page-btn:hover:not(:disabled) {
    background: var(--hover-color);
  }

  .page-info {
    padding: 0 0.5rem;
    color: var(--text-muted);
  }

  .row-info, .sort-info {
    color: var(--text-muted);
  }
</style>
```

### 17.2 Filter Builder Component

```svelte
<!-- src/lib/components/viewer/FilterBuilder.svelte -->
<script lang="ts">
  import { Plus } from 'lucide-svelte';
  import type { Column } from '$lib/services/schema';
  import type { TableFilter } from '$lib/stores/tableViewer.svelte';

  interface Props {
    columns: Column[];
    onAdd: (filter: TableFilter) => void;
    onClose: () => void;
  }

  let { columns, onAdd, onClose }: Props = $props();

  let selectedColumn = $state(columns[0]?.name ?? '');
  let operator = $state('=');
  let value = $state('');

  const selectedColumnInfo = $derived(
    columns.find(c => c.name === selectedColumn)
  );

  const operatorOptions = $derived(
    getOperatorsForType(selectedColumnInfo?.base_type ?? 'text')
  );

  function getOperatorsForType(type: string): { value: string; label: string }[] {
    const base = [
      { value: '=', label: '=' },
      { value: '!=', label: '!=' },
      { value: 'IS NULL', label: 'IS NULL' },
      { value: 'IS NOT NULL', label: 'IS NOT NULL' },
    ];

    switch (type) {
      case 'int2':
      case 'int4':
      case 'int8':
      case 'float4':
      case 'float8':
      case 'numeric':
      case 'money':
        return [
          ...base,
          { value: '<', label: '<' },
          { value: '<=', label: '<=' },
          { value: '>', label: '>' },
          { value: '>=', label: '>=' },
          { value: 'BETWEEN', label: 'BETWEEN' },
        ];

      case 'text':
      case 'varchar':
      case 'bpchar':
        return [
          ...base,
          { value: 'LIKE', label: 'LIKE' },
          { value: 'ILIKE', label: 'ILIKE' },
          { value: 'NOT LIKE', label: 'NOT LIKE' },
          { value: 'NOT ILIKE', label: 'NOT ILIKE' },
          { value: 'SIMILAR TO', label: 'SIMILAR TO' },
        ];

      case 'timestamp':
      case 'timestamptz':
      case 'date':
        return [
          ...base,
          { value: '<', label: 'Before' },
          { value: '>', label: 'After' },
          { value: 'BETWEEN', label: 'Between' },
        ];

      case 'bool':
        return [
          { value: '= TRUE', label: 'is TRUE' },
          { value: '= FALSE', label: 'is FALSE' },
          { value: 'IS NULL', label: 'IS NULL' },
        ];

      case 'jsonb':
      case 'json':
        return [
          ...base,
          { value: '@>', label: 'contains (@>)' },
          { value: '<@', label: 'contained by (<@)' },
          { value: '?', label: 'has key (?)' },
          { value: '?|', label: 'has any key (?|)' },
          { value: '?&', label: 'has all keys (?&)' },
        ];

      case 'array':
        return [
          ...base,
          { value: '@>', label: 'contains (@>)' },
          { value: '<@', label: 'contained by (<@)' },
          { value: '&&', label: 'overlaps (&&)' },
        ];

      default:
        return base;
    }
  }

  function handleAdd() {
    if (!selectedColumn) return;

    // NULL operators don't need a value
    if (operator === 'IS NULL' || operator === 'IS NOT NULL') {
      onAdd({ column: selectedColumn, operator, value: '' });
    } else if (!value.trim() && operator !== '= TRUE' && operator !== '= FALSE') {
      return; // Need value for other operators
    } else {
      onAdd({ column: selectedColumn, operator, value: value.trim() });
    }

    // Reset
    value = '';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      handleAdd();
    } else if (e.key === 'Escape') {
      onClose();
    }
  }

  function needsValue(op: string): boolean {
    return !['IS NULL', 'IS NOT NULL', '= TRUE', '= FALSE'].includes(op);
  }
</script>

<div class="filter-builder">
  <select
    class="filter-select column-select"
    bind:value={selectedColumn}
  >
    {#each columns as col}
      <option value={col.name}>
        {col.name} ({col.type})
      </option>
    {/each}
  </select>

  <select
    class="filter-select operator-select"
    bind:value={operator}
  >
    {#each operatorOptions as op}
      <option value={op.value}>{op.label}</option>
    {/each}
  </select>

  {#if needsValue(operator)}
    {#if operator === 'BETWEEN'}
      <input
        class="filter-input"
        type="text"
        placeholder="Min value"
        bind:value
        onkeydown={handleKeydown}
      />
      <span class="between-and">and</span>
      <input
        class="filter-input"
        type="text"
        placeholder="Max value"
        onkeydown={handleKeydown}
      />
    {:else}
      <input
        class="filter-input"
        type="text"
        placeholder="Value"
        bind:value
        onkeydown={handleKeydown}
      />
    {/if}
  {/if}

  <button class="add-btn" onclick={handleAdd}>
    <Plus size={16} />
    Add
  </button>
</div>

<style>
  .filter-builder {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .filter-select {
    padding: 0.375rem 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 0.375rem;
    background: var(--surface-color);
    font-size: 0.8125rem;
  }

  .column-select {
    min-width: 150px;
  }

  .operator-select {
    min-width: 100px;
  }

  .filter-input {
    padding: 0.375rem 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 0.375rem;
    background: var(--surface-color);
    font-size: 0.8125rem;
    min-width: 150px;
  }

  .filter-input:focus {
    outline: none;
    border-color: var(--primary-color);
  }

  .between-and {
    color: var(--text-muted);
    font-size: 0.8125rem;
  }

  .add-btn {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.375rem 0.75rem;
    border: none;
    border-radius: 0.375rem;
    background: var(--primary-color);
    color: white;
    font-size: 0.8125rem;
    cursor: pointer;
    transition: background 0.15s;
  }

  .add-btn:hover {
    background: var(--primary-hover);
  }
</style>
```

### 17.3 Table Viewer Store

```typescript
// src/lib/stores/tableViewer.svelte.ts

import { invoke } from '@tauri-apps/api/core';
import { gridState } from './gridState.svelte';
import type { ColumnMeta, Value } from '$lib/services/query';

export interface TableFilter {
  column: string;
  operator: string;
  value: string;
}

export interface TableSort {
  column: string;
  direction: 'asc' | 'desc';
}

export interface TableViewerState {
  connectionId: string;
  schema: string;
  table: string;
  filters: TableFilter[];
  sorts: TableSort[];
  currentPage: number;
  pageSize: number;
  totalRows: number;
  totalPages: number;
  isLoading: boolean;
  error: string | null;
  rows: Value[][];
  columns: ColumnMeta[];
}

class TableViewerStore {
  private states = $state<Map<string, TableViewerState>>(new Map());

  private getKey(connectionId: string, schema: string, table: string): string {
    return `${connectionId}:${schema}:${table}`;
  }

  getState(connectionId: string, schema: string, table: string): TableViewerState | undefined {
    return this.states.get(this.getKey(connectionId, schema, table));
  }

  async init(connectionId: string, schema: string, table: string) {
    const key = this.getKey(connectionId, schema, table);

    // Initialize state if not exists
    if (!this.states.has(key)) {
      this.states.set(key, {
        connectionId,
        schema,
        table,
        filters: [],
        sorts: [],
        currentPage: 1,
        pageSize: 1000,
        totalRows: 0,
        totalPages: 0,
        isLoading: false,
        error: null,
        rows: [],
        columns: [],
      });
    }

    await this.fetchData(connectionId, schema, table);
  }

  cleanup(connectionId: string, schema: string, table: string) {
    // Optionally cleanup state when viewer closes
  }

  async fetchData(connectionId: string, schema: string, table: string) {
    const key = this.getKey(connectionId, schema, table);
    const state = this.states.get(key);
    if (!state) return;

    state.isLoading = true;
    state.error = null;

    try {
      // First, get total count
      const countSql = this.buildCountSql(state);
      const countResult = await invoke<any>('execute_query', {
        connId: connectionId,
        sql: countSql,
      });

      const totalRows = Number(countResult.rows?.[0]?.[0] ?? 0);
      state.totalRows = totalRows;
      state.totalPages = Math.ceil(totalRows / state.pageSize);

      // Then fetch page data
      const dataSql = this.buildDataSql(state);
      const result = await invoke<any>('execute_query', {
        connId: connectionId,
        sql: dataSql,
      });

      state.columns = result.columns ?? [];
      state.rows = result.rows ?? [];

      // Update grid state
      gridState.setData(state.columns, state.rows, state.rows.length, result.query_id);
    } catch (err) {
      state.error = String(err);
    } finally {
      state.isLoading = false;
    }
  }

  private buildCountSql(state: TableViewerState): string {
    let sql = `SELECT COUNT(*) FROM "${state.schema}"."${state.table}"`;

    if (state.filters.length > 0) {
      const whereClauses = state.filters.map(f => this.filterToSql(f));
      sql += ` WHERE ${whereClauses.join(' AND ')}`;
    }

    return sql;
  }

  private buildDataSql(state: TableViewerState): string {
    let sql = `SELECT * FROM "${state.schema}"."${state.table}"`;

    // WHERE clause
    if (state.filters.length > 0) {
      const whereClauses = state.filters.map(f => this.filterToSql(f));
      sql += ` WHERE ${whereClauses.join(' AND ')}`;
    }

    // ORDER BY clause
    if (state.sorts.length > 0) {
      const orderClauses = state.sorts.map(s =>
        `"${s.column}" ${s.direction.toUpperCase()}`
      );
      sql += ` ORDER BY ${orderClauses.join(', ')}`;
    }

    // LIMIT and OFFSET
    const offset = (state.currentPage - 1) * state.pageSize;
    sql += ` LIMIT ${state.pageSize} OFFSET ${offset}`;

    return sql;
  }

  private filterToSql(filter: TableFilter): string {
    const col = `"${filter.column}"`;

    switch (filter.operator) {
      case 'IS NULL':
        return `${col} IS NULL`;
      case 'IS NOT NULL':
        return `${col} IS NOT NULL`;
      case '= TRUE':
        return `${col} = TRUE`;
      case '= FALSE':
        return `${col} = FALSE`;
      case 'LIKE':
      case 'ILIKE':
      case 'NOT LIKE':
      case 'NOT ILIKE':
      case 'SIMILAR TO':
        return `${col} ${filter.operator} '${this.escapeString(filter.value)}'`;
      case '@>':
      case '<@':
      case '?':
      case '?|':
      case '?&':
      case '&&':
        // JSON/Array operators - value should be properly formatted
        return `${col} ${filter.operator} '${this.escapeString(filter.value)}'`;
      case 'BETWEEN':
        const [min, max] = filter.value.split(',').map(v => v.trim());
        return `${col} BETWEEN '${this.escapeString(min)}' AND '${this.escapeString(max)}'`;
      default:
        return `${col} ${filter.operator} '${this.escapeString(filter.value)}'`;
    }
  }

  private escapeString(value: string): string {
    return value.replace(/'/g, "''");
  }

  addFilter(connectionId: string, schema: string, table: string, filter: TableFilter) {
    const state = this.getState(connectionId, schema, table);
    if (!state) return;

    state.filters = [...state.filters, filter];
    state.currentPage = 1; // Reset to first page
    this.fetchData(connectionId, schema, table);
  }

  removeFilter(connectionId: string, schema: string, table: string, index: number) {
    const state = this.getState(connectionId, schema, table);
    if (!state) return;

    state.filters = state.filters.filter((_, i) => i !== index);
    state.currentPage = 1;
    this.fetchData(connectionId, schema, table);
  }

  clearFilters(connectionId: string, schema: string, table: string) {
    const state = this.getState(connectionId, schema, table);
    if (!state) return;

    state.filters = [];
    state.currentPage = 1;
    this.fetchData(connectionId, schema, table);
  }

  toggleSort(connectionId: string, schema: string, table: string, column: string) {
    const state = this.getState(connectionId, schema, table);
    if (!state) return;

    const existingIndex = state.sorts.findIndex(s => s.column === column);

    if (existingIndex >= 0) {
      const existing = state.sorts[existingIndex];
      if (existing.direction === 'asc') {
        // Change to desc
        state.sorts[existingIndex] = { ...existing, direction: 'desc' };
      } else {
        // Remove sort
        state.sorts = state.sorts.filter((_, i) => i !== existingIndex);
      }
    } else {
      // Add new sort
      state.sorts = [...state.sorts, { column, direction: 'asc' }];
    }

    state.sorts = [...state.sorts]; // Trigger reactivity
    this.fetchData(connectionId, schema, table);
  }

  setPage(connectionId: string, schema: string, table: string, page: number) {
    const state = this.getState(connectionId, schema, table);
    if (!state) return;

    state.currentPage = Math.max(1, Math.min(page, state.totalPages));
    this.fetchData(connectionId, schema, table);
  }

  refresh(connectionId: string, schema: string, table: string) {
    this.fetchData(connectionId, schema, table);
  }

  generateSql(connectionId: string, schema: string, table: string): string {
    const state = this.getState(connectionId, schema, table);
    if (!state) return '';

    let sql = `SELECT *\nFROM "${state.schema}"."${state.table}"`;

    if (state.filters.length > 0) {
      const whereClauses = state.filters.map(f => this.filterToSql(f));
      sql += `\nWHERE ${whereClauses.join('\n  AND ')}`;
    }

    if (state.sorts.length > 0) {
      const orderClauses = state.sorts.map(s =>
        `"${s.column}" ${s.direction.toUpperCase()}`
      );
      sql += `\nORDER BY ${orderClauses.join(', ')}`;
    }

    sql += `\nLIMIT ${state.pageSize};`;

    return sql;
  }
}

export const tableViewerStore = new TableViewerStore();
```

## Acceptance Criteria

1. **Data Display**
   - Show table data in grid with all column types
   - Display row count and table info in header
   - Support virtual scrolling for large results

2. **Filtering**
   - Visual filter builder with column selection
   - Type-appropriate operators (text: LIKE, ILIKE; numeric: <, >; etc.)
   - Multiple active filters with AND logic
   - Filter chips showing active filters
   - Clear individual or all filters

3. **Sorting**
   - Click column header to sort
   - Multi-column sorting supported
   - Sort indicator in column header
   - Sort info in footer

4. **Pagination**
   - Navigate pages with buttons
   - Show current page and total pages
   - Display row range (1-1000 of 50000)
   - Configurable page size

5. **Edit Mode Toggle**
   - Toggle button to enable editing
   - Visual indicator when edit mode active
   - Integrates with Feature 18 (Inline Editing)

6. **SQL Export**
   - "Open as SQL" generates equivalent query
   - Opens in new query tab
   - Includes filters and sorting

## MCP Testing Instructions

### Using Tauri MCP

```typescript
// Open table viewer
await mcp.ipc_execute_command({
  command: 'create_table_tab',
  args: { connectionId, schema: 'public', table: 'users' }
});

// Verify table viewer renders
const snapshot = await mcp.webview_dom_snapshot({ type: 'accessibility' });
assert(snapshot.includes('public.users'));

// Add a filter
await mcp.webview_click({ selector: '.filter-btn', element: 'Filter button' });
await mcp.webview_fill_form({
  fields: [
    { name: 'Column', type: 'combobox', ref: 'column-select', value: 'status' },
    { name: 'Operator', type: 'combobox', ref: 'operator-select', value: '=' },
    { name: 'Value', type: 'textbox', ref: 'value-input', value: 'active' },
  ]
});
await mcp.webview_click({ selector: '.add-btn', element: 'Add filter' });

// Verify filter applied
await mcp.browser_wait_for({ text: 'status = active' });

// Test pagination
await mcp.webview_click({ selector: '.page-btn:last-child', element: 'Next page' });

// Verify page changed
await mcp.browser_wait_for({ text: 'Page 2' });
```

## Dependencies

- Feature 14: Results Grid
- Feature 11: Query Execution
- Feature 10: Schema Introspection
