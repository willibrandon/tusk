# Credential Storage Contract

**Module**: `tusk_core::services::credentials`
**Requirements**: FR-017 through FR-019a

---

## Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `KEYRING_SERVICE` | `"dev.tusk.Tusk"` | Service name for OS keychain entries |

---

## CredentialService

Secure credential storage service (FR-017, FR-019).

Stores passwords in OS keychain. Falls back to in-memory session storage when keychain is unavailable (FR-019a).

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `available` | `bool` | Whether keychain is accessible |
| `fallback_reason` | `Option<String>` | Reason keychain is unavailable |
| `session_store` | `Option<RwLock<HashMap<String, String>>>` | In-memory fallback |

### Constructor

```rust
// Create a new credential service
// Checks keychain availability at startup
// If unavailable, enables in-memory session storage with warning (FR-019a)
fn new() -> Self
```

### Status Methods

```rust
// Check if keychain is available
fn is_available(&self) -> bool

// Get reason keychain is unavailable
fn unavailable_reason(&self) -> Option<&str>

// Check if using in-memory fallback
fn is_using_fallback(&self) -> bool
```

### Database Password Operations

```rust
// Store a database password (FR-017, FR-018)
// Password stored in OS keychain under key `db:{connection_id}`
// If keychain unavailable, stores in session memory (FR-019a)
// Password is NEVER logged or written to files (FR-018)
// Completes within 500ms (SC-005)
fn store_password(&self, connection_id: Uuid, password: &str) -> Result<(), TuskError>

// Retrieve a database password (FR-019)
// Returns None if password not found
// Completes within 500ms (SC-005)
fn get_password(&self, connection_id: Uuid) -> Result<Option<String>, TuskError>

// Delete a database password (FR-019)
fn delete_password(&self, connection_id: Uuid) -> Result<(), TuskError>

// Check if a password exists (FR-019)
fn has_password(&self, connection_id: Uuid) -> Result<bool, TuskError>
```

### SSH Passphrase Operations

```rust
// Store an SSH passphrase (stored under key `ssh:{tunnel_id}`)
fn store_ssh_passphrase(&self, tunnel_id: Uuid, passphrase: &str) -> Result<(), TuskError>

// Retrieve an SSH passphrase
fn get_ssh_passphrase(&self, tunnel_id: Uuid) -> Result<Option<String>, TuskError>

// Delete an SSH passphrase
fn delete_ssh_passphrase(&self, tunnel_id: Uuid) -> Result<(), TuskError>
```

### Session Management

```rust
// Clear all session-stored credentials (called on app exit)
// Only affects in-memory fallback storage, not OS keychain
fn clear_session_credentials(&self)
```

---

## Keychain Key Format

| Credential Type | Key Format | Example |
|-----------------|------------|---------|
| Database password | `db:{connection_id}` | `db:550e8400-e29b-41d4-a716-446655440000` |
| SSH passphrase | `ssh:{tunnel_id}` | `ssh:6ba7b810-9dad-11d1-80b4-00c04fd430c8` |

---

## Platform Support

| Platform | Backend | Feature Flag |
|----------|---------|--------------|
| macOS | Keychain Services | `apple-native` |
| Windows | Credential Manager | `windows-native` |
| Linux | D-Bus Secret Service | `sync-secret-service` |

**Note**: Avoid `async-secret-service` feature due to deadlock issues with Tokio runtime.

---

## Fallback Behavior (FR-019a)

When keychain is unavailable:

1. **Detection**: At startup, attempt test set/delete operation
2. **Warning**: Show user message explaining passwords will only be stored for session
3. **Storage**: Use in-memory HashMap protected by RwLock
4. **Cleanup**: Clear session credentials on app exit
5. **Never**: Store passwords in plaintext files

### User-Facing Message

> "Keychain unavailable. Passwords will only be stored for this session."

---

## Thread Safety

CredentialService is `Send + Sync`:
- `session_store` uses `parking_lot::RwLock`
- Keyring operations are serialized internally
- Safe to access from multiple threads
