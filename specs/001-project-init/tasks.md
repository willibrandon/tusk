# Tasks: Project Initialization

**Input**: Design documents from `/specs/001-project-init/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Tests**: Not explicitly requested in feature specification. No test tasks included.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Frontend**: `src/` (Svelte/SvelteKit)
- **Backend**: `src-tauri/` (Rust/Tauri)
- Follows Tauri v2 standard layout per plan.md

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic directory structure

- [X] T001 Create root directory structure per plan.md (src/, src-tauri/, docs/, static/, tests/)
- [X] T002 [P] Create .gitignore with Node, Rust, IDE, and build artifact patterns
- [X] T003 [P] Create package.json with name, version, type module, and script placeholders
- [X] T004 [P] Create src-tauri/Cargo.toml with package metadata and all dependencies per research.md
- [X] T005 [P] Create src-tauri/build.rs with tauri_build::build() call
- [X] T006 Create src-tauri/tauri.conf.json with Tauri v2 schema, app metadata, and build paths
- [X] T007 [P] Create src-tauri/src/main.rs entry point calling tusk_lib::run()
- [X] T008 [P] Create src-tauri/src/lib.rs with Tauri builder, plugin init, and devtools setup
- [X] T009 [P] Create src-tauri/src/error.rs with AppError type per contracts/ipc-commands.md
- [X] T010 [P] Create src-tauri/src/commands/mod.rs with get_app_info command
- [X] T011 [P] Create src-tauri/src/services/mod.rs as empty module placeholder
- [X] T012 [P] Create src-tauri/src/models/mod.rs with AppInfo struct
- [X] T013 Create src-tauri/capabilities/default.json with core:default and shell:allow-open permissions
- [X] T014 [P] Create src/app.html with HTML template, lang attribute, dark mode class hook
- [X] T015 [P] Create src/routes/+layout.svelte with app.css import and slot
- [X] T016 [P] Create src/routes/+layout.ts with ssr = false export
- [X] T017 Create src/routes/+page.svelte with placeholder UI (sidebar, main content)
- [X] T018 [P] Create src/lib/components/ directory structure (shell/, editor/, grid/, tree/, dialogs/, common/)
- [X] T019 [P] Create src/lib/stores/ directory with index.ts
- [X] T020 [P] Create src/lib/services/ directory with index.ts
- [X] T021 [P] Create src/lib/utils/ directory with index.ts
- [X] T022 [P] Create static/ directory with .gitkeep
- [X] T023 [P] Create tests/e2e/ directory with .gitkeep
- [X] T024 [P] Create tests/unit/ directory with .gitkeep
- [X] T025 [P] Create docs/features/ directory with .gitkeep
- [X] T064 [P] Create .github/workflows/ci.yml with lint, type-check, and build jobs for all platforms
- [X] T065 [P] Create .github/workflows/release.yml with release build triggers and artifact upload

**Checkpoint**: Basic project structure exists, ready for configuration files

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core configuration that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T026 Install Node.js dependencies via npm install (creates node_modules, package-lock.json)
- [X] T027 Run cargo build in src-tauri/ to download and compile Rust dependencies (creates Cargo.lock)
- [X] T028 Verify Tauri CLI available via npx tauri --version

**Checkpoint**: Dependencies installed, foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Developer Starts Development Session (Priority: P1) 🎯 MVP

**Goal**: Developer runs `npm run tauri dev` and the application launches with hot reload enabled

**Independent Test**: Run `npm run tauri dev`, verify window opens within 30 seconds, modify a .svelte file, verify changes appear within 2 seconds

### Implementation for User Story 1

- [X] T029 [US1] Create svelte.config.js with adapter-static, SPA fallback, and path aliases per research.md
- [X] T030 [US1] Create vite.config.ts with sveltekit plugin, tailwindcss plugin, port 5173, strictPort per research.md
- [X] T031 [US1] Create tsconfig.json extending .svelte-kit/tsconfig.json with strict mode per research.md
- [X] T032 [US1] Update package.json scripts with dev, build, preview, tauri commands
- [X] T033 [US1] Update src-tauri/tauri.conf.json build section with beforeDevCommand, devUrl, beforeBuildCommand, frontendDist
- [X] T034 [US1] Verify npm run tauri dev launches application window with hot reload functional

**Checkpoint**: User Story 1 complete - developers can start development sessions with hot reload

---

## Phase 4: User Story 2 - Developer Builds Production Binary (Priority: P2)

**Goal**: Developer runs `npm run tauri build` and receives platform-specific installer

**Independent Test**: Run `npm run tauri build`, verify .app/.dmg (macOS), .exe/.msi (Windows), or .deb/.rpm/.AppImage (Linux) is created in src-tauri/target/release/bundle/

### Implementation for User Story 2

- [X] T035 [US2] Update src-tauri/tauri.conf.json bundle section with active: true, targets: all, icon paths
- [X] T036 [US2] Create src-tauri/icons/ directory with placeholder icon files (32x32.png, 128x128.png, 128x128@2x.png, icon.icns, icon.ico)
- [X] T037 [US2] Update src-tauri/tauri.conf.json bundle.macOS with minimumSystemVersion: 10.15
- [X] T038 [US2] Update src-tauri/tauri.conf.json bundle.linux with deb depends and rpm depends
- [X] T039 [US2] Verify npm run tauri build produces platform-appropriate installer

**Checkpoint**: User Story 2 complete - production builds work for all platforms

---

## Phase 5: User Story 3 - Developer Validates Code Quality (Priority: P3)

**Goal**: Developer runs lint and type check commands to validate code quality

**Independent Test**: Run `npm run lint` and `npm run check`, verify both pass with zero errors on initial project state

### Implementation for User Story 3

- [X] T040 [P] [US3] Create eslint.config.js with ESLint v9 flat config, TypeScript, Svelte per research.md
- [X] T041 [P] [US3] Create .prettierrc with semi, singleQuote, tabWidth, plugins configuration
- [X] T042 [US3] Update package.json devDependencies with eslint, eslint-plugin-svelte, typescript-eslint, prettier packages
- [X] T043 [US3] Update package.json scripts with lint, lint:fix, format, check, check:watch commands
- [X] T044 [US3] Verify npm run lint passes with zero errors
- [X] T045 [US3] Verify npm run check passes with zero TypeScript errors
- [X] T046 [US3] Verify cargo build in src-tauri/ compiles with zero errors

**Checkpoint**: User Story 3 complete - code quality validation tools work

---

## Phase 6: User Story 4 - Application Window Configuration (Priority: P4)

**Goal**: Application window opens at 1400x900, centered, with 800x600 minimum, devtools available in dev mode

**Independent Test**: Launch application, verify window dimensions are 1400x900, try to resize below 800x600 (should be prevented), open devtools in dev mode

### Implementation for User Story 4

- [X] T047 [US4] Update src-tauri/tauri.conf.json app.windows with width: 1400, height: 900, minWidth: 800, minHeight: 600
- [X] T048 [US4] Update src-tauri/tauri.conf.json app.windows with center: true, resizable: true, decorations: true
- [X] T049 [US4] Update src-tauri/src/lib.rs setup closure to open devtools in debug mode via window.open_devtools()
- [X] T050 [US4] Verify application window opens at 1400x900 centered on screen
- [X] T051 [US4] Verify window cannot be resized below 800x600
- [X] T052 [US4] Verify devtools accessible in development mode (Cmd+Option+I on macOS)

**Checkpoint**: User Story 4 complete - window configuration meets all requirements

---

## Phase 7: User Story 5 - Dark Mode Support (Priority: P5)

**Goal**: Application supports dark mode via CSS class toggle with appropriate contrast

**Independent Test**: Toggle dark class on document, verify interface switches to dark colors with readable text

### Implementation for User Story 5

- [X] T053 [US5] Create src/app.css with @import tailwindcss, @theme for custom colors per research.md
- [X] T054 [US5] Add dark mode CSS variables and scrollbar styling to src/app.css
- [X] T055 [US5] Create src/lib/stores/theme.ts with mode and preferSystem state per data-model.md
- [X] T056 [US5] Update src/routes/+layout.svelte to apply dark class to document based on theme store
- [X] T057 [US5] Update src/routes/+page.svelte with dark: variants for all background and text colors
- [X] T058 [US5] Verify dark mode toggles correctly with sufficient contrast ratios
- [X] T066 [US5] Verify light mode text meets WCAG AA contrast ratio (4.5:1 minimum) using contrast checker
- [X] T067 [US5] Verify dark mode text meets WCAG AA contrast ratio (4.5:1 minimum) using contrast checker

**Checkpoint**: User Story 5 complete - dark mode support works with WCAG AA compliance

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Final validation and documentation

- [X] T059 [P] Create README.md with project description, setup instructions, and development commands
- [X] T060 Verify all success criteria from spec.md are met (SC-001 through SC-010)
- [X] T061 Run quickstart.md validation checklist to confirm all setup steps work
- [X] T062 Verify cold start completes in under 1 second (SC-001)
- [X] T063 Verify hot reload reflects changes in under 2 seconds (SC-002)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-7)**: All depend on Foundational phase completion
  - User stories can proceed in priority order (P1 → P2 → P3 → P4 → P5)
  - Or in parallel if staffed
- **Polish (Phase 8)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after US1 (needs dev build working first)
- **User Story 3 (P3)**: Can start after Foundational - Independent of US1/US2
- **User Story 4 (P4)**: Can start after Foundational - Independent of other stories
- **User Story 5 (P5)**: Can start after US1 (needs basic app running)

### Within Each Phase

- Tasks marked [P] can run in parallel
- Sequential tasks must complete in order
- Verification tasks depend on implementation tasks

### Parallel Opportunities

**Phase 1 (Setup):**
```
Parallel Group A: T002, T003, T004, T005
Parallel Group B: T007, T008, T009, T010, T011, T012
Parallel Group C: T014, T015, T016
Parallel Group D: T018, T019, T020, T021, T022, T023, T024, T025, T064, T065
```

**Phase 5 (US3):**
```
Parallel: T040, T041 (different config files)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T025)
2. Complete Phase 2: Foundational (T026-T028)
3. Complete Phase 3: User Story 1 (T029-T034)
4. **STOP and VALIDATE**: Verify dev mode works with hot reload
5. MVP is functional - developers can start building features

### Incremental Delivery

1. Setup + Foundational → Project structure ready
2. Add User Story 1 → Dev mode works (MVP!)
3. Add User Story 2 → Production builds work
4. Add User Story 3 → Code quality tools work
5. Add User Story 4 → Window properly configured
6. Add User Story 5 → Dark mode available
7. Polish → All success criteria verified

### Task Summary

| Phase | Story | Task Count | Parallel Tasks |
|-------|-------|------------|----------------|
| Phase 1 | Setup | 27 | 23 |
| Phase 2 | Foundation | 3 | 0 |
| Phase 3 | US1 (P1) | 6 | 0 |
| Phase 4 | US2 (P2) | 5 | 0 |
| Phase 5 | US3 (P3) | 7 | 2 |
| Phase 6 | US4 (P4) | 6 | 0 |
| Phase 7 | US5 (P5) | 8 | 0 |
| Phase 8 | Polish | 5 | 1 |
| **Total** | | **67** | **26** |

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently testable after completion
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
