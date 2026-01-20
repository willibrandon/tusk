# Feature 19: Query Plan Visualization

## Overview

Query plan visualization displays EXPLAIN output in interactive visual formats, helping developers understand query performance characteristics, identify bottlenecks, and optimize queries effectively. Built with GPUI for GPU-accelerated rendering of complex plan trees and timeline visualizations.

## Goals

- Parse and visualize EXPLAIN JSON output
- Provide tree, timeline, and text view modes
- Color-code nodes by execution time percentage
- Display detailed node information
- Highlight performance warnings and suggestions
- Support all EXPLAIN options (ANALYZE, BUFFERS, TIMING, etc.)

## Dependencies

- Feature 11: Query Execution Engine (for executing EXPLAIN queries)
- Feature 14: Results Grid (for text view display)

## GPUI Architecture

This feature uses pure Rust with GPUI for all UI components. No IPC layer - components interact directly with the plan service.

## Technical Specification

### 19.1 Query Plan Data Models

```rust
// src/models/plan.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Complete parsed query plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub raw: String,
    pub format: PlanFormat,
    pub root: PlanNode,
    pub planning_time: f64,
    pub execution_time: Option<f64>,
    pub triggers: Vec<TriggerTiming>,
    pub total_time: f64,
    pub jit_info: Option<JitInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanFormat {
    Text,
    Json,
    Xml,
    Yaml,
}

/// Individual node in the execution plan tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNode {
    /// Unique identifier for this node
    pub node_id: String,
    /// Node type (e.g., "Seq Scan", "Index Scan", "Hash Join")
    pub node_type: String,

    // Source objects
    pub relation_name: Option<String>,
    pub alias: Option<String>,
    pub schema_name: Option<String>,
    pub index_name: Option<String>,
    pub cte_name: Option<String>,
    pub function_name: Option<String>,

    // Join info
    pub join_type: Option<JoinType>,
    pub parent_relationship: Option<String>,

    // Estimates (planner)
    pub startup_cost: f64,
    pub total_cost: f64,
    pub plan_rows: i64,
    pub plan_width: i32,

    // Actuals (ANALYZE only)
    pub actual_startup_time: Option<f64>,
    pub actual_total_time: Option<f64>,
    pub actual_rows: Option<i64>,
    pub actual_loops: Option<i64>,

    // Conditions
    pub filter: Option<String>,
    pub index_cond: Option<String>,
    pub recheck_cond: Option<String>,
    pub join_filter: Option<String>,
    pub hash_cond: Option<String>,
    pub merge_cond: Option<String>,
    pub tid_cond: Option<String>,
    pub one_time_filter: Option<String>,

    // Output columns
    pub output: Option<Vec<String>>,

    // Sorting
    pub sort_key: Option<Vec<String>>,
    pub sort_method: Option<String>,
    pub sort_space_used: Option<i64>,
    pub sort_space_type: Option<SortSpaceType>,
    pub presorted_key: Option<Vec<String>>,

    // Hashing
    pub hash_buckets: Option<i64>,
    pub original_hash_buckets: Option<i64>,
    pub hash_batches: Option<i64>,
    pub original_hash_batches: Option<i64>,
    pub peak_memory_usage: Option<i64>,

    // Buffer stats (BUFFERS option)
    pub shared_hit_blocks: Option<i64>,
    pub shared_read_blocks: Option<i64>,
    pub shared_dirtied_blocks: Option<i64>,
    pub shared_written_blocks: Option<i64>,
    pub local_hit_blocks: Option<i64>,
    pub local_read_blocks: Option<i64>,
    pub local_dirtied_blocks: Option<i64>,
    pub local_written_blocks: Option<i64>,
    pub temp_read_blocks: Option<i64>,
    pub temp_written_blocks: Option<i64>,

    // I/O timing (TIMING option)
    pub io_read_time: Option<f64>,
    pub io_write_time: Option<f64>,

    // WAL stats (WAL option)
    pub wal_records: Option<i64>,
    pub wal_fpi: Option<i64>,
    pub wal_bytes: Option<i64>,

    // Parallel execution
    pub workers_planned: Option<i32>,
    pub workers_launched: Option<i32>,
    pub worker_details: Vec<WorkerDetail>,

    // Partial aggregate info
    pub partial_mode: Option<String>,

    // Group/aggregate info
    pub group_key: Option<Vec<String>>,
    pub grouping_sets: Option<Vec<Vec<String>>>,
    pub strategy: Option<String>,

    // Child nodes
    pub children: Vec<PlanNode>,

    // Computed values for visualization
    pub percent_of_total: f64,
    pub exclusive_time: f64,
    pub exclusive_percent: f64,
    pub is_slowest: bool,
    pub warnings: Vec<PlanWarning>,
    pub depth: i32,
    pub rows_removed: Option<i64>,
    pub heap_fetches: Option<i64>,
    pub exact_heap_blocks: Option<i64>,
    pub lossy_heap_blocks: Option<i64>,
}

impl PlanNode {
    pub fn new(node_type: String) -> Self {
        Self {
            node_id: Uuid::new_v4().to_string(),
            node_type,
            relation_name: None,
            alias: None,
            schema_name: None,
            index_name: None,
            cte_name: None,
            function_name: None,
            join_type: None,
            parent_relationship: None,
            startup_cost: 0.0,
            total_cost: 0.0,
            plan_rows: 0,
            plan_width: 0,
            actual_startup_time: None,
            actual_total_time: None,
            actual_rows: None,
            actual_loops: None,
            filter: None,
            index_cond: None,
            recheck_cond: None,
            join_filter: None,
            hash_cond: None,
            merge_cond: None,
            tid_cond: None,
            one_time_filter: None,
            output: None,
            sort_key: None,
            sort_method: None,
            sort_space_used: None,
            sort_space_type: None,
            presorted_key: None,
            hash_buckets: None,
            original_hash_buckets: None,
            hash_batches: None,
            original_hash_batches: None,
            peak_memory_usage: None,
            shared_hit_blocks: None,
            shared_read_blocks: None,
            shared_dirtied_blocks: None,
            shared_written_blocks: None,
            local_hit_blocks: None,
            local_read_blocks: None,
            local_dirtied_blocks: None,
            local_written_blocks: None,
            temp_read_blocks: None,
            temp_written_blocks: None,
            io_read_time: None,
            io_write_time: None,
            wal_records: None,
            wal_fpi: None,
            wal_bytes: None,
            workers_planned: None,
            workers_launched: None,
            worker_details: Vec::new(),
            partial_mode: None,
            group_key: None,
            grouping_sets: None,
            strategy: None,
            children: Vec::new(),
            percent_of_total: 0.0,
            exclusive_time: 0.0,
            exclusive_percent: 0.0,
            is_slowest: false,
            warnings: Vec::new(),
            depth: 0,
            rows_removed: None,
            heap_fetches: None,
            exact_heap_blocks: None,
            lossy_heap_blocks: None,
        }
    }

    /// Get display name for the node
    pub fn display_name(&self) -> String {
        let mut name = self.node_type.clone();

        if let Some(ref rel) = self.relation_name {
            if let Some(ref alias) = self.alias {
                if alias != rel {
                    name.push_str(&format!(" on {} as {}", rel, alias));
                } else {
                    name.push_str(&format!(" on {}", rel));
                }
            } else {
                name.push_str(&format!(" on {}", rel));
            }
        }

        if let Some(ref idx) = self.index_name {
            name.push_str(&format!(" using {}", idx));
        }

        name
    }

    /// Get the effective rows accounting for loops
    pub fn effective_rows(&self) -> Option<i64> {
        self.actual_rows.map(|r| r * self.actual_loops.unwrap_or(1))
    }

    /// Check if this node has buffer statistics
    pub fn has_buffer_stats(&self) -> bool {
        self.shared_hit_blocks.is_some()
            || self.shared_read_blocks.is_some()
            || self.local_hit_blocks.is_some()
            || self.temp_read_blocks.is_some()
    }

    /// Get total buffer reads
    pub fn total_buffer_reads(&self) -> i64 {
        self.shared_read_blocks.unwrap_or(0)
            + self.local_read_blocks.unwrap_or(0)
            + self.temp_read_blocks.unwrap_or(0)
    }

    /// Get total buffer hits
    pub fn total_buffer_hits(&self) -> i64 {
        self.shared_hit_blocks.unwrap_or(0) + self.local_hit_blocks.unwrap_or(0)
    }

    /// Calculate buffer hit ratio
    pub fn buffer_hit_ratio(&self) -> Option<f64> {
        let hits = self.total_buffer_hits();
        let reads = self.total_buffer_reads();
        let total = hits + reads;

        if total > 0 {
            Some((hits as f64 / total as f64) * 100.0)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Semi,
    Anti,
}

impl std::fmt::Display for JoinType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JoinType::Inner => write!(f, "Inner"),
            JoinType::Left => write!(f, "Left"),
            JoinType::Right => write!(f, "Right"),
            JoinType::Full => write!(f, "Full"),
            JoinType::Semi => write!(f, "Semi"),
            JoinType::Anti => write!(f, "Anti"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortSpaceType {
    Memory,
    Disk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerDetail {
    pub worker_id: i32,
    pub actual_startup_time: f64,
    pub actual_total_time: f64,
    pub actual_rows: i64,
    pub actual_loops: i64,
    pub shared_hit_blocks: Option<i64>,
    pub shared_read_blocks: Option<i64>,
    pub temp_read_blocks: Option<i64>,
    pub temp_written_blocks: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerTiming {
    pub trigger_name: String,
    pub relation: String,
    pub time: f64,
    pub calls: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitInfo {
    pub functions: i64,
    pub options: JitOptions,
    pub timing: JitTiming,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitOptions {
    pub inlining: bool,
    pub optimization: bool,
    pub expressions: bool,
    pub deforming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitTiming {
    pub generation: f64,
    pub inlining: f64,
    pub optimization: f64,
    pub emission: f64,
    pub total: f64,
}

/// Warning detected during plan analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanWarning {
    pub warning_type: WarningType,
    pub severity: WarningSeverity,
    pub message: String,
    pub suggestion: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningType {
    SeqScanLargeTable,
    RowEstimateMismatch,
    NestedLoopHighLoops,
    SortOnDisk,
    HashExceedsWorkMem,
    UnusedIndex,
    MissingIndex,
    FilterRemovesMostRows,
    IndexRecheckHigh,
    LowBufferHitRatio,
    ParallelWorkersNotLaunched,
    HighStartupCost,
    CteScanMultiple,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningSeverity {
    Info,
    Warning,
    Critical,
}

impl WarningSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            WarningSeverity::Info => "info",
            WarningSeverity::Warning => "warning",
            WarningSeverity::Critical => "critical",
        }
    }
}

/// Options for EXPLAIN command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainOptions {
    pub analyze: bool,
    pub verbose: bool,
    pub costs: bool,
    pub buffers: bool,
    pub timing: bool,
    pub wal: bool,
    pub settings: bool,
    pub format: PlanFormat,
}

impl Default for ExplainOptions {
    fn default() -> Self {
        Self {
            analyze: true,
            verbose: false,
            costs: true,
            buffers: true,
            timing: true,
            wal: false,
            settings: false,
            format: PlanFormat::Json,
        }
    }
}

impl ExplainOptions {
    /// Build the EXPLAIN command prefix
    pub fn to_explain_prefix(&self) -> String {
        let mut options = Vec::new();

        if self.analyze {
            options.push("ANALYZE");
        }
        if self.verbose {
            options.push("VERBOSE");
        }
        if self.costs {
            options.push("COSTS");
        }
        if self.buffers {
            options.push("BUFFERS");
        }
        if self.timing {
            options.push("TIMING");
        }
        if self.wal {
            options.push("WAL");
        }
        if self.settings {
            options.push("SETTINGS");
        }

        options.push(match self.format {
            PlanFormat::Json => "FORMAT JSON",
            PlanFormat::Text => "FORMAT TEXT",
            PlanFormat::Xml => "FORMAT XML",
            PlanFormat::Yaml => "FORMAT YAML",
        });

        if options.is_empty() {
            "EXPLAIN".to_string()
        } else {
            format!("EXPLAIN ({})", options.join(", "))
        }
    }
}
```

### 19.2 Plan Parser Service

```rust
// src/services/plan.rs

use crate::models::plan::*;
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;
use tokio_postgres::Client;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PlanError {
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Plan parse error: {0}")]
    ParseError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] tokio_postgres::Error),

    #[error("Query error: {0}")]
    QueryError(String),
}

pub struct PlanService {
    // Configurable thresholds for warnings
    seq_scan_row_threshold: i64,
    estimate_ratio_threshold: f64,
    nested_loop_threshold: i64,
    filter_removal_threshold: f64,
    buffer_hit_ratio_threshold: f64,
}

impl PlanService {
    pub fn new() -> Self {
        Self {
            seq_scan_row_threshold: 10_000,
            estimate_ratio_threshold: 10.0,
            nested_loop_threshold: 1000,
            filter_removal_threshold: 0.9,
            buffer_hit_ratio_threshold: 90.0,
        }
    }

    /// Execute EXPLAIN and parse the result
    pub async fn explain(
        &self,
        client: &Client,
        sql: &str,
        options: &ExplainOptions,
    ) -> Result<QueryPlan, PlanError> {
        let explain_sql = format!("{} {}", options.to_explain_prefix(), sql);

        let rows = client.query(&explain_sql, &[]).await?;

        match options.format {
            PlanFormat::Json => {
                // JSON format returns single row with QUERY PLAN column
                if rows.is_empty() {
                    return Err(PlanError::ParseError("Empty result".to_string()));
                }

                let json_str: String = rows[0].get(0);
                self.parse_json(&json_str)
            }
            _ => {
                // Text/XML/YAML format - collect all rows
                let text: String = rows
                    .iter()
                    .map(|row| row.get::<_, String>(0))
                    .collect::<Vec<_>>()
                    .join("\n");

                Ok(QueryPlan {
                    raw: text,
                    format: options.format,
                    root: PlanNode::new("Text Plan".to_string()),
                    planning_time: 0.0,
                    execution_time: None,
                    triggers: Vec::new(),
                    total_time: 0.0,
                    jit_info: None,
                })
            }
        }
    }

    /// Parse EXPLAIN JSON output into a QueryPlan
    pub fn parse_json(&self, json_str: &str) -> Result<QueryPlan, PlanError> {
        let value: Value = serde_json::from_str(json_str)?;

        // EXPLAIN JSON returns an array with one element
        let plan_array = value
            .as_array()
            .ok_or_else(|| PlanError::ParseError("Expected array".to_string()))?;

        let plan_obj = plan_array
            .first()
            .ok_or_else(|| PlanError::ParseError("Empty plan array".to_string()))?;

        let planning_time = plan_obj
            .get("Planning Time")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let execution_time = plan_obj.get("Execution Time").and_then(|v| v.as_f64());

        let total_time = planning_time + execution_time.unwrap_or(0.0);

        // Parse triggers
        let triggers = self.parse_triggers(plan_obj.get("Triggers"));

        // Parse JIT info
        let jit_info = self.parse_jit(plan_obj.get("JIT"));

        // Parse the plan tree
        let plan_value = plan_obj
            .get("Plan")
            .ok_or_else(|| PlanError::ParseError("Missing Plan field".to_string()))?;

        let mut root = self.parse_node(plan_value, 0)?;

        // Calculate derived values
        let root_time = root.actual_total_time.unwrap_or(root.total_cost);
        self.calculate_percentages(&mut root, root_time);
        self.calculate_exclusive_times(&mut root);
        self.mark_slowest_node(&mut root);
        self.detect_warnings(&mut root);

        Ok(QueryPlan {
            raw: json_str.to_string(),
            format: PlanFormat::Json,
            root,
            planning_time,
            execution_time,
            triggers,
            total_time,
            jit_info,
        })
    }

    fn parse_node(&self, value: &Value, depth: i32) -> Result<PlanNode, PlanError> {
        let obj = value
            .as_object()
            .ok_or_else(|| PlanError::ParseError("Expected object for node".to_string()))?;

        let node_type = obj
            .get("Node Type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PlanError::ParseError("Missing Node Type".to_string()))?
            .to_string();

        let mut node = PlanNode::new(node_type);
        node.depth = depth;

        // Source objects
        node.relation_name = Self::get_str(obj, "Relation Name");
        node.alias = Self::get_str(obj, "Alias");
        node.schema_name = Self::get_str(obj, "Schema");
        node.index_name = Self::get_str(obj, "Index Name");
        node.cte_name = Self::get_str(obj, "CTE Name");
        node.function_name = Self::get_str(obj, "Function Name");
        node.parent_relationship = Self::get_str(obj, "Parent Relationship");

        // Join type
        node.join_type = Self::get_str(obj, "Join Type").and_then(|s| match s.as_str() {
            "Inner" => Some(JoinType::Inner),
            "Left" => Some(JoinType::Left),
            "Right" => Some(JoinType::Right),
            "Full" => Some(JoinType::Full),
            "Semi" => Some(JoinType::Semi),
            "Anti" => Some(JoinType::Anti),
            _ => None,
        });

        // Estimates
        node.startup_cost = Self::get_f64(obj, "Startup Cost").unwrap_or(0.0);
        node.total_cost = Self::get_f64(obj, "Total Cost").unwrap_or(0.0);
        node.plan_rows = Self::get_i64(obj, "Plan Rows").unwrap_or(0);
        node.plan_width = Self::get_i64(obj, "Plan Width").unwrap_or(0) as i32;

        // Actuals
        node.actual_startup_time = Self::get_f64(obj, "Actual Startup Time");
        node.actual_total_time = Self::get_f64(obj, "Actual Total Time");
        node.actual_rows = Self::get_i64(obj, "Actual Rows");
        node.actual_loops = Self::get_i64(obj, "Actual Loops");

        // Conditions
        node.filter = Self::get_str(obj, "Filter");
        node.index_cond = Self::get_str(obj, "Index Cond");
        node.recheck_cond = Self::get_str(obj, "Recheck Cond");
        node.join_filter = Self::get_str(obj, "Join Filter");
        node.hash_cond = Self::get_str(obj, "Hash Cond");
        node.merge_cond = Self::get_str(obj, "Merge Cond");
        node.tid_cond = Self::get_str(obj, "TID Cond");
        node.one_time_filter = Self::get_str(obj, "One-Time Filter");

        // Output
        node.output = obj.get("Output").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(|s| s.as_str().map(String::from)).collect())
        });

        // Sorting
        node.sort_key = obj.get("Sort Key").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(|s| s.as_str().map(String::from)).collect())
        });
        node.sort_method = Self::get_str(obj, "Sort Method");
        node.sort_space_used = Self::get_i64(obj, "Sort Space Used");
        node.sort_space_type = Self::get_str(obj, "Sort Space Type").and_then(|s| {
            match s.as_str() {
                "Memory" => Some(SortSpaceType::Memory),
                "Disk" => Some(SortSpaceType::Disk),
                _ => None,
            }
        });
        node.presorted_key = obj.get("Presorted Key").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(|s| s.as_str().map(String::from)).collect())
        });

        // Hashing
        node.hash_buckets = Self::get_i64(obj, "Hash Buckets");
        node.original_hash_buckets = Self::get_i64(obj, "Original Hash Buckets");
        node.hash_batches = Self::get_i64(obj, "Hash Batches");
        node.original_hash_batches = Self::get_i64(obj, "Original Hash Batches");
        node.peak_memory_usage = Self::get_i64(obj, "Peak Memory Usage");

        // Buffer stats
        node.shared_hit_blocks = Self::get_i64(obj, "Shared Hit Blocks");
        node.shared_read_blocks = Self::get_i64(obj, "Shared Read Blocks");
        node.shared_dirtied_blocks = Self::get_i64(obj, "Shared Dirtied Blocks");
        node.shared_written_blocks = Self::get_i64(obj, "Shared Written Blocks");
        node.local_hit_blocks = Self::get_i64(obj, "Local Hit Blocks");
        node.local_read_blocks = Self::get_i64(obj, "Local Read Blocks");
        node.local_dirtied_blocks = Self::get_i64(obj, "Local Dirtied Blocks");
        node.local_written_blocks = Self::get_i64(obj, "Local Written Blocks");
        node.temp_read_blocks = Self::get_i64(obj, "Temp Read Blocks");
        node.temp_written_blocks = Self::get_i64(obj, "Temp Written Blocks");

        // I/O timing
        node.io_read_time = Self::get_f64(obj, "I/O Read Time");
        node.io_write_time = Self::get_f64(obj, "I/O Write Time");

        // WAL stats
        node.wal_records = Self::get_i64(obj, "WAL Records");
        node.wal_fpi = Self::get_i64(obj, "WAL FPI");
        node.wal_bytes = Self::get_i64(obj, "WAL Bytes");

        // Parallel
        node.workers_planned = Self::get_i64(obj, "Workers Planned").map(|v| v as i32);
        node.workers_launched = Self::get_i64(obj, "Workers Launched").map(|v| v as i32);

        // Parse worker details
        if let Some(workers) = obj.get("Workers").and_then(|v| v.as_array()) {
            for worker in workers {
                if let Some(w) = worker.as_object() {
                    node.worker_details.push(WorkerDetail {
                        worker_id: Self::get_i64_from_map(w, "Worker Number").unwrap_or(0) as i32,
                        actual_startup_time: Self::get_f64_from_map(w, "Actual Startup Time")
                            .unwrap_or(0.0),
                        actual_total_time: Self::get_f64_from_map(w, "Actual Total Time")
                            .unwrap_or(0.0),
                        actual_rows: Self::get_i64_from_map(w, "Actual Rows").unwrap_or(0),
                        actual_loops: Self::get_i64_from_map(w, "Actual Loops").unwrap_or(1),
                        shared_hit_blocks: Self::get_i64_from_map(w, "Shared Hit Blocks"),
                        shared_read_blocks: Self::get_i64_from_map(w, "Shared Read Blocks"),
                        temp_read_blocks: Self::get_i64_from_map(w, "Temp Read Blocks"),
                        temp_written_blocks: Self::get_i64_from_map(w, "Temp Written Blocks"),
                    });
                }
            }
        }

        // Aggregate/group info
        node.partial_mode = Self::get_str(obj, "Partial Mode");
        node.group_key = obj.get("Group Key").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(|s| s.as_str().map(String::from)).collect())
        });
        node.strategy = Self::get_str(obj, "Strategy");

        // Bitmap info
        node.heap_fetches = Self::get_i64(obj, "Heap Fetches");
        node.exact_heap_blocks = Self::get_i64(obj, "Exact Heap Blocks");
        node.lossy_heap_blocks = Self::get_i64(obj, "Lossy Heap Blocks");

        // Calculate rows removed by filter
        let rows_removed_by_filter = Self::get_i64(obj, "Rows Removed by Filter");
        let rows_removed_by_index_recheck = Self::get_i64(obj, "Rows Removed by Index Recheck");
        let rows_removed_by_join_filter = Self::get_i64(obj, "Rows Removed by Join Filter");

        node.rows_removed = match (
            rows_removed_by_filter,
            rows_removed_by_index_recheck,
            rows_removed_by_join_filter,
        ) {
            (Some(a), Some(b), Some(c)) => Some(a + b + c),
            (Some(a), Some(b), None) => Some(a + b),
            (Some(a), None, Some(c)) => Some(a + c),
            (None, Some(b), Some(c)) => Some(b + c),
            (Some(a), None, None) => Some(a),
            (None, Some(b), None) => Some(b),
            (None, None, Some(c)) => Some(c),
            (None, None, None) => None,
        };

        // Parse children recursively
        if let Some(plans) = obj.get("Plans").and_then(|v| v.as_array()) {
            for child_value in plans {
                node.children.push(self.parse_node(child_value, depth + 1)?);
            }
        }

        Ok(node)
    }

    fn get_str(obj: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
        obj.get(key).and_then(|v| v.as_str()).map(String::from)
    }

    fn get_f64(obj: &serde_json::Map<String, Value>, key: &str) -> Option<f64> {
        obj.get(key).and_then(|v| v.as_f64())
    }

    fn get_i64(obj: &serde_json::Map<String, Value>, key: &str) -> Option<i64> {
        obj.get(key).and_then(|v| v.as_i64())
    }

    fn get_f64_from_map(obj: &serde_json::Map<String, Value>, key: &str) -> Option<f64> {
        obj.get(key).and_then(|v| v.as_f64())
    }

    fn get_i64_from_map(obj: &serde_json::Map<String, Value>, key: &str) -> Option<i64> {
        obj.get(key).and_then(|v| v.as_i64())
    }

    fn parse_triggers(&self, value: Option<&Value>) -> Vec<TriggerTiming> {
        let Some(arr) = value.and_then(|v| v.as_array()) else {
            return Vec::new();
        };

        arr.iter()
            .filter_map(|t| {
                let obj = t.as_object()?;
                Some(TriggerTiming {
                    trigger_name: obj.get("Trigger Name")?.as_str()?.to_string(),
                    relation: obj.get("Relation")?.as_str()?.to_string(),
                    time: obj.get("Time")?.as_f64()?,
                    calls: obj.get("Calls")?.as_i64()?,
                })
            })
            .collect()
    }

    fn parse_jit(&self, value: Option<&Value>) -> Option<JitInfo> {
        let obj = value?.as_object()?;

        Some(JitInfo {
            functions: obj.get("Functions")?.as_i64()?,
            options: JitOptions {
                inlining: obj.get("Options")?.get("Inlining")?.as_bool()?,
                optimization: obj.get("Options")?.get("Optimization")?.as_bool()?,
                expressions: obj.get("Options")?.get("Expressions")?.as_bool()?,
                deforming: obj.get("Options")?.get("Deforming")?.as_bool()?,
            },
            timing: JitTiming {
                generation: obj.get("Timing")?.get("Generation")?.as_f64()?,
                inlining: obj.get("Timing")?.get("Inlining")?.as_f64()?,
                optimization: obj.get("Timing")?.get("Optimization")?.as_f64()?,
                emission: obj.get("Timing")?.get("Emission")?.as_f64()?,
                total: obj.get("Timing")?.get("Total")?.as_f64()?,
            },
        })
    }

    fn calculate_percentages(&self, node: &mut PlanNode, total_time: f64) {
        if total_time > 0.0 {
            let node_time = node.actual_total_time.unwrap_or(node.total_cost);
            node.percent_of_total = (node_time / total_time) * 100.0;
        }

        for child in &mut node.children {
            self.calculate_percentages(child, total_time);
        }
    }

    fn calculate_exclusive_times(&self, node: &mut PlanNode) {
        let node_time = node.actual_total_time.unwrap_or(0.0);
        let children_time: f64 = node
            .children
            .iter()
            .filter_map(|c| c.actual_total_time)
            .sum();

        node.exclusive_time = (node_time - children_time).max(0.0);

        // Calculate exclusive percentage if we have actual times
        if let Some(total) = node.actual_total_time {
            if total > 0.0 {
                node.exclusive_percent = (node.exclusive_time / total) * 100.0;
            }
        }

        for child in &mut node.children {
            self.calculate_exclusive_times(child);
        }
    }

    fn mark_slowest_node(&self, node: &mut PlanNode) {
        let mut slowest_time = 0.0;
        let mut slowest_id = String::new();

        self.find_slowest(node, &mut slowest_time, &mut slowest_id);
        self.set_slowest(node, &slowest_id);
    }

    fn find_slowest(&self, node: &PlanNode, slowest_time: &mut f64, slowest_id: &mut String) {
        if node.exclusive_time > *slowest_time {
            *slowest_time = node.exclusive_time;
            *slowest_id = node.node_id.clone();
        }

        for child in &node.children {
            self.find_slowest(child, slowest_time, slowest_id);
        }
    }

    fn set_slowest(&self, node: &mut PlanNode, slowest_id: &str) {
        node.is_slowest = node.node_id == slowest_id;

        for child in &mut node.children {
            self.set_slowest(child, slowest_id);
        }
    }

    fn detect_warnings(&self, node: &mut PlanNode) {
        node.warnings.clear();

        // Sequential scan on large table
        if node.node_type == "Seq Scan" {
            let rows = node.actual_rows.or(Some(node.plan_rows)).unwrap_or(0);
            if rows > self.seq_scan_row_threshold {
                node.warnings.push(PlanWarning {
                    warning_type: WarningType::SeqScanLargeTable,
                    severity: WarningSeverity::Warning,
                    message: format!("Sequential scan on {} rows", Self::format_number(rows)),
                    suggestion: "Consider adding an index on the filtered columns".to_string(),
                    details: node.filter.clone(),
                });
            }
        }

        // Row estimate mismatch
        if let Some(actual) = node.actual_rows {
            let estimated = node.plan_rows;
            if estimated > 0 && actual > 0 {
                let ratio = (actual as f64) / (estimated as f64);
                if ratio > self.estimate_ratio_threshold
                    || ratio < (1.0 / self.estimate_ratio_threshold)
                {
                    let severity = if ratio > 100.0 || ratio < 0.01 {
                        WarningSeverity::Critical
                    } else {
                        WarningSeverity::Warning
                    };

                    node.warnings.push(PlanWarning {
                        warning_type: WarningType::RowEstimateMismatch,
                        severity,
                        message: format!(
                            "Actual rows ({}) differ significantly from estimate ({})",
                            Self::format_number(actual),
                            Self::format_number(estimated)
                        ),
                        suggestion: "Run ANALYZE on the table to update statistics".to_string(),
                        details: Some(format!("Ratio: {:.1}x", ratio)),
                    });
                }
            }
        }

        // Nested loop with high loop count
        if node.node_type == "Nested Loop" {
            if let Some(loops) = node.actual_loops {
                if loops > self.nested_loop_threshold {
                    node.warnings.push(PlanWarning {
                        warning_type: WarningType::NestedLoopHighLoops,
                        severity: WarningSeverity::Warning,
                        message: format!("Nested loop executed {} times", Self::format_number(loops)),
                        suggestion:
                            "Consider using a hash or merge join by adding appropriate indexes"
                                .to_string(),
                        details: None,
                    });
                }
            }
        }

        // Sort spilling to disk
        if node.node_type == "Sort" && node.sort_space_type == Some(SortSpaceType::Disk) {
            node.warnings.push(PlanWarning {
                warning_type: WarningType::SortOnDisk,
                severity: WarningSeverity::Critical,
                message: format!(
                    "Sort spilled to disk ({} KB)",
                    node.sort_space_used.unwrap_or(0)
                ),
                suggestion: "Increase work_mem or add an index to avoid sorting".to_string(),
                details: node.sort_key.as_ref().map(|k| format!("Sort key: {}", k.join(", "))),
            });
        }

        // Hash batches > 1 indicates work_mem exceeded
        if let Some(batches) = node.hash_batches {
            if batches > 1 {
                node.warnings.push(PlanWarning {
                    warning_type: WarningType::HashExceedsWorkMem,
                    severity: WarningSeverity::Warning,
                    message: format!(
                        "Hash used {} batches (indicates work_mem exceeded)",
                        batches
                    ),
                    suggestion: "Consider increasing work_mem for this query".to_string(),
                    details: node.peak_memory_usage.map(|m| format!("Peak memory: {} KB", m)),
                });
            }
        }

        // Filter removes most rows
        if let (Some(actual), Some(removed)) = (node.actual_rows, node.rows_removed) {
            if removed > 0 && actual >= 0 {
                let total = actual + removed;
                let ratio = removed as f64 / total as f64;
                if ratio > self.filter_removal_threshold {
                    node.warnings.push(PlanWarning {
                        warning_type: WarningType::FilterRemovesMostRows,
                        severity: WarningSeverity::Info,
                        message: format!(
                            "Filter removed {}% of rows ({} of {})",
                            (ratio * 100.0) as i32,
                            Self::format_number(removed),
                            Self::format_number(total)
                        ),
                        suggestion: "Consider adding a partial index with this filter condition"
                            .to_string(),
                        details: node.filter.clone(),
                    });
                }
            }
        }

        // Low buffer hit ratio
        if let Some(ratio) = node.buffer_hit_ratio() {
            if ratio < self.buffer_hit_ratio_threshold && node.total_buffer_reads() > 100 {
                node.warnings.push(PlanWarning {
                    warning_type: WarningType::LowBufferHitRatio,
                    severity: WarningSeverity::Warning,
                    message: format!("Low buffer hit ratio: {:.1}%", ratio),
                    suggestion: "Consider increasing shared_buffers or improving query to reduce random I/O".to_string(),
                    details: Some(format!(
                        "Hits: {}, Reads: {}",
                        node.total_buffer_hits(),
                        node.total_buffer_reads()
                    )),
                });
            }
        }

        // Parallel workers not launched
        if let (Some(planned), Some(launched)) = (node.workers_planned, node.workers_launched) {
            if launched < planned {
                node.warnings.push(PlanWarning {
                    warning_type: WarningType::ParallelWorkersNotLaunched,
                    severity: WarningSeverity::Info,
                    message: format!(
                        "Only {} of {} planned parallel workers were launched",
                        launched, planned
                    ),
                    suggestion: "Check max_parallel_workers and max_parallel_workers_per_gather settings".to_string(),
                    details: None,
                });
            }
        }

        // High recheck in bitmap scan
        if node.node_type.contains("Bitmap") {
            if let Some(lossy) = node.lossy_heap_blocks {
                if lossy > 0 {
                    let exact = node.exact_heap_blocks.unwrap_or(0);
                    let total = lossy + exact;
                    let lossy_pct = (lossy as f64 / total as f64) * 100.0;

                    if lossy_pct > 50.0 {
                        node.warnings.push(PlanWarning {
                            warning_type: WarningType::IndexRecheckHigh,
                            severity: WarningSeverity::Warning,
                            message: format!(
                                "Bitmap scan has {:.1}% lossy blocks ({} of {})",
                                lossy_pct, lossy, total
                            ),
                            suggestion: "Consider increasing work_mem to reduce lossy bitmap heap scans".to_string(),
                            details: None,
                        });
                    }
                }
            }
        }

        for child in &mut node.children {
            self.detect_warnings(child);
        }
    }

    fn format_number(n: i64) -> String {
        if n >= 1_000_000_000 {
            format!("{:.1}B", n as f64 / 1_000_000_000.0)
        } else if n >= 1_000_000 {
            format!("{:.1}M", n as f64 / 1_000_000.0)
        } else if n >= 1_000 {
            format!("{:.1}K", n as f64 / 1_000.0)
        } else {
            n.to_string()
        }
    }
}

impl Default for PlanService {
    fn default() -> Self {
        Self::new()
    }
}
```

### 19.3 Plan Viewer State (GPUI Global)

```rust
// src/state/plan_state.rs

use crate::models::plan::*;
use crate::services::plan::PlanService;
use crate::services::connection::ConnectionService;
use gpui::*;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::runtime::Handle;

/// View mode for the plan visualization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlanViewMode {
    #[default]
    Tree,
    Timeline,
    Text,
}

/// State for a single query plan visualization
pub struct PlanViewerInstance {
    pub plan: Option<QueryPlan>,
    pub loading: bool,
    pub error: Option<String>,
    pub view_mode: PlanViewMode,
    pub selected_node_id: Option<String>,
    pub expanded_nodes: HashSet<String>,
    pub show_only_warnings: bool,
    pub options: ExplainOptions,
}

impl Default for PlanViewerInstance {
    fn default() -> Self {
        Self {
            plan: None,
            loading: false,
            error: None,
            view_mode: PlanViewMode::Tree,
            selected_node_id: None,
            expanded_nodes: HashSet::new(),
            show_only_warnings: false,
            options: ExplainOptions::default(),
        }
    }
}

/// Key for identifying plan viewer instances
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlanViewerKey {
    pub connection_id: String,
    pub tab_id: String,
}

/// Global state for all plan visualizations
pub struct PlanViewerState {
    plan_service: Arc<PlanService>,
    connection_service: Arc<ConnectionService>,
    instances: RwLock<HashMap<PlanViewerKey, PlanViewerInstance>>,
    runtime: Handle,
}

impl Global for PlanViewerState {}

impl PlanViewerState {
    pub fn new(
        plan_service: Arc<PlanService>,
        connection_service: Arc<ConnectionService>,
        runtime: Handle,
    ) -> Self {
        Self {
            plan_service,
            connection_service,
            instances: RwLock::new(HashMap::new()),
            runtime,
        }
    }

    /// Get or create an instance for the given key
    pub fn get_or_create_instance(&self, key: &PlanViewerKey) -> PlanViewerInstance {
        let instances = self.instances.read();
        instances.get(key).cloned().unwrap_or_default()
    }

    /// Get instance if it exists
    pub fn get_instance(&self, key: &PlanViewerKey) -> Option<PlanViewerInstance> {
        let instances = self.instances.read();
        instances.get(key).cloned()
    }

    /// Execute EXPLAIN and store the result
    pub fn explain(
        &self,
        key: PlanViewerKey,
        sql: String,
        options: ExplainOptions,
        cx: &mut AppContext,
    ) {
        // Mark as loading
        {
            let mut instances = self.instances.write();
            let instance = instances.entry(key.clone()).or_default();
            instance.loading = true;
            instance.error = None;
            instance.options = options.clone();
        }

        let plan_service = self.plan_service.clone();
        let connection_service = self.connection_service.clone();
        let instances = Arc::new(self.instances.clone());
        let key_clone = key.clone();

        cx.spawn(|mut cx| async move {
            let result = async {
                // Get connection
                let pool = connection_service
                    .get_pool(&key_clone.connection_id)
                    .ok_or_else(|| "Connection not found".to_string())?;

                let client = pool
                    .get()
                    .await
                    .map_err(|e| format!("Failed to get connection: {}", e))?;

                // Execute EXPLAIN
                plan_service
                    .explain(&client, &sql, &options)
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;

            cx.update(|cx| {
                let mut instances = instances.write();
                let instance = instances.entry(key_clone).or_default();
                instance.loading = false;

                match result {
                    Ok(plan) => {
                        // Auto-expand first two levels
                        instance.expanded_nodes.clear();
                        Self::expand_to_depth(&plan.root, 2, &mut instance.expanded_nodes);
                        instance.plan = Some(plan);
                        instance.selected_node_id = None;
                    }
                    Err(e) => {
                        instance.error = Some(e);
                    }
                }

                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn expand_to_depth(node: &PlanNode, max_depth: i32, expanded: &mut HashSet<String>) {
        if node.depth < max_depth {
            expanded.insert(node.node_id.clone());
            for child in &node.children {
                Self::expand_to_depth(child, max_depth, expanded);
            }
        }
    }

    /// Select a node by ID
    pub fn select_node(&self, key: &PlanViewerKey, node_id: Option<String>) {
        let mut instances = self.instances.write();
        if let Some(instance) = instances.get_mut(key) {
            instance.selected_node_id = node_id;
        }
    }

    /// Toggle node expansion
    pub fn toggle_node(&self, key: &PlanViewerKey, node_id: &str) {
        let mut instances = self.instances.write();
        if let Some(instance) = instances.get_mut(key) {
            if instance.expanded_nodes.contains(node_id) {
                instance.expanded_nodes.remove(node_id);
            } else {
                instance.expanded_nodes.insert(node_id.to_string());
            }
        }
    }

    /// Expand all nodes
    pub fn expand_all(&self, key: &PlanViewerKey) {
        let mut instances = self.instances.write();
        if let Some(instance) = instances.get_mut(key) {
            if let Some(ref plan) = instance.plan {
                instance.expanded_nodes.clear();
                Self::expand_all_nodes(&plan.root, &mut instance.expanded_nodes);
            }
        }
    }

    fn expand_all_nodes(node: &PlanNode, expanded: &mut HashSet<String>) {
        expanded.insert(node.node_id.clone());
        for child in &node.children {
            Self::expand_all_nodes(child, expanded);
        }
    }

    /// Collapse all nodes
    pub fn collapse_all(&self, key: &PlanViewerKey) {
        let mut instances = self.instances.write();
        if let Some(instance) = instances.get_mut(key) {
            instance.expanded_nodes.clear();
        }
    }

    /// Set view mode
    pub fn set_view_mode(&self, key: &PlanViewerKey, mode: PlanViewMode) {
        let mut instances = self.instances.write();
        if let Some(instance) = instances.get_mut(key) {
            instance.view_mode = mode;
        }
    }

    /// Toggle warnings-only view
    pub fn toggle_warnings_only(&self, key: &PlanViewerKey) {
        let mut instances = self.instances.write();
        if let Some(instance) = instances.get_mut(key) {
            instance.show_only_warnings = !instance.show_only_warnings;
        }
    }

    /// Get all warnings from the plan
    pub fn get_all_warnings(&self, key: &PlanViewerKey) -> Vec<(PlanNode, PlanWarning)> {
        let instances = self.instances.read();
        let Some(instance) = instances.get(key) else {
            return Vec::new();
        };
        let Some(ref plan) = instance.plan else {
            return Vec::new();
        };

        let mut warnings = Vec::new();
        Self::collect_warnings(&plan.root, &mut warnings);

        // Sort by severity
        warnings.sort_by(|a, b| {
            let severity_order = |s: &WarningSeverity| match s {
                WarningSeverity::Critical => 0,
                WarningSeverity::Warning => 1,
                WarningSeverity::Info => 2,
            };
            severity_order(&a.1.severity).cmp(&severity_order(&b.1.severity))
        });

        warnings
    }

    fn collect_warnings(node: &PlanNode, warnings: &mut Vec<(PlanNode, PlanWarning)>) {
        for warning in &node.warnings {
            warnings.push((node.clone(), warning.clone()));
        }
        for child in &node.children {
            Self::collect_warnings(child, warnings);
        }
    }

    /// Find a node by ID
    pub fn find_node(&self, key: &PlanViewerKey, node_id: &str) -> Option<PlanNode> {
        let instances = self.instances.read();
        let instance = instances.get(key)?;
        let plan = instance.plan.as_ref()?;
        Self::find_node_recursive(&plan.root, node_id)
    }

    fn find_node_recursive(node: &PlanNode, node_id: &str) -> Option<PlanNode> {
        if node.node_id == node_id {
            return Some(node.clone());
        }
        for child in &node.children {
            if let Some(found) = Self::find_node_recursive(child, node_id) {
                return Some(found);
            }
        }
        None
    }

    /// Clear the plan
    pub fn clear(&self, key: &PlanViewerKey) {
        let mut instances = self.instances.write();
        if let Some(instance) = instances.get_mut(key) {
            instance.plan = None;
            instance.error = None;
            instance.selected_node_id = None;
            instance.expanded_nodes.clear();
        }
    }

    /// Remove instance
    pub fn remove_instance(&self, key: &PlanViewerKey) {
        let mut instances = self.instances.write();
        instances.remove(key);
    }
}

// Clone implementation for PlanViewerInstance
impl Clone for PlanViewerInstance {
    fn clone(&self) -> Self {
        Self {
            plan: self.plan.clone(),
            loading: self.loading,
            error: self.error.clone(),
            view_mode: self.view_mode,
            selected_node_id: self.selected_node_id.clone(),
            expanded_nodes: self.expanded_nodes.clone(),
            show_only_warnings: self.show_only_warnings,
            options: self.options.clone(),
        }
    }
}
```

### 19.4 Query Plan Viewer Component (GPUI)

```rust
// src/components/plan/plan_viewer.rs

use crate::models::plan::*;
use crate::state::plan_state::*;
use crate::theme::Theme;
use gpui::*;

/// Main query plan viewer component
pub struct QueryPlanViewer {
    key: PlanViewerKey,
    sql: String,
    show_options_dialog: bool,
}

impl QueryPlanViewer {
    pub fn new(connection_id: String, tab_id: String, sql: String) -> Self {
        Self {
            key: PlanViewerKey {
                connection_id,
                tab_id,
            },
            sql,
            show_options_dialog: false,
        }
    }
}

impl Render for QueryPlanViewer {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let state = cx.global::<PlanViewerState>();
        let instance = state.get_or_create_instance(&self.key);

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .child(self.render_toolbar(&instance, theme, cx))
            .child(self.render_content(&instance, theme, cx))
            .when(self.show_options_dialog, |el| {
                el.child(self.render_options_dialog(&instance, theme, cx))
            })
    }
}

impl QueryPlanViewer {
    fn render_toolbar(
        &self,
        instance: &PlanViewerInstance,
        theme: &Theme,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let key = self.key.clone();
        let sql = self.sql.clone();

        div()
            .flex()
            .items_center()
            .gap_2()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(theme.border)
            // Explain button
            .child(
                div()
                    .px_3()
                    .py(px(6.0))
                    .bg(theme.accent)
                    .text_color(theme.on_accent)
                    .text_sm()
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.accent_hover))
                    .on_click(cx.listener(|this, _, cx| {
                        this.show_options_dialog = true;
                        cx.notify();
                    }))
                    .child("⚡ Explain"),
            )
            // View mode tabs (when plan exists)
            .when(instance.plan.is_some(), |el| {
                let key_tree = key.clone();
                let key_timeline = key.clone();
                let key_text = key.clone();

                el.child(
                    div()
                        .flex()
                        .border_1()
                        .border_color(theme.border)
                        .rounded_md()
                        .overflow_hidden()
                        .child(self.render_view_tab(
                            "Tree",
                            instance.view_mode == PlanViewMode::Tree,
                            theme,
                            cx.listener(move |_, _, cx| {
                                let state = cx.global::<PlanViewerState>();
                                state.set_view_mode(&key_tree, PlanViewMode::Tree);
                                cx.notify();
                            }),
                        ))
                        .child(self.render_view_tab(
                            "Timeline",
                            instance.view_mode == PlanViewMode::Timeline,
                            theme,
                            cx.listener(move |_, _, cx| {
                                let state = cx.global::<PlanViewerState>();
                                state.set_view_mode(&key_timeline, PlanViewMode::Timeline);
                                cx.notify();
                            }),
                        ))
                        .child(self.render_view_tab(
                            "Text",
                            instance.view_mode == PlanViewMode::Text,
                            theme,
                            cx.listener(move |_, _, cx| {
                                let state = cx.global::<PlanViewerState>();
                                state.set_view_mode(&key_text, PlanViewMode::Text);
                                cx.notify();
                            }),
                        )),
                )
            })
            // Expand/collapse controls
            .when(instance.plan.is_some(), |el| {
                let key_expand = key.clone();
                let key_collapse = key.clone();

                el.child(
                    div()
                        .flex()
                        .gap_1()
                        .ml_2()
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .text_xs()
                                .text_color(theme.text_secondary)
                                .cursor_pointer()
                                .hover(|s| s.text_color(theme.text))
                                .on_click(cx.listener(move |_, _, cx| {
                                    let state = cx.global::<PlanViewerState>();
                                    state.expand_all(&key_expand);
                                    cx.notify();
                                }))
                                .child("Expand All"),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .text_xs()
                                .text_color(theme.text_secondary)
                                .cursor_pointer()
                                .hover(|s| s.text_color(theme.text))
                                .on_click(cx.listener(move |_, _, cx| {
                                    let state = cx.global::<PlanViewerState>();
                                    state.collapse_all(&key_collapse);
                                    cx.notify();
                                }))
                                .child("Collapse All"),
                        ),
                )
            })
            // Warnings button
            .when_some(instance.plan.as_ref(), |el, plan| {
                let warnings = self.count_warnings(&plan.root);
                if warnings > 0 {
                    let key_warnings = key.clone();
                    let is_active = instance.show_only_warnings;

                    el.child(
                        div()
                            .px_2()
                            .py_1()
                            .text_xs()
                            .rounded_md()
                            .cursor_pointer()
                            .when(is_active, |s| {
                                s.bg(theme.warning_background)
                                    .text_color(theme.warning)
                            })
                            .when(!is_active, |s| {
                                s.text_color(theme.text_secondary)
                                    .hover(|s| s.bg(theme.surface_hover))
                            })
                            .on_click(cx.listener(move |_, _, cx| {
                                let state = cx.global::<PlanViewerState>();
                                state.toggle_warnings_only(&key_warnings);
                                cx.notify();
                            }))
                            .child(format!("⚠ {} Warning{}", warnings, if warnings == 1 { "" } else { "s" })),
                    )
                } else {
                    el
                }
            })
            // Spacer
            .child(div().flex_1())
            // Timing summary
            .when_some(instance.plan.as_ref(), |el, plan| {
                el.child(
                    div()
                        .text_sm()
                        .text_color(theme.text_secondary)
                        .child(format!(
                            "Planning: {} | {}Total: {}",
                            format_time(plan.planning_time),
                            plan.execution_time
                                .map(|t| format!("Execution: {} | ", format_time(t)))
                                .unwrap_or_default(),
                            format_time(plan.total_time)
                        )),
                )
            })
    }

    fn render_view_tab(
        &self,
        label: &str,
        active: bool,
        theme: &Theme,
        on_click: impl Fn(&ClickEvent, &mut WindowContext) + 'static,
    ) -> impl IntoElement {
        div()
            .px_3()
            .py(px(6.0))
            .text_sm()
            .cursor_pointer()
            .when(active, |s| s.bg(theme.surface_active))
            .when(!active, |s| s.hover(|s| s.bg(theme.surface_hover)))
            .on_click(on_click)
            .child(label)
    }

    fn count_warnings(&self, node: &PlanNode) -> usize {
        let mut count = node.warnings.len();
        for child in &node.children {
            count += self.count_warnings(child);
        }
        count
    }

    fn render_content(
        &self,
        instance: &PlanViewerInstance,
        theme: &Theme,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        div()
            .flex_1()
            .overflow_y_scroll()
            .p_4()
            .when(instance.loading, |el| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size_full()
                        .child(
                            div()
                                .w_8()
                                .h_8()
                                .rounded_full()
                                .border_2()
                                .border_color(theme.accent)
                                .border_t_color(gpui::transparent_black())
                                .with_animation(
                                    "spin",
                                    Animation::new(Duration::from_secs(1))
                                        .repeat()
                                        .with_easing(linear),
                                    |el, progress| el.rotate(progress * std::f32::consts::TAU),
                                ),
                        ),
                )
            })
            .when_some(instance.error.as_ref(), |el, error| {
                el.child(
                    div()
                        .p_4()
                        .bg(theme.error_background)
                        .border_1()
                        .border_color(theme.error)
                        .rounded_md()
                        .text_color(theme.error)
                        .child(format!("Error: {}", error)),
                )
            })
            .when(
                !instance.loading && instance.error.is_none() && instance.plan.is_none(),
                |el| {
                    el.child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .size_full()
                            .text_color(theme.text_muted)
                            .child(
                                div()
                                    .text_6xl()
                                    .opacity(0.5)
                                    .mb_4()
                                    .child("📊"),
                            )
                            .child(div().text_lg().mb_2().child("No query plan"))
                            .child(
                                div()
                                    .text_sm()
                                    .child("Click \"Explain\" to analyze the query execution plan"),
                            ),
                    )
                },
            )
            .when_some(instance.plan.as_ref(), |el, plan| {
                if instance.show_only_warnings {
                    el.child(PlanWarningsList::new(self.key.clone()))
                } else {
                    match instance.view_mode {
                        PlanViewMode::Tree => {
                            el.child(PlanTreeView::new(self.key.clone(), plan.root.clone()))
                        }
                        PlanViewMode::Timeline => {
                            el.child(PlanTimelineView::new(self.key.clone(), plan.clone()))
                        }
                        PlanViewMode::Text => {
                            el.child(PlanTextView::new(plan.raw.clone()))
                        }
                    }
                }
            })
    }

    fn render_options_dialog(
        &self,
        instance: &PlanViewerInstance,
        theme: &Theme,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        ExplainOptionsDialog::new(
            self.key.clone(),
            self.sql.clone(),
            instance.options.clone(),
            cx.listener(|this, _, cx| {
                this.show_options_dialog = false;
                cx.notify();
            }),
        )
    }
}

fn format_time(ms: f64) -> String {
    if ms >= 1000.0 {
        format!("{:.2}s", ms / 1000.0)
    } else {
        format!("{:.2}ms", ms)
    }
}
```

### 19.5 EXPLAIN Options Dialog (GPUI)

```rust
// src/components/plan/explain_options_dialog.rs

use crate::models::plan::*;
use crate::state::plan_state::*;
use crate::theme::Theme;
use gpui::*;

pub struct ExplainOptionsDialog {
    key: PlanViewerKey,
    sql: String,
    options: ExplainOptions,
    on_close: Box<dyn Fn(&ClickEvent, &mut WindowContext) + 'static>,
}

impl ExplainOptionsDialog {
    pub fn new(
        key: PlanViewerKey,
        sql: String,
        options: ExplainOptions,
        on_close: impl Fn(&ClickEvent, &mut WindowContext) + 'static,
    ) -> Self {
        Self {
            key,
            sql,
            options,
            on_close: Box::new(on_close),
        }
    }
}

impl Render for ExplainOptionsDialog {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        // Backdrop
        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .z_index(50)
            .on_click({
                let on_close = self.on_close.clone();
                move |event, cx| on_close(event, cx)
            })
            .child(
                // Dialog
                div()
                    .w(px(420.0))
                    .bg(theme.surface)
                    .rounded_lg()
                    .shadow_xl()
                    .overflow_hidden()
                    .on_click(|_, _| {}) // Prevent click propagation
                    // Header
                    .child(
                        div()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(theme.border)
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("EXPLAIN Options"),
                            ),
                    )
                    // Body
                    .child(self.render_body(theme, cx))
                    // Footer
                    .child(self.render_footer(theme, cx)),
            )
    }
}

impl ExplainOptionsDialog {
    fn render_body(&self, theme: &Theme, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            // Primary options grid
            .child(
                div()
                    .grid()
                    .gap_4()
                    .child(self.render_checkbox(
                        "ANALYZE",
                        "Execute query",
                        self.options.analyze,
                        cx.listener(|this, _, cx| {
                            this.options.analyze = !this.options.analyze;
                            cx.notify();
                        }),
                        theme,
                    ))
                    .child(self.render_checkbox(
                        "BUFFERS",
                        "Show buffer usage",
                        self.options.buffers,
                        cx.listener(|this, _, cx| {
                            this.options.buffers = !this.options.buffers;
                            cx.notify();
                        }),
                        theme,
                    ))
                    .child(self.render_checkbox(
                        "VERBOSE",
                        "Additional details",
                        self.options.verbose,
                        cx.listener(|this, _, cx| {
                            this.options.verbose = !this.options.verbose;
                            cx.notify();
                        }),
                        theme,
                    ))
                    .child(self.render_checkbox(
                        "TIMING",
                        "Show timing info",
                        self.options.timing,
                        cx.listener(|this, _, cx| {
                            this.options.timing = !this.options.timing;
                            cx.notify();
                        }),
                        theme,
                    ))
                    .child(self.render_checkbox(
                        "COSTS",
                        "Show cost estimates",
                        self.options.costs,
                        cx.listener(|this, _, cx| {
                            this.options.costs = !this.options.costs;
                            cx.notify();
                        }),
                        theme,
                    ))
                    .child(self.render_checkbox(
                        "WAL",
                        "Show WAL usage",
                        self.options.wal,
                        cx.listener(|this, _, cx| {
                            this.options.wal = !this.options.wal;
                            cx.notify();
                        }),
                        theme,
                    )),
            )
            // Format selection
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child("Format"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(self.render_format_option(
                                "JSON",
                                PlanFormat::Json,
                                theme,
                                cx,
                            ))
                            .child(self.render_format_option(
                                "Text",
                                PlanFormat::Text,
                                theme,
                                cx,
                            ))
                            .child(self.render_format_option(
                                "XML",
                                PlanFormat::Xml,
                                theme,
                                cx,
                            ))
                            .child(self.render_format_option(
                                "YAML",
                                PlanFormat::Yaml,
                                theme,
                                cx,
                            )),
                    ),
            )
            // Warning for ANALYZE
            .when(self.options.analyze, |el| {
                el.child(
                    div()
                        .p_3()
                        .bg(theme.warning_background)
                        .border_1()
                        .border_color(theme.warning)
                        .rounded_md()
                        .text_sm()
                        .text_color(theme.warning)
                        .child(
                            "Note: ANALYZE will actually execute the query. For DML statements (INSERT, UPDATE, DELETE), changes will be made to the database.",
                        ),
                )
            })
    }

    fn render_checkbox(
        &self,
        label: &str,
        description: &str,
        checked: bool,
        on_click: impl Fn(&ClickEvent, &mut WindowContext) + 'static,
        theme: &Theme,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .on_click(on_click)
            .child(
                div()
                    .w_4()
                    .h_4()
                    .rounded(px(3.0))
                    .border_1()
                    .border_color(theme.border)
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(checked, |s| s.bg(theme.accent).border_color(theme.accent))
                    .when(checked, |s| {
                        s.child(div().text_xs().text_color(theme.on_accent).child("✓"))
                    }),
            )
            .child(
                div()
                    .text_sm()
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .child(label),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_secondary)
                            .child(description),
                    ),
            )
    }

    fn render_format_option(
        &self,
        label: &str,
        format: PlanFormat,
        theme: &Theme,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let is_selected = self.options.format == format;

        div()
            .px_3()
            .py_2()
            .text_sm()
            .rounded_md()
            .cursor_pointer()
            .border_1()
            .when(is_selected, |s| {
                s.bg(theme.accent)
                    .text_color(theme.on_accent)
                    .border_color(theme.accent)
            })
            .when(!is_selected, |s| {
                s.border_color(theme.border)
                    .hover(|s| s.bg(theme.surface_hover))
            })
            .on_click(cx.listener(move |this, _, cx| {
                this.options.format = format;
                cx.notify();
            }))
            .child(label)
    }

    fn render_footer(&self, theme: &Theme, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let key = self.key.clone();
        let sql = self.sql.clone();
        let options = self.options.clone();
        let on_close = self.on_close.clone();

        div()
            .px_4()
            .py_3()
            .border_t_1()
            .border_color(theme.border)
            .flex()
            .justify_end()
            .gap_2()
            .child(
                div()
                    .px_4()
                    .py_2()
                    .text_sm()
                    .text_color(theme.text_secondary)
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.surface_hover))
                    .rounded_md()
                    .on_click({
                        let on_close = on_close.clone();
                        move |event, cx| on_close(event, cx)
                    })
                    .child("Cancel"),
            )
            .child(
                div()
                    .px_4()
                    .py_2()
                    .text_sm()
                    .bg(theme.accent)
                    .text_color(theme.on_accent)
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.accent_hover))
                    .on_click(cx.listener(move |_, event, cx| {
                        let state = cx.global::<PlanViewerState>();
                        state.explain(key.clone(), sql.clone(), options.clone(), cx);
                        on_close(event, cx);
                    }))
                    .child("⚡ Run EXPLAIN"),
            )
    }
}
```

### 19.6 Plan Tree View (GPUI)

```rust
// src/components/plan/plan_tree_view.rs

use crate::models::plan::*;
use crate::state::plan_state::*;
use crate::theme::Theme;
use gpui::*;

pub struct PlanTreeView {
    key: PlanViewerKey,
    root: PlanNode,
}

impl PlanTreeView {
    pub fn new(key: PlanViewerKey, root: PlanNode) -> Self {
        Self { key, root }
    }
}

impl Render for PlanTreeView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let state = cx.global::<PlanViewerState>();
        let instance = state.get_or_create_instance(&self.key);

        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(self.render_node(&self.root, &instance, theme, cx))
            // Detail panel when node is selected
            .when_some(instance.selected_node_id.as_ref(), |el, node_id| {
                if let Some(node) = state.find_node(&self.key, node_id) {
                    el.child(
                        div()
                            .mt_4()
                            .pt_4()
                            .border_t_1()
                            .border_color(theme.border)
                            .child(PlanNodeDetail::new(node)),
                    )
                } else {
                    el
                }
            })
    }
}

impl PlanTreeView {
    fn render_node(
        &self,
        node: &PlanNode,
        instance: &PlanViewerInstance,
        theme: &Theme,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let is_expanded = instance.expanded_nodes.contains(&node.node_id);
        let is_selected = instance.selected_node_id.as_ref() == Some(&node.node_id);
        let has_children = !node.children.is_empty();

        let key = self.key.clone();
        let node_id = node.node_id.clone();
        let node_id_toggle = node.node_id.clone();

        div()
            .flex()
            .flex_col()
            // Node row
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .py_1()
                    .px_2()
                    .rounded_md()
                    .cursor_pointer()
                    .pl(px(node.depth as f32 * 20.0 + 8.0))
                    .when(is_selected, |s| s.bg(theme.selection_background))
                    .when(!is_selected, |s| s.hover(|s| s.bg(theme.surface_hover)))
                    .on_click(cx.listener(move |_, _, cx| {
                        let state = cx.global::<PlanViewerState>();
                        state.select_node(&key, Some(node_id.clone()));
                        cx.notify();
                    }))
                    // Expand/collapse toggle
                    .child(
                        div()
                            .w_5()
                            .h_5()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(theme.text_secondary)
                            .when(has_children, |s| {
                                let key = self.key.clone();
                                s.cursor_pointer()
                                    .hover(|s| s.text_color(theme.text))
                                    .on_click(cx.listener(move |_, event: &ClickEvent, cx| {
                                        event.stop_propagation();
                                        let state = cx.global::<PlanViewerState>();
                                        state.toggle_node(&key, &node_id_toggle);
                                        cx.notify();
                                    }))
                                    .child(if is_expanded { "▼" } else { "▶" })
                            }),
                    )
                    // Node content
                    .child(self.render_node_row(node, theme)),
            )
            // Children
            .when(is_expanded && has_children, |el| {
                let mut children_el = el;
                for child in &node.children {
                    children_el = children_el
                        .child(self.render_node(child, instance, theme, cx));
                }
                children_el
            })
    }

    fn render_node_row(&self, node: &PlanNode, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_3()
            .flex_1()
            .min_w_0()
            // Node icon
            .child(
                div()
                    .text_base()
                    .flex_shrink_0()
                    .child(get_node_icon(&node.node_type)),
            )
            // Node type and relation
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_sm()
                                    .child(&node.node_type),
                            )
                            .when_some(node.relation_name.as_ref(), |el, rel| {
                                el.child(
                                    div()
                                        .text_color(theme.text_secondary)
                                        .text_sm()
                                        .child(format!("on {}", rel)),
                                )
                            })
                            .when_some(node.index_name.as_ref(), |el, idx| {
                                el.child(
                                    div()
                                        .text_color(theme.info)
                                        .text_sm()
                                        .child(format!("using {}", idx)),
                                )
                            }),
                    ),
            )
            // Rows
            .child(
                div()
                    .text_right()
                    .text_sm()
                    .w(px(96.0))
                    .flex_shrink_0()
                    .when_some(node.actual_rows, |el, actual| {
                        el.child(
                            div()
                                .font_family("monospace")
                                .child(format_number(actual))
                                .when(actual != node.plan_rows, |s| {
                                    s.child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.text_muted)
                                            .child(format!("/ est. {}", format_number(node.plan_rows))),
                                    )
                                }),
                        )
                    })
                    .when(node.actual_rows.is_none(), |el| {
                        el.child(
                            div()
                                .font_family("monospace")
                                .text_color(theme.text_muted)
                                .child(format!("est. {}", format_number(node.plan_rows))),
                        )
                    }),
            )
            // Time bar
            .child(
                div()
                    .w(px(128.0))
                    .flex_shrink_0()
                    .when_some(node.actual_total_time, |el, time| {
                        let color = get_time_color(node.percent_of_total, theme);
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .flex_1()
                                        .h(px(8.0))
                                        .bg(theme.surface_alt)
                                        .rounded_sm()
                                        .overflow_hidden()
                                        .child(
                                            div()
                                                .h_full()
                                                .bg(color)
                                                .w(pct(node.percent_of_total.min(100.0) as f32)),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .font_family("monospace")
                                        .text_color(color)
                                        .w(px(64.0))
                                        .text_right()
                                        .child(format_time(time)),
                                ),
                        )
                    })
                    .when(node.actual_total_time.is_none(), |el| {
                        el.child(
                            div()
                                .text_sm()
                                .text_color(theme.text_muted)
                                .font_family("monospace")
                                .child(format!("cost: {:.0}", node.total_cost)),
                        )
                    }),
            )
            // Warnings badge
            .when(!node.warnings.is_empty(), |el| {
                let has_critical = node.warnings.iter().any(|w| w.severity == WarningSeverity::Critical);
                el.child(
                    div()
                        .flex_shrink_0()
                        .w_5()
                        .h_5()
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .when(has_critical, |s| {
                            s.bg(theme.error_background).text_color(theme.error)
                        })
                        .when(!has_critical, |s| {
                            s.bg(theme.warning_background).text_color(theme.warning)
                        })
                        .child(format!("{}", node.warnings.len())),
                )
            })
            // Slowest indicator
            .when(node.is_slowest, |el| {
                el.child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .px_2()
                        .py(px(2.0))
                        .bg(theme.error_background)
                        .text_color(theme.error)
                        .rounded_sm()
                        .child("SLOWEST"),
                )
            })
    }
}

fn get_node_icon(node_type: &str) -> &'static str {
    let lower = node_type.to_lowercase();
    if lower.contains("seq scan") {
        "📋"
    } else if lower.contains("index only scan") {
        "⚡"
    } else if lower.contains("index scan") {
        "🔍"
    } else if lower.contains("bitmap") {
        "🗺️"
    } else if lower.contains("nested loop") {
        "🔄"
    } else if lower.contains("hash join") {
        "#️⃣"
    } else if lower.contains("merge join") {
        "🔀"
    } else if lower.contains("sort") {
        "📊"
    } else if lower.contains("aggregate") {
        "∑"
    } else if lower.contains("hash") {
        "#️⃣"
    } else if lower.contains("materialize") {
        "💾"
    } else if lower.contains("cte") {
        "📦"
    } else if lower.contains("result") {
        "📤"
    } else if lower.contains("limit") {
        "🔢"
    } else if lower.contains("gather") {
        "🧲"
    } else if lower.contains("parallel") {
        "⚡"
    } else {
        "⚙️"
    }
}

fn get_time_color(percent: f64, theme: &Theme) -> Hsla {
    if percent >= 50.0 {
        theme.error
    } else if percent >= 25.0 {
        theme.warning
    } else if percent >= 10.0 {
        hsla(0.12, 0.9, 0.5, 1.0) // yellow
    } else {
        theme.success
    }
}

fn format_number(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_time(ms: f64) -> String {
    if ms >= 1000.0 {
        format!("{:.2}s", ms / 1000.0)
    } else {
        format!("{:.2}ms", ms)
    }
}
```

### 19.7 Plan Node Detail Panel (GPUI)

```rust
// src/components/plan/plan_node_detail.rs

use crate::models::plan::*;
use crate::theme::Theme;
use gpui::*;

pub struct PlanNodeDetail {
    node: PlanNode,
}

impl PlanNodeDetail {
    pub fn new(node: PlanNode) -> Self {
        Self { node }
    }
}

impl Render for PlanNodeDetail {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let node = &self.node;

        div()
            .bg(theme.surface_alt)
            .rounded_lg()
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            // Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(&node.node_type),
                    )
                    .when(node.is_slowest, |el| {
                        el.child(
                            div()
                                .text_xs()
                                .px_2()
                                .py_1()
                                .bg(theme.error_background)
                                .text_color(theme.error)
                                .rounded_sm()
                                .child("Slowest Node"),
                        )
                    }),
            )
            // Object info
            .when(node.relation_name.is_some() || node.index_name.is_some(), |el| {
                el.child(
                    div()
                        .grid()
                        .gap_4()
                        .when_some(node.relation_name.as_ref(), |el, rel| {
                            el.child(self.render_info_item(
                                "Table",
                                &format!(
                                    "{}{}",
                                    node.schema_name.as_ref().map(|s| format!("{}.", s)).unwrap_or_default(),
                                    rel
                                ),
                                theme,
                            ))
                        })
                        .when_some(node.index_name.as_ref(), |el, idx| {
                            el.child(self.render_info_item("Index", idx, theme))
                        })
                        .when(node.alias.is_some() && node.alias != node.relation_name, |el| {
                            el.child(self.render_info_item(
                                "Alias",
                                node.alias.as_ref().unwrap(),
                                theme,
                            ))
                        }),
                )
            })
            // Conditions
            .when(self.has_conditions(), |el| {
                el.child(self.render_conditions(theme))
            })
            // Estimates vs Actuals
            .child(self.render_estimates_actuals(theme))
            // Buffer stats
            .when(node.has_buffer_stats(), |el| {
                el.child(self.render_buffer_stats(theme))
            })
            // I/O timing
            .when(node.io_read_time.is_some() || node.io_write_time.is_some(), |el| {
                el.child(self.render_io_timing(theme))
            })
            // Sort info
            .when(node.sort_key.is_some(), |el| {
                el.child(self.render_sort_info(theme))
            })
            // Percent of total bar
            .child(self.render_percent_bar(theme))
            // Warnings
            .when(!node.warnings.is_empty(), |el| {
                el.child(self.render_warnings(theme))
            })
    }
}

impl PlanNodeDetail {
    fn render_info_item(&self, label: &str, value: &str, theme: &Theme) -> impl IntoElement {
        div()
            .text_sm()
            .child(
                div()
                    .text_color(theme.text_secondary)
                    .child(format!("{}:", label)),
            )
            .child(
                div()
                    .font_family("monospace")
                    .ml_2()
                    .child(value),
            )
    }

    fn has_conditions(&self) -> bool {
        self.node.filter.is_some()
            || self.node.index_cond.is_some()
            || self.node.recheck_cond.is_some()
            || self.node.hash_cond.is_some()
            || self.node.join_filter.is_some()
            || self.node.merge_cond.is_some()
    }

    fn render_conditions(&self, theme: &Theme) -> impl IntoElement {
        let node = &self.node;

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text_secondary)
                    .child("Conditions"),
            )
            .when_some(node.index_cond.as_ref(), |el, cond| {
                el.child(self.render_condition("Index Cond", cond, theme))
            })
            .when_some(node.filter.as_ref(), |el, cond| {
                el.child(self.render_condition("Filter", cond, theme))
            })
            .when_some(node.recheck_cond.as_ref(), |el, cond| {
                el.child(self.render_condition("Recheck Cond", cond, theme))
            })
            .when_some(node.hash_cond.as_ref(), |el, cond| {
                el.child(self.render_condition("Hash Cond", cond, theme))
            })
            .when_some(node.merge_cond.as_ref(), |el, cond| {
                el.child(self.render_condition("Merge Cond", cond, theme))
            })
            .when_some(node.join_filter.as_ref(), |el, cond| {
                el.child(self.render_condition("Join Filter", cond, theme))
            })
    }

    fn render_condition(&self, label: &str, value: &str, theme: &Theme) -> impl IntoElement {
        div()
            .text_sm()
            .child(
                div()
                    .text_color(theme.text_secondary)
                    .child(format!("{}:", label)),
            )
            .child(
                div()
                    .ml_2()
                    .px_2()
                    .py(px(2.0))
                    .bg(theme.surface)
                    .rounded_sm()
                    .font_family("monospace")
                    .text_xs()
                    .child(value),
            )
    }

    fn render_estimates_actuals(&self, theme: &Theme) -> impl IntoElement {
        let node = &self.node;

        div()
            .grid()
            .gap_4()
            // Estimated
            .child(
                div()
                    .bg(theme.surface)
                    .rounded_md()
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text_secondary)
                            .mb_2()
                            .child("Estimated"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .text_sm()
                            .child(self.render_stat_row("Rows", &format_number(node.plan_rows), theme))
                            .child(self.render_stat_row("Width", &format!("{} bytes", node.plan_width), theme))
                            .child(self.render_stat_row("Startup Cost", &format!("{:.2}", node.startup_cost), theme))
                            .child(self.render_stat_row("Total Cost", &format!("{:.2}", node.total_cost), theme)),
                    ),
            )
            // Actual (if available)
            .when(node.actual_rows.is_some(), |el| {
                el.child(
                    div()
                        .bg(theme.surface)
                        .rounded_md()
                        .p_3()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.text_secondary)
                                .mb_2()
                                .child("Actual"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .text_sm()
                                .child(self.render_stat_row(
                                    "Rows",
                                    &format_number(node.actual_rows.unwrap()),
                                    theme,
                                ))
                                .child(self.render_stat_row(
                                    "Loops",
                                    &format_number(node.actual_loops.unwrap_or(1)),
                                    theme,
                                ))
                                .child(self.render_stat_row(
                                    "Startup Time",
                                    &format_time(node.actual_startup_time.unwrap_or(0.0)),
                                    theme,
                                ))
                                .child(self.render_stat_row(
                                    "Total Time",
                                    &format_time(node.actual_total_time.unwrap_or(0.0)),
                                    theme,
                                )),
                        ),
                )
            })
    }

    fn render_stat_row(&self, label: &str, value: &str, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .justify_between()
            .child(div().child(label))
            .child(div().font_family("monospace").child(value))
    }

    fn render_buffer_stats(&self, theme: &Theme) -> impl IntoElement {
        let node = &self.node;

        div()
            .bg(theme.surface)
            .rounded_md()
            .p_3()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text_secondary)
                    .mb_2()
                    .child("Buffer Usage"),
            )
            .child(
                div()
                    .grid()
                    .gap_4()
                    .text_sm()
                    .child(self.render_stat_row(
                        "Shared Hit",
                        &format_number(node.shared_hit_blocks.unwrap_or(0)),
                        theme,
                    ))
                    .child(self.render_stat_row(
                        "Shared Read",
                        &format_number(node.shared_read_blocks.unwrap_or(0)),
                        theme,
                    ))
                    .child(self.render_stat_row(
                        "Shared Written",
                        &format_number(node.shared_written_blocks.unwrap_or(0)),
                        theme,
                    ))
                    .when(node.local_hit_blocks.is_some() || node.local_read_blocks.is_some(), |el| {
                        el.child(self.render_stat_row(
                            "Local Hit",
                            &format_number(node.local_hit_blocks.unwrap_or(0)),
                            theme,
                        ))
                        .child(self.render_stat_row(
                            "Local Read",
                            &format_number(node.local_read_blocks.unwrap_or(0)),
                            theme,
                        ))
                    })
                    .when(node.temp_read_blocks.is_some() || node.temp_written_blocks.is_some(), |el| {
                        el.child(self.render_stat_row(
                            "Temp Read",
                            &format_number(node.temp_read_blocks.unwrap_or(0)),
                            theme,
                        ))
                        .child(self.render_stat_row(
                            "Temp Written",
                            &format_number(node.temp_written_blocks.unwrap_or(0)),
                            theme,
                        ))
                    })
                    .when_some(node.buffer_hit_ratio(), |el, ratio| {
                        el.child(self.render_stat_row("Hit Ratio", &format!("{:.1}%", ratio), theme))
                    }),
            )
    }

    fn render_io_timing(&self, theme: &Theme) -> impl IntoElement {
        let node = &self.node;

        div()
            .bg(theme.surface)
            .rounded_md()
            .p_3()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text_secondary)
                    .mb_2()
                    .child("I/O Timing"),
            )
            .child(
                div()
                    .grid()
                    .gap_4()
                    .text_sm()
                    .child(self.render_stat_row(
                        "Read Time",
                        &format_time(node.io_read_time.unwrap_or(0.0)),
                        theme,
                    ))
                    .child(self.render_stat_row(
                        "Write Time",
                        &format_time(node.io_write_time.unwrap_or(0.0)),
                        theme,
                    )),
            )
    }

    fn render_sort_info(&self, theme: &Theme) -> impl IntoElement {
        let node = &self.node;

        div()
            .bg(theme.surface)
            .rounded_md()
            .p_3()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text_secondary)
                    .mb_2()
                    .child("Sort"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .text_sm()
                    .when_some(node.sort_key.as_ref(), |el, keys| {
                        el.child(
                            div()
                                .child(
                                    div().text_color(theme.text_secondary).child("Key:"),
                                )
                                .child(
                                    div()
                                        .ml_2()
                                        .font_family("monospace")
                                        .child(keys.join(", ")),
                                ),
                        )
                    })
                    .when_some(node.sort_method.as_ref(), |el, method| {
                        el.child(self.render_stat_row("Method", method, theme))
                    })
                    .when_some(node.sort_space_used, |el, space| {
                        let space_type = node
                            .sort_space_type
                            .map(|t| match t {
                                SortSpaceType::Memory => "Memory",
                                SortSpaceType::Disk => "Disk",
                            })
                            .unwrap_or("Unknown");
                        el.child(self.render_stat_row(
                            "Space Used",
                            &format!("{} KB ({})", space, space_type),
                            theme,
                        ))
                    }),
            )
    }

    fn render_percent_bar(&self, theme: &Theme) -> impl IntoElement {
        let node = &self.node;

        div()
            .flex()
            .items_center()
            .gap_4()
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text_secondary)
                    .child("% of Total:"),
            )
            .child(
                div()
                    .flex_1()
                    .h_3()
                    .bg(theme.surface)
                    .rounded_sm()
                    .overflow_hidden()
                    .child(
                        div()
                            .h_full()
                            .bg(theme.accent)
                            .w(pct(node.percent_of_total.min(100.0) as f32)),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .font_family("monospace")
                    .w(px(64.0))
                    .text_right()
                    .child(format!("{:.1}%", node.percent_of_total)),
            )
    }

    fn render_warnings(&self, theme: &Theme) -> impl IntoElement {
        let node = &self.node;

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text_secondary)
                    .child("Warnings"),
            )
            .children(node.warnings.iter().map(|warning| {
                let (bg, border, text) = match warning.severity {
                    WarningSeverity::Critical => {
                        (theme.error_background, theme.error, theme.error)
                    }
                    WarningSeverity::Warning => {
                        (theme.warning_background, theme.warning, theme.warning)
                    }
                    WarningSeverity::Info => {
                        (theme.info_background, theme.info, theme.info)
                    }
                };

                div()
                    .p_3()
                    .rounded_md()
                    .bg(bg)
                    .border_1()
                    .border_color(border)
                    .text_sm()
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(text)
                            .child(&warning.message),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_color(theme.text_secondary)
                            .child(format!("💡 {}", warning.suggestion)),
                    )
                    .when_some(warning.details.as_ref(), |el, details| {
                        el.child(
                            div()
                                .mt_1()
                                .text_xs()
                                .text_color(theme.text_muted)
                                .child(details),
                        )
                    })
            }))
    }
}

fn format_number(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_time(ms: f64) -> String {
    if ms >= 1000.0 {
        format!("{:.3}s", ms / 1000.0)
    } else {
        format!("{:.3}ms", ms)
    }
}
```

### 19.8 Timeline View (GPUI)

```rust
// src/components/plan/plan_timeline_view.rs

use crate::models::plan::*;
use crate::state::plan_state::*;
use crate::theme::Theme;
use gpui::*;

/// Timeline bar representation of a node
struct TimelineBar {
    node: PlanNode,
    start_percent: f64,
    width_percent: f64,
    row: usize,
}

pub struct PlanTimelineView {
    key: PlanViewerKey,
    plan: QueryPlan,
    bars: Vec<TimelineBar>,
    max_row: usize,
}

impl PlanTimelineView {
    pub fn new(key: PlanViewerKey, plan: QueryPlan) -> Self {
        let (bars, max_row) = Self::calculate_bars(&plan);
        Self {
            key,
            plan,
            bars,
            max_row,
        }
    }

    fn calculate_bars(plan: &QueryPlan) -> (Vec<TimelineBar>, usize) {
        let mut bars = Vec::new();
        let total_time = plan.root.actual_total_time.unwrap_or(plan.root.total_cost);

        if total_time == 0.0 {
            return (bars, 0);
        }

        let mut row_end_times: Vec<f64> = Vec::new();

        fn process_node(
            node: &PlanNode,
            total_time: f64,
            bars: &mut Vec<TimelineBar>,
            row_end_times: &mut Vec<f64>,
        ) {
            let node_time = node.actual_total_time.unwrap_or(node.total_cost);
            let start_time = node.actual_startup_time.unwrap_or(node.startup_cost);

            let start_percent = (start_time / total_time) * 100.0;
            let width_percent = ((node_time - start_time) / total_time * 100.0).max(1.0);

            // Find a row where this node fits
            let mut row = 0;
            for (i, end_time) in row_end_times.iter().enumerate() {
                if *end_time <= start_time {
                    row = i;
                    break;
                }
                row = i + 1;
            }

            if row >= row_end_times.len() {
                row_end_times.push(node_time);
            } else {
                row_end_times[row] = node_time;
            }

            bars.push(TimelineBar {
                node: node.clone(),
                start_percent,
                width_percent,
                row,
            });

            for child in &node.children {
                process_node(child, total_time, bars, row_end_times);
            }
        }

        process_node(&plan.root, total_time, &mut bars, &mut row_end_times);

        let max_row = row_end_times.len().saturating_sub(1);
        (bars, max_row)
    }
}

impl Render for PlanTimelineView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let state = cx.global::<PlanViewerState>();
        let instance = state.get_or_create_instance(&self.key);
        let total_time = self.plan.root.actual_total_time.unwrap_or(self.plan.root.total_cost);

        div()
            .flex()
            .flex_col()
            .gap_4()
            // Time axis
            .child(
                div()
                    .flex()
                    .items_center()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .px_2()
                    .child(div().child("0ms"))
                    .child(div().flex_1())
                    .child(div().child(format_time(total_time))),
            )
            // Timeline bars
            .child(
                div()
                    .relative()
                    .h(px((self.max_row + 1) as f32 * 36.0))
                    .children(self.bars.iter().map(|bar| {
                        let key = self.key.clone();
                        let node_id = bar.node.node_id.clone();
                        let is_selected = instance.selected_node_id.as_ref() == Some(&bar.node.node_id);
                        let color = get_time_color(bar.node.percent_of_total, theme);

                        div()
                            .absolute()
                            .h_8()
                            .rounded_md()
                            .flex()
                            .items_center()
                            .px_2()
                            .text_xs()
                            .text_color(gpui::white())
                            .overflow_hidden()
                            .cursor_pointer()
                            .bg(color)
                            .when(is_selected, |s| {
                                s.outline_2().outline_color(theme.accent).outline_offset(px(2.0))
                            })
                            .hover(|s| s.outline_2().outline_color(theme.accent))
                            .left(pct(bar.start_percent as f32))
                            .w(pct(bar.width_percent as f32))
                            .min_w(px(60.0))
                            .top(px(bar.row as f32 * 36.0))
                            .on_click(cx.listener(move |_, _, cx| {
                                let state = cx.global::<PlanViewerState>();
                                state.select_node(&key, Some(node_id.clone()));
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .truncate()
                                    .child(format!(
                                        "{}{}",
                                        bar.node.node_type,
                                        bar.node
                                            .relation_name
                                            .as_ref()
                                            .map(|r| format!(" ({})", r))
                                            .unwrap_or_default()
                                    )),
                            )
                    })),
            )
            // Legend
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_4()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .child(self.render_legend_item("< 10%", theme.success, theme))
                    .child(self.render_legend_item("10-25%", hsla(0.12, 0.9, 0.5, 1.0), theme))
                    .child(self.render_legend_item("25-50%", theme.warning, theme))
                    .child(self.render_legend_item("> 50%", theme.error, theme)),
            )
    }
}

impl PlanTimelineView {
    fn render_legend_item(
        &self,
        label: &str,
        color: Hsla,
        theme: &Theme,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(div().w_3().h_3().rounded_sm().bg(color))
            .child(label)
    }
}

fn get_time_color(percent: f64, theme: &Theme) -> Hsla {
    if percent >= 50.0 {
        theme.error
    } else if percent >= 25.0 {
        theme.warning
    } else if percent >= 10.0 {
        hsla(0.12, 0.9, 0.5, 1.0) // yellow
    } else {
        theme.success
    }
}

fn format_time(ms: f64) -> String {
    if ms >= 1000.0 {
        format!("{:.2}s", ms / 1000.0)
    } else {
        format!("{:.2}ms", ms)
    }
}
```

### 19.9 Text View (GPUI)

```rust
// src/components/plan/plan_text_view.rs

use crate::theme::Theme;
use gpui::*;
use regex::Regex;

pub struct PlanTextView {
    raw: String,
    highlighted_spans: Vec<HighlightedSpan>,
}

struct HighlightedSpan {
    text: String,
    style: SpanStyle,
}

#[derive(Clone, Copy)]
enum SpanStyle {
    Normal,
    NodeType,
    Relation,
    Cost,
    Rows,
    Time,
    Condition,
    Keyword,
}

impl PlanTextView {
    pub fn new(raw: String) -> Self {
        let highlighted_spans = Self::highlight_text(&raw);
        Self {
            raw,
            highlighted_spans,
        }
    }

    fn highlight_text(text: &str) -> Vec<HighlightedSpan> {
        // For simplicity, we'll do line-by-line highlighting
        // A full implementation would use regex to identify and colorize patterns

        let mut spans = Vec::new();

        for line in text.lines() {
            // Check for node types
            let node_types = [
                "Seq Scan", "Index Scan", "Index Only Scan", "Bitmap Heap Scan",
                "Bitmap Index Scan", "Nested Loop", "Hash Join", "Merge Join",
                "Sort", "Aggregate", "Hash", "Materialize", "CTE Scan", "Result",
                "Limit", "Unique", "Append", "GroupAggregate", "HashAggregate",
                "Gather", "Gather Merge",
            ];

            let mut current_line = line.to_string();
            let mut line_spans = Vec::new();

            // Simple approach: push the whole line with appropriate style detection
            let style = if node_types.iter().any(|nt| line.contains(nt)) {
                SpanStyle::NodeType
            } else if line.contains("cost=") || line.contains("rows=") {
                SpanStyle::Cost
            } else if line.contains("actual time=") || line.contains("Execution Time:") || line.contains("Planning Time:") {
                SpanStyle::Time
            } else if line.contains("Filter:") || line.contains("Index Cond:") || line.contains("Join Filter:") {
                SpanStyle::Condition
            } else {
                SpanStyle::Normal
            };

            spans.push(HighlightedSpan {
                text: format!("{}\n", line),
                style,
            });
        }

        spans
    }
}

impl Render for PlanTextView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .font_family("monospace")
            .text_sm()
            .whitespace_pre()
            .overflow_auto()
            .p_4()
            .bg(theme.surface_alt)
            .rounded_md()
            .children(self.highlighted_spans.iter().map(|span| {
                let color = match span.style {
                    SpanStyle::Normal => theme.text,
                    SpanStyle::NodeType => theme.info,
                    SpanStyle::Relation => theme.accent,
                    SpanStyle::Cost => theme.warning,
                    SpanStyle::Rows => theme.success,
                    SpanStyle::Time => theme.error,
                    SpanStyle::Condition => hsla(0.08, 0.8, 0.6, 1.0), // amber
                    SpanStyle::Keyword => theme.text_secondary,
                };

                div()
                    .text_color(color)
                    .when(matches!(span.style, SpanStyle::NodeType), |s| {
                        s.font_weight(FontWeight::MEDIUM)
                    })
                    .child(&span.text)
            }))
    }
}
```

### 19.10 Warnings List (GPUI)

```rust
// src/components/plan/plan_warnings_list.rs

use crate::models::plan::*;
use crate::state::plan_state::*;
use crate::theme::Theme;
use gpui::*;

pub struct PlanWarningsList {
    key: PlanViewerKey,
}

impl PlanWarningsList {
    pub fn new(key: PlanViewerKey) -> Self {
        Self { key }
    }
}

impl Render for PlanWarningsList {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let state = cx.global::<PlanViewerState>();
        let warnings = state.get_all_warnings(&self.key);

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format!("Warnings ({})", warnings.len())),
            )
            .children(warnings.iter().map(|(node, warning)| {
                let key = self.key.clone();
                let node_id = node.node_id.clone();

                let (bg, border, icon) = match warning.severity {
                    WarningSeverity::Critical => (theme.error_background, theme.error, "🔴"),
                    WarningSeverity::Warning => (theme.warning_background, theme.warning, "🟡"),
                    WarningSeverity::Info => (theme.info_background, theme.info, "🔵"),
                };

                div()
                    .w_full()
                    .text_left()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .bg(bg)
                    .border_color(border)
                    .cursor_pointer()
                    .hover(|s| s.opacity(0.9))
                    .on_click(cx.listener(move |_, _, cx| {
                        let state = cx.global::<PlanViewerState>();
                        state.toggle_warnings_only(&key);
                        state.select_node(&key, Some(node_id.clone()));
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap_2()
                            .child(div().text_lg().child(icon))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.text)
                                            .child(&warning.message),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.text_secondary)
                                            .mt(px(2.0))
                                            .child(format!(
                                                "Node: {}{}",
                                                node.node_type,
                                                node.relation_name
                                                    .as_ref()
                                                    .map(|r| format!(" on {}", r))
                                                    .unwrap_or_default()
                                            )),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.text_muted)
                                            .mt_1()
                                            .child(format!("💡 {}", warning.suggestion)),
                                    ),
                            )
                            .child(
                                div()
                                    .w_5()
                                    .h_5()
                                    .text_color(theme.text_muted)
                                    .child("→"),
                            ),
                    )
            }))
    }
}
```

### 19.11 Keyboard Shortcuts

```rust
// Keyboard shortcuts for plan viewer
// Add to src/actions/plan.rs

use gpui::*;

actions!(
    plan,
    [
        RunExplain,
        RunExplainAnalyze,
        SwitchToTreeView,
        SwitchToTimelineView,
        SwitchToTextView,
        ExpandAll,
        CollapseAll,
        ToggleWarnings,
        SelectNextNode,
        SelectPrevNode,
        ExpandNode,
        CollapseNode,
    ]
);

// Register in keymap
pub fn register_plan_keybindings(cx: &mut AppContext) {
    cx.bind_keys([
        KeyBinding::new("cmd-e", RunExplain, None),
        KeyBinding::new("cmd-shift-e", RunExplainAnalyze, None),
        KeyBinding::new("cmd-1", SwitchToTreeView, Some("PlanViewer")),
        KeyBinding::new("cmd-2", SwitchToTimelineView, Some("PlanViewer")),
        KeyBinding::new("cmd-3", SwitchToTextView, Some("PlanViewer")),
        KeyBinding::new("cmd-shift-]", ExpandAll, Some("PlanViewer")),
        KeyBinding::new("cmd-shift-[", CollapseAll, Some("PlanViewer")),
        KeyBinding::new("cmd-w", ToggleWarnings, Some("PlanViewer")),
        KeyBinding::new("down", SelectNextNode, Some("PlanTreeView")),
        KeyBinding::new("up", SelectPrevNode, Some("PlanTreeView")),
        KeyBinding::new("right", ExpandNode, Some("PlanTreeView")),
        KeyBinding::new("left", CollapseNode, Some("PlanTreeView")),
    ]);
}
```

## Acceptance Criteria

1. **EXPLAIN Execution**
   - [ ] Support all EXPLAIN options (ANALYZE, VERBOSE, COSTS, BUFFERS, TIMING, WAL)
   - [ ] Parse JSON format into structured plan tree
   - [ ] Handle text format for display
   - [ ] Show warning for ANALYZE with DML statements
   - [ ] Execute EXPLAIN via direct service call (no IPC)

2. **Tree View**
   - [ ] Display hierarchical plan structure with GPUI
   - [ ] Show node type, table/index names
   - [ ] Display row estimates vs actuals
   - [ ] Color-code by execution time percentage
   - [ ] Expand/collapse nodes with animation
   - [ ] Highlight slowest node

3. **Timeline View**
   - [ ] Horizontal bars showing execution time
   - [ ] Proper positioning based on start/end times
   - [ ] Handle parallel operations (stacked rows)
   - [ ] Click to select node
   - [ ] GPU-accelerated rendering

4. **Text View**
   - [ ] Display raw EXPLAIN output
   - [ ] Syntax highlighting for plan elements
   - [ ] Monospace font rendering

5. **Node Details**
   - [ ] Show all node properties
   - [ ] Display conditions (filter, index cond, etc.)
   - [ ] Show buffer statistics with hit ratio
   - [ ] Display I/O timing
   - [ ] Show row estimates vs actuals comparison

6. **Warnings**
   - [ ] Detect sequential scans on large tables
   - [ ] Identify row estimate mismatches
   - [ ] Warn on nested loops with high counts
   - [ ] Alert on disk spills (sort, hash)
   - [ ] Detect low buffer hit ratios
   - [ ] Provide actionable suggestions

## MCP Testing Instructions

### Tauri MCP Testing

```typescript
// Start driver session
await mcp___hypothesi_tauri_mcp_server__driver_session({
    action: 'start',
    port: 9223
});

// Take screenshot of empty plan viewer
await mcp___hypothesi_tauri_mcp_server__webview_screenshot({
    filePath: '/tmp/plan-viewer-empty.png'
});

// Click Explain button
await mcp___hypothesi_tauri_mcp_server__webview_interact({
    action: 'click',
    selector: 'button:has-text("Explain")'
});

// Wait for options dialog
await mcp___hypothesi_tauri_mcp_server__webview_wait_for({
    type: 'text',
    value: 'EXPLAIN Options'
});

// Get DOM snapshot of dialog
const dialogSnapshot = await mcp___hypothesi_tauri_mcp_server__webview_dom_snapshot({
    type: 'accessibility',
    selector: '[role="dialog"]'
});

// Check ANALYZE option
await mcp___hypothesi_tauri_mcp_server__webview_interact({
    action: 'click',
    selector: 'label:has-text("ANALYZE")'
});

// Run EXPLAIN
await mcp___hypothesi_tauri_mcp_server__webview_interact({
    action: 'click',
    selector: 'button:has-text("Run EXPLAIN")'
});

// Wait for plan to load
await mcp___hypothesi_tauri_mcp_server__webview_wait_for({
    type: 'text',
    value: 'Planning Time'
});

// Screenshot tree view
await mcp___hypothesi_tauri_mcp_server__webview_screenshot({
    filePath: '/tmp/plan-tree-view.png'
});

// Switch to timeline view
await mcp___hypothesi_tauri_mcp_server__webview_interact({
    action: 'click',
    selector: 'button:has-text("Timeline")'
});

// Screenshot timeline view
await mcp___hypothesi_tauri_mcp_server__webview_screenshot({
    filePath: '/tmp/plan-timeline-view.png'
});

// Click on a node in tree view
await mcp___hypothesi_tauri_mcp_server__webview_interact({
    action: 'click',
    selector: 'button:has-text("Tree")'
});

await mcp___hypothesi_tauri_mcp_server__webview_interact({
    action: 'click',
    selector: '[role="treeitem"]:first-child'
});

// Verify detail panel shows
await mcp___hypothesi_tauri_mcp_server__webview_wait_for({
    type: 'text',
    value: 'Estimated'
});

// Screenshot with detail panel
await mcp___hypothesi_tauri_mcp_server__webview_screenshot({
    filePath: '/tmp/plan-detail-panel.png'
});

// Test expand/collapse
await mcp___hypothesi_tauri_mcp_server__webview_interact({
    action: 'click',
    selector: 'button:has-text("Expand All")'
});

// Verify warnings display
const hasWarnings = await mcp___hypothesi_tauri_mcp_server__webview_find_element({
    selector: 'button:has-text("Warning")'
});

if (hasWarnings) {
    await mcp___hypothesi_tauri_mcp_server__webview_interact({
        action: 'click',
        selector: 'button:has-text("Warning")'
    });

    await mcp___hypothesi_tauri_mcp_server__webview_screenshot({
        filePath: '/tmp/plan-warnings.png'
    });
}
```

### Playwright MCP Testing (for isolated component testing)

```typescript
// Navigate to plan viewer test page
await mcp__playwright__browser_navigate({
    url: 'http://localhost:5173/test/plan-viewer'
});

// Take accessibility snapshot
await mcp__playwright__browser_snapshot({
    filename: '/tmp/plan-viewer-snapshot.md'
});

// Test EXPLAIN options dialog
await mcp__playwright__browser_click({
    element: 'Explain button',
    ref: 'button:has-text("Explain")'
});

await mcp__playwright__browser_wait_for({
    text: 'EXPLAIN Options'
});

// Fill form with options
await mcp__playwright__browser_fill_form({
    fields: [
        { name: 'ANALYZE', type: 'checkbox', ref: '#analyze', value: 'true' },
        { name: 'BUFFERS', type: 'checkbox', ref: '#buffers', value: 'true' },
    ]
});

// Select JSON format
await mcp__playwright__browser_click({
    element: 'JSON format option',
    ref: 'button:has-text("JSON")'
});

// Run explain
await mcp__playwright__browser_click({
    element: 'Run EXPLAIN button',
    ref: 'button:has-text("Run EXPLAIN")'
});

// Wait for results
await mcp__playwright__browser_wait_for({
    text: 'Planning Time'
});

// Take screenshot
await mcp__playwright__browser_take_screenshot({
    filename: 'plan-viewer-results.png'
});

// Test keyboard navigation
await mcp__playwright__browser_press_key({ key: 'ArrowDown' });
await mcp__playwright__browser_press_key({ key: 'ArrowDown' });
await mcp__playwright__browser_press_key({ key: 'ArrowRight' }); // Expand

// Test view switching with keyboard
await mcp__playwright__browser_press_key({ key: 'Meta+2' }); // Timeline view
await mcp__playwright__browser_take_screenshot({
    filename: 'plan-timeline-keyboard.png'
});
```
