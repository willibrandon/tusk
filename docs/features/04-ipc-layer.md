# Feature 04: IPC Layer

## Overview

Design and implement the Tauri IPC (Inter-Process Communication) layer that connects the Svelte frontend to the Rust backend. This includes command definitions, event streaming for large datasets, and TypeScript type generation.

## Goals

- Define all Tauri commands with proper types
- Implement event streaming for large result sets
- Create TypeScript wrappers for type-safe IPC
- Handle errors consistently across the boundary
- Support query cancellation

## Technical Specification

### 1. Command Categories

```
Commands:
├── Connection
│   ├── connect
│   ├── disconnect
│   ├── test_connection
│   ├── list_connections
│   ├── save_connection
│   ├── delete_connection
│   └── get_connection_status
├── Query
│   ├── execute_query
│   ├── execute_query_stream
│   ├── cancel_query
│   └── explain_query
├── Schema
│   ├── get_schema
│   ├── refresh_schema
│   ├── get_table_columns
│   ├── get_table_indexes
│   ├── get_table_constraints
│   └── generate_ddl
├── Admin
│   ├── get_activity
│   ├── get_server_stats
│   ├── get_table_stats
│   ├── get_index_stats
│   ├── get_locks
│   ├── kill_query
│   ├── vacuum_table
│   └── reindex
├── Storage
│   ├── get_query_history
│   ├── save_query
│   ├── delete_query
│   ├── get_saved_queries
│   ├── get_settings
│   └── save_settings
└── Credentials
    ├── store_password
    ├── get_password
    └── delete_password
```

### 2. Command Implementation Pattern

```rust
// commands/connection.rs
use tauri::{command, State};
use uuid::Uuid;
use crate::state::AppState;
use crate::error::{Result, TuskError};
use crate::models::connection::{ConnectionConfig, ConnectionStatus};
use crate::services::connection::ConnectionPool;

#[command]
pub async fn connect(
    state: State<'_, AppState>,
    config: ConnectionConfig,
) -> Result<ConnectionStatus> {
    tracing::info!("Connecting to: {}:{}", config.host, config.port);

    // Create connection pool
    let pool = ConnectionPool::new(config.clone()).await?;

    // Test the connection
    let client = pool.get_client().await?;
    let _row = client
        .query_one("SELECT 1", &[])
        .await
        .map_err(|e| TuskError::ConnectionFailed {
            message: format!("Connection test failed: {}", e),
            source: Some(Box::new(e)),
        })?;

    // Store in state
    state.add_connection(config.id, pool).await;

    // Update storage with last connected time
    state.storage.update_connection_last_used(&config.id).await?;

    Ok(ConnectionStatus::Connected)
}

#[command]
pub async fn disconnect(
    state: State<'_, AppState>,
    connection_id: Uuid,
) -> Result<()> {
    tracing::info!("Disconnecting: {}", connection_id);

    state.remove_connection(&connection_id).await
        .ok_or(TuskError::ConnectionNotFound { id: connection_id.to_string() })?;

    Ok(())
}

#[command]
pub async fn test_connection(config: ConnectionConfig) -> Result<String> {
    tracing::info!("Testing connection to: {}:{}", config.host, config.port);

    // Create temporary pool
    let pool = ConnectionPool::new(config).await?;
    let client = pool.get_client().await?;

    // Get server version
    let row = client.query_one("SELECT version()", &[]).await?;
    let version: String = row.get(0);

    Ok(version)
}

#[command]
pub async fn list_connections(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectionConfig>> {
    state.storage.get_all_connections().await
}

#[command]
pub async fn save_connection(
    state: State<'_, AppState>,
    config: ConnectionConfig,
) -> Result<()> {
    state.storage.save_connection(&config).await
}

#[command]
pub async fn delete_connection(
    state: State<'_, AppState>,
    connection_id: Uuid,
) -> Result<()> {
    // Disconnect if connected
    state.remove_connection(&connection_id).await;

    // Delete from storage
    state.storage.delete_connection(&connection_id).await
}

#[command]
pub async fn get_connection_status(
    state: State<'_, AppState>,
    connection_id: Uuid,
) -> Result<ConnectionStatus> {
    if state.get_connection(&connection_id).await.is_some() {
        Ok(ConnectionStatus::Connected)
    } else {
        Ok(ConnectionStatus::Disconnected)
    }
}
```

### 3. Query Execution with Streaming

```rust
// commands/query.rs
use tauri::{command, AppHandle, State, Emitter};
use uuid::Uuid;
use tokio_postgres::types::ToSql;
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::error::{Result, TuskError};
use crate::models::query::{QueryResult, ColumnMeta, Value};

#[derive(Debug, Serialize)]
pub struct RowBatch {
    pub query_id: Uuid,
    pub batch_num: u32,
    pub rows: Vec<Vec<Value>>,
}

#[derive(Debug, Serialize)]
pub struct QueryComplete {
    pub query_id: Uuid,
    pub total_rows: u64,
    pub elapsed_ms: u64,
    pub command: String,
}

#[derive(Debug, Serialize)]
pub struct QueryError {
    pub query_id: Uuid,
    pub error: crate::error::ErrorResponse,
}

/// Execute a query and return all results at once (for small result sets)
#[command]
pub async fn execute_query(
    state: State<'_, AppState>,
    connection_id: Uuid,
    sql: String,
    params: Vec<serde_json::Value>,
) -> Result<QueryResult> {
    let pool = state.get_connection(&connection_id).await
        .ok_or(TuskError::ConnectionNotFound { id: connection_id.to_string() })?;

    let client = pool.get_client().await?;

    let start = std::time::Instant::now();

    // Convert params to postgres types
    let params: Vec<Box<dyn ToSql + Sync + Send>> = params
        .into_iter()
        .map(|v| json_to_sql(v))
        .collect();

    let params_ref: Vec<&(dyn ToSql + Sync)> = params
        .iter()
        .map(|p| p.as_ref() as &(dyn ToSql + Sync))
        .collect();

    let rows = client.query(&sql, &params_ref).await?;

    let elapsed_ms = start.elapsed().as_millis() as u64;

    // Convert to QueryResult
    let result = rows_to_query_result(rows, elapsed_ms)?;

    // Save to history
    state.storage.add_query_history(
        &connection_id,
        &sql,
        elapsed_ms,
        result.rows.as_ref().map(|r| r.len() as i64),
        None,
    ).await?;

    Ok(result)
}

/// Execute a query with streaming results (for large result sets)
#[command]
pub async fn execute_query_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    connection_id: Uuid,
    sql: String,
    params: Vec<serde_json::Value>,
    batch_size: Option<u32>,
) -> Result<QueryStarted> {
    let pool = state.get_connection(&connection_id).await
        .ok_or(TuskError::ConnectionNotFound { id: connection_id.to_string() })?;

    let query_id = Uuid::new_v4();
    let batch_size = batch_size.unwrap_or(1000);

    // Register for cancellation
    let cancel_rx = state.register_query(query_id).await;

    // Spawn streaming task
    let app_handle = app.clone();
    let state_clone = state.inner().clone();

    tauri::async_runtime::spawn(async move {
        let result = stream_query(
            app_handle.clone(),
            pool,
            query_id,
            sql.clone(),
            params,
            batch_size,
            cancel_rx,
        ).await;

        // Unregister query
        state_clone.unregister_query(&query_id).await;

        // Handle errors
        if let Err(e) = result {
            let _ = app_handle.emit("query:error", QueryError {
                query_id,
                error: e.into(),
            });
        }

        // Save to history
        let _ = state_clone.storage.add_query_history(
            &connection_id,
            &sql,
            0, // TODO: track elapsed time
            None,
            result.err().map(|e| e.to_string()),
        ).await;
    });

    Ok(QueryStarted { query_id })
}

async fn stream_query(
    app: AppHandle,
    pool: Arc<ConnectionPool>,
    query_id: Uuid,
    sql: String,
    params: Vec<serde_json::Value>,
    batch_size: u32,
    mut cancel_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<()> {
    let client = pool.get_client().await?;
    let start = std::time::Instant::now();

    // Convert params
    let params: Vec<Box<dyn ToSql + Sync + Send>> = params
        .into_iter()
        .map(|v| json_to_sql(v))
        .collect();

    let params_ref: Vec<&(dyn ToSql + Sync)> = params
        .iter()
        .map(|p| p.as_ref() as &(dyn ToSql + Sync))
        .collect();

    // Execute query
    let row_stream = client.query_raw(&sql, params_ref).await?;
    tokio::pin!(row_stream);

    // Emit column metadata first
    let columns = get_column_metadata(&row_stream);
    app.emit("query:columns", QueryColumns { query_id, columns })?;

    let mut batch: Vec<Vec<Value>> = Vec::with_capacity(batch_size as usize);
    let mut batch_num = 0u32;
    let mut total_rows = 0u64;

    use futures::StreamExt;

    while let Some(row_result) = row_stream.next().await {
        // Check for cancellation
        if *cancel_rx.borrow() {
            return Err(TuskError::QueryCancelled);
        }

        let row = row_result?;
        let values = row_to_values(&row)?;
        batch.push(values);
        total_rows += 1;

        // Emit batch when full
        if batch.len() >= batch_size as usize {
            app.emit("query:rows", RowBatch {
                query_id,
                batch_num,
                rows: std::mem::take(&mut batch),
            })?;
            batch_num += 1;
            batch = Vec::with_capacity(batch_size as usize);
        }
    }

    // Emit final batch
    if !batch.is_empty() {
        app.emit("query:rows", RowBatch {
            query_id,
            batch_num,
            rows: batch,
        })?;
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;

    // Emit completion
    app.emit("query:complete", QueryComplete {
        query_id,
        total_rows,
        elapsed_ms,
        command: extract_command(&sql),
    })?;

    Ok(())
}

#[command]
pub async fn cancel_query(
    state: State<'_, AppState>,
    query_id: Uuid,
) -> Result<bool> {
    Ok(state.cancel_query(&query_id).await)
}

#[derive(Debug, Serialize)]
pub struct QueryStarted {
    pub query_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct QueryColumns {
    pub query_id: Uuid,
    pub columns: Vec<ColumnMeta>,
}

// Helper functions
fn json_to_sql(value: serde_json::Value) -> Box<dyn ToSql + Sync + Send> {
    match value {
        serde_json::Value::Null => Box::new(Option::<String>::None),
        serde_json::Value::Bool(b) => Box::new(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Box::new(i)
            } else if let Some(f) = n.as_f64() {
                Box::new(f)
            } else {
                Box::new(n.to_string())
            }
        }
        serde_json::Value::String(s) => Box::new(s),
        _ => Box::new(value.to_string()),
    }
}

fn row_to_values(row: &tokio_postgres::Row) -> Result<Vec<Value>> {
    // Implementation converts each column to Value enum
    todo!()
}

fn extract_command(sql: &str) -> String {
    sql.trim()
        .split_whitespace()
        .next()
        .unwrap_or("UNKNOWN")
        .to_uppercase()
}
```

### 4. TypeScript IPC Service

```typescript
// services/ipc.ts
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ConnectionConfig, ConnectionStatus } from '$types/connection';
import type { QueryResult, ColumnMeta, Value } from '$types/query';

// Error type from backend
export interface TuskError {
	error_type: string;
	message: string;
	detail?: string;
	hint?: string;
	position?: number;
	code?: string;
}

// Generic invoke wrapper with error handling
async function ipc<T>(command: string, args?: Record<string, unknown>): Promise<T> {
	try {
		return await invoke<T>(command, args);
	} catch (error) {
		// Error is already TuskError from backend
		throw error as TuskError;
	}
}

// Connection commands
export const connectionCommands = {
	connect: (config: ConnectionConfig) => ipc<ConnectionStatus>('connect', { config }),

	disconnect: (connectionId: string) => ipc<void>('disconnect', { connectionId }),

	testConnection: (config: ConnectionConfig) => ipc<string>('test_connection', { config }),

	listConnections: () => ipc<ConnectionConfig[]>('list_connections'),

	saveConnection: (config: ConnectionConfig) => ipc<void>('save_connection', { config }),

	deleteConnection: (connectionId: string) => ipc<void>('delete_connection', { connectionId }),

	getConnectionStatus: (connectionId: string) =>
		ipc<ConnectionStatus>('get_connection_status', { connectionId })
};

// Query commands
export interface QueryStarted {
	query_id: string;
}

export interface RowBatch {
	query_id: string;
	batch_num: number;
	rows: Value[][];
}

export interface QueryComplete {
	query_id: string;
	total_rows: number;
	elapsed_ms: number;
	command: string;
}

export interface QueryError {
	query_id: string;
	error: TuskError;
}

export interface QueryColumns {
	query_id: string;
	columns: ColumnMeta[];
}

export const queryCommands = {
	executeQuery: (connectionId: string, sql: string, params: unknown[] = []) =>
		ipc<QueryResult>('execute_query', { connectionId, sql, params }),

	executeQueryStream: (
		connectionId: string,
		sql: string,
		params: unknown[] = [],
		batchSize?: number
	) =>
		ipc<QueryStarted>('execute_query_stream', {
			connectionId,
			sql,
			params,
			batchSize
		}),

	cancelQuery: (queryId: string) => ipc<boolean>('cancel_query', { queryId }),

	explainQuery: (connectionId: string, sql: string, options: ExplainOptions) =>
		ipc<QueryPlan>('explain_query', { connectionId, sql, options })
};

// Query event listeners
export function onQueryColumns(callback: (data: QueryColumns) => void): Promise<UnlistenFn> {
	return listen<QueryColumns>('query:columns', (event) => callback(event.payload));
}

export function onQueryRows(callback: (data: RowBatch) => void): Promise<UnlistenFn> {
	return listen<RowBatch>('query:rows', (event) => callback(event.payload));
}

export function onQueryComplete(callback: (data: QueryComplete) => void): Promise<UnlistenFn> {
	return listen<QueryComplete>('query:complete', (event) => callback(event.payload));
}

export function onQueryError(callback: (data: QueryError) => void): Promise<UnlistenFn> {
	return listen<QueryError>('query:error', (event) => callback(event.payload));
}

// Schema commands
export const schemaCommands = {
	getSchema: (connectionId: string) => ipc<Schema>('get_schema', { connectionId }),

	refreshSchema: (connectionId: string) => ipc<Schema>('refresh_schema', { connectionId }),

	getTableColumns: (connectionId: string, schema: string, table: string) =>
		ipc<Column[]>('get_table_columns', { connectionId, schema, table }),

	getTableIndexes: (connectionId: string, schema: string, table: string) =>
		ipc<Index[]>('get_table_indexes', { connectionId, schema, table }),

	generateDdl: (connectionId: string, objectType: string, schema: string, name: string) =>
		ipc<string>('generate_ddl', { connectionId, objectType, schema, name })
};

// Admin commands
export const adminCommands = {
	getActivity: (connectionId: string) => ipc<Activity[]>('get_activity', { connectionId }),

	getServerStats: (connectionId: string) => ipc<ServerStats>('get_server_stats', { connectionId }),

	getTableStats: (connectionId: string, schema?: string) =>
		ipc<TableStats[]>('get_table_stats', { connectionId, schema }),

	getIndexStats: (connectionId: string, schema?: string) =>
		ipc<IndexStats[]>('get_index_stats', { connectionId, schema }),

	getLocks: (connectionId: string) => ipc<Lock[]>('get_locks', { connectionId }),

	killQuery: (connectionId: string, pid: number) =>
		ipc<boolean>('kill_query', { connectionId, pid }),

	vacuumTable: (connectionId: string, schema: string, table: string, options: VacuumOptions) =>
		ipc<void>('vacuum_table', { connectionId, schema, table, options }),

	reindex: (connectionId: string, target: ReindexTarget) =>
		ipc<void>('reindex', { connectionId, target })
};

// Storage commands
export const storageCommands = {
	getQueryHistory: (connectionId: string, limit?: number) =>
		ipc<QueryHistoryItem[]>('get_query_history', { connectionId, limit }),

	saveQuery: (query: SavedQuery) => ipc<void>('save_query', { query }),

	deleteQuery: (queryId: string) => ipc<void>('delete_query', { queryId }),

	getSavedQueries: (connectionId?: string) =>
		ipc<SavedQuery[]>('get_saved_queries', { connectionId }),

	getSettings: () => ipc<Settings>('get_settings'),

	saveSettings: (settings: Settings) => ipc<void>('save_settings', { settings })
};

// Credential commands
export const credentialCommands = {
	storePassword: (connectionId: string, password: string) =>
		ipc<void>('store_password', { connectionId, password }),

	getPassword: (connectionId: string) => ipc<string | null>('get_password', { connectionId }),

	deletePassword: (connectionId: string) => ipc<void>('delete_password', { connectionId })
};
```

### 5. Streaming Query Service

```typescript
// services/query.ts
import {
	queryCommands,
	onQueryColumns,
	onQueryRows,
	onQueryComplete,
	onQueryError,
	type QueryColumns,
	type RowBatch,
	type QueryComplete,
	type QueryError
} from './ipc';
import type { ColumnMeta, Value } from '$types/query';

export interface StreamingQueryCallbacks {
	onColumns?: (columns: ColumnMeta[]) => void;
	onRows?: (rows: Value[][], batchNum: number) => void;
	onComplete?: (totalRows: number, elapsedMs: number, command: string) => void;
	onError?: (error: QueryError['error']) => void;
}

export class StreamingQuery {
	private queryId: string | null = null;
	private unlisteners: Array<() => void> = [];
	private callbacks: StreamingQueryCallbacks;

	constructor(callbacks: StreamingQueryCallbacks) {
		this.callbacks = callbacks;
	}

	async execute(connectionId: string, sql: string, params: unknown[] = []): Promise<void> {
		// Set up listeners first
		const [unlistenColumns, unlistenRows, unlistenComplete, unlistenError] = await Promise.all([
			onQueryColumns((data) => this.handleColumns(data)),
			onQueryRows((data) => this.handleRows(data)),
			onQueryComplete((data) => this.handleComplete(data)),
			onQueryError((data) => this.handleError(data))
		]);

		this.unlisteners = [unlistenColumns, unlistenRows, unlistenComplete, unlistenError];

		try {
			const result = await queryCommands.executeQueryStream(connectionId, sql, params);
			this.queryId = result.query_id;
		} catch (error) {
			this.cleanup();
			throw error;
		}
	}

	async cancel(): Promise<boolean> {
		if (this.queryId) {
			const result = await queryCommands.cancelQuery(this.queryId);
			this.cleanup();
			return result;
		}
		return false;
	}

	private handleColumns(data: QueryColumns) {
		if (data.query_id === this.queryId && this.callbacks.onColumns) {
			this.callbacks.onColumns(data.columns);
		}
	}

	private handleRows(data: RowBatch) {
		if (data.query_id === this.queryId && this.callbacks.onRows) {
			this.callbacks.onRows(data.rows, data.batch_num);
		}
	}

	private handleComplete(data: QueryComplete) {
		if (data.query_id === this.queryId) {
			if (this.callbacks.onComplete) {
				this.callbacks.onComplete(data.total_rows, data.elapsed_ms, data.command);
			}
			this.cleanup();
		}
	}

	private handleError(data: QueryError) {
		if (data.query_id === this.queryId) {
			if (this.callbacks.onError) {
				this.callbacks.onError(data.error);
			}
			this.cleanup();
		}
	}

	private cleanup() {
		for (const unlisten of this.unlisteners) {
			unlisten();
		}
		this.unlisteners = [];
		this.queryId = null;
	}
}
```

### 6. Command Registration

```rust
// commands/mod.rs
pub mod connection;
pub mod query;
pub mod schema;
pub mod admin;
pub mod storage;
pub mod credentials;

// Re-export all commands for registration in lib.rs
pub use connection::*;
pub use query::*;
pub use schema::*;
pub use admin::*;
pub use storage::*;
pub use credentials::*;
```

```rust
// In lib.rs, register all commands:
.invoke_handler(tauri::generate_handler![
    // Connection
    commands::connect,
    commands::disconnect,
    commands::test_connection,
    commands::list_connections,
    commands::save_connection,
    commands::delete_connection,
    commands::get_connection_status,

    // Query
    commands::execute_query,
    commands::execute_query_stream,
    commands::cancel_query,
    commands::explain_query,

    // Schema
    commands::get_schema,
    commands::refresh_schema,
    commands::get_table_columns,
    commands::get_table_indexes,
    commands::get_table_constraints,
    commands::generate_ddl,

    // Admin
    commands::get_activity,
    commands::get_server_stats,
    commands::get_table_stats,
    commands::get_index_stats,
    commands::get_locks,
    commands::kill_query,
    commands::vacuum_table,
    commands::reindex,

    // Storage
    commands::get_query_history,
    commands::save_query,
    commands::delete_query,
    commands::get_saved_queries,
    commands::get_settings,
    commands::save_settings,

    // Credentials
    commands::store_password,
    commands::get_password,
    commands::delete_password,
])
```

## Acceptance Criteria

1. [ ] All commands compile and are registered
2. [ ] TypeScript types match Rust types
3. [ ] Error responses serialize correctly
4. [ ] Streaming queries emit correct events
5. [ ] Query cancellation works
6. [ ] Connection lifecycle commands work
7. [ ] Frontend can call all commands via ipc service
8. [ ] Events are properly cleaned up after query completion

## Testing with MCP

```
1. Start app: npm run tauri dev
2. Connect: driver_session action=start
3. Test IPC command: ipc_execute_command command="test_connection" args={"config": {...}}
4. Monitor IPC: ipc_monitor action=start
5. Execute query from UI
6. Check captured IPC: ipc_get_captured
7. Verify event flow: query:columns → query:rows → query:complete
```

## Dependencies on Other Features

- 02-backend-architecture.md
- 03-frontend-architecture.md

## Dependent Features

- 05-local-storage.md
- 07-connection-management.md
- 11-query-execution.md
- All features that communicate with backend
