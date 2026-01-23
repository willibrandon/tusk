# Tusk

A fast, free, native PostgreSQL client built with Rust and GPUI.

Tusk aims to be a complete replacement for pgAdmin and DBeaver for PostgreSQL workflows, with sub-second startup, minimal memory footprint, and native performance for large result sets.

## Status

**Active Development** - Backend services and frontend architecture implemented. The application builds and runs on macOS, Windows, and Linux with a functional workspace UI, connection pooling, local storage, credential management, and structured logging.

## Goals

- **Native Performance**: GPU-accelerated UI via GPUI, handles 1M+ rows efficiently
- **Cross-Platform**: macOS (Intel/Apple Silicon), Windows, Linux
- **PostgreSQL Only**: Deep integration, no multi-database abstraction overhead
- **Fully Local**: No cloud sync, no telemetry, no network calls except to your databases
- **Secure**: Credentials stored in OS keychain, never in config files

## Current Features

- **Connection Management**: Connection dialog with form validation, connection pooling via deadpool-postgres
- **Query Execution**: Execute SQL with streaming results, cancellation support, and execution timing
- **Workspace Layout**: Resizable docks (schema browser, results panel) with persistent state
- **Tabbed Query Editors**: Multiple tabs with drag-and-drop reordering and dirty state tracking
- **Pane Splitting**: Split editors horizontally/vertically with keyboard navigation
- **Schema Browser**: Live database introspection showing schemas, tables, views, and functions
- **Results Panel**: Streaming data grid with text truncation, hover tooltips, and row counts
- **Error Handling**: 21 documented error scenarios with actionable hints and recovery suggestions
- **Toast Notifications**: Transient feedback for user actions (query cancelled, connection lost, etc.)
- **Native Menus**: Full application menu bar (File, Edit, View, Window, Help)
- **Cross-Platform Menus**: In-window application menu for Windows/Linux platforms
- **Keyboard Shortcuts**: Platform-specific shortcuts (Cmd on macOS, Ctrl on Windows/Linux)
- **Credential Storage**: Pluggable credential providers with OS keychain support
- **Theming**: Catppuccin color palette with consistent styling throughout

## Planned Features

- SQL editor with schema-aware autocomplete and syntax highlighting
- Query plan visualization (EXPLAIN ANALYZE)
- Inline data editing with transaction support
- Connection management UI with SSH tunneling and SSL/TLS
- Admin dashboard (activity monitor, table/index stats, locks)
- Import/export (CSV, JSON, pg_dump/pg_restore)
- ER diagram generation
- Role and extension management

See [docs/design.md](docs/design.md) for the complete design specification.

## Technology Stack

| Component | Technology |
|-----------|------------|
| Language | Rust 1.80+ |
| UI Framework | GPUI (from Zed) |
| PostgreSQL Driver | tokio-postgres |
| Connection Pooling | deadpool-postgres |
| SSH Tunneling | russh |
| Local Storage | rusqlite |
| Credentials | keyring (OS keychain) |

## Building

### Prerequisites

**All Platforms:**
- Rust 1.80 or later (`rustup update stable`)
- Git

**macOS:**
```bash
xcode-select --install
```

**Linux (Ubuntu/Debian):**
```bash
sudo apt install -y \
    libxkbcommon-dev \
    libxkbcommon-x11-dev \
    libwayland-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libxcb1-dev \
    libxcb-render0-dev \
    libxcb-shm0-dev \
    libvulkan-dev \
    libasound2-dev \
    libdbus-1-dev
```

**Windows:**
- Visual Studio Build Tools 2022 with "Desktop development with C++"

### Build and Run

```bash
git clone https://github.com/willibrandon/tusk.git
cd tusk
cargo build
cargo run
```

### Release Build

```bash
cargo build --release
./target/release/tusk
```

## Development

### Code Quality

```bash
# Format code
cargo fmt

# Run lints (warnings as errors)
cargo clippy -- -D warnings

# Run tests
cargo test
```

### Hot Reload

```bash
cargo install cargo-watch
cargo watch -x run
```

### Debug Logging

```bash
RUST_LOG=tusk=debug cargo run
```

## Project Structure

```
tusk/
├── crates/
│   ├── tusk/           # Main application binary
│   ├── tusk_core/      # Backend services, models, and state management
│   └── tusk_ui/        # UI components and theming
├── assets/
│   ├── fonts/          # JetBrains Mono font files
│   └── icons/          # Application icons (icns, ico)
├── docs/
│   ├── design.md       # Complete design specification
│   └── features/       # Feature implementation documents
└── specs/              # Implementation specifications
```

## Keyboard Shortcuts

| Action | macOS | Windows/Linux |
|--------|-------|---------------|
| New Query Tab | Cmd+N | Ctrl+N |
| Close Tab | Cmd+W | Ctrl+W |
| Run Query | Cmd+Enter | Ctrl+Enter |
| Toggle Schema Browser | Cmd+B | Ctrl+B |
| Toggle Results Panel | Cmd+J | Ctrl+J |
| Split Right | Cmd+\\ | Ctrl+\\ |
| Split Down | Cmd+\| | Ctrl+\| |
| Show All Shortcuts | Cmd+/ | Ctrl+/ |

See Help > Keyboard Shortcuts for the complete list.

## Contributing

Contributions are welcome! Please read the design document before starting work on new features to ensure alignment with the project's architecture.

## License

[MIT](LICENSE)
