# Feature 20: Admin Dashboard

## Overview

The Admin Dashboard provides real-time monitoring of PostgreSQL server activity, statistics, and health metrics. It displays active queries, connection status, table statistics, index usage, and lock information.

## Goals

- Display active queries from pg_stat_activity
- Show server connection and performance statistics
- Present table statistics with maintenance recommendations
- Display index usage and identify unused indexes
- Visualize lock dependencies and blocking queries
- Support auto-refresh with configurable intervals

## Dependencies

- Feature 07: Connection Pool Management (for database connections)
- Feature 14: Results Grid (for tabular displays)

## Technical Specification

### 20.1 Admin Data Models

```typescript
// src/lib/types/admin.ts

export interface ServerStats {
  version: string;
  startTime: Date;
  uptime: string;
  connectionCount: number;
  maxConnections: number;
  activeQueries: number;
  databaseSizes: DatabaseSize[];
  cacheHitRatio: number;
  transactionsPerSecond: number;
  replicationLag?: string;
  checkpointStats?: CheckpointStats;
}

export interface DatabaseSize {
  name: string;
  sizeBytes: number;
  sizeFormatted: string;
}

export interface CheckpointStats {
  checkpointsTimedCount: number;
  checkpointsRequestedCount: number;
  buffersCheckpoint: number;
  buffersClean: number;
  maxWrittenClean: number;
  buffersBackend: number;
  buffersBackendFsync: number;
  buffersAlloc: number;
}

export interface ActiveQuery {
  pid: number;
  user: string;
  database: string;
  application: string;
  clientAddr: string | null;
  clientPort: number | null;
  state: QueryState;
  waitEventType: string | null;
  waitEvent: string | null;
  query: string;
  queryStart: Date | null;
  stateChange: Date | null;
  duration: number | null; // milliseconds
  backendStart: Date;
  xactStart: Date | null;
  backendType: string;
}

export type QueryState =
  | 'active'
  | 'idle'
  | 'idle in transaction'
  | 'idle in transaction (aborted)'
  | 'fastpath function call'
  | 'disabled';

export interface TableStats {
  schemaName: string;
  tableName: string;
  rowCountEstimate: number;
  totalSizeBytes: number;
  tableSizeBytes: number;
  indexSizeBytes: number;
  seqScans: number;
  seqTuplesRead: number;
  idxScans: number;
  idxTuplesFetch: number;
  insertCount: number;
  updateCount: number;
  deleteCount: number;
  hotUpdateCount: number;
  liveRowCount: number;
  deadRowCount: number;
  lastVacuum: Date | null;
  lastAutoVacuum: Date | null;
  lastAnalyze: Date | null;
  lastAutoAnalyze: Date | null;
  vacuumCount: number;
  autoVacuumCount: number;
  analyzeCount: number;
  autoAnalyzeCount: number;
}

export interface IndexStats {
  schemaName: string;
  tableName: string;
  indexName: string;
  sizeBytes: number;
  scans: number;
  tuplesRead: number;
  tuplesFetched: number;
  isUnique: boolean;
  isPrimary: boolean;
  isValid: boolean;
  definition: string;
  // Computed
  isUnused: boolean;
  isDuplicate: boolean;
}

export interface LockInfo {
  pid: number;
  lockType: string;
  database: string;
  relation: string | null;
  mode: string;
  granted: boolean;
  waitStart: Date | null;
  query: string;
  user: string;
  // For blocked queries
  blockingPid: number | null;
  blockingQuery: string | null;
  blockingUser: string | null;
}

export interface ReplicationStats {
  slotName: string;
  slotType: string;
  active: boolean;
  clientAddr: string | null;
  state: string;
  sentLsn: string;
  writeLsn: string;
  flushLsn: string;
  replayLsn: string;
  writeLag: string | null;
  flushLag: string | null;
  replayLag: string | null;
}
```

### 20.2 Admin Service (Rust)

```rust
// src-tauri/src/services/admin.rs

use serde::{Deserialize, Serialize};
use tokio_postgres::Client;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStats {
    pub version: String,
    pub start_time: DateTime<Utc>,
    pub uptime: String,
    pub connection_count: i32,
    pub max_connections: i32,
    pub active_queries: i32,
    pub database_sizes: Vec<DatabaseSize>,
    pub cache_hit_ratio: f64,
    pub transactions_per_second: f64,
    pub replication_lag: Option<String>,
    pub checkpoint_stats: Option<CheckpointStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseSize {
    pub name: String,
    pub size_bytes: i64,
    pub size_formatted: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointStats {
    pub checkpoints_timed_count: i64,
    pub checkpoints_requested_count: i64,
    pub buffers_checkpoint: i64,
    pub buffers_clean: i64,
    pub max_written_clean: i64,
    pub buffers_backend: i64,
    pub buffers_backend_fsync: i64,
    pub buffers_alloc: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveQuery {
    pub pid: i32,
    pub user: String,
    pub database: String,
    pub application: String,
    pub client_addr: Option<String>,
    pub client_port: Option<i32>,
    pub state: String,
    pub wait_event_type: Option<String>,
    pub wait_event: Option<String>,
    pub query: String,
    pub query_start: Option<DateTime<Utc>>,
    pub state_change: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub backend_start: DateTime<Utc>,
    pub xact_start: Option<DateTime<Utc>>,
    pub backend_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableStats {
    pub schema_name: String,
    pub table_name: String,
    pub row_count_estimate: i64,
    pub total_size_bytes: i64,
    pub table_size_bytes: i64,
    pub index_size_bytes: i64,
    pub seq_scans: i64,
    pub seq_tuples_read: i64,
    pub idx_scans: i64,
    pub idx_tuples_fetch: i64,
    pub insert_count: i64,
    pub update_count: i64,
    pub delete_count: i64,
    pub hot_update_count: i64,
    pub live_row_count: i64,
    pub dead_row_count: i64,
    pub last_vacuum: Option<DateTime<Utc>>,
    pub last_auto_vacuum: Option<DateTime<Utc>>,
    pub last_analyze: Option<DateTime<Utc>>,
    pub last_auto_analyze: Option<DateTime<Utc>>,
    pub vacuum_count: i64,
    pub auto_vacuum_count: i64,
    pub analyze_count: i64,
    pub auto_analyze_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStats {
    pub schema_name: String,
    pub table_name: String,
    pub index_name: String,
    pub size_bytes: i64,
    pub scans: i64,
    pub tuples_read: i64,
    pub tuples_fetched: i64,
    pub is_unique: bool,
    pub is_primary: bool,
    pub is_valid: bool,
    pub definition: String,
    pub is_unused: bool,
    pub is_duplicate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockInfo {
    pub pid: i32,
    pub lock_type: String,
    pub database: String,
    pub relation: Option<String>,
    pub mode: String,
    pub granted: bool,
    pub wait_start: Option<DateTime<Utc>>,
    pub query: String,
    pub user: String,
    pub blocking_pid: Option<i32>,
    pub blocking_query: Option<String>,
    pub blocking_user: Option<String>,
}

pub struct AdminService;

impl AdminService {
    /// Get server statistics
    pub async fn get_server_stats(client: &Client) -> Result<ServerStats, AdminError> {
        // Get version and start time
        let version_row = client
            .query_one("SELECT version(), pg_postmaster_start_time()", &[])
            .await?;
        let version: String = version_row.get(0);
        let start_time: DateTime<Utc> = version_row.get(1);

        // Calculate uptime
        let uptime = Self::format_uptime(start_time);

        // Get connection stats
        let conn_row = client
            .query_one(
                r#"
                SELECT
                    (SELECT count(*) FROM pg_stat_activity)::int AS connection_count,
                    (SELECT setting::int FROM pg_settings WHERE name = 'max_connections') AS max_connections,
                    (SELECT count(*) FROM pg_stat_activity WHERE state = 'active')::int AS active_queries
                "#,
                &[],
            )
            .await?;

        let connection_count: i32 = conn_row.get("connection_count");
        let max_connections: i32 = conn_row.get("max_connections");
        let active_queries: i32 = conn_row.get("active_queries");

        // Get database sizes
        let size_rows = client
            .query(
                r#"
                SELECT
                    datname AS name,
                    pg_database_size(datname) AS size_bytes,
                    pg_size_pretty(pg_database_size(datname)) AS size_formatted
                FROM pg_database
                WHERE datistemplate = false
                ORDER BY pg_database_size(datname) DESC
                "#,
                &[],
            )
            .await?;

        let database_sizes: Vec<DatabaseSize> = size_rows
            .iter()
            .map(|row| DatabaseSize {
                name: row.get("name"),
                size_bytes: row.get("size_bytes"),
                size_formatted: row.get("size_formatted"),
            })
            .collect();

        // Get cache hit ratio
        let cache_row = client
            .query_one(
                r#"
                SELECT
                    CASE
                        WHEN (blks_hit + blks_read) = 0 THEN 100.0
                        ELSE round(blks_hit::numeric / (blks_hit + blks_read) * 100, 2)
                    END AS cache_hit_ratio
                FROM pg_stat_database
                WHERE datname = current_database()
                "#,
                &[],
            )
            .await?;
        let cache_hit_ratio: f64 = cache_row
            .get::<_, rust_decimal::Decimal>("cache_hit_ratio")
            .to_string()
            .parse()
            .unwrap_or(0.0);

        // Get TPS (approximate from stats)
        let tps_row = client
            .query_one(
                r#"
                SELECT
                    COALESCE(xact_commit + xact_rollback, 0) AS total_xacts,
                    EXTRACT(EPOCH FROM (now() - stats_reset))::float AS seconds
                FROM pg_stat_database
                WHERE datname = current_database()
                "#,
                &[],
            )
            .await?;
        let total_xacts: i64 = tps_row.get("total_xacts");
        let seconds: f64 = tps_row.get("seconds");
        let transactions_per_second = if seconds > 0.0 {
            total_xacts as f64 / seconds
        } else {
            0.0
        };

        // Get checkpoint stats
        let checkpoint_stats = Self::get_checkpoint_stats(client).await.ok();

        Ok(ServerStats {
            version,
            start_time,
            uptime,
            connection_count,
            max_connections,
            active_queries,
            database_sizes,
            cache_hit_ratio,
            transactions_per_second,
            replication_lag: None, // Would need to check if replica
            checkpoint_stats,
        })
    }

    async fn get_checkpoint_stats(client: &Client) -> Result<CheckpointStats, AdminError> {
        let row = client
            .query_one(
                r#"
                SELECT
                    checkpoints_timed,
                    checkpoints_req,
                    buffers_checkpoint,
                    buffers_clean,
                    maxwritten_clean,
                    buffers_backend,
                    buffers_backend_fsync,
                    buffers_alloc
                FROM pg_stat_bgwriter
                "#,
                &[],
            )
            .await?;

        Ok(CheckpointStats {
            checkpoints_timed_count: row.get("checkpoints_timed"),
            checkpoints_requested_count: row.get("checkpoints_req"),
            buffers_checkpoint: row.get("buffers_checkpoint"),
            buffers_clean: row.get("buffers_clean"),
            max_written_clean: row.get("maxwritten_clean"),
            buffers_backend: row.get("buffers_backend"),
            buffers_backend_fsync: row.get("buffers_backend_fsync"),
            buffers_alloc: row.get("buffers_alloc"),
        })
    }

    /// Get active queries
    pub async fn get_active_queries(client: &Client) -> Result<Vec<ActiveQuery>, AdminError> {
        let rows = client
            .query(
                r#"
                SELECT
                    pid,
                    usename AS user,
                    datname AS database,
                    COALESCE(application_name, '') AS application,
                    client_addr::text,
                    client_port,
                    state,
                    wait_event_type,
                    wait_event,
                    COALESCE(query, '') AS query,
                    query_start,
                    state_change,
                    CASE
                        WHEN state = 'active' AND query_start IS NOT NULL
                        THEN EXTRACT(EPOCH FROM (now() - query_start)) * 1000
                        ELSE NULL
                    END::bigint AS duration_ms,
                    backend_start,
                    xact_start,
                    backend_type
                FROM pg_stat_activity
                WHERE pid != pg_backend_pid()
                ORDER BY
                    CASE state
                        WHEN 'active' THEN 0
                        WHEN 'idle in transaction' THEN 1
                        ELSE 2
                    END,
                    query_start DESC NULLS LAST
                "#,
                &[],
            )
            .await?;

        let queries: Vec<ActiveQuery> = rows
            .iter()
            .map(|row| ActiveQuery {
                pid: row.get("pid"),
                user: row.get("user"),
                database: row.get("database"),
                application: row.get("application"),
                client_addr: row.get("client_addr"),
                client_port: row.get("client_port"),
                state: row.get("state"),
                wait_event_type: row.get("wait_event_type"),
                wait_event: row.get("wait_event"),
                query: row.get("query"),
                query_start: row.get("query_start"),
                state_change: row.get("state_change"),
                duration_ms: row.get("duration_ms"),
                backend_start: row.get("backend_start"),
                xact_start: row.get("xact_start"),
                backend_type: row.get("backend_type"),
            })
            .collect();

        Ok(queries)
    }

    /// Cancel a query
    pub async fn cancel_query(client: &Client, pid: i32) -> Result<bool, AdminError> {
        let row = client
            .query_one("SELECT pg_cancel_backend($1)", &[&pid])
            .await?;
        Ok(row.get(0))
    }

    /// Terminate a connection
    pub async fn terminate_connection(client: &Client, pid: i32) -> Result<bool, AdminError> {
        let row = client
            .query_one("SELECT pg_terminate_backend($1)", &[&pid])
            .await?;
        Ok(row.get(0))
    }

    /// Get table statistics
    pub async fn get_table_stats(client: &Client) -> Result<Vec<TableStats>, AdminError> {
        let rows = client
            .query(
                r#"
                SELECT
                    schemaname AS schema_name,
                    relname AS table_name,
                    n_live_tup AS row_count_estimate,
                    pg_total_relation_size(relid) AS total_size_bytes,
                    pg_table_size(relid) AS table_size_bytes,
                    pg_indexes_size(relid) AS index_size_bytes,
                    seq_scan AS seq_scans,
                    seq_tup_read AS seq_tuples_read,
                    idx_scan AS idx_scans,
                    idx_tup_fetch AS idx_tuples_fetch,
                    n_tup_ins AS insert_count,
                    n_tup_upd AS update_count,
                    n_tup_del AS delete_count,
                    n_tup_hot_upd AS hot_update_count,
                    n_live_tup AS live_row_count,
                    n_dead_tup AS dead_row_count,
                    last_vacuum,
                    last_autovacuum AS last_auto_vacuum,
                    last_analyze,
                    last_autoanalyze AS last_auto_analyze,
                    vacuum_count,
                    autovacuum_count AS auto_vacuum_count,
                    analyze_count,
                    autoanalyze_count AS auto_analyze_count
                FROM pg_stat_user_tables
                ORDER BY pg_total_relation_size(relid) DESC
                "#,
                &[],
            )
            .await?;

        let stats: Vec<TableStats> = rows
            .iter()
            .map(|row| TableStats {
                schema_name: row.get("schema_name"),
                table_name: row.get("table_name"),
                row_count_estimate: row.get("row_count_estimate"),
                total_size_bytes: row.get("total_size_bytes"),
                table_size_bytes: row.get("table_size_bytes"),
                index_size_bytes: row.get("index_size_bytes"),
                seq_scans: row.get("seq_scans"),
                seq_tuples_read: row.get("seq_tuples_read"),
                idx_scans: row.get::<_, Option<i64>>("idx_scans").unwrap_or(0),
                idx_tuples_fetch: row.get::<_, Option<i64>>("idx_tuples_fetch").unwrap_or(0),
                insert_count: row.get("insert_count"),
                update_count: row.get("update_count"),
                delete_count: row.get("delete_count"),
                hot_update_count: row.get("hot_update_count"),
                live_row_count: row.get("live_row_count"),
                dead_row_count: row.get("dead_row_count"),
                last_vacuum: row.get("last_vacuum"),
                last_auto_vacuum: row.get("last_auto_vacuum"),
                last_analyze: row.get("last_analyze"),
                last_auto_analyze: row.get("last_auto_analyze"),
                vacuum_count: row.get("vacuum_count"),
                auto_vacuum_count: row.get("auto_vacuum_count"),
                analyze_count: row.get("analyze_count"),
                auto_analyze_count: row.get("auto_analyze_count"),
            })
            .collect();

        Ok(stats)
    }

    /// Get index statistics
    pub async fn get_index_stats(client: &Client) -> Result<Vec<IndexStats>, AdminError> {
        let rows = client
            .query(
                r#"
                SELECT
                    schemaname AS schema_name,
                    s.relname AS table_name,
                    indexrelname AS index_name,
                    pg_relation_size(i.indexrelid) AS size_bytes,
                    idx_scan AS scans,
                    idx_tup_read AS tuples_read,
                    idx_tup_fetch AS tuples_fetched,
                    i.indisunique AS is_unique,
                    i.indisprimary AS is_primary,
                    i.indisvalid AS is_valid,
                    pg_get_indexdef(i.indexrelid) AS definition
                FROM pg_stat_user_indexes s
                JOIN pg_index i ON s.indexrelid = i.indexrelid
                ORDER BY pg_relation_size(i.indexrelid) DESC
                "#,
                &[],
            )
            .await?;

        let mut stats: Vec<IndexStats> = rows
            .iter()
            .map(|row| IndexStats {
                schema_name: row.get("schema_name"),
                table_name: row.get("table_name"),
                index_name: row.get("index_name"),
                size_bytes: row.get("size_bytes"),
                scans: row.get("scans"),
                tuples_read: row.get("tuples_read"),
                tuples_fetched: row.get("tuples_fetched"),
                is_unique: row.get("is_unique"),
                is_primary: row.get("is_primary"),
                is_valid: row.get("is_valid"),
                definition: row.get("definition"),
                is_unused: false,
                is_duplicate: false,
            })
            .collect();

        // Mark unused indexes (0 scans since stats reset)
        for stat in &mut stats {
            stat.is_unused = stat.scans == 0 && !stat.is_primary && !stat.is_unique;
        }

        // Detect duplicate indexes (same table, same columns)
        // This is a simplified check
        Self::detect_duplicate_indexes(&mut stats);

        Ok(stats)
    }

    fn detect_duplicate_indexes(stats: &mut Vec<IndexStats>) {
        // Extract column lists from definitions and compare
        let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for (i, stat) in stats.iter().enumerate() {
            // Create a key from table + columns
            // This is simplified - a real implementation would parse the definition
            let key = format!("{}.{}", stat.schema_name, stat.table_name);

            if let Some(_prev_idx) = seen.get(&key) {
                // Mark as potential duplicate for review
                // In practice, would need deeper analysis of column lists
            }
            seen.insert(key.clone(), i);
        }
    }

    /// Get lock information
    pub async fn get_locks(client: &Client) -> Result<Vec<LockInfo>, AdminError> {
        let rows = client
            .query(
                r#"
                SELECT
                    l.pid,
                    l.locktype AS lock_type,
                    COALESCE(d.datname, '') AS database,
                    COALESCE(c.relname, '') AS relation,
                    l.mode,
                    l.granted,
                    l.waitstart AS wait_start,
                    COALESCE(a.query, '') AS query,
                    COALESCE(a.usename, '') AS user,
                    bl.pid AS blocking_pid,
                    ba.query AS blocking_query,
                    ba.usename AS blocking_user
                FROM pg_locks l
                LEFT JOIN pg_database d ON l.database = d.oid
                LEFT JOIN pg_class c ON l.relation = c.oid
                LEFT JOIN pg_stat_activity a ON l.pid = a.pid
                LEFT JOIN pg_locks bl ON bl.granted AND l.locktype = bl.locktype
                    AND l.database IS NOT DISTINCT FROM bl.database
                    AND l.relation IS NOT DISTINCT FROM bl.relation
                    AND l.page IS NOT DISTINCT FROM bl.page
                    AND l.tuple IS NOT DISTINCT FROM bl.tuple
                    AND l.virtualxid IS NOT DISTINCT FROM bl.virtualxid
                    AND l.transactionid IS NOT DISTINCT FROM bl.transactionid
                    AND l.classid IS NOT DISTINCT FROM bl.classid
                    AND l.objid IS NOT DISTINCT FROM bl.objid
                    AND l.objsubid IS NOT DISTINCT FROM bl.objsubid
                    AND l.pid != bl.pid
                LEFT JOIN pg_stat_activity ba ON bl.pid = ba.pid
                WHERE NOT l.granted OR l.pid IN (
                    SELECT pid FROM pg_stat_activity WHERE state = 'active'
                )
                ORDER BY l.granted, l.waitstart NULLS LAST
                "#,
                &[],
            )
            .await?;

        let locks: Vec<LockInfo> = rows
            .iter()
            .map(|row| LockInfo {
                pid: row.get("pid"),
                lock_type: row.get("lock_type"),
                database: row.get("database"),
                relation: {
                    let rel: String = row.get("relation");
                    if rel.is_empty() { None } else { Some(rel) }
                },
                mode: row.get("mode"),
                granted: row.get("granted"),
                wait_start: row.get("wait_start"),
                query: row.get("query"),
                user: row.get("user"),
                blocking_pid: row.get("blocking_pid"),
                blocking_query: row.get("blocking_query"),
                blocking_user: row.get("blocking_user"),
            })
            .collect();

        Ok(locks)
    }

    fn format_uptime(start_time: DateTime<Utc>) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(start_time);

        let days = duration.num_days();
        let hours = duration.num_hours() % 24;
        let minutes = duration.num_minutes() % 60;

        if days > 0 {
            format!("{} days, {} hours, {} minutes", days, hours, minutes)
        } else if hours > 0 {
            format!("{} hours, {} minutes", hours, minutes)
        } else {
            format!("{} minutes", minutes)
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] tokio_postgres::Error),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}
```

### 20.3 Tauri Commands

```rust
// src-tauri/src/commands/admin.rs

use tauri::State;
use crate::services::admin::{AdminService, ServerStats, ActiveQuery, TableStats, IndexStats, LockInfo};
use crate::state::AppState;
use crate::error::Error;

#[tauri::command]
pub async fn get_server_stats(
    state: State<'_, AppState>,
    conn_id: String,
) -> Result<ServerStats, Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    let stats = AdminService::get_server_stats(&client).await?;
    Ok(stats)
}

#[tauri::command]
pub async fn get_active_queries(
    state: State<'_, AppState>,
    conn_id: String,
) -> Result<Vec<ActiveQuery>, Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    let queries = AdminService::get_active_queries(&client).await?;
    Ok(queries)
}

#[tauri::command]
pub async fn cancel_query(
    state: State<'_, AppState>,
    conn_id: String,
    pid: i32,
) -> Result<bool, Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    let result = AdminService::cancel_query(&client, pid).await?;
    Ok(result)
}

#[tauri::command]
pub async fn terminate_connection(
    state: State<'_, AppState>,
    conn_id: String,
    pid: i32,
) -> Result<bool, Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    let result = AdminService::terminate_connection(&client, pid).await?;
    Ok(result)
}

#[tauri::command]
pub async fn get_table_stats(
    state: State<'_, AppState>,
    conn_id: String,
) -> Result<Vec<TableStats>, Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    let stats = AdminService::get_table_stats(&client).await?;
    Ok(stats)
}

#[tauri::command]
pub async fn get_index_stats(
    state: State<'_, AppState>,
    conn_id: String,
) -> Result<Vec<IndexStats>, Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    let stats = AdminService::get_index_stats(&client).await?;
    Ok(stats)
}

#[tauri::command]
pub async fn get_locks(
    state: State<'_, AppState>,
    conn_id: String,
) -> Result<Vec<LockInfo>, Error> {
    let pool = state.get_connection(&conn_id)?;
    let client = pool.get().await?;
    let locks = AdminService::get_locks(&client).await?;
    Ok(locks)
}
```

### 20.4 Admin Store (Svelte)

```typescript
// src/lib/stores/adminStore.svelte.ts

import { invoke } from '@tauri-apps/api/core';
import type {
  ServerStats,
  ActiveQuery,
  TableStats,
  IndexStats,
  LockInfo,
} from '$lib/types/admin';

type AdminTab = 'activity' | 'tables' | 'indexes' | 'locks';

interface AdminState {
  connId: string | null;
  activeTab: AdminTab;
  autoRefresh: boolean;
  refreshInterval: number; // seconds

  serverStats: ServerStats | null;
  activeQueries: ActiveQuery[];
  tableStats: TableStats[];
  indexStats: IndexStats[];
  locks: LockInfo[];

  loading: boolean;
  error: string | null;
}

export function createAdminStore() {
  let state = $state<AdminState>({
    connId: null,
    activeTab: 'activity',
    autoRefresh: true,
    refreshInterval: 5,

    serverStats: null,
    activeQueries: [],
    tableStats: [],
    indexStats: [],
    locks: [],

    loading: false,
    error: null,
  });

  let refreshTimer: ReturnType<typeof setInterval> | null = null;

  function setConnection(connId: string) {
    state.connId = connId;
    refresh();
    startAutoRefresh();
  }

  function setActiveTab(tab: AdminTab) {
    state.activeTab = tab;
    refresh();
  }

  function setAutoRefresh(enabled: boolean) {
    state.autoRefresh = enabled;
    if (enabled) {
      startAutoRefresh();
    } else {
      stopAutoRefresh();
    }
  }

  function setRefreshInterval(seconds: number) {
    state.refreshInterval = seconds;
    if (state.autoRefresh) {
      startAutoRefresh();
    }
  }

  function startAutoRefresh() {
    stopAutoRefresh();
    if (state.autoRefresh && state.connId) {
      refreshTimer = setInterval(() => {
        refresh();
      }, state.refreshInterval * 1000);
    }
  }

  function stopAutoRefresh() {
    if (refreshTimer) {
      clearInterval(refreshTimer);
      refreshTimer = null;
    }
  }

  async function refresh() {
    if (!state.connId) return;

    state.loading = true;
    state.error = null;

    try {
      // Always fetch server stats
      state.serverStats = await invoke<ServerStats>('get_server_stats', {
        connId: state.connId,
      });

      // Fetch tab-specific data
      switch (state.activeTab) {
        case 'activity':
          state.activeQueries = await invoke<ActiveQuery[]>('get_active_queries', {
            connId: state.connId,
          });
          break;

        case 'tables':
          state.tableStats = await invoke<TableStats[]>('get_table_stats', {
            connId: state.connId,
          });
          break;

        case 'indexes':
          state.indexStats = await invoke<IndexStats[]>('get_index_stats', {
            connId: state.connId,
          });
          break;

        case 'locks':
          state.locks = await invoke<LockInfo[]>('get_locks', {
            connId: state.connId,
          });
          break;
      }
    } catch (err) {
      state.error = err instanceof Error ? err.message : String(err);
    } finally {
      state.loading = false;
    }
  }

  async function cancelQuery(pid: number) {
    if (!state.connId) return;

    try {
      const success = await invoke<boolean>('cancel_query', {
        connId: state.connId,
        pid,
      });

      if (success) {
        // Refresh to show updated state
        await refresh();
      }

      return success;
    } catch (err) {
      state.error = err instanceof Error ? err.message : String(err);
      return false;
    }
  }

  async function terminateConnection(pid: number) {
    if (!state.connId) return;

    try {
      const success = await invoke<boolean>('terminate_connection', {
        connId: state.connId,
        pid,
      });

      if (success) {
        await refresh();
      }

      return success;
    } catch (err) {
      state.error = err instanceof Error ? err.message : String(err);
      return false;
    }
  }

  function cleanup() {
    stopAutoRefresh();
  }

  return {
    get connId() { return state.connId; },
    get activeTab() { return state.activeTab; },
    get autoRefresh() { return state.autoRefresh; },
    get refreshInterval() { return state.refreshInterval; },
    get serverStats() { return state.serverStats; },
    get activeQueries() { return state.activeQueries; },
    get tableStats() { return state.tableStats; },
    get indexStats() { return state.indexStats; },
    get locks() { return state.locks; },
    get loading() { return state.loading; },
    get error() { return state.error; },

    setConnection,
    setActiveTab,
    setAutoRefresh,
    setRefreshInterval,
    refresh,
    cancelQuery,
    terminateConnection,
    cleanup,
  };
}

export const adminStore = createAdminStore();
```

### 20.5 Server Stats Component

```svelte
<!-- src/lib/components/admin/ServerStats.svelte -->
<script lang="ts">
  import type { ServerStats } from '$lib/types/admin';

  interface Props {
    stats: ServerStats;
  }

  let { stats }: Props = $props();

  function formatBytes(bytes: number): string {
    if (bytes >= 1_073_741_824) return (bytes / 1_073_741_824).toFixed(2) + ' GB';
    if (bytes >= 1_048_576) return (bytes / 1_048_576).toFixed(2) + ' MB';
    if (bytes >= 1024) return (bytes / 1024).toFixed(2) + ' KB';
    return bytes + ' B';
  }

  function getCacheHitColor(ratio: number): string {
    if (ratio >= 99) return 'text-green-600 dark:text-green-400';
    if (ratio >= 95) return 'text-yellow-600 dark:text-yellow-400';
    return 'text-red-600 dark:text-red-400';
  }

  const connectionPercent = $derived(
    (stats.connectionCount / stats.maxConnections) * 100
  );
</script>

<div class="grid grid-cols-4 gap-4">
  <!-- Connections -->
  <div class="bg-white dark:bg-gray-800 rounded-lg p-4 shadow">
    <div class="text-sm text-gray-500 dark:text-gray-400 mb-1">Connections</div>
    <div class="flex items-end gap-2">
      <span class="text-2xl font-bold">{stats.connectionCount}</span>
      <span class="text-gray-400">/ {stats.maxConnections}</span>
    </div>
    <div class="mt-2 h-2 bg-gray-200 dark:bg-gray-700 rounded overflow-hidden">
      <div
        class="h-full transition-all duration-300
               {connectionPercent >= 90 ? 'bg-red-500' :
                connectionPercent >= 70 ? 'bg-yellow-500' : 'bg-green-500'}"
        style="width: {connectionPercent}%"
      ></div>
    </div>
    <div class="text-xs text-gray-500 mt-1">
      {stats.activeQueries} active queries
    </div>
  </div>

  <!-- Cache Hit Ratio -->
  <div class="bg-white dark:bg-gray-800 rounded-lg p-4 shadow">
    <div class="text-sm text-gray-500 dark:text-gray-400 mb-1">Cache Hit Ratio</div>
    <div class="text-2xl font-bold {getCacheHitColor(stats.cacheHitRatio)}">
      {stats.cacheHitRatio.toFixed(2)}%
    </div>
    <div class="text-xs text-gray-500 mt-1">
      Target: &gt; 99%
    </div>
  </div>

  <!-- TPS -->
  <div class="bg-white dark:bg-gray-800 rounded-lg p-4 shadow">
    <div class="text-sm text-gray-500 dark:text-gray-400 mb-1">Transactions/sec</div>
    <div class="text-2xl font-bold">
      {stats.transactionsPerSecond.toFixed(1)}
    </div>
    <div class="text-xs text-gray-500 mt-1">
      Average since stats reset
    </div>
  </div>

  <!-- Uptime -->
  <div class="bg-white dark:bg-gray-800 rounded-lg p-4 shadow">
    <div class="text-sm text-gray-500 dark:text-gray-400 mb-1">Uptime</div>
    <div class="text-lg font-medium">
      {stats.uptime}
    </div>
    <div class="text-xs text-gray-500 mt-1">
      PostgreSQL {stats.version.split(' ')[1]}
    </div>
  </div>
</div>

<!-- Database Sizes -->
<div class="mt-4 bg-white dark:bg-gray-800 rounded-lg p-4 shadow">
  <h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">Database Sizes</h3>
  <div class="space-y-2">
    {#each stats.databaseSizes as db}
      <div class="flex items-center justify-between">
        <span class="text-sm">{db.name}</span>
        <span class="text-sm font-mono text-gray-600 dark:text-gray-400">
          {db.sizeFormatted}
        </span>
      </div>
    {/each}
  </div>
</div>
```

### 20.6 Active Queries Component

```svelte
<!-- src/lib/components/admin/ActiveQueries.svelte -->
<script lang="ts">
  import type { ActiveQuery } from '$lib/types/admin';
  import { adminStore } from '$lib/stores/adminStore.svelte';

  interface Props {
    queries: ActiveQuery[];
  }

  let { queries }: Props = $props();

  let selectedPid = $state<number | null>(null);
  let showCancelConfirm = $state(false);
  let showTerminateConfirm = $state(false);

  function formatDuration(ms: number | null): string {
    if (ms === null) return '-';
    if (ms < 1000) return ms + 'ms';
    if (ms < 60000) return (ms / 1000).toFixed(1) + 's';
    if (ms < 3600000) return (ms / 60000).toFixed(1) + 'm';
    return (ms / 3600000).toFixed(1) + 'h';
  }

  function getStateColor(state: string): string {
    switch (state) {
      case 'active': return 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400';
      case 'idle': return 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-400';
      case 'idle in transaction': return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400';
      case 'idle in transaction (aborted)': return 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400';
      default: return 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-400';
    }
  }

  function getDurationWarning(ms: number | null): boolean {
    return ms !== null && ms > 30000; // > 30 seconds
  }

  async function handleCancel() {
    if (selectedPid !== null) {
      await adminStore.cancelQuery(selectedPid);
      showCancelConfirm = false;
      selectedPid = null;
    }
  }

  async function handleTerminate() {
    if (selectedPid !== null) {
      await adminStore.terminateConnection(selectedPid);
      showTerminateConfirm = false;
      selectedPid = null;
    }
  }

  function showCancel(pid: number) {
    selectedPid = pid;
    showCancelConfirm = true;
  }

  function showTerminate(pid: number) {
    selectedPid = pid;
    showTerminateConfirm = true;
  }

  const selectedQuery = $derived(
    queries.find(q => q.pid === selectedPid)
  );
</script>

<div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
  <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
    <thead class="bg-gray-50 dark:bg-gray-900/50">
      <tr>
        <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
          PID
        </th>
        <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
          User
        </th>
        <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
          Database
        </th>
        <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
          State
        </th>
        <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
          Duration
        </th>
        <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
          Query
        </th>
        <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
          Actions
        </th>
      </tr>
    </thead>
    <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
      {#each queries as query (query.pid)}
        <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/50">
          <td class="px-4 py-3 text-sm font-mono">
            {query.pid}
          </td>
          <td class="px-4 py-3 text-sm">
            {query.user}
          </td>
          <td class="px-4 py-3 text-sm">
            {query.database}
          </td>
          <td class="px-4 py-3">
            <span class="inline-flex px-2 py-0.5 rounded text-xs font-medium {getStateColor(query.state)}">
              {query.state}
            </span>
            {#if query.waitEventType}
              <span class="ml-1 text-xs text-gray-500">
                ({query.waitEventType}: {query.waitEvent})
              </span>
            {/if}
          </td>
          <td class="px-4 py-3 text-sm font-mono
                     {getDurationWarning(query.durationMs) ? 'text-red-600 dark:text-red-400 font-bold' : ''}">
            {formatDuration(query.durationMs)}
            {#if getDurationWarning(query.durationMs)}
              <span class="text-red-500">⚠</span>
            {/if}
          </td>
          <td class="px-4 py-3 text-sm max-w-md">
            <div class="truncate font-mono text-xs" title={query.query}>
              {query.query || '-'}
            </div>
          </td>
          <td class="px-4 py-3 text-right">
            {#if query.state === 'active'}
              <button
                onclick={() => showCancel(query.pid)}
                class="text-yellow-600 hover:text-yellow-700 dark:text-yellow-400
                       dark:hover:text-yellow-300 text-sm mr-2"
                title="Cancel Query"
              >
                Cancel
              </button>
            {/if}
            <button
              onclick={() => showTerminate(query.pid)}
              class="text-red-600 hover:text-red-700 dark:text-red-400
                     dark:hover:text-red-300 text-sm"
              title="Terminate Connection"
            >
              Kill
            </button>
          </td>
        </tr>
      {:else}
        <tr>
          <td colspan="7" class="px-4 py-8 text-center text-gray-500">
            No active connections
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<!-- Cancel Confirmation Dialog -->
{#if showCancelConfirm && selectedQuery}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-[500px] p-6">
      <h3 class="text-lg font-semibold mb-4">Cancel Query?</h3>
      <p class="text-sm text-gray-600 dark:text-gray-400 mb-4">
        This will cancel the running query for PID {selectedQuery.pid} ({selectedQuery.user}).
        The connection will remain open.
      </p>
      <div class="p-3 bg-gray-100 dark:bg-gray-900 rounded font-mono text-xs mb-4 max-h-32 overflow-auto">
        {selectedQuery.query}
      </div>
      <div class="flex justify-end gap-2">
        <button
          onclick={() => showCancelConfirm = false}
          class="px-4 py-2 text-sm text-gray-700 dark:text-gray-300
                 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
        >
          Cancel
        </button>
        <button
          onclick={handleCancel}
          class="px-4 py-2 text-sm bg-yellow-600 text-white rounded hover:bg-yellow-700"
        >
          Cancel Query
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Terminate Confirmation Dialog -->
{#if showTerminateConfirm && selectedQuery}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-[500px] p-6">
      <h3 class="text-lg font-semibold mb-4 text-red-600">Terminate Connection?</h3>
      <p class="text-sm text-gray-600 dark:text-gray-400 mb-4">
        This will forcefully terminate the connection for PID {selectedQuery.pid} ({selectedQuery.user}).
        Any uncommitted transactions will be rolled back.
      </p>
      <div class="p-3 bg-gray-100 dark:bg-gray-900 rounded font-mono text-xs mb-4 max-h-32 overflow-auto">
        {selectedQuery.query || '(no active query)'}
      </div>
      <div class="flex justify-end gap-2">
        <button
          onclick={() => showTerminateConfirm = false}
          class="px-4 py-2 text-sm text-gray-700 dark:text-gray-300
                 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
        >
          Cancel
        </button>
        <button
          onclick={handleTerminate}
          class="px-4 py-2 text-sm bg-red-600 text-white rounded hover:bg-red-700"
        >
          Terminate
        </button>
      </div>
    </div>
  </div>
{/if}
```

### 20.7 Table Stats Component

```svelte
<!-- src/lib/components/admin/TableStats.svelte -->
<script lang="ts">
  import type { TableStats } from '$lib/types/admin';

  interface Props {
    stats: TableStats[];
    onAction: (action: string, schema: string, table: string) => void;
  }

  let { stats, onAction }: Props = $props();

  let sortColumn = $state<keyof TableStats>('totalSizeBytes');
  let sortDirection = $state<'asc' | 'desc'>('desc');
  let filter = $state('');

  function formatBytes(bytes: number): string {
    if (bytes >= 1_073_741_824) return (bytes / 1_073_741_824).toFixed(2) + ' GB';
    if (bytes >= 1_048_576) return (bytes / 1_048_576).toFixed(2) + ' MB';
    if (bytes >= 1024) return (bytes / 1024).toFixed(2) + ' KB';
    return bytes + ' B';
  }

  function formatNumber(n: number): string {
    if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M';
    if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K';
    return n.toString();
  }

  function formatDate(date: Date | null): string {
    if (!date) return 'Never';
    const now = new Date();
    const diff = now.getTime() - new Date(date).getTime();
    const hours = Math.floor(diff / 3600000);
    if (hours < 1) return 'Just now';
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    if (days < 7) return `${days}d ago`;
    return new Date(date).toLocaleDateString();
  }

  function needsVacuum(stat: TableStats): boolean {
    // Heuristic: needs vacuum if dead rows > 10% of live rows
    return stat.deadRowCount > stat.liveRowCount * 0.1;
  }

  function needsAnalyze(stat: TableStats): boolean {
    // Heuristic: needs analyze if last analyze was > 7 days ago or never
    if (!stat.lastAnalyze && !stat.lastAutoAnalyze) return true;
    const lastAnalyze = stat.lastAutoAnalyze || stat.lastAnalyze;
    if (!lastAnalyze) return true;
    const daysSince = (Date.now() - new Date(lastAnalyze).getTime()) / 86400000;
    return daysSince > 7;
  }

  function toggleSort(column: keyof TableStats) {
    if (sortColumn === column) {
      sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
      sortColumn = column;
      sortDirection = 'desc';
    }
  }

  const filteredStats = $derived(
    stats.filter(s =>
      !filter ||
      s.tableName.toLowerCase().includes(filter.toLowerCase()) ||
      s.schemaName.toLowerCase().includes(filter.toLowerCase())
    )
  );

  const sortedStats = $derived(
    [...filteredStats].sort((a, b) => {
      const aVal = a[sortColumn];
      const bVal = b[sortColumn];
      const cmp = aVal < bVal ? -1 : aVal > bVal ? 1 : 0;
      return sortDirection === 'asc' ? cmp : -cmp;
    })
  );
</script>

<div class="space-y-4">
  <!-- Filter -->
  <div class="flex items-center gap-4">
    <input
      type="text"
      bind:value={filter}
      placeholder="Filter tables..."
      class="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
             bg-white dark:bg-gray-700 text-sm w-64"
    />
    <span class="text-sm text-gray-500">
      {filteredStats.length} of {stats.length} tables
    </span>
  </div>

  <!-- Table -->
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-x-auto">
    <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
      <thead class="bg-gray-50 dark:bg-gray-900/50">
        <tr>
          <th
            class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800"
            onclick={() => toggleSort('tableName')}
          >
            Table
            {#if sortColumn === 'tableName'}
              <span>{sortDirection === 'asc' ? '▲' : '▼'}</span>
            {/if}
          </th>
          <th
            class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800"
            onclick={() => toggleSort('rowCountEstimate')}
          >
            Rows
            {#if sortColumn === 'rowCountEstimate'}
              <span>{sortDirection === 'asc' ? '▲' : '▼'}</span>
            {/if}
          </th>
          <th
            class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800"
            onclick={() => toggleSort('totalSizeBytes')}
          >
            Size
            {#if sortColumn === 'totalSizeBytes'}
              <span>{sortDirection === 'asc' ? '▲' : '▼'}</span>
            {/if}
          </th>
          <th
            class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800"
            onclick={() => toggleSort('seqScans')}
          >
            Seq Scans
            {#if sortColumn === 'seqScans'}
              <span>{sortDirection === 'asc' ? '▲' : '▼'}</span>
            {/if}
          </th>
          <th
            class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800"
            onclick={() => toggleSort('idxScans')}
          >
            Idx Scans
            {#if sortColumn === 'idxScans'}
              <span>{sortDirection === 'asc' ? '▲' : '▼'}</span>
            {/if}
          </th>
          <th
            class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800"
            onclick={() => toggleSort('deadRowCount')}
          >
            Dead Rows
            {#if sortColumn === 'deadRowCount'}
              <span>{sortDirection === 'asc' ? '▲' : '▼'}</span>
            {/if}
          </th>
          <th class="px-4 py-3 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">
            Last Vacuum
          </th>
          <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
            Actions
          </th>
        </tr>
      </thead>
      <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
        {#each sortedStats as stat (stat.schemaName + '.' + stat.tableName)}
          <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/50">
            <td class="px-4 py-3 text-sm">
              <span class="text-gray-500">{stat.schemaName}.</span>
              <span class="font-medium">{stat.tableName}</span>
            </td>
            <td class="px-4 py-3 text-sm text-right font-mono">
              {formatNumber(stat.rowCountEstimate)}
            </td>
            <td class="px-4 py-3 text-sm text-right font-mono">
              {formatBytes(stat.totalSizeBytes)}
            </td>
            <td class="px-4 py-3 text-sm text-right font-mono">
              {formatNumber(stat.seqScans)}
            </td>
            <td class="px-4 py-3 text-sm text-right font-mono">
              {formatNumber(stat.idxScans)}
            </td>
            <td class="px-4 py-3 text-sm text-right font-mono
                       {needsVacuum(stat) ? 'text-red-600 dark:text-red-400' : ''}">
              {formatNumber(stat.deadRowCount)}
              {#if needsVacuum(stat)}
                <span title="High dead row count">⚠</span>
              {/if}
            </td>
            <td class="px-4 py-3 text-sm text-center
                       {needsAnalyze(stat) ? 'text-yellow-600 dark:text-yellow-400' : ''}">
              {formatDate(stat.lastAutoVacuum || stat.lastVacuum)}
            </td>
            <td class="px-4 py-3 text-right">
              <div class="flex items-center justify-end gap-1">
                <button
                  onclick={() => onAction('vacuum', stat.schemaName, stat.tableName)}
                  class="px-2 py-1 text-xs bg-blue-100 text-blue-700 dark:bg-blue-900/30
                         dark:text-blue-400 rounded hover:bg-blue-200 dark:hover:bg-blue-900/50"
                  title="VACUUM"
                >
                  Vacuum
                </button>
                <button
                  onclick={() => onAction('analyze', stat.schemaName, stat.tableName)}
                  class="px-2 py-1 text-xs bg-green-100 text-green-700 dark:bg-green-900/30
                         dark:text-green-400 rounded hover:bg-green-200 dark:hover:bg-green-900/50"
                  title="ANALYZE"
                >
                  Analyze
                </button>
              </div>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
```

### 20.8 Admin Dashboard Page

```svelte
<!-- src/lib/components/admin/AdminDashboard.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { adminStore } from '$lib/stores/adminStore.svelte';
  import ServerStats from './ServerStats.svelte';
  import ActiveQueries from './ActiveQueries.svelte';
  import TableStats from './TableStats.svelte';
  import IndexStats from './IndexStats.svelte';
  import LocksView from './LocksView.svelte';

  interface Props {
    connId: string;
  }

  let { connId }: Props = $props();

  onMount(() => {
    adminStore.setConnection(connId);
  });

  onDestroy(() => {
    adminStore.cleanup();
  });

  function handleTableAction(action: string, schema: string, table: string) {
    // Open maintenance dialog
    console.log(`${action} on ${schema}.${table}`);
  }

  const tabs = [
    { id: 'activity', label: 'Activity', icon: '📊' },
    { id: 'tables', label: 'Tables', icon: '📋' },
    { id: 'indexes', label: 'Indexes', icon: '🔍' },
    { id: 'locks', label: 'Locks', icon: '🔒' },
  ] as const;
</script>

<div class="flex flex-col h-full">
  <!-- Header -->
  <div class="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700">
    <h1 class="text-lg font-semibold">Admin Dashboard</h1>

    <div class="flex items-center gap-4">
      <!-- Auto-refresh toggle -->
      <label class="flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          checked={adminStore.autoRefresh}
          onchange={(e) => adminStore.setAutoRefresh(e.currentTarget.checked)}
          class="rounded"
        />
        Auto-refresh
      </label>

      <!-- Refresh interval -->
      <select
        value={adminStore.refreshInterval}
        onchange={(e) => adminStore.setRefreshInterval(parseInt(e.currentTarget.value))}
        disabled={!adminStore.autoRefresh}
        class="px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded
               bg-white dark:bg-gray-700 disabled:opacity-50"
      >
        <option value="1">1s</option>
        <option value="5">5s</option>
        <option value="10">10s</option>
        <option value="30">30s</option>
        <option value="60">60s</option>
      </select>

      <!-- Manual refresh -->
      <button
        onclick={() => adminStore.refresh()}
        disabled={adminStore.loading}
        class="px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700
               disabled:opacity-50 flex items-center gap-2"
      >
        {#if adminStore.loading}
          <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
          </svg>
        {:else}
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0
                     0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
        {/if}
        Refresh
      </button>
    </div>
  </div>

  <!-- Server Stats -->
  {#if adminStore.serverStats}
    <div class="px-4 py-4 border-b border-gray-200 dark:border-gray-700">
      <ServerStats stats={adminStore.serverStats} />
    </div>
  {/if}

  <!-- Tabs -->
  <div class="flex border-b border-gray-200 dark:border-gray-700">
    {#each tabs as tab}
      <button
        onclick={() => adminStore.setActiveTab(tab.id)}
        class="px-4 py-3 text-sm font-medium transition-colors
               {adminStore.activeTab === tab.id
                 ? 'border-b-2 border-blue-500 text-blue-600 dark:text-blue-400'
                 : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'}"
      >
        <span class="mr-2">{tab.icon}</span>
        {tab.label}
      </button>
    {/each}
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-auto p-4">
    {#if adminStore.error}
      <div class="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200
                  dark:border-red-800 rounded text-red-700 dark:text-red-400">
        {adminStore.error}
      </div>
    {:else if adminStore.activeTab === 'activity'}
      <ActiveQueries queries={adminStore.activeQueries} />
    {:else if adminStore.activeTab === 'tables'}
      <TableStats stats={adminStore.tableStats} onAction={handleTableAction} />
    {:else if adminStore.activeTab === 'indexes'}
      <IndexStats stats={adminStore.indexStats} />
    {:else if adminStore.activeTab === 'locks'}
      <LocksView locks={adminStore.locks} />
    {/if}
  </div>
</div>
```

## Acceptance Criteria

1. **Server Statistics**
   - [ ] Display PostgreSQL version and uptime
   - [ ] Show connection count with max connections
   - [ ] Calculate and display cache hit ratio
   - [ ] Show transactions per second
   - [ ] List database sizes

2. **Activity Monitor**
   - [ ] Display all active connections from pg_stat_activity
   - [ ] Show query state, duration, and wait events
   - [ ] Highlight long-running queries
   - [ ] Support canceling queries
   - [ ] Support terminating connections
   - [ ] Auto-refresh with configurable interval

3. **Table Statistics**
   - [ ] Show all user tables with sizes
   - [ ] Display row counts, scan counts
   - [ ] Show dead row counts
   - [ ] Display last vacuum/analyze times
   - [ ] Highlight tables needing maintenance
   - [ ] Support sorting and filtering

4. **Index Statistics**
   - [ ] List all indexes with usage stats
   - [ ] Identify unused indexes
   - [ ] Show index sizes
   - [ ] Display scan counts

5. **Lock Monitoring**
   - [ ] Show current locks
   - [ ] Identify blocking/blocked queries
   - [ ] Display lock types and modes

## MCP Testing Instructions

### Tauri MCP Testing

```typescript
// Connect to database
await mcp___hypothesi_tauri_mcp_server__ipc_execute_command({
  command: 'get_server_stats',
  args: { connId: 'test-conn' }
});

// Verify server stats display
const snapshot = await mcp___hypothesi_tauri_mcp_server__webview_dom_snapshot({
  type: 'accessibility'
});

// Test auto-refresh toggle
await mcp___hypothesi_tauri_mcp_server__webview_click({
  selector: 'input[type="checkbox"]:near(:text("Auto-refresh"))'
});

// Switch to Tables tab
await mcp___hypothesi_tauri_mcp_server__webview_click({
  selector: 'button:has-text("Tables")'
});

// Verify table stats load
await mcp___hypothesi_tauri_mcp_server__webview_wait_for({
  type: 'selector',
  value: 'table tbody tr'
});
```

### Playwright MCP Testing

```typescript
// Open admin dashboard
await mcp__playwright__browser_navigate({
  url: 'http://localhost:1420/admin'
});

// Take snapshot of dashboard
await mcp__playwright__browser_snapshot({});

// Test cancel query flow
await mcp__playwright__browser_click({
  element: 'Cancel query button',
  ref: 'button:has-text("Cancel"):first'
});

// Verify confirmation dialog
await mcp__playwright__browser_wait_for({
  text: 'Cancel Query?'
});

// Take screenshot
await mcp__playwright__browser_take_screenshot({
  filename: 'admin-dashboard.png'
});
```
