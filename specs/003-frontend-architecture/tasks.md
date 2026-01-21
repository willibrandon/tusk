# Tasks: Frontend Architecture

**Input**: Design documents from `/specs/003-frontend-architecture/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Not explicitly requested in the feature specification. Test tasks are omitted.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Cargo workspace**: `crates/tusk_ui/src/` for UI components
- **Main app**: `crates/tusk/src/` for application entry point

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization, dependencies, and core utilities

- [ ] T001 Update `crates/tusk_ui/Cargo.toml` with dependencies: smallvec, uuid, parking_lot, serde, tusk_core
- [ ] T002 [P] Extend `crates/tusk_ui/src/theme.rs` with additional colors per research.md (tab_bar_background, panel_background, list_active_selection_background, element_background, elevated_surface_background, etc.)
- [ ] T003 [P] Create `crates/tusk_ui/src/layout.rs` with layout utilities (h_flex, v_flex wrappers, spacing constants)
- [ ] T004 [P] Create `crates/tusk_ui/src/spinner.rs` with Spinner component implementing RenderOnce and animation via with_animation()
- [ ] T005 Rename `crates/tusk_ui/src/icons.rs` to `crates/tusk_ui/src/icon.rs` and expand with full IconName enum (ChevronRight, ChevronDown, Database, Table, Column, Key, etc.) and IconSize enum per contracts/components.md

**Checkpoint**: Core utilities ready. Foundation for all components established.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**WARNING**: No user story work can begin until this phase is complete

- [ ] T006 Create `crates/tusk_ui/src/key_bindings.rs` with global actions (ToggleLeftDock, ToggleRightDock, ToggleBottomDock, NewQueryTab, CloseActiveTab, etc.) and register_key_bindings() function per contracts/keyboard.md
- [ ] T007 [P] Create `crates/tusk_ui/src/button.rs` with Button component implementing RenderOnce, ButtonVariant enum (Primary/Secondary/Ghost/Danger), ButtonSize enum (Small/Medium/Large), and fluent builder pattern per contracts/components.md
- [ ] T008 [P] Create `crates/tusk_ui/src/resizer.rs` with Resizer component implementing RenderOnce for Axis::Horizontal and Axis::Vertical resize handles per contracts/resizer.md
- [ ] T009 [P] Create `crates/tusk_ui/src/panel.rs` with Panel trait definition (panel_id, title, icon, focus, closable, is_dirty, position), PanelEvent enum, and AnyPanel wrapper type for type-erased panel storage per contracts/panel.md
- [ ] T010 Create `crates/tusk_ui/src/pane.rs` with TabItem struct, Pane struct implementing Render/Focusable/EventEmitter<PaneEvent>, and PaneNode enum (Single/Split) per contracts/pane.md
- [ ] T011 Create PaneGroup struct in `crates/tusk_ui/src/pane.rs` implementing Render/Focusable/EventEmitter<PaneGroupEvent> with split(), close_pane(), resize_split() methods per contracts/pane.md
- [ ] T012 Create `crates/tusk_ui/src/dock.rs` with Dock struct implementing Render/Focusable/EventEmitter<DockEvent>, DockPosition enum (Left/Right/Bottom), size constraints (120px-600px for side, 100px-50vh for bottom) per contracts/dock.md
- [ ] T013 Create `crates/tusk_ui/src/workspace.rs` with Workspace struct implementing Render/Focusable/EventEmitter<WorkspaceEvent>, containing left_dock, bottom_dock, center PaneGroup, and status_bar per contracts/workspace.md
- [ ] T014 Add WorkspaceState struct for persistence (dock sizes, visibility, pane layout) in `crates/tusk_ui/src/workspace.rs` with Serialize/Deserialize per contracts/workspace.md
- [ ] T015 Update `crates/tusk_ui/src/lib.rs` with all module declarations and public exports for workspace, dock, pane, panel, button, icon, spinner, resizer, key_bindings, layout, theme, confirm_dialog
- [ ] T121 Create `crates/tusk_ui/src/confirm_dialog.rs` with ConfirmDialog struct implementing Render/Focusable for simple yes/no confirmations (title, message, confirm_label, cancel_label, on_confirm, on_cancel) - lightweight alternative to full Modal for immediate use in P1 stories

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Basic Workspace Shell (Priority: P1)

**Goal**: Launch Tusk and see a complete workspace with left sidebar, main content area, and status bar

**Independent Test**: Launch the application and verify the workspace renders with all three regions (left dock, center workspace, status bar) visible and properly sized

### Implementation for User Story 1

- [ ] T016 [US1] Create `crates/tusk_ui/src/status_bar.rs` with StatusBar struct implementing Render, displaying connection status placeholder (left) and execution state placeholder (right) per contracts/components.md
- [ ] T017 [US1] Implement Workspace::new() constructor in `crates/tusk_ui/src/workspace.rs` creating left_dock (240px default), bottom_dock, center PaneGroup, and StatusBar
- [ ] T018 [US1] Implement Workspace::render() in `crates/tusk_ui/src/workspace.rs` with layout: left dock (conditional), center pane group (flex-grow), status bar (bottom)
- [ ] T019 [US1] Create `crates/tusk_ui/src/panels/mod.rs` with module structure for panels
- [ ] T020 [US1] Create `crates/tusk_ui/src/panels/schema_browser.rs` with SchemaBrowserPanel struct implementing Panel trait (placeholder content for now)
- [ ] T021 [US1] Wire up workspace in `crates/tusk/src/app.rs` to create and render Workspace as root component
- [ ] T022 [US1] Register left dock with SchemaBrowserPanel in Workspace initialization in `crates/tusk_ui/src/workspace.rs`

**Checkpoint**: User Story 1 complete - application launches with visible workspace shell (left dock, center area, status bar)

---

## Phase 4: User Story 2 - Dock Resizing (Priority: P1)

**Goal**: Resize docks by dragging their edges to customize workspace layout with persistence

**Independent Test**: Drag a dock edge and verify the dock resizes smoothly while respecting min/max constraints (120px-600px for side docks)

### Implementation for User Story 2

- [ ] T023 [US2] Implement Dock resize via Resizer component in `crates/tusk_ui/src/dock.rs` render method, placing Resizer at dock edge
- [ ] T024 [US2] Implement Dock::set_size() with constraint clamping (min 120px, max 600px for side docks) in `crates/tusk_ui/src/dock.rs`
- [ ] T025 [US2] Add on_resize callback from Resizer to Dock.set_size() in `crates/tusk_ui/src/dock.rs` render method
- [ ] T026 [US2] Implement WorkspaceState persistence: save_state() and restore_state() methods in `crates/tusk_ui/src/workspace.rs`
- [ ] T027 [US2] Add persistence load on Workspace::new() and save on dock resize in `crates/tusk_ui/src/workspace.rs`
- [ ] T108 [US2] Implement dock collapsed state with toggle indicator (collapsed/expanded chevron icon) and smooth visibility transition (FR-007) in `crates/tusk_ui/src/dock.rs`
- [ ] T109 [US2] Implement bottom dock 50% viewport max height constraint with dynamic calculation on window resize in `crates/tusk_ui/src/dock.rs`

**Checkpoint**: User Story 2 complete - dock resizing works with constraints, collapse/expand, and persists across restarts

---

## Phase 5: User Story 3 - Tabbed Query Editors (Priority: P1)

**Goal**: Open multiple query editors in tabs, switch between them, and close individual tabs

**Independent Test**: Open multiple query tabs, type different content in each, switch between them, and verify content persists per tab

### Implementation for User Story 3

- [ ] T028 [US3] Implement Pane::add_tab() method to add TabItem to tabs Vec in `crates/tusk_ui/src/pane.rs`
- [ ] T029 [US3] Implement Pane::close_tab() method with dirty state check in `crates/tusk_ui/src/pane.rs`
- [ ] T030 [US3] Implement Pane::activate_tab() method to switch active tab in `crates/tusk_ui/src/pane.rs`
- [ ] T031 [US3] Implement Pane::render() with tab bar (tabs with close buttons) and content area in `crates/tusk_ui/src/pane.rs`
- [ ] T032 [US3] Render tab bar with tab items showing title, dirty indicator (*), and close button in `crates/tusk_ui/src/pane.rs`
- [ ] T033 [US3] Implement empty state UI when all tabs closed in `crates/tusk_ui/src/pane.rs` (message with "Create new query" prompt)
- [ ] T034 [US3] Wire NewQueryTab action handler in Workspace to add new tab to active pane in `crates/tusk_ui/src/workspace.rs`
- [ ] T035 [US3] Wire CloseActiveTab action handler in Workspace in `crates/tusk_ui/src/workspace.rs`
- [ ] T107 [US3] Implement tab reordering via drag-and-drop (FR-012) in `crates/tusk_ui/src/pane.rs`
- [ ] T122 [US3] Wire dirty tab close confirmation using ConfirmDialog in Pane::close_tab() - prompt "Unsaved changes will be lost. Close anyway?" when TabItem.is_dirty=true (FR-011) in `crates/tusk_ui/src/pane.rs`

**Checkpoint**: User Story 3 complete - multiple tabs work with switching, closing, reordering, and dirty indicators with save prompts

---

## Phase 6: User Story 5 - Schema Browser Navigation (Priority: P1)

**Goal**: Browse database schema in tree view, expanding databases, schemas, tables, and columns

**Independent Test**: Connect to database and verify schema tree shows databases, schemas, tables with correct hierarchy and expand/collapse behavior

### Implementation for User Story 5

- [ ] T036 [US5] Create `crates/tusk_ui/src/tree.rs` with TreeItem trait (id, label, icon, children, is_expandable) per contracts/tree.md
- [ ] T037 [US5] Implement Tree<T: TreeItem> struct with items, expanded HashSet, selected Option, focus_handle in `crates/tusk_ui/src/tree.rs`
- [ ] T038 [US5] Implement Tree expand/collapse methods (expand, collapse, toggle_expanded, expand_all, collapse_all) in `crates/tusk_ui/src/tree.rs`
- [ ] T039 [US5] Implement Tree::visible_items() returning flattened list with depth for rendering in `crates/tusk_ui/src/tree.rs`
- [ ] T040 [US5] Implement Tree::render() using UniformList for virtualization (60fps for 1000+ items) in `crates/tusk_ui/src/tree.rs`
- [ ] T041 [US5] Add tree item rendering with indentation, expand/collapse chevron, icon, and label in `crates/tusk_ui/src/tree.rs`
- [ ] T042 [US5] Implement TreeEvent emission (Selected, Expanded, Collapsed, Activated) in `crates/tusk_ui/src/tree.rs`
- [ ] T043 [US5] Create SchemaItem enum (Connection/Database/Schema/Table/View/Function/Column) implementing TreeItem in `crates/tusk_ui/src/panels/schema_browser.rs`
- [ ] T044 [US5] Implement Tree keyboard navigation actions (SelectPrevious, SelectNext, ExpandSelected, CollapseSelected, ActivateSelected) in `crates/tusk_ui/src/tree.rs`
- [ ] T045 [US5] Implement SchemaBrowserPanel::render() with Tree<SchemaItem> and filter input in `crates/tusk_ui/src/panels/schema_browser.rs`
- [ ] T046 [US5] Implement Tree::set_filter() for text filtering of visible items in `crates/tusk_ui/src/tree.rs`

**Checkpoint**: User Story 5 complete - schema browser tree works with expand/collapse, virtualization, and filtering

---

## Phase 7: User Story 10 - Keyboard Navigation (Priority: P1)

**Goal**: Navigate the entire application using keyboard shortcuts with discoverable shortcuts

**Independent Test**: Perform common tasks (new tab, switch tabs, focus panels, toggle docks) using only keyboard shortcuts

### Implementation for User Story 10

- [ ] T047 [US10] Implement tab switching shortcuts (Cmd+1-9, Cmd+Shift+[, Cmd+Shift+]) in Workspace in `crates/tusk_ui/src/workspace.rs`
- [ ] T048 [US10] Implement dock toggle action handlers (ToggleLeftDock, ToggleRightDock, ToggleBottomDock) in Workspace in `crates/tusk_ui/src/workspace.rs`
- [ ] T049 [US10] Implement panel focus shortcuts (FocusSchemaBrowser, FocusResults, FocusMessages) in Workspace in `crates/tusk_ui/src/workspace.rs`
- [ ] T050 [US10] Add key_context("Workspace") to workspace render and register all action handlers via on_action() in `crates/tusk_ui/src/workspace.rs`
- [ ] T051 [US10] Add key_context("Pane") to pane render and register tab-related action handlers in `crates/tusk_ui/src/pane.rs`
- [ ] T052 [US10] Add key_context("Tree") to tree render and register tree navigation action handlers in `crates/tusk_ui/src/tree.rs`
- [ ] T053 [US10] Implement visible focus indicators on all focusable elements (Workspace, Dock, Pane, Tree) using track_focus() and CSS focus ring

**Checkpoint**: User Story 10 complete - full keyboard navigation working for all P1 stories

---

## Phase 8: User Story 4 - Pane Splitting (Priority: P2)

**Goal**: Split editor pane horizontally or vertically to view multiple queries side by side

**Independent Test**: Split a pane, verify both panes render independently with separate tabs, then close one pane

### Implementation for User Story 4

- [ ] T054 [US4] Implement PaneGroup::split() creating new Pane and updating PaneNode to Split in `crates/tusk_ui/src/pane.rs`
- [ ] T055 [US4] Implement PaneGroup::close_pane() collapsing Split to Single when one child remains in `crates/tusk_ui/src/pane.rs`
- [ ] T056 [US4] Implement PaneGroup::resize_split() to adjust ratios between split panes in `crates/tusk_ui/src/pane.rs`
- [ ] T057 [US4] Implement PaneGroup::render() with recursive PaneNode rendering and Resizer between splits in `crates/tusk_ui/src/pane.rs`
- [ ] T058 [US4] Wire SplitRight (Cmd+\) and SplitDown (Cmd+Shift+\) action handlers in Workspace in `crates/tusk_ui/src/workspace.rs`
- [ ] T059 [US4] Implement FocusNextPane and FocusPreviousPane action handlers in Workspace in `crates/tusk_ui/src/workspace.rs`
- [ ] T060 [US4] Add pane layout persistence to WorkspaceState (PaneLayout enum: Single/Split) in `crates/tusk_ui/src/workspace.rs`

**Checkpoint**: User Story 4 complete - pane splitting works with resize and navigation

---

## Phase 9: User Story 6 - Button and Input Components (Priority: P2)

**Goal**: Interact with consistent, accessible buttons and input fields throughout the application

**Independent Test**: Tab through form with buttons and inputs, verify focus indicators, hover states, and keyboard activation all work

### Implementation for User Story 6

- [ ] T061 [US6] Add hover state styling to Button component in `crates/tusk_ui/src/button.rs` using on_hover()
- [ ] T062 [US6] Add disabled state styling and interaction blocking to Button in `crates/tusk_ui/src/button.rs`
- [ ] T063 [US6] Add loading state with Spinner to Button in `crates/tusk_ui/src/button.rs`
- [ ] T064 [US6] Create `crates/tusk_ui/src/input.rs` with TextInput struct implementing Render/Focusable/EventEmitter<TextInputEvent>
- [ ] T065 [US6] Implement TextInput::render() with text display, cursor, placeholder, and focus ring in `crates/tusk_ui/src/input.rs`
- [ ] T066 [US6] Implement TextInput keyboard handling (character input, backspace, enter for submit) in `crates/tusk_ui/src/input.rs`
- [ ] T067 [US6] Implement TextInputEvent emission (Changed, Submitted, Focus, Blur) in `crates/tusk_ui/src/input.rs`
- [ ] T068 [US6] Add focus ring styling to all interactive components (Button, TextInput) meeting WCAG 2.1 AA contrast requirements

**Checkpoint**: User Story 6 complete - buttons and inputs work with full accessibility support

---

## Phase 10: User Story 9 - Select/Dropdown Components (Priority: P2)

**Goal**: Use dropdown select components with keyboard navigation and search filtering

**Independent Test**: Click dropdown, use arrow keys to navigate options, type to filter, press Enter to select

### Implementation for User Story 9

- [ ] T069 [US9] Create `crates/tusk_ui/src/select.rs` with Select<T> struct implementing Render/Focusable/EventEmitter<SelectEvent<T>>
- [ ] T070 [US9] Implement SelectOption<T> struct (value, label, disabled) in `crates/tusk_ui/src/select.rs`
- [ ] T071 [US9] Implement Select::render() with closed state showing selected value or placeholder in `crates/tusk_ui/src/select.rs`
- [ ] T072 [US9] Implement Select dropdown popover rendering when open in `crates/tusk_ui/src/select.rs`
- [ ] T073 [US9] Implement Select keyboard navigation (Open, Close, SelectNext, SelectPrevious, Confirm) actions in `crates/tusk_ui/src/select.rs`
- [ ] T074 [US9] Implement SelectEvent emission (Changed, Opened, Closed) in `crates/tusk_ui/src/select.rs`
- [ ] T075 [US9] Add key_context("Select") and key_context("SelectPopover") for keyboard handling in `crates/tusk_ui/src/select.rs`

**Checkpoint**: User Story 9 complete - dropdowns work with full keyboard navigation

---

## Phase 11: User Story 11 - Status Bar Information (Priority: P2)

**Goal**: See relevant status information in bottom bar including connection status, database, query state, row counts

**Independent Test**: Connect to database, run query, verify status bar updates to show connection, execution time, and row count

### Implementation for User Story 11

- [ ] T076 [US11] Implement StatusBar connection status display (connected/disconnected with icon) reading from TuskState in `crates/tusk_ui/src/status_bar.rs`
- [ ] T077 [US11] Implement StatusBar database name display when connected in `crates/tusk_ui/src/status_bar.rs`
- [ ] T078 [US11] Implement StatusBar query execution state display (spinner + "Executing..." or timing) in `crates/tusk_ui/src/status_bar.rs`
- [ ] T079 [US11] Implement StatusBar row count display from last query result in `crates/tusk_ui/src/status_bar.rs`
- [ ] T080 [US11] Implement StatusBar::render() with left-aligned and right-aligned sections per FR-024 in `crates/tusk_ui/src/status_bar.rs`

**Checkpoint**: User Story 11 complete - status bar shows connection and query information

---

## Phase 12: User Story 12 - Loading States (Priority: P2)

**Goal**: See clear loading indicators when operations are in progress

**Independent Test**: Trigger slow operation, verify spinner appears and disappears appropriately

### Implementation for User Story 12

- [ ] T081 [US12] Add loading state to SchemaBrowserPanel showing Spinner during schema fetch in `crates/tusk_ui/src/panels/schema_browser.rs`
- [ ] T082 [US12] Create `crates/tusk_ui/src/panels/results.rs` with ResultsPanel implementing Panel trait with loading Spinner state
- [ ] T083 [US12] Create `crates/tusk_ui/src/panels/messages.rs` with MessagesPanel implementing Panel trait
- [ ] T084 [US12] Update `crates/tusk_ui/src/panels/mod.rs` to export ResultsPanel and MessagesPanel
- [ ] T085 [US12] Register ResultsPanel and MessagesPanel with bottom dock in Workspace in `crates/tusk_ui/src/workspace.rs`

**Checkpoint**: User Story 12 complete - loading states work across all panels

---

## Phase 13: User Story 7 - Modal Dialogs (Priority: P2)

**Goal**: Interact with modal dialogs for confirmations and forms with focus trapping and Escape to close

**Independent Test**: Open modal, verify focus trapped within it, press Escape to close, verify background non-interactive

### Implementation for User Story 7

- [ ] T086 [US7] Create `crates/tusk_ui/src/modal.rs` with Modal struct implementing Render/Focusable/EventEmitter<ModalEvent>
- [ ] T087 [US7] Implement ModalAction struct (label, variant, disabled, handler) in `crates/tusk_ui/src/modal.rs`
- [ ] T088 [US7] Implement Modal::render() with backdrop, header (title), body (children), footer (actions) in `crates/tusk_ui/src/modal.rs`
- [ ] T089 [US7] Implement ModalLayer global struct with show(), dismiss(), has_modal() in `crates/tusk_ui/src/modal.rs`
- [ ] T090 [US7] Implement Modal focus trapping (Tab cycles within modal) in `crates/tusk_ui/src/modal.rs`
- [ ] T091 [US7] Implement Modal Escape key handling via Dismiss action in `crates/tusk_ui/src/modal.rs`
- [ ] T092 [US7] Implement Modal backdrop click to close (when closable=true) in `crates/tusk_ui/src/modal.rs`
- [ ] T093 [US7] Register ModalLayer as Global in app initialization in `crates/tusk/src/app.rs`
- [ ] T094 [US7] Integrate ModalLayer rendering in Workspace (render modals above all content) in `crates/tusk_ui/src/workspace.rs`

**Checkpoint**: User Story 7 complete - modals work with focus trapping and keyboard dismissal

---

## Phase 14: User Story 8 - Context Menus (Priority: P2)

**Goal**: Right-click elements to see contextual action menus with keyboard shortcuts and submenus

**Independent Test**: Right-click table in schema browser, verify menu appears at cursor with relevant options

### Implementation for User Story 8

- [ ] T095 [US8] Create `crates/tusk_ui/src/context_menu.rs` with ContextMenu struct implementing Render/Focusable/EventEmitter<ContextMenuEvent>
- [ ] T096 [US8] Implement ContextMenuItem enum (Action/Separator/Submenu) with builder methods in `crates/tusk_ui/src/context_menu.rs`
- [ ] T097 [US8] Implement ContextMenu::render() with menu items, shortcut hints, and submenu arrows in `crates/tusk_ui/src/context_menu.rs`
- [ ] T098 [US8] Implement ContextMenuLayer global struct with show(), dismiss(), is_open() in `crates/tusk_ui/src/context_menu.rs`
- [ ] T099 [US8] Implement context menu positioning to avoid viewport overflow in `crates/tusk_ui/src/context_menu.rs`
- [ ] T100 [US8] Implement context menu keyboard navigation (SelectNext, SelectPrevious, Confirm, Dismiss, OpenSubmenu, CloseSubmenu) in `crates/tusk_ui/src/context_menu.rs`
- [ ] T101 [US8] Implement submenu rendering and hover activation in `crates/tusk_ui/src/context_menu.rs`
- [ ] T102 [US8] Implement click-outside-to-close behavior in `crates/tusk_ui/src/context_menu.rs`
- [ ] T103 [US8] Register ContextMenuLayer as Global in app initialization in `crates/tusk/src/app.rs`
- [ ] T104 [US8] Integrate ContextMenuLayer rendering in Workspace in `crates/tusk_ui/src/workspace.rs`
- [ ] T105 [US8] Add right-click handler to Tree items emitting ContextMenu event in `crates/tusk_ui/src/tree.rs`
- [ ] T106 [US8] Wire schema browser context menu with type-specific actions (Select Top 100, View DDL, Copy Name) in `crates/tusk_ui/src/panels/schema_browser.rs`

**Checkpoint**: User Story 8 complete - context menus work with keyboard navigation and submenus

---

## Phase 15: Polish & Cross-Cutting Concerns

**Purpose**: Final integration, edge cases, and performance validation

**Note**: Tasks T107, T108, T109 have been relocated to their respective user story phases (T107→US3, T108/T109→US2) for proper dependency ordering. Task IDs preserved per Constitution Principle V.

- [ ] T110 Add text truncation with ellipsis and hover tooltips for long names in tree items in `crates/tusk_ui/src/tree.rs`
- [ ] T111 Verify performance: workspace renders in <500ms (SC-001)
- [ ] T112 Verify performance: dock resize at 60fps (SC-002)
- [ ] T113 Verify performance: tab switch <16ms (SC-003)
- [ ] T114 Verify performance: tree with 1000+ items at 60fps (SC-004)
- [ ] T115 Verify WCAG 2.1 AA focus indicators on all interactive elements (SC-005)
- [ ] T116 Verify performance: keyboard shortcuts <16ms (SC-006)
- [ ] T117 Verify performance: modal open/close <200ms (SC-007)
- [ ] T118 Verify performance: context menu appears <16ms (SC-008)
- [ ] T119 Verify persistence: dock/pane state survives restart (SC-009)
- [ ] T120 Final code cleanup and consistency check across all components

**Checkpoint**: All user stories complete with performance targets met

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-14)**: All depend on Foundational phase completion
  - P1 stories (US1, US2, US3, US5, US10) should be completed first
  - P2 stories can proceed after P1 stories
- **Polish (Phase 15)**: Depends on all user stories being complete

### User Story Dependencies

| Story | Priority | Dependencies | Notes |
|-------|----------|--------------|-------|
| US1 - Basic Workspace Shell | P1 | Foundational | First story, establishes workspace |
| US2 - Dock Resizing | P1 | US1 | Needs workspace; includes T108/T109 for collapse/constraints |
| US3 - Tabbed Query Editors | P1 | US1, T121 | Needs pane; T122 uses ConfirmDialog for dirty close; T107 adds drag reorder |
| US5 - Schema Browser | P1 | US1 | Needs left dock to exist |
| US10 - Keyboard Navigation | P1 | US1, US3, US5 | Wires up actions across components |
| US4 - Pane Splitting | P2 | US3 | Extends pane system |
| US6 - Button/Input | P2 | Foundational | Independent components |
| US9 - Select/Dropdown | P2 | US6 | Builds on input patterns |
| US11 - Status Bar | P2 | US1 | Extends status bar from US1 |
| US12 - Loading States | P2 | US5 | Extends panels |
| US7 - Modal Dialogs | P2 | US6 | Uses button component; full modal system |
| US8 - Context Menus | P2 | US5 | Adds to tree/schema browser |

### Within Each User Story

- Models/structs before services/methods
- Render methods before action handlers
- Core implementation before integration
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel
- P2 stories US6, US9, US11, US12 can potentially run in parallel once P1 complete
- Different user stories can be worked on by different team members

---

## Parallel Example: Foundational Phase

```bash
# Launch all parallel foundational tasks together:
Task: T007 "Create button.rs with Button component"
Task: T008 "Create resizer.rs with Resizer component"
Task: T009 "Create panel.rs with Panel trait"
```

## Parallel Example: User Story 5

```bash
# After Tree struct is created (T037), these can run in parallel:
Task: T038 "Implement Tree expand/collapse methods"
Task: T039 "Implement Tree::visible_items()"
Task: T044 "Implement Tree keyboard navigation actions"
```

---

## Implementation Strategy

### MVP First (P1 Stories Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Basic Workspace Shell)
4. Complete Phase 4: User Story 2 (Dock Resizing)
5. Complete Phase 5: User Story 3 (Tabbed Query Editors)
6. Complete Phase 6: User Story 5 (Schema Browser Navigation)
7. Complete Phase 7: User Story 10 (Keyboard Navigation)
8. **STOP and VALIDATE**: Test all P1 stories independently
9. Deploy/demo if ready (MVP!)

### Incremental Delivery

1. Complete Setup + Foundational -> Foundation ready
2. Add User Story 1 -> Test independently -> Basic workspace visible
3. Add User Story 2 -> Test independently -> Dock resizing works
4. Add User Story 3 -> Test independently -> Tabs work
5. Add User Story 5 -> Test independently -> Schema browser works
6. Add User Story 10 -> Test independently -> Full keyboard nav (P1 MVP!)
7. Continue with P2 stories for full feature set

### Suggested MVP Scope

**Minimum Viable Product = User Stories 1, 2, 3, 5, 10 (all P1)**

This delivers:
- Workspace shell with docks and status bar
- Resizable docks with persistence
- Multiple query tabs
- Schema browser with tree navigation
- Full keyboard navigation

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
- **Task ID range**: T001-T122 (122 total tasks)
- **Analysis-driven additions**: T121 (ConfirmDialog), T122 (dirty tab confirmation wiring)
- **Relocated tasks**: T107→US3, T108→US2, T109→US2 (from Polish phase to proper user stories)

## TASK IMMUTABILITY (Constitution Principle V)

**Once tasks are created, they are IMMUTABLE:**

- Tasks MUST NEVER be removed, deleted, or merged
- Tasks MUST NEVER be renumbered (task IDs are permanent)
- Tasks MUST NEVER have their scope reduced
- If a task seems wrong, FLAG IT for human review - do NOT modify or delete it
- The ONLY valid change is marking a task complete (unchecked -> checked)

**Violation Consequence**: Task removal/merger/scope reduction requires immediate branch deletion.
