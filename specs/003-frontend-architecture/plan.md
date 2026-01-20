# Implementation Plan: Frontend Architecture

**Branch**: `003-frontend-architecture` | **Date**: 2026-01-19 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/003-frontend-architecture/spec.md`

## Summary

Establish the Svelte 5 frontend structure with a complete application shell (sidebar, tabs, status bar), state management using Svelte 5 stores with runes, and component organization. The shell provides the foundational layout for all user interaction including connection browsing, multi-tab workspace management, and connection status display.

## Technical Context

**Language/Version**: TypeScript 5.5+ (frontend)
**Primary Dependencies**: Svelte 5.17+, SvelteKit 2.15+, Tailwind CSS 4.0+, @tauri-apps/api 2.2+
**Storage**: localStorage (UI state persistence), OS keychain via Tauri backend (credentials - future)
**Testing**: Vitest (unit tests), Playwright (E2E tests)
**Target Platform**: Tauri v2 WebView (macOS, Windows, Linux)
**Project Type**: Web (frontend) within Tauri desktop application
**Performance Goals**: 60fps during resize/interactions, shell render <1s, tab operations <100ms
**Constraints**: <100MB idle memory, maintain responsive layout 800x600 to max screen
**Scale/Scope**: Shell layout (3 regions), tab management (unlimited tabs), sidebar (resizable 200-500px), theme (3 modes)

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

### Principle I: Postgres Exclusivity — ✅ PASS

Frontend architecture is Postgres-agnostic infrastructure. Connection display will show Postgres-specific information when connections feature is implemented.

### Principle II: Complete Local Privacy — ✅ PASS

- All state persists to localStorage (local only)
- No telemetry, analytics, or external network calls
- Theme, sidebar width, tab state remain on user's machine

### Principle III: OS Keychain for Credentials — ✅ PASS (N/A)

Frontend stores no credentials. Connection passwords will be handled by Rust backend via keyring crate in future connection feature.

### Principle IV: Complete Implementation — ✅ WILL COMPLY

All shell components, stores, and features will be fully implemented:

- No placeholder components
- No TODO comments
- No stub functions
- Complete tab management with unsaved changes handling
- Full theme support including system preference tracking

### Principle V: Task Immutability — ✅ WILL COMPLY

Once tasks.md is generated, no tasks will be removed, merged, or renumbered.

### Principle VI: Performance Discipline — ✅ WILL COMPLY

| Metric         | Target       | Approach                                          |
| -------------- | ------------ | ------------------------------------------------- |
| Cold start     | < 1 second   | Minimal component tree, lazy load future features |
| Sidebar resize | 60fps (16ms) | CSS-based resize, no re-renders during drag       |
| Tab operations | < 100ms      | Svelte 5 fine-grained reactivity                  |
| Theme change   | < 100ms      | CSS custom properties, class toggle               |

**NON-NEGOTIABLE Principles (automatic failure if violated):**

- Principle IV: Complete Implementation — No placeholders, TODOs, "future work", or scope reduction
- Principle V: Task Immutability — Tasks MUST NEVER be removed, merged, renumbered, or reduced in scope

## Project Structure

### Documentation (this feature)

```text
specs/003-frontend-architecture/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (TypeScript interfaces)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/                           # Svelte frontend (Tauri WebView)
├── app.css                    # Tailwind styles, dark mode, scrollbars
├── app.html                   # HTML entry point
├── lib/
│   ├── components/
│   │   ├── shell/             # Application shell components
│   │   │   ├── Shell.svelte          # Main shell layout container
│   │   │   ├── Sidebar.svelte        # Resizable sidebar panel
│   │   │   ├── SidebarHeader.svelte  # Sidebar header with title/actions
│   │   │   ├── SidebarSearch.svelte  # Connection filter search input
│   │   │   ├── TabBar.svelte         # Tab container with drag-drop
│   │   │   ├── Tab.svelte            # Individual tab component
│   │   │   ├── StatusBar.svelte      # Bottom status bar
│   │   │   └── Resizer.svelte        # Drag handle for panel resizing
│   │   ├── dialogs/
│   │   │   └── ConfirmDialog.svelte  # Confirmation dialog (unsaved changes)
│   │   ├── common/
│   │   │   ├── Button.svelte         # Reusable button component
│   │   │   └── Icon.svelte           # Icon wrapper component
│   │   ├── editor/            # Monaco editor (future feature)
│   │   ├── grid/              # Results grid (future feature)
│   │   └── tree/              # Schema browser tree (future feature)
│   ├── stores/
│   │   ├── index.ts           # Store exports
│   │   ├── theme.svelte.ts    # Theme state (existing, needs enhancement)
│   │   ├── tabs.svelte.ts     # Tab management state
│   │   ├── connections.svelte.ts  # Connection state (placeholder for future)
│   │   └── ui.svelte.ts       # UI preferences (sidebar width, collapsed)
│   ├── services/
│   │   └── index.ts           # Service exports (IPC wrappers)
│   ├── types/
│   │   ├── index.ts           # Type exports
│   │   ├── tab.ts             # Tab-related types
│   │   ├── connection.ts      # Connection-related types
│   │   └── ui.ts              # UI state types
│   └── utils/
│       ├── index.ts           # Utility exports
│       ├── storage.ts         # localStorage helpers with error handling
│       └── keyboard.ts        # Keyboard shortcut utilities
├── routes/
│   ├── +layout.svelte         # Root layout with Shell
│   ├── +layout.ts             # SSR disabled, prerender enabled
│   └── +page.svelte           # Main content area
└── tests/
    ├── unit/                  # Vitest unit tests
    │   ├── stores/            # Store tests
    │   └── components/        # Component tests
    └── e2e/                   # Playwright E2E tests

src-tauri/                     # Rust backend (existing)
└── [unchanged by this feature]
```

**Structure Decision**: Tauri v2 application with Svelte frontend in `src/` and Rust backend in `src-tauri/`. Frontend follows SvelteKit conventions with organized component directories matching CLAUDE.md specification. All shell components go in `components/shell/`, stores use Svelte 5 runes pattern.

## Complexity Tracking

> No constitution violations. No complexity justification needed.

## Post-Design Constitution Re-Check

_Re-evaluated after Phase 1 design completion._

### Principle I: Postgres Exclusivity — ✅ PASS

Design artifacts (data-model.md, contracts/) define connection types that will display Postgres-specific info. No generic database abstractions.

### Principle II: Complete Local Privacy — ✅ PASS

All persistence uses localStorage. No external network calls in any artifact. Connection data comes from local SQLite via backend.

### Principle III: OS Keychain for Credentials — ✅ PASS

Connection interface explicitly excludes password field. Contracts document that passwords are stored via keyring crate.

### Principle IV: Complete Implementation — ✅ DESIGNED FOR COMPLIANCE

- 26 files planned in quickstart.md with concrete implementation order
- Type contracts fully defined (no "TBD" or "to be determined")
- Data model complete with validation rules and state transitions
- No deferred functionality within this feature's scope

### Principle V: Task Immutability — ✅ READY FOR TASKS

Plan complete. `/speckit.tasks` will generate immutable task list.

### Principle VI: Performance Discipline — ✅ DESIGNED FOR COMPLIANCE

- research.md documents RAF throttling for resize operations
- Svelte 5 runes provide fine-grained reactivity
- CSS class toggle for instant theme changes
- No blocking operations identified in design
