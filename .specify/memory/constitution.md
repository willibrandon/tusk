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
| Cold start                        | < 1 second |
| Memory (idle)                     | < 100 MB   |
| Memory (1M rows loaded)           | < 500 MB   |
| Query result render (1000 rows)   | < 100ms    |
| Schema browser load (1000 tables) | < 500ms    |
| Autocomplete response             | < 50ms     |

Performance MUST be achieved through streaming (batch row emission via Tauri events) and virtual scrolling (render only visible content). Lazy loading and pagination are acceptable; blocking the UI thread is not.

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

**Frontend** (WebView):

- Svelte 5 for compiled reactivity
- Monaco Editor for SQL editing
- TanStack Table + custom virtualization for data grids
- @xyflow/svelte for ER diagrams
- Tailwind CSS for styling

**Backend** (Rust):

- Tauri v2 for native shell
- tokio-postgres for async Postgres operations
- deadpool-postgres for connection pooling
- russh for SSH tunnels
- rusqlite for local metadata storage
- keyring for OS credential storage

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

**Version**: 1.1.0 | **Ratified**: 2026-01-19 | **Last Amended**: 2026-01-19
