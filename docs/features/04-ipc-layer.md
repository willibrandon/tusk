# Feature 04: Service Integration Layer

> **Status:** Not Started
> **Dependencies:** 02-backend-architecture, 03-frontend-architecture
> **Estimated Complexity:** Medium

## Overview

In a pure GPUI application, there is no IPC (Inter-Process Communication) layer because the frontend and backend are unified in a single Rust process. This document specifies how GPUI UI components integrate with backend services through direct Rust calls, async task spawning, and event notification patterns.

## Goals

1. Define service access patterns from GPUI components
2. Implement async task execution for database operations
3. Create streaming result patterns for large datasets
4. Handle errors consistently across service boundaries
5. Support query cancellation and timeout

## Non-Goals

1. WebView bridges (no JavaScript)
2. IPC serialization (no process boundary)
3. TypeScript type generation (pure Rust)

---

## 1. Architecture Overview

### 1.1 Unified Rust Stack

```
┌─────────────────────────────────────────────────────────────────┐
│                        GPUI Application                          │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    UI Components                             ││
│  │   (Workspace, Panes, Editors, Grids, Panels)                ││
│  └─────────────────────────────────────────────────────────────┘│
│                              │                                   │
│                              ▼                                   │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    TuskState (Global)                        ││
│  │   - ConnectionService                                        ││
│  │   - QueryService                                             ││
│  │   - SchemaService                                            ││
│  │   - StorageService                                           ││
│  │   - KeyringService                                           ││
│  └─────────────────────────────────────────────────────────────┘│
│                              │                                   │
│                              ▼                                   │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                  BackgroundExecutor                          ││
│  │   - Async database operations                                ││
│  │   - Connection pool management                               ││
│  │   - Long-running tasks                                       ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Key Differences from Tauri IPC

| Tauri IPC                          | GPUI Service Integration           |
| ---------------------------------- | ---------------------------------- |
| `#[tauri::command]` functions      | Direct service method calls        |
| JSON serialization across boundary | Native Rust types, zero-copy       |
| `invoke()` from JavaScript         | `cx.global::<TuskState>()` access  |
| Event emission via channels        | `cx.notify()` and subscriptions    |
| TypeScript types generated         | Single type system (Rust)          |
| Process isolation                  | Shared memory, thread safety       |

---

## 2. Global State Access

### 2.1 TuskState as Global

All services are accessed through `TuskState`, registered as a GPUI Global:

```rust
// crates/tusk_core/src/state.rs

use gpui::*;
use parking_lot::RwLock;
use std::sync::Arc;

use crate::services::{
    ConnectionService,
    QueryService,
    SchemaService,
    StorageService,
    KeyringService,
};

/// Application-wide state container
pub struct TuskState {
    pub connections: Arc<ConnectionService>,
    pub queries: Arc<QueryService>,
    pub schema: Arc<SchemaService>,
    pub storage: Arc<StorageService>,
    pub keyring: Arc<KeyringService>,

    /// Active connection ID
    active_connection_id: RwLock<Option<uuid::Uuid>>,

    /// Running queries for cancellation
    running_queries: RwLock<HashMap<uuid::Uuid, QueryHandle>>,
}

impl TuskState {
    pub fn new(executor: BackgroundExecutor) -> Self {
        let storage = Arc::new(StorageService::new().expect("Failed to initialize storage"));
        let keyring = Arc::new(KeyringService::new());
        let connections = Arc::new(ConnectionService::new(executor.clone()));
        let queries = Arc::new(QueryService::new(executor.clone()));
        let schema = Arc::new(SchemaService::new(executor.clone()));

        Self {
            connections,
            queries,
            schema,
            storage,
            keyring,
            active_connection_id: RwLock::new(None),
            running_queries: RwLock::new(HashMap::new()),
        }
    }

    pub fn active_connection_id(&self) -> Option<uuid::Uuid> {
        *self.active_connection_id.read()
    }

    pub fn set_active_connection(&self, id: Option<uuid::Uuid>) {
        *self.active_connection_id.write() = id;
    }

    pub fn register_query(&self, handle: QueryHandle) -> uuid::Uuid {
        let id = handle.id;
        self.running_queries.write().insert(id, handle);
        id
    }

    pub fn cancel_query(&self, id: &uuid::Uuid) -> bool {
        if let Some(handle) = self.running_queries.write().remove(id) {
            handle.cancel();
            true
        } else {
            false
        }
    }

    pub fn unregister_query(&self, id: &uuid::Uuid) {
        self.running_queries.write().remove(id);
    }
}

impl Global for TuskState {}
```

### 2.2 Registering Global State

```rust
// crates/tusk_app/src/main.rs

fn main() {
    App::new().run(|cx: &mut App| {
        // Get background executor
        let executor = cx.background_executor().clone();

        // Initialize and register global state
        let state = TuskState::new(executor);
        cx.set_global(state);

        // Register theme
        let theme = Theme::default();
        cx.set_global(theme);

        // Open main window
        cx.open_window(window_options(), |window, cx| {
            let state = cx.global::<TuskState>();
            cx.new(|cx| Workspace::new(cx))
        });
    });
}
```

---

## 3. Service Access Patterns

### 3.1 Synchronous Access (Read State)

For reading current state, access services directly:

```rust
// In a GPUI component
impl MyComponent {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.global::<TuskState>();

        // Read active connection
        let connection_id = state.active_connection_id();

        // Read cached schema
        if let Some(id) = connection_id {
            if let Some(schema) = state.schema.get_cached(&id) {
                // Use schema for rendering
            }
        }

        // ... render UI
    }
}
```

### 3.2 Async Operations (Database Calls)

For async operations, spawn tasks on the background executor:

```rust
impl QueryEditor {
    fn execute_query(&mut self, cx: &mut Context<Self>) {
        let sql = self.editor_content.clone();
        let connection_id = self.connection_id;

        // Spawn async task
        cx.spawn(async move |this, cx| {
            let state = cx.global::<TuskState>();

            // Execute query on background executor
            let result = state.queries
                .execute(&connection_id, &sql)
                .await;

            // Update UI on main thread
            this.update(&cx, |this, cx| {
                match result {
                    Ok(query_result) => {
                        this.handle_query_result(query_result, cx);
                    }
                    Err(e) => {
                        this.handle_query_error(e, cx);
                    }
                }
            })?;

            Ok(())
        }).detach();
    }
}
```

### 3.3 Streaming Results Pattern

For large result sets, use channel-based streaming:

```rust
// crates/tusk_core/src/services/query.rs

use tokio::sync::mpsc;
use futures::StreamExt;

pub struct QueryService {
    executor: BackgroundExecutor,
}

/// Events emitted during query execution
pub enum QueryEvent {
    Columns(Vec<ColumnMeta>),
    Rows(Vec<Row>, u32),  // rows, batch_num
    Complete(QueryComplete),
    Error(TuskError),
}

pub struct StreamingQuery {
    pub id: uuid::Uuid,
    pub receiver: mpsc::Receiver<QueryEvent>,
    pub cancel_token: CancellationToken,
}

impl QueryService {
    pub async fn execute_streaming(
        &self,
        connection_id: &uuid::Uuid,
        sql: &str,
        batch_size: usize,
    ) -> Result<StreamingQuery, TuskError> {
        let (tx, rx) = mpsc::channel(32);
        let cancel_token = CancellationToken::new();
        let query_id = uuid::Uuid::new_v4();

        let pool = self.get_pool(connection_id)?;
        let sql = sql.to_string();
        let token = cancel_token.clone();

        // Spawn streaming task
        self.executor.spawn(async move {
            let result = Self::stream_query_inner(
                pool,
                &sql,
                batch_size,
                tx.clone(),
                token,
            ).await;

            if let Err(e) = result {
                let _ = tx.send(QueryEvent::Error(e)).await;
            }
        }).detach();

        Ok(StreamingQuery {
            id: query_id,
            receiver: rx,
            cancel_token,
        })
    }

    async fn stream_query_inner(
        pool: Arc<ConnectionPool>,
        sql: &str,
        batch_size: usize,
        tx: mpsc::Sender<QueryEvent>,
        cancel_token: CancellationToken,
    ) -> Result<(), TuskError> {
        let client = pool.get().await?;
        let start = std::time::Instant::now();

        // Execute query
        let row_stream = client.query_raw(sql, &[] as &[&str]).await?;
        tokio::pin!(row_stream);

        // Send column metadata
        if let Some(Ok(first_row)) = row_stream.as_mut().peek().await {
            let columns = extract_column_metadata(first_row);
            tx.send(QueryEvent::Columns(columns)).await
                .map_err(|_| TuskError::ChannelClosed)?;
        }

        let mut batch: Vec<Row> = Vec::with_capacity(batch_size);
        let mut batch_num = 0u32;
        let mut total_rows = 0u64;

        while let Some(row_result) = row_stream.next().await {
            // Check cancellation
            if cancel_token.is_cancelled() {
                return Err(TuskError::QueryCancelled);
            }

            let row = row_result?;
            batch.push(convert_row(row)?);
            total_rows += 1;

            if batch.len() >= batch_size {
                tx.send(QueryEvent::Rows(
                    std::mem::take(&mut batch),
                    batch_num,
                )).await.map_err(|_| TuskError::ChannelClosed)?;

                batch_num += 1;
                batch = Vec::with_capacity(batch_size);
            }
        }

        // Send final batch
        if !batch.is_empty() {
            tx.send(QueryEvent::Rows(batch, batch_num)).await
                .map_err(|_| TuskError::ChannelClosed)?;
        }

        // Send completion
        tx.send(QueryEvent::Complete(QueryComplete {
            total_rows,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })).await.map_err(|_| TuskError::ChannelClosed)?;

        Ok(())
    }
}
```

### 3.4 Consuming Streaming Results in UI

```rust
impl ResultsPanel {
    fn execute_streaming_query(&mut self, sql: String, cx: &mut Context<Self>) {
        let connection_id = self.connection_id;

        cx.spawn(async move |this, cx| {
            let state = cx.global::<TuskState>();

            // Start streaming query
            let mut streaming = state.queries
                .execute_streaming(&connection_id, &sql, 1000)
                .await?;

            // Register for cancellation
            let query_id = streaming.id;
            state.register_query(QueryHandle {
                id: query_id,
                cancel_token: streaming.cancel_token.clone(),
            });

            // Process events
            while let Some(event) = streaming.receiver.recv().await {
                let should_continue = this.update(&cx, |this, cx| {
                    match event {
                        QueryEvent::Columns(columns) => {
                            this.set_columns(columns, cx);
                            true
                        }
                        QueryEvent::Rows(rows, batch_num) => {
                            this.append_rows(rows, batch_num, cx);
                            true
                        }
                        QueryEvent::Complete(info) => {
                            this.mark_complete(info, cx);
                            false
                        }
                        QueryEvent::Error(e) => {
                            this.show_error(e, cx);
                            false
                        }
                    }
                })?;

                if !should_continue {
                    break;
                }
            }

            // Unregister query
            state.unregister_query(&query_id);

            Ok(())
        }).detach();
    }

    fn cancel_query(&mut self, cx: &mut Context<Self>) {
        if let Some(query_id) = self.running_query_id.take() {
            let state = cx.global::<TuskState>();
            state.cancel_query(&query_id);
            self.set_status(QueryStatus::Cancelled, cx);
        }
    }
}
```

---

## 4. Entity-Based State Updates

### 4.1 Subscription Pattern

Components can subscribe to state changes:

```rust
// crates/tusk_core/src/state.rs

impl TuskState {
    /// Subscribe to connection status changes
    pub fn observe_connection_status(
        &self,
        connection_id: uuid::Uuid,
        cx: &mut App,
        callback: impl Fn(ConnectionStatus, &mut App) + 'static,
    ) -> Subscription {
        self.connections.observe_status(connection_id, cx, callback)
    }

    /// Subscribe to schema refresh
    pub fn observe_schema(
        &self,
        connection_id: uuid::Uuid,
        cx: &mut App,
        callback: impl Fn(&Schema, &mut App) + 'static,
    ) -> Subscription {
        self.schema.observe(connection_id, cx, callback)
    }
}

// Usage in component
impl SchemaBrowser {
    fn new(connection_id: uuid::Uuid, cx: &mut Context<Self>) -> Self {
        let state = cx.global::<TuskState>();

        // Subscribe to schema changes
        let _subscription = state.observe_schema(
            connection_id,
            cx,
            |schema, cx| {
                // This closure runs when schema refreshes
                cx.notify();
            },
        );

        Self {
            connection_id,
            schema: None,
            _subscription,
        }
    }
}
```

### 4.2 Entity Update Pattern

```rust
// When services need to notify UI of changes
impl ConnectionService {
    pub async fn connect(
        &self,
        config: &ConnectionConfig,
        cx: &AsyncApp,
    ) -> Result<(), TuskError> {
        // Create pool
        let pool = self.create_pool(config).await?;

        // Store connection
        self.pools.write().insert(config.id, pool);

        // Notify observers
        self.status_changed.write().insert(config.id, ConnectionStatus::Connected);

        // Trigger UI update
        cx.update(|cx| {
            cx.notify();
        })?;

        Ok(())
    }
}
```

---

## 5. Error Handling Across Boundaries

### 5.1 Error Propagation

```rust
// crates/tusk_core/src/error.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TuskError {
    #[error("Connection failed: {message}")]
    ConnectionFailed {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Query execution failed: {message}")]
    QueryFailed {
        message: String,
        code: Option<String>,
        position: Option<u32>,
        hint: Option<String>,
    },

    #[error("Query cancelled")]
    QueryCancelled,

    #[error("Connection not found: {id}")]
    ConnectionNotFound { id: String },

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Storage error: {0}")]
    Storage(#[from] rusqlite::Error),

    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),
}

impl TuskError {
    /// Convert to user-friendly error info for display
    pub fn to_error_info(&self) -> ErrorInfo {
        match self {
            TuskError::ConnectionFailed { message, .. } => ErrorInfo {
                title: "Connection Failed".into(),
                message: message.clone(),
                detail: None,
                hint: Some("Check your connection settings and ensure the database is running.".into()),
                recoverable: true,
            },
            TuskError::QueryFailed { message, hint, position, .. } => ErrorInfo {
                title: "Query Error".into(),
                message: message.clone(),
                detail: position.map(|p| format!("Error at position {}", p)),
                hint: hint.clone(),
                recoverable: true,
            },
            TuskError::QueryCancelled => ErrorInfo {
                title: "Query Cancelled".into(),
                message: "The query was cancelled by user request.".into(),
                detail: None,
                hint: None,
                recoverable: true,
            },
            _ => ErrorInfo {
                title: "Error".into(),
                message: self.to_string(),
                detail: None,
                hint: None,
                recoverable: false,
            },
        }
    }
}

/// User-facing error information
#[derive(Debug, Clone)]
pub struct ErrorInfo {
    pub title: SharedString,
    pub message: String,
    pub detail: Option<String>,
    pub hint: Option<SharedString>,
    pub recoverable: bool,
}
```

### 5.2 Error Display in UI

```rust
impl ResultsPanel {
    fn show_error(&mut self, error: TuskError, cx: &mut Context<Self>) {
        let info = error.to_error_info();

        self.error_state = Some(ErrorState {
            info: info.clone(),
            dismissed: false,
        });

        // Show toast notification for non-blocking errors
        if info.recoverable {
            cx.emit(ToastEvent::Error(info.message.clone()));
        }

        cx.notify();
    }

    fn render_error(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        let error = self.error_state.as_ref()?;
        let theme = cx.global::<Theme>();

        Some(
            div()
                .p_4()
                .rounded_md()
                .bg(theme.colors.error.opacity(0.1))
                .border_1()
                .border_color(theme.colors.error)
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_start()
                        .gap_3()
                        .child(
                            Icon::new(IconName::Error)
                                .color(theme.colors.error)
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(error.info.title.clone())
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .child(error.info.message.clone())
                                )
                                .when_some(error.info.hint.clone(), |this, hint| {
                                    this.child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.colors.text_muted)
                                            .child(format!("Hint: {}", hint))
                                    )
                                })
                        )
                )
        )
    }
}
```

---

## 6. Connection Management Integration

### 6.1 Connection Service Interface

```rust
// crates/tusk_core/src/services/connection.rs

use deadpool_postgres::{Pool, Config, Runtime};
use tokio_postgres::NoTls;

pub struct ConnectionService {
    executor: BackgroundExecutor,
    pools: RwLock<HashMap<uuid::Uuid, Pool>>,
    status: RwLock<HashMap<uuid::Uuid, ConnectionStatus>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

impl ConnectionService {
    pub fn new(executor: BackgroundExecutor) -> Self {
        Self {
            executor,
            pools: RwLock::new(HashMap::new()),
            status: RwLock::new(HashMap::new()),
        }
    }

    pub fn status(&self, id: &uuid::Uuid) -> ConnectionStatus {
        self.status.read().get(id).copied().unwrap_or(ConnectionStatus::Disconnected)
    }

    pub fn is_connected(&self, id: &uuid::Uuid) -> bool {
        self.status(id) == ConnectionStatus::Connected
    }

    pub async fn connect(&self, config: &ConnectionConfig) -> Result<(), TuskError> {
        // Update status
        self.status.write().insert(config.id, ConnectionStatus::Connecting);

        // Build connection string
        let conn_str = config.to_connection_string()?;

        // Create pool config
        let mut pool_config = Config::new();
        pool_config.host = Some(config.host.clone());
        pool_config.port = Some(config.port);
        pool_config.dbname = Some(config.database.clone());
        pool_config.user = Some(config.username.clone());

        // Get password from keyring if needed
        if config.password_in_keyring {
            // Password retrieved separately
        }

        // Create pool
        let pool = pool_config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| TuskError::ConnectionFailed {
                message: e.to_string(),
                source: Some(Box::new(e)),
            })?;

        // Test connection
        let client = pool.get().await.map_err(|e| TuskError::ConnectionFailed {
            message: format!("Failed to get connection from pool: {}", e),
            source: Some(Box::new(e)),
        })?;

        client.query_one("SELECT 1", &[]).await.map_err(|e| TuskError::ConnectionFailed {
            message: format!("Connection test failed: {}", e),
            source: Some(Box::new(e)),
        })?;

        // Store pool
        self.pools.write().insert(config.id, pool);
        self.status.write().insert(config.id, ConnectionStatus::Connected);

        Ok(())
    }

    pub fn disconnect(&self, id: &uuid::Uuid) -> Result<(), TuskError> {
        self.pools.write().remove(id);
        self.status.write().insert(*id, ConnectionStatus::Disconnected);
        Ok(())
    }

    pub fn get_pool(&self, id: &uuid::Uuid) -> Result<Pool, TuskError> {
        self.pools.read().get(id).cloned().ok_or(TuskError::ConnectionNotFound {
            id: id.to_string(),
        })
    }
}
```

### 6.2 UI Integration for Connection

```rust
impl ConnectionDialog {
    fn test_connection(&mut self, cx: &mut Context<Self>) {
        let config = self.build_config();

        self.testing = true;
        self.test_result = None;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let state = cx.global::<TuskState>();

            let result = state.connections
                .test_connection(&config)
                .await;

            this.update(&cx, |this, cx| {
                this.testing = false;
                this.test_result = Some(result);
                cx.notify();
            })?;

            Ok(())
        }).detach();
    }

    fn connect(&mut self, cx: &mut Context<Self>) {
        let config = self.build_config();

        cx.spawn(async move |this, cx| {
            let state = cx.global::<TuskState>();

            // Save connection config
            state.storage.save_connection(&config).await?;

            // Connect
            state.connections.connect(&config).await?;

            // Set as active
            state.set_active_connection(Some(config.id));

            // Close dialog
            this.update(&cx, |this, cx| {
                cx.emit(DialogEvent::Close);
            })?;

            Ok(())
        }).detach();
    }
}
```

---

## 7. Schema Service Integration

### 7.1 Schema Loading with Cache

```rust
// crates/tusk_core/src/services/schema.rs

pub struct SchemaService {
    executor: BackgroundExecutor,
    cache: RwLock<HashMap<uuid::Uuid, CachedSchema>>,
}

struct CachedSchema {
    schema: Schema,
    loaded_at: std::time::Instant,
    ttl: std::time::Duration,
}

impl SchemaService {
    pub fn get_cached(&self, connection_id: &uuid::Uuid) -> Option<Schema> {
        let cache = self.cache.read();
        cache.get(connection_id).and_then(|cached| {
            if cached.loaded_at.elapsed() < cached.ttl {
                Some(cached.schema.clone())
            } else {
                None
            }
        })
    }

    pub async fn load_schema(
        &self,
        pool: &Pool,
        connection_id: uuid::Uuid,
    ) -> Result<Schema, TuskError> {
        // Check cache first
        if let Some(schema) = self.get_cached(&connection_id) {
            return Ok(schema);
        }

        // Load from database
        let client = pool.get().await?;

        // Load tables
        let tables = self.load_tables(&client).await?;

        // Load views
        let views = self.load_views(&client).await?;

        // Load functions
        let functions = self.load_functions(&client).await?;

        // Build schema
        let schema = Schema {
            tables,
            views,
            functions,
        };

        // Cache
        self.cache.write().insert(connection_id, CachedSchema {
            schema: schema.clone(),
            loaded_at: std::time::Instant::now(),
            ttl: std::time::Duration::from_secs(300), // 5 minute TTL
        });

        Ok(schema)
    }

    pub fn invalidate_cache(&self, connection_id: &uuid::Uuid) {
        self.cache.write().remove(connection_id);
    }
}
```

---

## 8. Task Spawning Patterns

### 8.1 Background Tasks

```rust
impl QueryEditor {
    /// Execute query in background, update UI when complete
    fn execute(&mut self, cx: &mut Context<Self>) {
        let sql = self.get_sql();
        let connection_id = self.connection_id;

        // Show loading state
        self.executing = true;
        cx.notify();

        // Spawn background task
        cx.spawn(async move |this, cx| {
            let state = cx.global::<TuskState>();
            let pool = state.connections.get_pool(&connection_id)?;

            // Execute query
            let result = state.queries.execute(&pool, &sql).await;

            // Update UI
            this.update(&cx, |this, cx| {
                this.executing = false;
                match result {
                    Ok(data) => {
                        this.last_result = Some(data);
                        cx.emit(QueryEditorEvent::ResultsReady);
                    }
                    Err(e) => {
                        this.last_error = Some(e);
                    }
                }
                cx.notify();
            })?;

            Ok(())
        }).detach();
    }
}
```

### 8.2 Cancellable Tasks

```rust
impl QueryEditor {
    fn execute_with_cancellation(&mut self, cx: &mut Context<Self>) {
        let sql = self.get_sql();
        let connection_id = self.connection_id;
        let cancel_token = CancellationToken::new();

        // Store token for cancellation
        self.cancel_token = Some(cancel_token.clone());
        self.executing = true;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let state = cx.global::<TuskState>();
            let pool = state.connections.get_pool(&connection_id)?;

            // Execute with cancellation support
            let result = tokio::select! {
                result = state.queries.execute(&pool, &sql) => result,
                _ = cancel_token.cancelled() => Err(TuskError::QueryCancelled),
            };

            this.update(&cx, |this, cx| {
                this.executing = false;
                this.cancel_token = None;

                match result {
                    Ok(data) => this.last_result = Some(data),
                    Err(TuskError::QueryCancelled) => {
                        // User cancelled, don't show error
                    }
                    Err(e) => this.last_error = Some(e),
                }
                cx.notify();
            })?;

            Ok(())
        }).detach();
    }

    fn cancel_execution(&mut self, cx: &mut Context<Self>) {
        if let Some(token) = self.cancel_token.take() {
            token.cancel();
        }
    }
}
```

---

## 9. Performance Considerations

### 9.1 Avoiding UI Thread Blocking

```rust
// BAD: Blocking the UI thread
impl BadComponent {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // DON'T DO THIS - blocks UI
        let data = futures::executor::block_on(async {
            load_data().await
        });

        div().child(format!("{:?}", data))
    }
}

// GOOD: Async with state
impl GoodComponent {
    fn new(cx: &mut Context<Self>) -> Self {
        let this = Self {
            data: None,
            loading: true,
        };

        // Spawn async load
        cx.spawn(async move |this, cx| {
            let data = load_data().await?;

            this.update(&cx, |this, cx| {
                this.data = Some(data);
                this.loading = false;
                cx.notify();
            })?;

            Ok(())
        }).detach();

        this
    }

    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.loading {
            Spinner::new()
        } else if let Some(data) = &self.data {
            div().child(format!("{:?}", data))
        } else {
            div().child("No data")
        }.into_any_element()
    }
}
```

### 9.2 Batching Updates

```rust
impl ResultsGrid {
    fn append_rows(&mut self, rows: Vec<Row>, cx: &mut Context<Self>) {
        // Batch row additions
        self.rows.extend(rows);

        // Only notify once per batch, not per row
        cx.notify();
    }
}
```

---

## 10. Acceptance Criteria

### 10.1 Service Access

- [ ] TuskState registered as Global and accessible from all components
- [ ] Services accessible via `cx.global::<TuskState>()`
- [ ] Connection service manages pool lifecycle
- [ ] Query service executes queries with results

### 10.2 Async Operations

- [ ] Background tasks execute without blocking UI
- [ ] Tasks can be cancelled via CancellationToken
- [ ] UI updates correctly when tasks complete
- [ ] Errors propagate and display correctly

### 10.3 Streaming

- [ ] Streaming queries send batched results
- [ ] Batches update UI incrementally
- [ ] Stream can be cancelled mid-execution
- [ ] Completion notification sent when done

### 10.4 Error Handling

- [ ] Errors convert to ErrorInfo for display
- [ ] Recoverable errors show toast notifications
- [ ] Fatal errors show error panel
- [ ] Error context preserved for debugging

---

## 11. Dependencies

```toml
# crates/tusk_core/Cargo.toml
[dependencies]
gpui = { git = "https://github.com/zed-industries/zed" }
tokio = { version = "1", features = ["full"] }
tokio-util = { version = "0.7", features = ["rt"] }
futures = "0.3"
parking_lot = "0.12"
uuid = { version = "1.6", features = ["v4"] }
thiserror = "1.0"
deadpool-postgres = "0.12"
tokio-postgres = "0.7"
```

---

## 12. Testing

### 12.1 Service Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    async fn test_connection_service(cx: &mut TestAppContext) {
        let executor = cx.background_executor().clone();
        let service = ConnectionService::new(executor);

        // Test connection (requires test database)
        let config = ConnectionConfig {
            id: uuid::Uuid::new_v4(),
            host: "localhost".to_string(),
            port: 5432,
            database: "test".to_string(),
            username: "test".to_string(),
            ..Default::default()
        };

        let result = service.connect(&config).await;
        assert!(result.is_ok());
        assert_eq!(service.status(&config.id), ConnectionStatus::Connected);

        service.disconnect(&config.id).unwrap();
        assert_eq!(service.status(&config.id), ConnectionStatus::Disconnected);
    }

    #[gpui::test]
    async fn test_query_cancellation(cx: &mut TestAppContext) {
        let executor = cx.background_executor().clone();
        let service = QueryService::new(executor);

        // Start long-running query
        let pool = create_test_pool().await;
        let mut streaming = service
            .execute_streaming(&pool, "SELECT pg_sleep(10)", 100)
            .await
            .unwrap();

        // Cancel immediately
        streaming.cancel_token.cancel();

        // Should receive error event
        let event = streaming.receiver.recv().await.unwrap();
        assert!(matches!(event, QueryEvent::Error(TuskError::QueryCancelled)));
    }
}
```

---

## 13. Migration Notes

This document replaces the Tauri IPC layer. Key changes:

| Tauri IPC                          | GPUI Service Integration           |
| ---------------------------------- | ---------------------------------- |
| `#[tauri::command]` macros         | Direct service methods             |
| `invoke()` JavaScript calls        | `cx.global::<TuskState>()` access  |
| JSON serialization                 | Native Rust types                  |
| `app.emit()` events                | `mpsc::channel` streams            |
| TypeScript service wrappers        | Not needed (pure Rust)             |
| `UnlistenFn` cleanup               | Rust RAII via `Subscription`       |

The core concepts remain:
- Async execution for database operations
- Streaming for large result sets
- Cancellation support for long queries
- Error conversion for UI display

But implementation is simpler without the process boundary.
