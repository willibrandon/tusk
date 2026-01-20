# Data Model: Frontend Architecture

**Feature**: 003-frontend-architecture
**Date**: 2026-01-19

## Overview

This document defines the data entities, their relationships, and state management patterns for the Tusk frontend architecture. All entities are managed client-side with localStorage persistence.

---

## Entities

### Tab

Represents an open workspace in the application. Each tab has its own content and state.

| Field          | Type             | Required | Description                            |
| -------------- | ---------------- | -------- | -------------------------------------- |
| `id`           | `string`         | Yes      | Unique identifier (UUID v4)            |
| `type`         | `TabType`        | Yes      | Type of tab content                    |
| `title`        | `string`         | Yes      | Display title (truncated if >30 chars) |
| `connectionId` | `string \| null` | No       | Associated connection ID               |
| `isModified`   | `boolean`        | Yes      | Has unsaved changes                    |
| `content`      | `TabContent`     | Yes      | Tab-specific content data              |
| `createdAt`    | `number`         | Yes      | Unix timestamp of creation             |

**TabType Values**:

- `query` — SQL query editor
- `table` — Table data viewer
- `view` — View data viewer
- `function` — Function/procedure editor
- `schema` — Schema browser

**Validation Rules**:

- `id` must be a valid UUID v4
- `title` must be non-empty, max 100 characters
- `type` must be a valid TabType value

**State Transitions**:

```
[Created] --> [Active] <--> [Inactive]
    |            |
    v            v
[Closed]     [Closed]
              (with unsaved changes dialog if isModified)
```

---

### TabContent

Union type for tab-specific content. Each tab type has different content structure.

**QueryTabContent**:
| Field | Type | Description |
|-------|------|-------------|
| `sql` | `string` | SQL query text |
| `cursorPosition` | `CursorPosition` | Current cursor location |
| `selectionRange` | `SelectionRange \| null` | Selected text range |
| `results` | `QueryResult \| null` | Last query results |

**TableTabContent**:
| Field | Type | Description |
|-------|------|-------------|
| `schema` | `string` | Schema name |
| `table` | `string` | Table name |
| `filters` | `ColumnFilter[]` | Active column filters |
| `sortColumn` | `string \| null` | Current sort column |
| `sortDirection` | `'asc' \| 'desc'` | Sort direction |

**Note**: For this feature (003), only `QueryTabContent` is fully implemented. Other content types are typed but functionality deferred to future features.

---

### CursorPosition

Represents cursor location in an editor.

| Field    | Type     | Description           |
| -------- | -------- | --------------------- |
| `line`   | `number` | 1-based line number   |
| `column` | `number` | 1-based column number |

---

### SelectionRange

Represents a text selection range.

| Field   | Type             | Description     |
| ------- | ---------------- | --------------- |
| `start` | `CursorPosition` | Selection start |
| `end`   | `CursorPosition` | Selection end   |

---

### Connection

Represents a database connection configuration. Connection management is a future feature; this entity is defined for status display and tab association.

| Field      | Type             | Required | Description                           |
| ---------- | ---------------- | -------- | ------------------------------------- |
| `id`       | `string`         | Yes      | Unique identifier (UUID v4)           |
| `name`     | `string`         | Yes      | User-defined connection name          |
| `host`     | `string`         | Yes      | Database server hostname              |
| `port`     | `number`         | Yes      | Database server port                  |
| `database` | `string`         | Yes      | Database name                         |
| `username` | `string`         | Yes      | Database username                     |
| `sslMode`  | `SslMode`        | Yes      | SSL/TLS configuration                 |
| `color`    | `string \| null` | No       | Color for visual identification (hex) |
| `groupId`  | `string \| null` | No       | Parent connection group               |

**SslMode Values**:

- `disable` — No SSL
- `prefer` — Use SSL if available (default)
- `require` — Require SSL
- `verify-ca` — Require SSL with CA verification
- `verify-full` — Require SSL with full verification

**Note**: Connection credentials (password) are NOT stored in this entity. Passwords are stored in OS keychain via Rust backend.

---

### ConnectionGroup

Organizes connections into hierarchical folders.

| Field        | Type             | Required | Description                   |
| ------------ | ---------------- | -------- | ----------------------------- |
| `id`         | `string`         | Yes      | Unique identifier (UUID v4)   |
| `name`       | `string`         | Yes      | Group display name            |
| `parentId`   | `string \| null` | No       | Parent group ID (for nesting) |
| `sortOrder`  | `number`         | Yes      | Position within parent        |
| `color`      | `string \| null` | No       | Group color (hex)             |
| `isExpanded` | `boolean`        | Yes      | Tree expansion state          |

---

### ConnectionStatus

Represents the current state of a connection.

| Field          | Type              | Description                       |
| -------------- | ----------------- | --------------------------------- |
| `connectionId` | `string`          | Associated connection ID          |
| `state`        | `ConnectionState` | Current connection state          |
| `error`        | `string \| null`  | Error message if state is `error` |
| `connectedAt`  | `number \| null`  | Unix timestamp when connected     |

**ConnectionState Values**:

- `disconnected` — No connection attempt
- `connecting` — Connection in progress
- `connected` — Successfully connected
- `error` — Connection failed

---

### ThemePreference

User's theme configuration.

| Field        | Type                            | Description         |
| ------------ | ------------------------------- | ------------------- |
| `mode`       | `'light' \| 'dark'`             | Resolved theme mode |
| `preference` | `'light' \| 'dark' \| 'system'` | User preference     |

---

### UIState

Persistent UI layout preferences.

| Field                | Type      | Default | Description                    |
| -------------------- | --------- | ------- | ------------------------------ |
| `sidebarWidth`       | `number`  | `264`   | Sidebar width in pixels        |
| `sidebarCollapsed`   | `boolean` | `false` | Sidebar visibility state       |
| `resultsPanelHeight` | `number`  | `300`   | Results panel height in pixels |

**Constraints**:

- `sidebarWidth`: min 200, max 500
- `resultsPanelHeight`: min 100, max 800

---

## Relationships

```
┌─────────────────┐     ┌─────────────────┐
│ ConnectionGroup │◄────│   Connection    │
│                 │     │                 │
│ - id            │     │ - id            │
│ - name          │     │ - name          │
│ - parentId ─────┼──┐  │ - groupId ──────┼───┐
│                 │  │  │ - color         │   │
└─────────────────┘  │  └────────┬────────┘   │
        ▲            │           │            │
        │            │           │            │
        └────────────┘           │            │
        (self-reference)         │            │
                                 ▼            │
                    ┌─────────────────┐       │
                    │ ConnectionStatus│       │
                    │                 │       │
                    │ - connectionId  │       │
                    │ - state         │       │
                    │ - error         │       │
                    └─────────────────┘       │
                                              │
┌─────────────────┐                           │
│      Tab        │◄──────────────────────────┘
│                 │
│ - id            │
│ - type          │
│ - title         │
│ - connectionId ─┼─── references Connection.id
│ - isModified    │
│ - content       │
└─────────────────┘
```

---

## Store Structure

### TabStore

Manages all open tabs and active tab selection.

```typescript
interface TabStore {
	// State (reactive getters)
	readonly tabs: Tab[];
	readonly activeTabId: string | null;
	readonly activeTab: Tab | null;
	readonly hasUnsavedChanges: boolean;

	// Actions
	createTab(type: TabType, options?: Partial<Tab>): Tab;
	closeTab(id: string): Promise<CloseResult>;
	setActiveTab(id: string): void;
	updateTab(id: string, updates: Partial<Tab>): void;
	reorderTabs(newOrder: Tab[]): void;
	markModified(id: string, modified: boolean): void;
}

type CloseResult = 'closed' | 'cancelled' | 'saved';
```

---

### ConnectionStore

Manages connection configurations and status. Connections are loaded from backend.

```typescript
interface ConnectionStore {
	// State
	readonly connections: Connection[];
	readonly groups: ConnectionGroup[];
	readonly activeConnectionId: string | null;
	readonly activeConnection: Connection | null;
	readonly connectionStatuses: Map<string, ConnectionStatus>;

	// Actions
	setConnections(connections: Connection[]): void;
	setGroups(groups: ConnectionGroup[]): void;
	setActiveConnection(id: string | null): void;
	updateStatus(connectionId: string, status: Partial<ConnectionStatus>): void;
}
```

---

### UIStore

Manages persistent UI preferences.

```typescript
interface UIStore {
	// State
	readonly sidebarWidth: number;
	readonly sidebarCollapsed: boolean;
	readonly resultsPanelHeight: number;

	// Actions
	setSidebarWidth(width: number): void;
	toggleSidebar(): void;
	setResultsPanelHeight(height: number): void;
}
```

---

### ThemeStore

Manages theme preference and resolved mode. Already partially implemented in existing codebase.

```typescript
interface ThemeStore {
	// State
	readonly mode: 'light' | 'dark';
	readonly preference: 'light' | 'dark' | 'system';
	readonly isDark: boolean;

	// Actions
	setPreference(preference: 'light' | 'dark' | 'system'): void;
	toggle(): void;
}
```

---

## localStorage Keys

| Key               | Data              | Description                                            |
| ----------------- | ----------------- | ------------------------------------------------------ |
| `tusk-tabs`       | `Tab[]`           | Open tabs (content may be truncated for large queries) |
| `tusk-active-tab` | `string`          | Active tab ID                                          |
| `tusk-ui-state`   | `UIState`         | Layout preferences                                     |
| `tusk-theme`      | `ThemePreference` | Theme settings                                         |

**Note**: Connection data is NOT stored in localStorage. Connections are stored in SQLite via Rust backend and loaded on startup.

---

## Validation Functions

```typescript
function isValidTab(tab: unknown): tab is Tab {
	return (
		typeof tab === 'object' &&
		tab !== null &&
		typeof (tab as Tab).id === 'string' &&
		isValidUUID((tab as Tab).id) &&
		typeof (tab as Tab).title === 'string' &&
		isValidTabType((tab as Tab).type)
	);
}

function isValidUUID(str: string): boolean {
	return /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(str);
}

function isValidTabType(type: unknown): type is TabType {
	return ['query', 'table', 'view', 'function', 'schema'].includes(type as string);
}
```

---

## Migration Strategy

For future schema changes, localStorage data includes a version field:

```typescript
interface StorageSchema {
	version: number;
	data: unknown;
}

function migrate(stored: StorageSchema): StorageSchema {
	let current = stored;

	// Version 1 -> 2: Add cursorPosition to tabs
	if (current.version < 2) {
		current = migrateV1ToV2(current);
	}

	return current;
}
```

Current version: **1**
