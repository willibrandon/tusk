# Data Model: Project Initialization

**Feature**: 001-project-init
**Date**: 2026-01-19

## Overview

Project initialization establishes the foundational structure but does not introduce persistent data entities. This document describes the configuration and runtime entities that exist during the scaffold phase.

---

## Configuration Entities

### 1. TauriConfiguration

**Location**: `src-tauri/tauri.conf.json`
**Purpose**: Application metadata, window settings, build configuration

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| productName | string | Yes | Display name: "Tusk" |
| version | string | Yes | Semantic version: "0.1.0" |
| identifier | string | Yes | Bundle ID: "com.tusk.app" |
| build.devUrl | string | Yes | Dev server URL |
| build.frontendDist | string | Yes | Production build path |
| app.windows | Window[] | Yes | Window configurations |
| bundle.targets | string[] | Yes | Build targets per platform |

### 2. WindowConfiguration

**Location**: `src-tauri/tauri.conf.json` (nested in app.windows)
**Purpose**: Main window properties

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| title | string | "Tusk" | Window title |
| width | number | 1400 | Initial width in pixels |
| height | number | 900 | Initial height in pixels |
| minWidth | number | 800 | Minimum width constraint |
| minHeight | number | 600 | Minimum height constraint |
| resizable | boolean | true | Allow window resize |
| center | boolean | true | Center on screen at launch |
| decorations | boolean | true | Show window chrome |

### 3. CargoConfiguration

**Location**: `src-tauri/Cargo.toml`
**Purpose**: Rust dependencies and build settings

| Field | Type | Description |
|-------|------|-------------|
| package.name | string | Crate name: "tusk" |
| package.version | string | Must match tauri.conf.json |
| lib.name | string | Library name: "tusk_lib" |
| lib.crate-type | string[] | ["staticlib", "cdylib", "rlib"] |
| dependencies | Dependency[] | All Rust dependencies |
| profile.release | Profile | Release build optimizations |

### 4. PackageConfiguration

**Location**: `package.json`
**Purpose**: Node.js dependencies and scripts

| Field | Type | Description |
|-------|------|-------------|
| name | string | Package name: "tusk" |
| version | string | Must match tauri.conf.json |
| type | string | "module" for ESM |
| scripts | Scripts | NPM script commands |
| dependencies | Dependencies | Runtime dependencies |
| devDependencies | Dependencies | Build-time dependencies |

---

## Runtime Entities

### 5. AppState (Rust)

**Location**: `src-tauri/src/lib.rs` (managed state)
**Purpose**: Application-wide state container (empty for init, expanded later)

```rust
pub struct AppState {
    // To be populated in subsequent features:
    // - Connection pool manager
    // - Query executor
    // - Settings manager
}
```

### 6. ThemeState (Frontend)

**Location**: `src/lib/stores/theme.ts` (Svelte store)
**Purpose**: Dark/light mode toggle state

| Field | Type | Description |
|-------|------|-------------|
| mode | "light" \| "dark" | Current theme mode |
| preferSystem | boolean | Follow OS preference |

---

## Entity Relationships

```
TauriConfiguration
    └── WindowConfiguration (1:N, windows array)

PackageConfiguration
    └── References TauriConfiguration.version

CargoConfiguration
    └── References TauriConfiguration.version
    └── Defines AppState structure

AppState
    └── Manages application lifecycle
    └── (Future: ConnectionPool, QueryExecutor, etc.)

ThemeState
    └── Controls CSS class on document root
```

---

## Validation Rules

### Version Synchronization
- `tauri.conf.json` version MUST equal `package.json` version
- `Cargo.toml` version MUST equal `tauri.conf.json` version

### Window Constraints
- width >= minWidth (enforced by Tauri runtime)
- height >= minHeight (enforced by Tauri runtime)
- minWidth >= 400 (usability minimum)
- minHeight >= 300 (usability minimum)

### Build Targets
- macOS: dmg, app
- Windows: msi, exe
- Linux: deb, rpm, appimage

---

## State Transitions

### Application Lifecycle

```
[Not Running]
    → Launch (npm run tauri dev | built binary)
    → [Initializing]
        → Load TauriConfiguration
        → Initialize Rust backend
        → Create WebView
    → [Running]
        → Main window visible
        → Dev tools available (dev mode only)
    → [Closing]
        → Window close requested
        → Cleanup resources
    → [Not Running]
```

### Theme State

```
[light] ↔ [dark]
    Toggle via CSS class "dark" on <html>
    Persisted to localStorage (future feature)
```

---

## Notes

- Database entities (connections, queries, history) introduced in later features
- This model covers only project scaffolding configuration
- All entities are file-based or in-memory; no persistent storage yet
