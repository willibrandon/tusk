# Tasks: Backend Architecture

**Input**: Design documents from `/specs/002-backend-architecture/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Not explicitly requested in spec - test tasks omitted per template guidelines.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Based on plan.md structure:
- **Core services**: `crates/tusk_core/src/`
- **Main application**: `crates/tusk/src/`

---

## Phase 1: Setup (Dependencies & Project Structure)

**Purpose**: Add new dependencies and create module structure

- [X] T001 Add new dependencies to crates/tusk_core/Cargo.toml (tokio, tokio-postgres, deadpool-postgres, rusqlite, keyring, uuid, chrono, dirs, tracing-appender, tokio-util)
- [X] T002 [P] Create services module structure in crates/tusk_core/src/services/mod.rs
- [X] T003 [P] Create models module structure in crates/tusk_core/src/models/mod.rs
- [X] T004 Update crates/tusk_core/src/lib.rs to export new modules (services, models, state, logging)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T005 Implement data directory initialization with OS-appropriate paths in crates/tusk_core/src/services/storage.rs (FR-025, FR-026, FR-027, FR-027a)
- [X] T006 Implement SQLite database schema and migrations in crates/tusk_core/src/services/storage.rs
- [X] T007 Implement LocalStorage struct with thread-safe connection handling in crates/tusk_core/src/services/storage.rs
- [X] T008 Implement connection CRUD operations (save, load, load_all, delete, update_last_connected) in crates/tusk_core/src/services/storage.rs
- [X] T009 Implement SSH tunnel CRUD operations in crates/tusk_core/src/services/storage.rs
- [X] T010 Implement query history operations (add, load, search, clear) in crates/tusk_core/src/services/storage.rs
- [X] T011 Implement saved queries operations in crates/tusk_core/src/services/storage.rs
- [X] T012 Implement UI state persistence operations in crates/tusk_core/src/services/storage.rs
- [X] T013 Implement default_data_dir() and init_data_dir() helper functions in crates/tusk_core/src/services/storage.rs

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Robust Error Handling (Priority: P1) 🎯 MVP

**Goal**: Comprehensive error handling system that captures detailed error information for debugging and user feedback

**Independent Test**: Trigger various error conditions (connection failures, query errors, storage errors) and verify appropriate error details are captured, categorized, and convertible to user-friendly messages

### Implementation for User Story 1

- [ ] T014 [US1] Expand TuskError enum with all variants (Connection, Authentication, Ssl, Ssh, Query, Storage, Keyring, PoolTimeout, Internal) in crates/tusk_core/src/error.rs
- [ ] T015 [US1] Implement Query variant with PostgreSQL-specific fields (message, detail, hint, position, code) in crates/tusk_core/src/error.rs (FR-002)
- [ ] T016 [US1] Implement TuskError constructors (connection, authentication, query, storage, keyring, pool_timeout, internal) in crates/tusk_core/src/error.rs
- [ ] T017 [US1] Implement TuskError methods (category, hint, pg_code, position) in crates/tusk_core/src/error.rs
- [ ] T018 [US1] Implement ErrorInfo struct for UI display in crates/tusk_core/src/error.rs (FR-003)
- [ ] T019 [US1] Implement TuskError::to_error_info() conversion method in crates/tusk_core/src/error.rs (FR-003)
- [ ] T020 [US1] Implement From<tokio_postgres::Error> for TuskError with PostgreSQL error code mapping in crates/tusk_core/src/error.rs (FR-004)
- [ ] T021 [P] [US1] Implement From<rusqlite::Error> for TuskError in crates/tusk_core/src/error.rs (FR-004)
- [ ] T022 [P] [US1] Implement From<std::io::Error> for TuskError in crates/tusk_core/src/error.rs (FR-004)
- [ ] T023 [P] [US1] Implement From<serde_json::Error> for TuskError in crates/tusk_core/src/error.rs (FR-004)
- [ ] T099 [P] [US1] Implement From<keyring::Error> for TuskError in crates/tusk_core/src/error.rs (FR-004)

**Checkpoint**: User Story 1 complete - error handling is fully functional and independently testable

---

## Phase 4: User Story 2 - Application State Management (Priority: P1)

**Goal**: Centralized state management for connections, queries, and caches accessible from any component

**Independent Test**: Create connections, run queries, verify state is accessible, updatable, and consistent across simulated component access

### Implementation for User Story 2

- [ ] T024 [US2] Create TuskState struct with all fields (connections, schema_caches, active_queries, storage, data_dir, credential_service, tokio_runtime) in crates/tusk_core/src/state.rs (FR-005)
- [ ] T025 [US2] Implement TuskState::new() with data directory initialization and runtime creation in crates/tusk_core/src/state.rs (SC-002)
- [ ] T026 [US2] Implement TuskState::with_data_dir() for custom data directory (testing) in crates/tusk_core/src/state.rs
- [ ] T027 [US2] Implement connection management methods (add_connection, get_connection, remove_connection, connection_ids, all_pool_statuses) in crates/tusk_core/src/state.rs (FR-006)
- [ ] T028 [US2] Implement schema cache management methods (get_schema_cache, set_schema_cache, remove_schema_cache) in crates/tusk_core/src/state.rs (FR-007)
- [ ] T029 [US2] Implement connection removal invariant: removing connection also removes schema cache in crates/tusk_core/src/state.rs
- [ ] T030 [US2] Implement query tracking methods (register_query, get_query, unregister_query, cancel_query, active_query_ids) in crates/tusk_core/src/state.rs (FR-008)
- [ ] T031 [US2] Implement storage and credentials accessor methods in crates/tusk_core/src/state.rs
- [ ] T032 [US2] Implement gpui::Global for TuskState in crates/tusk_core/src/state.rs
- [ ] T033 [US2] Implement SchemaCache placeholder struct in crates/tusk_core/src/state.rs (FR-007)

**Checkpoint**: User Story 2 complete - state management is fully functional and independently testable

---

## Phase 5: User Story 3 - Database Connection Pooling (Priority: P1)

**Goal**: Efficient connection pooling with deadpool-postgres for reliable database connectivity

**Independent Test**: Create a connection pool, acquire connections, execute queries, verify pool status (size, available, waiting)

### Implementation for User Story 3

- [ ] T034 [P] [US3] Create ConnectionConfig struct with all fields in crates/tusk_core/src/models/connection.rs (FR-012)
- [ ] T035 [P] [US3] Create SslMode enum in crates/tusk_core/src/models/connection.rs
- [ ] T036 [P] [US3] Create SshTunnelConfig struct in crates/tusk_core/src/models/connection.rs
- [ ] T037 [P] [US3] Create SshAuthMethod enum in crates/tusk_core/src/models/connection.rs
- [ ] T038 [P] [US3] Create ConnectionOptions struct with defaults in crates/tusk_core/src/models/connection.rs
- [ ] T039 [US3] Implement ConnectionConfig constructors (new, builder) and validate() method in crates/tusk_core/src/models/connection.rs
- [ ] T040 [US3] Implement ConnectionConfigBuilder for complex configurations in crates/tusk_core/src/models/connection.rs
- [ ] T041 [P] [US3] Create PoolStatus struct in crates/tusk_core/src/models/connection.rs (FR-013)
- [ ] T042 [US3] Create ConnectionPool struct with deadpool-postgres integration in crates/tusk_core/src/services/connection.rs (FR-010)
- [ ] T043 [US3] Implement ConnectionPool::new() with connection validation on creation in crates/tusk_core/src/services/connection.rs (FR-011, SC-003)
- [ ] T044 [US3] Implement ConnectionPool::with_pool_config() for custom pool settings in crates/tusk_core/src/services/connection.rs
- [ ] T045 [US3] Configure TCP keepalive settings for connection health in crates/tusk_core/src/services/connection.rs
- [ ] T046 [US3] Implement post_create hook for session defaults (statement_timeout, idle_in_transaction_session_timeout) in crates/tusk_core/src/services/connection.rs
- [ ] T047 [US3] Implement ConnectionPool::get() with 30s wait timeout for pool exhaustion in crates/tusk_core/src/services/connection.rs (FR-013a)
- [ ] T048 [US3] Implement ConnectionPool::status() returning PoolStatus in crates/tusk_core/src/services/connection.rs (FR-013, SC-010)
- [ ] T049 [US3] Implement ConnectionPool::close() and is_closed() methods in crates/tusk_core/src/services/connection.rs
- [ ] T050 [US3] Implement PooledConnection wrapper with query, execute, prepare, transaction methods in crates/tusk_core/src/services/connection.rs
- [ ] T051 [US3] Implement Transaction struct with commit and rollback in crates/tusk_core/src/services/connection.rs
- [ ] T052 [US3] Update crates/tusk_core/src/models/mod.rs to export connection module

**Checkpoint**: User Story 3 complete - connection pooling is fully functional and independently testable

---

## Phase 6: User Story 4 - Query Execution with Cancellation (Priority: P2)

**Goal**: Execute queries with unique identifiers and cancellation support

**Independent Test**: Execute a query, initiate cancellation, verify the query stops and returns appropriate cancellation indication

### Implementation for User Story 4

- [ ] T053 [P] [US4] Create QueryHandle struct with cancellation token in crates/tusk_core/src/models/query.rs (FR-014, FR-015)
- [ ] T054 [P] [US4] Create QueryType enum in crates/tusk_core/src/models/query.rs
- [ ] T055 [P] [US4] Create ColumnInfo struct in crates/tusk_core/src/models/query.rs
- [ ] T056 [P] [US4] Create QueryResult struct in crates/tusk_core/src/models/query.rs
- [ ] T057 [US4] Implement QueryHandle constructor and methods (id, connection_id, sql, started_at, elapsed, cancel, is_cancelled) in crates/tusk_core/src/models/query.rs
- [ ] T058 [US4] Create QueryHistoryEntry struct with from_result and from_error constructors in crates/tusk_core/src/models/history.rs
- [ ] T059 [US4] Create QueryService with execute() method supporting cancellation in crates/tusk_core/src/services/query.rs (FR-015)
- [ ] T060 [US4] Implement QueryService::execute_with_params() for parameterized queries in crates/tusk_core/src/services/query.rs
- [ ] T061 [US4] Implement cancellation propagation within 50ms in crates/tusk_core/src/services/query.rs (SC-004)
- [ ] T062 [US4] Implement QueryService::detect_query_type() for SQL parsing in crates/tusk_core/src/services/query.rs
- [ ] T063 [US4] Handle cancellation race with completion: return results normally if query completed before cancellation in crates/tusk_core/src/services/query.rs
- [ ] T064 [US4] Update crates/tusk_core/src/models/mod.rs to export query and history modules

**Checkpoint**: User Story 4 complete - query execution with cancellation is fully functional and independently testable

---

## Phase 7: User Story 5 - Secure Credential Storage (Priority: P2)

**Goal**: Store passwords securely in OS keychain with session fallback

**Independent Test**: Store a password, retrieve it, check existence, delete it from the keychain

### Implementation for User Story 5

- [ ] T065 [US5] Create CredentialService struct with keychain availability detection in crates/tusk_core/src/services/credentials.rs (FR-017)
- [ ] T066 [US5] Implement CredentialService::new() with startup availability check in crates/tusk_core/src/services/credentials.rs
- [ ] T067 [US5] Implement status methods (is_available, unavailable_reason, is_using_fallback) in crates/tusk_core/src/services/credentials.rs
- [ ] T068 [US5] Implement store_password() with OS keychain storage in crates/tusk_core/src/services/credentials.rs (FR-017, FR-018, SC-005)
- [ ] T069 [US5] Implement get_password() with keychain retrieval in crates/tusk_core/src/services/credentials.rs (FR-019, SC-005)
- [ ] T070 [US5] Implement delete_password() in crates/tusk_core/src/services/credentials.rs (FR-019)
- [ ] T071 [US5] Implement has_password() existence check in crates/tusk_core/src/services/credentials.rs (FR-019)
- [ ] T072 [US5] Implement SSH passphrase operations (store, get, delete) in crates/tusk_core/src/services/credentials.rs
- [ ] T073 [US5] Implement in-memory session storage fallback when keychain unavailable in crates/tusk_core/src/services/credentials.rs (FR-019a)
- [ ] T074 [US5] Implement clear_session_credentials() for app exit cleanup in crates/tusk_core/src/services/credentials.rs
- [ ] T075 [US5] Ensure passwords are NEVER logged (verify no tracing calls with password values) in crates/tusk_core/src/services/credentials.rs (FR-018)

**Checkpoint**: User Story 5 complete - credential storage is fully functional and independently testable

---

## Phase 8: User Story 6 - Asynchronous Task Execution (Priority: P2)

**Goal**: Background execution for database operations without blocking UI

**Independent Test**: Spawn background tasks, verify they execute asynchronously, confirm results returned without blocking

### Implementation for User Story 6

- [ ] T076 [US6] Create dedicated Tokio runtime with multi-threaded configuration in crates/tusk_core/src/state.rs (FR-020)
- [ ] T077 [US6] Implement TuskState::runtime() accessor for Tokio runtime handle in crates/tusk_core/src/state.rs
- [ ] T078 [US6] Implement TuskState::spawn() for spawning futures on Tokio runtime in crates/tusk_core/src/state.rs (FR-020, FR-021)
- [ ] T079 [US6] Ensure all database futures are Send + 'static for background execution in crates/tusk_core/src/state.rs
- [ ] T080 [US6] Document GPUI-Tokio bridge pattern for UI integration in crates/tusk_core/src/state.rs

**Checkpoint**: User Story 6 complete - async execution is fully functional and independently testable

---

## Phase 9: User Story 7 - Application Logging (Priority: P3)

**Goal**: Structured logging to console and rotating files for debugging and troubleshooting

**Independent Test**: Generate log events at various levels and verify output appears in both console and rotating log files

### Implementation for User Story 7

- [ ] T081 [P] [US7] Create LogConfig struct with log_dir, is_pty, log_filter fields in crates/tusk_core/src/logging.rs
- [ ] T082 [P] [US7] Create LoggingGuard struct with WorkerGuard for flush on shutdown in crates/tusk_core/src/logging.rs
- [ ] T083 [US7] Implement init_logging() with console and file output in crates/tusk_core/src/logging.rs (FR-022)
- [ ] T084 [US7] Implement daily log rotation with tracing_appender in crates/tusk_core/src/logging.rs (FR-023)
- [ ] T085 [US7] Implement non-blocking file writes via tracing_appender::non_blocking in crates/tusk_core/src/logging.rs (SC-007)
- [ ] T086 [US7] Implement build-type conditional log levels (debug for dev, info for release) in crates/tusk_core/src/logging.rs (FR-024)
- [ ] T087 [US7] Implement TUSK_LOG and RUST_LOG environment variable override in crates/tusk_core/src/logging.rs
- [ ] T088 [US7] Implement graceful fallback to console-only logging when file logging fails in crates/tusk_core/src/logging.rs (FR-024a)
- [ ] T089 [US7] Implement init_logging_default() convenience function in crates/tusk_core/src/logging.rs
- [ ] T090 [US7] Implement default_log_filter() and log_dir() helper functions in crates/tusk_core/src/logging.rs

**Checkpoint**: User Story 7 complete - logging is fully functional and independently testable

---

## Phase 10: Polish & Integration

**Purpose**: Final integration and cross-cutting validation

- [ ] T091 [P] Update crates/tusk_core/src/services/mod.rs to export all service modules (connection, query, credentials, storage)
- [ ] T092 Integrate TuskState initialization into main application in crates/tusk/src/main.rs
- [ ] T093 Initialize logging before TuskState in crates/tusk/src/main.rs
- [ ] T094 Set TuskState as gpui::Global in application context in crates/tusk/src/main.rs
- [ ] T095 Verify all success criteria performance targets are met (SC-001 through SC-010)
- [ ] T096 Run cargo clippy and fix any warnings in crates/tusk_core/
- [ ] T097 Run cargo fmt on all modified files
- [ ] T098 Validate quickstart.md examples compile and work correctly

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-9)**: All depend on Foundational phase completion
  - US1 (Error Handling) can proceed first - other stories depend on error types
  - US2 (State) depends on US1 for error handling
  - US3 (Connection) depends on US1, US2
  - US4 (Query) depends on US1, US2, US3
  - US5 (Credentials) depends on US1 (can run parallel to US3, US4)
  - US6 (Async) depends on US2 (can run parallel after US2)
  - US7 (Logging) has no story dependencies - can run after Foundational
- **Polish (Phase 10)**: Depends on all user stories being complete

### User Story Dependencies

```
US1 (Error Handling)
 │
 ├─► US2 (State Management)
 │    │
 │    ├─► US3 (Connection Pooling)
 │    │    │
 │    │    └─► US4 (Query Execution)
 │    │
 │    └─► US6 (Async Execution)
 │
 └─► US5 (Credentials) [parallel with US3, US4, US6]

US7 (Logging) [independent - after Foundational only]
```

### Within Each User Story

- Models before services
- Structs before methods
- Core implementation before integration

### Parallel Opportunities

- **Phase 1**: T002, T003 can run in parallel
- **Phase 3 (US1)**: T021, T022, T023, T099 can run in parallel (different From impls)
- **Phase 5 (US3)**: T034-T038, T041 can run in parallel (different structs)
- **Phase 6 (US4)**: T053-T056 can run in parallel (different structs)
- **Phase 9 (US7)**: T081, T082 can run in parallel
- **Phase 10**: T091 can run in parallel with other tasks

---

## Parallel Example: User Story 3

```bash
# Launch all model structs together:
Task: "Create ConnectionConfig struct in crates/tusk_core/src/models/connection.rs"
Task: "Create SslMode enum in crates/tusk_core/src/models/connection.rs"
Task: "Create SshTunnelConfig struct in crates/tusk_core/src/models/connection.rs"
Task: "Create SshAuthMethod enum in crates/tusk_core/src/models/connection.rs"
Task: "Create ConnectionOptions struct in crates/tusk_core/src/models/connection.rs"
Task: "Create PoolStatus struct in crates/tusk_core/src/models/connection.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1-3)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Error Handling)
4. Complete Phase 4: User Story 2 (State Management)
5. Complete Phase 5: User Story 3 (Connection Pooling)
6. **STOP and VALIDATE**: Test MVP independently
7. Deploy/demo if ready

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add US1 (Error Handling) → Test independently
3. Add US2 (State) → Test independently
4. Add US3 (Connections) → Test independently → **MVP Complete!**
5. Add US4 (Query) → Test independently
6. Add US5 (Credentials) → Test independently
7. Add US6 (Async) → Test independently
8. Add US7 (Logging) → Test independently
9. Polish → **Feature Complete!**

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently

## ⚠️ TASK IMMUTABILITY (Constitution Principle V)

**Once tasks are created, they are IMMUTABLE:**

- Tasks MUST NEVER be removed, deleted, or merged
- Tasks MUST NEVER be renumbered (task IDs are permanent)
- Tasks MUST NEVER have their scope reduced
- If a task seems wrong, FLAG IT for human review — do NOT modify or delete it
- The ONLY valid change is marking a task complete (unchecked → checked)

**Violation Consequence**: Task removal/merger/scope reduction requires immediate branch deletion.
