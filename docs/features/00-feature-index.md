# Tusk Feature Implementation Index

This document defines the complete ordered sequence of feature documents required to build Tusk from scratch. Each feature must be implemented in order as later features depend on earlier ones.

**Total Documents:** 29
**Reference:** `docs/design.md`

---

## Phase 1: Foundation

These features establish the project infrastructure and must be completed first.

| #   | Document                                                                   | Description                                       | Dependencies |
| --- | -------------------------------------------------------------------------- | ------------------------------------------------- | ------------ |
| 01  | [01-project-initialization.md](./01-project-initialization.md)             | Tauri project setup, tooling, build configuration | None         |
| 02  | [02-backend-architecture.md](./02-backend-architecture.md)                 | Rust backend structure, modules, error handling   | 01           |
| 03  | [03-frontend-architecture.md](./03-frontend-architecture.md)               | Svelte 5 setup, component structure, Tailwind     | 01           |
| 04  | [04-ipc-layer.md](./04-ipc-layer.md)                                       | Tauri commands, events, serialization patterns    | 02, 03       |
| 05  | [05-local-storage.md](./05-local-storage.md)                               | SQLite schema, migrations, CRUD operations        | 02, 04       |
| 06  | [06-settings-theming-credentials.md](./06-settings-theming-credentials.md) | Settings system, light/dark theme, OS keychain    | 03, 04, 05   |

---

## Phase 2: Connection System

Core connection functionality required for all database operations.

| #   | Document                                                     | Description                                        | Dependencies |
| --- | ------------------------------------------------------------ | -------------------------------------------------- | ------------ |
| 07  | [07-connection-management.md](./07-connection-management.md) | Connection model, pooling, lifecycle, validation   | 04, 05, 06   |
| 08  | [08-ssl-ssh-security.md](./08-ssl-ssh-security.md)           | SSL/TLS modes, SSH tunneling via russh             | 07           |
| 09  | [09-connection-ui.md](./09-connection-ui.md)                 | Connection dialog, tree, groups, status indicators | 06, 07, 08   |

---

## Phase 3: Schema System

Schema introspection powers autocomplete, browser, and many other features.

| #   | Document                                                   | Description                                    | Dependencies |
| --- | ---------------------------------------------------------- | ---------------------------------------------- | ------------ |
| 10  | [10-schema-introspection.md](./10-schema-introspection.md) | Schema queries, caching, LISTEN/NOTIFY refresh | 07           |

---

## Phase 4: Query System

Query execution is the core functionality of the application.

| #   | Document                                         | Description                                           | Dependencies |
| --- | ------------------------------------------------ | ----------------------------------------------------- | ------------ |
| 11  | [11-query-execution.md](./11-query-execution.md) | Execution engine, streaming, cancellation, parsing    | 07, 10       |
| 12  | [12-monaco-editor.md](./12-monaco-editor.md)     | Monaco integration, autocomplete, syntax highlighting | 03, 10, 11   |
| 13  | [13-tabs-history.md](./13-tabs-history.md)       | Tab management, query history, saved queries          | 05, 12       |

---

## Phase 5: Results System

Display and interact with query results.

| #   | Document                                   | Description                                     | Dependencies |
| --- | ------------------------------------------ | ----------------------------------------------- | ------------ |
| 14  | [14-results-grid.md](./14-results-grid.md) | TanStack Table, virtualization, cell rendering  | 03, 11       |
| 15  | [15-export-copy.md](./15-export-copy.md)   | Export formats, copy operations, cell selection | 14           |

---

## Phase 6: Schema Browser

Navigate and explore database objects.

| #   | Document                                       | Description                                  | Dependencies |
| --- | ---------------------------------------------- | -------------------------------------------- | ------------ |
| 16  | [16-schema-browser.md](./16-schema-browser.md) | Tree view, object search, context menus, DDL | 09, 10       |

---

## Phase 7: Data Operations

View and edit table data directly.

| #   | Document                                             | Description                                       | Dependencies |
| --- | ---------------------------------------------------- | ------------------------------------------------- | ------------ |
| 17  | [17-table-data-viewer.md](./17-table-data-viewer.md) | Table viewer, filter builder, sorting, pagination | 14, 16       |
| 18  | [18-inline-editing.md](./18-inline-editing.md)       | Edit mode, change tracking, transaction handling  | 11, 17       |

---

## Phase 8: Query Analysis

Understand and optimize query performance.

| #   | Document                                                           | Description                                        | Dependencies |
| --- | ------------------------------------------------------------------ | -------------------------------------------------- | ------------ |
| 19  | [19-query-plan-visualization.md](./19-query-plan-visualization.md) | EXPLAIN options, plan parsing, tree/timeline views | 11, 14       |

---

## Phase 9: Administration

Monitor and manage database server.

| #   | Document                                                       | Description                                       | Dependencies |
| --- | -------------------------------------------------------------- | ------------------------------------------------- | ------------ |
| 20  | [20-admin-dashboard.md](./20-admin-dashboard.md)               | Activity monitor, server/table/index stats, locks | 07, 14       |
| 21  | [21-maintenance-operations.md](./21-maintenance-operations.md) | VACUUM, REINDEX, ANALYZE operations               | 16, 20       |

---

## Phase 10: User Management

Manage database roles and permissions.

| #   | Document                                         | Description                        | Dependencies |
| --- | ------------------------------------------------ | ---------------------------------- | ------------ |
| 22  | [22-role-management.md](./22-role-management.md) | Role list, editor, privileges grid | 07, 14, 16   |

---

## Phase 11: Extensions

Manage Postgres extensions.

| #   | Document                                             | Description                                | Dependencies |
| --- | ---------------------------------------------------- | ------------------------------------------ | ------------ |
| 23  | [23-extension-manager.md](./23-extension-manager.md) | Extension list, install/uninstall, details | 07, 14, 16   |

---

## Phase 12: Import/Export

Data import and backup functionality.

| #   | Document                                       | Description                                 | Dependencies |
| --- | ---------------------------------------------- | ------------------------------------------- | ------------ |
| 24  | [24-import-wizard.md](./24-import-wizard.md)   | CSV/JSON import, column mapping, transforms | 11, 14, 16   |
| 25  | [25-backup-restore.md](./25-backup-restore.md) | pg_dump/pg_restore integration, progress    | 07, 16       |

---

## Phase 13: Visualization

Visual database schema representation.

| #   | Document                               | Description                                      | Dependencies |
| --- | -------------------------------------- | ------------------------------------------------ | ------------ |
| 26  | [26-er-diagram.md](./26-er-diagram.md) | @xyflow/svelte canvas, layout algorithms, export | 10, 16       |

---

## Phase 14: Platform & Polish

Platform-specific features and final polish.

| #   | Document                                                                       | Description                                     | Dependencies |
| --- | ------------------------------------------------------------------------------ | ----------------------------------------------- | ------------ |
| 27  | [27-platform-integration.md](./27-platform-integration.md)                     | macOS/Windows/Linux specifics, native menus     | 06, 09       |
| 28  | [28-error-handling.md](./28-error-handling.md)                                 | Error display, recovery, reconnection           | 07, 11       |
| 29  | [29-keyboard-shortcuts-performance.md](./29-keyboard-shortcuts-performance.md) | Shortcuts, session restore, performance targets | All          |

---

## Implementation Notes

### Critical Path

The minimum viable path to a working query interface:

1. Project Initialization (01)
2. Backend/Frontend Architecture (02, 03)
3. IPC Layer (04)
4. Local Storage (05)
5. Connection Management (07)
6. Query Execution (11)
7. Results Grid (14)

### Testing Strategy

- Each feature document specifies testable acceptance criteria
- Use Tauri MCP for end-to-end testing
- Use Playwright MCP for isolated UI component testing
- Maintain test coverage throughout development

### No Deferrals

Every feature in `docs/design.md` is covered by these documents. Nothing is deferred to "v2" or "future work". The design document's Section 10 (Future Considerations) items are explicitly out of scope for this implementation but everything in Sections 1-9 and the Appendices is fully covered.

---

## Document Status Tracking

| Status      | Meaning                                      |
| ----------- | -------------------------------------------- |
| Not Started | Document exists but implementation not begun |
| In Progress | Active development                           |
| Review      | Implementation complete, under review        |
| Complete    | Implemented, tested, and verified            |

Update this table as implementation progresses:

| #   | Document                         | Status      |
| --- | -------------------------------- | ----------- |
| 01  | Project Initialization           | Not Started |
| 02  | Backend Architecture             | Not Started |
| 03  | Frontend Architecture            | Not Started |
| 04  | IPC Layer                        | Not Started |
| 05  | Local Storage                    | Not Started |
| 06  | Settings/Theming/Credentials     | Not Started |
| 07  | Connection Management            | Not Started |
| 08  | SSL/SSH Security                 | Not Started |
| 09  | Connection UI                    | Not Started |
| 10  | Schema Introspection             | Not Started |
| 11  | Query Execution                  | Not Started |
| 12  | Monaco Editor                    | Not Started |
| 13  | Tabs & History                   | Not Started |
| 14  | Results Grid                     | Not Started |
| 15  | Export & Copy                    | Not Started |
| 16  | Schema Browser                   | Not Started |
| 17  | Table Data Viewer                | Not Started |
| 18  | Inline Editing                   | Not Started |
| 19  | Query Plan Visualization         | Not Started |
| 20  | Admin Dashboard                  | Not Started |
| 21  | Maintenance Operations           | Not Started |
| 22  | Role Management                  | Not Started |
| 23  | Extension Manager                | Not Started |
| 24  | Import Wizard                    | Not Started |
| 25  | Backup/Restore                   | Not Started |
| 26  | ER Diagram                       | Not Started |
| 27  | Platform Integration             | Not Started |
| 28  | Error Handling                   | Not Started |
| 29  | Keyboard Shortcuts & Performance | Not Started |
