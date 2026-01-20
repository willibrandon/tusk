# Tasks: Frontend Architecture

**Input**: Design documents from `/specs/003-frontend-architecture/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Tests are NOT explicitly requested in the feature specification. Test tasks are excluded.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Project type**: Tauri v2 application with Svelte frontend
- **Frontend**: `src/` (Svelte/SvelteKit)
- **Backend**: `src-tauri/` (Rust - not modified by this feature)
- **Types**: `src/lib/types/`
- **Stores**: `src/lib/stores/`
- **Components**: `src/lib/components/`
- **Utilities**: `src/lib/utils/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization, types, and utility modules

- [x] T001 Copy type contracts from `specs/003-frontend-architecture/contracts/tab.ts` to `src/lib/types/tab.ts`
- [x] T002 [P] Copy type contracts from `specs/003-frontend-architecture/contracts/connection.ts` to `src/lib/types/connection.ts`
- [x] T003 [P] Copy type contracts from `specs/003-frontend-architecture/contracts/ui.ts` to `src/lib/types/ui.ts`
- [x] T004 Create type exports barrel file in `src/lib/types/index.ts`
- [x] T005 Create localStorage helper utilities in `src/lib/utils/storage.ts` with error handling and type-safe get/set
- [x] T006 [P] Create keyboard utility module in `src/lib/utils/keyboard.ts` with platform detection (isMac) and modifier key helpers
- [x] T007 Create utility exports barrel file in `src/lib/utils/index.ts`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core stores and base components that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T008 Create UI store in `src/lib/stores/ui.svelte.ts` implementing UIStoreInterface with sidebar width/collapsed state and localStorage persistence
- [x] T009 Create tab store in `src/lib/stores/tabs.svelte.ts` implementing TabStoreInterface with tab CRUD operations and localStorage persistence
- [x] T010 [P] Create connections store in `src/lib/stores/connections.svelte.ts` implementing ConnectionStoreInterface (placeholder for future backend integration)
- [x] T011 Enhance existing theme store in `src/lib/stores/theme.svelte.ts` to implement ThemeStoreInterface with three-way preference (light/dark/system) and system preference tracking
- [x] T012 Create store exports barrel file in `src/lib/stores/index.ts` exporting all stores
- [x] T013 [P] Create Button component in `src/lib/components/common/Button.svelte` with variant support (primary, secondary, ghost, danger)
- [x] T014 [P] Create Icon component in `src/lib/components/common/Icon.svelte` for consistent icon rendering
- [x] T015 Update SvelteKit layout configuration in `src/routes/+layout.ts` to disable SSR and enable prerender for Tauri

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Application Shell Layout (Priority: P1) 🎯 MVP

**Goal**: Display a three-region layout with sidebar (left), main content area with tab bar (center/right), and status bar (bottom)

**Independent Test**: Launch the application and verify the three main regions are visible, properly positioned, and visually distinct

### Implementation for User Story 1

- [x] T016 [US1] Create Resizer component in `src/lib/components/shell/Resizer.svelte` with drag handle, pointer event capture, and ARIA separator role
- [x] T017 [US1] Create StatusBar component in `src/lib/components/shell/StatusBar.svelte` displaying "No connection" state with connection status indicator area
- [x] T018 [US1] Create empty TabBar placeholder component in `src/lib/components/shell/TabBar.svelte` with container structure (full implementation in US2)
- [x] T019 [US1] Create SidebarSearch component in `src/lib/components/shell/SidebarSearch.svelte` with filter input for connections
- [x] T020 [US1] Create SidebarHeader component in `src/lib/components/shell/SidebarHeader.svelte` displaying "Connections" title with "New Connection" button
- [x] T021 [US1] Create Sidebar component in `src/lib/components/shell/Sidebar.svelte` composing SidebarHeader, SidebarSearch, and placeholder tree area with resizable width
- [x] T022 [US1] Create Shell component in `src/lib/components/shell/Shell.svelte` composing Sidebar, Resizer, main content area, and StatusBar in three-region flexbox layout
- [x] T023 [US1] Update root layout in `src/routes/+layout.svelte` to wrap content with Shell component and global keyboard handler
- [x] T024 [US1] Update main page in `src/routes/+page.svelte` to display tab content area with empty state message

**Checkpoint**: User Story 1 complete - shell renders with sidebar, main area, and status bar

---

## Phase 4: User Story 2 - Tab Management (Priority: P1)

**Goal**: Enable users to create, switch, close, and reorder tabs with unsaved changes protection

**Independent Test**: Create new tabs, click to switch between them, close tabs (with and without modifications), drag to reorder, verify confirmation dialog for unsaved changes

### Implementation for User Story 2

- [x] T025 [US2] Create ConfirmDialog component in `src/lib/components/dialogs/ConfirmDialog.svelte` with Save/Discard/Cancel buttons, focus trap, and ESC handling
- [x] T026 [US2] Create Tab component in `src/lib/components/shell/Tab.svelte` with title, type icon, close button, modification indicator (blue dot), connection color, drag handle, and middle-click close
- [x] T027 [US2] Implement full TabBar component in `src/lib/components/shell/TabBar.svelte` with tab rendering, "New Tab" button, drag-and-drop reordering, and active tab highlighting
- [x] T028 [US2] Add tab-related keyboard shortcuts (Cmd/Ctrl+T new tab, Cmd/Ctrl+W close tab, Cmd/Ctrl+Tab/Shift+Tab cycle tabs) to keyboard handler in `src/routes/+layout.svelte`
- [x] T029 [US2] Update main page in `src/routes/+page.svelte` to display active tab content and empty state when no tabs are open
- [x] T030 [US2] Integrate ConfirmDialog with tab store closeTab() method to prompt on unsaved changes

**Checkpoint**: User Story 2 complete - full tab management with create, switch, close, reorder, and unsaved changes dialog

---

## Phase 5: User Story 3 - Resizable Sidebar Panel (Priority: P2)

**Goal**: Allow users to resize the sidebar by dragging its edge with min/max constraints and persistence

**Independent Test**: Click and drag the sidebar resize handle, verify width changes, verify min (200px) and max (500px) constraints, reload and verify persistence

### Implementation for User Story 3

- [x] T031 [US3] Implement resize logic in Resizer component in `src/lib/components/shell/Resizer.svelte` with pointerdown/move/up handlers and requestAnimationFrame throttling
- [x] T032 [US3] Connect Resizer to UI store in `src/lib/components/shell/Shell.svelte` to update sidebarWidth on drag with clamping
- [x] T033 [US3] Add keyboard resize support to Resizer (Arrow keys for ±10px adjustments when focused) in `src/lib/components/shell/Resizer.svelte`

**Checkpoint**: User Story 3 complete - sidebar resizes smoothly with constraints and persists across reloads

---

## Phase 6: User Story 4 - Sidebar Toggle (Priority: P2)

**Goal**: Allow users to collapse and expand the sidebar using Cmd/Ctrl+B keyboard shortcut with state persistence

**Independent Test**: Press Cmd/Ctrl+B to collapse sidebar, verify it disappears, press again to expand, reload and verify state persisted

### Implementation for User Story 4

- [x] T034 [US4] Add sidebar collapse/expand logic to Sidebar component in `src/lib/components/shell/Sidebar.svelte` using sidebarCollapsed state from UI store
- [x] T035 [US4] Add Cmd/Ctrl+B keyboard shortcut to toggle sidebar in keyboard handler in `src/routes/+layout.svelte`
- [x] T036 [US4] Hide Resizer when sidebar is collapsed in `src/lib/components/shell/Shell.svelte`

**Checkpoint**: User Story 4 complete - sidebar toggles with Cmd/Ctrl+B and state persists

---

## Phase 7: User Story 5 - Connection Status Display (Priority: P2)

**Goal**: Display current connection status with visual indicators (green/yellow/red/gray) and connection details in status bar

**Independent Test**: Set connection state via store, verify status bar shows correct color indicator and connection info (name, host, port), verify "No connection" when disconnected

### Implementation for User Story 5

- [x] T037 [US5] Enhance StatusBar component in `src/lib/components/shell/StatusBar.svelte` to display connection name, host, port, and colored status indicator based on ConnectionState
- [x] T038 [US5] Add cursor position display area (line, column) to StatusBar in `src/lib/components/shell/StatusBar.svelte` (shows when editor tab active)
- [x] T039 [US5] Add query result info display area (row count, execution time) to StatusBar in `src/lib/components/shell/StatusBar.svelte` (shows after query execution)

**Checkpoint**: User Story 5 complete - status bar displays connection status with visual indicators and additional info

---

## Phase 8: User Story 6 - Theme Support (Priority: P3)

**Goal**: Allow users to switch between light, dark, and system-following themes with persistence and no flash of wrong theme

**Independent Test**: Change theme via settings (or store), verify visual appearance changes, set to system and toggle OS dark mode, reload and verify persistence with no flash

### Implementation for User Story 6

- [x] T040 [US6] Add FOUC prevention inline script to `src/app.html` that applies dark class before Svelte hydration
- [x] T041 [US6] Add system preference media query listener to theme store in `src/lib/stores/theme.svelte.ts` for live updates when OS theme changes
- [x] T042 [US6] Ensure all shell components use Tailwind dark: variants for proper theme support across `src/lib/components/shell/*.svelte`
- [x] T043 [US6] Create theme toggle control (can be temporary UI or keyboard shortcut) for testing theme switching

**Checkpoint**: User Story 6 complete - themes switch correctly with no flash and system preference is tracked

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Final integration, accessibility audit, and performance verification

- [x] T044 [P] Verify all interactive elements have keyboard access and proper focus styles across all components
- [x] T045 [P] Add ARIA attributes to Tab components (role="tab", aria-selected) and TabBar (role="tablist") in `src/lib/components/shell/Tab.svelte` and `src/lib/components/shell/TabBar.svelte`
- [x] T046 Add aria-orientation="vertical" to Resizer component in `src/lib/components/shell/Resizer.svelte`
- [x] T047 Verify localStorage error handling and graceful degradation in `src/lib/utils/storage.ts`
- [x] T048 Verify performance targets: shell render <1s, tab operations <100ms, resize at 60fps
- [x] T049 Run `npm run check` to verify TypeScript compilation passes
- [x] T050 Run `npm run lint` to verify code style compliance
- [x] T051 Run quickstart.md verification checklist manually
- [x] T052 [P] Add user-select: none CSS to UI chrome elements (sidebar, tab bar, status bar, resizer) while preserving selection in content areas per FR-002 in `src/app.css` or component styles

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-8)**: All depend on Foundational phase completion
  - User stories can proceed in priority order (P1 → P1 → P2 → P2 → P2 → P3)
  - Some parallelism possible: US3, US4, US5 can start once US1+US2 complete
- **Polish (Phase 9)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P1)**: Can start after Foundational (Phase 2) - Uses shell structure from US1
- **User Story 3 (P2)**: Depends on US1 (Resizer component exists) - Independent enhancement
- **User Story 4 (P2)**: Depends on US1 (Sidebar component exists) - Independent enhancement
- **User Story 5 (P2)**: Depends on US1 (StatusBar component exists) - Independent enhancement
- **User Story 6 (P3)**: Depends on Foundational (theme store) - Can be parallel with US3-US5

### Within Each User Story

- Component dependencies: Shell → Sidebar/TabBar/StatusBar → subcomponents
- Store changes before component usage
- Story complete before moving to next priority

### Parallel Opportunities

- T001, T002, T003 can run in parallel (independent type files)
- T005, T006 can run in parallel (independent utility files)
- T008, T010 can run in parallel (independent stores)
- T013, T014 can run in parallel (independent base components)
- Once US1 completes, US3, US4, US5 can run in parallel (different components)
- T044, T045 can run in parallel (different accessibility concerns)

---

## Parallel Example: Setup Phase

```bash
# Launch all type copy tasks together:
Task: "Copy type contracts from specs/003-frontend-architecture/contracts/tab.ts to src/lib/types/tab.ts"
Task: "Copy type contracts from specs/003-frontend-architecture/contracts/connection.ts to src/lib/types/connection.ts"
Task: "Copy type contracts from specs/003-frontend-architecture/contracts/ui.ts to src/lib/types/ui.ts"
```

## Parallel Example: Post-US1 User Stories

```bash
# After US1+US2 complete, these can run in parallel:
Task: US3 - Resizable Sidebar (T031-T033)
Task: US4 - Sidebar Toggle (T034-T036)
Task: US5 - Connection Status Display (T037-T039)
```

---

## Implementation Strategy

### MVP First (User Stories 1+2)

1. Complete Phase 1: Setup (types and utilities)
2. Complete Phase 2: Foundational (stores and base components)
3. Complete Phase 3: User Story 1 (shell layout)
4. Complete Phase 4: User Story 2 (tab management)
5. **STOP and VALIDATE**: Test US1+US2 independently - this is the MVP
6. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (shell visible)
3. Add User Story 2 → Test independently → Deploy/Demo (full tab management - MVP!)
4. Add User Story 3 → Test independently → Deploy/Demo (resizable sidebar)
5. Add User Story 4 → Test independently → Deploy/Demo (sidebar toggle)
6. Add User Story 5 → Test independently → Deploy/Demo (connection status)
7. Add User Story 6 → Test independently → Deploy/Demo (theme support)
8. Polish phase → Final validation

### Suggested MVP Scope

**MVP = User Story 1 + User Story 2** (Phases 1-4, Tasks T001-T030)

This delivers:
- Complete application shell with sidebar, main area, status bar
- Full tab management with create, switch, close, reorder
- Unsaved changes protection
- Basic status bar showing "No connection"
- Foundation for all future features

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
