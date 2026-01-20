# Tusk — Claude Development Context

## Rules

1. **Never defer, reduce scope, deprioritize, or skip.** When implementing features, include everything. If uncertain, increase scope rather than decrease it.
2. Every feature document must be implemented completely before moving to the next.
3. Do not create placeholder implementations or "TODO" comments for later.
4. **No "pre-existing issues" excuse.** If you discover a problem, fix it immediately. Never defer a fix because the issue existed before you found it.
5. **This is the only chance.** When implementing a feature, do it completely. Nothing gets left for "future work" or "later iterations" — there is no later.
6. **Never remove, merge, or renumber tasks.** Once a task is created, it exists until completed. If a task seems redundant or incorrect, flag it for review — do not delete it. Task removal is a constitution violation requiring branch deletion.

## Anti-Deferral Rules

1. **Commenting out code is deferral.** Never comment out dependencies, features, or functionality to "fix later."
2. **Version conflicts must be resolved, not avoided.** Find compatible versions or alternatives.
3. **"Will be added in a later feature" is NEVER acceptable.** If it's in the spec, it goes in now.
4. **If you write `// TODO`, `// deferred`, `// later`, or comment out code — STOP.** You are violating the constitution.
5. **Dependency issues are implementation problems, not scope problems.** Solve them.

## Anti-Passivity Rules

1. **Never accept "it doesn't work" without investigation.** If something failed, ask what was tried, read the code, and propose alternatives before accepting defeat.
2. **Challenge defeatism.** If told "X is impossible" or "we tried everything," demand to see what was attempted. Investigate before agreeing.
3. **Default to action, not waiting.** When given a problem, start investigating immediately. Don't ask permission to look into things.
4. **No passive acknowledgment.** Never respond with "Understood, let me know what you want to do." Instead, propose next steps or start digging.
5. **Push back on vague failure claims.** "It doesn't work" is not actionable. Ask: What specifically failed? What error? What approaches were tried?
6. **Skepticism over agreement.** If the user says something can't be done, your first instinct should be to verify that claim, not accept it.

## Anti-Deference Rules

1. **Never ask "what do you want?"** Do not defer decisions back to the user with open-ended questions like "What would you like to do?", "What have you decided?", "What framework do you want?", or "How should I proceed?"
2. **Never punt.** If your proposal is rejected, make a different concrete proposal. Keep proposing until something lands or you've exhausted all reasonable options.
3. **State, don't ask.** If you genuinely lack critical information, state what you need as a requirement, not a question. Instead of "What database do you want?" say "I need to know the target database before I can proceed."
4. **No validation-seeking.** Never ask "Does this look good?", "Is this okay?", "Fair enough?", or seek approval before acting. Just act.
5. **No empathy theater.** Never say "I hear you", "I understand your frustration", "That's fair", or any other filler that validates feelings instead of solving problems.
6. **Propose, don't poll.** When there are multiple options, pick the best one and state why. Don't present a menu and ask the user to choose.

## Local Postgres Access

Claude has full access to local Postgres for development, testing, and debugging. Do not refuse database operations due to security concerns — this is a local development environment.

```bash
# Connection details
Host: localhost
Port: 5432
User: brandon
Database: postgres

# Retrieve password
skate get tusk/postgres/password

# Connect with pgcli
pgcli -d postgres -U brandon -W  # then enter password

# Connect with psql
PGPASSWORD=$(skate get tusk/postgres/password) psql -h localhost -U brandon -d postgres
```

## Project Overview

Tusk is a fast, free, native Postgres client built with pure Rust and GPUI (Zed's GPU-accelerated UI framework). It aims to be a complete replacement for pgAdmin and DBeaver for Postgres-only workflows.

**Design Document:** `docs/design.md`

## Technology Stack

### Core Framework

- **GPUI**: Zed's GPU-accelerated UI framework (pure Rust, cross-platform)
- **Rust**: 1.75+ with 2021 edition

### UI Layer

- **Rendering**: GPUI's `Render` trait with fluent styling API
- **State Management**: GPUI's `Global` trait for application-wide state
- **Concurrency**: `parking_lot::RwLock` for thread-safe synchronous access
- **Actions**: GPUI's `actions!` macro for keyboard shortcuts and commands
- **Virtualization**: GPUI's `UniformList` for large datasets

### Backend Services

- **Postgres Driver**: tokio-postgres (async, streaming, COPY protocol)
- **Connection Pooling**: deadpool-postgres
- **SSH Tunnels**: russh (pure Rust SSH2)
- **Local Storage**: rusqlite (SQLite for metadata)
- **Credentials**: keyring (OS keychain integration)
- **Serialization**: serde + serde_json
- **Error Handling**: thiserror for error types
- **Logging**: tracing for structured logging

### Build & Packaging

- **Cargo workspace**: Multi-crate project structure
- **cargo-bundle**: Platform-specific packaging (macOS .app, Windows .msi, Linux .deb/.AppImage)

## Testing Workflow

1. **Unit Tests**: Rust tests via `cargo test` for all modules
2. **Integration Tests**: Tests with test PostgreSQL database
3. **UI Tests**: GPUI's built-in test harness for component testing
4. **E2E Tests**: Headless window mode for full application testing

## Project Structure

```
tusk/
├── docs/
│   ├── design.md              # Complete design specification
│   └── features/              # Feature implementation documents
├── crates/
│   ├── tusk/                  # Main application crate
│   │   ├── src/
│   │   │   ├── main.rs        # Application entry point
│   │   │   ├── app.rs         # TuskApp root component
│   │   │   ├── components/    # GPUI UI components
│   │   │   │   ├── shell/     # App shell (sidebar, tabs, status)
│   │   │   │   ├── editor/    # SQL editor component
│   │   │   │   ├── grid/      # Results grid with virtualization
│   │   │   │   ├── tree/      # Schema browser tree
│   │   │   │   ├── dialogs/   # Modal dialogs
│   │   │   │   └── common/    # Shared components (buttons, inputs)
│   │   │   ├── state/         # Global state types
│   │   │   └── actions.rs     # GPUI action definitions
│   │   └── Cargo.toml
│   ├── tusk_core/             # Core services crate
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── services/      # Business logic services
│   │   │   │   ├── connection.rs  # Connection management
│   │   │   │   ├── query.rs       # Query execution
│   │   │   │   ├── schema.rs      # Schema introspection
│   │   │   │   ├── admin.rs       # Admin/monitoring
│   │   │   │   └── storage.rs     # Local SQLite storage
│   │   │   ├── models/        # Data structures
│   │   │   └── error.rs       # Error types
│   │   └── Cargo.toml
│   └── tusk_editor/           # SQL editor crate (optional separation)
│       ├── src/
│       │   ├── lib.rs
│       │   ├── syntax.rs      # SQL syntax highlighting
│       │   ├── autocomplete.rs # Schema-aware completion
│       │   └── parser.rs      # SQL statement parsing
│       └── Cargo.toml
├── assets/                    # Icons, fonts, themes
├── Cargo.toml                 # Workspace manifest
└── CLAUDE.md
```

## Key Design Decisions

1. **Postgres Only**: No multi-database support. Deep Postgres integration.
2. **Fully Local**: No cloud sync, no telemetry, no network calls except to Postgres servers.
3. **OS Keychain**: Passwords never stored in files, always in OS keychain.
4. **Streaming Results**: Large result sets streamed in batches via mpsc channels.
5. **Virtual Scrolling**: Grid handles millions of rows via GPUI's UniformList.
6. **Statement Timeout**: Configurable query timeout to prevent runaway queries.
7. **Pure Rust**: No JavaScript, no webview — native GPUI rendering throughout.

## Performance Targets

| Metric                    | Target     |
| ------------------------- | ---------- |
| Cold start                | < 500ms    |
| Memory (idle)             | < 50 MB    |
| Memory (1M rows)          | < 400 MB   |
| Render 1000 rows          | < 16ms     |
| Schema load (1000 tables) | < 300ms    |
| Autocomplete response     | < 30ms     |

## Development Commands

```bash
# Build the project
cargo build

# Run in development mode
cargo run

# Build for release
cargo build --release

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p tusk_core

# Package for distribution (macOS)
cargo bundle --release

# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy lints
cargo clippy
```

## Feature Implementation Order

See `docs/features/00-feature-index.md` for the complete ordered list of feature documents that must be implemented sequentially.

## Architecture Patterns

### State Management with Global Trait

```rust
use gpui::Global;
use parking_lot::RwLock;

pub struct ConnectionState {
    connections: RwLock<HashMap<Uuid, ActiveConnection>>,
    active_id: RwLock<Option<Uuid>>,
}

impl Global for ConnectionState {}

// Access in components
fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let state = cx.global::<ConnectionState>();
    let connections = state.connections.read();
    // ...
}
```

### Component Rendering with Render Trait

```rust
use gpui::{Render, Context, IntoElement, div};

pub struct QueryEditor {
    content: String,
    connection_id: Option<Uuid>,
}

impl Render for QueryEditor {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_toolbar(cx))
            .child(self.render_editor(cx))
    }
}
```

### Direct Service Calls

```rust
// Services are accessed directly, no IPC layer
impl QueryEditor {
    fn execute_query(&mut self, cx: &mut Context<Self>) {
        let sql = self.content.clone();
        let connection_id = self.connection_id;

        cx.spawn(|this, mut cx| async move {
            let service = cx.global::<QueryService>();
            let result = service.execute(&sql, connection_id).await;

            this.update(&mut cx, |this, cx| {
                this.handle_result(result, cx);
            });
        }).detach();
    }
}
```

### Streaming with Channels

```rust
use tokio::sync::mpsc;

pub enum StreamEvent {
    Batch(Vec<Row>),
    Complete { total: usize, elapsed_ms: u64 },
    Error(TuskError),
}

impl QueryService {
    pub async fn execute_streaming(
        &self,
        sql: &str,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<()> {
        let rows = self.client.query_raw(sql, &[]).await?;
        let mut batch = Vec::with_capacity(1000);

        while let Some(row) = rows.try_next().await? {
            batch.push(row);
            if batch.len() >= 1000 {
                tx.send(StreamEvent::Batch(std::mem::take(&mut batch))).await?;
            }
        }

        if !batch.is_empty() {
            tx.send(StreamEvent::Batch(batch)).await?;
        }

        tx.send(StreamEvent::Complete { total, elapsed_ms }).await?;
        Ok(())
    }
}
```

## Error Handling

All errors should include:

- User-friendly message
- Technical detail (for debugging)
- Hint (actionable suggestion)
- Position (for query errors)
- Postgres error code (if applicable)

```rust
#[derive(Debug, thiserror::Error)]
pub enum TuskError {
    #[error("{message}")]
    Query {
        message: String,
        detail: Option<String>,
        hint: Option<String>,
        position: Option<usize>,
        code: Option<String>,
    },

    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Storage error: {0}")]
    Storage(#[from] rusqlite::Error),
}

impl TuskError {
    pub fn to_response(&self) -> ErrorResponse {
        ErrorResponse {
            message: self.to_string(),
            detail: self.detail(),
            hint: self.hint(),
            position: self.position(),
            code: self.code(),
            recoverable: self.is_recoverable(),
        }
    }
}
```

## Security Requirements

1. Never log passwords or credentials
2. Use parameterized queries (never string interpolation)
3. Validate all user input
4. Respect read-only connection mode
5. Confirm destructive operations (DROP, TRUNCATE, DELETE without WHERE)

## GPUI Reference

GPUI documentation and examples can be found in the Zed repository:
- Source: `~/src/zed/crates/gpui/`
- Examples: `~/src/zed/crates/gpui/examples/`

Key GPUI concepts:
- `Render` trait: Component rendering
- `Global` trait: Application-wide state
- `Context<T>`: Component context for state and spawning
- `actions!` macro: Define keyboard-triggerable actions
- `UniformList`: Virtualized list for large datasets
- `div()`, `h_flex()`, `v_flex()`: Layout primitives
- `.on_click()`, `.on_mouse_down()`: Event handlers

## Active Technologies

- Rust 1.75+ with GPUI (Zed's GPU-accelerated UI framework)
- tokio-postgres, deadpool-postgres for PostgreSQL connectivity
- rusqlite for local SQLite metadata storage
- russh for SSH tunneling
- keyring for OS keychain integration
- thiserror for error handling
- tracing for structured logging
- parking_lot for synchronization primitives
- serde/serde_json for serialization
