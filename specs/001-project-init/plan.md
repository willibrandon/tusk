# Implementation Plan: Project Initialization

**Branch**: `001-project-init` | **Date**: 2026-01-19 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-project-init/spec.md`

## Summary

Initialize the Tusk project with a Tauri v2 + Svelte 5 foundation, establishing the build system, directory structure, and development tooling for a native Postgres client. This is the foundational feature that enables all subsequent development.

## Technical Context

**Language/Version**: TypeScript 5.5+ (frontend), Rust 1.75+ (backend)
**Primary Dependencies**: Tauri v2, Svelte 5, Vite, TailwindCSS, Monaco Editor, TanStack Table, @xyflow/svelte (frontend); tokio-postgres, deadpool-postgres, rusqlite, keyring, russh, serde (backend)
**Storage**: N/A (project scaffolding only; SQLite for metadata in future features)
**Testing**: Vitest (frontend unit), cargo test (backend), Tauri MCP (E2E)
**Target Platform**: macOS 10.15+, Windows 10+, Linux (AppImage/deb/rpm)
**Project Type**: Desktop application with Tauri (Rust backend + Svelte frontend)
**Performance Goals**: Cold start < 1 second, hot reload < 2 seconds (per constitution)
**Constraints**: Memory idle < 100MB, strict port 5173 for dev server
**Scale/Scope**: Single-developer local application, no cloud/network dependencies

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Verification |
|-----------|--------|--------------|
| I. Postgres Exclusivity | N/A | Project init has no database features yet |
| II. Complete Local Privacy | PASS | No network calls, no telemetry in scaffold |
| III. OS Keychain for Credentials | N/A | No credential handling in this feature |
| IV. Complete Implementation | COMMIT | All scaffold files will be complete, no TODOs |
| V. Task Immutability | COMMIT | Tasks once created will not be modified |
| VI. Performance Discipline | PASS | Cold start < 1s target in acceptance criteria |

**NON-NEGOTIABLE Principles (automatic failure if violated):**
- Principle IV: Complete Implementation — No placeholders, TODOs, "future work", or scope reduction
- Principle V: Task Immutability — Tasks MUST NEVER be removed, merged, renumbered, or reduced in scope

**Security Requirements Compliance:**
- No credentials in this feature (scaffold only)
- Future credential handling will use OS keychain per constitution

## Project Structure

### Documentation (this feature)

```text
specs/001-project-init/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output (minimal for scaffold)
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (IPC command signatures)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
tusk/
├── .github/
│   └── workflows/
│       ├── ci.yml              # CI pipeline (lint, type-check, test, build)
│       └── release.yml         # Release builds for all platforms
├── docs/
│   ├── design.md               # Project design document
│   └── features/               # Feature documentation
├── src-tauri/                  # Rust backend (Tauri)
│   ├── src/
│   │   ├── main.rs             # Entry point
│   │   ├── lib.rs              # Library root with Tauri builder
│   │   ├── commands/           # Tauri IPC commands
│   │   │   └── mod.rs
│   │   ├── services/           # Business logic services
│   │   │   └── mod.rs
│   │   ├── models/             # Data structures
│   │   │   └── mod.rs
│   │   └── error.rs            # Error types
│   ├── icons/                  # App icons (all required sizes)
│   ├── Cargo.toml              # Rust dependencies
│   ├── Cargo.lock
│   ├── tauri.conf.json         # Tauri configuration
│   └── build.rs                # Build script
├── src/                        # Svelte frontend
│   ├── lib/
│   │   ├── components/         # UI components
│   │   │   ├── shell/          # App shell (sidebar, tabs, status)
│   │   │   ├── editor/         # Monaco editor wrapper
│   │   │   ├── grid/           # Results grid
│   │   │   ├── tree/           # Schema browser tree
│   │   │   ├── dialogs/        # Modal dialogs
│   │   │   └── common/         # Shared components
│   │   ├── stores/             # Svelte stores
│   │   ├── services/           # Frontend services (IPC wrappers)
│   │   └── utils/              # Utilities
│   ├── routes/                 # SvelteKit routes
│   │   ├── +layout.svelte      # Root layout
│   │   └── +page.svelte        # Main page
│   ├── app.html                # HTML template
│   ├── app.css                 # Global styles with Tailwind
│   └── main.ts                 # Frontend entry (if needed)
├── static/                     # Static assets
├── tests/
│   ├── e2e/                    # E2E tests (Tauri MCP)
│   └── unit/                   # Unit tests
├── .gitignore
├── .prettierrc                 # Prettier config
├── eslint.config.js            # ESLint flat config (v9+)
├── package.json
├── svelte.config.js
├── tsconfig.json
├── vite.config.ts              # Includes Tailwind v4 Vite plugin (no separate config needed)
├── CLAUDE.md                   # AI development context
└── README.md                   # Project readme
```

**Structure Decision**: Tauri v2 standard layout with `src-tauri/` for Rust backend and `src/` for Svelte frontend. This follows Tauri conventions and the constitution's technology stack requirements.

## Complexity Tracking

No constitution violations requiring justification. The structure follows standard Tauri patterns with no unnecessary abstractions.
