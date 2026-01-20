# Tasks: Backend Architecture

**Input**: Design documents from `/specs/002-backend-architecture/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Not explicitly requested in spec - test tasks omitted.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Backend**: `src-tauri/src/` (Rust)
- **Frontend**: `src/` (Svelte/TypeScript)

---

## Phase 1: Setup

**Purpose**: Add missing dependencies and prepare module structure

- [x] T001 Add `tracing-appender = "0.2"` dependency to src-tauri/Cargo.toml
- [x] T002 [P] Create commands module structure with src-tauri/src/commands/mod.rs
- [x] T003 [P] Create services module structure with src-tauri/src/services/mod.rs
- [x] T004 [P] Create models module structure with src-tauri/src/models/mod.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T005 Create TuskError enum with all variants in src-tauri/src/error.rs
- [x] T006 Implement From<tokio_postgres::Error> for TuskError with PostgreSQL error preservation in src-tauri/src/error.rs
- [x] T007 Implement From<rusqlite::Error> for TuskError in src-tauri/src/error.rs
- [x] T008 Implement From<keyring::Error> for TuskError in src-tauri/src/error.rs
- [x] T009 [P] Create ErrorKind enum in src-tauri/src/models/error.rs for frontend serialization
- [x] T010 [P] Create AppInfo struct in src-tauri/src/models/app.rs
- [x] T011 Create AppDirs struct with Tauri path resolver in src-tauri/src/services/dirs.rs
- [x] T012 Implement directory creation on first launch in src-tauri/src/services/dirs.rs
- [x] T013 Export error module from src-tauri/src/lib.rs

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Application Starts Successfully (Priority: P1) 🎯 MVP

**Goal**: Application starts reliably within 1 second, creates required directories, shows clear errors on failure

**Independent Test**: Launch application and verify window appears within 1 second with main interface ready

### Implementation for User Story 1

- [x] T014 [US1] Create LogGuard struct in src-tauri/src/services/logging.rs
- [x] T015 [US1] Implement init_logging() with daily rotation in src-tauri/src/services/logging.rs
- [x] T016 [US1] Configure debug/release log levels in src-tauri/src/services/logging.rs
- [x] T017 [US1] Implement health_check command in src-tauri/src/commands/app.rs
- [x] T018 [US1] Implement get_log_directory command in src-tauri/src/commands/app.rs
- [x] T019 [US1] Export commands/app module from src-tauri/src/commands/mod.rs
- [x] T020 [US1] Create AppState struct with service fields in src-tauri/src/state.rs
- [x] T021 [US1] Implement AppState::new() initialization in src-tauri/src/state.rs
- [x] T022 [US1] Initialize logging before Builder in src-tauri/src/lib.rs
- [x] T023 [US1] Create AppState in setup() closure in src-tauri/src/lib.rs
- [x] T024 [US1] Register health_check and get_log_directory commands in src-tauri/src/lib.rs invoke_handler
- [x] T025 [US1] Add graceful shutdown handling in src-tauri/src/lib.rs

**Checkpoint**: Application starts successfully, health_check returns valid response, logs appear in app data directory

---

## Phase 4: User Story 2 - Error Messages Are Clear and Actionable (Priority: P1)

**Goal**: All errors include human-readable messages, hints, and PostgreSQL error codes/positions where applicable

**Independent Test**: Trigger connection failure and verify error message includes reason and suggestion for resolution

### Implementation for User Story 2

- [x] T026 [US2] Add Validation variant to TuskError in src-tauri/src/error.rs
- [x] T027 [US2] Implement Display trait for TuskError with actionable messages in src-tauri/src/error.rs
- [x] T028 [US2] Add hint generation helper for common error types in src-tauri/src/error.rs
- [x] T029 [US2] Create ErrorResponse struct matching frontend interface in src-tauri/src/models/error.rs
- [x] T030 [US2] Implement From<TuskError> for ErrorResponse in src-tauri/src/models/error.rs
- [x] T031 [US2] Export models/error module from src-tauri/src/models/mod.rs

**Checkpoint**: Error types are complete with actionable messages, hints, and PostgreSQL error preservation

---

## Phase 5: User Story 3 - Application State Persists Correctly (Priority: P2)

**Goal**: Saved connections and preferences persist between sessions, corrupted storage is repaired or backed up

**Independent Test**: Save connection, close app, reopen, verify connection is present

### Implementation for User Story 3

- [x] T032 [US3] Create ConnectionConfig struct in src-tauri/src/models/connection.rs
- [x] T033 [US3] Create SslMode enum in src-tauri/src/models/connection.rs
- [x] T034 [US3] Create SshTunnel struct in src-tauri/src/models/connection.rs
- [x] T035 [US3] Create SshAuthMethod enum in src-tauri/src/models/connection.rs
- [x] T036 [US3] Implement Serialize/Deserialize for ConnectionConfig in src-tauri/src/models/connection.rs
- [x] T037 [US3] Export models/connection module from src-tauri/src/models/mod.rs
- [x] T038 [US3] Create StorageService struct in src-tauri/src/services/storage.rs
- [x] T039 [US3] Implement open_or_repair() with PRAGMA integrity_check in src-tauri/src/services/storage.rs
- [x] T040 [US3] Implement backup_corrupted() with timestamp naming in src-tauri/src/services/storage.rs
- [x] T041 [US3] Implement create_fresh() database initialization in src-tauri/src/services/storage.rs
- [x] T042 [US3] Create SQLite schema with connections table in src-tauri/src/services/storage.rs
- [x] T043 [US3] Create SQLite schema with preferences table in src-tauri/src/services/storage.rs
- [x] T044 [US3] Create SQLite schema with migrations table in src-tauri/src/services/storage.rs
- [x] T045 [US3] Implement migration system for schema versioning in src-tauri/src/services/storage.rs
- [x] T046 [US3] Implement list_connections() in src-tauri/src/services/storage.rs
- [x] T047 [US3] Implement get_connection() in src-tauri/src/services/storage.rs
- [x] T048 [US3] Implement save_connection() in src-tauri/src/services/storage.rs
- [x] T049 [US3] Implement delete_connection() in src-tauri/src/services/storage.rs
- [x] T050 [US3] Implement get_preference() in src-tauri/src/services/storage.rs
- [x] T051 [US3] Implement set_preference() in src-tauri/src/services/storage.rs
- [x] T052 [US3] Implement check_integrity() returning DatabaseHealth in src-tauri/src/services/storage.rs
- [x] T053 [US3] Create DatabaseHealth struct in src-tauri/src/models/app.rs
- [x] T054 [US3] Export services/storage module from src-tauri/src/services/mod.rs
- [x] T055 [US3] Implement list_connections command in src-tauri/src/commands/storage.rs
- [x] T056 [US3] Implement get_connection command in src-tauri/src/commands/storage.rs
- [x] T057 [US3] Implement save_connection command in src-tauri/src/commands/storage.rs
- [x] T058 [US3] Implement delete_connection command in src-tauri/src/commands/storage.rs
- [x] T059 [US3] Implement get_preference command in src-tauri/src/commands/storage.rs
- [x] T060 [US3] Implement set_preference command in src-tauri/src/commands/storage.rs
- [x] T061 [US3] Implement check_database_health command in src-tauri/src/commands/storage.rs
- [x] T062 [US3] Export commands/storage module from src-tauri/src/commands/mod.rs
- [x] T063 [US3] Register storage commands in src-tauri/src/lib.rs invoke_handler
- [x] T064 [US3] Wire StorageService into AppState in src-tauri/src/state.rs

**Checkpoint**: Connections and preferences persist across restarts, corrupted storage triggers backup and reset

---

## Phase 6: User Story 4 - Multiple Connections Work Independently (Priority: P2)

**Goal**: Multiple database connections operate in isolation; one failing doesn't affect others

**Independent Test**: Connect to two databases, disconnect one, verify other continues working

### Implementation for User Story 4

- [x] T065 [US4] Create ConnectionPool struct in src-tauri/src/models/connection.rs
- [x] T066 [US4] Create ActiveConnection struct in src-tauri/src/models/connection.rs
- [x] T067 [US4] Create ConnectionTestResult struct in src-tauri/src/models/connection.rs
- [x] T068 [US4] Create ConnectionService struct in src-tauri/src/services/connection.rs
- [x] T069 [US4] Implement create_pool() with deadpool-postgres configuration in src-tauri/src/services/connection.rs
- [x] T070 [US4] Implement SSL/TLS configuration helper in src-tauri/src/services/connection.rs
- [x] T071 [US4] Implement connect() storing pool in AppState.connections in src-tauri/src/services/connection.rs
- [x] T072 [US4] Implement disconnect() removing pool and cleaning up resources in src-tauri/src/services/connection.rs
- [x] T073 [US4] Implement test_connection() for validation without saving in src-tauri/src/services/connection.rs
- [x] T074 [US4] Implement get_active_connections() listing all pools in src-tauri/src/services/connection.rs
- [x] T075 [US4] Export services/connection module from src-tauri/src/services/mod.rs
- [x] T076 [US4] Implement connect command in src-tauri/src/commands/connection.rs
- [x] T077 [US4] Implement disconnect command in src-tauri/src/commands/connection.rs
- [x] T078 [US4] Implement test_connection command in src-tauri/src/commands/connection.rs
- [x] T079 [US4] Implement get_active_connections command in src-tauri/src/commands/connection.rs
- [x] T080 [US4] Export commands/connection module from src-tauri/src/commands/mod.rs
- [x] T081 [US4] Register connection commands in src-tauri/src/lib.rs invoke_handler
- [x] T082 [US4] Add connections HashMap<Uuid, ConnectionPool> to AppState in src-tauri/src/state.rs
- [x] T083 [US4] Wire ConnectionService into AppState in src-tauri/src/state.rs

**Checkpoint**: Multiple connections can be established and operate independently

---

## Phase 7: User Story 5 - Long-Running Queries Can Be Cancelled (Priority: P2)

**Goal**: Running queries can be cancelled within 2 seconds without killing the connection

**Independent Test**: Execute `SELECT pg_sleep(10)`, click cancel, verify query stops within 2 seconds and connection remains usable

### Implementation for User Story 5

- [x] T084 [US5] Create QueryHandle struct with CancelToken in src-tauri/src/models/query.rs
- [x] T085 [US5] Create QueryStatus enum in src-tauri/src/models/query.rs
- [x] T086 [US5] Create QueryResult struct in src-tauri/src/models/query.rs
- [x] T087 [US5] Create Column struct in src-tauri/src/models/query.rs
- [x] T088 [US5] Create ActiveQuery struct in src-tauri/src/models/query.rs
- [x] T089 [US5] Export models/query module from src-tauri/src/models/mod.rs
- [x] T090 [US5] Create QueryService struct in src-tauri/src/services/query.rs
- [x] T091 [US5] Implement execute() with tokio::select! timeout in src-tauri/src/services/query.rs
- [x] T092 [US5] Implement query registration in active_queries HashMap in src-tauri/src/services/query.rs
- [x] T093 [US5] Implement cancel() using CancelToken in src-tauri/src/services/query.rs
- [x] T094 [US5] Implement get_active_queries() listing running queries in src-tauri/src/services/query.rs
- [x] T095 [US5] Implement query result serialization for IPC transport in src-tauri/src/services/query.rs
- [x] T096 [US5] Export services/query module from src-tauri/src/services/mod.rs
- [x] T097 [US5] Implement execute_query command in src-tauri/src/commands/query.rs
- [x] T098 [US5] Implement cancel_query command in src-tauri/src/commands/query.rs
- [x] T099 [US5] Implement get_active_queries command in src-tauri/src/commands/query.rs
- [x] T100 [US5] Export commands/query module from src-tauri/src/commands/mod.rs
- [x] T101 [US5] Register query commands in src-tauri/src/lib.rs invoke_handler
- [x] T102 [US5] Add active_queries HashMap<Uuid, QueryHandle> to AppState in src-tauri/src/state.rs

**Checkpoint**: Queries can be executed and cancelled, connection remains usable after cancellation

---

## Phase 8: User Story 6 - Credentials Are Stored Securely (Priority: P3)

**Goal**: Database passwords stored in OS keychain, not in plain text files

**Independent Test**: Save connection with password, search app data directory for password text, verify it's not found

### Implementation for User Story 6

- [x] T103 [US6] Create CredentialService struct in src-tauri/src/services/credentials.rs
- [x] T104 [US6] Implement keychain availability detection in src-tauri/src/services/credentials.rs
- [x] T105 [US6] Implement store_password() using keyring crate in src-tauri/src/services/credentials.rs
- [x] T106 [US6] Implement get_password() with NoEntry handling in src-tauri/src/services/credentials.rs
- [x] T107 [US6] Implement delete_password() in src-tauri/src/services/credentials.rs
- [x] T108 [US6] Implement has_password() for checking existence in src-tauri/src/services/credentials.rs
- [x] T109 [US6] Export services/credentials module from src-tauri/src/services/mod.rs
- [x] T110 [US6] Implement check_keychain_available command in src-tauri/src/commands/storage.rs
- [x] T111 [US6] Implement has_stored_password command in src-tauri/src/commands/storage.rs
- [x] T112 [US6] Register credential commands in src-tauri/src/lib.rs invoke_handler
- [x] T113 [US6] Wire CredentialService into AppState in src-tauri/src/state.rs
- [x] T114 [US6] Integrate credential storage into save_connection flow in src-tauri/src/commands/storage.rs
- [x] T115 [US6] Integrate credential deletion into delete_connection flow in src-tauri/src/commands/storage.rs
- [x] T116 [US6] Integrate credential retrieval into connect flow in src-tauri/src/commands/connection.rs

**Checkpoint**: Passwords stored in OS keychain, not visible in app data directory

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Final integration, verification, and cleanup

- [x] T117 Verify all 19 IPC commands are registered in src-tauri/src/lib.rs
- [x] T118 Add PRAGMA busy_timeout = 5000 to SQLite connections in src-tauri/src/services/storage.rs
- [x] T119 Verify TuskError is serializable and all variants have actionable messages
- [x] T120 Run cargo build and fix any compilation errors
- [x] T121 Run cargo clippy and address warnings
- [x] T122 Verify cold start completes in under 1 second per SC-001
- [x] T123 Verify memory usage under 100MB idle per SC-002
- [x] T124 Verify log files are created in app data directory per SC-009

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Phase 2 - Enables application startup
- **User Story 2 (Phase 4)**: Depends on Phase 2 - Can parallel with US1
- **User Story 3 (Phase 5)**: Depends on Phase 2 - Can parallel with US1, US2
- **User Story 4 (Phase 6)**: Depends on Phase 2, partial US3 (ConnectionConfig model) - Should follow US3
- **User Story 5 (Phase 7)**: Depends on US4 (requires active connections) - Should follow US4
- **User Story 6 (Phase 8)**: Depends on Phase 2 - Can parallel with others but integrates with US3, US4
- **Polish (Phase 9)**: Depends on all user stories complete

### User Story Dependencies

- **US1 (P1)**: Foundational only - can start first
- **US2 (P1)**: Foundational only - can parallel with US1
- **US3 (P2)**: Foundational only - provides ConnectionConfig for US4
- **US4 (P2)**: Uses ConnectionConfig from US3
- **US5 (P2)**: Requires ConnectionPool from US4
- **US6 (P3)**: Integrates with US3 (save_connection) and US4 (connect)

### Recommended Execution Order

1. Phase 1: Setup (T001-T004)
2. Phase 2: Foundational (T005-T013)
3. Phase 3: US1 (T014-T025) - MVP startup
4. Phase 4: US2 (T026-T031) - can parallel with US1
5. Phase 5: US3 (T032-T064) - persistence
6. Phase 6: US4 (T065-T083) - depends on US3 models
7. Phase 7: US5 (T084-T102) - depends on US4
8. Phase 8: US6 (T103-T116) - integrates with US3, US4
9. Phase 9: Polish (T117-T124)

### Parallel Opportunities

**Within Phase 1:**

```
T002 [P], T003 [P], T004 [P] can run in parallel
```

**Within Phase 2:**

```
T009 [P], T010 [P] can run in parallel (after T005-T008)
```

**User Stories can parallel by developer:**

- Developer A: US1 (startup) + US2 (errors)
- Developer B: US3 (persistence) → US4 (connections)
- Developer C: US5 (queries) + US6 (credentials)

---

## Implementation Strategy

### MVP First (User Stories 1 + 2)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1 (Application Starts)
4. Complete Phase 4: User Story 2 (Error Messages)
5. **VALIDATE**: App starts, health_check works, errors are actionable
6. Deploy/demo MVP

### Full Feature Delivery

1. MVP (above)
2. Add US3 (Persistence) → Test save/load connections
3. Add US4 (Multiple Connections) → Test connection isolation
4. Add US5 (Query Cancellation) → Test cancel functionality
5. Add US6 (Secure Credentials) → Verify no plaintext passwords
6. Polish phase → Final verification

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
