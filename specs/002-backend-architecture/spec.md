# Feature Specification: Backend Architecture

**Feature Branch**: `002-backend-architecture`
**Created**: 2026-01-20
**Status**: Draft
**Input**: User description: "Establish the Rust backend structure for a pure GPUI application with proper module organization, error handling, async patterns, and service layer abstractions. Since GPUI is a pure Rust framework, there is no frontend/backend split—the entire application is Rust code. This document focuses on the service layer, data access patterns, and async execution that power the UI."

## Clarifications

### Session 2026-01-20

- Q: What happens when a connection pool is exhausted? → A: Wait with configurable timeout (default 30s), then return a timeout error
- Q: What happens when the data directory cannot be created (permissions issue)? → A: Show clear error with options to select a different location or exit (A+D hybrid approach)
- Q: How does the system handle keychain access being denied by the OS? → A: Use in-memory session storage (password stored in memory for current session only, cleared on app exit). Show warning explaining passwords will only be stored for this session.
- Q: How does query cancellation behave if the database has already completed the query? → A: Return results normally. Cancellation is best-effort; if query completed before cancellation reached the database, return the results.
- Q: What happens when logging initialization fails (disk full, permissions)? → A: Continue without file logging (console only). Show warning but don't block app startup.

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Robust Error Handling (Priority: P1)

As a developer working on Tusk, I need a comprehensive error handling system that captures detailed error information so that users see helpful error messages and developers can debug issues effectively.

**Why this priority**: Error handling is foundational - every other feature depends on consistent, informative error reporting. Without this, debugging becomes impossible and users receive unhelpful feedback.

**Independent Test**: Can be fully tested by triggering various error conditions (connection failures, query errors, storage errors) and verifying that appropriate error details are captured, categorized, and convertible to user-friendly messages.

**Acceptance Scenarios**:

1. **Given** a database connection fails, **When** the system processes the error, **Then** the error includes the failure message, optional source details, and is categorized as a connection error
2. **Given** a query execution fails with a syntax error, **When** the system processes the error, **Then** the error includes the message, position in query, PostgreSQL error code, detail, and hint if available
3. **Given** any error occurs, **When** the error is converted for UI display, **Then** it includes error type, user-friendly message, and actionable hint where applicable
4. **Given** an authentication fails, **When** the system processes the error, **Then** the hint suggests checking username and password

---

### User Story 2 - Application State Management (Priority: P1)

As a user of Tusk, I need the application to maintain consistent state across all components so that my active connections, running queries, and cached schema information remain synchronized.

**Why this priority**: State management is the backbone of the application - all UI components depend on accessing and updating shared state reliably.

**Independent Test**: Can be fully tested by creating connections, running queries, and verifying state is accessible, updatable, and consistent across simulated component access.

**Acceptance Scenarios**:

1. **Given** the application starts, **When** global state is initialized, **Then** the data directory is created and local storage is available
2. **Given** a connection is established, **When** it's added to state, **Then** it becomes accessible by its identifier from any component
3. **Given** multiple queries are running, **When** a query is cancelled, **Then** only that specific query is affected and its cancellation is tracked
4. **Given** a connection is removed, **When** state is updated, **Then** the associated schema cache is also removed

---

### User Story 3 - Database Connection Pooling (Priority: P1)

As a user of Tusk, I need reliable database connections that are managed efficiently so that queries execute promptly without exhausting database resources.

**Why this priority**: Connection pooling is essential for performance - without it, every query would require a new connection, causing delays and potential connection exhaustion.

**Independent Test**: Can be fully tested by creating a connection pool, acquiring connections, executing queries, and verifying pool status (size, available, waiting).

**Acceptance Scenarios**:

1. **Given** a valid connection configuration, **When** a connection pool is created, **Then** a test connection is established to verify connectivity
2. **Given** an active connection pool, **When** multiple components need connections, **Then** they can acquire connections from the pool without creating new ones
3. **Given** a pool with connections in use, **When** pool status is queried, **Then** it reports current size, available connections, and waiting requests

---

### User Story 4 - Query Execution with Cancellation (Priority: P2)

As a user of Tusk, I need to execute queries with the ability to cancel long-running ones so that I can stop queries that take too long or were executed by mistake.

**Why this priority**: Query cancellation prevents users from being stuck waiting for runaway queries - critical for usability but depends on state management and connection pooling.

**Independent Test**: Can be fully tested by executing a query, initiating cancellation, and verifying the query stops and returns an appropriate cancellation indication.

**Acceptance Scenarios**:

1. **Given** a valid connection, **When** a query is executed, **Then** it returns a unique query identifier for tracking
2. **Given** a running query, **When** cancellation is requested, **Then** the query is interrupted and returns a cancelled status
3. **Given** a query completes, **When** the query handle is unregistered, **Then** it is removed from active query tracking

---

### User Story 5 - Secure Credential Storage (Priority: P2)

As a user of Tusk, I need my database passwords stored securely in the operating system's keychain so that credentials are never exposed in files or logs.

**Why this priority**: Security is non-negotiable - passwords must never be stored in plain text. This enables saved connections to reconnect without re-entering passwords.

**Independent Test**: Can be fully tested by storing a password, retrieving it, checking existence, and deleting it from the keychain.

**Acceptance Scenarios**:

1. **Given** a connection with a password, **When** the password is stored, **Then** it is saved to the OS keychain under a unique key
2. **Given** a stored password, **When** retrieval is requested, **Then** the correct password is returned
3. **Given** a stored password, **When** deletion is requested, **Then** the password is removed from the keychain
4. **Given** a non-existent password key, **When** retrieval is attempted, **Then** a specific "credential not found" error is returned

---

### User Story 6 - Asynchronous Task Execution (Priority: P2)

As a developer building Tusk features, I need async operations to run in the background so that the UI remains responsive during database operations.

**Why this priority**: Without background execution, database operations would freeze the UI - essential for a responsive application experience.

**Independent Test**: Can be fully tested by spawning background tasks, verifying they execute asynchronously, and confirming results are returned without blocking.

**Acceptance Scenarios**:

1. **Given** a database operation request, **When** it's submitted, **Then** it executes in the background while UI remains responsive
2. **Given** a background task completes, **When** the result is ready, **Then** the calling component can access the result
3. **Given** multiple background tasks, **When** they run concurrently, **Then** they don't block each other

---

### User Story 7 - Application Logging (Priority: P3)

As a developer or user troubleshooting Tusk, I need structured logs written to both console and files so that I can diagnose issues effectively.

**Why this priority**: Logging is essential for debugging and support but doesn't block core functionality - users can work without it, but troubleshooting becomes harder.

**Independent Test**: Can be fully tested by generating log events at various levels and verifying output appears in both console and rotating log files.

**Acceptance Scenarios**:

1. **Given** the application starts, **When** logging is initialized, **Then** logs are written to console and daily rotating log files
2. **Given** a debug build, **When** logging is active, **Then** more verbose logs are captured (debug level)
3. **Given** a production build, **When** logging is active, **Then** only important logs are captured (info level and above)

---

### Edge Cases

- When the data directory cannot be created (permissions issue): Show clear error with options to select a different location or exit
- When keychain access is denied by OS: use in-memory session storage with warning, cleared on app exit
- When connection pool is exhausted: wait with configurable timeout (default 30s), then return a pool timeout error
- When query cancellation races with completion: return results normally (cancellation is best-effort)
- When logging initialization fails (disk full, permissions): continue with console-only logging, show warning
- When local storage is corrupted: backup corrupted database with timestamp, recreate fresh database, fall back to in-memory if recreation fails, notify user of data reset

## Requirements _(mandatory)_

### Functional Requirements

**Error Handling**:
- **FR-001**: System MUST categorize errors into distinct types: connection, authentication, SSL, SSH, query, storage, keyring, and internal
- **FR-002**: System MUST capture PostgreSQL-specific error details including message, detail, hint, position, and error code
- **FR-003**: System MUST provide error-to-user-display conversion with actionable hints
- **FR-004**: System MUST support error conversion from external sources (database driver, storage, serialization, I/O)

**State Management**:
- **FR-005**: System MUST maintain a central state accessible from any application component
- **FR-006**: System MUST track active database connections by unique identifier
- **FR-007**: System MUST track schema caches per connection
- **FR-008**: System MUST track active queries with cancellation capability
- **FR-009**: System MUST provide thread-safe read/write access to state

**Connection Pooling**:
- **FR-010**: System MUST pool database connections to reuse connections efficiently
- **FR-011**: System MUST test connection validity when creating a pool
- **FR-012**: System MUST support connection configuration including host, port, database, user, SSL mode, and timeouts
- **FR-013**: System MUST report pool status (size, available, waiting)
- **FR-013a**: System MUST wait with configurable timeout (default 30s) when pool is exhausted, then return a pool timeout error

**Query Execution**:
- **FR-014**: System MUST assign unique identifiers to each query execution
- **FR-015**: System MUST support query cancellation via cancellation tokens
- **FR-016**: System MUST unregister queries upon completion or cancellation

**Credential Management**:
- **FR-017**: System MUST store passwords in the operating system's native keychain
- **FR-018**: System MUST NEVER log or persist passwords in plain text
- **FR-019**: System MUST support password retrieval, storage, deletion, and existence checking
- **FR-019a**: System MUST fall back to in-memory session storage when keychain access is denied, showing a warning and clearing credentials on app exit

**Async Execution**:
- **FR-020**: System MUST execute database operations in background threads
- **FR-021**: System MUST NOT block the UI thread during database operations

**Logging**:
- **FR-022**: System MUST log to console and file simultaneously
- **FR-023**: System MUST rotate log files daily
- **FR-024**: System MUST support different log verbosity levels based on build type
- **FR-024a**: System MUST continue with console-only logging if file logging initialization fails, showing a warning but not blocking startup

**Data Directory**:
- **FR-025**: System MUST create application data directory if it doesn't exist
- **FR-026**: System MUST use OS-appropriate data directory paths in production
- **FR-027**: System MUST use a local development directory in debug builds
- **FR-027a**: System MUST show a clear error with options to select an alternate directory or exit when the data directory cannot be created due to permissions

### Key Entities

- **TuskError**: Represents all possible error conditions in the application, categorized by type with associated context (message, detail, hint, position, code)
- **ErrorInfo**: User-displayable error information extracted from errors for UI presentation
- **TuskState**: Central application state holding active connections, schema caches, query handles, local storage access, and data directory path
- **ConnectionConfig**: Configuration for a database connection including identity, server details, SSL settings, SSH tunnel settings, and connection options
- **ConnectionPool**: A managed pool of database connections for a single connection configuration
- **QueryHandle**: A handle for tracking and potentially cancelling a running query
- **QueryResult**: Results from query execution including columns, rows, affected count, execution time, and query type
- **QueryHistoryEntry**: A record of a previously executed query for history tracking

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: All error conditions produce errors with appropriate type classification and actionable information within 10ms of occurrence
- **SC-002**: Application state initialization completes within 100ms on application startup
- **SC-003**: Connection pool creation and initial connection test completes within the configured connection timeout (default 10 seconds)
- **SC-004**: Query cancellation request propagates to running query within 50ms of request
- **SC-005**: Password storage and retrieval operations complete within 500ms using OS keychain
- **SC-006**: Background tasks execute without blocking UI interactions (UI remains responsive during database operations)
- **SC-007**: Log entries are written to both console and file with less than 100ms latency
- **SC-008**: 100% of error types have corresponding user-friendly hints where applicable
- **SC-009**: Application handles 10 concurrent database operations without state corruption
- **SC-010**: Pool reports accurate status (size, available, waiting) at all times
