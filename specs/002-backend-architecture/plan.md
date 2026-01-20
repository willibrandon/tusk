# Implementation Plan: Backend Architecture

**Branch**: `002-backend-architecture` | **Date**: 2026-01-19 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-backend-architecture/spec.md`

## Summary

Establish the Rust backend structure with proper module organization, error handling patterns, application state management, and foundational services for storage, logging, credentials, and connection management. This feature creates the architecture that enables all subsequent features.

## Technical Context

**Language/Version**: Rust 1.75+ (backend), TypeScript 5.5+ (frontend)
**Primary Dependencies**: Tauri v2, tokio-postgres, deadpool-postgres, rusqlite, keyring, russh, thiserror, tracing, directories
**Storage**: SQLite (rusqlite) for local metadata; PostgreSQL for user databases (via tokio-postgres)
**Testing**: cargo test (unit tests), Tauri MCP (integration tests)
**Target Platform**: macOS, Windows, Linux desktop (Tauri cross-platform)
**Project Type**: Desktop application with web frontend (Tauri architecture)
**Performance Goals**: Cold start < 1 second, idle memory < 100MB, query cancellation < 2 seconds
**Constraints**: Zero plaintext credential storage, rotating log files, graceful degradation on keychain unavailability
**Scale/Scope**: Single-user desktop application, multiple concurrent database connections

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

### Pre-Design Check (Phase 0)

| Principle                        | Status  | Notes                                                                               |
| -------------------------------- | ------- | ----------------------------------------------------------------------------------- |
| I. Postgres Exclusivity          | ✅ PASS | All database operations use tokio-postgres, Postgres-specific error codes preserved |
| II. Complete Local Privacy       | ✅ PASS | No network calls except to user-configured Postgres servers; logs stored locally    |
| III. OS Keychain for Credentials | ✅ PASS | FR-007 mandates OS keychain via `keyring` crate; passwords never in files           |
| IV. Complete Implementation      | ✅ PASS | Plan includes all 13 functional requirements; no placeholders                       |
| V. Task Immutability             | ✅ PASS | Tasks created in tasks.md will be immutable per constitution                        |
| VI. Performance Discipline       | ✅ PASS | Cold start < 1s, memory < 100MB specified in success criteria                       |
| Security Requirements            | ✅ PASS | Parameterized queries, no credential logging, SSL preferred                         |

### Post-Design Check (Phase 1)

| Principle                        | Status  | Notes                                                                                                                   |
| -------------------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------- |
| I. Postgres Exclusivity          | ✅ PASS | Data model preserves PostgreSQL error codes (code, position, hint, detail); tokio-postgres used exclusively             |
| II. Complete Local Privacy       | ✅ PASS | Logs written to local app data directory; no telemetry or external calls                                                |
| III. OS Keychain for Credentials | ✅ PASS | Credential storage patterns in research.md use `keyring` crate; SQLite stores no passwords                              |
| IV. Complete Implementation      | ✅ PASS | All 19 IPC commands defined in contracts; data-model.md covers all entities; quickstart.md has 10 implementation phases |
| V. Task Immutability             | ✅ PASS | Ready for task generation via `/speckit.tasks`                                                                          |
| VI. Performance Discipline       | ✅ PASS | Query cancellation via `tokio::select!`; non-blocking logging via `tracing-appender`; deadpool pooling                  |
| Security Requirements            | ✅ PASS | No credential logging (research.md); parameterized queries required; SSL preferred by default                           |

**NON-NEGOTIABLE Principles (automatic failure if violated):**

- Principle IV: Complete Implementation — No placeholders, TODOs, "future work", or scope reduction
- Principle V: Task Immutability — Tasks MUST NEVER be removed, merged, renumbered, or reduced in scope

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src-tauri/                      # Rust backend (Tauri v2)
├── src/
│   ├── main.rs                 # Entry point (calls lib.rs run())
│   ├── lib.rs                  # Tauri builder setup, plugin registration
│   ├── error.rs                # Unified error types (TuskError, ErrorResponse)
│   ├── state.rs                # Application state (AppState struct)
│   ├── commands/               # Tauri IPC command handlers
│   │   ├── mod.rs              # Command module exports
│   │   ├── app.rs              # App-level commands (health_check, get_app_info)
│   │   ├── connection.rs       # Connection management commands
│   │   ├── query.rs            # Query execution commands
│   │   └── storage.rs          # Local storage commands
│   ├── services/               # Business logic services
│   │   ├── mod.rs              # Service module exports
│   │   ├── connection.rs       # Connection pooling, SSH tunnels
│   │   ├── query.rs            # Query execution, cancellation
│   │   ├── storage.rs          # SQLite local storage, data repair
│   │   ├── credentials.rs      # OS keychain integration
│   │   └── logging.rs          # File-based rotating logs
│   └── models/                 # Data structures
│       ├── mod.rs              # Model module exports
│       ├── connection.rs       # ConnectionConfig, PoolConfig
│       ├── query.rs            # Query, QueryResult, RowBatch
│       ├── error.rs            # ErrorResponse, ErrorKind
│       └── app.rs              # AppInfo, AppState types
├── Cargo.toml
└── tauri.conf.json

src/                            # Svelte frontend
├── lib/
│   ├── components/             # UI components (future features)
│   ├── stores/                 # Svelte stores
│   │   ├── index.ts
│   │   └── theme.svelte.ts
│   ├── services/               # IPC wrappers
│   │   └── index.ts
│   └── utils/
│       └── index.ts
├── routes/                     # SvelteKit routes
│   ├── +layout.svelte
│   ├── +layout.ts
│   └── +page.svelte
└── app.html
```

**Structure Decision**: Tauri architecture with Rust backend (`src-tauri/`) and Svelte frontend (`src/`). The backend structure follows the established pattern from 001-project-init with `commands/`, `services/`, and `models/` modules.

## Complexity Tracking

No constitution violations requiring justification.

---

## Generated Artifacts

| Artifact      | Path                                                     | Description                                              |
| ------------- | -------------------------------------------------------- | -------------------------------------------------------- |
| Research      | [research.md](./research.md)                             | Technical research findings for all implementation areas |
| Data Model    | [data-model.md](./data-model.md)                         | Entity definitions, SQLite schema, relationships         |
| IPC Contracts | [contracts/ipc-commands.md](./contracts/ipc-commands.md) | All 19 Tauri IPC command specifications                  |
| Quickstart    | [quickstart.md](./quickstart.md)                         | Implementation guide with 10 phases                      |

---

## Next Steps

Run `/speckit.tasks` to generate the implementation task list based on this plan.
