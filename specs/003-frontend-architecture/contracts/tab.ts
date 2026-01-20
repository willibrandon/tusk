/**
 * Tab-related type definitions for Tusk frontend architecture.
 *
 * @module contracts/tab
 */

// =============================================================================
// Tab Types
// =============================================================================

/**
 * Types of tabs that can be opened in the application.
 */
export type TabType = 'query' | 'table' | 'view' | 'function' | 'schema';

/**
 * Represents an open workspace tab in the application.
 */
export interface Tab {
	/** Unique identifier (UUID v4) */
	id: string;

	/** Type of tab content */
	type: TabType;

	/** Display title (may be truncated in UI) */
	title: string;

	/** Associated connection ID, null if no connection */
	connectionId: string | null;

	/** Whether the tab has unsaved changes */
	isModified: boolean;

	/** Tab-specific content data */
	content: TabContent;

	/** Unix timestamp of tab creation */
	createdAt: number;
}

// =============================================================================
// Tab Content Types
// =============================================================================

/**
 * Union type for all tab content types.
 */
export type TabContent =
	| QueryTabContent
	| TableTabContent
	| ViewTabContent
	| FunctionTabContent
	| SchemaTabContent;

/**
 * Content for query editor tabs.
 */
export interface QueryTabContent {
	type: 'query';

	/** SQL query text */
	sql: string;

	/** Current cursor position */
	cursorPosition: CursorPosition;

	/** Selected text range, null if no selection */
	selectionRange: SelectionRange | null;

	/** Last query execution results */
	results: QueryResult | null;
}

/**
 * Content for table data viewer tabs.
 */
export interface TableTabContent {
	type: 'table';

	/** Schema name */
	schema: string;

	/** Table name */
	table: string;

	/** Active column filters */
	filters: ColumnFilter[];

	/** Current sort column */
	sortColumn: string | null;

	/** Sort direction */
	sortDirection: 'asc' | 'desc';

	/** Pagination offset */
	offset: number;

	/** Page size */
	limit: number;
}

/**
 * Content for view data viewer tabs.
 */
export interface ViewTabContent {
	type: 'view';

	/** Schema name */
	schema: string;

	/** View name */
	view: string;

	/** Active column filters */
	filters: ColumnFilter[];

	/** Current sort column */
	sortColumn: string | null;

	/** Sort direction */
	sortDirection: 'asc' | 'desc';
}

/**
 * Content for function/procedure editor tabs.
 */
export interface FunctionTabContent {
	type: 'function';

	/** Schema name */
	schema: string;

	/** Function name */
	name: string;

	/** Function source code */
	source: string;

	/** Current cursor position */
	cursorPosition: CursorPosition;
}

/**
 * Content for schema browser tabs.
 */
export interface SchemaTabContent {
	type: 'schema';

	/** Schema name */
	schema: string;

	/** Expanded tree nodes */
	expandedNodes: string[];
}

// =============================================================================
// Editor Types
// =============================================================================

/**
 * Cursor position in an editor.
 */
export interface CursorPosition {
	/** 1-based line number */
	line: number;

	/** 1-based column number */
	column: number;
}

/**
 * Text selection range.
 */
export interface SelectionRange {
	/** Selection start position */
	start: CursorPosition;

	/** Selection end position */
	end: CursorPosition;
}

// =============================================================================
// Query Result Types
// =============================================================================

/**
 * Result of a query execution.
 */
export interface QueryResult {
	/** Column definitions */
	columns: ColumnDefinition[];

	/** Result rows */
	rows: unknown[][];

	/** Total number of rows returned */
	rowCount: number;

	/** Query execution time in milliseconds */
	executionTimeMs: number;

	/** Whether there are more rows available */
	hasMore: boolean;
}

/**
 * Column definition from query results.
 */
export interface ColumnDefinition {
	/** Column name */
	name: string;

	/** PostgreSQL data type */
	dataType: string;

	/** Whether the column allows null values */
	nullable: boolean;
}

/**
 * Filter applied to a table column.
 */
export interface ColumnFilter {
	/** Column name */
	column: string;

	/** Filter operator */
	operator: FilterOperator;

	/** Filter value */
	value: string;
}

/**
 * Filter operators for column filtering.
 */
export type FilterOperator =
	| 'eq' // Equal
	| 'neq' // Not equal
	| 'gt' // Greater than
	| 'gte' // Greater than or equal
	| 'lt' // Less than
	| 'lte' // Less than or equal
	| 'like' // LIKE pattern
	| 'ilike' // Case-insensitive LIKE
	| 'is_null' // IS NULL
	| 'is_not_null'; // IS NOT NULL

// =============================================================================
// Tab Store Interface
// =============================================================================

/**
 * Result of attempting to close a tab.
 */
export type CloseResult = 'closed' | 'cancelled' | 'saved';

/**
 * Options for creating a new tab.
 */
export interface CreateTabOptions {
	/** Tab title (auto-generated if not provided) */
	title?: string;

	/** Associated connection ID */
	connectionId?: string | null;

	/** Initial content */
	content?: Partial<TabContent>;
}

/**
 * Tab store interface for state management.
 */
export interface TabStoreInterface {
	/** All open tabs */
	readonly tabs: Tab[];

	/** Currently active tab ID */
	readonly activeTabId: string | null;

	/** Currently active tab */
	readonly activeTab: Tab | null;

	/** Whether any tab has unsaved changes */
	readonly hasUnsavedChanges: boolean;

	/** Create a new tab */
	createTab(type: TabType, options?: CreateTabOptions): Tab;

	/** Close a tab (may prompt for unsaved changes) */
	closeTab(id: string): Promise<CloseResult>;

	/** Set the active tab */
	setActiveTab(id: string): void;

	/** Update tab properties */
	updateTab(id: string, updates: Partial<Tab>): void;

	/** Reorder tabs */
	reorderTabs(newOrder: Tab[]): void;

	/** Mark a tab as modified or unmodified */
	markModified(id: string, modified: boolean): void;

	/** Get tab by ID */
	getTab(id: string): Tab | undefined;
}

// =============================================================================
// Type Guards
// =============================================================================

/**
 * Check if a value is a valid TabType.
 */
export function isTabType(value: unknown): value is TabType {
	return (
		typeof value === 'string' && ['query', 'table', 'view', 'function', 'schema'].includes(value)
	);
}

/**
 * Check if content is QueryTabContent.
 */
export function isQueryContent(content: TabContent): content is QueryTabContent {
	return content.type === 'query';
}

/**
 * Check if content is TableTabContent.
 */
export function isTableContent(content: TabContent): content is TableTabContent {
	return content.type === 'table';
}
