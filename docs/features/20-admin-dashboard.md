# Feature 20: Admin Dashboard

## Overview

This feature implements the admin dashboard for monitoring and managing PostgreSQL server health. It provides real-time server statistics, active query monitoring, table and index statistics, and lock monitoring—all built as native GPUI components with direct service integration.

**Dependencies:** Features 07 (Connection Management), 14 (Results Grid)

## 20.1 Admin Data Models

```rust
// src/models/admin.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Server-wide statistics from pg_stat_database and pg_stat_bgwriter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStats {
    /// PostgreSQL version string
    pub version: String,
    /// Server uptime formatted as human-readable string
    pub uptime: String,
    /// Current number of connections
    pub connection_count: i32,
    /// Maximum allowed connections
    pub max_connections: i32,
    /// Number of currently active queries
    pub active_queries: i32,
    /// Buffer cache hit ratio percentage
    pub cache_hit_ratio: f64,
    /// Transactions per second since stats reset
    pub transactions_per_second: f64,
    /// Commits since stats reset
    pub commits: i64,
    /// Rollbacks since stats reset
    pub rollbacks: i64,
    /// Block reads (from disk)
    pub blocks_read: i64,
    /// Block hits (from cache)
    pub blocks_hit: i64,
    /// Temporary files created
    pub temp_files: i64,
    /// Total size of temporary files in bytes
    pub temp_bytes: i64,
    /// Deadlocks detected
    pub deadlocks: i64,
    /// Checkpoints timed
    pub checkpoints_timed: i64,
    /// Checkpoints requested
    pub checkpoints_req: i64,
    /// Database sizes
    pub database_sizes: Vec<DatabaseSize>,
}

/// Individual database size information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSize {
    /// Database name
    pub name: String,
    /// Size in bytes
    pub size_bytes: i64,
    /// Human-readable size string
    pub size_formatted: String,
}

/// Active connection/query from pg_stat_activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveQuery {
    /// Process ID
    pub pid: i32,
    /// Database name
    pub database: String,
    /// Username
    pub user: String,
    /// Application name
    pub application_name: String,
    /// Client IP address
    pub client_addr: Option<String>,
    /// Connection state (active, idle, idle in transaction, etc.)
    pub state: String,
    /// Wait event type if waiting
    pub wait_event_type: Option<String>,
    /// Wait event if waiting
    pub wait_event: Option<String>,
    /// Current query text (may be truncated)
    pub query: Option<String>,
    /// Query start time
    pub query_start: Option<DateTime<Utc>>,
    /// Duration in milliseconds
    pub duration_ms: Option<i64>,
    /// Transaction start time
    pub xact_start: Option<DateTime<Utc>>,
    /// Backend start time
    pub backend_start: DateTime<Utc>,
    /// State change time
    pub state_change: Option<DateTime<Utc>>,
    /// Backend type (client backend, autovacuum worker, etc.)
    pub backend_type: String,
}

/// Table statistics from pg_stat_user_tables and pg_class
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStats {
    /// Schema name
    pub schema_name: String,
    /// Table name
    pub table_name: String,
    /// Estimated row count
    pub row_count_estimate: i64,
    /// Total table size (including indexes) in bytes
    pub total_size_bytes: i64,
    /// Table data size in bytes
    pub table_size_bytes: i64,
    /// Indexes size in bytes
    pub indexes_size_bytes: i64,
    /// Toast size in bytes
    pub toast_size_bytes: i64,
    /// Sequential scans
    pub seq_scans: i64,
    /// Rows fetched by sequential scans
    pub seq_tup_read: i64,
    /// Index scans
    pub idx_scans: i64,
    /// Rows fetched by index scans
    pub idx_tup_fetch: i64,
    /// Rows inserted
    pub n_tup_ins: i64,
    /// Rows updated
    pub n_tup_upd: i64,
    /// Rows deleted
    pub n_tup_del: i64,
    /// Rows HOT updated
    pub n_tup_hot_upd: i64,
    /// Live row count
    pub live_row_count: i64,
    /// Dead row count
    pub dead_row_count: i64,
    /// Last manual vacuum time
    pub last_vacuum: Option<DateTime<Utc>>,
    /// Last autovacuum time
    pub last_autovacuum: Option<DateTime<Utc>>,
    /// Last manual analyze time
    pub last_analyze: Option<DateTime<Utc>>,
    /// Last autoanalyze time
    pub last_autoanalyze: Option<DateTime<Utc>>,
    /// Vacuum count
    pub vacuum_count: i64,
    /// Autovacuum count
    pub autovacuum_count: i64,
    /// Analyze count
    pub analyze_count: i64,
    /// Autoanalyze count
    pub autoanalyze_count: i64,
}

impl TableStats {
    /// Check if table needs vacuum based on dead row ratio
    pub fn needs_vacuum(&self) -> bool {
        if self.live_row_count == 0 {
            return self.dead_row_count > 0;
        }
        // Needs vacuum if dead rows > 10% of live rows
        self.dead_row_count > (self.live_row_count as f64 * 0.1) as i64
    }

    /// Check if table needs analyze based on last analyze time
    pub fn needs_analyze(&self) -> bool {
        let last_analyze = self.last_autoanalyze.or(self.last_analyze);
        match last_analyze {
            None => true,
            Some(dt) => {
                let days_since = (Utc::now() - dt).num_days();
                days_since > 7
            }
        }
    }
}

/// Index statistics from pg_stat_user_indexes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Schema name
    pub schema_name: String,
    /// Table name
    pub table_name: String,
    /// Index name
    pub index_name: String,
    /// Index size in bytes
    pub index_size_bytes: i64,
    /// Number of index scans
    pub idx_scan: i64,
    /// Tuples read
    pub idx_tup_read: i64,
    /// Tuples fetched
    pub idx_tup_fetch: i64,
    /// Index definition
    pub index_def: String,
    /// Whether index is unique
    pub is_unique: bool,
    /// Whether index is primary key
    pub is_primary: bool,
    /// Whether index is valid
    pub is_valid: bool,
    /// Index type (btree, hash, gist, etc.)
    pub index_type: String,
}

impl IndexStats {
    /// Check if index is potentially unused
    pub fn is_unused(&self) -> bool {
        // Consider unused if no scans and not a primary/unique constraint
        self.idx_scan == 0 && !self.is_primary && !self.is_unique
    }
}

/// Lock information from pg_locks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    /// Process ID holding/waiting for lock
    pub pid: i32,
    /// Lock type (relation, transactionid, tuple, etc.)
    pub locktype: String,
    /// Database OID
    pub database: Option<i32>,
    /// Relation OID
    pub relation: Option<i32>,
    /// Relation name (if available)
    pub relation_name: Option<String>,
    /// Schema name (if available)
    pub schema_name: Option<String>,
    /// Lock mode (AccessShareLock, RowExclusiveLock, etc.)
    pub mode: String,
    /// Whether lock is granted
    pub granted: bool,
    /// Transaction ID
    pub transaction_id: Option<i64>,
    /// Query of the process
    pub query: Option<String>,
    /// Username
    pub user: String,
    /// Whether this lock is blocking others
    pub is_blocking: bool,
    /// PIDs blocked by this lock
    pub blocking_pids: Vec<i32>,
}

/// Admin dashboard tab identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AdminTab {
    #[default]
    Activity,
    Tables,
    Indexes,
    Locks,
}

impl AdminTab {
    pub fn label(&self) -> &'static str {
        match self {
            AdminTab::Activity => "Activity",
            AdminTab::Tables => "Tables",
            AdminTab::Indexes => "Indexes",
            AdminTab::Locks => "Locks",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            AdminTab::Activity => "activity",
            AdminTab::Tables => "table",
            AdminTab::Indexes => "search",
            AdminTab::Locks => "lock",
        }
    }

    pub fn all() -> &'static [AdminTab] {
        &[
            AdminTab::Activity,
            AdminTab::Tables,
            AdminTab::Indexes,
            AdminTab::Locks,
        ]
    }
}

/// Column for sorting table stats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableSortColumn {
    Name,
    RowCount,
    TotalSize,
    SeqScans,
    IdxScans,
    DeadRows,
    LastVacuum,
}

/// Column for sorting index stats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexSortColumn {
    Name,
    Table,
    Size,
    Scans,
    Reads,
    Type,
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    Ascending,
    #[default]
    Descending,
}

impl SortDirection {
    pub fn toggle(&self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }
}
```

## 20.2 Admin Service

```rust
// src/services/admin.rs

use crate::error::{Error, Result};
use crate::models::admin::*;
use crate::services::connection::ConnectionService;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio_postgres::Row;

/// Service for fetching admin/monitoring data from PostgreSQL
pub struct AdminService {
    connection_service: Arc<ConnectionService>,
}

impl AdminService {
    pub fn new(connection_service: Arc<ConnectionService>) -> Self {
        Self { connection_service }
    }

    /// Get server-wide statistics
    pub async fn get_server_stats(&self, conn_id: &str) -> Result<ServerStats> {
        let pool = self.connection_service.get_pool(conn_id)?;
        let client = pool.get().await?;

        // Get PostgreSQL version
        let version_row = client.query_one("SELECT version()", &[]).await?;
        let version: String = version_row.get(0);

        // Get uptime
        let uptime_row = client
            .query_one(
                "SELECT current_timestamp - pg_postmaster_start_time()",
                &[],
            )
            .await?;
        let uptime_interval: postgres_types::Interval = uptime_row.get(0);
        let uptime = format_interval(&uptime_interval);

        // Get connection stats
        let conn_row = client
            .query_one(
                r#"
                SELECT
                    (SELECT count(*) FROM pg_stat_activity) as conn_count,
                    (SELECT setting::int FROM pg_settings WHERE name = 'max_connections') as max_conn,
                    (SELECT count(*) FROM pg_stat_activity WHERE state = 'active') as active
                "#,
                &[],
            )
            .await?;
        let connection_count: i64 = conn_row.get(0);
        let max_connections: i32 = conn_row.get(1);
        let active_queries: i64 = conn_row.get(2);

        // Get database stats (aggregated)
        let db_stats_row = client
            .query_one(
                r#"
                SELECT
                    COALESCE(sum(xact_commit), 0) as commits,
                    COALESCE(sum(xact_rollback), 0) as rollbacks,
                    COALESCE(sum(blks_read), 0) as blks_read,
                    COALESCE(sum(blks_hit), 0) as blks_hit,
                    COALESCE(sum(temp_files), 0) as temp_files,
                    COALESCE(sum(temp_bytes), 0) as temp_bytes,
                    COALESCE(sum(deadlocks), 0) as deadlocks,
                    EXTRACT(EPOCH FROM (current_timestamp - min(stats_reset)))::float as stats_age
                FROM pg_stat_database
                WHERE datname IS NOT NULL
                "#,
                &[],
            )
            .await?;

        let commits: i64 = db_stats_row.get(0);
        let rollbacks: i64 = db_stats_row.get(1);
        let blocks_read: i64 = db_stats_row.get(2);
        let blocks_hit: i64 = db_stats_row.get(3);
        let temp_files: i64 = db_stats_row.get(4);
        let temp_bytes: i64 = db_stats_row.get(5);
        let deadlocks: i64 = db_stats_row.get(6);
        let stats_age: Option<f64> = db_stats_row.get(7);

        // Calculate cache hit ratio
        let total_blocks = blocks_read + blocks_hit;
        let cache_hit_ratio = if total_blocks > 0 {
            (blocks_hit as f64 / total_blocks as f64) * 100.0
        } else {
            100.0
        };

        // Calculate TPS
        let transactions_per_second = if let Some(age) = stats_age {
            if age > 0.0 {
                (commits + rollbacks) as f64 / age
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Get bgwriter stats
        let bgwriter_row = client
            .query_one(
                "SELECT checkpoints_timed, checkpoints_req FROM pg_stat_bgwriter",
                &[],
            )
            .await?;
        let checkpoints_timed: i64 = bgwriter_row.get(0);
        let checkpoints_req: i64 = bgwriter_row.get(1);

        // Get database sizes
        let size_rows = client
            .query(
                r#"
                SELECT
                    datname,
                    pg_database_size(datname) as size_bytes
                FROM pg_database
                WHERE datistemplate = false
                ORDER BY pg_database_size(datname) DESC
                "#,
                &[],
            )
            .await?;

        let database_sizes: Vec<DatabaseSize> = size_rows
            .iter()
            .map(|row| {
                let name: String = row.get(0);
                let size_bytes: i64 = row.get(1);
                DatabaseSize {
                    name,
                    size_bytes,
                    size_formatted: format_bytes(size_bytes),
                }
            })
            .collect();

        Ok(ServerStats {
            version,
            uptime,
            connection_count: connection_count as i32,
            max_connections,
            active_queries: active_queries as i32,
            cache_hit_ratio,
            transactions_per_second,
            commits,
            rollbacks,
            blocks_read,
            blocks_hit,
            temp_files,
            temp_bytes,
            deadlocks,
            checkpoints_timed,
            checkpoints_req,
            database_sizes,
        })
    }

    /// Get active connections/queries
    pub async fn get_active_queries(&self, conn_id: &str) -> Result<Vec<ActiveQuery>> {
        let pool = self.connection_service.get_pool(conn_id)?;
        let client = pool.get().await?;

        let rows = client
            .query(
                r#"
                SELECT
                    pid,
                    datname,
                    usename,
                    application_name,
                    client_addr::text,
                    state,
                    wait_event_type,
                    wait_event,
                    query,
                    query_start,
                    EXTRACT(EPOCH FROM (clock_timestamp() - query_start)) * 1000 as duration_ms,
                    xact_start,
                    backend_start,
                    state_change,
                    backend_type
                FROM pg_stat_activity
                WHERE pid != pg_backend_pid()
                ORDER BY
                    CASE state
                        WHEN 'active' THEN 1
                        WHEN 'idle in transaction' THEN 2
                        WHEN 'idle in transaction (aborted)' THEN 3
                        ELSE 4
                    END,
                    query_start DESC NULLS LAST
                "#,
                &[],
            )
            .await?;

        let queries = rows.iter().map(|row| row_to_active_query(row)).collect();
        Ok(queries)
    }

    /// Cancel a running query
    pub async fn cancel_query(&self, conn_id: &str, pid: i32) -> Result<bool> {
        let pool = self.connection_service.get_pool(conn_id)?;
        let client = pool.get().await?;

        let row = client
            .query_one("SELECT pg_cancel_backend($1)", &[&pid])
            .await?;
        let success: bool = row.get(0);
        Ok(success)
    }

    /// Terminate a connection
    pub async fn terminate_connection(&self, conn_id: &str, pid: i32) -> Result<bool> {
        let pool = self.connection_service.get_pool(conn_id)?;
        let client = pool.get().await?;

        let row = client
            .query_one("SELECT pg_terminate_backend($1)", &[&pid])
            .await?;
        let success: bool = row.get(0);
        Ok(success)
    }

    /// Get table statistics
    pub async fn get_table_stats(&self, conn_id: &str) -> Result<Vec<TableStats>> {
        let pool = self.connection_service.get_pool(conn_id)?;
        let client = pool.get().await?;

        let rows = client
            .query(
                r#"
                SELECT
                    schemaname,
                    relname,
                    n_live_tup,
                    pg_total_relation_size(schemaname || '.' || relname) as total_size,
                    pg_relation_size(schemaname || '.' || relname) as table_size,
                    pg_indexes_size(schemaname || '.' || relname) as indexes_size,
                    COALESCE(pg_total_relation_size(schemaname || '.' || relname || '_toast'), 0) as toast_size,
                    seq_scan,
                    seq_tup_read,
                    idx_scan,
                    idx_tup_fetch,
                    n_tup_ins,
                    n_tup_upd,
                    n_tup_del,
                    n_tup_hot_upd,
                    n_live_tup,
                    n_dead_tup,
                    last_vacuum,
                    last_autovacuum,
                    last_analyze,
                    last_autoanalyze,
                    vacuum_count,
                    autovacuum_count,
                    analyze_count,
                    autoanalyze_count
                FROM pg_stat_user_tables
                ORDER BY pg_total_relation_size(schemaname || '.' || relname) DESC
                "#,
                &[],
            )
            .await?;

        let stats = rows.iter().map(|row| row_to_table_stats(row)).collect();
        Ok(stats)
    }

    /// Get index statistics
    pub async fn get_index_stats(&self, conn_id: &str) -> Result<Vec<IndexStats>> {
        let pool = self.connection_service.get_pool(conn_id)?;
        let client = pool.get().await?;

        let rows = client
            .query(
                r#"
                SELECT
                    s.schemaname,
                    s.relname as tablename,
                    s.indexrelname as indexname,
                    pg_relation_size(s.indexrelid) as index_size,
                    s.idx_scan,
                    s.idx_tup_read,
                    s.idx_tup_fetch,
                    pg_get_indexdef(s.indexrelid) as index_def,
                    i.indisunique,
                    i.indisprimary,
                    i.indisvalid,
                    am.amname as index_type
                FROM pg_stat_user_indexes s
                JOIN pg_index i ON s.indexrelid = i.indexrelid
                JOIN pg_am am ON (
                    SELECT relam FROM pg_class WHERE oid = s.indexrelid
                ) = am.oid
                ORDER BY s.idx_scan DESC
                "#,
                &[],
            )
            .await?;

        let stats = rows.iter().map(|row| row_to_index_stats(row)).collect();
        Ok(stats)
    }

    /// Get current locks
    pub async fn get_locks(&self, conn_id: &str) -> Result<Vec<LockInfo>> {
        let pool = self.connection_service.get_pool(conn_id)?;
        let client = pool.get().await?;

        let rows = client
            .query(
                r#"
                WITH lock_info AS (
                    SELECT
                        l.pid,
                        l.locktype,
                        l.database,
                        l.relation,
                        c.relname as relation_name,
                        n.nspname as schema_name,
                        l.mode,
                        l.granted,
                        l.transactionid::bigint as transaction_id,
                        a.query,
                        a.usename as username,
                        COALESCE(
                            (SELECT array_agg(bl.pid)
                             FROM pg_locks bl
                             WHERE bl.relation = l.relation
                               AND NOT bl.granted
                               AND l.granted),
                            ARRAY[]::int[]
                        ) as blocking_pids
                    FROM pg_locks l
                    LEFT JOIN pg_class c ON l.relation = c.oid
                    LEFT JOIN pg_namespace n ON c.relnamespace = n.oid
                    LEFT JOIN pg_stat_activity a ON l.pid = a.pid
                    WHERE l.pid != pg_backend_pid()
                )
                SELECT *,
                    array_length(blocking_pids, 1) > 0 as is_blocking
                FROM lock_info
                ORDER BY
                    NOT granted,  -- Waiting locks first
                    is_blocking DESC,  -- Then blocking locks
                    pid
                "#,
                &[],
            )
            .await?;

        let locks = rows.iter().map(|row| row_to_lock_info(row)).collect();
        Ok(locks)
    }
}

// Helper functions

fn format_interval(interval: &postgres_types::Interval) -> String {
    let total_seconds = interval.microseconds / 1_000_000;
    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

fn format_bytes(bytes: i64) -> String {
    const GB: i64 = 1_073_741_824;
    const MB: i64 = 1_048_576;
    const KB: i64 = 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn format_number(n: i64) -> String {
    const MILLION: i64 = 1_000_000;
    const THOUSAND: i64 = 1_000;

    if n >= MILLION {
        format!("{:.1}M", n as f64 / MILLION as f64)
    } else if n >= THOUSAND {
        format!("{:.1}K", n as f64 / THOUSAND as f64)
    } else {
        n.to_string()
    }
}

fn format_duration_ms(ms: i64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else if ms < 3_600_000 {
        format!("{:.1}m", ms as f64 / 60_000.0)
    } else {
        format!("{:.1}h", ms as f64 / 3_600_000.0)
    }
}

fn row_to_active_query(row: &Row) -> ActiveQuery {
    ActiveQuery {
        pid: row.get(0),
        database: row.get::<_, Option<String>>(1).unwrap_or_default(),
        user: row.get::<_, Option<String>>(2).unwrap_or_default(),
        application_name: row.get::<_, Option<String>>(3).unwrap_or_default(),
        client_addr: row.get(4),
        state: row.get::<_, Option<String>>(5).unwrap_or_else(|| "unknown".to_string()),
        wait_event_type: row.get(6),
        wait_event: row.get(7),
        query: row.get(8),
        query_start: row.get(9),
        duration_ms: row.get::<_, Option<f64>>(10).map(|f| f as i64),
        xact_start: row.get(11),
        backend_start: row.get(12),
        state_change: row.get(13),
        backend_type: row.get::<_, Option<String>>(14).unwrap_or_default(),
    }
}

fn row_to_table_stats(row: &Row) -> TableStats {
    TableStats {
        schema_name: row.get(0),
        table_name: row.get(1),
        row_count_estimate: row.get::<_, Option<i64>>(2).unwrap_or(0),
        total_size_bytes: row.get::<_, Option<i64>>(3).unwrap_or(0),
        table_size_bytes: row.get::<_, Option<i64>>(4).unwrap_or(0),
        indexes_size_bytes: row.get::<_, Option<i64>>(5).unwrap_or(0),
        toast_size_bytes: row.get::<_, Option<i64>>(6).unwrap_or(0),
        seq_scans: row.get::<_, Option<i64>>(7).unwrap_or(0),
        seq_tup_read: row.get::<_, Option<i64>>(8).unwrap_or(0),
        idx_scans: row.get::<_, Option<i64>>(9).unwrap_or(0),
        idx_tup_fetch: row.get::<_, Option<i64>>(10).unwrap_or(0),
        n_tup_ins: row.get::<_, Option<i64>>(11).unwrap_or(0),
        n_tup_upd: row.get::<_, Option<i64>>(12).unwrap_or(0),
        n_tup_del: row.get::<_, Option<i64>>(13).unwrap_or(0),
        n_tup_hot_upd: row.get::<_, Option<i64>>(14).unwrap_or(0),
        live_row_count: row.get::<_, Option<i64>>(15).unwrap_or(0),
        dead_row_count: row.get::<_, Option<i64>>(16).unwrap_or(0),
        last_vacuum: row.get(17),
        last_autovacuum: row.get(18),
        last_analyze: row.get(19),
        last_autoanalyze: row.get(20),
        vacuum_count: row.get::<_, Option<i64>>(21).unwrap_or(0),
        autovacuum_count: row.get::<_, Option<i64>>(22).unwrap_or(0),
        analyze_count: row.get::<_, Option<i64>>(23).unwrap_or(0),
        autoanalyze_count: row.get::<_, Option<i64>>(24).unwrap_or(0),
    }
}

fn row_to_index_stats(row: &Row) -> IndexStats {
    IndexStats {
        schema_name: row.get(0),
        table_name: row.get(1),
        index_name: row.get(2),
        index_size_bytes: row.get::<_, Option<i64>>(3).unwrap_or(0),
        idx_scan: row.get::<_, Option<i64>>(4).unwrap_or(0),
        idx_tup_read: row.get::<_, Option<i64>>(5).unwrap_or(0),
        idx_tup_fetch: row.get::<_, Option<i64>>(6).unwrap_or(0),
        index_def: row.get::<_, Option<String>>(7).unwrap_or_default(),
        is_unique: row.get::<_, Option<bool>>(8).unwrap_or(false),
        is_primary: row.get::<_, Option<bool>>(9).unwrap_or(false),
        is_valid: row.get::<_, Option<bool>>(10).unwrap_or(true),
        index_type: row.get::<_, Option<String>>(11).unwrap_or_else(|| "btree".to_string()),
    }
}

fn row_to_lock_info(row: &Row) -> LockInfo {
    LockInfo {
        pid: row.get(0),
        locktype: row.get(1),
        database: row.get(2),
        relation: row.get(3),
        relation_name: row.get(4),
        schema_name: row.get(5),
        mode: row.get(6),
        granted: row.get(7),
        transaction_id: row.get(8),
        query: row.get(9),
        user: row.get::<_, Option<String>>(10).unwrap_or_default(),
        blocking_pids: row.get::<_, Option<Vec<i32>>>(11).unwrap_or_default(),
        is_blocking: row.get::<_, Option<bool>>(12).unwrap_or(false),
    }
}
```

## 20.3 Admin State (Global)

```rust
// src/state/admin_state.rs

use crate::models::admin::*;
use crate::services::admin::AdminService;
use crate::services::connection::ConnectionService;
use gpui::Global;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

/// Key for identifying admin dashboard instances
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct AdminKey {
    pub conn_id: String,
}

/// State for a single admin dashboard instance
pub struct AdminInstance {
    pub server_stats: Option<ServerStats>,
    pub active_queries: Vec<ActiveQuery>,
    pub table_stats: Vec<TableStats>,
    pub index_stats: Vec<IndexStats>,
    pub locks: Vec<LockInfo>,
    pub active_tab: AdminTab,
    pub loading: bool,
    pub error: Option<String>,
    pub auto_refresh: bool,
    pub refresh_interval_secs: u64,
    pub table_filter: String,
    pub table_sort_column: TableSortColumn,
    pub table_sort_direction: SortDirection,
    pub index_filter: String,
    pub index_sort_column: IndexSortColumn,
    pub index_sort_direction: SortDirection,
    pub show_only_waiting_locks: bool,
    /// Channel to stop auto-refresh task
    pub refresh_stop_tx: Option<mpsc::Sender<()>>,
}

impl Default for AdminInstance {
    fn default() -> Self {
        Self {
            server_stats: None,
            active_queries: Vec::new(),
            table_stats: Vec::new(),
            index_stats: Vec::new(),
            locks: Vec::new(),
            active_tab: AdminTab::Activity,
            loading: false,
            error: None,
            auto_refresh: true,
            refresh_interval_secs: 5,
            table_filter: String::new(),
            table_sort_column: TableSortColumn::TotalSize,
            table_sort_direction: SortDirection::Descending,
            index_filter: String::new(),
            index_sort_column: IndexSortColumn::Scans,
            index_sort_direction: SortDirection::Descending,
            show_only_waiting_locks: false,
            refresh_stop_tx: None,
        }
    }
}

/// Application-wide admin state
pub struct AdminState {
    admin_service: Arc<AdminService>,
    instances: RwLock<HashMap<AdminKey, AdminInstance>>,
    runtime: Handle,
}

impl Global for AdminState {}

impl AdminState {
    pub fn new(connection_service: Arc<ConnectionService>, runtime: Handle) -> Self {
        let admin_service = Arc::new(AdminService::new(connection_service));
        Self {
            admin_service,
            instances: RwLock::new(HashMap::new()),
            runtime,
        }
    }

    /// Initialize admin dashboard for a connection
    pub fn init_dashboard(&self, conn_id: &str) {
        let key = AdminKey {
            conn_id: conn_id.to_string(),
        };
        let mut instances = self.instances.write();
        if !instances.contains_key(&key) {
            instances.insert(key, AdminInstance::default());
        }
    }

    /// Get instance for a connection
    pub fn get_instance(&self, conn_id: &str) -> Option<AdminInstance> {
        let key = AdminKey {
            conn_id: conn_id.to_string(),
        };
        self.instances.read().get(&key).cloned()
    }

    /// Update instance
    fn update_instance<F>(&self, conn_id: &str, f: F)
    where
        F: FnOnce(&mut AdminInstance),
    {
        let key = AdminKey {
            conn_id: conn_id.to_string(),
        };
        if let Some(instance) = self.instances.write().get_mut(&key) {
            f(instance);
        }
    }

    /// Set active tab
    pub fn set_active_tab(&self, conn_id: &str, tab: AdminTab) {
        self.update_instance(conn_id, |instance| {
            instance.active_tab = tab;
        });
    }

    /// Set auto-refresh state
    pub fn set_auto_refresh(&self, conn_id: &str, enabled: bool) {
        self.update_instance(conn_id, |instance| {
            instance.auto_refresh = enabled;
            if !enabled {
                // Stop existing refresh task
                if let Some(tx) = instance.refresh_stop_tx.take() {
                    let _ = tx.blocking_send(());
                }
            }
        });
    }

    /// Set refresh interval
    pub fn set_refresh_interval(&self, conn_id: &str, seconds: u64) {
        self.update_instance(conn_id, |instance| {
            instance.refresh_interval_secs = seconds;
        });
    }

    /// Set table filter
    pub fn set_table_filter(&self, conn_id: &str, filter: String) {
        self.update_instance(conn_id, |instance| {
            instance.table_filter = filter;
        });
    }

    /// Toggle table sort column
    pub fn toggle_table_sort(&self, conn_id: &str, column: TableSortColumn) {
        self.update_instance(conn_id, |instance| {
            if instance.table_sort_column == column {
                instance.table_sort_direction = instance.table_sort_direction.toggle();
            } else {
                instance.table_sort_column = column;
                instance.table_sort_direction = SortDirection::Descending;
            }
        });
    }

    /// Set index filter
    pub fn set_index_filter(&self, conn_id: &str, filter: String) {
        self.update_instance(conn_id, |instance| {
            instance.index_filter = filter;
        });
    }

    /// Toggle index sort column
    pub fn toggle_index_sort(&self, conn_id: &str, column: IndexSortColumn) {
        self.update_instance(conn_id, |instance| {
            if instance.index_sort_column == column {
                instance.index_sort_direction = instance.index_sort_direction.toggle();
            } else {
                instance.index_sort_column = column;
                instance.index_sort_direction = SortDirection::Descending;
            }
        });
    }

    /// Toggle show only waiting locks
    pub fn toggle_waiting_locks_filter(&self, conn_id: &str) {
        self.update_instance(conn_id, |instance| {
            instance.show_only_waiting_locks = !instance.show_only_waiting_locks;
        });
    }

    /// Refresh all data for a connection
    pub async fn refresh(&self, conn_id: &str) -> Result<(), String> {
        let key = AdminKey {
            conn_id: conn_id.to_string(),
        };

        // Set loading state
        {
            let mut instances = self.instances.write();
            if let Some(instance) = instances.get_mut(&key) {
                instance.loading = true;
                instance.error = None;
            }
        }

        // Fetch all data concurrently
        let service = self.admin_service.clone();
        let conn_id_owned = conn_id.to_string();

        let (stats_result, queries_result, tables_result, indexes_result, locks_result) = tokio::join!(
            service.get_server_stats(&conn_id_owned),
            service.get_active_queries(&conn_id_owned),
            service.get_table_stats(&conn_id_owned),
            service.get_index_stats(&conn_id_owned),
            service.get_locks(&conn_id_owned),
        );

        // Update state with results
        let mut instances = self.instances.write();
        if let Some(instance) = instances.get_mut(&key) {
            instance.loading = false;

            // Handle errors - collect all errors
            let mut errors = Vec::new();

            match stats_result {
                Ok(stats) => instance.server_stats = Some(stats),
                Err(e) => errors.push(format!("Server stats: {}", e)),
            }

            match queries_result {
                Ok(queries) => instance.active_queries = queries,
                Err(e) => errors.push(format!("Active queries: {}", e)),
            }

            match tables_result {
                Ok(tables) => instance.table_stats = tables,
                Err(e) => errors.push(format!("Table stats: {}", e)),
            }

            match indexes_result {
                Ok(indexes) => instance.index_stats = indexes,
                Err(e) => errors.push(format!("Index stats: {}", e)),
            }

            match locks_result {
                Ok(locks) => instance.locks = locks,
                Err(e) => errors.push(format!("Locks: {}", e)),
            }

            if !errors.is_empty() {
                instance.error = Some(errors.join("; "));
            }
        }

        Ok(())
    }

    /// Cancel a query
    pub async fn cancel_query(&self, conn_id: &str, pid: i32) -> Result<bool, String> {
        self.admin_service
            .cancel_query(conn_id, pid)
            .await
            .map_err(|e| e.to_string())
    }

    /// Terminate a connection
    pub async fn terminate_connection(&self, conn_id: &str, pid: i32) -> Result<bool, String> {
        self.admin_service
            .terminate_connection(conn_id, pid)
            .await
            .map_err(|e| e.to_string())
    }

    /// Start auto-refresh task
    pub fn start_auto_refresh(
        &self,
        conn_id: &str,
        callback: impl Fn() + Send + 'static,
    ) {
        let key = AdminKey {
            conn_id: conn_id.to_string(),
        };

        // Stop existing task if any
        {
            let mut instances = self.instances.write();
            if let Some(instance) = instances.get_mut(&key) {
                if let Some(tx) = instance.refresh_stop_tx.take() {
                    let _ = tx.blocking_send(());
                }
            }
        }

        // Get refresh interval
        let interval_secs = {
            let instances = self.instances.read();
            instances
                .get(&key)
                .map(|i| i.refresh_interval_secs)
                .unwrap_or(5)
        };

        // Create stop channel
        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);

        // Store stop channel
        {
            let mut instances = self.instances.write();
            if let Some(instance) = instances.get_mut(&key) {
                instance.refresh_stop_tx = Some(stop_tx);
            }
        }

        // Spawn refresh task
        let admin_state = self.admin_service.clone();
        let conn_id_owned = conn_id.to_string();

        self.runtime.spawn(async move {
            let mut tick = interval(Duration::from_secs(interval_secs));

            loop {
                tokio::select! {
                    _ = tick.tick() => {
                        callback();
                    }
                    _ = stop_rx.recv() => {
                        break;
                    }
                }
            }
        });
    }

    /// Stop auto-refresh task
    pub fn stop_auto_refresh(&self, conn_id: &str) {
        let key = AdminKey {
            conn_id: conn_id.to_string(),
        };
        let mut instances = self.instances.write();
        if let Some(instance) = instances.get_mut(&key) {
            if let Some(tx) = instance.refresh_stop_tx.take() {
                let _ = tx.blocking_send(());
            }
        }
    }

    /// Cleanup dashboard for a connection
    pub fn cleanup(&self, conn_id: &str) {
        self.stop_auto_refresh(conn_id);
        let key = AdminKey {
            conn_id: conn_id.to_string(),
        };
        self.instances.write().remove(&key);
    }

    /// Get filtered and sorted table stats
    pub fn get_filtered_table_stats(&self, conn_id: &str) -> Vec<TableStats> {
        let key = AdminKey {
            conn_id: conn_id.to_string(),
        };
        let instances = self.instances.read();
        let Some(instance) = instances.get(&key) else {
            return Vec::new();
        };

        let filter = instance.table_filter.to_lowercase();
        let mut stats: Vec<_> = instance
            .table_stats
            .iter()
            .filter(|s| {
                filter.is_empty()
                    || s.table_name.to_lowercase().contains(&filter)
                    || s.schema_name.to_lowercase().contains(&filter)
            })
            .cloned()
            .collect();

        // Sort
        let direction = instance.table_sort_direction;
        match instance.table_sort_column {
            TableSortColumn::Name => stats.sort_by(|a, b| {
                let cmp = a.table_name.cmp(&b.table_name);
                if direction == SortDirection::Ascending { cmp } else { cmp.reverse() }
            }),
            TableSortColumn::RowCount => stats.sort_by(|a, b| {
                let cmp = a.row_count_estimate.cmp(&b.row_count_estimate);
                if direction == SortDirection::Ascending { cmp } else { cmp.reverse() }
            }),
            TableSortColumn::TotalSize => stats.sort_by(|a, b| {
                let cmp = a.total_size_bytes.cmp(&b.total_size_bytes);
                if direction == SortDirection::Ascending { cmp } else { cmp.reverse() }
            }),
            TableSortColumn::SeqScans => stats.sort_by(|a, b| {
                let cmp = a.seq_scans.cmp(&b.seq_scans);
                if direction == SortDirection::Ascending { cmp } else { cmp.reverse() }
            }),
            TableSortColumn::IdxScans => stats.sort_by(|a, b| {
                let cmp = a.idx_scans.cmp(&b.idx_scans);
                if direction == SortDirection::Ascending { cmp } else { cmp.reverse() }
            }),
            TableSortColumn::DeadRows => stats.sort_by(|a, b| {
                let cmp = a.dead_row_count.cmp(&b.dead_row_count);
                if direction == SortDirection::Ascending { cmp } else { cmp.reverse() }
            }),
            TableSortColumn::LastVacuum => stats.sort_by(|a, b| {
                let a_time = a.last_autovacuum.or(a.last_vacuum);
                let b_time = b.last_autovacuum.or(b.last_vacuum);
                let cmp = a_time.cmp(&b_time);
                if direction == SortDirection::Ascending { cmp } else { cmp.reverse() }
            }),
        }

        stats
    }

    /// Get filtered and sorted index stats
    pub fn get_filtered_index_stats(&self, conn_id: &str) -> Vec<IndexStats> {
        let key = AdminKey {
            conn_id: conn_id.to_string(),
        };
        let instances = self.instances.read();
        let Some(instance) = instances.get(&key) else {
            return Vec::new();
        };

        let filter = instance.index_filter.to_lowercase();
        let mut stats: Vec<_> = instance
            .index_stats
            .iter()
            .filter(|s| {
                filter.is_empty()
                    || s.index_name.to_lowercase().contains(&filter)
                    || s.table_name.to_lowercase().contains(&filter)
            })
            .cloned()
            .collect();

        // Sort
        let direction = instance.index_sort_direction;
        match instance.index_sort_column {
            IndexSortColumn::Name => stats.sort_by(|a, b| {
                let cmp = a.index_name.cmp(&b.index_name);
                if direction == SortDirection::Ascending { cmp } else { cmp.reverse() }
            }),
            IndexSortColumn::Table => stats.sort_by(|a, b| {
                let cmp = a.table_name.cmp(&b.table_name);
                if direction == SortDirection::Ascending { cmp } else { cmp.reverse() }
            }),
            IndexSortColumn::Size => stats.sort_by(|a, b| {
                let cmp = a.index_size_bytes.cmp(&b.index_size_bytes);
                if direction == SortDirection::Ascending { cmp } else { cmp.reverse() }
            }),
            IndexSortColumn::Scans => stats.sort_by(|a, b| {
                let cmp = a.idx_scan.cmp(&b.idx_scan);
                if direction == SortDirection::Ascending { cmp } else { cmp.reverse() }
            }),
            IndexSortColumn::Reads => stats.sort_by(|a, b| {
                let cmp = a.idx_tup_read.cmp(&b.idx_tup_read);
                if direction == SortDirection::Ascending { cmp } else { cmp.reverse() }
            }),
            IndexSortColumn::Type => stats.sort_by(|a, b| {
                let cmp = a.index_type.cmp(&b.index_type);
                if direction == SortDirection::Ascending { cmp } else { cmp.reverse() }
            }),
        }

        stats
    }

    /// Get filtered locks
    pub fn get_filtered_locks(&self, conn_id: &str) -> Vec<LockInfo> {
        let key = AdminKey {
            conn_id: conn_id.to_string(),
        };
        let instances = self.instances.read();
        let Some(instance) = instances.get(&key) else {
            return Vec::new();
        };

        if instance.show_only_waiting_locks {
            instance
                .locks
                .iter()
                .filter(|l| !l.granted || l.is_blocking)
                .cloned()
                .collect()
        } else {
            instance.locks.clone()
        }
    }
}

impl Clone for AdminInstance {
    fn clone(&self) -> Self {
        Self {
            server_stats: self.server_stats.clone(),
            active_queries: self.active_queries.clone(),
            table_stats: self.table_stats.clone(),
            index_stats: self.index_stats.clone(),
            locks: self.locks.clone(),
            active_tab: self.active_tab,
            loading: self.loading,
            error: self.error.clone(),
            auto_refresh: self.auto_refresh,
            refresh_interval_secs: self.refresh_interval_secs,
            table_filter: self.table_filter.clone(),
            table_sort_column: self.table_sort_column,
            table_sort_direction: self.table_sort_direction,
            index_filter: self.index_filter.clone(),
            index_sort_column: self.index_sort_column,
            index_sort_direction: self.index_sort_direction,
            show_only_waiting_locks: self.show_only_waiting_locks,
            refresh_stop_tx: None, // Don't clone the channel
        }
    }
}
```

## 20.4 Admin Dashboard Component

```rust
// src/components/admin/admin_dashboard.rs

use crate::components::admin::*;
use crate::models::admin::*;
use crate::state::admin_state::AdminState;
use crate::theme::Theme;
use gpui::*;

/// Main admin dashboard component
pub struct AdminDashboard {
    conn_id: String,
    show_cancel_dialog: Option<i32>,
    show_terminate_dialog: Option<i32>,
}

impl AdminDashboard {
    pub fn new(conn_id: String, cx: &mut Context<Self>) -> Self {
        // Initialize dashboard state
        cx.global::<AdminState>().init_dashboard(&conn_id);

        // Trigger initial refresh
        let conn_id_clone = conn_id.clone();
        cx.spawn(|this, mut cx| async move {
            if let Some(state) = cx.update(|cx| cx.global::<AdminState>().clone()).ok() {
                let _ = state.refresh(&conn_id_clone).await;
                let _ = cx.update(|cx| cx.notify());
            }
        })
        .detach();

        Self {
            conn_id,
            show_cancel_dialog: None,
            show_terminate_dialog: None,
        }
    }

    fn on_tab_click(&mut self, tab: AdminTab, cx: &mut Context<Self>) {
        cx.global::<AdminState>().set_active_tab(&self.conn_id, tab);
        cx.notify();
    }

    fn on_auto_refresh_toggle(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let state = cx.global::<AdminState>();
        state.set_auto_refresh(&self.conn_id, enabled);

        if enabled {
            let conn_id = self.conn_id.clone();
            let handle = cx.view().downgrade();
            state.start_auto_refresh(&self.conn_id, move || {
                if let Some(view) = handle.upgrade() {
                    let _ = view.update(&mut cx, |this, cx| {
                        this.refresh(cx);
                    });
                }
            });
        } else {
            state.stop_auto_refresh(&self.conn_id);
        }
        cx.notify();
    }

    fn on_refresh_interval_change(&mut self, seconds: u64, cx: &mut Context<Self>) {
        cx.global::<AdminState>().set_refresh_interval(&self.conn_id, seconds);
        cx.notify();
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let conn_id = self.conn_id.clone();
        cx.spawn(|this, mut cx| async move {
            if let Some(state) = cx.update(|cx| cx.global::<AdminState>().clone()).ok() {
                let _ = state.refresh(&conn_id).await;
                let _ = cx.update(|cx| cx.notify());
            }
        })
        .detach();
    }

    fn show_cancel_dialog(&mut self, pid: i32, cx: &mut Context<Self>) {
        self.show_cancel_dialog = Some(pid);
        cx.notify();
    }

    fn show_terminate_dialog(&mut self, pid: i32, cx: &mut Context<Self>) {
        self.show_terminate_dialog = Some(pid);
        cx.notify();
    }

    fn cancel_query(&mut self, pid: i32, cx: &mut Context<Self>) {
        self.show_cancel_dialog = None;
        let conn_id = self.conn_id.clone();
        cx.spawn(|this, mut cx| async move {
            if let Some(state) = cx.update(|cx| cx.global::<AdminState>().clone()).ok() {
                if let Ok(true) = state.cancel_query(&conn_id, pid).await {
                    let _ = state.refresh(&conn_id).await;
                }
                let _ = cx.update(|cx| cx.notify());
            }
        })
        .detach();
    }

    fn terminate_connection(&mut self, pid: i32, cx: &mut Context<Self>) {
        self.show_terminate_dialog = None;
        let conn_id = self.conn_id.clone();
        cx.spawn(|this, mut cx| async move {
            if let Some(state) = cx.update(|cx| cx.global::<AdminState>().clone()).ok() {
                if let Ok(true) = state.terminate_connection(&conn_id, pid).await {
                    let _ = state.refresh(&conn_id).await;
                }
                let _ = cx.update(|cx| cx.notify());
            }
        })
        .detach();
    }

    fn on_table_action(&mut self, action: &str, schema: &str, table: &str, cx: &mut Context<Self>) {
        // Emit event for maintenance dialog
        cx.emit(AdminEvent::MaintenanceRequested {
            action: action.to_string(),
            schema: schema.to_string(),
            table: table.to_string(),
        });
    }
}

/// Events emitted by admin dashboard
pub enum AdminEvent {
    MaintenanceRequested {
        action: String,
        schema: String,
        table: String,
    },
}

impl EventEmitter<AdminEvent> for AdminDashboard {}

impl Render for AdminDashboard {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let state = cx.global::<AdminState>();
        let instance = state.get_instance(&self.conn_id).unwrap_or_default();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .child(self.render_header(&instance, theme, cx))
            .when(instance.server_stats.is_some(), |this| {
                this.child(self.render_server_stats(
                    instance.server_stats.as_ref().unwrap(),
                    theme,
                    cx,
                ))
            })
            .child(self.render_tabs(&instance, theme, cx))
            .child(self.render_content(&instance, theme, cx))
            .when(self.show_cancel_dialog.is_some(), |this| {
                let pid = self.show_cancel_dialog.unwrap();
                let query = instance
                    .active_queries
                    .iter()
                    .find(|q| q.pid == pid);
                this.child(self.render_cancel_dialog(pid, query, theme, cx))
            })
            .when(self.show_terminate_dialog.is_some(), |this| {
                let pid = self.show_terminate_dialog.unwrap();
                let query = instance
                    .active_queries
                    .iter()
                    .find(|q| q.pid == pid);
                this.child(self.render_terminate_dialog(pid, query, theme, cx))
            })
    }
}

impl AdminDashboard {
    fn render_header(
        &self,
        instance: &AdminInstance,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text)
                    .child("Admin Dashboard"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    // Auto-refresh toggle
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Checkbox::new(instance.auto_refresh)
                                    .on_toggle(cx.listener(|this, checked, cx| {
                                        this.on_auto_refresh_toggle(checked, cx);
                                    })),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.text_muted)
                                    .child("Auto-refresh"),
                            ),
                    )
                    // Refresh interval dropdown
                    .child(
                        Select::new(instance.refresh_interval_secs.to_string())
                            .options(vec![
                                ("1", "1s"),
                                ("5", "5s"),
                                ("10", "10s"),
                                ("30", "30s"),
                                ("60", "60s"),
                            ])
                            .disabled(!instance.auto_refresh)
                            .on_change(cx.listener(|this, value: String, cx| {
                                if let Ok(secs) = value.parse::<u64>() {
                                    this.on_refresh_interval_change(secs, cx);
                                }
                            })),
                    )
                    // Manual refresh button
                    .child(
                        Button::new("refresh")
                            .label("Refresh")
                            .icon(Icon::Refresh)
                            .loading(instance.loading)
                            .on_click(cx.listener(|this, _, cx| {
                                this.refresh(cx);
                            })),
                    ),
            )
    }

    fn render_server_stats(
        &self,
        stats: &ServerStats,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let connection_percent = (stats.connection_count as f64 / stats.max_connections as f64) * 100.0;
        let connection_color = if connection_percent >= 90.0 {
            theme.error
        } else if connection_percent >= 70.0 {
            theme.warning
        } else {
            theme.success
        };

        let cache_hit_color = if stats.cache_hit_ratio >= 99.0 {
            theme.success
        } else if stats.cache_hit_ratio >= 95.0 {
            theme.warning
        } else {
            theme.error
        };

        div()
            .px_4()
            .py_4()
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .gap_4()
                    // Connections card
                    .child(
                        self.stat_card(
                            "Connections",
                            &format!("{} / {}", stats.connection_count, stats.max_connections),
                            &format!("{} active queries", stats.active_queries),
                            Some((connection_percent, connection_color)),
                            theme,
                        ),
                    )
                    // Cache hit ratio card
                    .child(
                        self.stat_card(
                            "Cache Hit Ratio",
                            &format!("{:.2}%", stats.cache_hit_ratio),
                            "Target: > 99%",
                            None,
                            theme,
                        )
                        .child(
                            div()
                                .text_2xl()
                                .font_weight(FontWeight::BOLD)
                                .text_color(cache_hit_color),
                        ),
                    )
                    // TPS card
                    .child(
                        self.stat_card(
                            "Transactions/sec",
                            &format!("{:.1}", stats.transactions_per_second),
                            "Average since stats reset",
                            None,
                            theme,
                        ),
                    )
                    // Uptime card
                    .child(
                        self.stat_card(
                            "Uptime",
                            &stats.uptime,
                            &format!(
                                "PostgreSQL {}",
                                stats.version.split(' ').nth(1).unwrap_or("")
                            ),
                            None,
                            theme,
                        ),
                    ),
            )
            // Database sizes
            .child(
                div()
                    .mt_4()
                    .p_4()
                    .bg(theme.surface)
                    .rounded_lg()
                    .shadow_sm()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text_muted)
                            .mb_3()
                            .child("Database Sizes"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .children(stats.database_sizes.iter().map(|db| {
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.text)
                                            .child(db.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_family("monospace")
                                            .text_color(theme.text_muted)
                                            .child(db.size_formatted.clone()),
                                    )
                            })),
                    ),
            )
    }

    fn stat_card(
        &self,
        title: &str,
        value: &str,
        subtitle: &str,
        progress: Option<(f64, Hsla)>,
        theme: &Theme,
    ) -> Div {
        let mut card = div()
            .flex_1()
            .p_4()
            .bg(theme.surface)
            .rounded_lg()
            .shadow_sm()
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .mb_1()
                    .child(title.to_string()),
            )
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text)
                    .child(value.to_string()),
            );

        if let Some((percent, color)) = progress {
            card = card.child(
                div()
                    .mt_2()
                    .h_2()
                    .bg(theme.surface_hover)
                    .rounded_full()
                    .overflow_hidden()
                    .child(
                        div()
                            .h_full()
                            .bg(color)
                            .w(relative(percent / 100.0)),
                    ),
            );
        }

        card.child(
            div()
                .text_xs()
                .text_color(theme.text_muted)
                .mt_1()
                .child(subtitle.to_string()),
        )
    }

    fn render_tabs(
        &self,
        instance: &AdminInstance,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .border_b_1()
            .border_color(theme.border)
            .children(AdminTab::all().iter().map(|tab| {
                let is_active = instance.active_tab == *tab;
                let tab_value = *tab;

                div()
                    .px_4()
                    .py_3()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_pointer()
                    .when(is_active, |this| {
                        this.border_b_2()
                            .border_color(theme.primary)
                            .text_color(theme.primary)
                    })
                    .when(!is_active, |this| {
                        this.text_color(theme.text_muted)
                            .hover(|this| this.text_color(theme.text))
                    })
                    .on_click(cx.listener(move |this, _, cx| {
                        this.on_tab_click(tab_value, cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(tab.icon()))
                            .child(tab.label()),
                    )
            }))
    }

    fn render_content(
        &self,
        instance: &AdminInstance,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let content = div()
            .flex_1()
            .overflow_auto()
            .p_4();

        if let Some(error) = &instance.error {
            return content.child(
                div()
                    .p_4()
                    .bg(theme.error_bg)
                    .border_1()
                    .border_color(theme.error)
                    .rounded_md()
                    .text_color(theme.error)
                    .child(error.clone()),
            );
        }

        match instance.active_tab {
            AdminTab::Activity => content.child(
                ActiveQueriesView::new(
                    instance.active_queries.clone(),
                    cx.listener(|this, pid, cx| this.show_cancel_dialog(pid, cx)),
                    cx.listener(|this, pid, cx| this.show_terminate_dialog(pid, cx)),
                ),
            ),
            AdminTab::Tables => {
                let state = cx.global::<AdminState>();
                let filtered = state.get_filtered_table_stats(&self.conn_id);
                content.child(TableStatsView::new(
                    filtered,
                    instance.table_filter.clone(),
                    instance.table_sort_column,
                    instance.table_sort_direction,
                    self.conn_id.clone(),
                    cx,
                ))
            }
            AdminTab::Indexes => {
                let state = cx.global::<AdminState>();
                let filtered = state.get_filtered_index_stats(&self.conn_id);
                content.child(IndexStatsView::new(
                    filtered,
                    instance.index_filter.clone(),
                    instance.index_sort_column,
                    instance.index_sort_direction,
                    self.conn_id.clone(),
                    cx,
                ))
            }
            AdminTab::Locks => {
                let state = cx.global::<AdminState>();
                let filtered = state.get_filtered_locks(&self.conn_id);
                content.child(LocksView::new(
                    filtered,
                    instance.show_only_waiting_locks,
                    self.conn_id.clone(),
                    cx,
                ))
            }
        }
    }

    fn render_cancel_dialog(
        &self,
        pid: i32,
        query: Option<&ActiveQuery>,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let query_text = query.and_then(|q| q.query.clone()).unwrap_or_default();
        let user = query.map(|q| q.user.clone()).unwrap_or_default();

        Modal::new("cancel-query")
            .title("Cancel Query?")
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child(format!(
                                "This will cancel the running query for PID {} ({}). The connection will remain open.",
                                pid, user
                            )),
                    )
                    .child(
                        div()
                            .p_3()
                            .bg(theme.surface_hover)
                            .rounded_md()
                            .font_family("monospace")
                            .text_xs()
                            .max_h_32()
                            .overflow_auto()
                            .child(query_text),
                    ),
            )
            .actions(vec![
                Button::new("cancel")
                    .label("Cancel")
                    .variant(ButtonVariant::Ghost)
                    .on_click(cx.listener(|this, _, cx| {
                        this.show_cancel_dialog = None;
                        cx.notify();
                    })),
                Button::new("confirm")
                    .label("Cancel Query")
                    .variant(ButtonVariant::Warning)
                    .on_click(cx.listener(move |this, _, cx| {
                        this.cancel_query(pid, cx);
                    })),
            ])
    }

    fn render_terminate_dialog(
        &self,
        pid: i32,
        query: Option<&ActiveQuery>,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let query_text = query
            .and_then(|q| q.query.clone())
            .unwrap_or_else(|| "(no active query)".to_string());
        let user = query.map(|q| q.user.clone()).unwrap_or_default();

        Modal::new("terminate-connection")
            .title("Terminate Connection?")
            .title_color(theme.error)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child(format!(
                                "This will forcefully terminate the connection for PID {} ({}). Any uncommitted transactions will be rolled back.",
                                pid, user
                            )),
                    )
                    .child(
                        div()
                            .p_3()
                            .bg(theme.surface_hover)
                            .rounded_md()
                            .font_family("monospace")
                            .text_xs()
                            .max_h_32()
                            .overflow_auto()
                            .child(query_text),
                    ),
            )
            .actions(vec![
                Button::new("cancel")
                    .label("Cancel")
                    .variant(ButtonVariant::Ghost)
                    .on_click(cx.listener(|this, _, cx| {
                        this.show_terminate_dialog = None;
                        cx.notify();
                    })),
                Button::new("confirm")
                    .label("Terminate")
                    .variant(ButtonVariant::Danger)
                    .on_click(cx.listener(move |this, _, cx| {
                        this.terminate_connection(pid, cx);
                    })),
            ])
    }
}
```

## 20.5 Active Queries View

```rust
// src/components/admin/active_queries_view.rs

use crate::models::admin::ActiveQuery;
use crate::theme::Theme;
use gpui::*;

/// View for displaying active connections/queries
pub struct ActiveQueriesView {
    queries: Vec<ActiveQuery>,
    on_cancel: Box<dyn Fn(i32, &mut WindowContext)>,
    on_terminate: Box<dyn Fn(i32, &mut WindowContext)>,
}

impl ActiveQueriesView {
    pub fn new(
        queries: Vec<ActiveQuery>,
        on_cancel: impl Fn(i32, &mut WindowContext) + 'static,
        on_terminate: impl Fn(i32, &mut WindowContext) + 'static,
    ) -> Self {
        Self {
            queries,
            on_cancel: Box::new(on_cancel),
            on_terminate: Box::new(on_terminate),
        }
    }

    fn format_duration(ms: Option<i64>) -> String {
        match ms {
            None => "-".to_string(),
            Some(ms) if ms < 1000 => format!("{}ms", ms),
            Some(ms) if ms < 60_000 => format!("{:.1}s", ms as f64 / 1000.0),
            Some(ms) if ms < 3_600_000 => format!("{:.1}m", ms as f64 / 60_000.0),
            Some(ms) => format!("{:.1}h", ms as f64 / 3_600_000.0),
        }
    }

    fn get_state_color(state: &str, theme: &Theme) -> Hsla {
        match state {
            "active" => theme.success,
            "idle" => theme.text_muted,
            "idle in transaction" => theme.warning,
            "idle in transaction (aborted)" => theme.error,
            _ => theme.text_muted,
        }
    }

    fn get_state_bg(state: &str, theme: &Theme) -> Hsla {
        match state {
            "active" => theme.success_bg,
            "idle" => theme.surface_hover,
            "idle in transaction" => theme.warning_bg,
            "idle in transaction (aborted)" => theme.error_bg,
            _ => theme.surface_hover,
        }
    }

    fn is_long_running(ms: Option<i64>) -> bool {
        ms.map(|ms| ms > 30_000).unwrap_or(false)
    }
}

impl Render for ActiveQueriesView {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .w_full()
            .bg(theme.surface)
            .rounded_lg()
            .shadow_sm()
            .overflow_hidden()
            .child(
                // Table header
                div()
                    .flex()
                    .bg(theme.surface_hover)
                    .px_4()
                    .py_3()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text_muted)
                    .child(div().w_16().child("PID"))
                    .child(div().w_24().child("User"))
                    .child(div().w_24().child("Database"))
                    .child(div().w_32().child("State"))
                    .child(div().w_20().child("Duration"))
                    .child(div().flex_1().child("Query"))
                    .child(div().w_24().text_right().child("Actions")),
            )
            .child(
                // Table body
                div()
                    .flex()
                    .flex_col()
                    .when(self.queries.is_empty(), |this| {
                        this.child(
                            div()
                                .px_4()
                                .py_8()
                                .text_center()
                                .text_color(theme.text_muted)
                                .child("No active connections"),
                        )
                    })
                    .when(!self.queries.is_empty(), |this| {
                        this.children(self.queries.iter().enumerate().map(|(i, query)| {
                            let pid = query.pid;
                            let is_active = query.state == "active";
                            let is_long = Self::is_long_running(query.duration_ms);

                            div()
                                .flex()
                                .items_center()
                                .px_4()
                                .py_3()
                                .border_t_1()
                                .border_color(theme.border)
                                .hover(|this| this.bg(theme.surface_hover))
                                // PID
                                .child(
                                    div()
                                        .w_16()
                                        .text_sm()
                                        .font_family("monospace")
                                        .text_color(theme.text)
                                        .child(query.pid.to_string()),
                                )
                                // User
                                .child(
                                    div()
                                        .w_24()
                                        .text_sm()
                                        .text_color(theme.text)
                                        .truncate()
                                        .child(query.user.clone()),
                                )
                                // Database
                                .child(
                                    div()
                                        .w_24()
                                        .text_sm()
                                        .text_color(theme.text)
                                        .truncate()
                                        .child(query.database.clone()),
                                )
                                // State
                                .child(
                                    div()
                                        .w_32()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            div()
                                                .px_2()
                                                .py_px()
                                                .rounded_sm()
                                                .text_xs()
                                                .font_weight(FontWeight::MEDIUM)
                                                .bg(Self::get_state_bg(&query.state, theme))
                                                .text_color(Self::get_state_color(&query.state, theme))
                                                .child(query.state.clone()),
                                        )
                                        .when(query.wait_event_type.is_some(), |this| {
                                            this.child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.text_muted)
                                                    .child(format!(
                                                        "({}: {})",
                                                        query.wait_event_type.as_deref().unwrap_or(""),
                                                        query.wait_event.as_deref().unwrap_or("")
                                                    )),
                                            )
                                        }),
                                )
                                // Duration
                                .child(
                                    div()
                                        .w_20()
                                        .text_sm()
                                        .font_family("monospace")
                                        .when(is_long, |this| {
                                            this.text_color(theme.error)
                                                .font_weight(FontWeight::BOLD)
                                        })
                                        .when(!is_long, |this| this.text_color(theme.text))
                                        .child(Self::format_duration(query.duration_ms))
                                        .when(is_long, |this| {
                                            this.child(
                                                div()
                                                    .ml_1()
                                                    .text_color(theme.warning)
                                                    .child("⚠"),
                                            )
                                        }),
                                )
                                // Query
                                .child(
                                    div()
                                        .flex_1()
                                        .text_sm()
                                        .font_family("monospace")
                                        .text_color(theme.text)
                                        .truncate()
                                        .overflow_hidden()
                                        .child(
                                            query.query.clone().unwrap_or_else(|| "-".to_string()),
                                        ),
                                )
                                // Actions
                                .child(
                                    div()
                                        .w_24()
                                        .flex()
                                        .items_center()
                                        .justify_end()
                                        .gap_2()
                                        .when(is_active, |this| {
                                            this.child(
                                                Button::new(format!("cancel-{}", pid))
                                                    .label("Cancel")
                                                    .size(ButtonSize::Small)
                                                    .variant(ButtonVariant::Warning)
                                                    .on_click(cx.listener(move |this, _, cx| {
                                                        (this.on_cancel)(pid, cx);
                                                    })),
                                            )
                                        })
                                        .child(
                                            Button::new(format!("kill-{}", pid))
                                                .label("Kill")
                                                .size(ButtonSize::Small)
                                                .variant(ButtonVariant::Danger)
                                                .on_click(cx.listener(move |this, _, cx| {
                                                    (this.on_terminate)(pid, cx);
                                                })),
                                        ),
                                )
                        }))
                    }),
            )
    }
}
```

## 20.6 Table Stats View

```rust
// src/components/admin/table_stats_view.rs

use crate::models::admin::{TableSortColumn, TableStats, SortDirection};
use crate::state::admin_state::AdminState;
use crate::theme::Theme;
use gpui::*;

/// View for displaying table statistics
pub struct TableStatsView {
    stats: Vec<TableStats>,
    filter: String,
    sort_column: TableSortColumn,
    sort_direction: SortDirection,
    conn_id: String,
}

impl TableStatsView {
    pub fn new(
        stats: Vec<TableStats>,
        filter: String,
        sort_column: TableSortColumn,
        sort_direction: SortDirection,
        conn_id: String,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            stats,
            filter,
            sort_column,
            sort_direction,
            conn_id,
        }
    }

    fn format_bytes(bytes: i64) -> String {
        const GB: i64 = 1_073_741_824;
        const MB: i64 = 1_048_576;
        const KB: i64 = 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    fn format_number(n: i64) -> String {
        const MILLION: i64 = 1_000_000;
        const THOUSAND: i64 = 1_000;

        if n >= MILLION {
            format!("{:.1}M", n as f64 / MILLION as f64)
        } else if n >= THOUSAND {
            format!("{:.1}K", n as f64 / THOUSAND as f64)
        } else {
            n.to_string()
        }
    }

    fn format_date(date: Option<chrono::DateTime<chrono::Utc>>) -> String {
        match date {
            None => "Never".to_string(),
            Some(dt) => {
                let now = chrono::Utc::now();
                let diff = now - dt;
                let hours = diff.num_hours();

                if hours < 1 {
                    "Just now".to_string()
                } else if hours < 24 {
                    format!("{}h ago", hours)
                } else {
                    let days = hours / 24;
                    if days < 7 {
                        format!("{}d ago", days)
                    } else {
                        dt.format("%Y-%m-%d").to_string()
                    }
                }
            }
        }
    }

    fn on_filter_change(&mut self, value: String, cx: &mut Context<Self>) {
        self.filter = value.clone();
        cx.global::<AdminState>().set_table_filter(&self.conn_id, value);
        cx.notify();
    }

    fn on_sort_click(&mut self, column: TableSortColumn, cx: &mut Context<Self>) {
        cx.global::<AdminState>().toggle_table_sort(&self.conn_id, column);
        cx.notify();
    }

    fn on_vacuum(&mut self, schema: &str, table: &str, cx: &mut Context<Self>) {
        // Emit event for maintenance dialog
        cx.emit(TableStatsEvent::VacuumRequested {
            schema: schema.to_string(),
            table: table.to_string(),
        });
    }

    fn on_analyze(&mut self, schema: &str, table: &str, cx: &mut Context<Self>) {
        cx.emit(TableStatsEvent::AnalyzeRequested {
            schema: schema.to_string(),
            table: table.to_string(),
        });
    }

    fn render_sort_indicator(&self, column: TableSortColumn, theme: &Theme) -> impl IntoElement {
        if self.sort_column == column {
            let arrow = if self.sort_direction == SortDirection::Ascending {
                "▲"
            } else {
                "▼"
            };
            div().text_color(theme.primary).child(arrow)
        } else {
            div()
        }
    }
}

pub enum TableStatsEvent {
    VacuumRequested { schema: String, table: String },
    AnalyzeRequested { schema: String, table: String },
}

impl EventEmitter<TableStatsEvent> for TableStatsView {}

impl Render for TableStatsView {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let total_count = self.stats.len();

        div()
            .flex()
            .flex_col()
            .gap_4()
            // Filter bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        TextInput::new("table-filter")
                            .placeholder("Filter tables...")
                            .value(self.filter.clone())
                            .w_64()
                            .on_change(cx.listener(|this, value, cx| {
                                this.on_filter_change(value, cx);
                            })),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child(format!("{} tables", total_count)),
                    ),
            )
            // Table
            .child(
                div()
                    .w_full()
                    .bg(theme.surface)
                    .rounded_lg()
                    .shadow_sm()
                    .overflow_x_auto()
                    // Header
                    .child(
                        div()
                            .flex()
                            .bg(theme.surface_hover)
                            .px_4()
                            .py_3()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text_muted)
                            .child(
                                self.sortable_header("Table", TableSortColumn::Name, theme, cx),
                            )
                            .child(
                                self.sortable_header("Rows", TableSortColumn::RowCount, theme, cx)
                                    .w_20()
                                    .text_right(),
                            )
                            .child(
                                self.sortable_header("Size", TableSortColumn::TotalSize, theme, cx)
                                    .w_24()
                                    .text_right(),
                            )
                            .child(
                                self.sortable_header("Seq Scans", TableSortColumn::SeqScans, theme, cx)
                                    .w_24()
                                    .text_right(),
                            )
                            .child(
                                self.sortable_header("Idx Scans", TableSortColumn::IdxScans, theme, cx)
                                    .w_24()
                                    .text_right(),
                            )
                            .child(
                                self.sortable_header("Dead Rows", TableSortColumn::DeadRows, theme, cx)
                                    .w_24()
                                    .text_right(),
                            )
                            .child(
                                self.sortable_header("Last Vacuum", TableSortColumn::LastVacuum, theme, cx)
                                    .w_28()
                                    .text_center(),
                            )
                            .child(div().w_32().text_right().child("Actions")),
                    )
                    // Body
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .children(self.stats.iter().map(|stat| {
                                let needs_vacuum = stat.needs_vacuum();
                                let needs_analyze = stat.needs_analyze();
                                let schema = stat.schema_name.clone();
                                let table = stat.table_name.clone();

                                div()
                                    .flex()
                                    .items_center()
                                    .px_4()
                                    .py_3()
                                    .border_t_1()
                                    .border_color(theme.border)
                                    .hover(|this| this.bg(theme.surface_hover))
                                    // Table name
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_sm()
                                            .child(
                                                div()
                                                    .flex()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_color(theme.text_muted)
                                                            .child(format!("{}.", stat.schema_name)),
                                                    )
                                                    .child(
                                                        div()
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(theme.text)
                                                            .child(stat.table_name.clone()),
                                                    ),
                                            ),
                                    )
                                    // Rows
                                    .child(
                                        div()
                                            .w_20()
                                            .text_sm()
                                            .font_family("monospace")
                                            .text_right()
                                            .text_color(theme.text)
                                            .child(Self::format_number(stat.row_count_estimate)),
                                    )
                                    // Size
                                    .child(
                                        div()
                                            .w_24()
                                            .text_sm()
                                            .font_family("monospace")
                                            .text_right()
                                            .text_color(theme.text)
                                            .child(Self::format_bytes(stat.total_size_bytes)),
                                    )
                                    // Seq scans
                                    .child(
                                        div()
                                            .w_24()
                                            .text_sm()
                                            .font_family("monospace")
                                            .text_right()
                                            .text_color(theme.text)
                                            .child(Self::format_number(stat.seq_scans)),
                                    )
                                    // Idx scans
                                    .child(
                                        div()
                                            .w_24()
                                            .text_sm()
                                            .font_family("monospace")
                                            .text_right()
                                            .text_color(theme.text)
                                            .child(Self::format_number(stat.idx_scans)),
                                    )
                                    // Dead rows
                                    .child(
                                        div()
                                            .w_24()
                                            .text_sm()
                                            .font_family("monospace")
                                            .text_right()
                                            .when(needs_vacuum, |this| {
                                                this.text_color(theme.error)
                                            })
                                            .when(!needs_vacuum, |this| {
                                                this.text_color(theme.text)
                                            })
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_end()
                                                    .gap_1()
                                                    .child(Self::format_number(stat.dead_row_count))
                                                    .when(needs_vacuum, |this| {
                                                        this.child(
                                                            div()
                                                                .text_color(theme.warning)
                                                                .child("⚠"),
                                                        )
                                                    }),
                                            ),
                                    )
                                    // Last vacuum
                                    .child(
                                        div()
                                            .w_28()
                                            .text_sm()
                                            .text_center()
                                            .when(needs_analyze, |this| {
                                                this.text_color(theme.warning)
                                            })
                                            .when(!needs_analyze, |this| {
                                                this.text_color(theme.text)
                                            })
                                            .child(Self::format_date(
                                                stat.last_autovacuum.or(stat.last_vacuum),
                                            )),
                                    )
                                    // Actions
                                    .child(
                                        div()
                                            .w_32()
                                            .flex()
                                            .items_center()
                                            .justify_end()
                                            .gap_1()
                                            .child(
                                                Button::new(format!("vacuum-{}-{}", schema, table))
                                                    .label("Vacuum")
                                                    .size(ButtonSize::ExtraSmall)
                                                    .variant(ButtonVariant::Primary)
                                                    .on_click(cx.listener({
                                                        let schema = schema.clone();
                                                        let table = table.clone();
                                                        move |this, _, cx| {
                                                            this.on_vacuum(&schema, &table, cx);
                                                        }
                                                    })),
                                            )
                                            .child(
                                                Button::new(format!("analyze-{}-{}", schema, table))
                                                    .label("Analyze")
                                                    .size(ButtonSize::ExtraSmall)
                                                    .variant(ButtonVariant::Success)
                                                    .on_click(cx.listener({
                                                        let schema = schema.clone();
                                                        let table = table.clone();
                                                        move |this, _, cx| {
                                                            this.on_analyze(&schema, &table, cx);
                                                        }
                                                    })),
                                            ),
                                    )
                            })),
                    ),
            )
    }
}

impl TableStatsView {
    fn sortable_header(
        &self,
        label: &str,
        column: TableSortColumn,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .flex_1()
            .flex()
            .items_center()
            .gap_1()
            .cursor_pointer()
            .hover(|this| this.bg(theme.surface))
            .on_click(cx.listener(move |this, _, cx| {
                this.on_sort_click(column, cx);
            }))
            .child(label)
            .child(self.render_sort_indicator(column, theme))
    }
}
```

## 20.7 Index Stats View

```rust
// src/components/admin/index_stats_view.rs

use crate::models::admin::{IndexSortColumn, IndexStats, SortDirection};
use crate::state::admin_state::AdminState;
use crate::theme::Theme;
use gpui::*;

/// View for displaying index statistics
pub struct IndexStatsView {
    stats: Vec<IndexStats>,
    filter: String,
    sort_column: IndexSortColumn,
    sort_direction: SortDirection,
    conn_id: String,
}

impl IndexStatsView {
    pub fn new(
        stats: Vec<IndexStats>,
        filter: String,
        sort_column: IndexSortColumn,
        sort_direction: SortDirection,
        conn_id: String,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            stats,
            filter,
            sort_column,
            sort_direction,
            conn_id,
        }
    }

    fn format_bytes(bytes: i64) -> String {
        const GB: i64 = 1_073_741_824;
        const MB: i64 = 1_048_576;
        const KB: i64 = 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    fn format_number(n: i64) -> String {
        const MILLION: i64 = 1_000_000;
        const THOUSAND: i64 = 1_000;

        if n >= MILLION {
            format!("{:.1}M", n as f64 / MILLION as f64)
        } else if n >= THOUSAND {
            format!("{:.1}K", n as f64 / THOUSAND as f64)
        } else {
            n.to_string()
        }
    }

    fn on_filter_change(&mut self, value: String, cx: &mut Context<Self>) {
        self.filter = value.clone();
        cx.global::<AdminState>().set_index_filter(&self.conn_id, value);
        cx.notify();
    }

    fn on_sort_click(&mut self, column: IndexSortColumn, cx: &mut Context<Self>) {
        cx.global::<AdminState>().toggle_index_sort(&self.conn_id, column);
        cx.notify();
    }
}

impl Render for IndexStatsView {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let total_count = self.stats.len();
        let unused_count = self.stats.iter().filter(|s| s.is_unused()).count();

        div()
            .flex()
            .flex_col()
            .gap_4()
            // Filter bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        TextInput::new("index-filter")
                            .placeholder("Filter indexes...")
                            .value(self.filter.clone())
                            .w_64()
                            .on_change(cx.listener(|this, value, cx| {
                                this.on_filter_change(value, cx);
                            })),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child(format!("{} indexes", total_count)),
                    )
                    .when(unused_count > 0, |this| {
                        this.child(
                            div()
                                .px_2()
                                .py_1()
                                .bg(theme.warning_bg)
                                .text_color(theme.warning)
                                .text_xs()
                                .rounded_md()
                                .child(format!("{} potentially unused", unused_count)),
                        )
                    }),
            )
            // Table
            .child(
                div()
                    .w_full()
                    .bg(theme.surface)
                    .rounded_lg()
                    .shadow_sm()
                    .overflow_x_auto()
                    // Header
                    .child(
                        div()
                            .flex()
                            .bg(theme.surface_hover)
                            .px_4()
                            .py_3()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text_muted)
                            .child(
                                self.sortable_header("Index", IndexSortColumn::Name, theme, cx)
                                    .flex_1(),
                            )
                            .child(
                                self.sortable_header("Table", IndexSortColumn::Table, theme, cx)
                                    .w_32(),
                            )
                            .child(
                                self.sortable_header("Type", IndexSortColumn::Type, theme, cx)
                                    .w_20(),
                            )
                            .child(
                                self.sortable_header("Size", IndexSortColumn::Size, theme, cx)
                                    .w_24()
                                    .text_right(),
                            )
                            .child(
                                self.sortable_header("Scans", IndexSortColumn::Scans, theme, cx)
                                    .w_20()
                                    .text_right(),
                            )
                            .child(
                                self.sortable_header("Reads", IndexSortColumn::Reads, theme, cx)
                                    .w_20()
                                    .text_right(),
                            )
                            .child(div().w_20().text_center().child("Flags")),
                    )
                    // Body
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .children(self.stats.iter().map(|stat| {
                                let is_unused = stat.is_unused();

                                div()
                                    .flex()
                                    .items_center()
                                    .px_4()
                                    .py_3()
                                    .border_t_1()
                                    .border_color(theme.border)
                                    .hover(|this| this.bg(theme.surface_hover))
                                    .when(is_unused, |this| {
                                        this.bg(theme.warning_bg.opacity(0.1))
                                    })
                                    // Index name
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_sm()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .child(
                                                        div()
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(theme.text)
                                                            .child(stat.index_name.clone()),
                                                    )
                                                    .when(is_unused, |this| {
                                                        this.child(
                                                            div()
                                                                .px_1()
                                                                .py_px()
                                                                .bg(theme.warning_bg)
                                                                .text_color(theme.warning)
                                                                .text_xs()
                                                                .rounded_sm()
                                                                .child("unused"),
                                                        )
                                                    }),
                                            ),
                                    )
                                    // Table
                                    .child(
                                        div()
                                            .w_32()
                                            .text_sm()
                                            .text_color(theme.text)
                                            .truncate()
                                            .child(stat.table_name.clone()),
                                    )
                                    // Type
                                    .child(
                                        div()
                                            .w_20()
                                            .text_sm()
                                            .text_color(theme.text_muted)
                                            .child(stat.index_type.clone()),
                                    )
                                    // Size
                                    .child(
                                        div()
                                            .w_24()
                                            .text_sm()
                                            .font_family("monospace")
                                            .text_right()
                                            .text_color(theme.text)
                                            .child(Self::format_bytes(stat.index_size_bytes)),
                                    )
                                    // Scans
                                    .child(
                                        div()
                                            .w_20()
                                            .text_sm()
                                            .font_family("monospace")
                                            .text_right()
                                            .when(stat.idx_scan == 0, |this| {
                                                this.text_color(theme.text_muted)
                                            })
                                            .when(stat.idx_scan > 0, |this| {
                                                this.text_color(theme.text)
                                            })
                                            .child(Self::format_number(stat.idx_scan)),
                                    )
                                    // Reads
                                    .child(
                                        div()
                                            .w_20()
                                            .text_sm()
                                            .font_family("monospace")
                                            .text_right()
                                            .text_color(theme.text)
                                            .child(Self::format_number(stat.idx_tup_read)),
                                    )
                                    // Flags
                                    .child(
                                        div()
                                            .w_20()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .gap_1()
                                            .when(stat.is_primary, |this| {
                                                this.child(
                                                    div()
                                                        .px_1()
                                                        .py_px()
                                                        .bg(theme.primary_bg)
                                                        .text_color(theme.primary)
                                                        .text_xs()
                                                        .rounded_sm()
                                                        .child("PK"),
                                                )
                                            })
                                            .when(stat.is_unique && !stat.is_primary, |this| {
                                                this.child(
                                                    div()
                                                        .px_1()
                                                        .py_px()
                                                        .bg(theme.info_bg)
                                                        .text_color(theme.info)
                                                        .text_xs()
                                                        .rounded_sm()
                                                        .child("UQ"),
                                                )
                                            })
                                            .when(!stat.is_valid, |this| {
                                                this.child(
                                                    div()
                                                        .px_1()
                                                        .py_px()
                                                        .bg(theme.error_bg)
                                                        .text_color(theme.error)
                                                        .text_xs()
                                                        .rounded_sm()
                                                        .child("Invalid"),
                                                )
                                            }),
                                    )
                            })),
                    ),
            )
    }
}

impl IndexStatsView {
    fn sortable_header(
        &self,
        label: &str,
        column: IndexSortColumn,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Div {
        let is_active = self.sort_column == column;
        let arrow = if is_active {
            if self.sort_direction == SortDirection::Ascending {
                "▲"
            } else {
                "▼"
            }
        } else {
            ""
        };

        div()
            .flex()
            .items_center()
            .gap_1()
            .cursor_pointer()
            .hover(|this| this.bg(theme.surface))
            .on_click(cx.listener(move |this, _, cx| {
                this.on_sort_click(column, cx);
            }))
            .child(label)
            .when(is_active, |this| {
                this.child(div().text_color(theme.primary).child(arrow))
            })
    }
}
```

## 20.8 Locks View

```rust
// src/components/admin/locks_view.rs

use crate::models::admin::LockInfo;
use crate::state::admin_state::AdminState;
use crate::theme::Theme;
use gpui::*;

/// View for displaying current locks
pub struct LocksView {
    locks: Vec<LockInfo>,
    show_only_waiting: bool,
    conn_id: String,
}

impl LocksView {
    pub fn new(
        locks: Vec<LockInfo>,
        show_only_waiting: bool,
        conn_id: String,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            locks,
            show_only_waiting,
            conn_id,
        }
    }

    fn get_mode_color(&self, mode: &str, theme: &Theme) -> Hsla {
        match mode {
            "AccessShareLock" | "RowShareLock" => theme.success,
            "ShareLock" | "ShareRowExclusiveLock" => theme.info,
            "RowExclusiveLock" | "ShareUpdateExclusiveLock" => theme.warning,
            "ExclusiveLock" | "AccessExclusiveLock" => theme.error,
            _ => theme.text_muted,
        }
    }

    fn get_short_mode(&self, mode: &str) -> &str {
        match mode {
            "AccessShareLock" => "AS",
            "RowShareLock" => "RS",
            "RowExclusiveLock" => "RX",
            "ShareUpdateExclusiveLock" => "SUX",
            "ShareLock" => "S",
            "ShareRowExclusiveLock" => "SRX",
            "ExclusiveLock" => "X",
            "AccessExclusiveLock" => "AX",
            _ => mode,
        }
    }

    fn on_toggle_filter(&mut self, cx: &mut Context<Self>) {
        cx.global::<AdminState>().toggle_waiting_locks_filter(&self.conn_id);
        cx.notify();
    }
}

impl Render for LocksView {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let waiting_count = self.locks.iter().filter(|l| !l.granted).count();
        let blocking_count = self.locks.iter().filter(|l| l.is_blocking).count();

        div()
            .flex()
            .flex_col()
            .gap_4()
            // Filter bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Checkbox::new(self.show_only_waiting)
                                    .on_toggle(cx.listener(|this, _, cx| {
                                        this.on_toggle_filter(cx);
                                    })),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.text_muted)
                                    .child("Show only waiting/blocking"),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child(format!("{} locks", self.locks.len())),
                    )
                    .when(waiting_count > 0, |this| {
                        this.child(
                            div()
                                .px_2()
                                .py_1()
                                .bg(theme.error_bg)
                                .text_color(theme.error)
                                .text_xs()
                                .rounded_md()
                                .child(format!("{} waiting", waiting_count)),
                        )
                    })
                    .when(blocking_count > 0, |this| {
                        this.child(
                            div()
                                .px_2()
                                .py_1()
                                .bg(theme.warning_bg)
                                .text_color(theme.warning)
                                .text_xs()
                                .rounded_md()
                                .child(format!("{} blocking", blocking_count)),
                        )
                    }),
            )
            // Legend
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .child("Lock modes:")
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(div().w_3().h_3().rounded_sm().bg(theme.success))
                            .child("AS/RS (Read)"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(div().w_3().h_3().rounded_sm().bg(theme.info))
                            .child("S/SRX (Share)"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(div().w_3().h_3().rounded_sm().bg(theme.warning))
                            .child("RX/SUX (Row Excl)"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(div().w_3().h_3().rounded_sm().bg(theme.error))
                            .child("X/AX (Exclusive)"),
                    ),
            )
            // Table
            .child(
                div()
                    .w_full()
                    .bg(theme.surface)
                    .rounded_lg()
                    .shadow_sm()
                    .overflow_x_auto()
                    // Header
                    .child(
                        div()
                            .flex()
                            .bg(theme.surface_hover)
                            .px_4()
                            .py_3()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text_muted)
                            .child(div().w_16().child("PID"))
                            .child(div().w_24().child("User"))
                            .child(div().w_24().child("Lock Type"))
                            .child(div().w_32().child("Relation"))
                            .child(div().w_16().child("Mode"))
                            .child(div().w_20().child("Status"))
                            .child(div().flex_1().child("Query")),
                    )
                    // Body
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .when(self.locks.is_empty(), |this| {
                                this.child(
                                    div()
                                        .px_4()
                                        .py_8()
                                        .text_center()
                                        .text_color(theme.text_muted)
                                        .child("No locks"),
                                )
                            })
                            .when(!self.locks.is_empty(), |this| {
                                this.children(self.locks.iter().map(|lock| {
                                    let is_waiting = !lock.granted;
                                    let is_blocking = lock.is_blocking;

                                    div()
                                        .flex()
                                        .items_center()
                                        .px_4()
                                        .py_3()
                                        .border_t_1()
                                        .border_color(theme.border)
                                        .when(is_waiting, |this| {
                                            this.bg(theme.error_bg.opacity(0.1))
                                        })
                                        .when(is_blocking && !is_waiting, |this| {
                                            this.bg(theme.warning_bg.opacity(0.1))
                                        })
                                        .hover(|this| this.bg(theme.surface_hover))
                                        // PID
                                        .child(
                                            div()
                                                .w_16()
                                                .text_sm()
                                                .font_family("monospace")
                                                .text_color(theme.text)
                                                .child(lock.pid.to_string()),
                                        )
                                        // User
                                        .child(
                                            div()
                                                .w_24()
                                                .text_sm()
                                                .text_color(theme.text)
                                                .truncate()
                                                .child(lock.user.clone()),
                                        )
                                        // Lock type
                                        .child(
                                            div()
                                                .w_24()
                                                .text_sm()
                                                .text_color(theme.text_muted)
                                                .child(lock.locktype.clone()),
                                        )
                                        // Relation
                                        .child(
                                            div()
                                                .w_32()
                                                .text_sm()
                                                .text_color(theme.text)
                                                .truncate()
                                                .child(
                                                    match (&lock.schema_name, &lock.relation_name) {
                                                        (Some(s), Some(r)) => format!("{}.{}", s, r),
                                                        (None, Some(r)) => r.clone(),
                                                        _ => "-".to_string(),
                                                    },
                                                ),
                                        )
                                        // Mode
                                        .child(
                                            div()
                                                .w_16()
                                                .child(
                                                    div()
                                                        .px_1()
                                                        .py_px()
                                                        .rounded_sm()
                                                        .text_xs()
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .bg(self.get_mode_color(&lock.mode, theme).opacity(0.2))
                                                        .text_color(self.get_mode_color(&lock.mode, theme))
                                                        .child(self.get_short_mode(&lock.mode)),
                                                ),
                                        )
                                        // Status
                                        .child(
                                            div()
                                                .w_20()
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .px_1()
                                                        .py_px()
                                                        .rounded_sm()
                                                        .text_xs()
                                                        .when(is_waiting, |this| {
                                                            this.bg(theme.error_bg)
                                                                .text_color(theme.error)
                                                                .child("waiting")
                                                        })
                                                        .when(!is_waiting, |this| {
                                                            this.bg(theme.success_bg)
                                                                .text_color(theme.success)
                                                                .child("granted")
                                                        }),
                                                )
                                                .when(is_blocking, |this| {
                                                    this.child(
                                                        div()
                                                            .px_1()
                                                            .py_px()
                                                            .bg(theme.warning_bg)
                                                            .text_color(theme.warning)
                                                            .text_xs()
                                                            .rounded_sm()
                                                            .child(format!(
                                                                "blocks {}",
                                                                lock.blocking_pids.len()
                                                            )),
                                                    )
                                                }),
                                        )
                                        // Query
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_sm()
                                                .font_family("monospace")
                                                .text_color(theme.text)
                                                .truncate()
                                                .child(
                                                    lock.query.clone().unwrap_or_else(|| "-".to_string()),
                                                ),
                                        )
                                }))
                            }),
                    ),
            )
    }
}
```

## 20.9 Module Exports

```rust
// src/components/admin/mod.rs

mod admin_dashboard;
mod active_queries_view;
mod table_stats_view;
mod index_stats_view;
mod locks_view;

pub use admin_dashboard::{AdminDashboard, AdminEvent};
pub use active_queries_view::ActiveQueriesView;
pub use table_stats_view::{TableStatsView, TableStatsEvent};
pub use index_stats_view::IndexStatsView;
pub use locks_view::LocksView;
```

## 20.10 Integration Example

```rust
// Example: Opening admin dashboard for a connection

use crate::components::admin::AdminDashboard;
use crate::state::admin_state::AdminState;
use gpui::*;

fn open_admin_dashboard(conn_id: &str, cx: &mut WindowContext) {
    // Ensure AdminState is initialized
    if !cx.has_global::<AdminState>() {
        let connection_service = /* get from app state */;
        let runtime = tokio::runtime::Handle::current();
        cx.set_global(AdminState::new(connection_service, runtime));
    }

    // Create the dashboard view
    let dashboard = cx.new_view(|cx| {
        AdminDashboard::new(conn_id.to_string(), cx)
    });

    // Subscribe to events
    cx.subscribe(&dashboard, |_, event, cx| {
        match event {
            AdminEvent::MaintenanceRequested { action, schema, table } => {
                // Open maintenance dialog (Feature 21)
                println!("{} requested on {}.{}", action, schema, table);
            }
        }
    }).detach();

    // Add to workspace or panel
    // workspace.add_panel(dashboard, cx);
}
```

## Acceptance Criteria

1. **Server Statistics**
   - [x] Display PostgreSQL version and uptime
   - [x] Show connection count with max connections
   - [x] Calculate and display cache hit ratio with color coding
   - [x] Show transactions per second
   - [x] List database sizes with formatted values
   - [x] Display checkpoint and temporary file statistics

2. **Activity Monitor**
   - [x] Display all active connections from pg_stat_activity
   - [x] Show query state with color-coded badges
   - [x] Show duration with warning for long-running queries (>30s)
   - [x] Display wait events when applicable
   - [x] Support canceling queries with confirmation dialog
   - [x] Support terminating connections with confirmation dialog
   - [x] Auto-refresh with configurable interval (1s, 5s, 10s, 30s, 60s)

3. **Table Statistics**
   - [x] Show all user tables with sizes (total, table, indexes, toast)
   - [x] Display row count estimates and scan counts
   - [x] Show dead row counts with warning indicator
   - [x] Display last vacuum/analyze times with staleness warning
   - [x] Highlight tables needing maintenance
   - [x] Support sorting by all columns
   - [x] Support filtering by table/schema name
   - [x] Quick actions for vacuum/analyze

4. **Index Statistics**
   - [x] List all indexes with usage stats
   - [x] Identify and highlight unused indexes
   - [x] Show index sizes and types
   - [x] Display scan counts and tuple reads
   - [x] Show index flags (PK, Unique, Invalid)
   - [x] Support sorting and filtering

5. **Lock Monitoring**
   - [x] Show current locks with lock type and mode
   - [x] Identify waiting and blocking locks
   - [x] Display lock modes with color-coded badges
   - [x] Show relation names when available
   - [x] Filter to show only waiting/blocking locks
   - [x] Display blocking PID counts

6. **Performance**
   - [x] Concurrent data fetching for all statistics
   - [x] Efficient auto-refresh with stop capability
   - [x] Filtered/sorted data computed on-demand
   - [x] Minimal re-renders through GPUI's reactive system

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_stats_needs_vacuum() {
        let stat = TableStats {
            live_row_count: 1000,
            dead_row_count: 150, // 15% dead
            ..Default::default()
        };
        assert!(stat.needs_vacuum());

        let stat2 = TableStats {
            live_row_count: 1000,
            dead_row_count: 50, // 5% dead
            ..Default::default()
        };
        assert!(!stat2.needs_vacuum());
    }

    #[test]
    fn test_index_stats_is_unused() {
        let stat = IndexStats {
            idx_scan: 0,
            is_primary: false,
            is_unique: false,
            ..Default::default()
        };
        assert!(stat.is_unused());

        let pk = IndexStats {
            idx_scan: 0,
            is_primary: true,
            is_unique: true,
            ..Default::default()
        };
        assert!(!pk.is_unused()); // Primary keys are never "unused"
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1_048_576), "1.00 MB");
        assert_eq!(format_bytes(1_073_741_824), "1.00 GB");
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(500), "500");
        assert_eq!(format_number(1500), "1.5K");
        assert_eq!(format_number(1_500_000), "1.5M");
    }
}
```

### Integration Tests with Tauri MCP

```rust
// Test using Tauri MCP for E2E testing

#[tokio::test]
async fn test_admin_dashboard_e2e() {
    // Start driver session
    let session = mcp___hypothesi_tauri_mcp_server__driver_session(json!({
        "action": "start"
    })).await;

    // Navigate to admin dashboard
    mcp___hypothesi_tauri_mcp_server__webview_interact(json!({
        "action": "click",
        "selector": "[data-testid='admin-dashboard-button']"
    })).await;

    // Wait for server stats to load
    mcp___hypothesi_tauri_mcp_server__webview_wait_for(json!({
        "type": "selector",
        "value": "[data-testid='server-stats']",
        "timeout": 5000
    })).await;

    // Take accessibility snapshot
    let snapshot = mcp___hypothesi_tauri_mcp_server__webview_dom_snapshot(json!({
        "type": "accessibility"
    })).await;

    // Verify server stats are displayed
    assert!(snapshot.contains("Connections"));
    assert!(snapshot.contains("Cache Hit Ratio"));

    // Switch to Tables tab
    mcp___hypothesi_tauri_mcp_server__webview_interact(json!({
        "action": "click",
        "selector": "button:has-text('Tables')"
    })).await;

    // Wait for table stats
    mcp___hypothesi_tauri_mcp_server__webview_wait_for(json!({
        "type": "selector",
        "value": "[data-testid='table-stats-view']",
        "timeout": 5000
    })).await;

    // Test filtering
    mcp___hypothesi_tauri_mcp_server__webview_keyboard(json!({
        "action": "type",
        "selector": "input[placeholder='Filter tables...']",
        "text": "users"
    })).await;

    // Verify filter applied
    let filtered_snapshot = mcp___hypothesi_tauri_mcp_server__webview_dom_snapshot(json!({
        "type": "accessibility"
    })).await;

    // Test cancel query dialog
    mcp___hypothesi_tauri_mcp_server__webview_interact(json!({
        "action": "click",
        "selector": "button:has-text('Activity')"
    })).await;

    // If there's an active query, test cancel
    mcp___hypothesi_tauri_mcp_server__webview_interact(json!({
        "action": "click",
        "selector": "button:has-text('Cancel'):first"
    })).await;

    // Verify confirmation dialog
    mcp___hypothesi_tauri_mcp_server__webview_wait_for(json!({
        "type": "text",
        "value": "Cancel Query?"
    })).await;

    // Close dialog
    mcp___hypothesi_tauri_mcp_server__webview_interact(json!({
        "action": "click",
        "selector": "button:has-text('Cancel'):last"
    })).await;

    // Stop session
    mcp___hypothesi_tauri_mcp_server__driver_session(json!({
        "action": "stop"
    })).await;
}
```

### Keyboard Navigation Tests

```rust
#[tokio::test]
async fn test_admin_keyboard_navigation() {
    let session = mcp___hypothesi_tauri_mcp_server__driver_session(json!({
        "action": "start"
    })).await;

    // Open admin dashboard
    mcp___hypothesi_tauri_mcp_server__webview_interact(json!({
        "action": "click",
        "selector": "[data-testid='admin-dashboard-button']"
    })).await;

    // Test tab navigation between tabs
    mcp___hypothesi_tauri_mcp_server__webview_keyboard(json!({
        "action": "press",
        "key": "Tab"
    })).await;

    // Press Enter to select tab
    mcp___hypothesi_tauri_mcp_server__webview_keyboard(json!({
        "action": "press",
        "key": "Enter"
    })).await;

    // Test refresh shortcut
    mcp___hypothesi_tauri_mcp_server__webview_keyboard(json!({
        "action": "press",
        "key": "r",
        "modifiers": ["Control"]
    })).await;

    mcp___hypothesi_tauri_mcp_server__driver_session(json!({
        "action": "stop"
    })).await;
}
```
