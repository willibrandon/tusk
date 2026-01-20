# Quickstart: Backend Architecture

**Feature**: 002-backend-architecture
**Date**: 2026-01-19

## Overview

This document provides implementation guidance for the backend architecture feature. Follow this guide to implement the foundational Rust backend services.

---

## Prerequisites

1. **Completed 001-project-init**: Tauri v2 project structure with basic scaffolding
2. **Rust 1.75+**: Required for async features
3. **Dependencies**: All dependencies already in `Cargo.toml` from 001-project-init

### Add Missing Dependency

Add `tracing-appender` to `src-tauri/Cargo.toml`:

```toml
tracing-appender = "0.2"
```

---

## Implementation Order

Execute these phases in order:

### Phase 1: Error Handling Foundation

**Files**: `src-tauri/src/error.rs`, `src-tauri/src/models/error.rs`

1. Create `TuskError` enum with all variants
2. Implement `From` conversions for library errors
3. Add serialization for IPC transport
4. Export from `src-tauri/src/lib.rs`

**Verification**:
```bash
cd src-tauri && cargo build
```

---

### Phase 2: Application Directories

**Files**: `src-tauri/src/services/dirs.rs`

1. Create `AppDirs` struct with Tauri path resolver
2. Implement directory creation on first launch
3. Add error handling for permission failures

**Verification**:
```bash
cd src-tauri && cargo test dirs
```

---

### Phase 3: Logging Service

**Files**: `src-tauri/src/services/logging.rs`

1. Create `LogGuard` struct to hold worker guard
2. Implement `init_logging()` with daily rotation
3. Configure debug/release log levels
4. Integrate in `lib.rs` before `Builder::setup()`

**Verification**:
- Run app, check logs appear in `{app_data_dir}/logs/`
- Verify log level respects `RUST_LOG` environment variable

---

### Phase 4: Storage Service

**Files**: `src-tauri/src/services/storage.rs`, `src-tauri/src/models/connection.rs`

1. Create `StorageService` with SQLite connection
2. Implement `open_or_repair()` with corruption handling
3. Create migration system for schema versioning
4. Implement connection CRUD operations
5. Implement preference get/set operations

**Verification**:
```bash
cd src-tauri && cargo test storage
```

---

### Phase 5: Credential Service

**Files**: `src-tauri/src/services/credentials.rs`

1. Create `CredentialService` with keychain availability check
2. Implement password store/get/delete operations
3. Add error handling for keychain unavailability

**Verification**:
```bash
cd src-tauri && cargo test credentials
```

---

### Phase 6: Application State

**Files**: `src-tauri/src/state.rs`

1. Create `AppState` struct with all services
2. Implement `AppState::new()` initialization
3. Add connection pool management (HashMap)
4. Add active query tracking (HashMap)

**Verification**:
```bash
cd src-tauri && cargo build
```

---

### Phase 7: Connection Service

**Files**: `src-tauri/src/services/connection.rs`

1. Create connection pool factory
2. Implement SSL/TLS configuration
3. Add SSH tunnel support (basic structure, full implementation in later feature)
4. Implement connect/disconnect lifecycle

**Verification**:
```bash
cd src-tauri && cargo test connection
```

---

### Phase 8: Query Service

**Files**: `src-tauri/src/services/query.rs`

1. Create query execution with timeout
2. Implement query cancellation via `select!`
3. Add query tracking for active queries
4. Create query result serialization

**Verification**:
```bash
cd src-tauri && cargo test query
```

---

### Phase 9: IPC Commands

**Files**: `src-tauri/src/commands/*.rs`

1. Create `commands/app.rs` - health_check, get_log_directory
2. Create `commands/connection.rs` - list/get/save/delete/test/connect/disconnect
3. Create `commands/query.rs` - execute_query, cancel_query, get_active_queries
4. Create `commands/storage.rs` - check_database_health, get/set_preference
5. Register all commands in `lib.rs` invoke_handler

**Verification**:
```bash
cd src-tauri && cargo build
```

---

### Phase 10: Tauri Integration

**Files**: `src-tauri/src/lib.rs`

1. Initialize logging before Builder
2. Create AppState in `setup()`
3. Register all commands
4. Add graceful shutdown handling

**Verification**:
```bash
npm run tauri dev
# Frontend should be able to call health_check command
```

---

## File Structure After Implementation

```
src-tauri/src/
├── main.rs
├── lib.rs                  # Tauri setup, command registration
├── error.rs                # TuskError enum
├── state.rs                # AppState struct
├── commands/
│   ├── mod.rs
│   ├── app.rs              # health_check, get_log_directory
│   ├── connection.rs       # Connection management commands
│   ├── query.rs            # Query execution commands
│   └── storage.rs          # Storage/preference commands
├── services/
│   ├── mod.rs
│   ├── dirs.rs             # AppDirs, directory creation
│   ├── logging.rs          # File-based logging setup
│   ├── storage.rs          # SQLite operations
│   ├── credentials.rs      # OS keychain operations
│   ├── connection.rs       # Connection pool management
│   └── query.rs            # Query execution, cancellation
└── models/
    ├── mod.rs
    ├── app.rs              # AppInfo
    ├── connection.rs       # ConnectionConfig, SshTunnel
    ├── query.rs            # Query, QueryResult, Column
    └── error.rs            # ErrorResponse (for frontend)
```

---

## Testing Strategy

### Unit Tests

Each service module should have unit tests:

```rust
// src-tauri/src/services/storage.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_storage_init() {
        let dir = tempdir().unwrap();
        let service = StorageService::new(dir.path()).await.unwrap();
        assert!(service.check_integrity().await.unwrap().is_healthy);
    }

    #[tokio::test]
    async fn test_connection_crud() {
        let dir = tempdir().unwrap();
        let service = StorageService::new(dir.path()).await.unwrap();

        let config = ConnectionConfig { /* ... */ };
        service.save_connection(&config).await.unwrap();

        let loaded = service.get_connection(&config.id).await.unwrap();
        assert_eq!(loaded.unwrap().name, config.name);

        service.delete_connection(&config.id).await.unwrap();
        assert!(service.get_connection(&config.id).await.unwrap().is_none());
    }
}
```

### Integration Tests

Use Tauri MCP to test IPC commands:

```typescript
// Test: health_check returns valid response
const info = await invoke<AppInfo>("health_check");
expect(info.name).toBe("tusk");
expect(info.platform).toMatch(/macos|windows|linux/);
```

---

## Common Pitfalls

1. **Forgetting LogGuard**: Store `LogGuard` for app lifetime or logs won't flush
2. **SQLite busy timeout**: Set `PRAGMA busy_timeout = 5000` to handle concurrent access
3. **Keychain on Linux**: Ensure `dbus` and `libsecret` are available
4. **RwLock deadlocks**: Never hold read lock while trying to acquire write lock
5. **Query cancellation race**: `cancel_query()` is best-effort; query may complete first

---

## Success Criteria Verification

| Criterion | How to Verify |
|-----------|---------------|
| SC-001: Cold start < 1s | Time `npm run tauri dev` to first window |
| SC-002: Memory < 100MB idle | Monitor with Activity Monitor / Task Manager |
| SC-003: Cancel < 2s | Execute `SELECT pg_sleep(10)`, cancel, measure |
| SC-004: Actionable errors | Trigger connection failures, verify hints |
| SC-005: Persistence | Save connection, restart app, verify present |
| SC-006: Health check | Call `health_check` command, verify response |
| SC-007: No memory leaks | Connect/disconnect repeatedly, monitor memory |
| SC-008: No plaintext creds | Search app data directory for passwords |
| SC-009: Accessible logs | Find and read log files in app data directory |
