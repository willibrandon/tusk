# Tusk — Claude Development Context

## Project Overview

Tusk is a fast, free, native Postgres client built with Tauri. It aims to be a complete replacement for pgAdmin and DBeaver for Postgres-only workflows.

**Design Document:** `docs/design.md`

## Technology Stack

### Frontend
- **Framework:** Svelte 5 (compiled reactivity, minimal runtime)
- **Editor:** Monaco Editor (SQL editing with autocomplete)
- **Data Grid:** TanStack Table + custom virtualization
- **Diagrams:** @xyflow/svelte (ER diagram canvas)
- **Styling:** Tailwind CSS
- **State:** Svelte stores + context

### Backend (Rust)
- **Framework:** Tauri v2
- **Postgres Driver:** tokio-postgres (async, streaming, COPY protocol)
- **Connection Pooling:** deadpool-postgres
- **SSH Tunnels:** russh (pure Rust SSH2)
- **Local Storage:** rusqlite (SQLite for metadata)
- **Credentials:** keyring (OS keychain integration)
- **Serialization:** serde + serde_json

## MCP Servers Available

### Playwright MCP
Browser automation for testing web interfaces. Available tools:
- `mcp__playwright__browser_navigate` - Navigate to URLs
- `mcp__playwright__browser_snapshot` - Capture accessibility snapshots
- `mcp__playwright__browser_click` - Click elements
- `mcp__playwright__browser_type` - Type text
- `mcp__playwright__browser_fill_form` - Fill multiple form fields
- `mcp__playwright__browser_take_screenshot` - Capture screenshots
- `mcp__playwright__browser_evaluate` - Execute JavaScript
- `mcp__playwright__browser_wait_for` - Wait for conditions

**Use for:** Testing the Svelte frontend in isolation, verifying UI components, accessibility testing.

### Tauri MCP Server
Native Tauri app automation and testing. Available tools:
- `mcp___hypothesi_tauri-mcp-server__driver_session` - Start/stop connection to running Tauri app
- `mcp___hypothesi_tauri-mcp-server__webview_screenshot` - Screenshot the webview
- `mcp___hypothesi_tauri-mcp-server__webview_dom_snapshot` - Get DOM/accessibility snapshot
- `mcp___hypothesi_tauri-mcp-server__webview_find_element` - Find DOM elements
- `mcp___hypothesi_tauri-mcp-server__webview_interact` - Click, scroll, swipe, focus
- `mcp___hypothesi_tauri-mcp-server__webview_keyboard` - Type text, key events
- `mcp___hypothesi_tauri-mcp-server__webview_execute_js` - Execute JavaScript in webview
- `mcp___hypothesi_tauri-mcp-server__webview_wait_for` - Wait for elements/text/events
- `mcp___hypothesi_tauri-mcp-server__webview_get_styles` - Get computed CSS styles
- `mcp___hypothesi_tauri-mcp-server__ipc_execute_command` - Execute Tauri IPC commands
- `mcp___hypothesi_tauri-mcp-server__ipc_monitor` - Monitor IPC traffic
- `mcp___hypothesi_tauri-mcp-server__ipc_emit_event` - Emit Tauri events
- `mcp___hypothesi_tauri-mcp-server__ipc_get_backend_state` - Get app metadata
- `mcp___hypothesi_tauri-mcp-server__manage_window` - List/resize windows
- `mcp___hypothesi_tauri-mcp-server__read_logs` - Read console/system logs

**Use for:** End-to-end testing of the complete Tauri application, testing IPC commands, verifying frontend-backend integration.

## Testing Workflow

1. **Unit Tests:** Rust backend tests via `cargo test`, Svelte component tests via Vitest
2. **Integration Tests:** Use Tauri MCP to test IPC commands and data flow
3. **E2E Tests:** Use Tauri MCP for full application testing with a running Postgres instance
4. **UI Tests:** Use Playwright MCP for isolated frontend component testing

## Project Structure

```
tusk/
├── docs/
│   ├── design.md              # Complete design specification
│   └── features/              # Feature implementation documents
├── src-tauri/                 # Rust backend
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── commands/          # Tauri IPC commands
│   │   ├── services/          # Business logic
│   │   │   ├── connection.rs  # Connection management
│   │   │   ├── query.rs       # Query execution
│   │   │   ├── schema.rs      # Schema introspection
│   │   │   ├── admin.rs       # Admin/monitoring
│   │   │   └── storage.rs     # Local SQLite storage
│   │   ├── models/            # Data structures
│   │   └── error.rs           # Error types
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                       # Svelte frontend
│   ├── lib/
│   │   ├── components/        # UI components
│   │   │   ├── shell/         # App shell (sidebar, tabs, status)
│   │   │   ├── editor/        # Monaco editor wrapper
│   │   │   ├── grid/          # Results grid
│   │   │   ├── tree/          # Schema browser tree
│   │   │   ├── dialogs/       # Modal dialogs
│   │   │   └── common/        # Shared components
│   │   ├── stores/            # Svelte stores
│   │   ├── services/          # Frontend services (IPC wrappers)
│   │   └── utils/             # Utilities
│   ├── routes/                # SvelteKit routes (if using)
│   └── app.html
├── package.json
├── svelte.config.js
├── tailwind.config.js
├── vite.config.ts
└── CLAUDE.md
```

## Key Design Decisions

1. **Postgres Only:** No multi-database support. Deep Postgres integration.
2. **Fully Local:** No cloud sync, no telemetry, no network calls except to Postgres servers.
3. **OS Keychain:** Passwords never stored in files, always in OS keychain.
4. **Streaming Results:** Large result sets streamed in batches via Tauri events.
5. **Virtual Scrolling:** Grid handles millions of rows via virtualization.
6. **Statement Timeout:** Configurable query timeout to prevent runaway queries.

## Performance Targets

| Metric | Target |
|--------|--------|
| Cold start | < 1 second |
| Memory (idle) | < 100 MB |
| Memory (1M rows) | < 500 MB |
| Render 1000 rows | < 100ms |
| Schema load (1000 tables) | < 500ms |
| Autocomplete response | < 50ms |

## Development Commands

```bash
# Install dependencies
npm install
cd src-tauri && cargo build

# Development
npm run tauri dev

# Build for production
npm run tauri build

# Run Rust tests
cd src-tauri && cargo test

# Run frontend tests
npm test
```

## Feature Implementation Order

See `docs/features/00-feature-index.md` for the complete ordered list of feature documents that must be implemented sequentially.

## IPC Command Patterns

All Tauri commands follow this pattern:

```rust
#[tauri::command]
async fn command_name(
    state: State<'_, AppState>,
    param: Type
) -> Result<ReturnType, Error> {
    // Implementation
}
```

For streaming large results:

```rust
// Emit batches via events
app.emit("query:rows", RowBatch { query_id, rows, batch_num })?;
app.emit("query:complete", QueryComplete { query_id, total_rows, elapsed_ms })?;
```

## Error Handling

All errors should include:
- User-friendly message
- Technical detail (for debugging)
- Hint (actionable suggestion)
- Position (for query errors)
- Postgres error code (if applicable)

## Security Requirements

1. Never log passwords or credentials
2. Use parameterized queries (never string interpolation)
3. Validate all user input
4. Respect read-only connection mode
5. Confirm destructive operations (DROP, TRUNCATE, DELETE without WHERE)
