# Tusk — Design Document

A fast, free, native Postgres client built with GPUI.

---

## 1. Goals

**Primary Goals**

- Complete replacement for pgAdmin and DBeaver for Postgres workflows
- Sub-second startup, minimal memory footprint (<200MB typical)
- Native performance for large result sets (1M+ rows)
- Cross-platform: Linux, macOS, Windows

**Non-Goals**

- Multi-database support (MySQL, SQLite, etc.) — Postgres only
- Cloud/sync features — fully local
- Plugin/extension system (v1)

---

## 2. Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Tusk Application (Rust + GPUI)              │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  UI Layer (GPUI Views & Elements)                        │   │
│  │  ┌────────────────┐ ┌────────────────┐ ┌──────────────┐  │   │
│  │  │  Workspace     │ │  Panels        │ │  Modals      │  │   │
│  │  │  ├── Panes     │ │  ├── Schema    │ │  ├── Connect │  │   │
│  │  │  ├── Tabs      │ │  │   Browser   │ │  ├── Settings│  │   │
│  │  │  └── StatusBar │ │  ├── History   │ │  └── Dialogs │  │   │
│  │  └────────────────┘ │  └── Admin     │ └──────────────┘  │   │
│  │                     └────────────────┘                    │   │
│  └──────────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Components                                               │   │
│  │  ├── SqlEditor (native text editor with syntax highlighting)│   │
│  │  ├── DataGrid (virtualized results using UniformList)     │   │
│  │  ├── QueryPlanViewer (EXPLAIN visualization)              │   │
│  │  ├── ErDiagram (schema visualization canvas)              │   │
│  │  └── Forms (connection, import, backup wizards)           │   │
│  └──────────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Services Layer (Async Rust)                              │   │
│  │  ┌────────────────┐ ┌────────────────┐ ┌───────────────┐ │   │
│  │  │  Connection    │ │  Query Engine  │ │  Schema       │ │   │
│  │  │  Manager       │ │                │ │  Service      │ │   │
│  │  │  - Pool mgmt   │ │  - Execution   │ │  - Introspect │ │   │
│  │  │  - SSH tunnels │ │  - Streaming   │ │  - DDL gen    │ │   │
│  │  │  - SSL/TLS     │ │  - Cancellation│ │  - Dep graph  │ │   │
│  │  └────────────────┘ └────────────────┘ └───────────────┘ │   │
│  │  ┌────────────────┐ ┌────────────────┐ ┌───────────────┐ │   │
│  │  │  Admin Service │ │  Import/Export │ │  Local Storage│ │   │
│  │  │                │ │                │ │               │ │   │
│  │  │  - pg_stat_*   │ │  - CSV/JSON    │ │  - SQLite     │ │   │
│  │  │  - Vacuum      │ │  - pg_dump     │ │  - Keyring    │ │   │
│  │  │  - Roles       │ │  - pg_restore  │ │  - Settings   │ │   │
│  │  └────────────────┘ └────────────────┘ └───────────────┘ │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 2.1 Technology Stack

| Component          | Library                  | Rationale                                                  |
| ------------------ | ------------------------ | ---------------------------------------------------------- |
| UI Framework       | GPUI (from Zed)          | GPU-accelerated, native performance, Rust-native           |
| Text Editor        | Custom GPUI Editor       | Native syntax highlighting, Zed-quality editing experience |
| Data Grid          | UniformList + custom     | Virtualized rendering, handles millions of rows            |
| Diagrams           | Custom GPUI Canvas       | Native rendering, no web dependencies                      |
| Postgres driver    | tokio-postgres           | Full async, streaming support, COPY protocol               |
| Connection pooling | deadpool-postgres        | Async-native pool management                               |
| SSH tunnels        | russh                    | Pure Rust SSH2 implementation                              |
| Local storage      | rusqlite                 | Embedded SQLite for metadata                               |
| Credentials        | keyring                  | OS keychain (macOS Keychain, Windows Credential Manager)   |
| Serialization      | serde + serde_json       | Standard Rust serialization                                |
| CLI tools          | std::process::Command    | Wraps pg_dump, pg_restore, psql                            |

### 2.2 GPUI Architecture Patterns

**Entity-Based State Management**

All application state is owned by the `App` context and accessed through `Entity<T>` handles:

```rust
// State is created and owned by the App
let connection_manager = cx.new(|_| ConnectionManager::new());
let query_service = cx.new(|_| QueryService::new());

// Entities are accessed through references
connection_manager.update(cx, |manager, cx| {
    manager.connect(config, cx);
});
```

**View Rendering with the Render Trait**

All UI components implement the `Render` trait:

```rust
impl Render for QueryTab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_toolbar(window, cx))
            .child(self.render_editor(window, cx))
            .child(self.render_results(window, cx))
    }
}
```

**Event-Driven Updates**

Components communicate through subscriptions and events:

```rust
impl EventEmitter<QueryEvent> for QueryTab {}

// Subscribe to events
cx.subscribe(&query_tab, |workspace, tab, event: &QueryEvent, cx| {
    match event {
        QueryEvent::ExecutionComplete { rows, elapsed } => {
            workspace.update_status(rows, elapsed, cx);
        }
        QueryEvent::Error { message } => {
            workspace.show_error(message, cx);
        }
    }
}).detach();
```

### 2.3 Workspace Architecture

The application uses a workspace-based architecture similar to Zed:

```rust
pub struct Workspace {
    /// Pane groups for split views
    center: PaneGroup,

    /// Side panels (schema browser, history)
    left_dock: Dock,
    right_dock: Dock,
    bottom_dock: Dock,

    /// Active connections
    connections: Entity<ConnectionManager>,

    /// Schema cache per connection
    schema_cache: Entity<SchemaCache>,

    /// Application settings
    settings: Entity<Settings>,

    /// Focus management
    focus_handle: FocusHandle,
}

pub struct Pane {
    /// Items (tabs) in this pane
    items: Vec<Box<dyn Item>>,

    /// Currently active item index
    active_item_index: usize,

    /// Scroll handle for tab bar
    tab_bar_scroll_handle: ScrollHandle,
}

pub trait Item: Render + EventEmitter<ItemEvent> {
    fn tab_content(&self, cx: &App) -> AnyElement;
    fn tab_icon(&self, cx: &App) -> Option<Icon>;
    fn is_dirty(&self, cx: &App) -> bool;
    fn can_save(&self, cx: &App) -> bool;
    fn save(&mut self, cx: &mut Context<Self>) -> Task<Result<()>>;
}
```

---

## 3. Data Models

### 3.1 Connection

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: Uuid,
    pub name: String,
    pub color: Option<String>,    // Hex color for visual identification
    pub group_id: Option<Uuid>,   // Folder grouping

    pub host: String,
    pub port: u16,                // Default 5432
    pub database: String,
    pub username: String,
    pub password_in_keyring: bool, // If true, fetch from OS keyring

    pub ssl_mode: SslMode,
    pub ssl_ca_cert: Option<PathBuf>,
    pub ssl_client_cert: Option<PathBuf>,
    pub ssl_client_key: Option<PathBuf>,

    pub ssh_tunnel: Option<SshTunnelConfig>,
    pub options: ConnectionOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SslMode {
    Disable,
    #[default]
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshTunnelConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,                 // Default 22
    pub username: String,
    pub auth: SshAuthMethod,
    pub key_path: Option<PathBuf>,
    pub passphrase_in_keyring: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SshAuthMethod {
    Password,
    Key,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionOptions {
    pub connect_timeout_sec: u64,     // Default 10
    pub statement_timeout_ms: Option<u64>,
    pub application_name: String,      // Default "Tusk"
    pub readonly: bool,                // Prevent writes
}
```

### 3.2 Schema Objects

```rust
#[derive(Debug, Clone)]
pub struct Schema {
    pub name: String,
    pub tables: Vec<Table>,
    pub views: Vec<View>,
    pub materialized_views: Vec<MaterializedView>,
    pub functions: Vec<Function>,
    pub sequences: Vec<Sequence>,
    pub types: Vec<Type>,
    pub extensions: Vec<Extension>,
}

#[derive(Debug, Clone)]
pub struct Table {
    pub oid: u32,
    pub schema: String,
    pub name: String,
    pub columns: Vec<Column>,
    pub primary_key: Option<Constraint>,
    pub foreign_keys: Vec<ForeignKey>,
    pub unique_constraints: Vec<Constraint>,
    pub check_constraints: Vec<CheckConstraint>,
    pub indexes: Vec<Index>,
    pub triggers: Vec<Trigger>,
    pub policies: Vec<Policy>,        // RLS policies
    pub row_count_estimate: i64,      // From pg_class.reltuples
    pub size_bytes: i64,              // From pg_total_relation_size
    pub comment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Column {
    pub ordinal: i16,
    pub name: String,
    pub type_name: String,            // Full type with modifiers (varchar(255))
    pub base_type: String,            // Base type (varchar)
    pub nullable: bool,
    pub default: Option<String>,
    pub is_identity: bool,
    pub identity_generation: Option<IdentityGeneration>,
    pub is_generated: bool,
    pub generation_expression: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Index {
    pub oid: u32,
    pub name: String,
    pub columns: Vec<String>,
    pub include_columns: Vec<String>, // INCLUDE clause
    pub is_unique: bool,
    pub is_primary: bool,
    pub is_partial: bool,
    pub predicate: Option<String>,    // WHERE clause for partial
    pub method: IndexMethod,
    pub size_bytes: i64,
    pub definition: String,           // Full CREATE INDEX statement
}

#[derive(Debug, Clone)]
pub enum IndexMethod {
    Btree,
    Hash,
    Gist,
    Gin,
    Brin,
}

#[derive(Debug, Clone)]
pub struct ForeignKey {
    pub name: String,
    pub columns: Vec<String>,
    pub referenced_schema: String,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_delete: ForeignKeyAction,
    pub on_update: ForeignKeyAction,
    pub deferrable: bool,
    pub initially_deferred: bool,
}

#[derive(Debug, Clone)]
pub enum ForeignKeyAction {
    NoAction,
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub oid: u32,
    pub schema: String,
    pub name: String,
    pub arguments: Vec<Argument>,
    pub return_type: String,
    pub language: String,             // plpgsql, sql, python, etc.
    pub volatility: Volatility,
    pub is_strict: bool,
    pub is_security_definer: bool,
    pub source: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Volatility {
    Immutable,
    Stable,
    Volatile,
}
```

### 3.3 Query Results

```rust
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub query_id: Uuid,
    pub status: QueryStatus,
    pub command: String,              // SELECT, INSERT, UPDATE, etc.

    // For SELECT queries
    pub columns: Option<Vec<ColumnMeta>>,
    pub rows: Option<Vec<Row>>,
    pub total_rows: Option<usize>,
    pub truncated: bool,              // True if row limit hit

    // For DML queries
    pub rows_affected: Option<u64>,

    // For EXPLAIN
    pub plan: Option<QueryPlan>,

    // Timing
    pub elapsed_ms: u64,

    // Errors
    pub error: Option<QueryError>,
}

#[derive(Debug, Clone)]
pub enum QueryStatus {
    Running,
    Success,
    Error,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct QueryError {
    pub message: String,
    pub detail: Option<String>,
    pub hint: Option<String>,
    pub position: Option<u32>,        // Character position in query
    pub code: String,                 // Postgres error code (23505, etc.)
}

#[derive(Debug, Clone)]
pub struct ColumnMeta {
    pub name: String,
    pub type_oid: u32,
    pub type_name: String,
    pub type_modifier: i32,
    pub table_oid: Option<u32>,
    pub column_ordinal: Option<i16>,
}

pub type Row = Vec<Value>;

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Numeric(String),
    Text(String),
    Bytea(Vec<u8>),
    Timestamp(chrono::NaiveDateTime),
    TimestampTz(chrono::DateTime<chrono::Utc>),
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
    Interval(String),
    Uuid(Uuid),
    Json(serde_json::Value),
    Array(Vec<Value>),
    Point { x: f64, y: f64 },
    Unknown(String),
}
```

### 3.4 Query Plan

```rust
#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub raw: String,                  // Original EXPLAIN output
    pub format: PlanFormat,
    pub root: PlanNode,
    pub planning_time_ms: f64,
    pub execution_time_ms: Option<f64>, // Only with ANALYZE
    pub triggers: Vec<TriggerTiming>,
}

#[derive(Debug, Clone)]
pub enum PlanFormat {
    Text,
    Json,
}

#[derive(Debug, Clone)]
pub struct PlanNode {
    pub node_type: String,            // Seq Scan, Index Scan, Nested Loop, etc.
    pub relation_name: Option<String>,
    pub alias: Option<String>,
    pub index_name: Option<String>,
    pub join_type: Option<String>,

    // Estimates
    pub startup_cost: f64,
    pub total_cost: f64,
    pub plan_rows: f64,
    pub plan_width: i32,

    // Actuals (ANALYZE only)
    pub actual_startup_time: Option<f64>,
    pub actual_total_time: Option<f64>,
    pub actual_rows: Option<u64>,
    pub actual_loops: Option<u64>,

    // Details
    pub filter: Option<String>,
    pub index_cond: Option<String>,
    pub recheck_cond: Option<String>,
    pub sort_key: Option<Vec<String>>,
    pub hash_cond: Option<String>,

    // Buffers (BUFFERS option)
    pub shared_hit_blocks: Option<u64>,
    pub shared_read_blocks: Option<u64>,
    pub shared_written_blocks: Option<u64>,

    // I/O timing (timing option)
    pub io_read_time_ms: Option<f64>,
    pub io_write_time_ms: Option<f64>,

    // Children
    pub children: Vec<PlanNode>,

    // Computed for visualization
    pub percent_of_total: f64,
    pub is_slowest: bool,
}
```

### 3.5 Local Storage Schema (SQLite)

```sql
-- Connection definitions (passwords in OS keyring)
CREATE TABLE connections (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  config_json TEXT NOT NULL,       -- Serialized Connection struct
  group_id TEXT REFERENCES groups(id),
  sort_order INTEGER,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP,
  last_connected_at TEXT
);

CREATE TABLE groups (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  parent_id TEXT REFERENCES groups(id),
  sort_order INTEGER,
  color TEXT
);

-- Query history per connection
CREATE TABLE query_history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  connection_id TEXT NOT NULL REFERENCES connections(id),
  sql TEXT NOT NULL,
  executed_at TEXT DEFAULT CURRENT_TIMESTAMP,
  duration_ms INTEGER,
  rows_affected INTEGER,
  error TEXT,
  favorited BOOLEAN DEFAULT FALSE
);
CREATE INDEX idx_history_conn_time ON query_history(connection_id, executed_at DESC);

-- Saved queries (snippets)
CREATE TABLE saved_queries (
  id TEXT PRIMARY KEY,
  connection_id TEXT REFERENCES connections(id),  -- NULL = global
  name TEXT NOT NULL,
  description TEXT,
  sql TEXT NOT NULL,
  folder_id TEXT REFERENCES saved_query_folders(id),
  tags TEXT,                        -- JSON array
  created_at TEXT DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT
);

CREATE TABLE saved_query_folders (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  parent_id TEXT REFERENCES saved_query_folders(id)
);

-- Editor tabs state (restore on reopen)
CREATE TABLE editor_state (
  id TEXT PRIMARY KEY,
  connection_id TEXT REFERENCES connections(id),
  tab_type TEXT NOT NULL,           -- 'query', 'table', 'view', 'function'
  tab_title TEXT,
  content_json TEXT,                -- Tab-specific state
  sort_order INTEGER,
  is_active BOOLEAN DEFAULT FALSE
);

-- Application settings
CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL
);

-- Workspace state for session restore
CREATE TABLE workspace_state (
  id TEXT PRIMARY KEY,
  state_json TEXT NOT NULL,         -- Serialized workspace layout
  updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

---

## 4. Feature Specifications

### 4.1 Connection Management

**Connection Dialog Fields**

- General: Name, Color, Host, Port, Database, Username, Password
- SSL: Mode dropdown, CA cert path, Client cert path, Client key path
- SSH Tunnel: Enable toggle, Host, Port, Username, Auth method, Key path
- Options: Connect timeout, Statement timeout, Application name, Read-only mode

**Connection Tree Behavior**

- Groups are collapsible folders
- Drag-drop to reorder connections and groups
- Right-click context menu: Connect, Edit, Duplicate, Delete, New Query
- Color dot indicator shows connection status (gray=disconnected, green=connected, yellow=connecting, red=error)
- Double-click opens new query tab connected to that database

**Connection Lifecycle**

1. On connect: establish pool with min=1, max=10 connections
2. SSH tunnel established first if configured (local port forwarding)
3. SSL negotiation per configured mode
4. Background keepalive query every 60 seconds
5. Auto-reconnect on transient failures (3 retries with exponential backoff)
6. On disconnect: close pool, close SSH tunnel, update UI state

### 4.2 Schema Browser

**Tree Structure**

```
▼ my_connection (green dot)
  ▼ Schemas
    ▼ public
      ▼ Tables (42)
        ▼ users
            Columns
              id (integer, PK)
              email (varchar(255), unique)
              created_at (timestamptz)
            Indexes
              users_pkey (btree, primary)
              users_email_key (btree, unique)
            Foreign Keys
            Triggers
            Policies
        ▶ orders
        ▶ products
      ▶ Views (5)
      ▶ Materialized Views (2)
      ▶ Functions (18)
      ▶ Sequences (3)
      ▶ Types (4)
    ▶ auth
  ▶ Extensions (8)
  ▶ Roles (12)
  ▶ Tablespaces (2)
```

**Context Menu Actions by Object Type**

| Object            | Actions                                                                             |
| ----------------- | ----------------------------------------------------------------------------------- |
| Table             | Open, View Data, New Query, Edit, Create Similar, Drop, Truncate, View DDL, Refresh |
| View              | Open, View Data, New Query, Edit, Drop, View DDL                                    |
| Materialized View | Open, View Data, Refresh View, New Query, Drop, View DDL                            |
| Column            | Add to Query, Filter by Value, Copy Name                                            |
| Index             | View DDL, Reindex, Drop                                                             |
| Function          | Open, Execute, Edit, Drop, View DDL                                                 |
| Schema            | New Table, New View, New Function, Drop                                             |
| Role              | Edit, Drop, View Grants                                                             |

**Object Search**

- Cmd/Ctrl+Shift+P opens command palette style search
- Fuzzy matches across all schemas: tables, views, functions, columns
- Results show object type icon, full path (schema.name), and object type
- Enter navigates to object in tree and opens it

**DDL Generation**
For any object, generate:

- CREATE statement (with all options, defaults, comments)
- DROP statement (with CASCADE option)
- ALTER templates for common modifications
- For tables: CREATE TABLE AS SELECT (empty or with data)

### 4.3 Query Editor

**Editor Features**

- Native GPUI text editor with SQL language mode
- Schema-aware autocomplete:
  - Table names (schema.table or just table if in search_path)
  - Column names (after table alias or in FROM context)
  - Function names with signature preview
  - Keywords and snippets
- Syntax highlighting with Postgres-specific keywords
- Error squiggles at position returned by Postgres
- Multi-cursor editing
- Code folding for subqueries and CTEs
- Bracket matching and auto-close

**Autocomplete Data Flow**

1. On connection: fetch full schema metadata
2. Cache in memory, refresh on schema change events (LISTEN/NOTIFY) or manual refresh
3. Completion provider queries cached schema
4. Rank completions: columns from tables in query > other columns > tables > functions > keywords

**Tab Management**

- Tabs show: query name or "Query N", connection color dot, modified indicator (dot)
- Middle-click closes tab
- Drag to reorder
- Right-click: Close, Close Others, Close All, Close to the Right
- Double-click tab to rename
- Unsaved tabs prompt on close

**Execution**

- Cmd/Ctrl+Enter: Execute current statement (at cursor or selected)
- Cmd/Ctrl+Shift+Enter: Execute all statements
- Cmd/Ctrl+.: Cancel running query
- Statement detection: split on semicolons, respecting string literals and $$ blocks

**Editor Toolbar**

```
[▶ Run] [■ Stop] [📋 Format] [💾 Save] | [Connection: mydb ▼] | [Limit: 1000 ▼]
```

**Keyboard Shortcuts**
| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| Execute | Ctrl+Enter | Cmd+Enter |
| Execute All | Ctrl+Shift+Enter | Cmd+Shift+Enter |
| Cancel | Ctrl+. | Cmd+. |
| Format | Ctrl+Shift+F | Cmd+Shift+F |
| Save | Ctrl+S | Cmd+S |
| Comment Line | Ctrl+/ | Cmd+/ |
| Find | Ctrl+F | Cmd+F |
| Find/Replace | Ctrl+H | Cmd+Option+F |
| Go to Line | Ctrl+G | Cmd+G |

### 4.4 Results Grid

**Display Modes**

- Grid (default): spreadsheet-style cells
- Transposed: single row as key-value pairs (useful for wide tables)
- JSON: raw JSON output for JSONB columns

**Grid Features**

- Virtual scrolling via UniformList: renders only visible rows, handles 10M+ rows
- Column resizing by drag
- Column reordering by drag
- Click header to sort (client-side for loaded data)
- Right-click header: Hide column, Size to fit, Size all to fit
- Sticky row numbers column

**Cell Rendering by Type**
| Type | Rendering |
|------|-----------:|
| NULL | Gray italic "NULL" |
| boolean | Checkbox icon (read-only unless editing) |
| integer, numeric | Right-aligned |
| text, varchar | Left-aligned, truncated with ellipsis |
| json, jsonb | Syntax highlighted preview, click to expand |
| bytea | Hex preview, option to save as file |
| timestamp, date, time | Formatted per locale settings |
| interval | Human-readable (2 days 3 hours) |
| array | Bracketed preview, click to expand |
| uuid | Monospace |
| inet, cidr | With CIDR notation |
| point, line, polygon | Coordinate preview |

**Cell Selection**

- Click selects cell
- Shift+click selects range
- Ctrl/Cmd+click adds to selection
- Ctrl/Cmd+A selects all
- Arrow keys navigate
- Ctrl/Cmd+C copies selection (TSV format by default)

**Context Menu**

- Copy (Ctrl+C)
- Copy as INSERT
- Copy as UPDATE (for single row)
- Copy as JSON
- Copy column name
- Filter to this value
- Filter to NOT this value
- Set to NULL (edit mode)
- Open in viewer (JSON, text)

**Pagination Controls**

```
[|◀] [◀] Page 1 of 100 [▶] [▶|] | Showing 1-1000 of 100,000 rows | 42ms
```

**Export Options** (from toolbar button)

- CSV (with options: delimiter, quote char, header row)
- JSON (array of objects or array of arrays)
- SQL INSERT statements (with batch size option)
- SQL COPY format
- Excel (XLSX)
- Markdown table

### 4.5 Inline Data Editing

**Enabling Edit Mode**

- Toggle "Edit Mode" in results toolbar
- Only available for single-table SELECT queries with primary key

**Edit Behavior**

- Double-click cell to edit
- Tab moves to next cell
- Enter commits cell and moves down
- Escape cancels edit
- Changed cells highlighted in yellow
- New rows highlighted in green
- Deleted rows highlighted in red (strikethrough)

**Toolbar in Edit Mode**

```
[✓ Save Changes (3)] [✗ Discard] [+ Add Row] [Edit Mode: ON]
```

**Change Tracking**

- Track original value and new value per cell
- On save: generate minimal UPDATE/INSERT/DELETE statements
- Show preview of SQL before executing
- Execute in transaction, rollback on any error

**NULL Handling**

- Ctrl+0 or context menu "Set to NULL"
- Clear distinction between empty string and NULL

### 4.6 Query Plan Visualization

**EXPLAIN Options Dialog**

```
☑ ANALYZE (execute query)     ☑ BUFFERS
☑ VERBOSE                     ☑ TIMING
☐ COSTS                       ☐ WAL
Format: [JSON ▼]
```

**Visualization Modes**

1. **Tree View** (default)
   - Hierarchical tree matching plan structure
   - Each node shows: operation type, table/index name, row estimates vs actuals
   - Color-coded by time percentage (green → red gradient)
   - Expandable details panel per node

2. **Timeline View**
   - Horizontal bars showing actual execution time
   - Parallel operations shown on separate rows
   - Hover for details

3. **Text View**
   - Raw EXPLAIN output with syntax highlighting
   - Clickable node references

**Node Detail Panel**

```
┌─────────────────────────────────────────┐
│ Index Scan using users_email_idx        │
├─────────────────────────────────────────┤
│ Table:     users                        │
│ Index:     users_email_idx              │
│ Condition: email = 'test@example.com'   │
├─────────────────────────────────────────┤
│           Estimated    Actual           │
│ Rows:     1            1                │
│ Loops:    -            1                │
│ Time:     -            0.042ms          │
├─────────────────────────────────────────┤
│ Buffers:  Hit: 3  Read: 0               │
│ % of Total: 2.3%                        │
└─────────────────────────────────────────┘
```

**Warnings and Suggestions**
Automatically highlight:

- Sequential scans on large tables (> 10k rows estimated)
- Significant row estimate misses (actual > 10x estimated)
- Nested loops with high loop counts
- Sorts spilling to disk
- Hash joins exceeding work_mem

### 4.7 Table Data Viewer

Opened by double-clicking table or "View Data" context menu.

**Layout**

```
┌─────────────────────────────────────────────────────────────┐
│ public.users                                    [Edit Mode] │
├─────────────────────────────────────────────────────────────┤
│ Filter: [                    ] [+ Add Filter] [Apply] [Clear]│
│         id = 5  ✕  |  created_at > '2024-01-01'  ✕          │
├─────────────────────────────────────────────────────────────┤
│  #  │ id │ email              │ name    │ created_at        │
│─────│────│────────────────────│─────────│───────────────────│
│  1  │  5 │ alice@example.com  │ Alice   │ 2024-03-15 10:30  │
│  2  │ 12 │ bob@example.com    │ Bob     │ 2024-03-16 14:22  │
│ ... │    │                    │         │                   │
├─────────────────────────────────────────────────────────────┤
│ [|◀] [◀] Page 1 [▶] [▶|]  | 1-100 of 5,432 | Sort: id ASC   │
└─────────────────────────────────────────────────────────────┘
```

**Filter Builder**

- Click column header → filter options for that column
- Visual filter builder: Column, Operator, Value
- Operators vary by type:
  - Text: =, !=, LIKE, ILIKE, IS NULL, IS NOT NULL
  - Numeric: =, !=, <, <=, >, >=, BETWEEN, IS NULL
  - Date/Time: =, !=, <, >, BETWEEN, IS NULL
  - Boolean: = true, = false, IS NULL
  - Array: @> (contains), <@ (contained by), && (overlaps)
- Raw SQL mode for complex filters

**Sorting**

- Click column header to sort
- Shift+click to add secondary sort
- Sort indicator in header (▲/▼)
- Current sort shown in footer

### 4.8 ER Diagram

**Generation**

- Select schemas/tables to include
- Options:
  - Show columns: All, PK/FK only, None
  - Show data types
  - Show nullable indicators
  - Show indexes
  - Show constraints

**Rendering**

- Tables as nodes with columns listed
- Foreign keys as directed edges (arrows point to referenced table)
- Color-code by schema
- Zoom and pan with mouse/trackpad
- Minimap for navigation

**Layout Algorithms**

- Auto-layout options: Hierarchical, Force-directed, Circular
- Manual positioning (drag nodes)
- Snap to grid option
- Save layout per diagram

**Export**

- PNG (with configurable DPI)
- SVG (vector)
- Save diagram configuration for later

### 4.9 Admin Dashboard

**Activity Monitor (pg_stat_activity)**

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Active Queries                                           [Auto-refresh 5s]│
├─────────────────────────────────────────────────────────────────────────┤
│ PID   │ User    │ Database │ State  │ Duration │ Query                  │
│───────│─────────│──────────│────────│──────────│────────────────────────│
│ 12345 │ app     │ mydb     │ active │ 2.3s     │ SELECT * FROM orders...│
│ 12346 │ admin   │ mydb     │ idle   │ -        │                        │
│ 12347 │ app     │ mydb     │ active │ 45.2s    │ UPDATE inventory SE... │ [!]
└─────────────────────────────────────────────────────────────────────────┘
Context menu: View full query, Kill query, Kill connection
```

**Server Stats**

- Connection count (used/max)
- Database sizes
- Transaction rate (TPS)
- Cache hit ratio
- Replication lag (if replica)

**Table Stats (pg_stat_user_tables)**

```
┌───────────────────────────────────────────────────────────────────────────────┐
│ Table          │ Rows Est. │ Size    │ Seq Scans │ Idx Scans │ Dead Tuples │ Last Vacuum │
│────────────────│───────────│─────────│───────────│───────────│─────────────│─────────────│
│ public.orders  │ 1.2M      │ 245 MB  │ 12        │ 45,231    │ 12,456      │ 2 hours ago │
│ public.users   │ 50,432    │ 12 MB   │ 3         │ 892       │ 234         │ 1 day ago   │
└───────────────────────────────────────────────────────────────────────────────┘
Context menu: VACUUM, VACUUM FULL, ANALYZE, REINDEX
```

**Index Stats (pg_stat_user_indexes)**

- Usage count
- Size
- Unused index detection (0 scans since stats reset)
- Duplicate index detection
- Suggestions for removal

**Locks View (pg_locks + pg_stat_activity)**

- Waiting queries
- Blocking queries
- Lock types and targets
- Visual lock dependency graph

### 4.10 Maintenance Operations

**Vacuum Dialog**

```
Target: [public.orders ▼]

☐ FULL (rewrites entire table - requires exclusive lock)
☐ FREEZE (aggressive freezing)
☑ ANALYZE (update statistics)
☐ VERBOSE (show progress)

Parallel workers: [0 ▼] (auto)

[Cancel] [Run VACUUM]
```

**Reindex Dialog**

```
Target: [Table ▼] [public.orders ▼]
        ○ Table (all indexes)
        ○ Index (specific) [orders_pkey ▼]
        ○ Schema
        ○ Database

☑ CONCURRENTLY (no locks, slower)
☐ VERBOSE

[Cancel] [Run REINDEX]
```

### 4.11 Backup and Restore

**Backup Dialog**

```
┌─────────────────────────────────────────────────────────────┐
│ Backup Database                                             │
├─────────────────────────────────────────────────────────────┤
│ Source: [mydb @ localhost ▼]                                │
│                                                             │
│ Format:  ○ Custom (.backup) - recommended                   │
│          ○ Plain SQL (.sql)                                 │
│          ○ Directory                                        │
│          ○ Tar                                              │
│                                                             │
│ Output: [/backups/mydb_2024-03-15.backup    ] [Browse]      │
│                                                             │
│ Objects:                                                    │
│   ☑ Schema definitions                                      │
│   ☑ Data                                                    │
│   ☑ Indexes                                                 │
│   ☑ Triggers                                                │
│   ☑ Constraints                                             │
│                                                             │
│ Tables: [All tables ▼] or [Select specific...]              │
│                                                             │
│ Advanced:                                                   │
│   Compression: [Default ▼]                                  │
│   Jobs: [4] (parallel dump)                                 │
│   ☐ Include CREATE DATABASE                                 │
│   ☑ Include privileges (GRANT/REVOKE)                       │
│   ☐ Exclude table data for: [Select tables...]              │
│                                                             │
│ [Cancel]                                    [Create Backup] │
└─────────────────────────────────────────────────────────────┘
```

**Restore Dialog**

```
┌─────────────────────────────────────────────────────────────┐
│ Restore Database                                            │
├─────────────────────────────────────────────────────────────┤
│ Target: [mydb @ localhost ▼]                                │
│                                                             │
│ Source: [/backups/mydb_2024-03-15.backup    ] [Browse]      │
│                                                             │
│ Backup info: Custom format, 245 MB, created 2024-03-15      │
│                                                             │
│ Options:                                                    │
│   ☐ Clean (drop existing objects first)                     │
│   ☐ Create database                                         │
│   ☑ Exit on error                                           │
│   Jobs: [4] (parallel restore)                              │
│                                                             │
│ Selective restore:                                          │
│   ○ Everything                                              │
│   ○ Schema only (no data)                                   │
│   ○ Data only (no schema)                                   │
│   ○ Specific objects: [Select...]                           │
│                                                             │
│ [Cancel]                                          [Restore] │
└─────────────────────────────────────────────────────────────┘
```

**Progress Display**

- Real-time output from pg_dump/pg_restore
- Progress bar where available (restore with custom format)
- Cancel button (sends SIGTERM)
- Log saved to file

### 4.12 Import Wizard

**Step 1: Source Selection**

```
Source file: [/data/users.csv                     ] [Browse]

File type: [Auto-detect ▼]
           ○ CSV
           ○ JSON (array of objects)
           ○ JSON Lines (newline delimited)

Preview (first 5 rows):
┌───────────────────────────────────────────┐
│ email              │ name    │ age │ active│
│ alice@example.com  │ Alice   │ 28  │ true  │
│ bob@example.com    │ Bob     │ 35  │ false │
│ ...                │         │     │       │
└───────────────────────────────────────────┘
```

**Step 2: CSV Options** (if CSV)

```
Delimiter: [, ▼] (auto-detected)
Quote char: [" ▼]
Escape char: [\ ▼]
☑ Has header row
Encoding: [UTF-8 ▼]
Null string: [\N ▼]
```

**Step 3: Target Selection**

```
Target table:
  ○ Existing: [public ▼].[users ▼]
  ○ Create new: [public ▼].[            ]

On conflict:
  ○ Error (abort on duplicate)
  ○ Skip (ignore duplicates)
  ○ Update (upsert on key: [id ▼])
```

**Step 4: Column Mapping**

```
┌──────────────────────────────────────────────────────────────────┐
│ Source Column  │ Target Column      │ Type        │ Transform   │
│────────────────│────────────────────│─────────────│─────────────│
│ email          │ [email ▼]          │ varchar(255)│ [None ▼]    │
│ name           │ [name ▼]           │ varchar(100)│ [None ▼]    │
│ age            │ [age ▼]            │ integer     │ [None ▼]    │
│ active         │ [is_active ▼]      │ boolean     │ [Boolean ▼] │
│ (skip)         │ -                  │ -           │ -           │
└──────────────────────────────────────────────────────────────────┘

Transforms: None, Trim, Uppercase, Lowercase, Parse date, Boolean, Custom SQL
```

**Step 5: Execute**

```
Import method:
  ○ INSERT (slower, per-row errors)
  ○ COPY (faster, batch)

Batch size: [1000]
☑ Use transaction (rollback all on error)

[Preview SQL] [Cancel] [Import]
```

### 4.13 Role Management

**Role List View**

```
┌────────────────────────────────────────────────────────────────────┐
│ Role         │ Login │ Superuser │ Create DB │ Connections │ Valid │
│──────────────│───────│───────────│───────────│─────────────│───────│
│ postgres     │ ✓     │ ✓         │ ✓         │ -1          │ ✓     │
│ app_user     │ ✓     │           │           │ 10          │ ✓     │
│ readonly     │ ✓     │           │           │ 5           │ ✓     │
│ developers   │       │           │           │ -           │ ✓     │
└────────────────────────────────────────────────────────────────────┘
```

**Role Editor Dialog**

```
┌─────────────────────────────────────────────────────────────┐
│ Edit Role: app_user                                         │
├─────────────────────────────────────────────────────────────┤
│ General                                                     │
│   Name: [app_user           ]                               │
│   Password: [••••••••       ] [Generate]                    │
│   Valid until: [No expiration ▼]                            │
│                                                             │
│ Privileges                                                  │
│   ☑ Can login                                               │
│   ☐ Superuser                                               │
│   ☐ Create databases                                        │
│   ☐ Create roles                                            │
│   ☐ Replication                                             │
│   ☐ Bypass RLS                                              │
│                                                             │
│ Limits                                                      │
│   Connection limit: [10    ] (-1 = unlimited)               │
│                                                             │
│ Membership                                                  │
│   Member of: [developers ✕] [readonly ✕] [+ Add]            │
│   Members:   [intern_1 ✕] [+ Add]                           │
│                                                             │
│ [View SQL] [Cancel] [Save]                                  │
└─────────────────────────────────────────────────────────────┘
```

**Privileges Grid**
Visual matrix: rows = roles, columns = objects, cells = permission indicators

```
                    │ SELECT │ INSERT │ UPDATE │ DELETE │ TRUNCATE │
────────────────────│────────│────────│────────│────────│──────────│
public.users        │   ✓    │   ✓    │   ✓    │        │          │
public.orders       │   ✓    │   ✓    │        │        │          │
public.audit_log    │   ✓    │        │        │        │          │
```

Click cell to toggle, batch operations via context menu.

### 4.14 Extension Manager

**Extension List**

```
┌──────────────────────────────────────────────────────────────────────────┐
│ Extension      │ Version   │ Installed │ Schema   │ Description          │
│────────────────│───────────│───────────│──────────│──────────────────────│
│ pgcrypto       │ 1.3       │ ✓         │ public   │ Cryptographic funcs  │
│ uuid-ossp      │ 1.1       │ ✓         │ public   │ UUID generation      │
│ pg_stat_stat...│ 1.10      │           │ -        │ Query statistics     │
│ postgis        │ 3.3       │           │ -        │ Geographic objects   │
└──────────────────────────────────────────────────────────────────────────┘

[Install Selected] [Uninstall Selected] [Refresh Available]
```

**Extension Details**

- Description
- Required dependencies
- Objects created (types, functions, operators)
- Configuration parameters
- Upgrade path if newer version available

---

## 5. Settings

### 5.1 Settings Categories

**General**

- Theme: Light / Dark / System
- Language: English (more later)
- Startup: Restore previous session / Start fresh
- Auto-save: query tabs every N seconds

**Editor**

- Font family, size
- Tab size, spaces vs tabs
- Line numbers
- Minimap
- Word wrap
- Auto-complete delay
- Bracket matching

**Results**

- Default row limit
- Date/time format
- Number format (locale)
- NULL display text
- Truncate text at N characters
- Copy format (TSV / CSV / JSON)

**Query Execution**

- Default statement timeout
- Confirm before executing DDL
- Confirm before executing DELETE/UPDATE without WHERE
- Auto-uppercase keywords

**Connections**

- Default SSL mode
- Default connection timeout
- Auto-reconnect attempts
- Keepalive interval

**Shortcuts**

- Full list of actions with customizable keybindings
- Search/filter
- Reset to defaults
- Import/export

### 5.2 Persisted Per-Connection

- Schema browser expanded state
- Default schema (search_path)
- Row limit override
- Statement timeout override

---

## 6. Platform-Specific Considerations

### 6.1 macOS

- Native menu bar integration via GPUI platform API
- Cmd key bindings
- Native window controls (traffic lights)
- Notarization for distribution
- macOS Keychain for credential storage
- Metal-based GPU rendering

### 6.2 Windows

- Native window chrome via GPUI platform API
- Ctrl key bindings
- Windows Credential Manager for credentials
- Installer (MSI or NSIS) and portable ZIP
- DirectX-based GPU rendering

### 6.3 Linux

- Follows XDG spec for config/data directories
- Secret Service API (GNOME Keyring, KWallet) for credentials
- AppImage, .deb, .rpm packages
- Vulkan-based GPU rendering via Blade
- Wayland and X11 support

---

## 7. Security

### 7.1 Credential Storage

- Passwords never stored in SQLite or config files
- All credentials in OS keychain (keyring-rs)
- SSH key passphrases also in keychain
- Option to require master password on startup (encrypts keychain access key)

### 7.2 Connection Security

- SSL/TLS by default (ssl_mode: prefer)
- Certificate validation when required
- SSH tunnel for connections over untrusted networks
- No telemetry or network calls except to configured Postgres servers

### 7.3 Query Safety

- Read-only mode per connection (blocks INSERT, UPDATE, DELETE, DDL)
- Confirmation dialogs for destructive operations
- Statement timeout to prevent runaway queries
- LIMIT clause auto-added for SELECT without LIMIT (configurable)

---

## 8. Error Handling

### 8.1 Connection Errors

| Error              | User Message                | Action                          |
| ------------------ | --------------------------- | ------------------------------- |
| ECONNREFUSED       | "Cannot connect to server"  | Check host/port, firewall       |
| Auth failure       | "Authentication failed"     | Check username/password         |
| SSL required       | "Server requires SSL"       | Enable SSL in connection        |
| SSH tunnel failure | "SSH tunnel failed: reason" | Check SSH credentials           |
| Timeout            | "Connection timed out"      | Increase timeout, check network |

### 8.2 Query Errors

- Show full error message with position highlighted in editor
- Include DETAIL and HINT if provided by Postgres
- Quick actions: Google error code, copy error, retry

### 8.3 Recovery

- Graceful handling of connection drops mid-query
- Auto-reconnect with retry UI
- Never lose unsaved query tabs (persist to SQLite immediately on change)

---

## 9. Performance Targets

| Metric                            | Target     |
| --------------------------------- | ---------- |
| Cold start time                   | < 1 second |
| Memory (idle)                     | < 100 MB   |
| Memory (1M rows loaded)           | < 500 MB   |
| Query result render (1000 rows)   | < 100ms    |
| Schema browser load (1000 tables) | < 500ms    |
| Autocomplete response             | < 50ms     |

### 9.1 Optimizations

**Result Streaming**

- Stream rows from Postgres in batches (default 1000)
- Render first batch immediately
- Continue streaming in background
- UniformList only renders visible rows

**Schema Caching**

- Full schema fetch on connect
- Incremental refresh on NOTIFY events
- Index autocomplete data in memory (trie or similar)

**UI Virtualization**

- Schema tree: UniformList for 1000s of objects
- Results grid: UniformList for rows, virtual columns
- Only render visible + small buffer

---

## 10. Future Considerations (v2+)

- Query formatting with customizable rules
- Compare data between two databases
- Schema diff and migration generation
- Query plan diff (compare two executions)
- Scheduled query execution
- Query performance history tracking
- Integration with version control (save queries to git)
- Team features (shared connections, snippets) - would require sync
- Support for Postgres-compatible databases (CockroachDB, Yugabyte, Aurora)
- Plugin system for custom visualizations

---

## Appendix A: Schema Introspection Queries

```sql
-- Tables with row count and size
SELECT
  n.nspname AS schema,
  c.relname AS name,
  c.oid,
  c.reltuples::bigint AS row_count_estimate,
  pg_total_relation_size(c.oid) AS size_bytes,
  obj_description(c.oid) AS comment
FROM pg_class c
JOIN pg_namespace n ON n.oid = c.relnamespace
WHERE c.relkind = 'r'
  AND n.nspname NOT IN ('pg_catalog', 'information_schema')
ORDER BY n.nspname, c.relname;

-- Columns for a table
SELECT
  a.attnum AS ordinal,
  a.attname AS name,
  pg_catalog.format_type(a.atttypid, a.atttypmod) AS type,
  t.typname AS base_type,
  NOT a.attnotnull AS nullable,
  pg_get_expr(d.adbin, d.adrelid) AS default,
  a.attidentity != '' AS is_identity,
  CASE a.attidentity WHEN 'a' THEN 'ALWAYS' WHEN 'd' THEN 'BY DEFAULT' END AS identity_generation,
  a.attgenerated != '' AS is_generated,
  pg_get_expr(d.adbin, d.adrelid) FILTER (WHERE a.attgenerated != '') AS generation_expression,
  col_description(a.attrelid, a.attnum) AS comment
FROM pg_attribute a
JOIN pg_type t ON t.oid = a.atttypid
LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
WHERE a.attrelid = $1::regclass
  AND a.attnum > 0
  AND NOT a.attisdropped
ORDER BY a.attnum;

-- Indexes for a table
SELECT
  i.indexrelid AS oid,
  c.relname AS name,
  array_agg(a.attname ORDER BY x.ordinality) AS columns,
  i.indisunique AS is_unique,
  i.indisprimary AS is_primary,
  pg_get_expr(i.indpred, i.indrelid) AS predicate,
  am.amname AS method,
  pg_relation_size(i.indexrelid) AS size_bytes,
  pg_get_indexdef(i.indexrelid) AS definition
FROM pg_index i
JOIN pg_class c ON c.oid = i.indexrelid
JOIN pg_am am ON am.oid = c.relam
CROSS JOIN LATERAL unnest(i.indkey) WITH ORDINALITY AS x(attnum, ordinality)
JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = x.attnum
WHERE i.indrelid = $1::regclass
GROUP BY i.indexrelid, c.relname, i.indisunique, i.indisprimary, i.indpred, i.indrelid, am.amname;

-- Foreign keys for a table
SELECT
  c.conname AS name,
  array_agg(a1.attname ORDER BY x.ordinality) AS columns,
  n2.nspname AS referenced_schema,
  c2.relname AS referenced_table,
  array_agg(a2.attname ORDER BY x.ordinality) AS referenced_columns,
  c.confupdtype AS on_update,
  c.confdeltype AS on_delete,
  c.condeferrable AS deferrable,
  c.condeferred AS initially_deferred
FROM pg_constraint c
JOIN pg_class c2 ON c2.oid = c.confrelid
JOIN pg_namespace n2 ON n2.oid = c2.relnamespace
CROSS JOIN LATERAL unnest(c.conkey, c.confkey) WITH ORDINALITY AS x(attnum1, attnum2, ordinality)
JOIN pg_attribute a1 ON a1.attrelid = c.conrelid AND a1.attnum = x.attnum1
JOIN pg_attribute a2 ON a2.attrelid = c.confrelid AND a2.attnum = x.attnum2
WHERE c.conrelid = $1::regclass
  AND c.contype = 'f'
GROUP BY c.conname, n2.nspname, c2.relname, c.confupdtype, c.confdeltype, c.condeferrable, c.condeferred;

-- Functions in schema
SELECT
  p.oid,
  n.nspname AS schema,
  p.proname AS name,
  pg_get_function_arguments(p.oid) AS arguments,
  pg_get_function_result(p.oid) AS return_type,
  l.lanname AS language,
  p.provolatile AS volatility,
  p.proisstrict AS is_strict,
  p.prosecdef AS is_security_definer,
  p.prosrc AS source,
  obj_description(p.oid) AS comment
FROM pg_proc p
JOIN pg_namespace n ON n.oid = p.pronamespace
JOIN pg_language l ON l.oid = p.prolang
WHERE n.nspname = $1
  AND p.prokind = 'f';
```

---

## Appendix B: Keyboard Shortcuts Reference

| Category       | Action           | Windows/Linux    | macOS           |
| -------------- | ---------------- | ---------------- | --------------- |
| **General**    | Settings         | Ctrl+,           | Cmd+,           |
|                | Command palette  | Ctrl+Shift+P     | Cmd+Shift+P     |
|                | New query tab    | Ctrl+N           | Cmd+N           |
|                | Close tab        | Ctrl+W           | Cmd+W           |
|                | Next tab         | Ctrl+Tab         | Cmd+Shift+]     |
|                | Previous tab     | Ctrl+Shift+Tab   | Cmd+Shift+[     |
|                | Toggle sidebar   | Ctrl+B           | Cmd+B           |
| **Editor**     | Execute          | Ctrl+Enter       | Cmd+Enter       |
|                | Execute all      | Ctrl+Shift+Enter | Cmd+Shift+Enter |
|                | Cancel query     | Ctrl+.           | Cmd+.           |
|                | Format           | Ctrl+Shift+F     | Cmd+Shift+F     |
|                | Save             | Ctrl+S           | Cmd+S           |
|                | Comment          | Ctrl+/           | Cmd+/           |
|                | Find             | Ctrl+F           | Cmd+F           |
|                | Replace          | Ctrl+H           | Cmd+Option+F    |
|                | Go to line       | Ctrl+G           | Cmd+G           |
|                | Duplicate line   | Ctrl+Shift+D     | Cmd+Shift+D     |
|                | Move line up     | Alt+Up           | Option+Up       |
|                | Move line down   | Alt+Down         | Option+Down     |
| **Results**    | Copy             | Ctrl+C           | Cmd+C           |
|                | Select all       | Ctrl+A           | Cmd+A           |
|                | Export           | Ctrl+E           | Cmd+E           |
|                | Toggle edit mode | Ctrl+Shift+E     | Cmd+Shift+E     |
| **Navigation** | Focus editor     | Ctrl+1           | Cmd+1           |
|                | Focus results    | Ctrl+2           | Cmd+2           |
|                | Focus sidebar    | Ctrl+0           | Cmd+0           |
|                | Search objects   | Ctrl+P           | Cmd+P           |
