# Backend Architecture Contracts

This directory defines the public API contracts for the Tusk backend service layer.

Since Tusk is a pure Rust GPUI application, these are internal Rust APIs—there is no REST/GraphQL API.

## Organization

| File | Module | Requirements | Description |
|------|--------|--------------|-------------|
| [error.md](./error.md) | `tusk_core::error` | FR-001 through FR-004 | Error handling types |
| [state.md](./state.md) | `tusk_core::state` | FR-005 through FR-009 | Application state management |
| [connection.md](./connection.md) | `tusk_core::services::connection` | FR-010 through FR-013a | Connection pooling |
| [query.md](./query.md) | `tusk_core::services::query` | FR-014 through FR-016 | Query execution with cancellation |
| [credentials.md](./credentials.md) | `tusk_core::services::credentials` | FR-017 through FR-019a | OS keychain integration |
| [storage.md](./storage.md) | `tusk_core::services::storage` | FR-025 through FR-027a | Local SQLite storage |
| [logging.md](./logging.md) | `tusk_core::logging` | FR-022 through FR-024a | Structured logging setup |

## Usage

These contracts define the interface between:
- **UI layer**: `tusk` (binary), `tusk_ui` (components)
- **Core services**: `tusk_core` (services, models, error handling)

The implementation lives in `tusk_core`. UI components access services through `TuskState`, which implements `gpui::Global`.

## Key Types

### Error Handling
- `TuskError` - Main error enum with PostgreSQL-specific details
- `ErrorInfo` - User-displayable error for UI

### State Management
- `TuskState` - Central application state (implements `gpui::Global`)
- Thread-safe access via `parking_lot::RwLock`

### Connection Pooling
- `ConnectionConfig` - Connection configuration
- `ConnectionPool` - Managed connection pool (deadpool-postgres)
- `PoolStatus` - Pool health metrics

### Query Execution
- `QueryHandle` - Trackable, cancellable query
- `QueryResult` - Query execution results
- `QueryHistoryEntry` - History record

### Credentials
- `CredentialService` - OS keychain with session fallback

### Storage
- `LocalStorage` - SQLite for connections, history, preferences
