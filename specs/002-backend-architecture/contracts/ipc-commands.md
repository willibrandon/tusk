# IPC Commands Contract: Backend Architecture

**Feature**: 002-backend-architecture
**Date**: 2026-01-19

## Overview

This document defines the Tauri IPC command contracts for the backend architecture feature. All commands follow the pattern:

```typescript
invoke<T>("command_name", { args }) → Promise<T | TuskError>
```

---

## Application Commands

### health_check

Verify backend is operational and return application metadata.

**Command**: `health_check`
**Arguments**: None
**Returns**: `AppInfo`

```typescript
interface AppInfo {
  name: string;
  version: string;
  tauriVersion: string;
  platform: "macos" | "windows" | "linux";
}

// Usage
const info = await invoke<AppInfo>("health_check");
```

**Errors**: None (infallible)

---

### get_log_directory

Get the path to the application log directory.

**Command**: `get_log_directory`
**Arguments**: None
**Returns**: `string` (path)

```typescript
const logDir = await invoke<string>("get_log_directory");
// e.g., "/Users/user/Library/Logs/com.tusk"
```

**Errors**: None (infallible)

---

## Connection Commands

### list_connections

List all saved connection configurations.

**Command**: `list_connections`
**Arguments**: None
**Returns**: `ConnectionConfig[]`

```typescript
interface ConnectionConfig {
  id: string;               // UUID
  name: string;
  host: string;
  port: number;
  database: string;
  username: string;
  sslMode: "disable" | "prefer" | "require" | "verify-ca" | "verify-full";
  sslCaCert: string | null;
  sshTunnel: SshTunnel | null;
  readOnly: boolean;
  statementTimeoutMs: number | null;
  createdAt: string;        // ISO 8601
  updatedAt: string;        // ISO 8601
}

interface SshTunnel {
  host: string;
  port: number;
  username: string;
  authMethod: SshAuthMethod;
  localPort: number | null;
}

type SshAuthMethod =
  | { type: "password" }
  | { type: "keyFile"; path: string }
  | { type: "agent" };

// Usage
const connections = await invoke<ConnectionConfig[]>("list_connections");
```

**Errors**:
- `Storage` - Failed to read from local database

---

### get_connection

Get a single connection configuration by ID.

**Command**: `get_connection`
**Arguments**: `{ id: string }`
**Returns**: `ConnectionConfig | null`

```typescript
const conn = await invoke<ConnectionConfig | null>("get_connection", { id });
```

**Errors**:
- `Storage` - Failed to read from local database

---

### save_connection

Create or update a connection configuration.

**Command**: `save_connection`
**Arguments**: `{ config: ConnectionConfig, password?: string }`
**Returns**: `ConnectionConfig`

```typescript
const saved = await invoke<ConnectionConfig>("save_connection", {
  config: {
    id: crypto.randomUUID(),
    name: "Production",
    host: "db.example.com",
    port: 5432,
    database: "myapp",
    username: "admin",
    sslMode: "require",
    sslCaCert: null,
    sshTunnel: null,
    readOnly: false,
    statementTimeoutMs: 30000,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
  password: "secretpassword",  // Optional: stored in OS keychain
});
```

**Errors**:
- `Validation` - Invalid configuration (empty name, invalid port, etc.)
- `Storage` - Failed to save to local database
- `Credential` - Failed to store password in keychain

---

### delete_connection

Delete a saved connection configuration.

**Command**: `delete_connection`
**Arguments**: `{ id: string }`
**Returns**: `void`

```typescript
await invoke("delete_connection", { id });
```

**Errors**:
- `Storage` - Failed to delete from local database
- `Credential` - Failed to delete password from keychain (non-fatal)

---

### test_connection

Test a connection configuration without saving.

**Command**: `test_connection`
**Arguments**: `{ config: ConnectionConfig, password: string }`
**Returns**: `ConnectionTestResult`

```typescript
interface ConnectionTestResult {
  success: boolean;
  serverVersion: string | null;
  latencyMs: number;
  error: TuskError | null;
}

const result = await invoke<ConnectionTestResult>("test_connection", {
  config,
  password,
});
```

**Errors**:
- `Connection` - Connection failed (with details)
- `Validation` - Invalid configuration

---

### connect

Establish a connection pool for a saved configuration.

**Command**: `connect`
**Arguments**: `{ id: string }`
**Returns**: `string` (connection pool ID, same as config ID)

```typescript
const poolId = await invoke<string>("connect", { id: connectionConfigId });
```

**Errors**:
- `Connection` - Failed to establish connection
- `Credential` - Failed to retrieve password from keychain
- `Storage` - Connection config not found

---

### disconnect

Close a connection pool and release resources.

**Command**: `disconnect`
**Arguments**: `{ id: string }`
**Returns**: `void`

```typescript
await invoke("disconnect", { id: poolId });
```

**Errors**:
- `Connection` - Pool not found (non-fatal)

---

### get_active_connections

List all currently active connection pools.

**Command**: `get_active_connections`
**Arguments**: None
**Returns**: `ActiveConnection[]`

```typescript
interface ActiveConnection {
  id: string;               // Pool ID (same as config ID)
  configName: string;
  connectedAt: string;      // ISO 8601
  activeQueries: number;
}

const active = await invoke<ActiveConnection[]>("get_active_connections");
```

**Errors**: None (infallible)

---

## Query Commands

### execute_query

Execute a SQL query and return results.

**Command**: `execute_query`
**Arguments**: `{ connectionId: string, sql: string, queryId?: string }`
**Returns**: `QueryResult`

```typescript
interface QueryResult {
  queryId: string;
  columns: Column[];
  rows: Row[];
  rowCount: number;
  elapsedMs: number;
}

interface Column {
  name: string;
  dataType: string;        // PostgreSQL type name
  nullable: boolean;
}

type Row = Record<string, JsonValue>;
type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };

const result = await invoke<QueryResult>("execute_query", {
  connectionId: poolId,
  sql: "SELECT * FROM users LIMIT 10",
  queryId: crypto.randomUUID(),  // Optional for cancellation
});
```

**Errors**:
- `Database` - Query execution error (includes position, hint, code)
- `Connection` - Pool not found or connection lost
- `QueryTimeout` - Statement timeout exceeded
- `QueryCancelled` - Query was cancelled

---

### cancel_query

Cancel a running query.

**Command**: `cancel_query`
**Arguments**: `{ queryId: string }`
**Returns**: `void`

```typescript
await invoke("cancel_query", { queryId });
```

**Errors**:
- `Internal` - Query not found (already completed or never started)

---

### get_active_queries

List all currently executing queries.

**Command**: `get_active_queries`
**Arguments**: `{ connectionId?: string }`
**Returns**: `ActiveQuery[]`

```typescript
interface ActiveQuery {
  queryId: string;
  connectionId: string;
  sql: string;              // First 100 chars truncated
  startedAt: string;        // ISO 8601
  elapsedMs: number;
}

// All active queries
const queries = await invoke<ActiveQuery[]>("get_active_queries", {});

// For specific connection
const connQueries = await invoke<ActiveQuery[]>("get_active_queries", {
  connectionId: poolId,
});
```

**Errors**: None (infallible)

---

## Storage Commands

### check_database_health

Check local SQLite database integrity.

**Command**: `check_database_health`
**Arguments**: None
**Returns**: `DatabaseHealth`

```typescript
interface DatabaseHealth {
  isHealthy: boolean;
  errors: string[];
  backupPath: string | null;  // Set if repair failed and backup was created
}

const health = await invoke<DatabaseHealth>("check_database_health");
```

**Errors**:
- `Storage` - Failed to run integrity check

---

### get_preference

Get a stored user preference.

**Command**: `get_preference`
**Arguments**: `{ key: string }`
**Returns**: `JsonValue | null`

```typescript
const theme = await invoke<string | null>("get_preference", { key: "theme" });
```

**Errors**:
- `Storage` - Failed to read from database

---

### set_preference

Set a user preference.

**Command**: `set_preference`
**Arguments**: `{ key: string, value: JsonValue }`
**Returns**: `void`

```typescript
await invoke("set_preference", { key: "theme", value: "dark" });
```

**Errors**:
- `Storage` - Failed to write to database

---

## Credential Commands

### check_keychain_available

Check if OS keychain is available.

**Command**: `check_keychain_available`
**Arguments**: None
**Returns**: `boolean`

```typescript
const available = await invoke<boolean>("check_keychain_available");
```

**Errors**: None (infallible)

---

### has_stored_password

Check if a password is stored for a connection.

**Command**: `has_stored_password`
**Arguments**: `{ connectionId: string }`
**Returns**: `boolean`

```typescript
const hasPassword = await invoke<boolean>("has_stored_password", {
  connectionId,
});
```

**Errors**: None (infallible, returns false on keychain errors)

---

## Error Response Format

All commands may return errors in this format:

```typescript
interface TuskError {
  kind: ErrorKind;
  data: ErrorData;
}

type ErrorKind =
  | "Database"
  | "Connection"
  | "Storage"
  | "Credential"
  | "QueryCancelled"
  | "QueryTimeout"
  | "Initialization"
  | "Validation"
  | "Internal";

interface DatabaseErrorData {
  message: string;
  code: string | null;      // PostgreSQL error code
  position: number | null;  // Character position in SQL
  hint: string | null;      // Actionable suggestion
  detail: string | null;    // Additional context
}

interface SimpleErrorData {
  message: string;
}

interface TimeoutErrorData {
  elapsedMs: number;
}
```

**Frontend Error Handling Pattern**:

```typescript
try {
  const result = await invoke<QueryResult>("execute_query", args);
  // Handle success
} catch (error) {
  const e = error as TuskError;
  switch (e.kind) {
    case "Database":
      // Show SQL error with position highlighting
      break;
    case "QueryTimeout":
      // Show timeout message with elapsed time
      break;
    case "QueryCancelled":
      // User cancelled, no error toast needed
      break;
    default:
      // Show generic error message
      break;
  }
}
```
