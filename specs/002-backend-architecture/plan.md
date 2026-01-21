# Implementation Plan: Backend Architecture

**Branch**: `002-backend-architecture` | **Date**: 2026-01-20 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-backend-architecture/spec.md`

## Summary

Establish the Rust backend service layer for Tusk, a pure GPUI PostgreSQL client. This feature implements comprehensive error handling with PostgreSQL-specific details, centralized application state management, connection pooling with deadpool-postgres, query execution with cancellation support, secure credential storage via OS keychain, async background execution, and structured logging. The entire application is Rust code—no frontend/backend split—so this focuses on the service layer, data access patterns, and async execution that power the UI.

## Technical Context

**Language/Version**: Rust 1.80+ with 2021 edition (established in workspace Cargo.toml)
**Primary Dependencies**:
- GPUI (from Zed repository, rev 89e9ab97aa5d978351ee8a28d9cc35c272c530f5) - UI framework
- tokio-postgres - async PostgreSQL driver
- deadpool-postgres - connection pooling
- rusqlite - local metadata storage (SQLite)
- keyring - OS keychain integration
- thiserror - error types (already in workspace)
- tracing/tracing-subscriber - structured logging (already in workspace)
- parking_lot - thread-safe synchronization (already in workspace)
- serde/serde_json - serialization (already in workspace)
- uuid - unique identifiers for connections and queries
- tokio - async runtime (required by tokio-postgres)
- dirs - OS-appropriate data directory paths

**Storage**:
- SQLite via rusqlite for local metadata (saved connections, query history, preferences)
- OS Keychain via keyring for credential storage
- PostgreSQL connections managed via deadpool-postgres pools

**Testing**: cargo test with unit tests and integration tests against test PostgreSQL database
**Target Platform**: macOS (primary), Windows, Linux - native desktop via GPUI
**Project Type**: Single Rust workspace with multiple crates
**Performance Goals**:
- Application state initialization < 100ms
- Connection pool creation within configured timeout (default 10s)
- Query cancellation propagation < 50ms
- Password operations < 500ms
- Log write latency < 100ms
- Handle 10 concurrent database operations without state corruption

**Constraints**:
- Passwords MUST use OS keychain, NEVER plaintext storage
- No network calls except to user-configured PostgreSQL servers
- UI thread MUST NOT block during database operations
- Must maintain accurate pool status at all times

**Scale/Scope**:
- 7 user stories across error handling, state management, pooling, query execution, credentials, async, and logging
- 31 functional requirements (27 original + 4 clarification-driven)
- 10 measurable success criteria

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

### Pre-Design Check ✅

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Postgres Exclusivity | ✅ PASS | Feature is Postgres-specific: uses tokio-postgres, PostgreSQL error codes, pg-specific connection pooling |
| II. Complete Local Privacy | ✅ PASS | No external network calls. Local SQLite for metadata, OS keychain for credentials |
| III. OS Keychain for Credentials | ✅ PASS | FR-017 requires OS keychain. FR-019a provides session fallback when denied |
| IV. Complete Implementation | ✅ PASS | All 31 FRs will be implemented. No placeholders or TODOs planned |
| V. Task Immutability | ✅ PASS | Will honor once tasks.md is generated |
| VI. Performance Discipline | ✅ PASS | SC-002 through SC-010 define specific timing/performance requirements |

### Post-Design Re-Evaluation ✅

| Principle | Status | Verification |
|-----------|--------|--------------|
| I. Postgres Exclusivity | ✅ PASS | Design uses tokio-postgres exclusively. TuskError includes PostgreSQL-specific error codes (23505, 42P01, etc.). ConnectionConfig supports pg-specific SSL modes. No generic database abstraction. |
| II. Complete Local Privacy | ✅ PASS | research.md confirms no external calls. LocalStorage uses rusqlite (SQLite). CredentialService uses OS keychain. Data directory is local (`~/Library/Application Support/dev.tusk.Tusk` on macOS). |
| III. OS Keychain for Credentials | ✅ PASS | credentials.md contract specifies keyring crate with platform backends (apple-native, windows-native, sync-secret-service). Session fallback uses in-memory HashMap, never files. |
| IV. Complete Implementation | ✅ PASS | All 31 FRs covered in contracts: error.md (FR-001–004), state.md (FR-005–009), connection.md (FR-010–013a), query.md (FR-014–016), credentials.md (FR-017–019a), logging.md (FR-022–024a), storage.md (FR-025–027a). |
| V. Task Immutability | ✅ PASS | Ready for tasks.md generation. Constitution rules embedded in design documents. |
| VI. Performance Discipline | ✅ PASS | SC targets embedded in contracts: state init <100ms (state.md), pool creation within timeout (connection.md), cancel <50ms (query.md), password ops <500ms (credentials.md), log writes <100ms (logging.md). |

**NON-NEGOTIABLE Principles (automatic failure if violated):**

- Principle IV: Complete Implementation — No placeholders, TODOs, "future work", or scope reduction
- Principle V: Task Immutability — Tasks MUST NEVER be removed, merged, renumbered, or reduced in scope

**Security Requirements Compliance:**

- Credential Handling: FR-018 (never log passwords), parameterized queries via tokio-postgres
- Connection Security: FR-012 includes SSL mode configuration

## Project Structure

### Documentation (this feature)

```text
specs/002-backend-architecture/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (internal Rust APIs)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
crates/
├── tusk/                    # Main application binary
│   ├── src/
│   │   ├── main.rs          # Application entry point (exists)
│   │   └── app.rs           # TuskApp component (exists)
│   └── Cargo.toml
├── tusk_core/               # Core services and types (expand)
│   ├── src/
│   │   ├── lib.rs           # Module exports (exists)
│   │   ├── error.rs         # TuskError types (expand)
│   │   ├── state.rs         # TuskState global state (new)
│   │   ├── services/        # Service layer (new)
│   │   │   ├── mod.rs
│   │   │   ├── connection.rs    # ConnectionPool management
│   │   │   ├── query.rs         # Query execution + cancellation
│   │   │   ├── storage.rs       # Local SQLite storage
│   │   │   └── credentials.rs   # OS keychain integration
│   │   ├── models/          # Data structures (new)
│   │   │   ├── mod.rs
│   │   │   ├── connection.rs    # ConnectionConfig, PoolStatus
│   │   │   ├── query.rs         # QueryHandle, QueryResult
│   │   │   └── history.rs       # QueryHistoryEntry
│   │   └── logging.rs       # Logging setup (new)
│   └── Cargo.toml
└── tusk_ui/                 # UI components (exists, not modified)
    ├── src/
    │   ├── lib.rs
    │   ├── icons.rs
    │   └── theme.rs
    └── Cargo.toml

tests/
├── integration/
│   ├── error_handling_tests.rs
│   ├── state_tests.rs
│   ├── connection_pool_tests.rs
│   ├── query_execution_tests.rs
│   ├── credential_tests.rs
│   └── logging_tests.rs
└── README.md
```

**Structure Decision**: Expand existing `tusk_core` crate with services/, models/, and new top-level modules. The workspace structure is already established with tusk (binary), tusk_core (library), and tusk_ui (library). This feature adds backend infrastructure to tusk_core.

## Complexity Tracking

No violations to justify. The design uses the existing crate structure and standard Rust patterns.
