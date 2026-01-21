# Tusk

A fast, free, native PostgreSQL client built with Rust and GPUI.

Tusk aims to be a complete replacement for pgAdmin and DBeaver for PostgreSQL workflows, with sub-second startup, minimal memory footprint, and native performance for large result sets.

## Status

**Early Development** - Backend services implemented. The application builds and runs on macOS, Windows, and Linux with connection pooling, local storage, credential management, and structured logging.

## Goals

- **Native Performance**: GPU-accelerated UI via GPUI, handles 1M+ rows efficiently
- **Cross-Platform**: macOS (Intel/Apple Silicon), Windows, Linux
- **PostgreSQL Only**: Deep integration, no multi-database abstraction overhead
- **Fully Local**: No cloud sync, no telemetry, no network calls except to your databases
- **Secure**: Credentials stored in OS keychain, never in config files

## Planned Features

- SQL editor with schema-aware autocomplete and syntax highlighting
- Virtualized results grid for large datasets
- Schema browser with search and DDL generation
- Query plan visualization (EXPLAIN ANALYZE)
- Inline data editing with transaction support
- Connection management with SSH tunneling and SSL/TLS
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

## Contributing

Contributions are welcome! Please read the design document before starting work on new features to ensure alignment with the project's architecture.

## License

[MIT](LICENSE)
