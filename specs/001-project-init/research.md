# Research: Project Initialization

**Feature**: 001-project-init
**Date**: 2026-01-19

## Executive Summary

All technology choices are defined by the constitution. This research validates current versions, best practices, and identifies configuration requirements for Tauri v2 + Svelte 5 project setup.

---

## 1. Tauri v2 Project Structure

### Decision
Use Tauri v2 with SvelteKit in SPA mode (adapter-static).

### Rationale
- Tauri v2 (2.9.x) is stable with capability-based security model
- SvelteKit adapter-static generates proper build output for Tauri
- SPA mode required because Tauri WebView doesn't support SSR

### Configuration Requirements

**tauri.conf.json (v2 schema):**
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Tusk",
  "version": "0.1.0",
  "identifier": "com.tusk.app",
  "build": {
    "frontendDist": "../build",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  }
}
```

**Key v2 Changes from v1:**
- `@tauri-apps/api/tauri` → `@tauri-apps/api/core`
- `Window` → `WebviewWindow`
- `get_window()` → `get_webview_window()`
- Allowlist replaced by capabilities system
- Plugin configuration in `plugins` key

### Alternatives Considered
- Tauri v1: Rejected (v2 is stable, v1 entering maintenance)
- Electron: Rejected (larger memory footprint, violates performance targets)

---

## 2. Frontend Stack Versions

### Decision
Use Svelte 5.47.x + SvelteKit 2.50.x + Vite + TypeScript 5.x

### Rationale
- Svelte 5 is stable with runes-based reactivity
- SvelteKit 2.50 is current stable (v3 not released)
- TypeScript in component markup now supported

### Package Versions (January 2026)

| Package | Version | Notes |
|---------|---------|-------|
| svelte | 5.47.x | TypeScript in markup supported |
| @sveltejs/kit | 2.50.x | Current stable |
| @sveltejs/adapter-static | 3.0.x | v3 breaking change from v2 |
| vite | 6.x | Current stable |
| typescript | 5.7.x | Current stable |
| tailwindcss | 4.x | Vite plugin approach |
| @tailwindcss/vite | 4.x | Replace postcss setup |

### SvelteKit Adapter Configuration

```javascript
// svelte.config.js
import adapter from '@sveltejs/adapter-static';

export default {
  kit: {
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      fallback: 'index.html',  // SPA mode
      precompress: false,
      strict: false
    })
  }
};
```

**Critical**: Disable SSR in root layout:
```typescript
// src/routes/+layout.ts
export const ssr = false;
```

### Alternatives Considered
- Svelte 4: Rejected (v5 stable with better reactivity model)
- Next.js/React: Rejected (constitution specifies Svelte)
- Vue: Rejected (constitution specifies Svelte)

---

## 3. ESLint v9 Configuration

### Decision
Use ESLint v9 flat config with eslint-plugin-svelte.

### Rationale
- ESLint v9 requires flat config (no .eslintrc.cjs)
- eslint-plugin-svelte v3 supports Svelte 5 rules

### Configuration

```javascript
// eslint.config.js
import js from '@eslint/js';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';
import ts from 'typescript-eslint';

export default ts.config(
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs.recommended,
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node
      }
    }
  },
  {
    files: ['**/*.svelte', '**/*.svelte.ts'],
    languageOptions: {
      parserOptions: {
        projectService: true,
        extraFileExtensions: ['.svelte'],
        parser: ts.parser
      }
    }
  },
  {
    ignores: ['build/', '.svelte-kit/', 'node_modules/']
  }
);
```

### Alternatives Considered
- ESLint v8: Rejected (v9 is current, flat config is future)
- Biome: Considered but eslint-plugin-svelte integration better

---

## 4. Tailwind CSS Setup

### Decision
Use Tailwind v4 with Vite plugin (not PostCSS).

### Rationale
- Tailwind v4 has native Vite plugin
- Simpler config than PostCSS approach
- Better build performance

### Configuration

**vite.config.ts:**
```typescript
import { defineConfig } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [
    sveltekit(),
    tailwindcss()
  ]
});
```

**app.css:**
```css
@import "tailwindcss";

@theme {
  --color-tusk-500: #0ea5e9;
  /* Custom colors defined here */
}
```

### Key v4 Changes
- Single `@import "tailwindcss"` replaces `@tailwind` directives
- Custom colors via `@theme` directive, not tailwind.config.js
- `@apply` works in global styles only

### Alternatives Considered
- Tailwind v3: Fallback if `@apply` needed in components
- CSS-in-JS: Rejected (Tailwind specified in constitution)

---

## 5. Rust Dependencies

### Decision
Use current stable crate versions with constitution-specified libraries.

### Cargo.toml Dependencies

```toml
[package]
name = "tusk"
version = "0.1.0"
description = "A fast, free, native Postgres client"
authors = ["Tusk Contributors"]
license = "MIT"
edition = "2021"
rust-version = "1.75"

[lib]
name = "tusk_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2.5", features = [] }

[dependencies]
# Tauri v2
tauri = { version = "2.9", features = ["macos-private-api"] }
tauri-plugin-shell = "2"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Async runtime
tokio = { version = "1.41", features = ["full"] }

# PostgreSQL
tokio-postgres = { version = "0.7", features = [
    "with-uuid-1",
    "with-chrono-0_4",
    "with-serde_json-1"
] }
deadpool-postgres = "0.14"

# Local storage
rusqlite = { version = "0.32", features = ["bundled"] }

# OS credentials
keyring = { version = "3.6", features = [
    "apple-native",
    "windows-native",
    "sync-secret-service"
] }

# SSH tunneling
russh = "0.54"
russh-keys = "0.54"

# Type support
uuid = { version = "1.10", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# TLS
native-tls = "0.2"
postgres-native-tls = "0.5"

# Utilities
thiserror = "1.0"
anyhow = "1.0"
directories = "5"

[target.'cfg(target_os = "macos")'.dependencies]
cocoa = "0.26"
objc = "0.2"

[features]
default = ["custom-protocol"]
custom-protocol = ["tauri/custom-protocol"]

[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### Version Notes
- Tauri 2.9.x: Latest stable with capability-based security
- russh 0.54.x: Latest with security fixes
- keyring 3.6.x: Latest with native OS support
- rusqlite 0.32.x: Constitution specifies; version per existing project

### Alternatives Considered
- sqlx: Rejected (tokio-postgres specified in constitution)
- bb8 pool: Rejected (deadpool specified in constitution)

---

## 6. TypeScript Configuration

### Decision
Extend @tsconfig/svelte with strict mode and bundler resolution.

### Configuration

```json
{
  "extends": "./.svelte-kit/tsconfig.json",
  "compilerOptions": {
    "allowJs": true,
    "checkJs": true,
    "esModuleInterop": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "skipLibCheck": true,
    "sourceMap": true,
    "strict": true,
    "moduleResolution": "bundler",
    "verbatimModuleSyntax": true,
    "target": "ES2020"
  }
}
```

### Key Settings
- `verbatimModuleSyntax`: Required for Svelte 5
- `moduleResolution: bundler`: Modern resolution for Vite
- `target: ES2020`: Required (ES2015 breaks classes)

---

## 7. Capabilities and Permissions (Tauri v2)

### Decision
Use minimal capabilities for project init (expand in later features).

### Initial Capability (src-tauri/capabilities/default.json)

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default capability for main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-open"
  ]
}
```

### Notes
- Full permission configuration deferred to features that need them
- Database, filesystem, and IPC permissions added in later features

---

## Summary of Decisions

| Area | Decision | Rationale |
|------|----------|-----------|
| Framework | Tauri v2.9.x | Stable, capability-based security |
| Frontend | Svelte 5.47.x + SvelteKit 2.50.x | Constitution, runes reactivity |
| Build | Vite 6.x + adapter-static | SPA mode for Tauri |
| Styling | Tailwind v4 (Vite plugin) | Simpler than PostCSS |
| Linting | ESLint v9 flat config | Current standard |
| TypeScript | 5.7.x strict mode | Type safety |
| Backend | Rust 1.75+ with Tauri | Constitution |
| Database | tokio-postgres 0.7.x | Constitution |
| Pooling | deadpool-postgres 0.14.x | Constitution |
| Storage | rusqlite 0.32.x | Constitution |
| Credentials | keyring 3.6.x | Constitution (OS keychain) |
| SSH | russh 0.54.x | Constitution |

All NEEDS CLARIFICATION items resolved. Ready for Phase 1 design.
