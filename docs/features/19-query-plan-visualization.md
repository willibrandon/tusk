# Feature 19: Query Plan Visualization

## Overview

Query plan visualization displays EXPLAIN output in interactive visual formats, helping developers understand query performance characteristics, identify bottlenecks, and optimize queries effectively.

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

## Technical Specification

### 19.1 Query Plan Data Models

```typescript
// src/lib/types/plan.ts

export interface QueryPlan {
	raw: string;
	format: 'text' | 'json';
	root: PlanNode;
	planningTime: number;
	executionTime?: number; // Only with ANALYZE
	triggers?: TriggerTiming[];
	totalTime: number;
	jitInfo?: JitInfo;
}

export interface PlanNode {
	// Identity
	nodeId: string; // Generated unique ID
	nodeType: string;

	// Source objects
	relationName?: string;
	alias?: string;
	schemaName?: string;
	indexName?: string;
	cteName?: string;

	// Join info
	joinType?: 'Inner' | 'Left' | 'Right' | 'Full' | 'Semi' | 'Anti';

	// Estimates
	startupCost: number;
	totalCost: number;
	planRows: number;
	planWidth: number;

	// Actuals (ANALYZE only)
	actualStartupTime?: number;
	actualTotalTime?: number;
	actualRows?: number;
	actualLoops?: number;

	// Conditions
	filter?: string;
	indexCond?: string;
	recheckCond?: string;
	joinFilter?: string;
	hashCond?: string;
	tidCond?: string;

	// Sorting
	sortKey?: string[];
	sortMethod?: string;
	sortSpaceUsed?: number;
	sortSpaceType?: 'Memory' | 'Disk';

	// Hashing
	hashBuckets?: number;
	hashBatches?: number;
	peakMemoryUsage?: number;

	// Buffer stats (BUFFERS option)
	sharedHitBlocks?: number;
	sharedReadBlocks?: number;
	sharedDirtiedBlocks?: number;
	sharedWrittenBlocks?: number;
	localHitBlocks?: number;
	localReadBlocks?: number;
	localDirtiedBlocks?: number;
	localWrittenBlocks?: number;
	tempReadBlocks?: number;
	tempWrittenBlocks?: number;

	// I/O timing (TIMING option)
	ioReadTime?: number;
	ioWriteTime?: number;

	// Workers (parallel queries)
	workersPlanned?: number;
	workersLaunched?: number;
	workerDetails?: WorkerDetail[];

	// Children
	children: PlanNode[];

	// Computed values for visualization
	percentOfTotal: number;
	exclusiveTime: number; // Time excluding children
	isSlowest: boolean;
	warnings: PlanWarning[];
	depth: number;
	rowsRemoved?: number; // Rows removed by filter
}

export interface WorkerDetail {
	workerId: number;
	actualStartupTime: number;
	actualTotalTime: number;
	actualRows: number;
	actualLoops: number;
}

export interface TriggerTiming {
	triggerName: string;
	relation: string;
	time: number;
	calls: number;
}

export interface JitInfo {
	functions: number;
	options: {
		inlining: boolean;
		optimization: boolean;
		expressions: boolean;
		deforming: boolean;
	};
	timing: {
		generation: number;
		inlining: number;
		optimization: number;
		emission: number;
		total: number;
	};
}

export interface PlanWarning {
	type: WarningType;
	severity: 'info' | 'warning' | 'critical';
	message: string;
	suggestion: string;
}

export type WarningType =
	| 'seq_scan_large_table'
	| 'row_estimate_mismatch'
	| 'nested_loop_high_loops'
	| 'sort_on_disk'
	| 'hash_exceeds_work_mem'
	| 'unused_index'
	| 'missing_index'
	| 'filter_removes_most_rows';

export interface ExplainOptions {
	analyze: boolean;
	verbose: boolean;
	costs: boolean;
	buffers: boolean;
	timing: boolean;
	wal: boolean;
	format: 'text' | 'json' | 'xml' | 'yaml';
}
```

### 19.2 Plan Parser (Rust)

```rust
// src-tauri/src/services/plan.rs

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanNode {
    pub node_id: String,
    pub node_type: String,

    // Source objects
    pub relation_name: Option<String>,
    pub alias: Option<String>,
    pub schema_name: Option<String>,
    pub index_name: Option<String>,
    pub cte_name: Option<String>,

    // Join info
    pub join_type: Option<String>,

    // Estimates
    pub startup_cost: f64,
    pub total_cost: f64,
    pub plan_rows: i64,
    pub plan_width: i32,

    // Actuals
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
    pub tid_cond: Option<String>,

    // Sorting
    pub sort_key: Option<Vec<String>>,
    pub sort_method: Option<String>,
    pub sort_space_used: Option<i64>,
    pub sort_space_type: Option<String>,

    // Hashing
    pub hash_buckets: Option<i64>,
    pub hash_batches: Option<i64>,
    pub peak_memory_usage: Option<i64>,

    // Buffer stats
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

    // I/O timing
    pub io_read_time: Option<f64>,
    pub io_write_time: Option<f64>,

    // Parallel
    pub workers_planned: Option<i32>,
    pub workers_launched: Option<i32>,
    pub worker_details: Vec<WorkerDetail>,

    // Children
    pub children: Vec<PlanNode>,

    // Computed
    pub percent_of_total: f64,
    pub exclusive_time: f64,
    pub is_slowest: bool,
    pub warnings: Vec<PlanWarning>,
    pub depth: i32,
    pub rows_removed: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerDetail {
    pub worker_id: i32,
    pub actual_startup_time: f64,
    pub actual_total_time: f64,
    pub actual_rows: i64,
    pub actual_loops: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerTiming {
    pub trigger_name: String,
    pub relation: String,
    pub time: f64,
    pub calls: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanWarning {
    pub warning_type: String,
    pub severity: String,
    pub message: String,
    pub suggestion: String,
}

pub struct PlanParser;

impl PlanParser {
    /// Parse EXPLAIN JSON output into a QueryPlan
    pub fn parse_json(json_str: &str) -> Result<QueryPlan, PlanError> {
        let value: Value = serde_json::from_str(json_str)?;

        // EXPLAIN JSON returns an array with one element
        let plan_array = value.as_array()
            .ok_or_else(|| PlanError::ParseError("Expected array".to_string()))?;

        let plan_obj = plan_array.first()
            .ok_or_else(|| PlanError::ParseError("Empty plan array".to_string()))?;

        let planning_time = plan_obj.get("Planning Time")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let execution_time = plan_obj.get("Execution Time")
            .and_then(|v| v.as_f64());

        let total_time = planning_time + execution_time.unwrap_or(0.0);

        // Parse triggers
        let triggers = Self::parse_triggers(plan_obj.get("Triggers"));

        // Parse JIT info
        let jit_info = Self::parse_jit(plan_obj.get("JIT"));

        // Parse the plan tree
        let plan_value = plan_obj.get("Plan")
            .ok_or_else(|| PlanError::ParseError("Missing Plan field".to_string()))?;

        let mut root = Self::parse_node(plan_value, 0)?;

        // Calculate derived values
        let root_time = root.actual_total_time.unwrap_or(root.total_cost);
        Self::calculate_percentages(&mut root, root_time);
        Self::calculate_exclusive_times(&mut root);
        Self::mark_slowest_node(&mut root);
        Self::detect_warnings(&mut root);

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

    fn parse_node(value: &Value, depth: i32) -> Result<PlanNode, PlanError> {
        let obj = value.as_object()
            .ok_or_else(|| PlanError::ParseError("Expected object for node".to_string()))?;

        let node_type = obj.get("Node Type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PlanError::ParseError("Missing Node Type".to_string()))?
            .to_string();

        // Parse children recursively
        let mut children = Vec::new();
        if let Some(plans) = obj.get("Plans").and_then(|v| v.as_array()) {
            for child_value in plans {
                children.push(Self::parse_node(child_value, depth + 1)?);
            }
        }

        // Parse worker details
        let mut worker_details = Vec::new();
        if let Some(workers) = obj.get("Workers").and_then(|v| v.as_array()) {
            for (i, worker) in workers.iter().enumerate() {
                if let Some(w) = worker.as_object() {
                    worker_details.push(WorkerDetail {
                        worker_id: i as i32,
                        actual_startup_time: w.get("Actual Startup Time")
                            .and_then(|v| v.as_f64()).unwrap_or(0.0),
                        actual_total_time: w.get("Actual Total Time")
                            .and_then(|v| v.as_f64()).unwrap_or(0.0),
                        actual_rows: w.get("Actual Rows")
                            .and_then(|v| v.as_i64()).unwrap_or(0),
                        actual_loops: w.get("Actual Loops")
                            .and_then(|v| v.as_i64()).unwrap_or(1),
                    });
                }
            }
        }

        // Calculate rows removed by filter
        let actual_rows = obj.get("Actual Rows").and_then(|v| v.as_i64());
        let rows_removed_by_filter = obj.get("Rows Removed by Filter")
            .and_then(|v| v.as_i64());
        let rows_removed_by_index_recheck = obj.get("Rows Removed by Index Recheck")
            .and_then(|v| v.as_i64());

        let rows_removed = match (rows_removed_by_filter, rows_removed_by_index_recheck) {
            (Some(a), Some(b)) => Some(a + b),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        Ok(PlanNode {
            node_id: Uuid::new_v4().to_string(),
            node_type,
            relation_name: Self::get_str(obj, "Relation Name"),
            alias: Self::get_str(obj, "Alias"),
            schema_name: Self::get_str(obj, "Schema"),
            index_name: Self::get_str(obj, "Index Name"),
            cte_name: Self::get_str(obj, "CTE Name"),
            join_type: Self::get_str(obj, "Join Type"),
            startup_cost: Self::get_f64(obj, "Startup Cost").unwrap_or(0.0),
            total_cost: Self::get_f64(obj, "Total Cost").unwrap_or(0.0),
            plan_rows: Self::get_i64(obj, "Plan Rows").unwrap_or(0),
            plan_width: Self::get_i64(obj, "Plan Width").unwrap_or(0) as i32,
            actual_startup_time: Self::get_f64(obj, "Actual Startup Time"),
            actual_total_time: Self::get_f64(obj, "Actual Total Time"),
            actual_rows,
            actual_loops: Self::get_i64(obj, "Actual Loops"),
            filter: Self::get_str(obj, "Filter"),
            index_cond: Self::get_str(obj, "Index Cond"),
            recheck_cond: Self::get_str(obj, "Recheck Cond"),
            join_filter: Self::get_str(obj, "Join Filter"),
            hash_cond: Self::get_str(obj, "Hash Cond"),
            tid_cond: Self::get_str(obj, "TID Cond"),
            sort_key: obj.get("Sort Key").and_then(|v| {
                v.as_array().map(|arr| {
                    arr.iter().filter_map(|s| s.as_str().map(String::from)).collect()
                })
            }),
            sort_method: Self::get_str(obj, "Sort Method"),
            sort_space_used: Self::get_i64(obj, "Sort Space Used"),
            sort_space_type: Self::get_str(obj, "Sort Space Type"),
            hash_buckets: Self::get_i64(obj, "Hash Buckets"),
            hash_batches: Self::get_i64(obj, "Hash Batches"),
            peak_memory_usage: Self::get_i64(obj, "Peak Memory Usage"),
            shared_hit_blocks: Self::get_i64(obj, "Shared Hit Blocks"),
            shared_read_blocks: Self::get_i64(obj, "Shared Read Blocks"),
            shared_dirtied_blocks: Self::get_i64(obj, "Shared Dirtied Blocks"),
            shared_written_blocks: Self::get_i64(obj, "Shared Written Blocks"),
            local_hit_blocks: Self::get_i64(obj, "Local Hit Blocks"),
            local_read_blocks: Self::get_i64(obj, "Local Read Blocks"),
            local_dirtied_blocks: Self::get_i64(obj, "Local Dirtied Blocks"),
            local_written_blocks: Self::get_i64(obj, "Local Written Blocks"),
            temp_read_blocks: Self::get_i64(obj, "Temp Read Blocks"),
            temp_written_blocks: Self::get_i64(obj, "Temp Written Blocks"),
            io_read_time: Self::get_f64(obj, "I/O Read Time"),
            io_write_time: Self::get_f64(obj, "I/O Write Time"),
            workers_planned: Self::get_i64(obj, "Workers Planned").map(|v| v as i32),
            workers_launched: Self::get_i64(obj, "Workers Launched").map(|v| v as i32),
            worker_details,
            children,
            percent_of_total: 0.0,
            exclusive_time: 0.0,
            is_slowest: false,
            warnings: Vec::new(),
            depth,
            rows_removed,
        })
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

    fn parse_triggers(value: Option<&Value>) -> Vec<TriggerTiming> {
        let Some(arr) = value.and_then(|v| v.as_array()) else {
            return Vec::new();
        };

        arr.iter().filter_map(|t| {
            let obj = t.as_object()?;
            Some(TriggerTiming {
                trigger_name: obj.get("Trigger Name")?.as_str()?.to_string(),
                relation: obj.get("Relation")?.as_str()?.to_string(),
                time: obj.get("Time")?.as_f64()?,
                calls: obj.get("Calls")?.as_i64()?,
            })
        }).collect()
    }

    fn parse_jit(value: Option<&Value>) -> Option<JitInfo> {
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

    fn calculate_percentages(node: &mut PlanNode, total_time: f64) {
        if total_time > 0.0 {
            let node_time = node.actual_total_time.unwrap_or(node.total_cost);
            node.percent_of_total = (node_time / total_time) * 100.0;
        }

        for child in &mut node.children {
            Self::calculate_percentages(child, total_time);
        }
    }

    fn calculate_exclusive_times(node: &mut PlanNode) {
        let node_time = node.actual_total_time.unwrap_or(0.0);
        let children_time: f64 = node.children.iter()
            .filter_map(|c| c.actual_total_time)
            .sum();

        node.exclusive_time = (node_time - children_time).max(0.0);

        for child in &mut node.children {
            Self::calculate_exclusive_times(child);
        }
    }

    fn mark_slowest_node(node: &mut PlanNode) {
        let mut slowest_time = 0.0;
        let mut slowest_id = String::new();

        Self::find_slowest(node, &mut slowest_time, &mut slowest_id);
        Self::set_slowest(node, &slowest_id);
    }

    fn find_slowest(node: &PlanNode, slowest_time: &mut f64, slowest_id: &mut String) {
        if node.exclusive_time > *slowest_time {
            *slowest_time = node.exclusive_time;
            *slowest_id = node.node_id.clone();
        }

        for child in &node.children {
            Self::find_slowest(child, slowest_time, slowest_id);
        }
    }

    fn set_slowest(node: &mut PlanNode, slowest_id: &str) {
        node.is_slowest = node.node_id == slowest_id;

        for child in &mut node.children {
            Self::set_slowest(child, slowest_id);
        }
    }

    fn detect_warnings(node: &mut PlanNode) {
        node.warnings.clear();

        // Sequential scan on large table
        if node.node_type == "Seq Scan" {
            let rows = node.actual_rows.or(Some(node.plan_rows)).unwrap_or(0);
            if rows > 10_000 {
                node.warnings.push(PlanWarning {
                    warning_type: "seq_scan_large_table".to_string(),
                    severity: "warning".to_string(),
                    message: format!(
                        "Sequential scan on {} rows",
                        rows
                    ),
                    suggestion: "Consider adding an index on the filtered columns".to_string(),
                });
            }
        }

        // Row estimate mismatch
        if let Some(actual) = node.actual_rows {
            let estimated = node.plan_rows;
            if estimated > 0 && actual > 0 {
                let ratio = (actual as f64) / (estimated as f64);
                if ratio > 10.0 || ratio < 0.1 {
                    node.warnings.push(PlanWarning {
                        warning_type: "row_estimate_mismatch".to_string(),
                        severity: "warning".to_string(),
                        message: format!(
                            "Actual rows ({}) differ significantly from estimate ({})",
                            actual, estimated
                        ),
                        suggestion: "Run ANALYZE on the table to update statistics".to_string(),
                    });
                }
            }
        }

        // Nested loop with high loop count
        if node.node_type == "Nested Loop" {
            if let Some(loops) = node.actual_loops {
                if loops > 1000 {
                    node.warnings.push(PlanWarning {
                        warning_type: "nested_loop_high_loops".to_string(),
                        severity: "warning".to_string(),
                        message: format!("Nested loop executed {} times", loops),
                        suggestion: "Consider using a hash or merge join by adding appropriate indexes".to_string(),
                    });
                }
            }
        }

        // Sort spilling to disk
        if node.node_type == "Sort" && node.sort_space_type.as_deref() == Some("Disk") {
            node.warnings.push(PlanWarning {
                warning_type: "sort_on_disk".to_string(),
                severity: "critical".to_string(),
                message: format!(
                    "Sort spilled to disk ({} KB)",
                    node.sort_space_used.unwrap_or(0)
                ),
                suggestion: "Increase work_mem or add an index to avoid sorting".to_string(),
            });
        }

        // Hash batches > 1 indicates work_mem exceeded
        if let Some(batches) = node.hash_batches {
            if batches > 1 {
                node.warnings.push(PlanWarning {
                    warning_type: "hash_exceeds_work_mem".to_string(),
                    severity: "warning".to_string(),
                    message: format!(
                        "Hash used {} batches (indicates work_mem exceeded)",
                        batches
                    ),
                    suggestion: "Consider increasing work_mem for this query".to_string(),
                });
            }
        }

        // Filter removes most rows
        if let (Some(actual), Some(removed)) = (node.actual_rows, node.rows_removed) {
            if removed > 0 && actual > 0 {
                let ratio = removed as f64 / (actual + removed) as f64;
                if ratio > 0.9 {
                    node.warnings.push(PlanWarning {
                        warning_type: "filter_removes_most_rows".to_string(),
                        severity: "info".to_string(),
                        message: format!(
                            "Filter removed {}% of rows ({} of {})",
                            (ratio * 100.0) as i32,
                            removed,
                            actual + removed
                        ),
                        suggestion: "Consider adding a partial index with this filter condition".to_string(),
                    });
                }
            }
        }

        for child in &mut node.children {
            Self::detect_warnings(child);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Plan parse error: {0}")]
    ParseError(String),
}
```

### 19.3 Tauri Commands

```rust
// src-tauri/src/commands/plan.rs

use tauri::State;
use crate::services::plan::{PlanParser, QueryPlan, ExplainOptions};
use crate::state::AppState;
use crate::error::Error;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainRequest {
    pub conn_id: String,
    pub sql: String,
    pub options: ExplainOptions,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainOptions {
    pub analyze: bool,
    pub verbose: bool,
    pub costs: bool,
    pub buffers: bool,
    pub timing: bool,
    pub wal: bool,
    pub format: String,
}

impl Default for ExplainOptions {
    fn default() -> Self {
        Self {
            analyze: false,
            verbose: false,
            costs: true,
            buffers: false,
            timing: true,
            wal: false,
            format: "json".to_string(),
        }
    }
}

#[tauri::command]
pub async fn explain_query(
    state: State<'_, AppState>,
    request: ExplainRequest,
) -> Result<QueryPlan, Error> {
    let pool = state.get_connection(&request.conn_id)?;
    let client = pool.get().await?;

    // Build EXPLAIN command
    let mut explain_parts = vec!["EXPLAIN".to_string()];
    let mut options = Vec::new();

    if request.options.analyze {
        options.push("ANALYZE");
    }
    if request.options.verbose {
        options.push("VERBOSE");
    }
    if request.options.costs {
        options.push("COSTS");
    }
    if request.options.buffers {
        options.push("BUFFERS");
    }
    if request.options.timing {
        options.push("TIMING");
    }
    if request.options.wal {
        options.push("WAL");
    }

    options.push(match request.options.format.as_str() {
        "json" => "FORMAT JSON",
        "xml" => "FORMAT XML",
        "yaml" => "FORMAT YAML",
        _ => "FORMAT TEXT",
    });

    if !options.is_empty() {
        explain_parts.push(format!("({})", options.join(", ")));
    }

    explain_parts.push(request.sql);

    let explain_sql = explain_parts.join(" ");

    // Execute EXPLAIN
    let rows = client.query(&explain_sql, &[]).await?;

    // Parse result based on format
    match request.options.format.as_str() {
        "json" => {
            // JSON format returns single row with QUERY PLAN column
            let json_str: String = rows[0].get(0);
            let plan = PlanParser::parse_json(&json_str)?;
            Ok(plan)
        }
        _ => {
            // Text format - collect all rows
            let text: String = rows.iter()
                .map(|row| row.get::<_, String>(0))
                .collect::<Vec<_>>()
                .join("\n");

            // For text format, return raw without detailed parsing
            Ok(QueryPlan {
                raw: text,
                format: crate::services::plan::PlanFormat::Text,
                root: crate::services::plan::PlanNode {
                    node_id: uuid::Uuid::new_v4().to_string(),
                    node_type: "Root".to_string(),
                    relation_name: None,
                    alias: None,
                    schema_name: None,
                    index_name: None,
                    cte_name: None,
                    join_type: None,
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
                    tid_cond: None,
                    sort_key: None,
                    sort_method: None,
                    sort_space_used: None,
                    sort_space_type: None,
                    hash_buckets: None,
                    hash_batches: None,
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
                    workers_planned: None,
                    workers_launched: None,
                    worker_details: Vec::new(),
                    children: Vec::new(),
                    percent_of_total: 0.0,
                    exclusive_time: 0.0,
                    is_slowest: false,
                    warnings: Vec::new(),
                    depth: 0,
                    rows_removed: None,
                },
                planning_time: 0.0,
                execution_time: None,
                triggers: Vec::new(),
                total_time: 0.0,
                jit_info: None,
            })
        }
    }
}
```

### 19.4 Plan Store (Svelte)

```typescript
// src/lib/stores/planStore.svelte.ts

import { invoke } from '@tauri-apps/api/core';
import type { QueryPlan, PlanNode, ExplainOptions, PlanWarning } from '$lib/types/plan';

export type PlanViewMode = 'tree' | 'timeline' | 'text';

interface PlanState {
	plan: QueryPlan | null;
	loading: boolean;
	error: string | null;
	viewMode: PlanViewMode;
	selectedNodeId: string | null;
	expandedNodes: Set<string>;
	showOnlyWarnings: boolean;
}

export function createPlanStore() {
	let state = $state<PlanState>({
		plan: null,
		loading: false,
		error: null,
		viewMode: 'tree',
		selectedNodeId: null,
		expandedNodes: new Set(),
		showOnlyWarnings: false
	});

	const defaultOptions: ExplainOptions = {
		analyze: true,
		verbose: false,
		costs: true,
		buffers: true,
		timing: true,
		wal: false,
		format: 'json'
	};

	async function explain(connId: string, sql: string, options?: Partial<ExplainOptions>) {
		state.loading = true;
		state.error = null;

		try {
			const mergedOptions = { ...defaultOptions, ...options };

			const plan = await invoke<QueryPlan>('explain_query', {
				request: {
					connId,
					sql,
					options: mergedOptions
				}
			});

			state.plan = plan;
			state.selectedNodeId = null;

			// Auto-expand first two levels
			state.expandedNodes = new Set();
			expandToDepth(plan.root, 2);
		} catch (err) {
			state.error = err instanceof Error ? err.message : String(err);
		} finally {
			state.loading = false;
		}
	}

	function expandToDepth(node: PlanNode, maxDepth: number) {
		if (node.depth < maxDepth) {
			state.expandedNodes.add(node.nodeId);
			for (const child of node.children) {
				expandToDepth(child, maxDepth);
			}
		}
	}

	function selectNode(nodeId: string | null) {
		state.selectedNodeId = nodeId;
	}

	function toggleNode(nodeId: string) {
		const newExpanded = new Set(state.expandedNodes);
		if (newExpanded.has(nodeId)) {
			newExpanded.delete(nodeId);
		} else {
			newExpanded.add(nodeId);
		}
		state.expandedNodes = newExpanded;
	}

	function expandAll() {
		if (!state.plan) return;

		const newExpanded = new Set<string>();
		function addAll(node: PlanNode) {
			newExpanded.add(node.nodeId);
			for (const child of node.children) {
				addAll(child);
			}
		}
		addAll(state.plan.root);
		state.expandedNodes = newExpanded;
	}

	function collapseAll() {
		state.expandedNodes = new Set();
	}

	function setViewMode(mode: PlanViewMode) {
		state.viewMode = mode;
	}

	function toggleWarningsOnly() {
		state.showOnlyWarnings = !state.showOnlyWarnings;
	}

	function clear() {
		state.plan = null;
		state.error = null;
		state.selectedNodeId = null;
		state.expandedNodes = new Set();
	}

	// Derived: Get all warnings from the plan
	const allWarnings = $derived.by(() => {
		if (!state.plan) return [];

		const warnings: Array<{ node: PlanNode; warning: PlanWarning }> = [];

		function collectWarnings(node: PlanNode) {
			for (const warning of node.warnings) {
				warnings.push({ node, warning });
			}
			for (const child of node.children) {
				collectWarnings(child);
			}
		}

		collectWarnings(state.plan.root);
		return warnings.sort((a, b) => {
			const severityOrder = { critical: 0, warning: 1, info: 2 };
			return (
				severityOrder[a.warning.severity as keyof typeof severityOrder] -
				severityOrder[b.warning.severity as keyof typeof severityOrder]
			);
		});
	});

	// Derived: Get selected node
	const selectedNode = $derived.by(() => {
		if (!state.plan || !state.selectedNodeId) return null;

		function findNode(node: PlanNode): PlanNode | null {
			if (node.nodeId === state.selectedNodeId) return node;
			for (const child of node.children) {
				const found = findNode(child);
				if (found) return found;
			}
			return null;
		}

		return findNode(state.plan.root);
	});

	return {
		get plan() {
			return state.plan;
		},
		get loading() {
			return state.loading;
		},
		get error() {
			return state.error;
		},
		get viewMode() {
			return state.viewMode;
		},
		get selectedNodeId() {
			return state.selectedNodeId;
		},
		get expandedNodes() {
			return state.expandedNodes;
		},
		get showOnlyWarnings() {
			return state.showOnlyWarnings;
		},
		get allWarnings() {
			return allWarnings;
		},
		get selectedNode() {
			return selectedNode;
		},

		explain,
		selectNode,
		toggleNode,
		expandAll,
		collapseAll,
		setViewMode,
		toggleWarningsOnly,
		clear,
		defaultOptions
	};
}

export const planStore = createPlanStore();
```

### 19.5 EXPLAIN Options Dialog

```svelte
<!-- src/lib/components/plan/ExplainOptionsDialog.svelte -->
<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { ExplainOptions } from '$lib/types/plan';

	interface Props {
		open: boolean;
		initialOptions?: Partial<ExplainOptions>;
	}

	let { open = $bindable(), initialOptions = {} }: Props = $props();

	const dispatch = createEventDispatcher<{
		run: ExplainOptions;
		cancel: void;
	}>();

	let options = $state<ExplainOptions>({
		analyze: true,
		verbose: false,
		costs: true,
		buffers: true,
		timing: true,
		wal: false,
		format: 'json',
		...initialOptions
	});

	function handleRun() {
		dispatch('run', options);
		open = false;
	}

	function handleCancel() {
		dispatch('cancel');
		open = false;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			handleCancel();
		} else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
			handleRun();
		}
	}
</script>

{#if open}
	<div
		class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
		onkeydown={handleKeydown}
		role="dialog"
		aria-modal="true"
		aria-labelledby="explain-dialog-title"
	>
		<div
			class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-[420px] max-h-[80vh] overflow-hidden"
		>
			<!-- Header -->
			<div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
				<h2 id="explain-dialog-title" class="text-lg font-semibold">EXPLAIN Options</h2>
			</div>

			<!-- Body -->
			<div class="p-4 space-y-4">
				<!-- Primary Options -->
				<div class="grid grid-cols-2 gap-4">
					<label class="flex items-center gap-2 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={options.analyze}
							class="rounded border-gray-300 dark:border-gray-600"
						/>
						<span class="text-sm">
							<span class="font-medium">ANALYZE</span>
							<span class="text-gray-500 dark:text-gray-400 block text-xs"> Execute query </span>
						</span>
					</label>

					<label class="flex items-center gap-2 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={options.buffers}
							class="rounded border-gray-300 dark:border-gray-600"
						/>
						<span class="text-sm">
							<span class="font-medium">BUFFERS</span>
							<span class="text-gray-500 dark:text-gray-400 block text-xs">
								Show buffer usage
							</span>
						</span>
					</label>

					<label class="flex items-center gap-2 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={options.verbose}
							class="rounded border-gray-300 dark:border-gray-600"
						/>
						<span class="text-sm">
							<span class="font-medium">VERBOSE</span>
							<span class="text-gray-500 dark:text-gray-400 block text-xs">
								Additional details
							</span>
						</span>
					</label>

					<label class="flex items-center gap-2 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={options.timing}
							class="rounded border-gray-300 dark:border-gray-600"
						/>
						<span class="text-sm">
							<span class="font-medium">TIMING</span>
							<span class="text-gray-500 dark:text-gray-400 block text-xs"> Show timing info </span>
						</span>
					</label>

					<label class="flex items-center gap-2 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={options.costs}
							class="rounded border-gray-300 dark:border-gray-600"
						/>
						<span class="text-sm">
							<span class="font-medium">COSTS</span>
							<span class="text-gray-500 dark:text-gray-400 block text-xs">
								Show cost estimates
							</span>
						</span>
					</label>

					<label class="flex items-center gap-2 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={options.wal}
							class="rounded border-gray-300 dark:border-gray-600"
						/>
						<span class="text-sm">
							<span class="font-medium">WAL</span>
							<span class="text-gray-500 dark:text-gray-400 block text-xs"> Show WAL usage </span>
						</span>
					</label>
				</div>

				<!-- Format Selection -->
				<div>
					<label class="block text-sm font-medium mb-2">Format</label>
					<select
						bind:value={options.format}
						class="w-full px-3 py-2 rounded border border-gray-300 dark:border-gray-600
                   bg-white dark:bg-gray-700 text-sm"
					>
						<option value="json">JSON (recommended)</option>
						<option value="text">Text</option>
						<option value="xml">XML</option>
						<option value="yaml">YAML</option>
					</select>
				</div>

				<!-- Warning for ANALYZE -->
				{#if options.analyze}
					<div
						class="p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200
                      dark:border-amber-800 rounded text-sm text-amber-800 dark:text-amber-200"
					>
						<strong>Note:</strong> ANALYZE will actually execute the query. For DML statements (INSERT,
						UPDATE, DELETE), changes will be made to the database.
					</div>
				{/if}
			</div>

			<!-- Footer -->
			<div
				class="px-4 py-3 border-t border-gray-200 dark:border-gray-700
                  flex justify-end gap-2"
			>
				<button
					onclick={handleCancel}
					class="px-4 py-2 text-sm text-gray-700 dark:text-gray-300
                 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
				>
					Cancel
				</button>
				<button
					onclick={handleRun}
					class="px-4 py-2 text-sm bg-blue-600 text-white rounded
                 hover:bg-blue-700 flex items-center gap-2"
				>
					<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M13 10V3L4 14h7v7l9-11h-7z"
						/>
					</svg>
					Run EXPLAIN
				</button>
			</div>
		</div>
	</div>
{/if}
```

### 19.6 Plan Tree View

```svelte
<!-- src/lib/components/plan/PlanTreeView.svelte -->
<script lang="ts">
	import type { PlanNode } from '$lib/types/plan';
	import { planStore } from '$lib/stores/planStore.svelte';
	import PlanNodeRow from './PlanNodeRow.svelte';
	import PlanNodeDetail from './PlanNodeDetail.svelte';

	interface Props {
		node: PlanNode;
		showDetail?: boolean;
	}

	let { node, showDetail = true }: Props = $props();

	const isExpanded = $derived(planStore.expandedNodes.has(node.nodeId));
	const isSelected = $derived(planStore.selectedNodeId === node.nodeId);
	const hasChildren = $derived(node.children.length > 0);
</script>

<div class="plan-tree">
	<!-- Node Row -->
	<div
		class="flex items-center gap-1 py-1 px-2 rounded cursor-pointer
           hover:bg-gray-100 dark:hover:bg-gray-700
           {isSelected ? 'bg-blue-100 dark:bg-blue-900/30' : ''}"
		style="padding-left: {node.depth * 20 + 8}px"
		onclick={() => planStore.selectNode(node.nodeId)}
		role="treeitem"
		aria-expanded={hasChildren ? isExpanded : undefined}
		aria-selected={isSelected}
	>
		<!-- Expand/Collapse Toggle -->
		{#if hasChildren}
			<button
				class="w-5 h-5 flex items-center justify-center text-gray-500
               hover:text-gray-700 dark:hover:text-gray-300"
				onclick={(e) => {
					e.stopPropagation();
					planStore.toggleNode(node.nodeId);
				}}
				aria-label={isExpanded ? 'Collapse' : 'Expand'}
			>
				<svg
					class="w-4 h-4 transform transition-transform {isExpanded ? 'rotate-90' : ''}"
					fill="none"
					stroke="currentColor"
					viewBox="0 0 24 24"
				>
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
				</svg>
			</button>
		{:else}
			<span class="w-5"></span>
		{/if}

		<!-- Node Content -->
		<PlanNodeRow {node} />
	</div>

	<!-- Children (recursively) -->
	{#if isExpanded && hasChildren}
		<div role="group">
			{#each node.children as child (child.nodeId)}
				<svelte:self node={child} {showDetail} />
			{/each}
		</div>
	{/if}
</div>

<!-- Detail Panel (only at root level) -->
{#if showDetail && node.depth === 0 && planStore.selectedNode}
	<div class="mt-4 border-t border-gray-200 dark:border-gray-700 pt-4">
		<PlanNodeDetail node={planStore.selectedNode} />
	</div>
{/if}
```

### 19.7 Plan Node Row

```svelte
<!-- src/lib/components/plan/PlanNodeRow.svelte -->
<script lang="ts">
	import type { PlanNode } from '$lib/types/plan';

	interface Props {
		node: PlanNode;
	}

	let { node }: Props = $props();

	// Color based on percentage of total time
	function getTimeColor(percent: number): string {
		if (percent >= 50) return 'text-red-600 dark:text-red-400';
		if (percent >= 25) return 'text-orange-600 dark:text-orange-400';
		if (percent >= 10) return 'text-yellow-600 dark:text-yellow-400';
		return 'text-green-600 dark:text-green-400';
	}

	// Background color for time bar
	function getTimeBarColor(percent: number): string {
		if (percent >= 50) return 'bg-red-500';
		if (percent >= 25) return 'bg-orange-500';
		if (percent >= 10) return 'bg-yellow-500';
		return 'bg-green-500';
	}

	function formatNumber(n: number): string {
		if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M';
		if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K';
		return n.toString();
	}

	function formatTime(ms: number): string {
		if (ms >= 1000) return (ms / 1000).toFixed(2) + 's';
		return ms.toFixed(2) + 'ms';
	}

	const nodeIcon = $derived.by(() => {
		const type = node.nodeType.toLowerCase();
		if (type.includes('seq scan')) return '📋';
		if (type.includes('index scan')) return '🔍';
		if (type.includes('index only scan')) return '⚡';
		if (type.includes('bitmap')) return '🗺️';
		if (type.includes('nested loop')) return '🔄';
		if (type.includes('hash join')) return '#️⃣';
		if (type.includes('merge join')) return '🔀';
		if (type.includes('sort')) return '📊';
		if (type.includes('aggregate')) return '∑';
		if (type.includes('hash')) return '#️⃣';
		if (type.includes('materialize')) return '💾';
		if (type.includes('cte')) return '📦';
		if (type.includes('result')) return '📤';
		return '⚙️';
	});

	const relationDisplay = $derived(
		node.relationName
			? node.alias && node.alias !== node.relationName
				? `${node.relationName} as ${node.alias}`
				: node.relationName
			: null
	);
</script>

<div class="flex items-center gap-3 flex-1 min-w-0">
	<!-- Node Icon -->
	<span class="text-base flex-shrink-0">{nodeIcon}</span>

	<!-- Node Type and Relation -->
	<div class="flex-1 min-w-0">
		<span class="font-medium text-sm">
			{node.nodeType}
		</span>
		{#if relationDisplay}
			<span class="text-gray-500 dark:text-gray-400 text-sm ml-1">
				on {relationDisplay}
			</span>
		{/if}
		{#if node.indexName}
			<span class="text-blue-600 dark:text-blue-400 text-sm ml-1">
				using {node.indexName}
			</span>
		{/if}
	</div>

	<!-- Rows -->
	<div class="text-right text-sm w-24 flex-shrink-0">
		{#if node.actualRows !== undefined}
			<span class="font-mono">
				{formatNumber(node.actualRows)}
			</span>
			{#if node.actualRows !== node.planRows}
				<span class="text-gray-400 text-xs">
					/ est. {formatNumber(node.planRows)}
				</span>
			{/if}
		{:else}
			<span class="font-mono text-gray-500">
				est. {formatNumber(node.planRows)}
			</span>
		{/if}
		<span class="text-gray-400 text-xs"> rows</span>
	</div>

	<!-- Time with visual bar -->
	<div class="w-32 flex-shrink-0">
		{#if node.actualTotalTime !== undefined}
			<div class="flex items-center gap-2">
				<div class="flex-1 h-2 bg-gray-200 dark:bg-gray-700 rounded overflow-hidden">
					<div
						class="h-full {getTimeBarColor(node.percentOfTotal)}"
						style="width: {Math.min(100, node.percentOfTotal)}%"
					></div>
				</div>
				<span class="text-sm font-mono {getTimeColor(node.percentOfTotal)} w-16 text-right">
					{formatTime(node.actualTotalTime)}
				</span>
			</div>
		{:else}
			<span class="text-sm text-gray-400 font-mono">
				cost: {node.totalCost.toFixed(0)}
			</span>
		{/if}
	</div>

	<!-- Warnings Badge -->
	{#if node.warnings.length > 0}
		<div class="flex-shrink-0">
			<span
				class="inline-flex items-center justify-center w-5 h-5 rounded-full text-xs
               {node.warnings.some((w) => w.severity === 'critical')
					? 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400'
					: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400'}"
				title="{node.warnings.length} warning(s)"
			>
				{node.warnings.length}
			</span>
		</div>
	{/if}

	<!-- Slowest indicator -->
	{#if node.isSlowest}
		<span
			class="flex-shrink-0 text-xs px-2 py-0.5 bg-red-100 text-red-700
             dark:bg-red-900/30 dark:text-red-400 rounded"
		>
			SLOWEST
		</span>
	{/if}
</div>
```

### 19.8 Plan Node Detail Panel

```svelte
<!-- src/lib/components/plan/PlanNodeDetail.svelte -->
<script lang="ts">
	import type { PlanNode } from '$lib/types/plan';

	interface Props {
		node: PlanNode;
	}

	let { node }: Props = $props();

	function formatBytes(blocks: number): string {
		const bytes = blocks * 8192; // Default block size
		if (bytes >= 1_073_741_824) return (bytes / 1_073_741_824).toFixed(2) + ' GB';
		if (bytes >= 1_048_576) return (bytes / 1_048_576).toFixed(2) + ' MB';
		if (bytes >= 1024) return (bytes / 1024).toFixed(2) + ' KB';
		return bytes + ' B';
	}

	function formatTime(ms: number): string {
		if (ms >= 1000) return (ms / 1000).toFixed(3) + 's';
		return ms.toFixed(3) + 'ms';
	}

	const hasBufferStats = $derived(
		node.sharedHitBlocks !== undefined ||
			node.sharedReadBlocks !== undefined ||
			node.localHitBlocks !== undefined ||
			node.tempReadBlocks !== undefined
	);

	const hasIoTiming = $derived(node.ioReadTime !== undefined || node.ioWriteTime !== undefined);
</script>

<div class="bg-gray-50 dark:bg-gray-900/50 rounded-lg p-4 space-y-4">
	<!-- Header -->
	<div class="flex items-center justify-between">
		<h3 class="text-lg font-semibold">
			{node.nodeType}
		</h3>
		{#if node.isSlowest}
			<span
				class="text-xs px-2 py-1 bg-red-100 text-red-700
                   dark:bg-red-900/30 dark:text-red-400 rounded"
			>
				Slowest Node
			</span>
		{/if}
	</div>

	<!-- Object Info -->
	{#if node.relationName || node.indexName}
		<div class="grid grid-cols-2 gap-4 text-sm">
			{#if node.relationName}
				<div>
					<span class="text-gray-500 dark:text-gray-400">Table:</span>
					<span class="ml-2 font-mono"
						>{node.schemaName ? `${node.schemaName}.` : ''}{node.relationName}</span
					>
				</div>
			{/if}
			{#if node.indexName}
				<div>
					<span class="text-gray-500 dark:text-gray-400">Index:</span>
					<span class="ml-2 font-mono">{node.indexName}</span>
				</div>
			{/if}
			{#if node.alias && node.alias !== node.relationName}
				<div>
					<span class="text-gray-500 dark:text-gray-400">Alias:</span>
					<span class="ml-2 font-mono">{node.alias}</span>
				</div>
			{/if}
		</div>
	{/if}

	<!-- Conditions -->
	{#if node.filter || node.indexCond || node.recheckCond || node.hashCond || node.joinFilter}
		<div class="space-y-2">
			<h4 class="text-sm font-medium text-gray-700 dark:text-gray-300">Conditions</h4>
			{#if node.indexCond}
				<div class="text-sm">
					<span class="text-gray-500 dark:text-gray-400">Index Cond:</span>
					<code class="ml-2 px-2 py-0.5 bg-gray-200 dark:bg-gray-700 rounded text-xs">
						{node.indexCond}
					</code>
				</div>
			{/if}
			{#if node.filter}
				<div class="text-sm">
					<span class="text-gray-500 dark:text-gray-400">Filter:</span>
					<code class="ml-2 px-2 py-0.5 bg-gray-200 dark:bg-gray-700 rounded text-xs">
						{node.filter}
					</code>
				</div>
			{/if}
			{#if node.recheckCond}
				<div class="text-sm">
					<span class="text-gray-500 dark:text-gray-400">Recheck Cond:</span>
					<code class="ml-2 px-2 py-0.5 bg-gray-200 dark:bg-gray-700 rounded text-xs">
						{node.recheckCond}
					</code>
				</div>
			{/if}
			{#if node.hashCond}
				<div class="text-sm">
					<span class="text-gray-500 dark:text-gray-400">Hash Cond:</span>
					<code class="ml-2 px-2 py-0.5 bg-gray-200 dark:bg-gray-700 rounded text-xs">
						{node.hashCond}
					</code>
				</div>
			{/if}
			{#if node.joinFilter}
				<div class="text-sm">
					<span class="text-gray-500 dark:text-gray-400">Join Filter:</span>
					<code class="ml-2 px-2 py-0.5 bg-gray-200 dark:bg-gray-700 rounded text-xs">
						{node.joinFilter}
					</code>
				</div>
			{/if}
		</div>
	{/if}

	<!-- Row Estimates vs Actuals -->
	<div class="grid grid-cols-2 gap-4">
		<div class="bg-white dark:bg-gray-800 rounded p-3">
			<h4 class="text-xs font-medium text-gray-500 dark:text-gray-400 mb-2">Estimated</h4>
			<div class="space-y-1 text-sm">
				<div class="flex justify-between">
					<span>Rows:</span>
					<span class="font-mono">{node.planRows.toLocaleString()}</span>
				</div>
				<div class="flex justify-between">
					<span>Width:</span>
					<span class="font-mono">{node.planWidth} bytes</span>
				</div>
				<div class="flex justify-between">
					<span>Startup Cost:</span>
					<span class="font-mono">{node.startupCost.toFixed(2)}</span>
				</div>
				<div class="flex justify-between">
					<span>Total Cost:</span>
					<span class="font-mono">{node.totalCost.toFixed(2)}</span>
				</div>
			</div>
		</div>

		{#if node.actualRows !== undefined}
			<div class="bg-white dark:bg-gray-800 rounded p-3">
				<h4 class="text-xs font-medium text-gray-500 dark:text-gray-400 mb-2">Actual</h4>
				<div class="space-y-1 text-sm">
					<div class="flex justify-between">
						<span>Rows:</span>
						<span class="font-mono">{node.actualRows.toLocaleString()}</span>
					</div>
					<div class="flex justify-between">
						<span>Loops:</span>
						<span class="font-mono">{node.actualLoops?.toLocaleString() ?? 1}</span>
					</div>
					<div class="flex justify-between">
						<span>Startup Time:</span>
						<span class="font-mono">{formatTime(node.actualStartupTime ?? 0)}</span>
					</div>
					<div class="flex justify-between">
						<span>Total Time:</span>
						<span class="font-mono">{formatTime(node.actualTotalTime ?? 0)}</span>
					</div>
				</div>
			</div>
		{/if}
	</div>

	<!-- Buffer Stats -->
	{#if hasBufferStats}
		<div class="bg-white dark:bg-gray-800 rounded p-3">
			<h4 class="text-xs font-medium text-gray-500 dark:text-gray-400 mb-2">Buffer Usage</h4>
			<div class="grid grid-cols-3 gap-4 text-sm">
				<div>
					<span class="text-gray-500">Shared Hit:</span>
					<span class="font-mono ml-1">{node.sharedHitBlocks ?? 0}</span>
				</div>
				<div>
					<span class="text-gray-500">Shared Read:</span>
					<span class="font-mono ml-1">{node.sharedReadBlocks ?? 0}</span>
				</div>
				<div>
					<span class="text-gray-500">Shared Written:</span>
					<span class="font-mono ml-1">{node.sharedWrittenBlocks ?? 0}</span>
				</div>
				{#if node.localHitBlocks || node.localReadBlocks}
					<div>
						<span class="text-gray-500">Local Hit:</span>
						<span class="font-mono ml-1">{node.localHitBlocks ?? 0}</span>
					</div>
					<div>
						<span class="text-gray-500">Local Read:</span>
						<span class="font-mono ml-1">{node.localReadBlocks ?? 0}</span>
					</div>
				{/if}
				{#if node.tempReadBlocks || node.tempWrittenBlocks}
					<div>
						<span class="text-gray-500">Temp Read:</span>
						<span class="font-mono ml-1">{node.tempReadBlocks ?? 0}</span>
					</div>
					<div>
						<span class="text-gray-500">Temp Written:</span>
						<span class="font-mono ml-1">{node.tempWrittenBlocks ?? 0}</span>
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<!-- I/O Timing -->
	{#if hasIoTiming}
		<div class="bg-white dark:bg-gray-800 rounded p-3">
			<h4 class="text-xs font-medium text-gray-500 dark:text-gray-400 mb-2">I/O Timing</h4>
			<div class="grid grid-cols-2 gap-4 text-sm">
				<div>
					<span class="text-gray-500">Read Time:</span>
					<span class="font-mono ml-1">{formatTime(node.ioReadTime ?? 0)}</span>
				</div>
				<div>
					<span class="text-gray-500">Write Time:</span>
					<span class="font-mono ml-1">{formatTime(node.ioWriteTime ?? 0)}</span>
				</div>
			</div>
		</div>
	{/if}

	<!-- Sort Info -->
	{#if node.sortKey}
		<div class="bg-white dark:bg-gray-800 rounded p-3">
			<h4 class="text-xs font-medium text-gray-500 dark:text-gray-400 mb-2">Sort</h4>
			<div class="space-y-1 text-sm">
				<div>
					<span class="text-gray-500">Key:</span>
					<code class="ml-2">{node.sortKey.join(', ')}</code>
				</div>
				{#if node.sortMethod}
					<div>
						<span class="text-gray-500">Method:</span>
						<span class="ml-1">{node.sortMethod}</span>
					</div>
				{/if}
				{#if node.sortSpaceUsed}
					<div>
						<span class="text-gray-500">Space Used:</span>
						<span class="ml-1 font-mono">{node.sortSpaceUsed} KB</span>
						<span class="text-gray-400">({node.sortSpaceType})</span>
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<!-- Percentage of Total -->
	<div class="flex items-center gap-4">
		<span class="text-sm text-gray-500 dark:text-gray-400"> % of Total: </span>
		<div class="flex-1 h-3 bg-gray-200 dark:bg-gray-700 rounded overflow-hidden">
			<div class="h-full bg-blue-500" style="width: {node.percentOfTotal}%"></div>
		</div>
		<span class="text-sm font-mono w-16 text-right">
			{node.percentOfTotal.toFixed(1)}%
		</span>
	</div>

	<!-- Warnings -->
	{#if node.warnings.length > 0}
		<div class="space-y-2">
			<h4 class="text-sm font-medium text-gray-700 dark:text-gray-300">Warnings</h4>
			{#each node.warnings as warning}
				<div
					class="p-3 rounded text-sm
                 {warning.severity === 'critical'
						? 'bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800'
						: warning.severity === 'warning'
							? 'bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800'
							: 'bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800'}"
				>
					<div
						class="font-medium
                      {warning.severity === 'critical'
							? 'text-red-700 dark:text-red-400'
							: warning.severity === 'warning'
								? 'text-amber-700 dark:text-amber-400'
								: 'text-blue-700 dark:text-blue-400'}"
					>
						{warning.message}
					</div>
					<div class="mt-1 text-gray-600 dark:text-gray-400">
						💡 {warning.suggestion}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
```

### 19.9 Timeline View

```svelte
<!-- src/lib/components/plan/PlanTimelineView.svelte -->
<script lang="ts">
	import type { QueryPlan, PlanNode } from '$lib/types/plan';
	import { planStore } from '$lib/stores/planStore.svelte';

	interface Props {
		plan: QueryPlan;
	}

	let { plan }: Props = $props();

	// Flatten the tree for timeline display
	interface TimelineNode {
		node: PlanNode;
		startPercent: number;
		widthPercent: number;
		row: number;
	}

	const timelineNodes = $derived.by(() => {
		const nodes: TimelineNode[] = [];
		const totalTime = plan.root.actualTotalTime ?? plan.root.totalCost;

		if (totalTime === 0) return nodes;

		let currentRow = 0;
		const rowEndTimes: number[] = [];

		function processNode(node: PlanNode) {
			const nodeTime = node.actualTotalTime ?? node.totalCost;
			const startTime = node.actualStartupTime ?? node.startupCost;

			const startPercent = (startTime / totalTime) * 100;
			const widthPercent = Math.max(1, ((nodeTime - startTime) / totalTime) * 100);

			// Find a row where this node fits
			let row = 0;
			const nodeEndTime = nodeTime;

			for (let i = 0; i < rowEndTimes.length; i++) {
				if (rowEndTimes[i] <= startTime) {
					row = i;
					rowEndTimes[i] = nodeEndTime;
					break;
				}
				row = i + 1;
			}

			if (row >= rowEndTimes.length) {
				rowEndTimes.push(nodeEndTime);
			}

			nodes.push({
				node,
				startPercent,
				widthPercent,
				row
			});

			for (const child of node.children) {
				processNode(child);
			}
		}

		processNode(plan.root);
		return nodes;
	});

	const maxRow = $derived(timelineNodes.reduce((max, n) => Math.max(max, n.row), 0));

	function getBarColor(percent: number): string {
		if (percent >= 50) return 'bg-red-500';
		if (percent >= 25) return 'bg-orange-500';
		if (percent >= 10) return 'bg-yellow-500';
		return 'bg-green-500';
	}

	function formatTime(ms: number): string {
		if (ms >= 1000) return (ms / 1000).toFixed(2) + 's';
		return ms.toFixed(2) + 'ms';
	}
</script>

<div class="space-y-4">
	<!-- Time axis -->
	<div class="flex items-center text-xs text-gray-500 dark:text-gray-400 px-2">
		<span>0ms</span>
		<span class="flex-1"></span>
		<span>{formatTime(plan.root.actualTotalTime ?? plan.root.totalCost)}</span>
	</div>

	<!-- Timeline bars -->
	<div class="relative" style="height: {(maxRow + 1) * 36}px">
		{#each timelineNodes as tl (tl.node.nodeId)}
			<button
				class="absolute h-8 rounded flex items-center px-2 text-xs text-white
               overflow-hidden cursor-pointer hover:ring-2 hover:ring-blue-400
               {getBarColor(tl.node.percentOfTotal)}
               {planStore.selectedNodeId === tl.node.nodeId ? 'ring-2 ring-blue-500' : ''}"
				style="
          left: {tl.startPercent}%;
          width: {tl.widthPercent}%;
          top: {tl.row * 36}px;
          min-width: 60px;
        "
				onclick={() => planStore.selectNode(tl.node.nodeId)}
				title="{tl.node.nodeType}: {formatTime(tl.node.actualTotalTime ?? 0)}"
			>
				<span class="truncate">
					{tl.node.nodeType}
					{#if tl.node.relationName}
						<span class="opacity-75">({tl.node.relationName})</span>
					{/if}
				</span>
			</button>
		{/each}
	</div>

	<!-- Legend -->
	<div class="flex items-center gap-4 text-xs text-gray-500 dark:text-gray-400 justify-center">
		<span class="flex items-center gap-1">
			<span class="w-3 h-3 rounded bg-green-500"></span>
			&lt; 10%
		</span>
		<span class="flex items-center gap-1">
			<span class="w-3 h-3 rounded bg-yellow-500"></span>
			10-25%
		</span>
		<span class="flex items-center gap-1">
			<span class="w-3 h-3 rounded bg-orange-500"></span>
			25-50%
		</span>
		<span class="flex items-center gap-1">
			<span class="w-3 h-3 rounded bg-red-500"></span>
			&gt; 50%
		</span>
	</div>
</div>
```

### 19.10 Query Plan Viewer Component

```svelte
<!-- src/lib/components/plan/QueryPlanViewer.svelte -->
<script lang="ts">
	import { planStore } from '$lib/stores/planStore.svelte';
	import PlanTreeView from './PlanTreeView.svelte';
	import PlanTimelineView from './PlanTimelineView.svelte';
	import PlanTextView from './PlanTextView.svelte';
	import PlanWarningsList from './PlanWarningsList.svelte';
	import ExplainOptionsDialog from './ExplainOptionsDialog.svelte';

	interface Props {
		connId: string;
		sql: string;
	}

	let { connId, sql }: Props = $props();

	let showOptionsDialog = $state(false);

	async function handleExplain(options: any) {
		await planStore.explain(connId, sql, options);
	}

	function formatTime(ms: number): string {
		if (ms >= 1000) return (ms / 1000).toFixed(3) + 's';
		return ms.toFixed(3) + 'ms';
	}
</script>

<div class="flex flex-col h-full">
	<!-- Toolbar -->
	<div class="flex items-center gap-2 px-4 py-2 border-b border-gray-200 dark:border-gray-700">
		<button
			onclick={() => (showOptionsDialog = true)}
			class="px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700
             flex items-center gap-2"
		>
			<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M13 10V3L4 14h7v7l9-11h-7z"
				/>
			</svg>
			Explain
		</button>

		{#if planStore.plan}
			<div
				class="flex items-center border border-gray-300 dark:border-gray-600 rounded overflow-hidden"
			>
				<button
					onclick={() => planStore.setViewMode('tree')}
					class="px-3 py-1.5 text-sm {planStore.viewMode === 'tree'
						? 'bg-gray-200 dark:bg-gray-700'
						: 'hover:bg-gray-100 dark:hover:bg-gray-800'}"
				>
					Tree
				</button>
				<button
					onclick={() => planStore.setViewMode('timeline')}
					class="px-3 py-1.5 text-sm {planStore.viewMode === 'timeline'
						? 'bg-gray-200 dark:bg-gray-700'
						: 'hover:bg-gray-100 dark:hover:bg-gray-800'}"
				>
					Timeline
				</button>
				<button
					onclick={() => planStore.setViewMode('text')}
					class="px-3 py-1.5 text-sm {planStore.viewMode === 'text'
						? 'bg-gray-200 dark:bg-gray-700'
						: 'hover:bg-gray-100 dark:hover:bg-gray-800'}"
				>
					Text
				</button>
			</div>

			<div class="flex items-center gap-2 ml-2">
				<button
					onclick={() => planStore.expandAll()}
					class="px-2 py-1 text-xs text-gray-600 dark:text-gray-400
                 hover:text-gray-900 dark:hover:text-gray-100"
					title="Expand All"
				>
					Expand All
				</button>
				<button
					onclick={() => planStore.collapseAll()}
					class="px-2 py-1 text-xs text-gray-600 dark:text-gray-400
                 hover:text-gray-900 dark:hover:text-gray-100"
					title="Collapse All"
				>
					Collapse All
				</button>
			</div>

			{#if planStore.allWarnings.length > 0}
				<button
					onclick={() => planStore.toggleWarningsOnly()}
					class="px-2 py-1 text-xs rounded flex items-center gap-1
                 {planStore.showOnlyWarnings
						? 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400'
						: 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800'}"
				>
					<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732
                     4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
						/>
					</svg>
					{planStore.allWarnings.length} Warning{planStore.allWarnings.length !== 1 ? 's' : ''}
				</button>
			{/if}

			<div class="flex-1"></div>

			<!-- Timing Summary -->
			<div class="text-sm text-gray-500 dark:text-gray-400">
				Planning: {formatTime(planStore.plan.planningTime)}
				{#if planStore.plan.executionTime !== undefined}
					<span class="mx-1">|</span>
					Execution: {formatTime(planStore.plan.executionTime)}
				{/if}
				<span class="mx-1">|</span>
				Total: {formatTime(planStore.plan.totalTime)}
			</div>
		{/if}
	</div>

	<!-- Content -->
	<div class="flex-1 overflow-auto p-4">
		{#if planStore.loading}
			<div class="flex items-center justify-center h-full">
				<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
			</div>
		{:else if planStore.error}
			<div
				class="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200
                  dark:border-red-800 rounded text-red-700 dark:text-red-400"
			>
				<strong>Error:</strong>
				{planStore.error}
			</div>
		{:else if planStore.plan}
			{#if planStore.showOnlyWarnings}
				<PlanWarningsList
					warnings={planStore.allWarnings}
					onSelectNode={(id) => {
						planStore.toggleWarningsOnly();
						planStore.selectNode(id);
					}}
				/>
			{:else if planStore.viewMode === 'tree'}
				<PlanTreeView node={planStore.plan.root} />
			{:else if planStore.viewMode === 'timeline'}
				<PlanTimelineView plan={planStore.plan} />
			{:else}
				<PlanTextView raw={planStore.plan.raw} />
			{/if}
		{:else}
			<div
				class="flex flex-col items-center justify-center h-full text-gray-500 dark:text-gray-400"
			>
				<svg
					class="w-16 h-16 mb-4 opacity-50"
					fill="none"
					stroke="currentColor"
					viewBox="0 0 24 24"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2
                   2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0
                   012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
					/>
				</svg>
				<p class="text-lg mb-2">No query plan</p>
				<p class="text-sm">Click "Explain" to analyze the query execution plan</p>
			</div>
		{/if}
	</div>
</div>

<ExplainOptionsDialog bind:open={showOptionsDialog} onrun={handleExplain} />
```

### 19.11 Text View Component

```svelte
<!-- src/lib/components/plan/PlanTextView.svelte -->
<script lang="ts">
	interface Props {
		raw: string;
	}

	let { raw }: Props = $props();

	// Apply syntax highlighting to plan text
	const highlightedText = $derived.by(() => {
		return (
			raw
				// Highlight node types
				.replace(
					/(Seq Scan|Index Scan|Index Only Scan|Bitmap Heap Scan|Bitmap Index Scan|Nested Loop|Hash Join|Merge Join|Sort|Aggregate|Hash|Materialize|CTE Scan|Result|Limit|Unique|Append|GroupAggregate|HashAggregate|Gather|Gather Merge)/g,
					'<span class="text-blue-600 dark:text-blue-400 font-medium">$1</span>'
				)
				// Highlight on/using
				.replace(
					/\b(on|using)\s+(\w+)/gi,
					'<span class="text-gray-500">$1</span> <span class="text-purple-600 dark:text-purple-400">$2</span>'
				)
				// Highlight costs and times
				.replace(
					/(cost=)([\d.]+)\.\.([\d.]+)/g,
					'<span class="text-gray-500">$1</span><span class="text-orange-600 dark:text-orange-400">$2..$3</span>'
				)
				.replace(
					/(rows=)(\d+)/g,
					'<span class="text-gray-500">$1</span><span class="text-green-600 dark:text-green-400">$2</span>'
				)
				.replace(
					/(width=)(\d+)/g,
					'<span class="text-gray-500">$1</span><span class="text-gray-600 dark:text-gray-400">$2</span>'
				)
				.replace(
					/(actual time=)([\d.]+)\.\.([\d.]+)/g,
					'<span class="text-gray-500">$1</span><span class="text-red-600 dark:text-red-400">$2..$3</span>'
				)
				// Highlight conditions
				.replace(
					/(Filter:|Index Cond:|Recheck Cond:|Hash Cond:|Join Filter:)\s*(.+)/g,
					'<span class="text-amber-600 dark:text-amber-400">$1</span> <span class="text-gray-700 dark:text-gray-300">$2</span>'
				)
				// Highlight timing at bottom
				.replace(
					/(Planning Time:|Execution Time:)\s*([\d.]+\s*ms)/g,
					'<span class="text-gray-500 font-medium">$1</span> <span class="text-cyan-600 dark:text-cyan-400">$2</span>'
				)
		);
	});
</script>

<pre
	class="font-mono text-sm whitespace-pre overflow-auto p-4
         bg-gray-50 dark:bg-gray-900/50 rounded">{@html highlightedText}</pre>
```

### 19.12 Warnings List Component

```svelte
<!-- src/lib/components/plan/PlanWarningsList.svelte -->
<script lang="ts">
	import type { PlanNode, PlanWarning } from '$lib/types/plan';

	interface WarningWithNode {
		node: PlanNode;
		warning: PlanWarning;
	}

	interface Props {
		warnings: WarningWithNode[];
		onSelectNode: (nodeId: string) => void;
	}

	let { warnings, onSelectNode }: Props = $props();

	function getSeverityIcon(severity: string): string {
		switch (severity) {
			case 'critical':
				return '🔴';
			case 'warning':
				return '🟡';
			case 'info':
				return '🔵';
			default:
				return '⚪';
		}
	}
</script>

<div class="space-y-3">
	<h3 class="text-lg font-semibold">
		Warnings ({warnings.length})
	</h3>

	{#each warnings as { node, warning }}
		<button
			class="w-full text-left p-3 rounded border transition-colors
             {warning.severity === 'critical'
				? 'bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800 hover:bg-red-100 dark:hover:bg-red-900/30'
				: warning.severity === 'warning'
					? 'bg-amber-50 dark:bg-amber-900/20 border-amber-200 dark:border-amber-800 hover:bg-amber-100 dark:hover:bg-amber-900/30'
					: 'bg-blue-50 dark:bg-blue-900/20 border-blue-200 dark:border-blue-800 hover:bg-blue-100 dark:hover:bg-blue-900/30'}"
			onclick={() => onSelectNode(node.nodeId)}
		>
			<div class="flex items-start gap-2">
				<span class="text-lg">{getSeverityIcon(warning.severity)}</span>
				<div class="flex-1 min-w-0">
					<div class="font-medium text-gray-900 dark:text-gray-100">
						{warning.message}
					</div>
					<div class="text-sm text-gray-600 dark:text-gray-400 mt-0.5">
						Node: {node.nodeType}
						{#if node.relationName}
							on {node.relationName}
						{/if}
					</div>
					<div class="text-sm text-gray-500 dark:text-gray-500 mt-1">
						💡 {warning.suggestion}
					</div>
				</div>
				<svg class="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
				</svg>
			</div>
		</button>
	{/each}
</div>
```

## Acceptance Criteria

1. **EXPLAIN Execution**
   - [ ] Support all EXPLAIN options (ANALYZE, VERBOSE, COSTS, BUFFERS, TIMING, WAL)
   - [ ] Parse JSON format into structured plan tree
   - [ ] Handle text format for display
   - [ ] Show warning for ANALYZE with DML statements

2. **Tree View**
   - [ ] Display hierarchical plan structure
   - [ ] Show node type, table/index names
   - [ ] Display row estimates vs actuals
   - [ ] Color-code by execution time percentage
   - [ ] Expand/collapse nodes
   - [ ] Highlight slowest node

3. **Timeline View**
   - [ ] Horizontal bars showing execution time
   - [ ] Proper positioning based on start/end times
   - [ ] Handle parallel operations
   - [ ] Click to select node

4. **Text View**
   - [ ] Display raw EXPLAIN output
   - [ ] Syntax highlighting for plan elements
   - [ ] Clickable node references

5. **Node Details**
   - [ ] Show all node properties
   - [ ] Display conditions (filter, index cond, etc.)
   - [ ] Show buffer statistics
   - [ ] Display I/O timing
   - [ ] Show row estimates vs actuals comparison

6. **Warnings**
   - [ ] Detect sequential scans on large tables
   - [ ] Identify row estimate mismatches
   - [ ] Warn on nested loops with high counts
   - [ ] Alert on disk spills (sort, hash)
   - [ ] Provide actionable suggestions

## MCP Testing Instructions

### Tauri MCP Testing

```typescript
// Test EXPLAIN execution
await mcp___hypothesi_tauri_mcp_server__ipc_execute_command({
	command: 'explain_query',
	args: {
		request: {
			connId: 'test-conn',
			sql: 'SELECT * FROM users WHERE email = $1',
			options: {
				analyze: true,
				buffers: true,
				timing: true,
				format: 'json'
			}
		}
	}
});

// Verify plan parsing
const snapshot = await mcp___hypothesi_tauri_mcp_server__webview_dom_snapshot({
	type: 'accessibility'
});
// Should show plan tree with nodes

// Test view mode switching
await mcp___hypothesi_tauri_mcp_server__webview_click({
	selector: 'button:has-text("Timeline")'
});

// Verify timeline view
await mcp___hypothesi_tauri_mcp_server__webview_wait_for({
	type: 'selector',
	value: '.plan-timeline'
});

// Test node selection
await mcp___hypothesi_tauri_mcp_server__webview_click({
	selector: '[role="treeitem"]:first-child'
});

// Verify detail panel shows
await mcp___hypothesi_tauri_mcp_server__webview_wait_for({
	type: 'text',
	value: 'Estimated'
});
```

### Playwright MCP Testing

```typescript
// Test EXPLAIN options dialog
await mcp__playwright__browser_click({
	element: 'Explain button',
	ref: 'button:has-text("Explain")'
});

// Verify dialog opens
await mcp__playwright__browser_wait_for({
	text: 'EXPLAIN Options'
});

// Configure options
await mcp__playwright__browser_click({
	element: 'ANALYZE checkbox',
	ref: 'input[type="checkbox"]:near(:text("ANALYZE"))'
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

// Take screenshot of plan visualization
await mcp__playwright__browser_take_screenshot({
	filename: 'query-plan-tree-view.png'
});

// Switch to timeline view
await mcp__playwright__browser_click({
	element: 'Timeline tab',
	ref: 'button:has-text("Timeline")'
});

await mcp__playwright__browser_take_screenshot({
	filename: 'query-plan-timeline-view.png'
});
```
