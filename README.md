# Tusk

A fast, free, native PostgreSQL client built with Tauri.

Tusk aims to replace pgAdmin and DBeaver for Postgres-only workflows with sub-second startup, minimal memory usage, and native performance for large datasets.

## Features

- **Query Editor** — Monaco-based SQL editor with autocomplete, syntax highlighting, and multi-tab support
- **Results Grid** — Virtual scrolling handles 1M+ rows without breaking a sweat
- **Schema Browser** — Tree view with tables, views, functions, and full introspection
- **Table Data Viewer** — Browse, filter, sort, and inline-edit table data
- **Query Plan Visualization** — EXPLAIN output rendered as interactive tree/timeline
- **ER Diagrams** — Generate entity-relationship diagrams from your schema
- **Admin Dashboard** — Monitor connections, locks, table stats, and server health
- **Backup/Restore** — Wrap pg_dump and pg_restore with a friendly UI
- **Import Wizard** — Import CSV/JSON with column mapping and type detection
- **SSH Tunnels** — Connect through bastion hosts with key-based auth
- **OS Keychain** — Passwords stored securely in macOS Keychain, Windows Credential Manager, or Secret Service

## Tech Stack

**Frontend**

- Svelte 5 with runes
- Monaco Editor
- TanStack Table + virtual scrolling
- @xyflow/svelte for diagrams
- Tailwind CSS

**Backend (Rust)**

- Tauri v2
- tokio-postgres (async, streaming, COPY protocol)
- deadpool-postgres (connection pooling)
- russh (SSH tunnels)
- rusqlite (local metadata storage)
- keyring (OS credential storage)

## Requirements

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) 1.70+
- Platform-specific dependencies for Tauri (see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))

## Development

```bash
# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev
```

## Building

```bash
# Build for production
npm run tauri build
```

Outputs are in `src-tauri/target/release/bundle/`:

- **macOS**: `.dmg` and `.app`
- **Windows**: `.msi` and `.exe`
- **Linux**: `.AppImage`, `.deb`, `.rpm`

## Project Structure

```
tusk/
├── src/                    # Svelte frontend
│   ├── lib/
│   │   ├── components/     # UI components
│   │   ├── stores/         # Svelte stores
│   │   └── services/       # IPC wrappers
│   └── routes/             # Pages
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── commands/       # Tauri IPC commands
│   │   ├── services/       # Business logic
│   │   └── models/         # Data structures
│   └── Cargo.toml
├── docs/
│   ├── design.md           # Full design specification
│   └── features/           # Implementation guides (01-29)
└── package.json
```

## Documentation

- [Design Document](docs/design.md) — Architecture, data models, and UI specifications
- [Feature Index](docs/features/00-feature-index.md) — Sequential implementation guides

## Performance Targets

| Metric                    | Target     |
| ------------------------- | ---------- |
| Cold start                | < 1 second |
| Memory (idle)             | < 100 MB   |
| Memory (1M rows loaded)   | < 500 MB   |
| Render 1000 rows          | < 100ms    |
| Schema load (1000 tables) | < 500ms    |
| Autocomplete response     | < 50ms     |

## License

MIT
