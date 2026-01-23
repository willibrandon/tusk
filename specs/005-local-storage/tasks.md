# Tasks: Local Storage

**Input**: Design documents from `/specs/005-local-storage/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are included as the feature specification requires validation of data integrity (SC-005) and performance targets.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Multi-crate workspace**: `crates/tusk_core/src/` for core services, `crates/tusk_ui/src/` for UI components
- Tests in `crates/tusk_core/src/` alongside implementation (Rust convention)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: New models and schema migration required by multiple user stories

- [ ] T001 Create settings.rs model file with ConnectionGroup, QueryFolder, EditorTab, WindowState, AppSettings, ExportData, ConflictResolution, ImportResult types in crates/tusk_core/src/models/settings.rs
- [ ] T002 Export new model types from crates/tusk_core/src/models/mod.rs
- [ ] T003 Add Migration 2 (connection_groups table, query_folders table, group_id column, folder_id column, new indexes) in crates/tusk_core/src/services/storage.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T004 Add serde derive macros to ConnectionGroup, QueryFolder, EditorTab, WindowState, AppSettings for JSON serialization in crates/tusk_core/src/models/settings.rs
- [ ] T005 [P] Add ConnectionGroup::new() constructor with name and optional color parameters in crates/tusk_core/src/models/settings.rs
- [ ] T006 [P] Add ConnectionGroup::validate() method to check name length (1-100) and color format (#RRGGBB) in crates/tusk_core/src/models/settings.rs
- [ ] T007 [P] Add QueryFolder::new() constructor with name and optional parent_id parameters in crates/tusk_core/src/models/settings.rs
- [ ] T008 [P] Add QueryFolder::validate() method to check name length (1-100) in crates/tusk_core/src/models/settings.rs
- [ ] T009 [P] Add Default impl for AppSettings with values from data-model.md (theme: "dark", font_size: 14, etc.) in crates/tusk_core/src/models/settings.rs
- [ ] T010 [P] Add Default impl for WindowState with values from contracts/storage_api.md (x: 100, y: 100, width: 1280, height: 800, etc.) in crates/tusk_core/src/models/settings.rs
- [ ] T011 Export storage types (ExportData, ConflictResolution, ImportResult) from crates/tusk_core/src/services/mod.rs

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 5 - Connection Groups (Priority: P2) 🎯 MVP Foundation

**Goal**: Enable users to organize connections into named, colored groups with custom ordering

**Independent Test**: Create a group named "Production" with color #FF5733, assign a connection to it, reorder groups, close and reopen the application, verify the group structure and assignments persist.

**Rationale for MVP**: Connection groups are foundational infrastructure needed by US1 (group_id field in connections). Implementing US5 first establishes the schema and CRUD operations that US1 depends on.

### Tests for User Story 5

- [ ] T012 [P] [US5] Write unit test test_connection_group_crud for save/load/delete/list operations in crates/tusk_core/src/services/storage.rs
- [ ] T013 [P] [US5] Write unit test test_connection_group_validation for name length and color format validation in crates/tusk_core/src/services/storage.rs
- [ ] T014 [P] [US5] Write unit test test_connection_group_reorder for sort order persistence in crates/tusk_core/src/services/storage.rs

### Implementation for User Story 5

- [ ] T015 [US5] Implement save_connection_group() with INSERT OR REPLACE on connection_groups table in crates/tusk_core/src/services/storage.rs
- [ ] T016 [US5] Implement load_connection_group(id: Uuid) returning Option<ConnectionGroup> in crates/tusk_core/src/services/storage.rs
- [ ] T017 [US5] Implement load_all_connection_groups() returning Vec<ConnectionGroup> ordered by sort_order in crates/tusk_core/src/services/storage.rs
- [ ] T018 [US5] Implement delete_connection_group(id: Uuid) which sets group_id=NULL on connections in that group in crates/tusk_core/src/services/storage.rs
- [ ] T019 [US5] Implement reorder_connection_groups(order: &[(Uuid, i32)]) for batch sort_order updates in crates/tusk_core/src/services/storage.rs

**Checkpoint**: Connection groups CRUD complete - can now assign connections to groups

---

## Phase 4: User Story 1 - Connection Persistence (Priority: P1)

**Goal**: Extend connection persistence to include group assignment

**Independent Test**: Create a connection, assign it to a group, modify settings, close and reopen the application. The connection should appear in its group with all settings intact.

**Note**: Basic connection CRUD already exists. This phase adds group_id support.

### Tests for User Story 1

- [ ] T020 [P] [US1] Write unit test test_connection_with_group for save/load with group_id in crates/tusk_core/src/services/storage.rs
- [ ] T021 [P] [US1] Write unit test test_connection_group_cascade for group deletion setting connection.group_id to NULL in crates/tusk_core/src/services/storage.rs

### Implementation for User Story 1

- [ ] T022 [US1] Add group_id: Option<Uuid> field to ConnectionConfig struct in crates/tusk_core/src/models/connection.rs
- [ ] T023 [US1] Update ConnectionConfigRow struct to include group_id field in crates/tusk_core/src/services/storage.rs
- [ ] T024 [US1] Update save_connection() to persist group_id column in crates/tusk_core/src/services/storage.rs
- [ ] T025 [US1] Update load_connection() and load_all_connections() queries to SELECT group_id in crates/tusk_core/src/services/storage.rs
- [ ] T026 [US1] Update row_to_connection_config() to parse and include group_id in crates/tusk_core/src/services/storage.rs
- [ ] T027 [US1] Implement assign_connection_to_group(connection_id: Uuid, group_id: Option<Uuid>) in crates/tusk_core/src/services/storage.rs

**Checkpoint**: Connections can now be assigned to groups and persist across sessions

---

## Phase 5: User Story 2 - Query History Tracking (Priority: P1)

**Goal**: Add automatic history pruning to keep history within configured limits

**Independent Test**: Execute 100 queries with limit set to 50, verify only the 50 most recent remain. Set max age to 1 day, verify old entries are pruned.

**Note**: Basic history CRUD already exists. This phase adds pruning.

### Tests for User Story 2

- [ ] T028 [P] [US2] Write unit test test_prune_history_by_count for pruning to N entries in crates/tusk_core/src/services/storage.rs
- [ ] T029 [P] [US2] Write unit test test_prune_history_by_age for removing entries older than duration in crates/tusk_core/src/services/storage.rs
- [ ] T030 [P] [US2] Write unit test test_auto_prune_on_add for automatic pruning when limit exceeded in crates/tusk_core/src/services/storage.rs

### Implementation for User Story 2

- [ ] T031 [US2] Implement prune_history(limit: usize) -> Result<usize> deleting oldest entries beyond limit in crates/tusk_core/src/services/storage.rs
- [ ] T032 [US2] Implement prune_history_by_age(max_age: chrono::Duration) -> Result<usize> in crates/tusk_core/src/services/storage.rs
- [ ] T033 [US2] Modify add_to_history() to call prune_history() when count exceeds configured limit in crates/tusk_core/src/services/storage.rs
- [ ] T034 [US2] Add history_count() method to efficiently count history entries in crates/tusk_core/src/services/storage.rs

**Checkpoint**: Query history now auto-prunes to stay within limits

---

## Phase 6: User Story 3 - Saved Queries Library (Priority: P2)

**Goal**: Enable hierarchical folder organization for saved queries

**Independent Test**: Create folder "Reports", create subfolder "Monthly" under it, save a query into "Monthly", move it to "Reports", delete "Monthly", verify query remains in "Reports".

### Tests for User Story 3

- [ ] T035 [P] [US3] Write unit test test_query_folder_crud for save/load/delete operations in crates/tusk_core/src/services/storage.rs
- [ ] T036 [P] [US3] Write unit test test_query_folder_hierarchy for parent-child relationships in crates/tusk_core/src/services/storage.rs
- [ ] T037 [P] [US3] Write unit test test_query_folder_cycle_detection for preventing circular parent references in crates/tusk_core/src/services/storage.rs
- [ ] T038 [P] [US3] Write unit test test_query_folder_max_depth for enforcing 5-level limit in crates/tusk_core/src/services/storage.rs
- [ ] T039 [P] [US3] Write unit test test_move_query_to_folder for folder_id updates in crates/tusk_core/src/services/storage.rs

### Implementation for User Story 3

- [ ] T040 [US3] Implement save_query_folder() with cycle detection and max depth (5) validation in crates/tusk_core/src/services/storage.rs
- [ ] T041 [US3] Implement load_query_folder(id: Uuid) returning Option<QueryFolder> in crates/tusk_core/src/services/storage.rs
- [ ] T042 [US3] Implement load_query_folders(parent_id: Option<Uuid>) for filtered listing in crates/tusk_core/src/services/storage.rs
- [ ] T043 [US3] Implement load_all_query_folders() returning all folders in crates/tusk_core/src/services/storage.rs
- [ ] T044 [US3] Implement delete_query_folder(id: Uuid) with recursive descendant deletion in crates/tusk_core/src/services/storage.rs
- [ ] T045 [US3] Implement is_descendant_of(folder_id: Uuid, potential_ancestor: Uuid) helper for cycle detection in crates/tusk_core/src/services/storage.rs
- [ ] T046 [US3] Implement get_folder_depth(folder_id: Uuid) helper for depth validation in crates/tusk_core/src/services/storage.rs
- [ ] T047 [US3] Implement move_query_to_folder(query_id: Uuid, folder_id: Option<Uuid>) in crates/tusk_core/src/services/storage.rs
- [ ] T048 [US3] Add folder_id: Option<Uuid> field to SavedQuery struct (alongside existing folder_path) in crates/tusk_core/src/services/storage.rs
- [ ] T049 [US3] Update save_query() to persist folder_id column in crates/tusk_core/src/services/storage.rs
- [ ] T050 [US3] Update load_saved_query() and load_all_saved_queries() to include folder_id in crates/tusk_core/src/services/storage.rs

**Checkpoint**: Saved queries can now be organized into hierarchical folders

---

## Phase 7: User Story 6 - Application Settings (Priority: P2)

**Goal**: Persist typed application settings with defaults and immediate write-through

**Independent Test**: Modify theme to "light", font_size to 18, query_timeout to 30, close and reopen application. Verify all settings retain modified values.

### Tests for User Story 6

- [ ] T051 [P] [US6] Write unit test test_load_settings_with_defaults for missing keys returning defaults in crates/tusk_core/src/services/storage.rs
- [ ] T052 [P] [US6] Write unit test test_save_setting_individual for single key updates in crates/tusk_core/src/services/storage.rs
- [ ] T053 [P] [US6] Write unit test test_reset_settings for clearing all to defaults in crates/tusk_core/src/services/storage.rs

### Implementation for User Story 6

- [ ] T054 [US6] Implement load_settings() loading from ui_state with "settings." prefix and merging with defaults in crates/tusk_core/src/services/storage.rs
- [ ] T055 [US6] Implement save_setting<T: Serialize>(key: &str, value: &T) writing to ui_state with "settings." prefix in crates/tusk_core/src/services/storage.rs
- [ ] T056 [US6] Implement reset_settings() deleting all "settings.*" keys from ui_state in crates/tusk_core/src/services/storage.rs
- [ ] T057 [US6] Add AppSettings::load(storage: &LocalStorage) convenience method in crates/tusk_core/src/models/settings.rs
- [ ] T058 [US6] Add AppSettings::save_all(storage: &LocalStorage) to persist entire settings struct in crates/tusk_core/src/models/settings.rs

**Checkpoint**: Application settings persist with typed access and defaults

---

## Phase 8: User Story 4 - Editor State Restoration (Priority: P2)

**Goal**: Persist editor tabs with content, cursor position, and selection state

**Independent Test**: Open 3 editor tabs with different SQL content and cursor positions, close application, reopen. Verify all tabs restored with exact content and cursor positions.

### Tests for User Story 4

- [ ] T059 [P] [US4] Write unit test test_save_load_editor_tabs for round-trip persistence in crates/tusk_core/src/services/storage.rs
- [ ] T060 [P] [US4] Write unit test test_editor_tabs_empty_state for no saved tabs returning empty vec in crates/tusk_core/src/services/storage.rs

### Implementation for User Story 4

- [ ] T061 [US4] Implement save_editor_tabs(tabs: &[EditorTab]) serializing to ui_state key "editor_tabs" in crates/tusk_core/src/services/storage.rs
- [ ] T062 [US4] Implement load_editor_tabs() -> Result<Vec<EditorTab>> deserializing from ui_state in crates/tusk_core/src/services/storage.rs
- [ ] T063 [US4] Add EditorTab::new() constructor with default values in crates/tusk_core/src/models/settings.rs

**Checkpoint**: Editor state persists across application restarts

---

## Phase 9: User Story 7 - Window State Persistence (Priority: P3)

**Goal**: Persist window geometry and panel layout

**Independent Test**: Resize window to 1600x900, adjust sidebar width to 300, close and reopen. Verify window appears at exact size and layout.

### Tests for User Story 7

- [ ] T064 [P] [US7] Write unit test test_save_load_window_state for round-trip persistence in crates/tusk_core/src/services/storage.rs
- [ ] T065 [P] [US7] Write unit test test_window_state_default for no saved state returning None in crates/tusk_core/src/services/storage.rs

### Implementation for User Story 7

- [ ] T066 [US7] Implement save_window_state(state: &WindowState) serializing to ui_state key "window_state" in crates/tusk_core/src/services/storage.rs
- [ ] T067 [US7] Implement load_window_state() -> Result<Option<WindowState>> deserializing from ui_state in crates/tusk_core/src/services/storage.rs

**Checkpoint**: Window state persists across application restarts

---

## Phase 10: User Story 8 - Data Export and Import (Priority: P3)

**Goal**: Enable backup and restore of all user data (excluding passwords)

**Independent Test**: Create connections, groups, folders, queries, settings. Export to file. Delete database. Import from file. Verify all data restored correctly.

### Tests for User Story 8

- [ ] T068 [P] [US8] Write unit test test_export_data_structure for correct ExportData format in crates/tusk_core/src/services/storage.rs
- [ ] T069 [P] [US8] Write unit test test_import_data_skip_conflicts for ConflictResolution::Skip behavior in crates/tusk_core/src/services/storage.rs
- [ ] T070 [P] [US8] Write unit test test_import_data_replace_conflicts for ConflictResolution::Replace behavior in crates/tusk_core/src/services/storage.rs
- [ ] T071 [P] [US8] Write unit test test_import_data_rename_conflicts for ConflictResolution::Rename behavior in crates/tusk_core/src/services/storage.rs
- [ ] T072 [P] [US8] Write unit test test_export_import_round_trip for data integrity in crates/tusk_core/src/services/storage.rs

### Implementation for User Story 8

- [ ] T073 [US8] Add ConnectionConfigExport struct (ConnectionConfig without password) in crates/tusk_core/src/models/settings.rs
- [ ] T074 [US8] Implement export_data() -> Result<ExportData> collecting all entities in crates/tusk_core/src/services/storage.rs
- [ ] T075 [US8] Implement import_connection_groups() helper for batch group import with conflict handling in crates/tusk_core/src/services/storage.rs
- [ ] T076 [US8] Implement import_connections() helper for batch connection import with conflict handling in crates/tusk_core/src/services/storage.rs
- [ ] T077 [US8] Implement import_query_folders() helper preserving hierarchy in crates/tusk_core/src/services/storage.rs
- [ ] T078 [US8] Implement import_saved_queries() helper for batch query import with conflict handling in crates/tusk_core/src/services/storage.rs
- [ ] T079 [US8] Implement import_data(data: &ExportData, conflict: ConflictResolution) -> Result<ImportResult> orchestrating all imports in crates/tusk_core/src/services/storage.rs
- [ ] T080 [US8] Add version compatibility check in import_data() for version field validation in crates/tusk_core/src/services/storage.rs

**Checkpoint**: Users can backup and restore all application data

---

## Phase 11: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T081 [P] Add tracing::debug! logging to all new storage methods in crates/tusk_core/src/services/storage.rs
- [ ] T082 [P] Write integration test test_migration_upgrade for Migration 1 → 2 data preservation in crates/tusk_core/src/services/storage.rs
- [ ] T083 [P] Write performance test test_load_100_connections for SC-001 (<200ms) in crates/tusk_core/src/services/storage.rs
- [ ] T084 [P] Write performance test test_load_10k_history for SC-002 (<100ms) in crates/tusk_core/src/services/storage.rs
- [ ] T085 Run quickstart.md verification checklist manually and document results
- [ ] T086 [P] Write unit test test_database_recovery_on_corruption for graceful recreation when database file is corrupted in crates/tusk_core/src/services/storage.rs
- [ ] T087 [P] Write unit test test_database_recovery_on_missing for graceful recreation when database file is missing in crates/tusk_core/src/services/storage.rs

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-10)**: All depend on Foundational phase completion
  - US5 (Phase 3) must complete before US1 (Phase 4) - groups needed for connections
  - US1, US2, US3, US4, US6, US7 can proceed in parallel after their dependencies
  - US8 (Phase 10) depends on all other user stories - needs complete data model
- **Polish (Phase 11)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 5 (Connection Groups)**: First - establishes group infrastructure
- **User Story 1 (Connection Persistence)**: Depends on US5 - needs group_id field
- **User Story 2 (Query History)**: Can start after Foundational - independent
- **User Story 3 (Saved Queries)**: Can start after Foundational - independent
- **User Story 4 (Editor State)**: Can start after Foundational - independent
- **User Story 6 (App Settings)**: Can start after Foundational - independent
- **User Story 7 (Window State)**: Can start after Foundational - independent
- **User Story 8 (Export/Import)**: Depends on US1-US7 - needs all entities

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Models/types before service methods
- CRUD helpers before public API methods
- Validation before persistence

### Parallel Opportunities

- All Setup tasks can run sequentially (single file)
- All Foundational tasks marked [P] can run in parallel
- After Phase 3 (US5) completes:
  - US1, US2, US3, US4, US6, US7 can all proceed in parallel
- Within each user story:
  - All tests marked [P] can run in parallel
  - Implementation tasks are sequential (same file)

---

## Parallel Example: User Story 3 (Query Folders)

```bash
# Launch all tests for User Story 3 together:
Task: "Write unit test test_query_folder_crud in crates/tusk_core/src/services/storage.rs"
Task: "Write unit test test_query_folder_hierarchy in crates/tusk_core/src/services/storage.rs"
Task: "Write unit test test_query_folder_cycle_detection in crates/tusk_core/src/services/storage.rs"
Task: "Write unit test test_query_folder_max_depth in crates/tusk_core/src/services/storage.rs"
Task: "Write unit test test_move_query_to_folder in crates/tusk_core/src/services/storage.rs"
```

---

## Implementation Strategy

### MVP First (Phase 1-4)

1. Complete Phase 1: Setup (models, migration)
2. Complete Phase 2: Foundational (constructors, defaults)
3. Complete Phase 3: User Story 5 (connection groups) - establishes infrastructure
4. Complete Phase 4: User Story 1 (connection persistence with groups)
5. **STOP and VALIDATE**: Test connection groups and persistence independently
6. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 5 → Test groups → Groups working
3. Add User Story 1 → Test connections with groups → Connections grouped (MVP!)
4. Add User Story 2 → Test history pruning → History managed
5. Add User Story 3 → Test folders → Queries organized
6. Add User Story 6 → Test settings → Preferences saved
7. Add User Story 4 → Test tabs → Editor state restored
8. Add User Story 7 → Test window → Layout persisted
9. Add User Story 8 → Test export/import → Backup/restore working
10. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Developer A completes US5, then US1
3. Once US5 done:
   - Developer B: US2, US3
   - Developer C: US4, US6, US7
4. All reconvene for US8 (needs complete model)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing
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
