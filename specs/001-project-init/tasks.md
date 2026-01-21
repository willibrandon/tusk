# Tasks: Project Initialization

**Input**: Design documents from `/specs/001-project-init/`
**Prerequisites**: plan.md ✓, spec.md ✓, research.md ✓, data-model.md ✓, quickstart.md ✓

**Tests**: Not explicitly requested in feature specification - tests omitted per template guidance.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Cargo workspace**: Root `Cargo.toml` with `crates/` directory
- **Crates**: `crates/tusk/`, `crates/tusk_core/`, `crates/tusk_ui/`
- **Assets**: `assets/fonts/`, `assets/icons/`
- **CI**: `.github/workflows/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create Cargo workspace structure and configuration files

- [ ] T001 Create root Cargo.toml workspace manifest with workspace members and shared dependencies at Cargo.toml
- [ ] T002 [P] Create rust-toolchain.toml pinning Rust version to 1.80+ at rust-toolchain.toml
- [ ] T003 [P] Create .cargo/config.toml with build configuration at .cargo/config.toml
- [ ] T004 [P] Create tusk_core crate directory structure at crates/tusk_core/
- [ ] T005 [P] Create tusk_ui crate directory structure at crates/tusk_ui/
- [ ] T006 [P] Create tusk binary crate directory structure at crates/tusk/
- [ ] T007 Create crates/tusk_core/Cargo.toml with thiserror dependency at crates/tusk_core/Cargo.toml
- [ ] T008 [P] Create crates/tusk_ui/Cargo.toml with gpui dependency at crates/tusk_ui/Cargo.toml
- [ ] T009 [P] Create crates/tusk/Cargo.toml with gpui, tracing, tusk_core, tusk_ui dependencies at crates/tusk/Cargo.toml

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T010 Implement TuskError enum with Window, Theme, Font, Config variants in crates/tusk_core/src/error.rs
- [ ] T011 Create crates/tusk_core/src/lib.rs exporting error module at crates/tusk_core/src/lib.rs
- [ ] T012 Define ThemeColors struct with all color fields (background, surface, text, border, accent, status colors) using Hsla type in crates/tusk_ui/src/theme.rs
- [ ] T013 Define TuskTheme struct with name, appearance, and colors fields in crates/tusk_ui/src/theme.rs
- [ ] T014 Implement Default trait for TuskTheme returning dark theme colors in crates/tusk_ui/src/theme.rs
- [ ] T015 Implement Global trait for TuskTheme to enable cx.global::<TuskTheme>() access in crates/tusk_ui/src/theme.rs
- [ ] T016 Create crates/tusk_ui/src/icons.rs with icon management module foundation at crates/tusk_ui/src/icons.rs
- [ ] T017 Create crates/tusk_ui/src/lib.rs exporting theme and icons modules at crates/tusk_ui/src/lib.rs

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Build and Run Application (Priority: P1) 🎯 MVP

**Goal**: Developer clones repo, runs `cargo build && cargo run`, sees themed window at 1400x900 with min size 800x600

**Independent Test**: Run `cargo build && cargo run` and verify a dark-themed window appears at expected size

### Implementation for User Story 1

- [ ] T018 [US1] Create TuskApp struct implementing Render trait with render method returning themed div in crates/tusk/src/app.rs
- [ ] T019 [US1] Implement TuskApp::new() constructor in crates/tusk/src/app.rs
- [ ] T020 [US1] Implement render() method using div().flex().flex_col().size_full().bg() with theme colors in crates/tusk/src/app.rs
- [ ] T021 [US1] Create main.rs with tracing_subscriber initialization in crates/tusk/src/main.rs
- [ ] T022 [US1] Implement Application::new().run() entry point with window creation in crates/tusk/src/main.rs
- [ ] T023 [US1] Configure WindowOptions with window_bounds using Bounds::centered(None, size(px(1400.0), px(900.0)), cx) in crates/tusk/src/main.rs
- [ ] T024 [US1] Configure WindowOptions with window_min_size of 800x600 pixels in crates/tusk/src/main.rs
- [ ] T025 [US1] Register TuskTheme as global state using cx.set_global() before window creation in crates/tusk/src/main.rs
- [ ] T026 [US1] Add cx.activate(true) after window creation to focus application in crates/tusk/src/main.rs
- [ ] T027 [P] [US1] Download and add JetBrainsMono-Regular.ttf to assets/fonts/JetBrainsMono-Regular.ttf
- [ ] T028 [P] [US1] Download and add JetBrainsMono-Bold.ttf to assets/fonts/JetBrainsMono-Bold.ttf
- [ ] T029 [P] [US1] Download and add JetBrainsMono-Italic.ttf to assets/fonts/JetBrainsMono-Italic.ttf
- [ ] T030 [P] [US1] Download and add JetBrainsMono-BoldItalic.ttf to assets/fonts/JetBrainsMono-BoldItalic.ttf
- [ ] T031 [US1] Verify cargo build completes successfully with no errors
- [ ] T032 [US1] Verify cargo run launches window with dark theme at 1400x900 pixels
- [ ] T033 [US1] Verify window minimum size constraint of 800x600 is enforced

**Checkpoint**: User Story 1 complete - application builds and runs with themed window

---

## Phase 4: User Story 2 - Cross-Platform Compilation (Priority: P2)

**Goal**: CI system compiles application for macOS x64/ARM, Windows x64, and Linux x64

**Independent Test**: CI workflow runs successfully on all platform runners

### Implementation for User Story 2

- [ ] T034 [P] [US2] Create macOS application icon at assets/icons/tusk.icns (16, 32, 128, 256, 512, 1024 px sizes)
- [ ] T035 [P] [US2] Create Windows application icon at assets/icons/tusk.ico (16, 32, 48, 256 px sizes)
- [ ] T036 [US2] Create .github/workflows/ci.yml with workflow name and trigger configuration at .github/workflows/ci.yml
- [ ] T037 [US2] Add macOS ARM64 job (macos-14 runner, aarch64-apple-darwin target) to .github/workflows/ci.yml
- [ ] T038 [US2] Add macOS x64 job (macos-13 runner, x86_64-apple-darwin target) to .github/workflows/ci.yml
- [ ] T039 [US2] Add Windows x64 job (windows-latest runner, x86_64-pc-windows-msvc target) to .github/workflows/ci.yml
- [ ] T040 [US2] Add Linux x64 job (ubuntu-latest runner, x86_64-unknown-linux-gnu target) with system dependencies to .github/workflows/ci.yml
- [ ] T041 [US2] Configure Linux job to install libxkbcommon-dev, libwayland-dev packages in .github/workflows/ci.yml
- [ ] T042 [US2] Add cargo build step to all CI jobs in .github/workflows/ci.yml
- [ ] T043 [US2] Add cargo test step to all CI jobs in .github/workflows/ci.yml
- [ ] T065 [P] [US2] Create Windows application manifest tusk.exe.manifest with DPI awareness settings at assets/tusk.exe.manifest
- [ ] T066 [US2] Create build.rs to embed Windows manifest for DPI awareness (FR-013) at build.rs

**Checkpoint**: User Story 2 complete - CI builds on all target platforms

---

## Phase 5: User Story 3 - Code Quality Enforcement (Priority: P3)

**Goal**: Developer runs cargo fmt --check and cargo clippy -- -D warnings to validate code quality

**Independent Test**: Run both commands on codebase and verify they pass without errors

### Implementation for User Story 3

- [ ] T044 [P] [US3] Create rustfmt.toml with edition="2021", max_width=100, use_small_heuristics="Max" at rustfmt.toml
- [ ] T045 [P] [US3] Create clippy.toml with default configuration at clippy.toml
- [ ] T046 [US3] Add cargo fmt --all -- --check step to all CI jobs in .github/workflows/ci.yml
- [ ] T047 [US3] Add cargo clippy --workspace -- -D warnings step to all CI jobs in .github/workflows/ci.yml
- [ ] T048 [US3] Run cargo fmt --all to format all code in workspace
- [ ] T049 [US3] Run cargo clippy --workspace -- -D warnings and fix any warnings
- [ ] T050 [US3] Verify cargo fmt --check passes with no violations
- [ ] T051 [US3] Verify cargo clippy -- -D warnings passes with no warnings

**Checkpoint**: User Story 3 complete - code quality tools configured and passing

---

## Phase 6: User Story 4 - Development Workflow (Priority: P4)

**Goal**: Developer runs cargo watch -x run for hot-reload development workflow

**Independent Test**: Run cargo watch -x run, modify a source file, verify automatic rebuild

### Implementation for User Story 4

- [ ] T052 [US4] Document cargo-watch installation instructions in quickstart.md at specs/001-project-init/quickstart.md
- [ ] T053 [US4] Document cargo watch -x run usage for hot-reload development in quickstart.md at specs/001-project-init/quickstart.md
- [ ] T054 [US4] Verify cargo watch -x run starts application and monitors for changes
- [ ] T055 [US4] Verify modifying a source file triggers automatic rebuild

**Checkpoint**: User Story 4 complete - hot-reload development workflow documented and working

---

## Phase 7: User Story 5 - Debug Logging (Priority: P5)

**Goal**: Developer runs with RUST_LOG=tusk=debug to see debug log messages

**Independent Test**: Set RUST_LOG=tusk=debug, run application, verify "Starting Tusk" message appears

### Implementation for User Story 5

- [ ] T056 [US5] Add tracing::info!("Starting Tusk") log statement at application startup in crates/tusk/src/main.rs
- [ ] T057 [US5] Configure tracing_subscriber with EnvFilter::from_default_env() for RUST_LOG support in crates/tusk/src/main.rs
- [ ] T058 [US5] Verify RUST_LOG=tusk=debug cargo run shows debug-level log messages
- [ ] T059 [US5] Verify "Starting Tusk" info message appears in console output

**Checkpoint**: User Story 5 complete - debug logging works via RUST_LOG environment variable

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Final validation and documentation

- [ ] T060 [P] Update quickstart.md with final build and run instructions at specs/001-project-init/quickstart.md
- [ ] T061 [P] Verify all fonts render correctly in application window
- [ ] T062 Run full CI workflow locally to validate all checks pass
- [ ] T063 Run cargo build --release and verify release build succeeds
- [ ] T064 Verify cold start time is under 500ms target

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-7)**: All depend on Foundational phase completion
  - Can proceed sequentially in priority order (P1 → P2 → P3 → P4 → P5)
  - Or in parallel if team capacity allows
- **Polish (Phase 8)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Applies to code from US1
- **User Story 4 (P4)**: Can start after US1 (needs runnable application)
- **User Story 5 (P5)**: Can start after Foundational (Phase 2) - Integrates into US1 main.rs

### Within Each User Story

- Configuration files before implementation
- Core types before components
- Components before main entry point
- Verification tasks at the end of each story

### Parallel Opportunities

- T002, T003, T004, T005, T006: All setup file creations can run in parallel
- T007, T008, T009: Crate Cargo.toml files can run in parallel
- T027, T028, T029, T030: Font file downloads can run in parallel
- T034, T035, T065: Icon and manifest file creations can run in parallel
- T044, T045: rustfmt.toml and clippy.toml can run in parallel
- T060, T061: Documentation and font verification can run in parallel

---

## Parallel Example: Phase 1 Setup

```bash
# Launch all parallel setup tasks together:
Task: "Create rust-toolchain.toml pinning Rust version to 1.80+"
Task: "Create .cargo/config.toml with build configuration"
Task: "Create tusk_core crate directory structure"
Task: "Create tusk_ui crate directory structure"
Task: "Create tusk binary crate directory structure"
```

## Parallel Example: User Story 1 Font Downloads

```bash
# Launch all font downloads together:
Task: "Download and add JetBrainsMono-Regular.ttf"
Task: "Download and add JetBrainsMono-Bold.ttf"
Task: "Download and add JetBrainsMono-Italic.ttf"
Task: "Download and add JetBrainsMono-BoldItalic.ttf"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Run `cargo build && cargo run`, verify themed window appears
5. Deploy/demo if ready - this is the MVP!

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. User Story 1 → **MVP complete** - application builds and runs
3. User Story 2 → Cross-platform CI working
4. User Story 3 → Code quality enforced
5. User Story 4 → Developer workflow optimized
6. User Story 5 → Debug logging available
7. Each story adds value without breaking previous stories

### Single Developer Strategy

1. Complete Setup (T001-T009)
2. Complete Foundational (T010-T017)
3. Complete User Stories in priority order: US1 → US2 → US3 → US4 → US5
4. Complete Polish phase
5. Total sequential execution with parallel opportunities within phases

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Font files must be downloaded from JetBrains website or included from a licensed source

## ⚠️ TASK IMMUTABILITY (Constitution Principle V)

**Once tasks are created, they are IMMUTABLE:**

- Tasks MUST NEVER be removed, deleted, or merged
- Tasks MUST NEVER be renumbered (task IDs are permanent)
- Tasks MUST NEVER have their scope reduced
- If a task seems wrong, FLAG IT for human review — do NOT modify or delete it
- The ONLY valid change is marking a task complete (unchecked → checked)

**Violation Consequence**: Task removal/merger/scope reduction requires immediate branch deletion.
