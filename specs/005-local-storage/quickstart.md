# Quickstart: Local Storage

**Feature**: 005-local-storage
**Date**: 2026-01-22

## Overview

This feature extends the existing LocalStorage service to provide complete persistence for:
- Connection groups (organization)
- Query folders (hierarchical saved queries)
- Application settings (user preferences)
- Editor tab state (session continuity)
- Window state (UI layout)
- History pruning (automatic cleanup)
- Data export/import (backup/restore)

## Prerequisites

- Rust 1.80+ installed
- Project builds successfully: `cargo build`
- Existing storage infrastructure in `crates/tusk_core/src/services/storage.rs`

## Key Files to Modify

| File | Changes |
|------|---------|
| `crates/tusk_core/src/services/storage.rs` | Add Migration 2, new CRUD methods |
| `crates/tusk_core/src/models/mod.rs` | Export new types |
| `crates/tusk_core/src/models/connection.rs` | Add ConnectionGroup |
| `crates/tusk_core/src/models/settings.rs` | **NEW**: AppSettings, WindowState, EditorTab |
| `crates/tusk_core/src/services/mod.rs` | Export new types from storage |

## Implementation Order

### Phase 1: Schema & Models

1. **Add ConnectionGroup model** to `models/connection.rs`
2. **Create settings.rs** with AppSettings, WindowState, EditorTab, QueryFolder
3. **Add Migration 2** to storage.rs:
   - Create `connection_groups` table
   - Create `query_folders` table
   - Add `group_id` column to `connections`
   - Add `folder_id` column to `saved_queries`
   - Create indexes

### Phase 2: Connection Groups

1. Implement `save_connection_group()`
2. Implement `load_connection_group()`, `load_all_connection_groups()`
3. Implement `delete_connection_group()`, `reorder_connection_groups()`
4. Modify `save_connection()` to handle group_id
5. Modify `row_to_connection_config()` to include group
6. Write tests

### Phase 3: Query Folders

1. Implement `save_query_folder()` with cycle detection
2. Implement `load_query_folder()`, `load_query_folders()`, `load_all_query_folders()`
3. Implement `delete_query_folder()` with cascade
4. Implement `move_query_to_folder()`
5. Update saved query loading to include folder_id
6. Write tests

### Phase 4: Application Settings

1. Implement `load_settings()` with defaults
2. Implement `save_setting()` for individual keys
3. Implement `reset_settings()`
4. Write tests

### Phase 5: Editor & Window State

1. Implement `save_editor_tabs()`, `load_editor_tabs()`
2. Implement `save_window_state()`, `load_window_state()`
3. Write tests

### Phase 6: History Pruning

1. Implement `prune_history()` by count
2. Implement `prune_history_by_age()`
3. Modify `add_to_history()` to auto-prune when exceeding limit
4. Write tests

### Phase 7: Export/Import

1. Define `ExportData` structure
2. Implement `export_data()`
3. Implement `import_data()` with conflict resolution
4. Write round-trip tests

## Code Patterns

### Adding a New CRUD Operation

```rust
// In storage.rs

/// Save a connection group.
pub fn save_connection_group(&self, group: &ConnectionGroup) -> Result<(), TuskError> {
    let conn = self.connection.lock();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO connection_groups (group_id, name, color, sort_order, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(group_id) DO UPDATE SET
            name = excluded.name,
            color = excluded.color,
            sort_order = excluded.sort_order",
        params![
            group.id.to_string(),
            group.name,
            group.color,
            group.sort_order,
            now,
        ],
    )
    .map_err(|e| TuskError::storage(format!("Failed to save group: {e}"), None))?;

    tracing::debug!(group_id = %group.id, name = %group.name, "Group saved");
    Ok(())
}
```

### Migration Pattern

```rust
// In migrate_schema()

// Migration 2: Connection groups and query folders
if current_step < 2 {
    conn.execute_batch(
        "
        CREATE TABLE connection_groups (
            group_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            color TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        ) STRICT;

        ALTER TABLE connections ADD COLUMN group_id TEXT
            REFERENCES connection_groups(group_id) ON DELETE SET NULL;

        -- ... more DDL ...
        ",
    )
    .map_err(|e| TuskError::storage(format!("Migration 2 failed: {e}"), None))?;

    conn.execute(
        "INSERT INTO migrations (domain, step, migration) VALUES (?, 2, 'groups_and_folders')",
        [DOMAIN],
    )
    .map_err(|e| TuskError::storage(format!("Failed to record migration: {e}"), None))?;

    tracing::info!("Applied migration 2: groups_and_folders");
}
```

### Settings Access Pattern

```rust
// In AppSettings impl

impl AppSettings {
    const KEY_PREFIX: &'static str = "settings.";

    pub fn load(storage: &LocalStorage) -> Result<Self, TuskError> {
        let mut settings = Self::default();

        // Load each setting individually, keeping defaults for missing
        if let Some(value) = storage.load_ui_state(&format!("{}theme", Self::KEY_PREFIX))? {
            if let Some(s) = value.as_str() {
                settings.theme = s.to_string();
            }
        }
        // ... repeat for each setting ...

        Ok(settings)
    }
}
```

## Testing

```bash
# Run all storage tests
cargo test -p tusk_core storage

# Run specific test
cargo test -p tusk_core test_connection_groups

# Run with logging
RUST_LOG=debug cargo test -p tusk_core storage -- --nocapture
```

### Test Template

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, LocalStorage) {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::open(temp_dir.path().to_path_buf()).unwrap();
        (temp_dir, storage)
    }

    #[test]
    fn test_connection_group_crud() {
        let (_temp, storage) = setup();

        // Create
        let group = ConnectionGroup::new("Production", Some("#FF5733".to_string()));
        storage.save_connection_group(&group).unwrap();

        // Read
        let loaded = storage.load_connection_group(group.id).unwrap().unwrap();
        assert_eq!(loaded.name, "Production");

        // Update
        let mut updated = loaded.clone();
        updated.name = "Prod".to_string();
        storage.save_connection_group(&updated).unwrap();

        // Delete
        storage.delete_connection_group(group.id).unwrap();
        assert!(storage.load_connection_group(group.id).unwrap().is_none());
    }
}
```

## Verification Checklist

After implementation, verify:

- [ ] `cargo build` succeeds
- [ ] `cargo test -p tusk_core` passes
- [ ] Migration runs on fresh database
- [ ] Migration runs on existing database (upgrade path)
- [ ] Connection groups CRUD works
- [ ] Query folders CRUD works with hierarchy
- [ ] Settings persist across restarts
- [ ] Editor tabs restore correctly
- [ ] Window state restores correctly
- [ ] History prunes automatically
- [ ] Export produces valid JSON
- [ ] Import handles conflicts correctly
- [ ] Performance: 100 connections load <200ms
- [ ] Performance: 10k history entries load <100ms
