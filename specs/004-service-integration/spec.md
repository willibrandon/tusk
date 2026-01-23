# Feature Specification: Service Integration Layer

**Feature Branch**: `004-service-integration`
**Created**: 2026-01-21
**Status**: Draft
**Dependencies**: 002-backend-architecture, 003-frontend-architecture

## Overview

This feature establishes how the user interface components access and interact with backend services in Tusk. As a unified Rust application (no IPC or JavaScript bridge), UI components directly call service methods, spawn background tasks for database operations, and receive streaming results for large datasets. The integration layer provides consistent error handling, query cancellation support, and responsive UI feedback during long-running operations.

## Clarifications

### Session 2026-01-21

- Q: What happens when the database server becomes unreachable during a long-running query? → A: Fail gracefully, preserve partial results received, update connection status to disconnected
- Q: What level of observability should the service integration layer provide? → A: Standard logging: service calls at DEBUG, errors at WARN/ERROR with context
- Q: What happens when schema refresh is triggered during an active query? → A: Allow in parallel; query and schema use separate connections from pool (minimum 2-3 connections per server)
- Q: How does the system behave when credential storage (keyring) is unavailable? → A: Fall back to session-only password (user enters once per app session, not persisted)

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Execute Query and View Results (Priority: P1)

A user writes a SQL query in the editor and executes it. The system runs the query against the connected database without freezing the UI, displays progress feedback during execution, and presents results when complete. If the query returns a large dataset, results stream in progressively so the user sees data immediately rather than waiting for the entire result set.

**Why this priority**: Query execution is the core function of Tusk. Without responsive query execution and result display, the application provides no value.

**Independent Test**: Can be tested by connecting to a database, running a query, and verifying results appear while the UI remains responsive.

**Acceptance Scenarios**:

1. **Given** a connected database and valid SQL query, **When** the user executes the query, **Then** results appear in the results panel and the UI remains responsive throughout execution
2. **Given** a query returning 100,000 rows, **When** the user executes the query, **Then** the first batch of results appears before the full result set is retrieved
3. **Given** a query in progress, **When** the user interacts with other UI elements, **Then** the interface responds immediately without blocking

---

### User Story 2 - Cancel Running Query (Priority: P1)

A user executes a query that takes longer than expected (e.g., complex join or table scan). The user decides to cancel it. The system stops the query execution, releases database resources, and returns the UI to a ready state without requiring an application restart.

**Why this priority**: Users frequently run queries that take unexpectedly long. Without cancellation, users must wait indefinitely or force-quit the application, losing work.

**Independent Test**: Can be tested by running a slow query (e.g., pg_sleep), clicking cancel, and verifying the query stops and the UI returns to ready state.

**Acceptance Scenarios**:

1. **Given** a query currently executing, **When** the user clicks the cancel button, **Then** the query stops and the status shows "Cancelled"
2. **Given** a streaming query delivering results, **When** the user cancels mid-stream, **Then** streaming stops and already-received results remain visible
3. **Given** a cancelled query, **When** the user runs a new query, **Then** the new query executes normally

---

### User Story 3 - Connect to Database (Priority: P1)

A user enters connection details (host, port, database, username, password) and connects to a PostgreSQL database. The system validates the connection, stores credentials securely, and makes the connection available for queries. Connection status is clearly visible in the UI.

**Why this priority**: Database connection is a prerequisite for all other functionality. Users cannot execute queries or browse schema without an active connection.

**Independent Test**: Can be tested by entering valid credentials, clicking connect, and verifying the connection status indicator shows connected.

**Acceptance Scenarios**:

1. **Given** valid connection credentials, **When** the user clicks connect, **Then** the connection is established and status shows "Connected"
2. **Given** invalid credentials, **When** the user clicks connect, **Then** an error message explains the failure with actionable hints
3. **Given** a test connection button, **When** the user clicks it, **Then** the system verifies connectivity without fully establishing a session

---

### User Story 4 - View and Navigate Database Schema (Priority: P2)

A user browses the database schema to understand table structures, views, and functions. The schema tree loads efficiently even for databases with hundreds of tables. Cached schema data provides instant navigation after initial load.

**Why this priority**: Schema browsing helps users write correct queries and understand data relationships. It enhances productivity but queries can still be written without it.

**Independent Test**: Can be tested by connecting to a database and expanding schema tree nodes to view tables, columns, and other objects.

**Acceptance Scenarios**:

1. **Given** an active connection, **When** the user opens the schema browser, **Then** database objects load and display in a navigable tree
2. **Given** a previously loaded schema, **When** the user navigates schema objects, **Then** navigation is instantaneous (from cache)
3. **Given** schema changes on the server, **When** the user refreshes schema, **Then** the cache updates with current database state

---

### User Story 5 - Handle Errors Gracefully (Priority: P2)

When database operations fail (connection errors, query syntax errors, permission issues), the user sees clear, actionable error messages. The error display includes the error title, message, detail (if available), and hints for resolution. The application remains stable and ready for the next operation.

**Why this priority**: Good error handling builds user confidence and enables self-service problem resolution. Without it, users cannot diagnose issues or recover from errors.

**Independent Test**: Can be tested by intentionally causing errors (bad SQL, wrong password) and verifying error messages are clear and helpful.

**Acceptance Scenarios**:

1. **Given** a SQL syntax error, **When** the query executes, **Then** the error panel shows the error with position indicator and hint
2. **Given** a connection failure, **When** the user attempts to connect, **Then** the error explains the cause and suggests remediation
3. **Given** a recoverable error, **When** the user corrects the issue and retries, **Then** the operation succeeds

---

### User Story 6 - Persist Application State (Priority: P3)

The application remembers saved connections, recent queries, and workspace layout between sessions. Users can quickly reconnect to previously used databases without re-entering credentials.

**Why this priority**: State persistence improves user experience but is not required for core functionality. Users can still use the application without persistence.

**Independent Test**: Can be tested by saving a connection, restarting the application, and verifying the saved connection appears in the connection list.

**Acceptance Scenarios**:

1. **Given** a saved connection, **When** the user restarts the application, **Then** the connection appears in the saved connections list
2. **Given** stored credentials, **When** the user selects a saved connection, **Then** they can connect without re-entering the password
3. **Given** workspace layout changes, **When** the user restarts the application, **Then** the layout restores to the previous state

---

### Edge Cases

- When the database server becomes unreachable during a long-running query, the system fails gracefully: preserve any partial results already received, display a connection-lost error, and update connection status to Disconnected
- When a query returns zero rows, the system emits QueryEvent::Complete with total_rows=0; the results panel displays "No rows returned" with execution time, and the UI returns to ready state
- When the user attempts to run a query without an active connection, the system displays an error toast "No active connection" and opens the connection dialog prompting the user to connect first
- When credential storage (keyring) is unavailable, system falls back to session-only password: user enters password once per application session, password held in memory only (not persisted to disk)
- Schema refresh proceeds in parallel with active queries using separate pooled connections; minimum pool size of 2-3 connections per server ensures schema browser, query execution, and multi-tab queries operate concurrently without blocking

## Requirements _(mandatory)_

### Functional Requirements

#### Service Access

- **FR-001**: System MUST provide a global state container accessible from all UI components
- **FR-002**: Services MUST be accessible through the global state without passing references through component hierarchies
- **FR-003**: System MUST support synchronous reads of cached state for immediate UI rendering
- **FR-004**: System MUST support asynchronous operations for database calls without blocking the UI thread

#### Connection Management

- **FR-005**: System MUST manage database connection pools with configurable pool sizes (minimum 2-3 connections per server to support concurrent schema browsing, query execution, and multi-tab queries)
- **FR-006**: System MUST track connection status (Disconnected, Connecting, Connected, Error) for each connection
- **FR-007**: System MUST validate connections before adding them to the pool
- **FR-008**: System MUST support disconnecting and reconnecting without application restart
- **FR-009**: System MUST retrieve passwords from OS keychain when configured to do so

#### Query Execution

- **FR-010**: System MUST execute queries asynchronously on background threads
- **FR-011**: System MUST support streaming results for queries returning large datasets
- **FR-012**: System MUST deliver results in configurable batch sizes (default: 1000 rows per batch)
- **FR-013**: System MUST support query cancellation at any point during execution
- **FR-014**: System MUST send column metadata before row data for proper result grid setup
- **FR-015**: System MUST track execution time and row counts for completed queries

#### Schema Management

- **FR-016**: System MUST load and cache database schema (tables, views, functions)
- **FR-017**: System MUST support cache invalidation and refresh on demand
- **FR-018**: System MUST apply time-to-live (TTL) for cached schema data

#### Error Handling

- **FR-019**: System MUST convert all errors to user-friendly format with title, message, detail, and hint
- **FR-020**: System MUST distinguish between recoverable and non-recoverable errors
- **FR-021**: System MUST preserve error context (position, code) for query errors
- **FR-022**: System MUST display toast notifications for recoverable errors
- **FR-023**: System MUST display error panels for critical errors

#### Observability

- **FR-024**: System MUST log service calls at DEBUG level with timing information
- **FR-025**: System MUST log errors at WARN/ERROR level with full context (error type, parameters, stack trace)
- **FR-026**: System MUST NOT log sensitive data (passwords, connection strings with credentials)

#### State Persistence

- **FR-027**: System MUST persist connection configurations to local storage
- **FR-028**: System MUST store sensitive credentials (passwords) in OS keychain, never in files
- **FR-029**: System MUST fall back to session-only password storage (in-memory only) when OS keychain is unavailable

### Key Entities

- **TuskState**: Application-wide state container holding references to all services and active connection tracking
- **ConnectionConfig**: Database connection settings (host, port, database, username, credential reference)
- **ConnectionStatus**: Current state of a database connection (Disconnected, Connecting, Connected, Error)
- **QueryEvent**: Stream events during query execution (Columns, Rows, Complete, Error)
- **ErrorInfo**: User-facing error information (title, message, detail, hint, recoverable flag)
- **Schema**: Cached database structure (tables, views, functions) with TTL metadata

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: UI remains responsive during query execution (interactions respond within 100ms)
- **SC-002**: First batch of query results appears within 500ms of query submission for simple queries
- **SC-003**: Query cancellation takes effect within 1 second of user request
- **SC-004**: Schema loads for databases with 1000+ tables within 300ms
- **SC-005**: Cached schema navigation is instantaneous (under 30ms response)
- **SC-006**: Error messages include actionable hints in 100% of documented error scenarios
- **SC-007**: Streaming queries handle result sets of 1 million+ rows without memory exhaustion
- **SC-008**: Connection pool supports 10 concurrent queries without degradation

## Assumptions

- The application runs as a single-process Rust application (no IPC layer needed)
- Users have PostgreSQL databases accessible on the network
- OS keychain is available for credential storage on all supported platforms (macOS, Windows, Linux)
- Background thread pool is available via the GPUI framework's BackgroundExecutor
- Local SQLite storage is available for persisting non-sensitive configuration
