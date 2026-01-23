# Research: Local Storage

**Feature**: 005-local-storage
**Date**: 2026-01-22

## Existing Implementation Analysis

The codebase already has substantial storage infrastructure in `crates/tusk_core/src/services/storage.rs`:

### Already Implemented

| Feature | Status | Location |
|---------|--------|----------|
| SQLite database initialization | ✅ Complete | `LocalStorage::open()` |
| WAL mode configuration | ✅ Complete | `configure_connection()` |
| Foreign keys enabled | ✅ Complete | PRAGMA statement |
| Migration framework | ✅ Complete | `run_migrations()` |
| Connection CRUD | ✅ Complete | `save_connection()`, `load_connection()`, etc. |
| SSH tunnel CRUD | ✅ Complete | `save_ssh_tunnel()`, `load_ssh_tunnel()`, etc. |
| Query history CRUD | ✅ Complete | `add_to_history()`, `load_history()`, etc. |
| Saved queries CRUD | ✅ Complete | `save_query()`, `load_saved_query()`, etc. |
| UI state (generic key-value) | ✅ Complete | `save_ui_state()`, `load_ui_state()` |
| Platform data directories | ✅ Complete | `default_data_dir()` |

### Needs Implementation

| Feature | Spec Reference | Notes |
|---------|---------------|-------|
| Connection groups | FR-004, User Story 5 | New table, model, CRUD operations |
| Query folders (hierarchical) | FR-005, User Story 3 | New table for folder hierarchy |
| Application settings (typed) | FR-008, User Story 6 | Typed settings model with defaults |
| Editor tab state | FR-006, User Story 4 | Structured tab state persistence |
| Window state | FR-007, User Story 7 | Window geometry and panel layout |
| History pruning | FR-010, User Story 2 | Automatic cleanup by count/age |
| Data export | FR-011, User Story 8 | JSON export format |
| Data import | FR-012, User Story 8 | Import with conflict resolution |
| Database recovery | FR-014 | Handle corruption gracefully |

## Technology Decisions

### Decision 1: Query Folder Storage

**Decision**: Store folders in a dedicated `query_folders` table with parent_id for hierarchy.

**Rationale**: The existing `saved_queries.folder_path` uses string paths (e.g., "/Reports/Monthly"). While functional, a proper foreign key relationship enables:
- Folder renaming without updating all queries
- Folder deletion with cascade or orphan handling
- Sort order per folder

**Alternatives Considered**:
- Keep folder_path strings: Simpler but no referential integrity
- Nested set model: Overkill for shallow hierarchies (2-3 levels typical)

### Decision 2: Settings Storage Pattern

**Decision**: Use strongly-typed `AppSettings` struct with individual getters/setters that serialize to the existing `ui_state` table.

**Rationale**:
- Reuses existing table infrastructure
- Type safety through Rust struct
- Default values baked into struct definition
- Individual key updates without full struct serialization

**Alternatives Considered**:
- Single JSON blob: No partial updates, wasteful
- New settings table with typed columns: Requires schema changes for each setting

### Decision 3: Editor Tab State Structure

**Decision**: Store as JSON array in `ui_state` table with key `editor_tabs`.

**Rationale**:
- Tab state is transient and session-oriented
- Existing `ui_state` table handles JSON already
- No relational queries needed on tab data

**Alternatives Considered**:
- Dedicated table per tab: Overhead not justified for ~10-20 tabs max

### Decision 4: Export Format

**Decision**: JSON with version header and entity collections.

**Rationale**:
- Human-readable for debugging
- Self-describing structure
- Supports forward/backward version detection
- Aligns with existing serde_json usage

**Alternatives Considered**:
- SQLite dump: Not portable, version-coupled
- MessagePack: Faster but not human-readable
- CSV: Multiple files, no hierarchy support

### Decision 5: History Pruning Strategy

**Decision**: Prune by count (default 10,000 entries) with configurable limit in settings.

**Rationale**:
- Count-based is predictable for database size
- User can adjust based on storage preferences
- Pruning runs on add_to_history() when count exceeds threshold

**Alternatives Considered**:
- Time-based (delete after N days): Unpredictable count
- Hybrid: Adds complexity without clear benefit

## Schema Migration Plan

Migration 2 will add:

```sql
-- Connection Groups
CREATE TABLE connection_groups (
    group_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    color TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT;

-- Add group_id to connections (nullable for ungrouped)
ALTER TABLE connections ADD COLUMN group_id TEXT REFERENCES connection_groups(group_id) ON DELETE SET NULL;

-- Query Folders (hierarchical)
CREATE TABLE query_folders (
    folder_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    parent_id TEXT REFERENCES query_folders(folder_id) ON DELETE CASCADE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT;

-- Add folder_id to saved_queries (replace folder_path eventually)
ALTER TABLE saved_queries ADD COLUMN folder_id TEXT REFERENCES query_folders(folder_id) ON DELETE SET NULL;

-- Indexes
CREATE INDEX idx_connections_group ON connections(group_id);
CREATE INDEX idx_query_folders_parent ON query_folders(parent_id);
CREATE INDEX idx_saved_queries_folder_id ON saved_queries(folder_id);
```

## Performance Validation

### Benchmark Targets (from spec)

| Metric | Target | Validation Approach |
|--------|--------|---------------------|
| Cold start (100 connections) | <200ms | Integration test with seeded DB |
| History load (10k entries) | <100ms | Benchmark test |
| CRUD operations | <10ms | Unit test timing assertions |
| DB size (50k+500+100) | <50MB | Integration test with seeded data |

### SQLite Optimization Checklist

- [x] WAL mode (already enabled)
- [x] Foreign keys (already enabled)
- [x] Busy timeout 5s (already set)
- [x] Cache size 64MB (already set)
- [ ] Add indexes on new columns (in migration)
- [ ] Use prepared statements for repeated queries
- [ ] Batch inserts for import operations

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Migration failure | Low | High | Transaction rollback, backup before migrate |
| Import conflicts | Medium | Low | User prompts for resolution |
| Large history performance | Low | Medium | Pagination, async loading |
| Multi-instance corruption | Low | High | SQLite locking + user warning |
