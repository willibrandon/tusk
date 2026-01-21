# Research: Backend Architecture

**Feature**: 002-backend-architecture
**Date**: 2026-01-20
**Purpose**: Resolve technical unknowns and establish best practices for backend implementation

---

## 1. Connection Pooling (deadpool-postgres + tokio-postgres)

### Decision: Conservative Pool Configuration for Desktop

**Configuration:**
- Pool size: 4 connections (max 8 for heavy workloads)
- Wait timeout: 5 seconds (then return pool timeout error per FR-013a)
- Create timeout: 10 seconds
- Recycle timeout: 5 seconds
- Recycling method: `RecyclingMethod::Fast` (adequate for stable networks)

**Rationale:**
- Desktop applications have single-user workloads, not server-scale concurrency
- Each PostgreSQL connection uses 1.3-10 MB of server RAM
- Small pool prevents resource exhaustion while maintaining responsiveness

**Alternatives Considered:**
- Larger pool (16-32): Rejected—server-appropriate but wasteful for single-user
- Single connection: Rejected—blocks UI during long queries
- Connection-per-query: Rejected—1-3ms overhead per connection unacceptable

### Decision: TCP Keepalive for Connection Health

**Configuration:**
```rust
pg_config.keepalives(true);
pg_config.keepalives_idle(Duration::from_secs(60));
pg_config.keepalives_interval(Duration::from_secs(15));
pg_config.keepalives_retries(3);
```

**Rationale:** Passive detection of dead connections without application-level polling. Desktop apps may have extended idle periods.

### Decision: Startup Validation

Explicitly test connection at pool creation with `pool.get().await` followed by `SELECT 1`. Pool creation is lazy in deadpool—explicit validation surfaces connection issues immediately with clear error messages.

### Decision: Session Defaults via post_create Hook

Use `post_create` hook to set:
- `statement_timeout = '30s'`
- `idle_in_transaction_session_timeout = '60s'`

### Decision: GPUI-Tokio Bridge Pattern

Follow Zed's `gpui_tokio` crate pattern:
1. Initialize dedicated Tokio runtime as GPUI Global
2. Spawn database operations on Tokio runtime
3. Bridge results back to GPUI via Task<T>
4. Cancel Tokio task if GPUI task is dropped

**Source:** Zed's `/Users/brandon/src/zed/crates/gpui_tokio/src/gpui_tokio.rs`

---

## 2. Credential Storage (keyring)

### Decision: Service Name and Username Pattern

- **Service name**: `dev.tusk.Tusk` (reverse domain notation)
- **Username format**: `db:{connection_id}` for database passwords, `ssh:{connection_id}` for SSH passphrases

**Rationale:** Follows platform conventions, avoids conflicts, enables multiple credential types per connection.

### Decision: Platform Feature Flags

```toml
keyring = { version = "3.6", features = [
    "apple-native",           # macOS Keychain
    "windows-native",         # Windows Credential Manager
    "sync-secret-service",    # Linux Secret Service (D-Bus, sync)
] }
```

**Critical:** Avoid `async-secret-service` feature—documented deadlock issues with Tokio runtime.

### Decision: Serialize Keyring Access

Access the same credential from multiple threads can fail, especially on Windows and Linux. Options:
1. Serialize through dedicated thread (safest)
2. Ensure single-context access (simpler if architecture allows)

### Decision: Graceful Degradation (FR-019a)

When keychain unavailable:
1. Detect availability at startup via test set/delete
2. Warn user with clear message
3. Use in-memory session storage (cleared on app exit)
4. Never store passwords in plaintext files

**Error Handling:** Map all `keyring::Error` variants including wildcard (enum is non-exhaustive).

---

## 3. Local Storage (rusqlite)

### Decision: Thread-Local Reads, Serialized Writes

Follow Zed's `sqlez` pattern:
- Thread-local connections for reads (no mutex contention)
- Single background thread for writes (serializes, prevents WAL contention)
- Connections marked read-only by default

**Rationale:** UI applications are read-heavy; this pattern eliminates contention while ensuring write integrity.

### Decision: WAL Mode with Optimized PRAGMAs

```sql
PRAGMA journal_mode = WAL;      -- Concurrent reads during writes
PRAGMA synchronous = NORMAL;    -- Good durability/performance balance
PRAGMA busy_timeout = 5000;     -- Wait 5s for locks
PRAGMA cache_size = -64000;     -- 64MB page cache
PRAGMA foreign_keys = ON;       -- Enforce referential integrity
PRAGMA temp_store = MEMORY;     -- Temp tables in RAM
```

### Decision: Domain-Based Migration System

Adopt Zed's migration pattern:
- Store migration text in database
- Run migrations in savepoint (failed migrations roll back)
- Validate existing migrations haven't changed
- Panic on migration mismatch (prevents data corruption)

### Decision: Recovery Strategy

1. Attempt normal open
2. On failure: backup corrupted DB with timestamp, recreate fresh
3. Final fallback: in-memory database
4. Notify user of data reset

**Schema:** STRICT tables, explicit primary keys, UUIDs as TEXT for connections/queries, no credential storage (keychain only).

---

## 4. Logging (tracing + tracing-subscriber)

### Decision: Non-Blocking File Logging

Use `tracing_appender::non_blocking` for file writes:
- Offloads I/O to background thread
- WorkerGuard ensures flush on shutdown
- Critical for UI responsiveness

```rust
let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
// Guard must live until main() exits
```

### Decision: Daily Log Rotation

Use `RollingFileAppender::builder()` with `Rotation::DAILY`:
- Files named: `tusk.2024-01-15.log`
- Appropriate for desktop apps (not too many files, manageable size)

### Decision: Conditional Log Levels

```rust
#[cfg(debug_assertions)]
{ "debug,tusk=trace,tokio_postgres=warn" }

#[cfg(not(debug_assertions))]
{ "info,tusk=info,tokio_postgres=warn" }
```

Override via `TUSK_LOG` or `RUST_LOG` environment variables.

### Decision: Graceful Fallback (FR-024a)

If file logging fails:
1. Log error to stderr
2. Continue with stdout-only logging
3. Don't block application startup

**Implementation:** Try file+stdout, on error fallback to stdout-only.

---

## 5. Error Handling Patterns

### Decision: Comprehensive Error Mapping

Map all external error types to `TuskError` with:
- Error category (connection, authentication, SSL, SSH, query, storage, keyring, internal)
- User-friendly message
- Actionable hint
- PostgreSQL error code (when applicable)
- Position in query (for syntax errors)

### Decision: PostgreSQL Error Code Hints

| Code | Category | Hint |
|------|----------|------|
| 28P01 | Authentication | "Invalid password—check your credentials" |
| 28000 | Authentication | "Authentication failed—check username and permissions" |
| 3D000 | Connection | "Database does not exist—verify database name" |
| 08xxx | Connection | "Connection exception—check network connectivity" |
| 42xxx | Query | Include position in query if available |

---

## 6. Async Execution Pattern

### Decision: GPUI BackgroundExecutor + Tokio Runtime

1. Store Tokio runtime as GPUI Global
2. Database operations run on Tokio worker threads
3. Results bridged back to GPUI main thread via `cx.update()`
4. All database futures must be `Send + 'static`

**Architecture:**
```
GPUI Main Thread (UI rendering)
    │
    ├── cx.spawn() / cx.background_spawn()
    │
    ▼
Tokio Runtime (2-4 worker threads)
    │
    ├── deadpool-postgres Pool
    └── Query execution, connection management
```

---

## Summary of Technology Decisions

| Area | Decision | Key Dependency |
|------|----------|----------------|
| PostgreSQL Driver | tokio-postgres + deadpool-postgres | `deadpool-postgres = "0.14"` |
| Async Runtime | Tokio multi-threaded | `tokio = { version = "1", features = ["full"] }` |
| Local Storage | rusqlite with thread-local pattern | `rusqlite = { version = "0.37", features = ["bundled"] }` |
| Credentials | keyring with platform features | `keyring = { version = "3.6", features = [...] }` |
| Logging | tracing with non-blocking file appender | `tracing-appender = "0.2"` |
| Error Types | thiserror with PostgreSQL mappings | `thiserror = "2.0"` (workspace) |
| Identifiers | UUID v4 for connections/queries | `uuid = { version = "1.0", features = ["v4"] }` |
| Data Directory | dirs crate for OS paths | `dirs = "5.0"` |

---

## Sources

- [deadpool-postgres documentation](https://docs.rs/deadpool-postgres/)
- [tokio-postgres Config](https://docs.rs/tokio-postgres/latest/tokio_postgres/config/struct.Config.html)
- [keyring crate documentation](https://docs.rs/keyring/latest/keyring/)
- [tracing-appender documentation](https://docs.rs/tracing-appender/)
- [tracing-subscriber fmt module](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/)
- Zed codebase: `/Users/brandon/src/zed/crates/gpui_tokio/` (async bridge)
- Zed codebase: `/Users/brandon/src/zed/crates/sqlez/` (SQLite patterns)
- Zed codebase: `/Users/brandon/src/zed/crates/zlog/` (logging patterns)
