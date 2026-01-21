<!--
SYNC IMPACT REPORT
==================
Version change: 1.1.0 → 2.0.0

Modified Principles:
- VI. Performance Discipline: Removed "Tauri events" reference, updated to "GPUI rendering and async channels"

Removed Sections:
- Technology Stack: Removed entire "Frontend (WebView)" section with Svelte/Monaco/TanStack
- Technology Stack: Removed Tauri v2 from Backend section

Added Sections:
- Technology Stack: New unified "Pure Rust + GPUI" section
- Technology Stack: New "Build & Packaging" subsection

Templates Status:
- .specify/templates/plan-template.md: ✅ No Tauri/Svelte references (generic template)
- .specify/templates/spec-template.md: ✅ No technology references (generic template)
- .specify/templates/tasks-template.md: ✅ No technology references (generic template)
- .specify/templates/agent-file-template.md: ✅ No technology references (generic template)
- .specify/templates/checklist-template.md: ✅ No technology references (generic template)

Follow-up TODOs: None

Bump Rationale: MAJOR version bump (1.1.0 → 2.0.0) because this is a backward-incompatible
redefinition of the Technology Stack section - the entire frontend architecture has been
replaced (WebView/Svelte/Monaco → pure Rust/GPUI).
-->

# Tusk Constitution

## Core Principles

### I. Postgres Exclusivity

Tusk MUST support only PostgreSQL. No multi-database abstractions, connection adapters for other databases, or "generic SQL" layers. Every feature MUST leverage Postgres-specific capabilities (pg*stat*\*, COPY protocol, LISTEN/NOTIFY, RLS, etc.) rather than lowest-common-denominator SQL.

**Rationale**: Deep Postgres integration delivers better performance, richer features, and simpler code than generic database support. This is a Postgres client, not a universal database tool.

### II. Complete Local Privacy

Tusk MUST NOT make any network calls except to user-configured PostgreSQL servers. No telemetry, no cloud sync, no update checks to external servers, no analytics. All data (connections, query history, settings) MUST remain on the user's machine.

**Rationale**: Database clients handle sensitive data. Users must trust that their queries, credentials, and database contents never leave their control.

### III. OS Keychain for Credentials

Passwords and sensitive credentials MUST be stored exclusively in the operating system's secure credential storage (macOS Keychain, Windows Credential Manager, Secret Service on Linux). Credentials MUST NEVER be written to SQLite, config files, logs, or any other plaintext storage.

**Rationale**: The OS keychain provides hardware-backed encryption and proper access controls. Rolling custom encryption invites security vulnerabilities.

### IV. Complete Implementation (NON-NEGOTIABLE)

Every feature MUST be implemented completely before moving to the next. This means:

- No placeholder implementations or stub functions
- No "TODO" comments deferring work
- No "future work" or "later iterations" references
- No scope reduction or deprioritization
- If a problem is discovered, fix it immediately regardless of when it was introduced

**Rationale**: Incomplete features accumulate technical debt and create maintenance burden. Each feature represents a commitment to users.

### V. Task Immutability (NON-NEGOTIABLE)

Once tasks are created in a tasks.md file, they are IMMUTABLE. This means:

- Tasks MUST NEVER be removed, deleted, or merged
- Tasks MUST NEVER be renumbered (task IDs are permanent)
- Tasks MUST NEVER have their scope reduced or simplified
- If a task appears redundant or incorrect, flag it for human review — do NOT modify or delete it
- Task completion is the ONLY valid state change (unchecked → checked)

**Violation Consequence**: Any task removal, merger, or scope reduction is a constitution violation requiring immediate branch deletion and restart from scratch.

**Rationale**: Task lists represent commitments. Removing tasks is scope reduction by stealth. Every task created reflects a deliberate decision that MUST be honored through completion.

### VI. Performance Discipline

All features MUST meet these performance targets:

| Metric                            | Target     |
| --------------------------------- | ---------- |
| Cold start                        | < 500ms    |
| Memory (idle)                     | < 50 MB    |
| Memory (1M rows loaded)           | < 400 MB   |
| Query result render (1000 rows)   | < 16ms     |
| Schema browser load (1000 tables) | < 300ms    |
| Autocomplete response             | < 30ms     |

Performance MUST be achieved through streaming (batch row emission via async channels) and virtual scrolling (GPUI's UniformList renders only visible content). Lazy loading and pagination are acceptable; blocking the main thread is not.

**Rationale**: Users choose native applications for performance. Failing these targets negates the value proposition versus web-based tools.

## Security Requirements

**Credential Handling**:

- MUST never log passwords, connection strings with passwords, or authentication tokens
- MUST use parameterized queries for all database operations (never string interpolation)
- MUST validate all user input before processing
- MUST respect read-only connection mode (block INSERT, UPDATE, DELETE, DDL)
- MUST confirm destructive operations (DROP, TRUNCATE, DELETE without WHERE)

**Connection Security**:

- SSL/TLS MUST be preferred by default (ssl_mode: prefer)
- SSH tunnels MUST be supported for secure remote access
- Certificate validation MUST be enforced when ssl_mode requires it

## Technology Stack

**Pure Rust + GPUI**:

- GPUI (Zed's GPU-accelerated UI framework) for all rendering
- Rust 1.75+ with 2021 edition
- No JavaScript, no WebView, no Electron — native GPUI rendering throughout

**Core Libraries**:

- tokio-postgres for async Postgres operations
- deadpool-postgres for connection pooling
- russh for SSH tunnels (pure Rust SSH2)
- rusqlite for local metadata storage
- keyring for OS credential storage
- parking_lot for thread-safe synchronization
- thiserror for error types
- tracing for structured logging
- serde/serde_json for serialization

**Build & Packaging**:

- Cargo workspace for multi-crate project structure
- cargo-bundle for platform-specific packaging (macOS .app, Windows .msi, Linux .deb/.AppImage)

Deviations from this stack require explicit justification and constitution amendment.

## Governance

This constitution supersedes all other practices. Violations discovered during development MUST be fixed before proceeding.

**Amendment Process**:

1. Document the proposed change with rationale
2. Assess impact on existing features
3. Update constitution version according to semantic versioning:
   - MAJOR: Principle removal or incompatible redefinition
   - MINOR: New principle added or material expansion
   - PATCH: Clarifications, wording, non-semantic refinements
4. Propagate changes to dependent templates

**Compliance**:

- All code reviews MUST verify constitution compliance
- Complexity MUST be justified against Principle IV (Complete Implementation)
- Task modifications MUST be rejected per Principle V (Task Immutability)
- See `CLAUDE.md` for runtime development guidance

**Version**: 2.0.0 | **Ratified**: 2026-01-19 | **Last Amended**: 2026-01-20
