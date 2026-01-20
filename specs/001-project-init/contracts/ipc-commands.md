# IPC Commands Contract: Project Initialization

**Feature**: 001-project-init
**Date**: 2026-01-19

## Overview

Project initialization establishes the IPC infrastructure but defines minimal commands. This contract documents the command pattern and the single health-check command included in the scaffold.

---

## Command Pattern

All Tauri commands follow this structure:

### Rust Handler
```rust
#[tauri::command]
async fn command_name(
    state: State<'_, AppState>,
    param: ParamType
) -> Result<ReturnType, AppError> {
    // Implementation
}
```

### TypeScript Invocation
```typescript
import { invoke } from '@tauri-apps/api/core';

const result = await invoke<ReturnType>('command_name', { param: value });
```

---

## Commands

### 1. get_app_info

**Purpose**: Health check and application metadata retrieval

**Rust Signature**:
```rust
#[tauri::command]
fn get_app_info() -> AppInfo
```

**Request**: None (no parameters)

**Response**:
```typescript
interface AppInfo {
  name: string;       // "Tusk"
  version: string;    // "0.1.0"
  tauriVersion: string; // Tauri runtime version
  platform: string;   // "macos" | "windows" | "linux"
}
```

**Example**:
```typescript
const info = await invoke<AppInfo>('get_app_info');
// { name: "Tusk", version: "0.1.0", tauriVersion: "2.9.5", platform: "macos" }
```

**Errors**: None (infallible command)

---

## Error Contract

All commands that can fail return `Result<T, AppError>`. The error type is serialized as:

```typescript
interface AppError {
  code: string;       // Machine-readable error code
  message: string;    // Human-readable message
  detail?: string;    // Technical detail for debugging
  hint?: string;      // Actionable suggestion
}
```

### Error Codes (Reserved for Future Features)
| Code | Category | Description |
|------|----------|-------------|
| `ERR_CONNECTION_*` | Connection | Database connection errors |
| `ERR_QUERY_*` | Query | Query execution errors |
| `ERR_STORAGE_*` | Storage | Local storage errors |
| `ERR_AUTH_*` | Auth | Credential/keychain errors |

---

## Event Contract

Events are emitted from Rust to frontend for async operations:

### Pattern
```rust
// Rust
app.emit("event_name", payload)?;

// TypeScript
import { listen } from '@tauri-apps/api/event';
const unlisten = await listen<PayloadType>('event_name', (event) => {
  console.log(event.payload);
});
```

### Reserved Events (Future Features)
| Event | Payload | Description |
|-------|---------|-------------|
| `query:rows` | RowBatch | Streaming query results |
| `query:complete` | QueryComplete | Query finished |
| `connection:status` | ConnectionStatus | Connection state change |

---

## Registration

Commands are registered in `src-tauri/src/lib.rs`:

```rust
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
        get_app_info,
        // Future commands registered here
    ])
```

---

## Capabilities

For project init, minimal capabilities are required:

**src-tauri/capabilities/default.json**:
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

Additional permissions added as features require them.

---

## Notes

- This contract covers only the scaffold phase
- Full IPC contracts defined in feature-specific documentation
- All commands are async-safe (Tokio runtime)
- Error handling follows Rust Result pattern with serde serialization
