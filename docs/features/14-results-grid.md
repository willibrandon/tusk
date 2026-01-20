# Feature 14: Results Grid

## Overview

The results grid displays query results in a high-performance, spreadsheet-like interface with virtual scrolling to handle millions of rows. It supports multiple display modes, column operations, cell selection, and type-specific rendering for all PostgreSQL data types.

## Goals

- Display query results in a performant virtualized grid
- Handle 10M+ rows through virtual scrolling
- Render all PostgreSQL types with appropriate formatting
- Support column resizing, reordering, sorting, and hiding
- Enable cell and range selection with keyboard navigation
- Provide multiple display modes (grid, transposed, JSON)

## Dependencies

- Feature 03: Frontend Architecture (Svelte component structure)
- Feature 11: Query Execution (result streaming and column metadata)
- Feature 04: IPC Layer (event streaming for large results)

## Technical Specification

### 14.1 Virtual Scrolling Core

```typescript
// src/lib/components/grid/virtualScroll.ts

export interface VirtualScrollState {
	visibleStartIndex: number;
	visibleEndIndex: number;
	offsetY: number;
	totalHeight: number;
}

export interface VirtualScrollConfig {
	rowHeight: number;
	overscan: number; // Extra rows to render outside viewport
	containerHeight: number;
}

export function calculateVirtualScroll(
	scrollTop: number,
	totalRows: number,
	config: VirtualScrollConfig
): VirtualScrollState {
	const { rowHeight, overscan, containerHeight } = config;

	const visibleCount = Math.ceil(containerHeight / rowHeight);
	const startIndex = Math.floor(scrollTop / rowHeight);

	const visibleStartIndex = Math.max(0, startIndex - overscan);
	const visibleEndIndex = Math.min(totalRows, startIndex + visibleCount + overscan);

	const offsetY = visibleStartIndex * rowHeight;
	const totalHeight = totalRows * rowHeight;

	return {
		visibleStartIndex,
		visibleEndIndex,
		offsetY,
		totalHeight
	};
}

export interface HorizontalVirtualState {
	visibleStartIndex: number;
	visibleEndIndex: number;
	offsetX: number;
	totalWidth: number;
}

export function calculateHorizontalVirtual(
	scrollLeft: number,
	columns: { width: number }[],
	containerWidth: number,
	overscan: number = 2
): HorizontalVirtualState {
	let accumulatedWidth = 0;
	let visibleStartIndex = 0;
	let offsetX = 0;

	// Find start index
	for (let i = 0; i < columns.length; i++) {
		if (accumulatedWidth + columns[i].width >= scrollLeft) {
			visibleStartIndex = Math.max(0, i - overscan);
			offsetX = columns.slice(0, visibleStartIndex).reduce((sum, c) => sum + c.width, 0);
			break;
		}
		accumulatedWidth += columns[i].width;
	}

	// Find end index
	accumulatedWidth = offsetX;
	let visibleEndIndex = visibleStartIndex;

	for (let i = visibleStartIndex; i < columns.length; i++) {
		accumulatedWidth += columns[i].width;
		visibleEndIndex = i + 1;
		if (accumulatedWidth >= scrollLeft + containerWidth + overscan * 100) {
			break;
		}
	}

	visibleEndIndex = Math.min(columns.length, visibleEndIndex + overscan);

	const totalWidth = columns.reduce((sum, c) => sum + c.width, 0);

	return {
		visibleStartIndex,
		visibleEndIndex,
		offsetX,
		totalWidth
	};
}
```

### 14.2 Grid State Management

```typescript
// src/lib/stores/gridState.svelte.ts

import type { ColumnMeta, Value } from '$lib/services/query';

export interface GridColumn {
	name: string;
	type: string;
	typeOid: number;
	width: number;
	minWidth: number;
	hidden: boolean;
	sortDirection: 'asc' | 'desc' | null;
	sortOrder: number | null;
}

export interface CellSelection {
	rowIndex: number;
	colIndex: number;
}

export interface RangeSelection {
	startRow: number;
	startCol: number;
	endRow: number;
	endCol: number;
}

export type DisplayMode = 'grid' | 'transposed' | 'json';

class GridState {
	// Data
	rows = $state<Value[][]>([]);
	columns = $state<GridColumn[]>([]);
	totalRows = $state(0);
	isLoading = $state(false);
	queryId = $state<string | null>(null);

	// Display
	displayMode = $state<DisplayMode>('grid');
	rowHeight = $state(28);

	// Selection
	selectedCell = $state<CellSelection | null>(null);
	rangeSelection = $state<RangeSelection | null>(null);
	selectedRows = $state<Set<number>>(new Set());

	// Sorting (client-side for loaded data)
	sortColumns = $state<{ colIndex: number; direction: 'asc' | 'desc' }[]>([]);

	// Scroll position
	scrollTop = $state(0);
	scrollLeft = $state(0);

	setData(columns: ColumnMeta[], rows: Value[][], totalRows: number, queryId: string) {
		this.columns = columns.map((col, i) => ({
			name: col.name,
			type: col.type_name,
			typeOid: col.type_oid,
			width: this.calculateColumnWidth(col, rows, i),
			minWidth: 50,
			hidden: false,
			sortDirection: null,
			sortOrder: null
		}));
		this.rows = rows;
		this.totalRows = totalRows;
		this.queryId = queryId;
		this.clearSelection();
	}

	appendRows(newRows: Value[][]) {
		this.rows = [...this.rows, ...newRows];
	}

	private calculateColumnWidth(col: ColumnMeta, rows: Value[][], colIndex: number): number {
		// Start with column name width
		let maxWidth = col.name.length * 8 + 32;

		// Sample first 100 rows for content width
		const sampleRows = rows.slice(0, 100);
		for (const row of sampleRows) {
			const value = row[colIndex];
			const displayValue = this.formatValueForWidth(value);
			const width = displayValue.length * 7 + 16;
			maxWidth = Math.max(maxWidth, width);
		}

		// Cap at reasonable limits
		return Math.min(Math.max(maxWidth, 60), 400);
	}

	private formatValueForWidth(value: Value): string {
		if (value === null) return 'NULL';
		if (typeof value === 'boolean') return value.toString();
		if (typeof value === 'number') return value.toString();
		if (typeof value === 'string') return value;
		if (Array.isArray(value)) return `[${value.length} items]`;
		if (typeof value === 'object') return JSON.stringify(value).slice(0, 50);
		return String(value);
	}

	// Selection methods
	selectCell(rowIndex: number, colIndex: number) {
		this.selectedCell = { rowIndex, colIndex };
		this.rangeSelection = null;
	}

	selectRange(startRow: number, startCol: number, endRow: number, endCol: number) {
		this.rangeSelection = {
			startRow: Math.min(startRow, endRow),
			startCol: Math.min(startCol, endCol),
			endRow: Math.max(startRow, endRow),
			endCol: Math.max(startCol, endCol)
		};
	}

	extendSelection(toRow: number, toCol: number) {
		if (!this.selectedCell) return;

		this.rangeSelection = {
			startRow: Math.min(this.selectedCell.rowIndex, toRow),
			startCol: Math.min(this.selectedCell.colIndex, toCol),
			endRow: Math.max(this.selectedCell.rowIndex, toRow),
			endCol: Math.max(this.selectedCell.colIndex, toCol)
		};
	}

	selectAll() {
		if (this.rows.length === 0 || this.columns.length === 0) return;

		this.rangeSelection = {
			startRow: 0,
			startCol: 0,
			endRow: this.rows.length - 1,
			endCol: this.columns.length - 1
		};
	}

	clearSelection() {
		this.selectedCell = null;
		this.rangeSelection = null;
		this.selectedRows.clear();
	}

	isCellSelected(rowIndex: number, colIndex: number): boolean {
		if (this.selectedCell?.rowIndex === rowIndex && this.selectedCell?.colIndex === colIndex) {
			return true;
		}

		if (this.rangeSelection) {
			const { startRow, startCol, endRow, endCol } = this.rangeSelection;
			return (
				rowIndex >= startRow && rowIndex <= endRow && colIndex >= startCol && colIndex <= endCol
			);
		}

		return false;
	}

	// Column operations
	resizeColumn(colIndex: number, width: number) {
		if (colIndex < 0 || colIndex >= this.columns.length) return;
		this.columns[colIndex].width = Math.max(width, this.columns[colIndex].minWidth);
	}

	reorderColumns(fromIndex: number, toIndex: number) {
		const newColumns = [...this.columns];
		const [moved] = newColumns.splice(fromIndex, 1);
		newColumns.splice(toIndex, 0, moved);
		this.columns = newColumns;

		// Also reorder row data
		this.rows = this.rows.map((row) => {
			const newRow = [...row];
			const [movedValue] = newRow.splice(fromIndex, 1);
			newRow.splice(toIndex, 0, movedValue);
			return newRow;
		});
	}

	toggleColumnVisibility(colIndex: number) {
		if (colIndex < 0 || colIndex >= this.columns.length) return;
		this.columns[colIndex].hidden = !this.columns[colIndex].hidden;
	}

	autoSizeColumn(colIndex: number) {
		const col = this.columns[colIndex];
		if (!col) return;

		let maxWidth = col.name.length * 8 + 32;

		for (const row of this.rows) {
			const value = row[colIndex];
			const displayValue = this.formatValueForWidth(value);
			const width = displayValue.length * 7 + 16;
			maxWidth = Math.max(maxWidth, width);
		}

		this.columns[colIndex].width = Math.min(maxWidth, 500);
	}

	autoSizeAllColumns() {
		this.columns.forEach((_, i) => this.autoSizeColumn(i));
	}

	// Sorting (client-side)
	sortByColumn(colIndex: number, multi: boolean = false) {
		const col = this.columns[colIndex];

		if (!multi) {
			// Clear other sorts
			this.columns.forEach((c, i) => {
				if (i !== colIndex) {
					c.sortDirection = null;
					c.sortOrder = null;
				}
			});
			this.sortColumns = [];
		}

		// Toggle or set sort direction
		if (col.sortDirection === 'asc') {
			col.sortDirection = 'desc';
		} else if (col.sortDirection === 'desc') {
			col.sortDirection = null;
		} else {
			col.sortDirection = 'asc';
		}

		// Update sort order
		if (col.sortDirection) {
			const existingIndex = this.sortColumns.findIndex((s) => s.colIndex === colIndex);
			if (existingIndex >= 0) {
				this.sortColumns[existingIndex].direction = col.sortDirection;
			} else {
				this.sortColumns.push({ colIndex, direction: col.sortDirection });
			}
		} else {
			this.sortColumns = this.sortColumns.filter((s) => s.colIndex !== colIndex);
		}

		// Apply sort
		this.applySorting();
	}

	private applySorting() {
		if (this.sortColumns.length === 0) return;

		this.rows = [...this.rows].sort((a, b) => {
			for (const { colIndex, direction } of this.sortColumns) {
				const aVal = a[colIndex];
				const bVal = b[colIndex];
				const comparison = this.compareValues(aVal, bVal);
				if (comparison !== 0) {
					return direction === 'asc' ? comparison : -comparison;
				}
			}
			return 0;
		});
	}

	private compareValues(a: Value, b: Value): number {
		// Handle nulls
		if (a === null && b === null) return 0;
		if (a === null) return -1;
		if (b === null) return 1;

		// Handle different types
		if (typeof a === 'number' && typeof b === 'number') {
			return a - b;
		}

		if (typeof a === 'string' && typeof b === 'string') {
			return a.localeCompare(b);
		}

		if (typeof a === 'boolean' && typeof b === 'boolean') {
			return a === b ? 0 : a ? 1 : -1;
		}

		// Convert to strings for comparison
		return String(a).localeCompare(String(b));
	}

	// Get selected data for copying
	getSelectedData(): Value[][] {
		if (this.rangeSelection) {
			const { startRow, startCol, endRow, endCol } = this.rangeSelection;
			return this.rows.slice(startRow, endRow + 1).map((row) => row.slice(startCol, endCol + 1));
		}

		if (this.selectedCell) {
			const { rowIndex, colIndex } = this.selectedCell;
			return [[this.rows[rowIndex]?.[colIndex]]];
		}

		return [];
	}

	getSelectedColumns(): GridColumn[] {
		if (this.rangeSelection) {
			return this.columns.slice(this.rangeSelection.startCol, this.rangeSelection.endCol + 1);
		}

		if (this.selectedCell) {
			return [this.columns[this.selectedCell.colIndex]];
		}

		return [];
	}
}

export const gridState = new GridState();
```

### 14.3 Cell Renderer

```typescript
// src/lib/components/grid/CellRenderer.svelte
<script lang="ts">
  import { settingsStore } from '$lib/stores/settings.svelte';
  import type { Value } from '$lib/services/query';

  interface Props {
    value: Value;
    typeName: string;
    isSelected?: boolean;
    onExpand?: () => void;
  }

  let { value, typeName, isSelected = false, onExpand }: Props = $props();

  function formatValue(val: Value, type: string): { display: string; className: string } {
    if (val === null) {
      return { display: $settingsStore.nullDisplay || 'NULL', className: 'cell-null' };
    }

    switch (type) {
      case 'bool':
        return { display: val ? '✓' : '✗', className: `cell-bool cell-bool-${val}` };

      case 'int2':
      case 'int4':
      case 'int8':
      case 'float4':
      case 'float8':
      case 'numeric':
      case 'money':
        return {
          display: formatNumber(val as number, type),
          className: 'cell-number'
        };

      case 'text':
      case 'varchar':
      case 'bpchar':
      case 'name':
        const text = val as string;
        const truncated = truncateText(text, $settingsStore.maxTextLength || 500);
        return {
          display: truncated,
          className: text.length > ($settingsStore.maxTextLength || 500) ? 'cell-text cell-truncated' : 'cell-text'
        };

      case 'json':
      case 'jsonb':
        return {
          display: JSON.stringify(val).slice(0, 100),
          className: 'cell-json'
        };

      case 'bytea':
        const hex = (val as { hex: string }).hex;
        return {
          display: `\\x${hex.slice(0, 20)}${hex.length > 20 ? '...' : ''}`,
          className: 'cell-bytea'
        };

      case 'timestamp':
      case 'timestamptz':
        return {
          display: formatTimestamp(val as string),
          className: 'cell-timestamp'
        };

      case 'date':
        return {
          display: formatDate(val as string),
          className: 'cell-date'
        };

      case 'time':
      case 'timetz':
        return {
          display: val as string,
          className: 'cell-time'
        };

      case 'interval':
        return {
          display: formatInterval((val as { iso: string }).iso),
          className: 'cell-interval'
        };

      case 'uuid':
        return { display: val as string, className: 'cell-uuid' };

      case 'inet':
      case 'cidr':
      case 'macaddr':
        return { display: val as string, className: 'cell-network' };

      case 'point':
        const point = val as { x: number; y: number };
        return { display: `(${point.x}, ${point.y})`, className: 'cell-geo' };

      default:
        // Arrays
        if (type.startsWith('_') || Array.isArray(val)) {
          const arr = val as Value[];
          return {
            display: `[${arr.length} items]`,
            className: 'cell-array'
          };
        }

        // Unknown
        if (typeof val === 'object' && 'text' in val) {
          return { display: (val as { text: string }).text, className: 'cell-unknown' };
        }

        return { display: String(val), className: 'cell-default' };
    }
  }

  function formatNumber(num: number, type: string): string {
    if (type === 'money') {
      return new Intl.NumberFormat($settingsStore.locale || 'en-US', {
        style: 'currency',
        currency: 'USD',
      }).format(num);
    }

    if (Number.isInteger(num)) {
      return num.toLocaleString($settingsStore.locale || 'en-US');
    }

    return num.toLocaleString($settingsStore.locale || 'en-US', {
      minimumFractionDigits: 0,
      maximumFractionDigits: 6,
    });
  }

  function formatTimestamp(ts: string): string {
    try {
      const date = new Date(ts);
      return date.toLocaleString($settingsStore.locale || 'en-US');
    } catch {
      return ts;
    }
  }

  function formatDate(d: string): string {
    try {
      const date = new Date(d);
      return date.toLocaleDateString($settingsStore.locale || 'en-US');
    } catch {
      return d;
    }
  }

  function formatInterval(iso: string): string {
    // Parse ISO 8601 interval and format human-readable
    return iso;
  }

  function truncateText(text: string, maxLength: number): string {
    if (text.length <= maxLength) return text;
    return text.slice(0, maxLength) + '…';
  }

  const formatted = $derived(formatValue(value, typeName));
  const isExpandable = $derived(
    typeName === 'json' || typeName === 'jsonb' ||
    typeName.startsWith('_') || Array.isArray(value) ||
    (typeof value === 'string' && value.length > ($settingsStore.maxTextLength || 500))
  );
</script>

<div
  class="cell {formatted.className}"
  class:selected={isSelected}
  class:expandable={isExpandable}
  ondblclick={isExpandable ? onExpand : undefined}
  title={value === null ? 'NULL' : undefined}
>
  {formatted.display}
</div>

<style>
  .cell {
    padding: 0 8px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    height: 100%;
    display: flex;
    align-items: center;
  }

  .cell.selected {
    background: var(--selected-color);
  }

  .cell.expandable {
    cursor: pointer;
  }

  .cell-null {
    color: var(--text-muted);
    font-style: italic;
  }

  .cell-bool {
    justify-content: center;
  }

  .cell-bool-true {
    color: #16a34a;
  }

  .cell-bool-false {
    color: #dc2626;
  }

  .cell-number {
    justify-content: flex-end;
    font-family: var(--font-mono);
  }

  .cell-text {
    font-family: var(--font-sans);
  }

  .cell-truncated {
    color: var(--text-muted);
  }

  .cell-json {
    font-family: var(--font-mono);
    color: var(--primary-color);
  }

  .cell-bytea {
    font-family: var(--font-mono);
    color: #9333ea;
  }

  .cell-timestamp,
  .cell-date,
  .cell-time {
    font-family: var(--font-mono);
  }

  .cell-uuid {
    font-family: var(--font-mono);
    font-size: 0.75rem;
  }

  .cell-array {
    color: var(--primary-color);
    font-style: italic;
  }
</style>
```

### 14.4 Results Grid Component

```svelte
<!-- src/lib/components/grid/ResultsGrid.svelte -->
<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { gridState, type GridColumn, type CellSelection } from '$lib/stores/gridState.svelte';
	import { calculateVirtualScroll, calculateHorizontalVirtual } from './virtualScroll';
	import CellRenderer from './CellRenderer.svelte';
	import ColumnHeader from './ColumnHeader.svelte';
	import ContextMenu from '$lib/components/common/ContextMenu.svelte';
	import { ChevronUp, ChevronDown } from 'lucide-svelte';

	interface Props {
		onCellDoubleClick?: (rowIndex: number, colIndex: number) => void;
	}

	let { onCellDoubleClick }: Props = $props();

	let containerRef: HTMLDivElement;
	let containerWidth = $state(0);
	let containerHeight = $state(0);

	let contextMenu: { x: number; y: number; rowIndex: number; colIndex: number } | null =
		$state(null);

	// Virtual scroll state
	const verticalState = $derived(
		calculateVirtualScroll($gridState.scrollTop, $gridState.rows.length, {
			rowHeight: $gridState.rowHeight,
			overscan: 5,
			containerHeight
		})
	);

	const visibleColumns = $derived($gridState.columns.filter((c) => !c.hidden));

	const horizontalState = $derived(
		calculateHorizontalVirtual($gridState.scrollLeft, visibleColumns, containerWidth, 2)
	);

	const visibleRows = $derived(
		$gridState.rows.slice(verticalState.visibleStartIndex, verticalState.visibleEndIndex)
	);

	const visibleColumnSlice = $derived(
		visibleColumns.slice(horizontalState.visibleStartIndex, horizontalState.visibleEndIndex)
	);

	// Row number column width
	const rowNumWidth = $derived(Math.max(50, String($gridState.totalRows).length * 10 + 16));

	onMount(() => {
		const resizeObserver = new ResizeObserver((entries) => {
			for (const entry of entries) {
				containerWidth = entry.contentRect.width;
				containerHeight = entry.contentRect.height;
			}
		});

		resizeObserver.observe(containerRef);

		return () => resizeObserver.disconnect();
	});

	function handleScroll(e: Event) {
		const target = e.target as HTMLDivElement;
		gridState.scrollTop = target.scrollTop;
		gridState.scrollLeft = target.scrollLeft;
	}

	function handleCellClick(rowIndex: number, colIndex: number, e: MouseEvent) {
		const actualRowIndex = verticalState.visibleStartIndex + rowIndex;
		const actualColIndex = horizontalState.visibleStartIndex + colIndex;

		if (e.shiftKey && $gridState.selectedCell) {
			gridState.extendSelection(actualRowIndex, actualColIndex);
		} else if (e.ctrlKey || e.metaKey) {
			// Toggle selection (for row selection)
			const newSet = new Set($gridState.selectedRows);
			if (newSet.has(actualRowIndex)) {
				newSet.delete(actualRowIndex);
			} else {
				newSet.add(actualRowIndex);
			}
			gridState.selectedRows = newSet;
		} else {
			gridState.selectCell(actualRowIndex, actualColIndex);
		}
	}

	function handleContextMenu(e: MouseEvent, rowIndex: number, colIndex: number) {
		e.preventDefault();
		const actualRowIndex = verticalState.visibleStartIndex + rowIndex;
		const actualColIndex = horizontalState.visibleStartIndex + colIndex;
		contextMenu = {
			x: e.clientX,
			y: e.clientY,
			rowIndex: actualRowIndex,
			colIndex: actualColIndex
		};
	}

	function handleKeydown(e: KeyboardEvent) {
		if (!$gridState.selectedCell) return;

		const { rowIndex, colIndex } = $gridState.selectedCell;

		switch (e.key) {
			case 'ArrowUp':
				e.preventDefault();
				if (e.shiftKey) {
					gridState.extendSelection(Math.max(0, rowIndex - 1), colIndex);
				} else {
					gridState.selectCell(Math.max(0, rowIndex - 1), colIndex);
				}
				break;

			case 'ArrowDown':
				e.preventDefault();
				if (e.shiftKey) {
					gridState.extendSelection(Math.min($gridState.rows.length - 1, rowIndex + 1), colIndex);
				} else {
					gridState.selectCell(Math.min($gridState.rows.length - 1, rowIndex + 1), colIndex);
				}
				break;

			case 'ArrowLeft':
				e.preventDefault();
				if (e.shiftKey) {
					gridState.extendSelection(rowIndex, Math.max(0, colIndex - 1));
				} else {
					gridState.selectCell(rowIndex, Math.max(0, colIndex - 1));
				}
				break;

			case 'ArrowRight':
				e.preventDefault();
				if (e.shiftKey) {
					gridState.extendSelection(rowIndex, Math.min(visibleColumns.length - 1, colIndex + 1));
				} else {
					gridState.selectCell(rowIndex, Math.min(visibleColumns.length - 1, colIndex + 1));
				}
				break;

			case 'a':
				if (e.ctrlKey || e.metaKey) {
					e.preventDefault();
					gridState.selectAll();
				}
				break;

			case 'c':
				if (e.ctrlKey || e.metaKey) {
					e.preventDefault();
					copySelection();
				}
				break;
		}
	}

	async function copySelection() {
		const data = gridState.getSelectedData();
		const tsv = data.map((row) => row.map((v) => formatForCopy(v)).join('\t')).join('\n');

		await navigator.clipboard.writeText(tsv);
	}

	function formatForCopy(value: any): string {
		if (value === null) return '';
		if (typeof value === 'object') return JSON.stringify(value);
		return String(value);
	}

	function getContextMenuItems() {
		return [
			{ label: 'Copy', shortcut: '⌘C', action: copySelection },
			{ label: 'Copy as INSERT', action: () => copyAsInsert() },
			{ label: 'Copy as JSON', action: () => copyAsJson() },
			{ type: 'separator' as const },
			{ label: 'Copy column name', action: () => copyColumnName() },
			{ type: 'separator' as const },
			{ label: 'Filter to this value', action: () => filterToValue() },
			{ label: 'Filter to NOT this value', action: () => filterNotValue() }
		];
	}

	function copyAsInsert() {
		// Implementation for INSERT statement generation
	}

	function copyAsJson() {
		const data = gridState.getSelectedData();
		const columns = gridState.getSelectedColumns();

		const json = data.map((row) => {
			const obj: Record<string, any> = {};
			row.forEach((val, i) => {
				obj[columns[i].name] = val;
			});
			return obj;
		});

		navigator.clipboard.writeText(JSON.stringify(json, null, 2));
	}

	function copyColumnName() {
		if (contextMenu) {
			const col = visibleColumns[contextMenu.colIndex];
			navigator.clipboard.writeText(col.name);
		}
		contextMenu = null;
	}

	function filterToValue() {
		// Emit filter event
		contextMenu = null;
	}

	function filterNotValue() {
		// Emit filter event
		contextMenu = null;
	}
</script>

<div
	bind:this={containerRef}
	class="results-grid"
	tabindex="0"
	onkeydown={handleKeydown}
	role="grid"
>
	{#if $gridState.rows.length === 0}
		<div class="empty-state">
			{$gridState.isLoading ? 'Loading results...' : 'No results to display'}
		</div>
	{:else}
		<div class="grid-scroll" onscroll={handleScroll}>
			<!-- Header -->
			<div class="grid-header" style:padding-left="{rowNumWidth}px">
				<div style:width="{horizontalState.offsetX}px"></div>
				{#each visibleColumnSlice as column, i}
					<ColumnHeader
						{column}
						index={horizontalState.visibleStartIndex + i}
						onResize={(width) =>
							gridState.resizeColumn(horizontalState.visibleStartIndex + i, width)}
						onSort={(multi) => gridState.sortByColumn(horizontalState.visibleStartIndex + i, multi)}
					/>
				{/each}
			</div>

			<!-- Body -->
			<div class="grid-body" style:height="{verticalState.totalHeight}px">
				<div style:height="{verticalState.offsetY}px"></div>

				{#each visibleRows as row, rowIdx}
					<div
						class="grid-row"
						class:selected={$gridState.selectedRows.has(verticalState.visibleStartIndex + rowIdx)}
						style:height="{$gridState.rowHeight}px"
					>
						<!-- Row number -->
						<div class="row-number" style:width="{rowNumWidth}px">
							{verticalState.visibleStartIndex + rowIdx + 1}
						</div>

						<!-- Horizontal offset -->
						<div style:width="{horizontalState.offsetX}px"></div>

						<!-- Visible cells -->
						{#each visibleColumnSlice as column, colIdx}
							{@const actualRowIdx = verticalState.visibleStartIndex + rowIdx}
							{@const actualColIdx = horizontalState.visibleStartIndex + colIdx}
							{@const value = $gridState.rows[actualRowIdx]?.[actualColIdx]}

							<div
								class="grid-cell"
								class:selected={gridState.isCellSelected(actualRowIdx, actualColIdx)}
								style:width="{column.width}px"
								onclick={(e) => handleCellClick(rowIdx, colIdx, e)}
								oncontextmenu={(e) => handleContextMenu(e, rowIdx, colIdx)}
								ondblclick={() => onCellDoubleClick?.(actualRowIdx, actualColIdx)}
								role="gridcell"
							>
								<CellRenderer
									{value}
									typeName={column.type}
									isSelected={gridState.isCellSelected(actualRowIdx, actualColIdx)}
								/>
							</div>
						{/each}
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>

{#if contextMenu}
	<ContextMenu
		x={contextMenu.x}
		y={contextMenu.y}
		onClose={() => (contextMenu = null)}
		items={getContextMenuItems()}
	/>
{/if}

<style>
	.results-grid {
		width: 100%;
		height: 100%;
		overflow: hidden;
		background: var(--background-color);
		font-size: 0.8125rem;
		outline: none;
	}

	.results-grid:focus {
		outline: 2px solid var(--primary-color);
		outline-offset: -2px;
	}

	.empty-state {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--text-muted);
	}

	.grid-scroll {
		width: 100%;
		height: 100%;
		overflow: auto;
	}

	.grid-header {
		display: flex;
		position: sticky;
		top: 0;
		z-index: 10;
		background: var(--surface-color);
		border-bottom: 1px solid var(--border-color);
	}

	.grid-body {
		position: relative;
	}

	.grid-row {
		display: flex;
		border-bottom: 1px solid var(--border-color);
	}

	.grid-row:hover {
		background: var(--hover-color);
	}

	.grid-row.selected {
		background: var(--selected-color);
	}

	.row-number {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		padding-right: 8px;
		background: var(--surface-secondary);
		color: var(--text-muted);
		font-size: 0.75rem;
		border-right: 1px solid var(--border-color);
		position: sticky;
		left: 0;
		z-index: 5;
	}

	.grid-cell {
		border-right: 1px solid var(--border-color);
		overflow: hidden;
	}

	.grid-cell.selected {
		background: var(--selected-color);
		outline: 2px solid var(--primary-color);
		outline-offset: -2px;
	}
</style>
```

### 14.5 Column Header Component

```svelte
<!-- src/lib/components/grid/ColumnHeader.svelte -->
<script lang="ts">
	import { ChevronUp, ChevronDown, EyeOff, Maximize2 } from 'lucide-svelte';
	import type { GridColumn } from '$lib/stores/gridState.svelte';
	import ContextMenu from '$lib/components/common/ContextMenu.svelte';

	interface Props {
		column: GridColumn;
		index: number;
		onResize: (width: number) => void;
		onSort: (multi: boolean) => void;
	}

	let { column, index, onResize, onSort }: Props = $props();

	let isResizing = $state(false);
	let startX = $state(0);
	let startWidth = $state(0);
	let contextMenu: { x: number; y: number } | null = $state(null);

	function handleMouseDown(e: MouseEvent) {
		e.preventDefault();
		isResizing = true;
		startX = e.clientX;
		startWidth = column.width;

		window.addEventListener('mousemove', handleMouseMove);
		window.addEventListener('mouseup', handleMouseUp);
	}

	function handleMouseMove(e: MouseEvent) {
		if (!isResizing) return;
		const diff = e.clientX - startX;
		onResize(startWidth + diff);
	}

	function handleMouseUp() {
		isResizing = false;
		window.removeEventListener('mousemove', handleMouseMove);
		window.removeEventListener('mouseup', handleMouseUp);
	}

	function handleClick(e: MouseEvent) {
		if (isResizing) return;
		onSort(e.shiftKey);
	}

	function handleContextMenu(e: MouseEvent) {
		e.preventDefault();
		contextMenu = { x: e.clientX, y: e.clientY };
	}
</script>

<div
	class="column-header"
	style:width="{column.width}px"
	onclick={handleClick}
	oncontextmenu={handleContextMenu}
	role="columnheader"
>
	<span class="column-name" title="{column.name} ({column.type})">
		{column.name}
	</span>

	{#if column.sortDirection}
		<span class="sort-indicator">
			{#if column.sortDirection === 'asc'}
				<ChevronUp size={14} />
			{:else}
				<ChevronDown size={14} />
			{/if}
			{#if column.sortOrder !== null && column.sortOrder > 0}
				<span class="sort-order">{column.sortOrder + 1}</span>
			{/if}
		</span>
	{/if}

	<div class="resize-handle" onmousedown={handleMouseDown} role="separator"></div>
</div>

{#if contextMenu}
	<ContextMenu
		x={contextMenu.x}
		y={contextMenu.y}
		onClose={() => (contextMenu = null)}
		items={[
			{
				label: 'Sort Ascending',
				action: () => {
					column.sortDirection !== 'asc' && onSort(false);
					contextMenu = null;
				}
			},
			{
				label: 'Sort Descending',
				action: () => {
					column.sortDirection !== 'desc' && onSort(false);
					contextMenu = null;
				}
			},
			{ type: 'separator' },
			{
				label: 'Hide Column',
				icon: EyeOff,
				action: () => {
					/* emit hide */ contextMenu = null;
				}
			},
			{
				label: 'Size to Fit',
				icon: Maximize2,
				action: () => {
					/* emit autosize */ contextMenu = null;
				}
			},
			{
				label: 'Size All to Fit',
				action: () => {
					/* emit autosize all */ contextMenu = null;
				}
			}
		]}
	/>
{/if}

<style>
	.column-header {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 0 8px;
		height: 32px;
		background: var(--surface-color);
		border-right: 1px solid var(--border-color);
		cursor: pointer;
		user-select: none;
		position: relative;
	}

	.column-header:hover {
		background: var(--hover-color);
	}

	.column-name {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 500;
		font-size: 0.8125rem;
	}

	.sort-indicator {
		display: flex;
		align-items: center;
		color: var(--primary-color);
	}

	.sort-order {
		font-size: 0.625rem;
		margin-left: 2px;
	}

	.resize-handle {
		position: absolute;
		right: 0;
		top: 0;
		bottom: 0;
		width: 4px;
		cursor: col-resize;
		background: transparent;
	}

	.resize-handle:hover {
		background: var(--primary-color);
	}
</style>
```

### 14.6 Results Toolbar Component

```svelte
<!-- src/lib/components/grid/ResultsToolbar.svelte -->
<script lang="ts">
	import {
		Download,
		Grid,
		List,
		Braces,
		ChevronLeft,
		ChevronRight,
		ChevronsLeft,
		ChevronsRight
	} from 'lucide-svelte';
	import { gridState, type DisplayMode } from '$lib/stores/gridState.svelte';

	interface Props {
		onExport: () => void;
		pageSize?: number;
		currentPage?: number;
		totalPages?: number;
		onPageChange?: (page: number) => void;
	}

	let {
		onExport,
		pageSize = 1000,
		currentPage = 1,
		totalPages = 1,
		onPageChange
	}: Props = $props();

	function setDisplayMode(mode: DisplayMode) {
		gridState.displayMode = mode;
	}

	function formatRowRange(): string {
		const start = (currentPage - 1) * pageSize + 1;
		const end = Math.min(currentPage * pageSize, $gridState.totalRows);
		return `${start.toLocaleString()}-${end.toLocaleString()} of ${$gridState.totalRows.toLocaleString()}`;
	}
</script>

<div class="results-toolbar">
	<div class="toolbar-group">
		<span class="results-info">
			{formatRowRange()} rows
		</span>

		{#if $gridState.queryId}
			<span class="timing">
				• {$gridState.elapsedMs?.toLocaleString() ?? '–'}ms
			</span>
		{/if}
	</div>

	<div class="toolbar-spacer"></div>

	<div class="toolbar-group">
		<!-- Display mode toggle -->
		<div class="mode-toggle">
			<button
				class="mode-btn"
				class:active={$gridState.displayMode === 'grid'}
				onclick={() => setDisplayMode('grid')}
				title="Grid view"
			>
				<Grid size={16} />
			</button>
			<button
				class="mode-btn"
				class:active={$gridState.displayMode === 'transposed'}
				onclick={() => setDisplayMode('transposed')}
				title="Transposed view"
			>
				<List size={16} />
			</button>
			<button
				class="mode-btn"
				class:active={$gridState.displayMode === 'json'}
				onclick={() => setDisplayMode('json')}
				title="JSON view"
			>
				<Braces size={16} />
			</button>
		</div>

		<!-- Export -->
		<button class="toolbar-btn" onclick={onExport} title="Export results">
			<Download size={16} />
		</button>
	</div>

	{#if totalPages > 1}
		<div class="toolbar-separator"></div>

		<div class="pagination">
			<button
				class="page-btn"
				onclick={() => onPageChange?.(1)}
				disabled={currentPage === 1}
				title="First page"
			>
				<ChevronsLeft size={16} />
			</button>
			<button
				class="page-btn"
				onclick={() => onPageChange?.(currentPage - 1)}
				disabled={currentPage === 1}
				title="Previous page"
			>
				<ChevronLeft size={16} />
			</button>

			<span class="page-info">
				Page {currentPage} of {totalPages}
			</span>

			<button
				class="page-btn"
				onclick={() => onPageChange?.(currentPage + 1)}
				disabled={currentPage === totalPages}
				title="Next page"
			>
				<ChevronRight size={16} />
			</button>
			<button
				class="page-btn"
				onclick={() => onPageChange?.(totalPages)}
				disabled={currentPage === totalPages}
				title="Last page"
			>
				<ChevronsRight size={16} />
			</button>
		</div>
	{/if}
</div>

<style>
	.results-toolbar {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.375rem 0.5rem;
		background: var(--surface-color);
		border-top: 1px solid var(--border-color);
		font-size: 0.8125rem;
	}

	.toolbar-group {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.toolbar-spacer {
		flex: 1;
	}

	.toolbar-separator {
		width: 1px;
		height: 20px;
		background: var(--border-color);
	}

	.results-info {
		color: var(--text-muted);
	}

	.timing {
		color: var(--text-muted);
	}

	.mode-toggle {
		display: flex;
		border: 1px solid var(--border-color);
		border-radius: 0.375rem;
		overflow: hidden;
	}

	.mode-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 24px;
		border: none;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
		transition: all 0.15s;
	}

	.mode-btn:hover {
		background: var(--hover-color);
		color: var(--text-color);
	}

	.mode-btn.active {
		background: var(--primary-color);
		color: white;
	}

	.mode-btn + .mode-btn {
		border-left: 1px solid var(--border-color);
	}

	.toolbar-btn {
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

	.toolbar-btn:hover {
		background: var(--hover-color);
		color: var(--text-color);
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
		width: 24px;
		height: 24px;
		border: none;
		border-radius: 0.25rem;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
		transition: all 0.15s;
	}

	.page-btn:hover:not(:disabled) {
		background: var(--hover-color);
		color: var(--text-color);
	}

	.page-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.page-info {
		color: var(--text-muted);
		min-width: 100px;
		text-align: center;
	}
</style>
```

## Acceptance Criteria

1. **Virtual Scrolling**
   - Render only visible rows + overscan buffer
   - Handle 10M+ rows without performance degradation
   - Smooth scrolling in both directions
   - Proper row height calculations

2. **Type Rendering**
   - NULL displayed as styled "NULL" text
   - Booleans as checkmarks/crosses
   - Numbers right-aligned with formatting
   - Timestamps in locale format
   - JSON with syntax highlighting preview
   - Arrays with item count
   - UUIDs in monospace font

3. **Column Operations**
   - Resize columns by dragging header border
   - Click header to sort (ascending/descending/none)
   - Shift+click for multi-column sort
   - Context menu for hide/autosize

4. **Cell Selection**
   - Click to select single cell
   - Shift+click for range selection
   - Ctrl/Cmd+A to select all
   - Arrow key navigation
   - Ctrl/Cmd+C to copy

5. **Display Modes**
   - Grid mode (default spreadsheet)
   - Transposed mode (single row as key-value)
   - JSON mode (raw JSON output)

6. **Pagination**
   - First/Previous/Next/Last buttons
   - Page number display
   - Row range display

## MCP Testing Instructions

### Using Tauri MCP

```typescript
// Execute query and verify grid renders
await mcp.ipc_execute_command({
	command: 'execute_query',
	args: {
		connId: connectionId,
		sql: 'SELECT * FROM generate_series(1, 1000000) AS n'
	}
});

// Verify virtual scrolling
const initialSnapshot = await mcp.webview_dom_snapshot({ type: 'accessibility' });
const rowCountBefore = (initialSnapshot.match(/gridcell/g) || []).length;

// Scroll down
await mcp.webview_interact({
	action: 'scroll',
	selector: '.results-grid',
	scrollY: 10000
});

// Verify new rows rendered
const afterScroll = await mcp.webview_dom_snapshot({ type: 'accessibility' });
// Row numbers should be different

// Test column resize
await mcp.webview_interact({
	action: 'click',
	selector: '.resize-handle'
});

// Test cell selection
await mcp.webview_click({
	selector: '.grid-cell:first-child',
	element: 'First cell'
});

// Copy and verify
await mcp.browser_press_key({ key: 'c', modifiers: ['Meta'] });
```

## Dependencies

- TanStack Virtual (optional, for advanced virtualization)
- Feature 11: Query Execution (result data)
- Feature 06: Settings (display preferences)
