# Data Model: Local Storage

**Feature**: 005-local-storage
**Date**: 2026-01-22

## Entity Relationship Diagram

```
┌─────────────────────┐     ┌─────────────────────┐
│  ConnectionGroup    │     │    QueryFolder      │
├─────────────────────┤     ├─────────────────────┤
│ group_id (PK)       │     │ folder_id (PK)      │
│ name                │     │ name                │
│ color               │     │ parent_id (FK, self)│◄──┐
│ sort_order          │     │ sort_order          │   │
│ created_at          │     │ created_at          │───┘
└─────────┬───────────┘     └─────────┬───────────┘
          │ 1:N                       │ 1:N
          ▼                           ▼
┌─────────────────────┐     ┌─────────────────────┐
│    Connection       │     │    SavedQuery       │
├─────────────────────┤     ├─────────────────────┤
│ connection_id (PK)  │     │ query_id (PK)       │
│ name                │     │ name                │
│ host                │     │ description         │
│ port                │     │ sql_text            │
│ database_name       │     │ folder_id (FK)      │
│ username            │     │ connection_id (FK)  │
│ ssl_mode            │     │ created_at          │
│ ssh_tunnel_id (FK)  │     │ updated_at          │
│ group_id (FK)       │◄────┤                     │
│ color               │     └─────────────────────┘
│ options (JSON)      │
│ created_at          │
│ updated_at          │
│ last_connected_at   │
└─────────┬───────────┘
          │ 1:N
          ▼
┌─────────────────────┐
│  QueryHistoryEntry  │
├─────────────────────┤
│ history_id (PK)     │
│ connection_id (FK)  │
│ sql_text            │
│ execution_time_ms   │
│ row_count           │
│ error_message       │
│ executed_at         │
└─────────────────────┘

┌─────────────────────┐     ┌─────────────────────┐
│    SshTunnel        │     │     UIState         │
├─────────────────────┤     ├─────────────────────┤
│ tunnel_id (PK)      │     │ key (PK)            │
│ name                │     │ value_json          │
│ host                │     │ updated_at          │
│ port                │     └─────────────────────┘
│ username            │
│ auth_method         │     Stores:
│ key_path            │     - editor_tabs (EditorTab[])
│ created_at          │     - window_state (WindowState)
└─────────────────────┘     - settings.* (AppSettings keys)
```

## Entity Definitions

### ConnectionGroup (NEW)

Named container for organizing connections.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| group_id | TEXT | PRIMARY KEY | UUID v4 |
| name | TEXT | NOT NULL | Display name (1-100 chars) |
| color | TEXT | NULL | Hex color (#RRGGBB) |
| sort_order | INTEGER | NOT NULL DEFAULT 0 | Display order |
| created_at | TEXT | NOT NULL | ISO 8601 timestamp |

**Validation Rules**:
- name: 1-100 characters, trimmed, non-empty
- color: NULL or valid hex format (#RRGGBB)
- sort_order: >= 0

### QueryFolder (NEW)

Hierarchical container for saved queries.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| folder_id | TEXT | PRIMARY KEY | UUID v4 |
| name | TEXT | NOT NULL | Display name (1-100 chars) |
| parent_id | TEXT | FK -> query_folders, NULL | Parent folder (NULL = root) |
| sort_order | INTEGER | NOT NULL DEFAULT 0 | Display order within parent |
| created_at | TEXT | NOT NULL | ISO 8601 timestamp |

**Validation Rules**:
- name: 1-100 characters, trimmed, non-empty
- parent_id: Valid folder_id or NULL
- No circular references (enforced in code)
- Max depth: 5 levels

### Connection (EXTENDED)

Database connection configuration. Extended with group_id.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| connection_id | TEXT | PRIMARY KEY | UUID v4 |
| name | TEXT | NOT NULL | Display name (1-255 chars) |
| host | TEXT | NOT NULL | Server hostname/IP |
| port | INTEGER | NOT NULL DEFAULT 5432 | Server port (1-65535) |
| database_name | TEXT | NOT NULL | Database name (1-63 chars) |
| username | TEXT | NOT NULL | Login username |
| ssl_mode | TEXT | NOT NULL DEFAULT 'prefer' | SSL configuration |
| ssh_tunnel_id | TEXT | FK -> ssh_tunnels, NULL | Optional SSH tunnel |
| group_id | TEXT | FK -> connection_groups, NULL | **NEW**: Optional group |
| color | TEXT | NULL | UI accent color |
| read_only | INTEGER | NOT NULL DEFAULT 0 | Read-only mode flag |
| connect_timeout_secs | INTEGER | NOT NULL DEFAULT 10 | Connection timeout |
| statement_timeout_secs | INTEGER | NULL | Query timeout |
| application_name | TEXT | NOT NULL DEFAULT 'Tusk' | PG application_name |
| created_at | TEXT | NOT NULL | ISO 8601 timestamp |
| updated_at | TEXT | NOT NULL | ISO 8601 timestamp |
| last_connected_at | TEXT | NULL | Last successful connection |

### SavedQuery (EXTENDED)

User-saved query. Extended with folder_id foreign key.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| query_id | TEXT | PRIMARY KEY | UUID v4 |
| connection_id | TEXT | FK -> connections, NULL | Optional association |
| name | TEXT | NOT NULL | Display name (1-255 chars) |
| description | TEXT | NULL | Optional description |
| sql_text | TEXT | NOT NULL | SQL content |
| folder_path | TEXT | NULL | **DEPRECATED**: String path |
| folder_id | TEXT | FK -> query_folders, NULL | **NEW**: Folder reference |
| created_at | TEXT | NOT NULL | ISO 8601 timestamp |
| updated_at | TEXT | NOT NULL | ISO 8601 timestamp |

### EditorTab (NEW - stored in UIState)

Persisted editor tab state. Stored as JSON array.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| tab_id | String | REQUIRED | UUID v4, unique per session |
| title | String | REQUIRED | Tab display title |
| content | String | REQUIRED | SQL content (may be empty) |
| cursor_position | usize | REQUIRED | Character offset |
| selection_start | Option<usize> | OPTIONAL | Selection start offset |
| selection_end | Option<usize> | OPTIONAL | Selection end offset |
| scroll_offset | f32 | REQUIRED | Vertical scroll position |
| saved_query_id | Option<String> | OPTIONAL | Associated saved query |
| connection_id | Option<String> | OPTIONAL | Associated connection |
| is_modified | bool | REQUIRED | Has unsaved changes |
| created_at | String | REQUIRED | ISO 8601 timestamp |

### WindowState (NEW - stored in UIState)

Window geometry and panel layout. Stored as JSON object.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| x | i32 | REQUIRED | Window X position |
| y | i32 | REQUIRED | Window Y position |
| width | u32 | REQUIRED | Window width (min 800) |
| height | u32 | REQUIRED | Window height (min 600) |
| is_maximized | bool | REQUIRED | Maximized state |
| sidebar_width | u32 | REQUIRED | Left sidebar width (min 200) |
| results_panel_height | u32 | REQUIRED | Bottom panel height (min 100) |
| messages_panel_visible | bool | REQUIRED | Messages panel visibility |

### AppSettings (NEW - stored in UIState)

Application-wide settings. Individual keys in UIState table.

| Setting Key | Type | Default | Description |
|-------------|------|---------|-------------|
| settings.theme | String | "dark" | UI theme (dark, light, system) |
| settings.font_family | String | "monospace" | Editor font family |
| settings.font_size | u32 | 14 | Editor font size (8-72) |
| settings.tab_size | u32 | 4 | Tab width (2, 4, 8) |
| settings.show_line_numbers | bool | true | Show line numbers in editor |
| settings.word_wrap | bool | false | Enable word wrap |
| settings.auto_complete | bool | true | Enable autocomplete |
| settings.confirm_destructive | bool | true | Confirm DROP/DELETE/TRUNCATE |
| settings.default_timeout_secs | Option<u32> | None | Default query timeout |
| settings.history_limit | u32 | 10000 | Max history entries |
| settings.recent_limit | u32 | 10 | Recent connections shown |

## State Transitions

### Connection Lifecycle

```
[New] ─create─► [Saved] ─connect─► [Active]
                  │  ▲                │
                  │  │                │
              edit│  │save        disconnect
                  │  │                │
                  ▼  │                ▼
               [Modified]         [Saved]
                  │
              delete
                  │
                  ▼
              [Deleted]
```

### Query History Entry Lifecycle

```
[Execute Query] ─success/error─► [Recorded] ─prune─► [Deleted]
```

### Editor Tab Lifecycle

```
[New Tab] ─type─► [Modified] ─save─► [Saved]
     │               │                  │
     │               │close             │close
     │               │                  │
     │               ▼                  ▼
     │         [Prompt Save?]      [Closed]
     │               │
     │         save/discard
     │               │
     └───────────────┴─────────────► [Closed]
```

## Indexes

### Existing Indexes
- `idx_connections_last_connected` ON connections(last_connected_at DESC)
- `idx_query_history_connection` ON query_history(connection_id, executed_at DESC)
- `idx_query_history_executed` ON query_history(executed_at DESC)
- `idx_saved_queries_folder` ON saved_queries(folder_path)

### New Indexes (Migration 2)
- `idx_connections_group` ON connections(group_id)
- `idx_query_folders_parent` ON query_folders(parent_id)
- `idx_saved_queries_folder_id` ON saved_queries(folder_id)
- `idx_connection_groups_sort` ON connection_groups(sort_order)

## Data Volume Estimates

| Entity | Expected Count | Row Size | Total Size |
|--------|----------------|----------|------------|
| Connections | 100 | ~500 bytes | ~50 KB |
| ConnectionGroups | 20 | ~200 bytes | ~4 KB |
| SshTunnels | 20 | ~300 bytes | ~6 KB |
| QueryHistory | 50,000 | ~500 bytes | ~25 MB |
| SavedQueries | 500 | ~2 KB | ~1 MB |
| QueryFolders | 100 | ~200 bytes | ~20 KB |
| UIState | 20 keys | ~10 KB avg | ~200 KB |

**Total estimated**: ~27 MB (well under 50 MB target)
