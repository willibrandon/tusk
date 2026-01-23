# Implementation Plan: Local Storage

**Branch**: `005-local-storage` | **Date**: 2026-01-22 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/005-local-storage/spec.md`

## Summary

Extend the existing LocalStorage service to support connection groups, query folders, application settings, editor tab state, window state persistence, history pruning, and data export/import. The foundation (connections, SSH tunnels, query history, saved queries, UI state) is already implemented in `crates/tusk_core/src/services/storage.rs`.

## Technical Context

**Language/Version**: Rust 1.80+ with 2021 edition
**Primary Dependencies**: rusqlite 0.38 (bundled), serde/serde_json 1.0, chrono 0.4, uuid 1.0, dirs 6.0, parking_lot 0.12
**Storage**: SQLite with WAL mode (already configured)
**Testing**: cargo test with tempfile for isolated database tests
**Target Platform**: macOS, Windows, Linux (cross-platform native)
**Project Type**: Multi-crate Rust workspace (tusk, tusk_core, tusk_ui)
**Performance Goals**: Cold start <200ms with 100 connections, CRUD <10ms, history load <100ms for 10k entries
**Constraints**: <50MB database with 50k history + 500 queries + 100 connections, no UI thread blocking
**Scale/Scope**: Single-user desktop application, 100s of connections, 10,000s of history entries

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

**NON-NEGOTIABLE Principles (automatic failure if violated):**

- Principle IV: Complete Implementation — No placeholders, TODOs, "future work", or scope reduction
- Principle V: Task Immutability — Tasks MUST NEVER be removed, merged, renumbered, or reduced in scope

**Pre-Design Constitution Compliance:**

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Postgres Exclusivity | ✅ PASS | Local SQLite for metadata only; no Postgres abstraction |
| II. Complete Local Privacy | ✅ PASS | All data local; no network calls |
| III. OS Keychain for Credentials | ✅ PASS | Passwords via keyring crate; never in SQLite |
| IV. Complete Implementation | ✅ PASS | All 8 user stories fully implemented |
| V. Task Immutability | ✅ PASS | N/A (no existing tasks) |
| VI. Performance Discipline | ✅ PASS | Targets align with spec SC-001 through SC-008 |

**Post-Design Constitution Re-check (Phase 1 Complete):**

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Postgres Exclusivity | ✅ PASS | Design uses SQLite only for local metadata |
| II. Complete Local Privacy | ✅ PASS | Export/import are local files; no network |
| III. OS Keychain for Credentials | ✅ PASS | ExportData excludes passwords |
| IV. Complete Implementation | ✅ PASS | All API contracts defined; no gaps |
| V. Task Immutability | ✅ PASS | N/A (tasks not yet created) |
| VI. Performance Discipline | ✅ PASS | Indexed queries, batch operations planned |

## Project Structure

### Documentation (this feature)

```text
specs/005-local-storage/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (internal Rust API)
└── tasks.md             # Phase 2 output (created by /speckit.tasks)
```

### Source Code (repository root)

```text
crates/
├── tusk/                      # Main application crate
│   └── src/
│       ├── main.rs            # Application entry point
│       └── app.rs             # TuskApp root component
├── tusk_core/                 # Core services crate
│   └── src/
│       ├── lib.rs
│       ├── error.rs           # TuskError types
│       ├── models/
│       │   ├── mod.rs
│       │   ├── connection.rs  # ConnectionConfig, ConnectionGroup (NEW)
│       │   ├── history.rs     # QueryHistoryEntry
│       │   ├── query.rs       # Query execution types
│       │   ├── schema.rs      # Database schema introspection
│       │   └── settings.rs    # AppSettings (NEW)
│       ├── services/
│       │   ├── mod.rs
│       │   ├── storage.rs     # LocalStorage (EXTEND)
│       │   ├── credentials.rs # OS keychain integration
│       │   ├── query.rs       # Query execution
│       │   └── schema.rs      # Schema introspection
│       └── state.rs           # TuskState global
└── tusk_ui/                   # UI components crate
    └── src/
        ├── lib.rs
        └── workspace.rs       # Window state persistence hooks

tests/
└── tusk_core/
    ├── storage_test.rs        # Storage service tests
    ├── settings_test.rs       # Settings model tests
    └── export_import_test.rs  # Export/import round-trip tests
```

**Structure Decision**: Extending existing multi-crate workspace. Core storage logic in tusk_core, UI hooks in tusk_ui.

## Complexity Tracking

No constitution violations requiring justification.
