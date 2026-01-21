# Implementation Plan: Project Initialization

**Branch**: `001-project-init` | **Date**: 2026-01-20 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-project-init/spec.md`

## Summary

Initialize Tusk as a pure Rust application using GPUI, establishing the foundational build system, directory structure, and development tooling for a cross-platform native PostgreSQL client. The application will display a themed window (1400x900, min 800x600) with GPU-accelerated rendering using patterns derived from Zed's GPUI implementation.

## Technical Context

**Language/Version**: Rust 1.80+ with 2021 edition
**Primary Dependencies**: GPUI (from Zed repository), tracing, serde/serde_json, parking_lot
**Storage**: N/A for this feature (local SQLite introduced in later features)
**Testing**: cargo test with GPUI's built-in test harness
**Target Platform**: macOS (Metal), Windows (DirectX 12), Linux (Vulkan)
**Project Type**: Cargo workspace with multiple crates
**Performance Goals**: Cold start < 500ms
**Constraints**: Window min size 800x600, default 1400x900, custom fonts bundled
**Scale/Scope**: Single window application with themed rendering

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

**NON-NEGOTIABLE Principles (automatic failure if violated):**

- ✅ Principle IV: Complete Implementation — No placeholders, TODOs, "future work", or scope reduction
- ✅ Principle V: Task Immutability — Tasks MUST NEVER be removed, merged, renumbered, or reduced in scope

**Other Principle Compliance:**

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Postgres Exclusivity | ✅ N/A | No database features in init |
| II. Complete Local Privacy | ✅ Pass | No network calls in init |
| III. OS Keychain for Credentials | ✅ N/A | No credentials in init |
| VI. Performance Discipline | ✅ Pass | Cold start < 500ms target set |

**Technology Stack Compliance:**

| Required | Planned | Status |
|----------|---------|--------|
| GPUI (Zed's framework) | GPUI from zed repo | ✅ |
| Rust 1.75+ | Rust 1.80+ | ✅ |
| No JavaScript/WebView | Pure Rust | ✅ |
| tracing for logging | tracing crate | ✅ |
| parking_lot | parking_lot crate | ✅ |
| serde/serde_json | serde crates | ✅ |
| Cargo workspace | Multi-crate workspace | ✅ |

**GATE PASSED**: All principles satisfied. Proceeding to Phase 0.

### Post-Design Re-evaluation (Phase 1 Complete)

| Principle | Status | Design Artifact |
|-----------|--------|-----------------|
| IV. Complete Implementation | ✅ Pass | No placeholders in data-model.md or research.md |
| V. Task Immutability | ✅ Pass | No tasks created yet (tasks.md in Phase 2) |
| I. Postgres Exclusivity | ✅ N/A | No database abstractions in design |
| II. Complete Local Privacy | ✅ Pass | No telemetry/network in design |
| III. OS Keychain | ✅ N/A | No credentials in design |
| VI. Performance Discipline | ✅ Pass | Cold start < 500ms achievable with GPUI |

**Technology Stack Verification**:
- research.md confirms GPUI patterns from Zed codebase
- data-model.md uses GPUI types (Hsla, Appearance)
- No external dependencies beyond constitution-approved stack

**POST-DESIGN GATE PASSED**: Ready for Phase 2 (task generation via /speckit.tasks)

### Requirement Coverage Notes

**FR-013 (Windows DPI Awareness)**: Implemented via `build.rs` which embeds a Windows application manifest declaring DPI awareness. The manifest sets `<dpiAware>true/pm</dpiAware>` and `<dpiAwareness>PerMonitorV2</dpiAwareness>` for proper high-DPI rendering.

**FR-014 (Platform Framework Linking)**: Handled automatically by GPUI's platform backends:
- macOS: Metal framework linked via `cocoa` and `metal` crates
- Windows: DirectX 12 linked via `windows` crate with `d3d12` feature
- Linux: Vulkan linked via `ash` crate and system `libvulkan`

No additional tasks required for FR-014; GPUI's Cargo.toml dependencies handle framework linking at compile time

## Project Structure

### Documentation (this feature)

```text
specs/001-project-init/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
# Cargo Workspace Structure
Cargo.toml               # Workspace manifest with shared dependencies
rust-toolchain.toml      # Pin Rust version to 1.80+
rustfmt.toml             # Code formatting rules
clippy.toml              # Linting configuration
.cargo/config.toml       # Cargo configuration (build flags)

crates/
├── tusk/                # Main application binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs      # Entry point: Application::new().run()
│       └── app.rs       # TuskApp root component (Render trait)
│
├── tusk_core/           # Shared types and errors
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── error.rs     # TuskError type
│
└── tusk_ui/             # UI components and theming
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── theme.rs     # Theme definitions (TuskTheme, colors)
        └── icons.rs     # Icon management module (minimal foundation)

assets/
├── fonts/
│   ├── JetBrainsMono-Regular.ttf
│   ├── JetBrainsMono-Bold.ttf
│   ├── JetBrainsMono-Italic.ttf
│   └── JetBrainsMono-BoldItalic.ttf
└── icons/
    ├── tusk.icns        # macOS app icon
    └── tusk.ico         # Windows app icon

.github/
└── workflows/
    └── ci.yml           # Build, test, format, lint for all platforms

build.rs                 # Windows manifest embedding for DPI awareness (FR-013)
```

**Structure Decision**: Cargo workspace with three initial crates:
1. `tusk` - Binary crate for application entry point
2. `tusk_core` - Library crate for shared types (error handling)
3. `tusk_ui` - Library crate for UI components and theming

This mirrors Zed's architecture and allows clean separation of concerns as features grow.

## Complexity Tracking

> No violations requiring justification. Design follows constitution constraints.

| Check | Status |
|-------|--------|
| Maximum 3 projects/crates | ✅ 3 crates (tusk, tusk_core, tusk_ui) |
| No unnecessary abstractions | ✅ Direct GPUI usage |
| No premature optimization | ✅ Standard patterns only |
