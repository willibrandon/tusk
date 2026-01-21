# Implementation Plan: Frontend Architecture

**Branch**: `003-frontend-architecture` | **Date**: 2026-01-21 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/003-frontend-architecture/spec.md`

## Summary

Implement the GPUI frontend architecture for Tusk, providing a workspace-based UI with docks, panes, tabs, panels, and a comprehensive component library. The architecture follows patterns established in Zed (the authoritative GPUI reference at `/Users/brandon/src/zed`) while adapting them for database client workflows. Core deliverables include: workspace shell, resizable dock system, tab/pane management with splitting, Panel trait for extensible content, schema browser tree component, full component library (Button, TextInput, Select, Modal, ContextMenu, Icon, Spinner), keyboard navigation system, and status bar.

## Technical Context

**Language/Version**: Rust 1.80+ with 2021 edition
**Primary Dependencies**:
- GPUI (Zed's GPU-accelerated UI framework, git rev `89e9ab97aa5d978351ee8a28d9cc35c272c530f5`)
- smallvec (for SmallVec in pane management)
- uuid (for entity identification)
- parking_lot (thread-safe synchronization, already in workspace)
- tusk_core (existing crate with TuskState)

**Storage**: Local SQLite via rusqlite (for persistence of dock sizes, workspace state)
**Testing**: cargo test with GPUI's TestAppContext, Tauri MCP for integration tests
**Target Platform**: macOS (primary), Linux, Windows (via GPUI cross-platform support)
**Project Type**: Single (Cargo workspace with multiple crates)
**Performance Goals**:
- Cold start: < 500ms
- Dock resize: 60fps
- Tab switch: < 16ms
- Schema tree (1000+ items): 60fps with virtualization
- Modal open/close: < 200ms
- Context menu: < 16ms
- Keyboard shortcuts: < 16ms

**Constraints**:
- Memory idle: < 50MB
- Memory with 1M rows: < 400MB
- WCAG 2.1 AA focus indicators
- Pure Rust/GPUI (no JavaScript, no WebView)

**Scale/Scope**:
- 20+ components (Workspace, Dock, Pane, PaneGroup, Tab, Panel, Tree, Icon, Button, TextInput, Select, Modal, ContextMenu, Spinner, StatusBar, Resizer, etc.)
- ~3000 lines of Rust code
- 17 source files in tusk_ui crate

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

**NON-NEGOTIABLE Principles (automatic failure if violated):**

- **Principle IV: Complete Implementation** — No placeholders, TODOs, "future work", or scope reduction
  - ✅ PASS: All 46 functional requirements from spec must be implemented completely
  - All components defined in feature document will be implemented in full

- **Principle V: Task Immutability** — Tasks MUST NEVER be removed, merged, renumbered, or reduced in scope
  - ✅ PASS: Task list will be generated in Phase 2 and remain immutable

**Additional Constitution Gates:**

- **Principle I: Postgres Exclusivity** — N/A for this feature (UI layer, no database abstraction)
  - ✅ PASS: This feature focuses on UI; database interactions use existing tusk_core services

- **Principle II: Complete Local Privacy** — No network calls except to Postgres servers
  - ✅ PASS: All UI state persisted locally via rusqlite; no external network calls

- **Principle III: OS Keychain for Credentials** — Credentials never in plaintext storage
  - ✅ PASS: UI displays connection status but delegates credential handling to tusk_core

- **Principle VI: Performance Discipline** — Must meet performance targets
  - ✅ PASS: Feature document specifies performance requirements; implementation will use:
    - Virtual scrolling (GPUI UniformList) for schema tree
    - Async channels for streaming data
    - Debounced resize events (60fps)
    - Immediate mode rendering for 60fps UI

- **GPUI Reference** — Zed codebase is authoritative for GPUI patterns
  - ✅ PASS: Implementation will reference `/Users/brandon/src/zed` for API verification

## Project Structure

### Documentation (this feature)

```text
specs/003-frontend-architecture/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (internal component contracts)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
crates/
├── tusk/                      # Main application crate (exists)
│   └── src/
│       ├── main.rs            # Entry point (exists)
│       └── app.rs             # TuskApp (exists, will be modified)
├── tusk_core/                 # Core services (exists)
│   └── src/
│       └── state.rs           # TuskState with Global trait (exists)
├── tusk_ui/                   # UI components crate (exists, minimal)
│   ├── Cargo.toml             # Dependencies (needs update)
│   └── src/
│       ├── lib.rs             # Crate root (needs expansion)
│       ├── workspace.rs       # Root workspace component [NEW]
│       ├── dock.rs            # Dock component [NEW]
│       ├── pane.rs            # Pane and PaneGroup [NEW]
│       ├── resizer.rs         # Resize handle component [NEW]
│       ├── status_bar.rs      # Status bar component [NEW]
│       ├── panel.rs           # Panel trait [NEW]
│       ├── tree.rs            # Generic tree component [NEW]
│       ├── icon.rs            # Icon system (exists as icons.rs, rename/expand)
│       ├── button.rs          # Button component [NEW]
│       ├── input.rs           # TextInput component [NEW]
│       ├── select.rs          # Select/dropdown component [NEW]
│       ├── modal.rs           # Modal dialog component [NEW]
│       ├── context_menu.rs    # Context menu component [NEW]
│       ├── spinner.rs         # Loading spinner [NEW]
│       ├── key_bindings.rs    # Keyboard shortcuts [NEW]
│       ├── layout.rs          # Layout utilities [NEW]
│       ├── theme.rs           # Theme system (exists, needs expansion)
│       └── panels/
│           ├── mod.rs         # Panel module [NEW]
│           ├── schema_browser.rs  # Schema browser panel [NEW]
│           ├── results.rs     # Query results panel [NEW]
│           └── messages.rs    # Messages panel [NEW]
└── tusk_editor/               # SQL editor crate [NEW in future feature]
    └── (placeholder for query editor component)
```

**Structure Decision**: Single project with Cargo workspace. The tusk_ui crate will be expanded significantly to house all UI components. This aligns with the existing project structure established in Feature 01 (project initialization) and Feature 02 (backend services).

## Complexity Tracking

No constitution violations requiring justification. The architecture follows established patterns from Zed's codebase and the existing Tusk project structure.

---

## Post-Design Constitution Re-Check

_Completed after Phase 1 design artifacts._

**NON-NEGOTIABLE Principles Re-verification:**

- **Principle IV: Complete Implementation**
  - ✅ VERIFIED: All 46 functional requirements mapped to contracts in `contracts/`
  - ✅ VERIFIED: 17 data model entities defined in `data-model.md`
  - ✅ VERIFIED: All components have complete API specifications
  - ✅ VERIFIED: No TODOs, placeholders, or "future work" in design artifacts

- **Principle V: Task Immutability**
  - ✅ VERIFIED: Tasks not yet generated (Phase 2 pending)
  - Commitment: Task list will remain immutable once created

**Additional Gates Re-verification:**

- **Principle I: Postgres Exclusivity** — ✅ VERIFIED: UI layer only; no database abstraction
- **Principle II: Complete Local Privacy** — ✅ VERIFIED: SQLite for persistence; no network calls
- **Principle III: OS Keychain** — ✅ VERIFIED: Delegated to tusk_core
- **Principle VI: Performance Discipline** — ✅ VERIFIED: UniformList for virtualization, 60fps targets
- **GPUI Reference** — ✅ VERIFIED: research.md documents all patterns from Zed codebase

**Design Artifacts Checklist:**

| Artifact | Status | Location |
|----------|--------|----------|
| research.md | ✅ Complete | `specs/003-frontend-architecture/research.md` |
| data-model.md | ✅ Complete | `specs/003-frontend-architecture/data-model.md` |
| contracts/ | ✅ Complete | `specs/003-frontend-architecture/contracts/` |
| quickstart.md | ✅ Complete | `specs/003-frontend-architecture/quickstart.md` |
| tasks.md | ⏳ Pending | Phase 2 via `/speckit.tasks` |

**Status**: ✅ READY FOR PHASE 2 (Task Generation)

