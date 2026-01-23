# Contract: Error Handling

**Feature**: 004-service-integration
**Type**: Internal Rust API (error types and display)
**Location**: `crates/tusk_core/src/error.rs`

## Overview

Error handling in Tusk follows a two-layer design:
1. **TuskError**: Internal error type with full technical context
2. **ErrorInfo**: User-facing display format with actionable hints

## TuskError Enum

All service operations return `Result<T, TuskError>`.

```rust
#[derive(Debug, thiserror::Error)]
pub enum TuskError {
    #[error("{message}")]
    Connection {
        message: String,
        #[source] source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("{message}")]
    Authentication {
        message: String,
        hint: Option<String>,
    },

    #[error("{message}")]
    Ssl {
        message: String,
        #[source] source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("{message}")]
    Ssh {
        message: String,
        #[source] source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("{message}")]
    Query {
        message: String,
        detail: Option<String>,
        hint: Option<String>,
        position: Option<usize>,
        code: Option<String>,
    },

    #[error("Query cancelled")]
    QueryCancelled {
        query_id: Uuid,
    },

    #[error("{message}")]
    Storage {
        message: String,
        hint: Option<String>,
        #[source] source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("{message}")]
    Keyring {
        message: String,
        hint: Option<String>,
    },

    #[error("{message}")]
    PoolTimeout {
        message: String,
        waiting: usize,
    },

    #[error("{message}")]
    Internal {
        message: String,
        #[source] source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}
```

---

## ErrorInfo Structure

User-facing error display format.

```rust
pub struct ErrorInfo {
    /// Error category for display header (e.g., "Query Error")
    pub error_type: String,

    /// User-friendly message
    pub message: String,

    /// Actionable suggestion (e.g., "Check your SQL syntax")
    pub hint: Option<String>,

    /// Technical detail for debugging
    pub technical_detail: Option<String>,

    /// Character position for query errors (1-indexed)
    pub position: Option<usize>,

    /// PostgreSQL error code (e.g., "42P01" for undefined table)
    pub code: Option<String>,

    /// Whether the error is recoverable (affects display type)
    pub recoverable: bool,
}
```

---

## Conversion: TuskError → ErrorInfo

```rust
impl TuskError {
    pub fn to_error_info(&self) -> ErrorInfo {
        match self {
            TuskError::Connection { message, .. } => ErrorInfo {
                error_type: "Connection Error".to_string(),
                message: message.clone(),
                hint: Some("Check that the database server is running and accessible.".to_string()),
                technical_detail: None,
                position: None,
                code: None,
                recoverable: true,
            },

            TuskError::Authentication { message, hint } => ErrorInfo {
                error_type: "Authentication Failed".to_string(),
                message: message.clone(),
                hint: hint.clone().or(Some("Verify username and password.".to_string())),
                technical_detail: None,
                position: None,
                code: None,
                recoverable: true,
            },

            TuskError::Query { message, detail, hint, position, code } => ErrorInfo {
                error_type: "Query Error".to_string(),
                message: message.clone(),
                hint: hint.clone(),
                technical_detail: detail.clone(),
                position: *position,
                code: code.clone(),
                recoverable: true,
            },

            TuskError::QueryCancelled { .. } => ErrorInfo {
                error_type: "Query Cancelled".to_string(),
                message: "Query was cancelled by user.".to_string(),
                hint: None,
                technical_detail: None,
                position: None,
                code: None,
                recoverable: true,
            },

            TuskError::PoolTimeout { message, waiting } => ErrorInfo {
                error_type: "Connection Pool Timeout".to_string(),
                message: message.clone(),
                hint: Some(format!("{} queries waiting. Consider closing unused tabs.", waiting)),
                technical_detail: None,
                position: None,
                code: None,
                recoverable: true,
            },

            TuskError::Keyring { message, hint } => ErrorInfo {
                error_type: "Credential Storage Error".to_string(),
                message: message.clone(),
                hint: hint.clone().or(Some("Password will be stored for this session only.".to_string())),
                technical_detail: None,
                position: None,
                code: None,
                recoverable: true,
            },

            TuskError::Internal { message, .. } => ErrorInfo {
                error_type: "Internal Error".to_string(),
                message: message.clone(),
                hint: Some("Please report this issue.".to_string()),
                technical_detail: None,
                position: None,
                code: None,
                recoverable: false,
            },

            // ... other variants
        }
    }
}
```

---

## Display Rules

| ErrorInfo.recoverable | Display Method | Auto-Dismiss |
|----------------------|----------------|--------------|
| true | Toast notification | Yes (10s) |
| true + has position | Error panel | No |
| false | Error panel + log | No |

### Toast Notification (Recoverable)

```rust
// For recoverable errors without position
if error_info.recoverable && error_info.position.is_none() {
    workspace.show_toast(
        StatusToast::new(&error_info.message, cx, |toast, _| {
            toast.icon(ToastIcon::new(IconName::Warning).color(Color::Warning))
        }),
        cx,
    );
}
```

### Error Panel (Query Errors)

```rust
// For query errors with position
if error_info.position.is_some() {
    results_panel.show_error(error_info, cx);
    // Error panel shows:
    // - Error type header
    // - Message
    // - Position indicator (highlights error in editor)
    // - Detail (if available)
    // - Hint (if available)
    // - PostgreSQL error code (if available)
}
```

### Critical Error (Non-Recoverable)

```rust
// For non-recoverable errors
if !error_info.recoverable {
    workspace.show_modal(
        ErrorModal::new(error_info),
        cx,
    );
    tracing::error!(
        error_type = %error_info.error_type,
        message = %error_info.message,
        "Critical error occurred"
    );
}
```

---

## PostgreSQL Error Code Mapping

Common PostgreSQL error codes and their handling:

| Code | Class | Description | Display Hint |
|------|-------|-------------|--------------|
| 28P01 | 28 | Invalid password | "Check your password" |
| 3D000 | 3D | Invalid database | "Database does not exist" |
| 42P01 | 42 | Undefined table | "Table '{name}' does not exist" |
| 42601 | 42 | Syntax error | "Check SQL syntax near position {pos}" |
| 42501 | 42 | Insufficient privilege | "Permission denied on '{object}'" |
| 53300 | 53 | Too many connections | "Server connection limit reached" |
| 57014 | 57 | Query cancelled | "Query was cancelled" |
| 57P01 | 57 | Admin shutdown | "Server is shutting down" |

---

## Logging Contract

**FR-024**: Service calls at DEBUG level
```rust
tracing::debug!(
    connection_id = %id,
    operation = "connect",
    elapsed_ms = elapsed,
    "Connection established"
);
```

**FR-025**: Errors at WARN/ERROR level
```rust
tracing::warn!(
    error_type = %err.error_type(),
    code = err.pg_code().as_deref().unwrap_or("N/A"),
    "Query failed"
);
```

**FR-026**: NEVER log sensitive data
```rust
// WRONG - logs password
tracing::debug!(password = %password, "Connecting...");

// CORRECT - no sensitive data
tracing::debug!(host = %config.host, database = %config.database, "Connecting...");
```

---

## Error Recovery Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Error     │────▶│ to_error_   │────▶│  ErrorInfo  │
│  (TuskError)│     │    info()   │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
                           ┌───────────────────┼───────────────────┐
                           ▼                   ▼                   ▼
                    ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
                    │    Toast    │     │ Error Panel │     │ Error Modal │
                    │ (recoverable│     │ (has pos/   │     │   (non-     │
                    │  no pos)    │     │  detail)    │     │ recoverable)│
                    └─────────────┘     └─────────────┘     └─────────────┘
```

---

## Documented Error Scenarios (SC-006)

This enumeration defines the complete set of error scenarios that MUST have actionable hints per SC-006.

| ID | Scenario | Error Type | Expected Hint |
|----|----------|------------|---------------|
| E01 | Invalid password | Authentication | "Check your password and try again" |
| E02 | Unknown host | Connection | "Verify the hostname is correct and reachable" |
| E03 | Connection refused | Connection | "Check that PostgreSQL is running on the specified port" |
| E04 | Connection timeout | Connection | "Server may be slow or unreachable. Check network connectivity" |
| E05 | Database does not exist | Connection | "Database '{name}' does not exist on this server" |
| E06 | SSL required but not available | SSL | "Server requires SSL. Enable SSL in connection settings" |
| E07 | Certificate validation failed | SSL | "Server certificate is invalid. Check SSL settings" |
| E08 | SQL syntax error | Query | "Check SQL syntax near position {pos}" |
| E09 | Undefined table | Query | "Table '{name}' does not exist in schema '{schema}'" |
| E10 | Undefined column | Query | "Column '{name}' does not exist in table '{table}'" |
| E11 | Permission denied | Query | "Insufficient privileges on '{object}'" |
| E12 | Query cancelled by user | QueryCancelled | (none - informational only) |
| E13 | Query cancelled by admin | Query | "Query was cancelled by database administrator" |
| E14 | Connection pool timeout | PoolTimeout | "{n} queries waiting. Consider closing unused tabs" |
| E15 | Connection lost mid-query | Connection | "Connection to server lost. Reconnect to continue" |
| E16 | Keychain access denied | Keyring | "Password will be stored for this session only" |
| E17 | Keychain unavailable | Keyring | "Password will be stored for this session only" |
| E18 | Server shutting down | Connection | "Database server is shutting down" |
| E19 | Too many connections | Connection | "Server connection limit reached. Try again later" |
| E20 | No active connection | Internal | "No active connection. Connect to a database first" |
| E21 | Zero rows returned | (none) | (informational - "No rows returned" in results panel) |

**Verification**: SC-006 compliance requires that each scenario above displays the expected hint (or equivalent) when triggered.

---

## FR Coverage Summary

| FR | Description | Implementation |
|----|-------------|----------------|
| FR-019 | User-friendly format | ErrorInfo struct |
| FR-020 | Distinguish recoverable | ErrorInfo.recoverable flag |
| FR-021 | Preserve query error context | position, code fields |
| FR-022 | Toast notifications | recoverable = true |
| FR-023 | Error panels | critical errors |
| FR-024 | DEBUG logging | tracing::debug! |
| FR-025 | WARN/ERROR logging | tracing::warn!, tracing::error! |
| FR-026 | No sensitive data in logs | explicit exclusion |
