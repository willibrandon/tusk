# Tasks: Service Integration Layer

**Input**: Design documents from `/specs/004-service-integration/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

This is a Rust workspace with multiple crates:
- `crates/tusk/src/` - Main application binary
- `crates/tusk_core/src/` - Core backend services & types
- `crates/tusk_ui/src/` - GPUI UI components

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and core type definitions

- [X] T001 Add ConnectionEntry struct to crates/tusk_core/src/state.rs wrapping pool with status tracking
- [X] T002 [P] Add ConnectionStatus enum (Disconnected, Connecting, Connected, Error) to crates/tusk_core/src/models/connection.rs
- [X] T003 [P] Add QueryEvent enum (Columns, Rows, Progress, Complete, Error) to crates/tusk_core/src/models/query.rs
- [X] T004 [P] Add SchemaCache struct with TTL support to crates/tusk_core/src/models/schema.rs
- [X] T005 [P] Add ColumnInfo struct for query column metadata to crates/tusk_core/src/models/query.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T006 Implement TuskState.connect() method for establishing database connections in crates/tusk_core/src/state.rs
- [X] T007 Implement TuskState.disconnect() method for closing connections in crates/tusk_core/src/state.rs
- [X] T008 Implement TuskState.get_connection_status() method in crates/tusk_core/src/state.rs
- [X] T009 Implement TuskState.test_connection() method for connection validation in crates/tusk_core/src/state.rs
- [X] T010 [P] Update QueryService to support streaming via mpsc channel in crates/tusk_core/src/services/query.rs
- [X] T011 [P] Implement query cancellation with CancellationToken and tokio::select! in crates/tusk_core/src/services/query.rs
- [X] T012 [P] Implement TuskError to ErrorInfo conversion (to_error_info method) in crates/tusk_core/src/error.rs
- [X] T013 [P] Complete CredentialService store_password and get_password methods in crates/tusk_core/src/services/credentials.rs
- [X] T014 [P] Implement session-only password fallback when keychain unavailable in crates/tusk_core/src/services/credentials.rs
- [X] T015 Add tracing instrumentation for service calls at DEBUG level in crates/tusk_core/src/services/query.rs
- [X] T016 [P] Add tracing instrumentation for errors at WARN/ERROR level in crates/tusk_core/src/error.rs
- [X] T017 Ensure passwords are never logged (FR-026) by auditing all tracing calls in crates/tusk_core/src/

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Execute Query and View Results (Priority: P1) 🎯 MVP

**Goal**: User executes SQL query and views streaming results while UI remains responsive

**Independent Test**: Connect to database, run a query, verify results appear while UI remains responsive

### Implementation for User Story 1

- [X] T018 [US1] Implement TuskState.execute_query() method returning QueryHandle in crates/tusk_core/src/state.rs
- [X] T019 [US1] Implement TuskState.execute_query_streaming() method with mpsc::Sender<QueryEvent> in crates/tusk_core/src/state.rs
- [X] T020 [US1] Add QueryEditorState struct with connection_id, active_query, and status fields to crates/tusk_ui/src/query_editor.rs
- [X] T021 [US1] Add QueryEditorStatus enum (Idle, Executing, Cancelled) to crates/tusk_ui/src/query_editor.rs
- [X] T022 [US1] Implement QueryEditor.execute_query() method using cx.spawn() pattern in crates/tusk_ui/src/query_editor.rs
- [X] T023 [US1] Add ResultsPanelState struct with columns, rows, status, and error fields to crates/tusk_ui/src/panels/results.rs
- [X] T024 [US1] Add ResultsStatus enum (Empty, Loading, Streaming, Complete, Error) to crates/tusk_ui/src/panels/results.rs
- [X] T025 [US1] Implement ResultsPanel.start_streaming() to receive QueryEvent stream in crates/tusk_ui/src/panels/results.rs
- [X] T026 [US1] Implement ResultsPanel.handle_event() to process Columns, Rows, Complete, Error events in crates/tusk_ui/src/panels/results.rs
- [X] T027 [US1] Wire query execution from QueryEditor to ResultsPanel via channel in crates/tusk_ui/src/workspace.rs
- [X] T028 [US1] Display execution time and row count in results panel status bar in crates/tusk_ui/src/panels/results.rs
- [X] T029 [US1] Implement batch size configuration (default 1000 rows) in QueryService in crates/tusk_core/src/services/query.rs

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Cancel Running Query (Priority: P1)

**Goal**: User can cancel a long-running query and UI returns to ready state

**Independent Test**: Run a slow query (pg_sleep), click cancel, verify query stops and UI returns to ready state

### Implementation for User Story 2

- [X] T030 [US2] Implement TuskState.cancel_query() method signaling CancellationToken in crates/tusk_core/src/state.rs
- [X] T031 [US2] Send PostgreSQL cancel to server when query is cancelled in crates/tusk_core/src/services/query.rs
- [X] T032 [US2] Emit QueryEvent::Error(QueryCancelled) when cancellation occurs in crates/tusk_core/src/services/query.rs
- [X] T033 [US2] Add cancel button to QueryEditor toolbar in crates/tusk_ui/src/query_editor.rs
- [X] T034 [US2] Implement QueryEditor.cancel_query() method in crates/tusk_ui/src/query_editor.rs
- [X] T035 [US2] Preserve already-received results when query is cancelled in crates/tusk_ui/src/panels/results.rs
- [X] T036 [US2] Show "Query cancelled" toast notification on cancellation in crates/tusk_ui/src/query_editor.rs
- [X] T037 [US2] Reset QueryEditor status to Idle after cancellation in crates/tusk_ui/src/query_editor.rs
- [X] T038 [US2] Implement task replacement pattern for automatic cancellation in crates/tusk_ui/src/query_editor.rs

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - Connect to Database (Priority: P1)

**Goal**: User enters connection details and connects to PostgreSQL database with clear status feedback

**Independent Test**: Enter valid credentials, click connect, verify connection status shows "Connected"

### Implementation for User Story 3

- [X] T039 [US3] Create ConnectionDialog component with form fields in crates/tusk_ui/src/connection_dialog.rs
- [X] T040 [US3] Add form fields for host, port, database, username, password in crates/tusk_ui/src/connection_dialog.rs
- [X] T041 [US3] Add SSL mode selector to connection dialog in crates/tusk_ui/src/connection_dialog.rs
- [X] T042 [US3] Implement Connect button action calling TuskState.connect() in crates/tusk_ui/src/connection_dialog.rs
- [X] T043 [US3] Implement Test Connection button calling TuskState.test_connection() in crates/tusk_ui/src/connection_dialog.rs
- [X] T044 [US3] Show connection progress indicator during connection attempt in crates/tusk_ui/src/connection_dialog.rs
- [X] T045 [US3] Display connection error with actionable hints on failure in crates/tusk_ui/src/connection_dialog.rs
- [X] T046 [US3] Add connection status indicator to workspace status bar in crates/tusk_ui/src/workspace.rs
- [X] T047 [US3] Update QueryEditor connection_id when connection established in crates/tusk_ui/src/query_editor.rs
- [X] T048 [US3] Update SchemaBrowser connection_id when connection established in crates/tusk_ui/src/panels/schema_browser.rs
- [X] T049 [US3] Handle connection lost during query gracefully (FR edge case) in crates/tusk_core/src/services/query.rs
- [X] T050 [US3] Store password in CredentialService after successful connection in crates/tusk_core/src/state.rs

**Checkpoint**: At this point, User Stories 1, 2, AND 3 should all work independently

---

## Phase 6: User Story 4 - View and Navigate Database Schema (Priority: P2)

**Goal**: User browses database schema with cached data for instant navigation

**Independent Test**: Connect to database, expand schema tree nodes, verify objects load and cache works

### Implementation for User Story 4

- [X] T051 [US4] Implement TuskState.get_schema() method with cache lookup in crates/tusk_core/src/state.rs
- [X] T052 [US4] Implement TuskState.refresh_schema() method invalidating cache in crates/tusk_core/src/state.rs
- [X] T053 [US4] Implement SchemaCache.is_expired() method with TTL check in crates/tusk_core/src/models/schema.rs
- [X] T054 [US4] Add SchemaBrowserState struct with loading, tree_items, expanded_nodes in crates/tusk_ui/src/panels/schema_browser.rs
- [X] T055 [US4] Implement SchemaBrowser.load_schema() using cx.spawn() pattern in crates/tusk_ui/src/panels/schema_browser.rs
- [X] T056 [US4] Add refresh button to schema browser toolbar in crates/tusk_ui/src/panels/schema_browser.rs
- [X] T057 [US4] Show loading indicator while schema loads in crates/tusk_ui/src/panels/schema_browser.rs
- [X] T058 [US4] Implement lazy loading for schema tree nodes in crates/tusk_ui/src/panels/schema_browser.rs
- [X] T059 [US4] Use separate pool connection for schema operations (FR-005) in crates/tusk_core/src/state.rs

**Checkpoint**: At this point, User Stories 1-4 should all work independently

---

## Phase 7: User Story 5 - Handle Errors Gracefully (Priority: P2)

**Goal**: Clear, actionable error messages for all failure scenarios with stable application state

**Independent Test**: Cause errors (bad SQL, wrong password), verify error messages are clear and helpful

### Implementation for User Story 5

- [X] T060 [US5] Implement error display rules based on ErrorInfo.recoverable flag in crates/tusk_ui/src/workspace.rs
- [X] T061 [US5] Show toast notifications for recoverable errors (auto-dismiss 10s) in crates/tusk_ui/src/toast.rs
- [X] T062 [US5] Create ErrorPanel component for detailed error display in crates/tusk_ui/src/error_panel.rs
- [X] T063 [US5] Display error position indicator for query errors in crates/tusk_ui/src/panels/results.rs
- [X] T064 [US5] Show PostgreSQL error code and hint in error panel in crates/tusk_ui/src/error_panel.rs
- [X] T065 [US5] Map PostgreSQL error codes to user-friendly hints in crates/tusk_core/src/error.rs
- [X] T066 [US5] Handle authentication errors with specific hints in crates/tusk_core/src/error.rs
- [X] T067 [US5] Handle connection timeout with retry suggestion in crates/tusk_core/src/error.rs
- [X] T068 [US5] Handle pool timeout with "close unused tabs" hint in crates/tusk_core/src/error.rs
- [X] T069 [US5] Log errors at WARN/ERROR level with context (no credentials) in crates/tusk_core/src/services/query.rs

**Checkpoint**: At this point, User Stories 1-5 should all work independently

---

## Phase 8: User Story 6 - Persist Application State (Priority: P3)

**Goal**: Application remembers connections and workspace layout between sessions

**Independent Test**: Save a connection, restart application, verify saved connection appears in list

### Implementation for User Story 6

- [X] T070 [US6] Complete LocalStorage SQLite schema for saved_connections table in crates/tusk_core/src/services/storage.rs
- [X] T071 [US6] Implement LocalStorage.save_connection() method in crates/tusk_core/src/services/storage.rs
- [X] T072 [US6] Implement LocalStorage.load_connections() method in crates/tusk_core/src/services/storage.rs
- [X] T073 [US6] Implement LocalStorage.delete_connection() method in crates/tusk_core/src/services/storage.rs
- [X] T074 [US6] Add query_history table schema to LocalStorage in crates/tusk_core/src/services/storage.rs
- [X] T075 [US6] Implement LocalStorage.save_query_history() method in crates/tusk_core/src/services/storage.rs
- [X] T076 [US6] Implement LocalStorage.load_query_history() method in crates/tusk_core/src/services/storage.rs
- [X] T077 [US6] Add ui_state table for workspace layout persistence in crates/tusk_core/src/services/storage.rs
- [X] T078 [US6] Create SavedConnectionsList component showing saved connections in crates/tusk_ui/src/connection_dialog.rs
- [X] T079 [US6] Add "Save Connection" checkbox to connection dialog in crates/tusk_ui/src/connection_dialog.rs
- [X] T080 [US6] Load saved connections on application startup in crates/tusk/src/main.rs
- [X] T081 [US6] Retrieve password from CredentialService when selecting saved connection in crates/tusk_ui/src/connection_dialog.rs
- [X] T082 [US6] Persist workspace layout on dock resize and pane changes in crates/tusk_ui/src/workspace.rs
- [X] T083 [US6] Restore workspace layout on application startup in crates/tusk_ui/src/workspace.rs

### Credential Provider Refactoring (Keychain Popup Fix)

**Problem**: macOS keychain access dialogs appear repeatedly during development because unsigned builds lack stable code signatures. The "Always Allow" selection cannot persist across rebuilds.

**Solution**: Use file-based credential storage for development builds; keychain for signed release builds (following Zed's approach).

**Reference**: See `/specs/004-service-integration/keychain-popup-analysis.md` for full root cause analysis.

- [X] T097 [US6] Create CredentialsProvider trait with store/get/delete methods in crates/tusk_core/src/services/credentials.rs
- [X] T098 [US6] Implement FileCredentialsProvider storing JSON at ~/.config/tusk/dev_credentials.json in crates/tusk_core/src/services/credentials.rs
- [X] T099 [US6] Implement KeychainCredentialsProvider wrapping keyring crate in crates/tusk_core/src/services/credentials.rs
- [X] T100 [US6] Update CredentialService to select FileCredentialsProvider when cfg!(debug_assertions) in crates/tusk_core/src/services/credentials.rs
- [X] T101 [US6] Add TUSK_USE_KEYCHAIN=1 env var override to force keychain in development builds in crates/tusk_core/src/services/credentials.rs
- [X] T102 [US6] Ensure dev_credentials.json file has restricted permissions (600) on creation in crates/tusk_core/src/services/credentials.rs
- [X] T103 [US6] Log which credential provider is active at DEBUG level on initialization in crates/tusk_core/src/services/credentials.rs

**Checkpoint**: At this point, all User Stories (1-6) should be fully functional

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [X] T084 Verify SC-001: UI responsive within 100ms during query execution
- [X] T085 Verify SC-002: First batch results within 500ms for simple queries
- [X] T086 Verify SC-003: Query cancellation within 1 second
- [X] T087 Verify SC-004: Schema load within 300ms for 1000+ tables
- [X] T088 Verify SC-005: Cached schema navigation under 30ms
- [X] T089 Verify SC-007: Streaming handles 1M+ rows without memory exhaustion
- [X] T090 Verify SC-008: Connection pool supports 10 concurrent queries
- [X] T091 [P] Audit all service calls have DEBUG level tracing (FR-024)
- [X] T092 [P] Audit all error paths have WARN/ERROR level tracing (FR-025)
- [X] T093 [P] Final audit that no passwords appear in logs (FR-026)
- [X] T094 Run quickstart.md validation scenarios
- [X] T095 Code cleanup and documentation
- [X] T096 Verify SC-006: All 21 documented error scenarios (E01-E21) display actionable hints per error-handling.md

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-8)**: All depend on Foundational phase completion
  - US1 (P1), US2 (P1), US3 (P1) are all Priority 1 - implement first
  - US4 (P2), US5 (P2) are Priority 2
  - US6 (P3) is Priority 3
- **Polish (Phase 9)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P1)**: Can start after Foundational (Phase 2) - Uses QueryHandle from US1 patterns
- **User Story 3 (P1)**: Can start after Foundational (Phase 2) - Provides connection for US1/US2
- **User Story 4 (P2)**: Can start after Foundational (Phase 2) - Uses connection from US3
- **User Story 5 (P2)**: Can start after Foundational (Phase 2) - Error handling for all stories
- **User Story 6 (P3)**: Can start after Foundational (Phase 2) - Persistence layer

### Within Each User Story

- Models before services (data structures first)
- Services before UI (backend before frontend)
- Core implementation before integration
- Story complete before moving to next priority

### Parallel Opportunities

**Phase 1 (Setup)**:
```
T002, T003, T004, T005 can all run in parallel (different files)
```

**Phase 2 (Foundational)**:
```
T010, T011 can run in parallel (same file but independent sections)
T012, T013, T014 can run in parallel (different files)
T015, T016 can run in parallel (different files)
```

**User Stories after Phase 2**:
```
US1, US2, US3 can be worked in parallel by different developers
US4, US5 can be worked in parallel after Phase 2
US6 can start anytime after Phase 2
```

---

## Parallel Example: User Story 1

```bash
# Launch model tasks in parallel:
T020: "Add QueryEditorState struct" (query_editor.rs)
T023: "Add ResultsPanelState struct" (results.rs)
T024: "Add ResultsStatus enum" (results.rs)

# Then service tasks:
T018: "Implement TuskState.execute_query()"
T019: "Implement TuskState.execute_query_streaming()"

# Then UI integration:
T022, T025, T026, T27, T28, T29
```

---

## Implementation Strategy

### MVP First (User Stories 1-3 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Execute Query)
4. Complete Phase 4: User Story 2 (Cancel Query)
5. Complete Phase 5: User Story 3 (Connect to Database)
6. **STOP and VALIDATE**: Test all P1 stories independently
7. Deploy/demo if ready

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Demo (Query Execution!)
3. Add User Story 2 → Test independently → Demo (Cancellation!)
4. Add User Story 3 → Test independently → Demo (Connection Management!)
5. Add User Story 4 → Test independently → Demo (Schema Browser!)
6. Add User Story 5 → Test independently → Demo (Error Handling!)
7. Add User Story 6 → Test independently → Demo (Persistence!)
8. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (Query Execution)
   - Developer B: User Story 2 (Cancellation)
   - Developer C: User Story 3 (Connection)
3. After P1 stories complete:
   - Developer A: User Story 4 (Schema)
   - Developer B: User Story 5 (Errors)
   - Developer C: User Story 6 (Persistence)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence

## ⚠️ TASK IMMUTABILITY (Constitution Principle V)

**Once tasks are created, they are IMMUTABLE:**

- Tasks MUST NEVER be removed, deleted, or merged
- Tasks MUST NEVER be renumbered (task IDs are permanent)
- Tasks MUST NEVER have their scope reduced
- If a task seems wrong, FLAG IT for human review — do NOT modify or delete it
- The ONLY valid change is marking a task complete (unchecked → checked)

**Violation Consequence**: Task removal/merger/scope reduction requires immediate branch deletion.
