# Implementation Plan: Service Integration Layer

**Branch**: `004-service-integration` | **Date**: 2026-01-21 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/004-service-integration/spec.md`

## Summary

This feature establishes the integration layer between Tusk's GPUI-based UI components and backend services. As a unified Rust application with no IPC layer, UI components directly call services via `cx.global::<TuskState>()`, spawn background tasks using the Tokio runtime, and receive streaming results through channels. The implementation provides consistent error handling, query cancellation support, and responsive UI feedback during long-running database operations.

## Technical Context

**Language/Version**: Rust 1.80+ with 2021 edition
**Primary Dependencies**: GPUI (from Zed), tokio-postgres 0.7, deadpool-postgres 0.14, tokio 1.x, parking_lot 0.12
**Storage**: PostgreSQL (target databases), SQLite via rusqlite 0.32 (local metadata)
**Testing**: cargo test, GPUI test harness for UI components
**Target Platform**: macOS, Windows, Linux (native desktop)
**Project Type**: Single Rust workspace with multiple crates (tusk, tusk_core, tusk_ui)
**Performance Goals**: UI responsive within 100ms, first batch results within 500ms, schema load within 300ms
**Constraints**: Memory < 400MB for 1M rows, no main thread blocking, streaming for large datasets
**Scale/Scope**: Connection pools with 2-3 connections per server, support 10 concurrent queries, 1M+ row result sets

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

### Principle I: Postgres Exclusivity ✅ PASS
- Feature targets PostgreSQL exclusively
- Uses tokio-postgres for Postgres-specific error codes, positions, hints
- No generic SQL abstractions

### Principle II: Complete Local Privacy ✅ PASS
- No network calls except to user-configured PostgreSQL servers
- All data remains local (SQLite for metadata, OS keychain for credentials)

### Principle III: OS Keychain for Credentials ✅ PASS
- FR-028: Passwords stored in OS keychain, never in files
- FR-029: Session-only fallback when keychain unavailable
- FR-026: Credentials never logged

### Principle IV: Complete Implementation ✅ WILL COMPLY
- All 29 functional requirements will be implemented
- All 6 user stories with acceptance scenarios
- No placeholders or TODOs

### Principle V: Task Immutability ✅ WILL COMPLY
- Tasks created will be immutable
- No removal, merger, or renumbering

### Principle VI: Performance Discipline ✅ PASS
- SC-001: UI responsive within 100ms (async operations, non-blocking)
- SC-002: First batch within 500ms (streaming with 1000-row batches)
- SC-004: Schema load within 300ms
- SC-007: 1M+ rows without memory exhaustion (streaming, not buffering)

### Security Requirements ✅ PASS
- FR-026: Never log passwords or credentials
- Parameterized queries via tokio-postgres
- Connection validation before pool addition

**NON-NEGOTIABLE Principles (automatic failure if violated):**

- Principle IV: Complete Implementation — No placeholders, TODOs, "future work", or scope reduction
- Principle V: Task Immutability — Tasks MUST NEVER be removed, merged, renumbered, or reduced in scope

## Project Structure

### Documentation (this feature)

```text
specs/004-service-integration/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (internal Rust contracts, not REST)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
crates/
├── tusk/                      # Main application binary
│   ├── src/
│   │   ├── main.rs            # App initialization, TuskState registration
│   │   ├── app.rs             # TuskApp root component
│   │   └── app_menus.rs       # Native menu bar
│   └── Cargo.toml
│
├── tusk_core/                 # Core backend services & types
│   ├── src/
│   │   ├── lib.rs
│   │   ├── error.rs           # TuskError, ErrorInfo
│   │   ├── logging.rs         # Structured logging
│   │   ├── state.rs           # TuskState (Global trait)
│   │   ├── models/
│   │   │   ├── connection.rs  # ConnectionConfig, ConnectionStatus
│   │   │   ├── query.rs       # QueryHandle, QueryResult, QueryEvent
│   │   │   ├── schema.rs      # DatabaseSchema, SchemaCache
│   │   │   └── history.rs     # QueryHistoryEntry
│   │   └── services/
│   │       ├── connection.rs  # ConnectionPool, PooledConnection
│   │       ├── query.rs       # QueryService (streaming, cancellation)
│   │       ├── schema.rs      # SchemaService (introspection, caching)
│   │       ├── credentials.rs # CredentialService (keychain)
│   │       └── storage.rs     # LocalStorage (SQLite)
│   └── Cargo.toml
│
└── tusk_ui/                   # GPUI UI components
    ├── src/
    │   ├── lib.rs
    │   ├── theme.rs           # TuskTheme, ThemeColors
    │   ├── workspace.rs       # Workspace (root), WorkspaceState
    │   ├── pane.rs            # Pane, tab management
    │   ├── dock.rs            # Dock (left, right, bottom)
    │   ├── panel.rs           # Panel abstraction
    │   ├── panels/
    │   │   ├── schema_browser.rs   # Schema tree view
    │   │   ├── results.rs          # Query results grid
    │   │   └── messages.rs         # Messages/log panel
    │   ├── query_editor.rs    # SQL editor component (integration point)
    │   ├── connection_dialog.rs    # Connection form
    │   ├── toast.rs           # Toast notifications
    │   └── error_panel.rs     # Error display panel
    └── Cargo.toml
```

**Structure Decision**: Existing multi-crate workspace structure. Feature 04 adds integration code across all three crates, primarily in tusk_ui components that call tusk_core services.

## Post-Design Constitution Re-Check

_Re-evaluated after Phase 1 design completion._

### Principle I: Postgres Exclusivity ✅ CONFIRMED
- data-model.md: All entities are PostgreSQL-specific (TuskError::Query includes pg error codes)
- contracts/: All APIs designed for PostgreSQL operations
- No generic database abstractions introduced

### Principle II: Complete Local Privacy ✅ CONFIRMED
- No external network calls in any contract
- Credentials stored locally via CredentialService

### Principle III: OS Keychain for Credentials ✅ CONFIRMED
- state-api.md: store_password/get_password use CredentialService
- Session fallback documented for keychain unavailability

### Principle IV: Complete Implementation ✅ DESIGN READY
- All 29 FRs mapped to contracts
- All 6 user stories have implementation patterns in quickstart.md
- No placeholders in design artifacts

### Principle V: Task Immutability ✅ READY
- Tasks not yet created (Phase 2: /speckit.tasks)
- Will be enforced during task generation

### Principle VI: Performance Discipline ✅ CONFIRMED
- Streaming design supports 1M+ rows (query-events.md)
- Async patterns prevent UI blocking (quickstart.md)
- Schema caching with TTL (data-model.md)

### Security Requirements ✅ CONFIRMED
- error-handling.md: FR-026 explicitly prohibits logging credentials
- state-api.md: Passwords never stored in TuskState

**Status: GATE PASSED — Ready for /speckit.tasks**

## Complexity Tracking

No constitution violations. No complexity justification needed.
