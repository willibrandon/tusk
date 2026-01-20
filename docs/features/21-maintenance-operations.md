# Feature 21: Maintenance Operations

## Overview

Maintenance operations provide GUI interfaces for PostgreSQL maintenance commands including VACUUM, ANALYZE, REINDEX, and CLUSTER. These dialogs expose all command options with helpful descriptions and execute operations with progress tracking.

## Goals

- Provide dialogs for VACUUM, ANALYZE, REINDEX, and CLUSTER
- Expose all command options with descriptions
- Show progress and output during execution
- Support targeting specific tables, indexes, or entire database
- Allow concurrent execution (CONCURRENTLY option where supported)

## Dependencies

- Feature 07: Connection Pool Management
- Feature 10: Schema Cache (for table/index lists)
- Feature 20: Admin Dashboard (for integration)

## Technical Specification

### 21.1 Maintenance Data Models

```typescript
// src/lib/types/maintenance.ts

export interface VacuumOptions {
	full: boolean;
	freeze: boolean;
	analyze: boolean;
	verbose: boolean;
	skipLocked: boolean;
	indexCleanup: 'auto' | 'on' | 'off';
	parallel: number; // 0 = auto
	truncate: boolean;
	processToast: boolean;
}

export interface ReindexOptions {
	concurrently: boolean;
	verbose: boolean;
	tablespace?: string;
}

export interface ReindexTarget {
	type: 'index' | 'table' | 'schema' | 'database';
	schema?: string;
	name?: string;
}

export interface AnalyzeOptions {
	verbose: boolean;
	skipLocked: boolean;
	columns?: string[];
}

export interface ClusterOptions {
	verbose: boolean;
	indexName?: string;
}

export interface MaintenanceJob {
	id: string;
	type: 'vacuum' | 'analyze' | 'reindex' | 'cluster';
	target: string;
	status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
	startTime: Date | null;
	endTime: Date | null;
	output: string[];
	error: string | null;
	progress?: number; // 0-100 if available
}

export interface MaintenanceResult {
	success: boolean;
	output: string[];
	duration: number;
	error?: string;
}
```

### 21.2 Maintenance Service (Rust)

```rust
// src-tauri/src/services/maintenance.rs

use serde::{Deserialize, Serialize};
use tokio_postgres::Client;
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VacuumOptions {
    pub full: bool,
    pub freeze: bool,
    pub analyze: bool,
    pub verbose: bool,
    pub skip_locked: bool,
    pub index_cleanup: String, // 'auto', 'on', 'off'
    pub parallel: i32,
    pub truncate: bool,
    pub process_toast: bool,
}

impl Default for VacuumOptions {
    fn default() -> Self {
        Self {
            full: false,
            freeze: false,
            analyze: false,
            verbose: true,
            skip_locked: false,
            index_cleanup: "auto".to_string(),
            parallel: 0,
            truncate: true,
            process_toast: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReindexOptions {
    pub concurrently: bool,
    pub verbose: bool,
    pub tablespace: Option<String>,
}

impl Default for ReindexOptions {
    fn default() -> Self {
        Self {
            concurrently: true,
            verbose: true,
            tablespace: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReindexTarget {
    pub target_type: String, // 'index', 'table', 'schema', 'database'
    pub schema: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeOptions {
    pub verbose: bool,
    pub skip_locked: bool,
    pub columns: Option<Vec<String>>,
}

impl Default for AnalyzeOptions {
    fn default() -> Self {
        Self {
            verbose: true,
            skip_locked: false,
            columns: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterOptions {
    pub verbose: bool,
    pub index_name: Option<String>,
}

impl Default for ClusterOptions {
    fn default() -> Self {
        Self {
            verbose: true,
            index_name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceJob {
    pub id: String,
    pub job_type: String,
    pub target: String,
    pub status: String,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub output: Vec<String>,
    pub error: Option<String>,
    pub progress: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceResult {
    pub success: bool,
    pub output: Vec<String>,
    pub duration_ms: i64,
    pub error: Option<String>,
}

pub struct MaintenanceService;

impl MaintenanceService {
    /// Execute VACUUM command
    pub async fn vacuum(
        client: &Client,
        target: Option<(&str, &str)>, // (schema, table) or None for all
        options: &VacuumOptions,
    ) -> Result<MaintenanceResult, MaintenanceError> {
        let start = std::time::Instant::now();

        // Build VACUUM command
        let mut sql = String::from("VACUUM");
        let mut opts = Vec::new();

        if options.full {
            opts.push("FULL");
        }
        if options.freeze {
            opts.push("FREEZE");
        }
        if options.verbose {
            opts.push("VERBOSE");
        }
        if options.analyze {
            opts.push("ANALYZE");
        }
        if options.skip_locked {
            opts.push("SKIP_LOCKED");
        }
        if options.index_cleanup != "auto" {
            opts.push(&format!("INDEX_CLEANUP {}", options.index_cleanup.to_uppercase()));
        }
        if options.parallel > 0 {
            opts.push(&format!("PARALLEL {}", options.parallel));
        }
        if !options.truncate {
            opts.push("TRUNCATE FALSE");
        }
        if !options.process_toast {
            opts.push("PROCESS_TOAST FALSE");
        }

        if !opts.is_empty() {
            sql.push_str(&format!(" ({})", opts.join(", ")));
        }

        if let Some((schema, table)) = target {
            sql.push_str(&format!(" {}.{}",
                Self::quote_ident(schema),
                Self::quote_ident(table)
            ));
        }

        // Execute
        let output = Self::execute_with_notices(client, &sql).await?;

        let duration = start.elapsed().as_millis() as i64;

        Ok(MaintenanceResult {
            success: true,
            output,
            duration_ms: duration,
            error: None,
        })
    }

    /// Execute ANALYZE command
    pub async fn analyze(
        client: &Client,
        target: Option<(&str, &str)>,
        options: &AnalyzeOptions,
    ) -> Result<MaintenanceResult, MaintenanceError> {
        let start = std::time::Instant::now();

        let mut sql = String::from("ANALYZE");
        let mut opts = Vec::new();

        if options.verbose {
            opts.push("VERBOSE");
        }
        if options.skip_locked {
            opts.push("SKIP_LOCKED");
        }

        if !opts.is_empty() {
            sql.push_str(&format!(" ({})", opts.join(", ")));
        }

        if let Some((schema, table)) = target {
            sql.push_str(&format!(" {}.{}",
                Self::quote_ident(schema),
                Self::quote_ident(table)
            ));

            // Add specific columns if provided
            if let Some(ref columns) = options.columns {
                if !columns.is_empty() {
                    let cols: Vec<String> = columns.iter()
                        .map(|c| Self::quote_ident(c))
                        .collect();
                    sql.push_str(&format!(" ({})", cols.join(", ")));
                }
            }
        }

        let output = Self::execute_with_notices(client, &sql).await?;
        let duration = start.elapsed().as_millis() as i64;

        Ok(MaintenanceResult {
            success: true,
            output,
            duration_ms: duration,
            error: None,
        })
    }

    /// Execute REINDEX command
    pub async fn reindex(
        client: &Client,
        target: &ReindexTarget,
        options: &ReindexOptions,
    ) -> Result<MaintenanceResult, MaintenanceError> {
        let start = std::time::Instant::now();

        let mut sql = String::from("REINDEX");
        let mut opts = Vec::new();

        if options.concurrently {
            opts.push("CONCURRENTLY");
        }
        if options.verbose {
            opts.push("VERBOSE");
        }
        if let Some(ref ts) = options.tablespace {
            opts.push(&format!("TABLESPACE {}", Self::quote_ident(ts)));
        }

        if !opts.is_empty() {
            sql.push_str(&format!(" ({})", opts.join(", ")));
        }

        // Add target type and name
        match target.target_type.as_str() {
            "index" => {
                sql.push_str(" INDEX");
                if let (Some(schema), Some(name)) = (&target.schema, &target.name) {
                    sql.push_str(&format!(" {}.{}",
                        Self::quote_ident(schema),
                        Self::quote_ident(name)
                    ));
                }
            }
            "table" => {
                sql.push_str(" TABLE");
                if let (Some(schema), Some(name)) = (&target.schema, &target.name) {
                    sql.push_str(&format!(" {}.{}",
                        Self::quote_ident(schema),
                        Self::quote_ident(name)
                    ));
                }
            }
            "schema" => {
                sql.push_str(" SCHEMA");
                if let Some(name) = &target.name {
                    sql.push_str(&format!(" {}", Self::quote_ident(name)));
                }
            }
            "database" => {
                sql.push_str(" DATABASE");
                // Uses current database
            }
            _ => return Err(MaintenanceError::InvalidTarget(target.target_type.clone())),
        }

        let output = Self::execute_with_notices(client, &sql).await?;
        let duration = start.elapsed().as_millis() as i64;

        Ok(MaintenanceResult {
            success: true,
            output,
            duration_ms: duration,
            error: None,
        })
    }

    /// Execute CLUSTER command
    pub async fn cluster(
        client: &Client,
        target: Option<(&str, &str)>,
        options: &ClusterOptions,
    ) -> Result<MaintenanceResult, MaintenanceError> {
        let start = std::time::Instant::now();

        let mut sql = String::from("CLUSTER");

        if options.verbose {
            sql.push_str(" (VERBOSE)");
        }

        if let Some((schema, table)) = target {
            sql.push_str(&format!(" {}.{}",
                Self::quote_ident(schema),
                Self::quote_ident(table)
            ));

            if let Some(ref idx) = options.index_name {
                sql.push_str(&format!(" USING {}", Self::quote_ident(idx)));
            }
        }

        let output = Self::execute_with_notices(client, &sql).await?;
        let duration = start.elapsed().as_millis() as i64;

        Ok(MaintenanceResult {
            success: true,
            output,
            duration_ms: duration,
            error: None,
        })
    }

    /// Execute command and capture NOTICE messages
    async fn execute_with_notices(
        client: &Client,
        sql: &str,
    ) -> Result<Vec<String>, MaintenanceError> {
        // Note: In a real implementation, you would set up a notice handler
        // to capture NOTICE/INFO messages from PostgreSQL
        // For now, we execute and return empty output

        client.execute(sql, &[]).await?;

        // In production, collect notices here
        Ok(vec![format!("Executed: {}", sql)])
    }

    fn quote_ident(s: &str) -> String {
        // Simple quoting - production code should use proper identifier escaping
        if s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            s.to_string()
        } else {
            format!("\"{}\"", s.replace('"', "\"\""))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MaintenanceError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] tokio_postgres::Error),

    #[error("Invalid target type: {0}")]
    InvalidTarget(String),

    #[error("Operation cancelled")]
    Cancelled,
}
```

### 21.3 Tauri Commands

```rust
// src-tauri/src/commands/maintenance.rs

use tauri::State;
use crate::services::maintenance::{
    MaintenanceService, MaintenanceResult,
    VacuumOptions, AnalyzeOptions, ReindexOptions, ReindexTarget, ClusterOptions,
};
use crate::state::AppState;
use crate::error::Error;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VacuumRequest {
    pub conn_id: String,
    pub schema: Option<String>,
    pub table: Option<String>,
    pub options: VacuumOptions,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeRequest {
    pub conn_id: String,
    pub schema: Option<String>,
    pub table: Option<String>,
    pub options: AnalyzeOptions,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReindexRequest {
    pub conn_id: String,
    pub target: ReindexTarget,
    pub options: ReindexOptions,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterRequest {
    pub conn_id: String,
    pub schema: Option<String>,
    pub table: Option<String>,
    pub options: ClusterOptions,
}

#[tauri::command]
pub async fn vacuum(
    state: State<'_, AppState>,
    request: VacuumRequest,
) -> Result<MaintenanceResult, Error> {
    let pool = state.get_connection(&request.conn_id)?;
    let client = pool.get().await?;

    let target = match (&request.schema, &request.table) {
        (Some(s), Some(t)) => Some((s.as_str(), t.as_str())),
        _ => None,
    };

    let result = MaintenanceService::vacuum(&client, target, &request.options).await?;
    Ok(result)
}

#[tauri::command]
pub async fn analyze(
    state: State<'_, AppState>,
    request: AnalyzeRequest,
) -> Result<MaintenanceResult, Error> {
    let pool = state.get_connection(&request.conn_id)?;
    let client = pool.get().await?;

    let target = match (&request.schema, &request.table) {
        (Some(s), Some(t)) => Some((s.as_str(), t.as_str())),
        _ => None,
    };

    let result = MaintenanceService::analyze(&client, target, &request.options).await?;
    Ok(result)
}

#[tauri::command]
pub async fn reindex(
    state: State<'_, AppState>,
    request: ReindexRequest,
) -> Result<MaintenanceResult, Error> {
    let pool = state.get_connection(&request.conn_id)?;
    let client = pool.get().await?;

    let result = MaintenanceService::reindex(&client, &request.target, &request.options).await?;
    Ok(result)
}

#[tauri::command]
pub async fn cluster(
    state: State<'_, AppState>,
    request: ClusterRequest,
) -> Result<MaintenanceResult, Error> {
    let pool = state.get_connection(&request.conn_id)?;
    let client = pool.get().await?;

    let target = match (&request.schema, &request.table) {
        (Some(s), Some(t)) => Some((s.as_str(), t.as_str())),
        _ => None,
    };

    let result = MaintenanceService::cluster(&client, target, &request.options).await?;
    Ok(result)
}
```

### 21.4 Maintenance Store (Svelte)

```typescript
// src/lib/stores/maintenanceStore.svelte.ts

import { invoke } from '@tauri-apps/api/core';
import type {
	VacuumOptions,
	AnalyzeOptions,
	ReindexOptions,
	ReindexTarget,
	ClusterOptions,
	MaintenanceJob,
	MaintenanceResult
} from '$lib/types/maintenance';

interface MaintenanceState {
	activeJobs: MaintenanceJob[];
	completedJobs: MaintenanceJob[];
	maxCompletedJobs: number;
}

export function createMaintenanceStore() {
	let state = $state<MaintenanceState>({
		activeJobs: [],
		completedJobs: [],
		maxCompletedJobs: 50
	});

	function createJob(type: MaintenanceJob['type'], target: string): MaintenanceJob {
		return {
			id: crypto.randomUUID(),
			type,
			target,
			status: 'pending',
			startTime: null,
			endTime: null,
			output: [],
			error: null
		};
	}

	function updateJob(jobId: string, updates: Partial<MaintenanceJob>) {
		state.activeJobs = state.activeJobs.map((job) =>
			job.id === jobId ? { ...job, ...updates } : job
		);
	}

	function completeJob(jobId: string, result: MaintenanceResult) {
		const job = state.activeJobs.find((j) => j.id === jobId);
		if (!job) return;

		const completedJob: MaintenanceJob = {
			...job,
			status: result.success ? 'completed' : 'failed',
			endTime: new Date(),
			output: result.output,
			error: result.error ?? null
		};

		state.activeJobs = state.activeJobs.filter((j) => j.id !== jobId);
		state.completedJobs = [completedJob, ...state.completedJobs].slice(0, state.maxCompletedJobs);
	}

	async function vacuum(
		connId: string,
		schema: string | null,
		table: string | null,
		options: VacuumOptions
	): Promise<MaintenanceResult> {
		const target = schema && table ? `${schema}.${table}` : 'database';
		const job = createJob('vacuum', target);

		state.activeJobs = [...state.activeJobs, job];
		updateJob(job.id, { status: 'running', startTime: new Date() });

		try {
			const result = await invoke<MaintenanceResult>('vacuum', {
				request: { connId, schema, table, options }
			});

			completeJob(job.id, result);
			return result;
		} catch (err) {
			const errorResult: MaintenanceResult = {
				success: false,
				output: [],
				duration: 0,
				error: err instanceof Error ? err.message : String(err)
			};
			completeJob(job.id, errorResult);
			return errorResult;
		}
	}

	async function analyze(
		connId: string,
		schema: string | null,
		table: string | null,
		options: AnalyzeOptions
	): Promise<MaintenanceResult> {
		const target = schema && table ? `${schema}.${table}` : 'database';
		const job = createJob('analyze', target);

		state.activeJobs = [...state.activeJobs, job];
		updateJob(job.id, { status: 'running', startTime: new Date() });

		try {
			const result = await invoke<MaintenanceResult>('analyze', {
				request: { connId, schema, table, options }
			});

			completeJob(job.id, result);
			return result;
		} catch (err) {
			const errorResult: MaintenanceResult = {
				success: false,
				output: [],
				duration: 0,
				error: err instanceof Error ? err.message : String(err)
			};
			completeJob(job.id, errorResult);
			return errorResult;
		}
	}

	async function reindex(
		connId: string,
		target: ReindexTarget,
		options: ReindexOptions
	): Promise<MaintenanceResult> {
		const targetStr =
			target.schema && target.name
				? `${target.schema}.${target.name}`
				: (target.name ?? target.targetType);
		const job = createJob('reindex', targetStr);

		state.activeJobs = [...state.activeJobs, job];
		updateJob(job.id, { status: 'running', startTime: new Date() });

		try {
			const result = await invoke<MaintenanceResult>('reindex', {
				request: { connId, target, options }
			});

			completeJob(job.id, result);
			return result;
		} catch (err) {
			const errorResult: MaintenanceResult = {
				success: false,
				output: [],
				duration: 0,
				error: err instanceof Error ? err.message : String(err)
			};
			completeJob(job.id, errorResult);
			return errorResult;
		}
	}

	async function cluster(
		connId: string,
		schema: string | null,
		table: string | null,
		options: ClusterOptions
	): Promise<MaintenanceResult> {
		const target = schema && table ? `${schema}.${table}` : 'all tables';
		const job = createJob('cluster', target);

		state.activeJobs = [...state.activeJobs, job];
		updateJob(job.id, { status: 'running', startTime: new Date() });

		try {
			const result = await invoke<MaintenanceResult>('cluster', {
				request: { connId, schema, table, options }
			});

			completeJob(job.id, result);
			return result;
		} catch (err) {
			const errorResult: MaintenanceResult = {
				success: false,
				output: [],
				duration: 0,
				error: err instanceof Error ? err.message : String(err)
			};
			completeJob(job.id, errorResult);
			return errorResult;
		}
	}

	function clearCompletedJobs() {
		state.completedJobs = [];
	}

	return {
		get activeJobs() {
			return state.activeJobs;
		},
		get completedJobs() {
			return state.completedJobs;
		},

		vacuum,
		analyze,
		reindex,
		cluster,
		clearCompletedJobs
	};
}

export const maintenanceStore = createMaintenanceStore();
```

### 21.5 Vacuum Dialog

```svelte
<!-- src/lib/components/maintenance/VacuumDialog.svelte -->
<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { VacuumOptions } from '$lib/types/maintenance';
	import type { Table } from '$lib/types/schema';

	interface Props {
		open: boolean;
		connId: string;
		tables: Table[];
		initialTable?: { schema: string; name: string };
	}

	let { open = $bindable(), connId, tables, initialTable }: Props = $props();

	const dispatch = createEventDispatcher<{
		run: { schema: string | null; table: string | null; options: VacuumOptions };
		cancel: void;
	}>();

	let selectedSchema = $state(initialTable?.schema ?? '');
	let selectedTable = $state(initialTable?.name ?? '');

	let options = $state<VacuumOptions>({
		full: false,
		freeze: false,
		analyze: false,
		verbose: true,
		skipLocked: false,
		indexCleanup: 'auto',
		parallel: 0,
		truncate: true,
		processToast: true
	});

	// Get unique schemas
	const schemas = $derived([...new Set(tables.map((t) => t.schema))].sort());

	// Get tables for selected schema
	const schemaTables = $derived(
		selectedSchema
			? tables
					.filter((t) => t.schema === selectedSchema)
					.map((t) => t.name)
					.sort()
			: []
	);

	function handleRun() {
		dispatch('run', {
			schema: selectedSchema || null,
			table: selectedTable || null,
			options
		});
		open = false;
	}

	function handleCancel() {
		dispatch('cancel');
		open = false;
	}

	// Reset table when schema changes
	$effect(() => {
		if (selectedSchema && !schemaTables.includes(selectedTable)) {
			selectedTable = '';
		}
	});
</script>

{#if open}
	<div
		class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
		role="dialog"
		aria-modal="true"
		aria-labelledby="vacuum-dialog-title"
	>
		<div
			class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-[500px] max-h-[80vh] overflow-hidden"
		>
			<!-- Header -->
			<div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
				<h2 id="vacuum-dialog-title" class="text-lg font-semibold">VACUUM</h2>
			</div>

			<!-- Body -->
			<div class="p-4 space-y-4 overflow-y-auto max-h-[60vh]">
				<!-- Target Selection -->
				<div>
					<label class="block text-sm font-medium mb-2">Target</label>
					<div class="grid grid-cols-2 gap-2">
						<select
							bind:value={selectedSchema}
							class="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                     bg-white dark:bg-gray-700 text-sm"
						>
							<option value="">All schemas</option>
							{#each schemas as schema}
								<option value={schema}>{schema}</option>
							{/each}
						</select>
						<select
							bind:value={selectedTable}
							disabled={!selectedSchema}
							class="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                     bg-white dark:bg-gray-700 text-sm disabled:opacity-50"
						>
							<option value="">All tables</option>
							{#each schemaTables as table}
								<option value={table}>{table}</option>
							{/each}
						</select>
					</div>
				</div>

				<!-- Options -->
				<div class="space-y-3">
					<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">Options</h3>

					<label class="flex items-start gap-3 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={options.full}
							class="mt-1 rounded border-gray-300 dark:border-gray-600"
						/>
						<div>
							<span class="font-medium text-sm">FULL</span>
							<p class="text-xs text-gray-500 dark:text-gray-400">
								Reclaims more space but takes longer and requires exclusive lock. Rewrites the
								entire table.
							</p>
						</div>
					</label>

					<label class="flex items-start gap-3 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={options.freeze}
							class="mt-1 rounded border-gray-300 dark:border-gray-600"
						/>
						<div>
							<span class="font-medium text-sm">FREEZE</span>
							<p class="text-xs text-gray-500 dark:text-gray-400">
								Aggressively freeze tuples. Useful before taking a pg_dump for archival.
							</p>
						</div>
					</label>

					<label class="flex items-start gap-3 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={options.analyze}
							class="mt-1 rounded border-gray-300 dark:border-gray-600"
						/>
						<div>
							<span class="font-medium text-sm">ANALYZE</span>
							<p class="text-xs text-gray-500 dark:text-gray-400">
								Update statistics used by the query planner.
							</p>
						</div>
					</label>

					<label class="flex items-start gap-3 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={options.verbose}
							class="mt-1 rounded border-gray-300 dark:border-gray-600"
						/>
						<div>
							<span class="font-medium text-sm">VERBOSE</span>
							<p class="text-xs text-gray-500 dark:text-gray-400">
								Print detailed progress report for each table.
							</p>
						</div>
					</label>

					<label class="flex items-start gap-3 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={options.skipLocked}
							class="mt-1 rounded border-gray-300 dark:border-gray-600"
						/>
						<div>
							<span class="font-medium text-sm">SKIP_LOCKED</span>
							<p class="text-xs text-gray-500 dark:text-gray-400">
								Skip tables that cannot be locked immediately.
							</p>
						</div>
					</label>

					<div class="flex items-center gap-4">
						<label class="flex items-center gap-2">
							<span class="text-sm">Index Cleanup:</span>
							<select
								bind:value={options.indexCleanup}
								class="px-2 py-1 text-sm border border-gray-300 dark:border-gray-600
                       rounded bg-white dark:bg-gray-700"
							>
								<option value="auto">Auto</option>
								<option value="on">On</option>
								<option value="off">Off</option>
							</select>
						</label>

						<label class="flex items-center gap-2">
							<span class="text-sm">Parallel Workers:</span>
							<input
								type="number"
								bind:value={options.parallel}
								min="0"
								max="32"
								class="w-16 px-2 py-1 text-sm border border-gray-300 dark:border-gray-600
                       rounded bg-white dark:bg-gray-700"
							/>
							<span class="text-xs text-gray-500">(0 = auto)</span>
						</label>
					</div>
				</div>

				<!-- Warning for FULL -->
				{#if options.full}
					<div
						class="p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200
                      dark:border-amber-800 rounded text-sm text-amber-800 dark:text-amber-200"
					>
						<strong>Warning:</strong> VACUUM FULL requires an exclusive lock on the table and rewrites
						the entire table. This can take a significant amount of time for large tables and will block
						all queries.
					</div>
				{/if}
			</div>

			<!-- Footer -->
			<div class="px-4 py-3 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-2">
				<button
					onclick={handleCancel}
					class="px-4 py-2 text-sm text-gray-700 dark:text-gray-300
                 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
				>
					Cancel
				</button>
				<button
					onclick={handleRun}
					class="px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
				>
					Run VACUUM
				</button>
			</div>
		</div>
	</div>
{/if}
```

### 21.6 Reindex Dialog

```svelte
<!-- src/lib/components/maintenance/ReindexDialog.svelte -->
<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { ReindexOptions, ReindexTarget } from '$lib/types/maintenance';
	import type { Table, Index } from '$lib/types/schema';

	interface Props {
		open: boolean;
		connId: string;
		tables: Table[];
		indexes: Array<{ schema: string; table: string; name: string }>;
		initialTarget?: ReindexTarget;
	}

	let { open = $bindable(), connId, tables, indexes, initialTarget }: Props = $props();

	const dispatch = createEventDispatcher<{
		run: { target: ReindexTarget; options: ReindexOptions };
		cancel: void;
	}>();

	let targetType = $state<'index' | 'table' | 'schema' | 'database'>(
		initialTarget?.type ?? 'table'
	);
	let selectedSchema = $state(initialTarget?.schema ?? '');
	let selectedTable = $state('');
	let selectedIndex = $state(initialTarget?.name ?? '');

	let options = $state<ReindexOptions>({
		concurrently: true,
		verbose: true,
		tablespace: undefined
	});

	// Get unique schemas
	const schemas = $derived([...new Set(tables.map((t) => t.schema))].sort());

	// Get tables for selected schema
	const schemaTables = $derived(
		selectedSchema
			? tables
					.filter((t) => t.schema === selectedSchema)
					.map((t) => t.name)
					.sort()
			: []
	);

	// Get indexes for selected table
	const tableIndexes = $derived(
		selectedSchema && selectedTable
			? indexes
					.filter((i) => i.schema === selectedSchema && i.table === selectedTable)
					.map((i) => i.name)
					.sort()
			: []
	);

	function handleRun() {
		const target: ReindexTarget = {
			type: targetType,
			schema: ['index', 'table'].includes(targetType) ? selectedSchema : undefined,
			name:
				targetType === 'index'
					? selectedIndex
					: targetType === 'table'
						? selectedTable
						: targetType === 'schema'
							? selectedSchema
							: undefined
		};

		dispatch('run', { target, options });
		open = false;
	}

	function handleCancel() {
		dispatch('cancel');
		open = false;
	}
</script>

{#if open}
	<div
		class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
		role="dialog"
		aria-modal="true"
	>
		<div
			class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-[500px] max-h-[80vh] overflow-hidden"
		>
			<!-- Header -->
			<div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
				<h2 class="text-lg font-semibold">REINDEX</h2>
			</div>

			<!-- Body -->
			<div class="p-4 space-y-4 overflow-y-auto max-h-[60vh]">
				<!-- Target Type -->
				<div>
					<label class="block text-sm font-medium mb-2">Target Type</label>
					<div class="flex gap-4">
						{#each ['table', 'index', 'schema', 'database'] as type}
							<label class="flex items-center gap-2 cursor-pointer">
								<input type="radio" bind:group={targetType} value={type} class="text-blue-600" />
								<span class="text-sm capitalize">{type}</span>
							</label>
						{/each}
					</div>
				</div>

				<!-- Target Selection -->
				{#if targetType !== 'database'}
					<div class="space-y-2">
						<label class="block text-sm font-medium mb-2">Target</label>

						{#if targetType === 'schema'}
							<select
								bind:value={selectedSchema}
								class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                       bg-white dark:bg-gray-700 text-sm"
							>
								<option value="">Select schema...</option>
								{#each schemas as schema}
									<option value={schema}>{schema}</option>
								{/each}
							</select>
						{:else}
							<div class="grid grid-cols-2 gap-2">
								<select
									bind:value={selectedSchema}
									class="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                         bg-white dark:bg-gray-700 text-sm"
								>
									<option value="">Select schema...</option>
									{#each schemas as schema}
										<option value={schema}>{schema}</option>
									{/each}
								</select>

								{#if targetType === 'table'}
									<select
										bind:value={selectedTable}
										disabled={!selectedSchema}
										class="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                           bg-white dark:bg-gray-700 text-sm disabled:opacity-50"
									>
										<option value="">Select table...</option>
										{#each schemaTables as table}
											<option value={table}>{table}</option>
										{/each}
									</select>
								{:else if targetType === 'index'}
									<select
										bind:value={selectedTable}
										disabled={!selectedSchema}
										class="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                           bg-white dark:bg-gray-700 text-sm disabled:opacity-50"
									>
										<option value="">Select table...</option>
										{#each schemaTables as table}
											<option value={table}>{table}</option>
										{/each}
									</select>
								{/if}
							</div>

							{#if targetType === 'index' && selectedTable}
								<select
									bind:value={selectedIndex}
									class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                         bg-white dark:bg-gray-700 text-sm"
								>
									<option value="">Select index...</option>
									{#each tableIndexes as idx}
										<option value={idx}>{idx}</option>
									{/each}
								</select>
							{/if}
						{/if}
					</div>
				{/if}

				<!-- Options -->
				<div class="space-y-3">
					<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">Options</h3>

					<label class="flex items-start gap-3 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={options.concurrently}
							class="mt-1 rounded border-gray-300 dark:border-gray-600"
						/>
						<div>
							<span class="font-medium text-sm">CONCURRENTLY</span>
							<p class="text-xs text-gray-500 dark:text-gray-400">
								Rebuild the index without locking writes. Takes longer but doesn't block normal
								database operations.
							</p>
						</div>
					</label>

					<label class="flex items-start gap-3 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={options.verbose}
							class="mt-1 rounded border-gray-300 dark:border-gray-600"
						/>
						<div>
							<span class="font-medium text-sm">VERBOSE</span>
							<p class="text-xs text-gray-500 dark:text-gray-400">Print progress report.</p>
						</div>
					</label>
				</div>

				<!-- Info for CONCURRENTLY -->
				{#if options.concurrently}
					<div
						class="p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200
                      dark:border-blue-800 rounded text-sm text-blue-800 dark:text-blue-200"
					>
						<strong>Note:</strong> CONCURRENTLY requires more time and resources but allows normal database
						operations to continue during the reindex.
					</div>
				{:else}
					<div
						class="p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200
                      dark:border-amber-800 rounded text-sm text-amber-800 dark:text-amber-200"
					>
						<strong>Warning:</strong> Without CONCURRENTLY, the table will be locked for writes during
						the entire reindex operation.
					</div>
				{/if}
			</div>

			<!-- Footer -->
			<div class="px-4 py-3 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-2">
				<button
					onclick={handleCancel}
					class="px-4 py-2 text-sm text-gray-700 dark:text-gray-300
                 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
				>
					Cancel
				</button>
				<button
					onclick={handleRun}
					disabled={(targetType === 'schema' && !selectedSchema) ||
						(targetType === 'table' && !selectedTable) ||
						(targetType === 'index' && !selectedIndex)}
					class="px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700
                 disabled:opacity-50 disabled:cursor-not-allowed"
				>
					Run REINDEX
				</button>
			</div>
		</div>
	</div>
{/if}
```

### 21.7 Maintenance Progress Dialog

```svelte
<!-- src/lib/components/maintenance/MaintenanceProgress.svelte -->
<script lang="ts">
	import type { MaintenanceJob } from '$lib/types/maintenance';
	import { maintenanceStore } from '$lib/stores/maintenanceStore.svelte';

	interface Props {
		open: boolean;
	}

	let { open = $bindable() }: Props = $props();

	function formatDuration(start: Date | null, end: Date | null): string {
		if (!start) return '-';
		const endTime = end ? new Date(end).getTime() : Date.now();
		const duration = endTime - new Date(start).getTime();

		if (duration < 1000) return `${duration}ms`;
		if (duration < 60000) return `${(duration / 1000).toFixed(1)}s`;
		return `${Math.floor(duration / 60000)}m ${Math.floor((duration % 60000) / 1000)}s`;
	}

	function getStatusColor(status: string): string {
		switch (status) {
			case 'running':
				return 'text-blue-600 dark:text-blue-400';
			case 'completed':
				return 'text-green-600 dark:text-green-400';
			case 'failed':
				return 'text-red-600 dark:text-red-400';
			case 'cancelled':
				return 'text-gray-500 dark:text-gray-400';
			default:
				return 'text-gray-600 dark:text-gray-400';
		}
	}

	function getStatusIcon(status: string): string {
		switch (status) {
			case 'running':
				return '⏳';
			case 'completed':
				return '✅';
			case 'failed':
				return '❌';
			case 'cancelled':
				return '⚪';
			default:
				return '⏸️';
		}
	}

	const allJobs = $derived([...maintenanceStore.activeJobs, ...maintenanceStore.completedJobs]);

	let expandedJobId = $state<string | null>(null);
</script>

{#if open}
	<div
		class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
		role="dialog"
		aria-modal="true"
	>
		<div
			class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-[600px] max-h-[80vh] overflow-hidden"
		>
			<!-- Header -->
			<div
				class="px-4 py-3 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between"
			>
				<h2 class="text-lg font-semibold">Maintenance Jobs</h2>
				<button
					onclick={() => (open = false)}
					class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
				>
					<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M6 18L18 6M6 6l12 12"
						/>
					</svg>
				</button>
			</div>

			<!-- Body -->
			<div class="overflow-y-auto max-h-[60vh]">
				{#if allJobs.length === 0}
					<div class="p-8 text-center text-gray-500 dark:text-gray-400">No maintenance jobs</div>
				{:else}
					<div class="divide-y divide-gray-200 dark:divide-gray-700">
						{#each allJobs as job (job.id)}
							<div class="p-4">
								<!-- Job Header -->
								<button
									class="w-full flex items-center justify-between text-left"
									onclick={() => (expandedJobId = expandedJobId === job.id ? null : job.id)}
								>
									<div class="flex items-center gap-3">
										<span class="text-lg">{getStatusIcon(job.status)}</span>
										<div>
											<span class="font-medium uppercase text-sm">{job.type}</span>
											<span class="text-gray-500 dark:text-gray-400 ml-2">{job.target}</span>
										</div>
									</div>
									<div class="flex items-center gap-4">
										<span class="text-sm {getStatusColor(job.status)}">
											{job.status}
										</span>
										<span class="text-sm text-gray-500">
											{formatDuration(job.startTime, job.endTime)}
										</span>
										<svg
											class="w-4 h-4 text-gray-400 transform transition-transform
                             {expandedJobId === job.id ? 'rotate-180' : ''}"
											fill="none"
											stroke="currentColor"
											viewBox="0 0 24 24"
										>
											<path
												stroke-linecap="round"
												stroke-linejoin="round"
												stroke-width="2"
												d="M19 9l-7 7-7-7"
											/>
										</svg>
									</div>
								</button>

								<!-- Job Details (Expanded) -->
								{#if expandedJobId === job.id}
									<div class="mt-3 pl-9 space-y-2">
										{#if job.output.length > 0}
											<div
												class="bg-gray-100 dark:bg-gray-900 rounded p-3 font-mono text-xs
                                  max-h-40 overflow-auto whitespace-pre-wrap"
											>
												{job.output.join('\n')}
											</div>
										{/if}

										{#if job.error}
											<div
												class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200
                                  dark:border-red-800 rounded text-sm text-red-700 dark:text-red-400"
											>
												{job.error}
											</div>
										{/if}

										{#if job.startTime}
											<div class="text-xs text-gray-500 dark:text-gray-400">
												Started: {new Date(job.startTime).toLocaleString()}
												{#if job.endTime}
													<span class="mx-2">|</span>
													Ended: {new Date(job.endTime).toLocaleString()}
												{/if}
											</div>
										{/if}
									</div>
								{/if}
							</div>
						{/each}
					</div>
				{/if}
			</div>

			<!-- Footer -->
			<div class="px-4 py-3 border-t border-gray-200 dark:border-gray-700 flex justify-between">
				<button
					onclick={() => maintenanceStore.clearCompletedJobs()}
					disabled={maintenanceStore.completedJobs.length === 0}
					class="px-3 py-1.5 text-sm text-gray-600 dark:text-gray-400
                 hover:text-gray-900 dark:hover:text-gray-100 disabled:opacity-50"
				>
					Clear Completed
				</button>
				<button
					onclick={() => (open = false)}
					class="px-4 py-2 text-sm bg-gray-200 dark:bg-gray-700 rounded
                 hover:bg-gray-300 dark:hover:bg-gray-600"
				>
					Close
				</button>
			</div>
		</div>
	</div>
{/if}
```

### 21.8 Analyze Dialog

```svelte
<!-- src/lib/components/maintenance/AnalyzeDialog.svelte -->
<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { AnalyzeOptions } from '$lib/types/maintenance';
	import type { Table, Column } from '$lib/types/schema';

	interface Props {
		open: boolean;
		connId: string;
		tables: Table[];
		initialTable?: { schema: string; name: string };
	}

	let { open = $bindable(), connId, tables, initialTable }: Props = $props();

	const dispatch = createEventDispatcher<{
		run: { schema: string | null; table: string | null; options: AnalyzeOptions };
		cancel: void;
	}>();

	let selectedSchema = $state(initialTable?.schema ?? '');
	let selectedTable = $state(initialTable?.name ?? '');
	let selectedColumns = $state<string[]>([]);

	let options = $state<AnalyzeOptions>({
		verbose: true,
		skipLocked: false,
		columns: undefined
	});

	// Get unique schemas
	const schemas = $derived([...new Set(tables.map((t) => t.schema))].sort());

	// Get tables for selected schema
	const schemaTables = $derived(
		selectedSchema
			? tables
					.filter((t) => t.schema === selectedSchema)
					.map((t) => t.name)
					.sort()
			: []
	);

	// Get columns for selected table
	const selectedTableObj = $derived(
		tables.find((t) => t.schema === selectedSchema && t.name === selectedTable)
	);

	const tableColumns = $derived(selectedTableObj?.columns.map((c) => c.name).sort() ?? []);

	function toggleColumn(col: string) {
		if (selectedColumns.includes(col)) {
			selectedColumns = selectedColumns.filter((c) => c !== col);
		} else {
			selectedColumns = [...selectedColumns, col];
		}
	}

	function handleRun() {
		dispatch('run', {
			schema: selectedSchema || null,
			table: selectedTable || null,
			options: {
				...options,
				columns: selectedColumns.length > 0 ? selectedColumns : undefined
			}
		});
		open = false;
	}

	function handleCancel() {
		dispatch('cancel');
		open = false;
	}

	// Reset columns when table changes
	$effect(() => {
		selectedColumns = [];
	});
</script>

{#if open}
	<div
		class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
		role="dialog"
		aria-modal="true"
	>
		<div
			class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-[500px] max-h-[80vh] overflow-hidden"
		>
			<!-- Header -->
			<div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
				<h2 class="text-lg font-semibold">ANALYZE</h2>
			</div>

			<!-- Body -->
			<div class="p-4 space-y-4 overflow-y-auto max-h-[60vh]">
				<!-- Target Selection -->
				<div>
					<label class="block text-sm font-medium mb-2">Target</label>
					<div class="grid grid-cols-2 gap-2">
						<select
							bind:value={selectedSchema}
							class="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                     bg-white dark:bg-gray-700 text-sm"
						>
							<option value="">All schemas</option>
							{#each schemas as schema}
								<option value={schema}>{schema}</option>
							{/each}
						</select>
						<select
							bind:value={selectedTable}
							disabled={!selectedSchema}
							class="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                     bg-white dark:bg-gray-700 text-sm disabled:opacity-50"
						>
							<option value="">All tables</option>
							{#each schemaTables as table}
								<option value={table}>{table}</option>
							{/each}
						</select>
					</div>
				</div>

				<!-- Specific Columns (only when table selected) -->
				{#if selectedTable && tableColumns.length > 0}
					<div>
						<label class="block text-sm font-medium mb-2"> Specific Columns (optional) </label>
						<div
							class="max-h-32 overflow-y-auto border border-gray-200 dark:border-gray-700
                        rounded p-2 space-y-1"
						>
							{#each tableColumns as col}
								<label class="flex items-center gap-2 cursor-pointer text-sm">
									<input
										type="checkbox"
										checked={selectedColumns.includes(col)}
										onchange={() => toggleColumn(col)}
										class="rounded border-gray-300 dark:border-gray-600"
									/>
									<span class="font-mono">{col}</span>
								</label>
							{/each}
						</div>
						<p class="text-xs text-gray-500 mt-1">Leave empty to analyze all columns</p>
					</div>
				{/if}

				<!-- Options -->
				<div class="space-y-3">
					<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">Options</h3>

					<label class="flex items-start gap-3 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={options.verbose}
							class="mt-1 rounded border-gray-300 dark:border-gray-600"
						/>
						<div>
							<span class="font-medium text-sm">VERBOSE</span>
							<p class="text-xs text-gray-500 dark:text-gray-400">
								Print progress messages for each table.
							</p>
						</div>
					</label>

					<label class="flex items-start gap-3 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={options.skipLocked}
							class="mt-1 rounded border-gray-300 dark:border-gray-600"
						/>
						<div>
							<span class="font-medium text-sm">SKIP_LOCKED</span>
							<p class="text-xs text-gray-500 dark:text-gray-400">
								Skip tables that cannot be locked immediately.
							</p>
						</div>
					</label>
				</div>

				<!-- Info -->
				<div
					class="p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200
                    dark:border-blue-800 rounded text-sm text-blue-800 dark:text-blue-200"
				>
					<strong>Note:</strong> ANALYZE collects statistics about the contents of tables in the database,
					which the query planner uses to generate better execution plans.
				</div>
			</div>

			<!-- Footer -->
			<div class="px-4 py-3 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-2">
				<button
					onclick={handleCancel}
					class="px-4 py-2 text-sm text-gray-700 dark:text-gray-300
                 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
				>
					Cancel
				</button>
				<button
					onclick={handleRun}
					class="px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
				>
					Run ANALYZE
				</button>
			</div>
		</div>
	</div>
{/if}
```

## Acceptance Criteria

1. **VACUUM Dialog**
   - [ ] Select target table or all tables
   - [ ] Configure FULL, FREEZE, ANALYZE options
   - [ ] Set VERBOSE, SKIP_LOCKED options
   - [ ] Configure INDEX_CLEANUP and PARALLEL
   - [ ] Show warning for FULL option
   - [ ] Display execution output

2. **ANALYZE Dialog**
   - [ ] Select target table or all tables
   - [ ] Optionally select specific columns
   - [ ] Configure VERBOSE option
   - [ ] Support SKIP_LOCKED option

3. **REINDEX Dialog**
   - [ ] Select target type (index, table, schema, database)
   - [ ] Support CONCURRENTLY option
   - [ ] Configure VERBOSE option
   - [ ] Warning for non-concurrent reindex

4. **CLUSTER Dialog**
   - [ ] Select target table
   - [ ] Select index to cluster on
   - [ ] Configure VERBOSE option

5. **Progress Tracking**
   - [ ] Show active jobs
   - [ ] Display completed jobs with output
   - [ ] Show error messages for failed jobs
   - [ ] Clear completed jobs

## MCP Testing Instructions

### Tauri MCP Testing

```typescript
// Test VACUUM execution
await mcp___hypothesi_tauri_mcp_server__ipc_execute_command({
	command: 'vacuum',
	args: {
		request: {
			connId: 'test-conn',
			schema: 'public',
			table: 'users',
			options: {
				full: false,
				freeze: false,
				analyze: true,
				verbose: true,
				skipLocked: false,
				indexCleanup: 'auto',
				parallel: 0,
				truncate: true,
				processToast: true
			}
		}
	}
});

// Test REINDEX execution
await mcp___hypothesi_tauri_mcp_server__ipc_execute_command({
	command: 'reindex',
	args: {
		request: {
			connId: 'test-conn',
			target: {
				targetType: 'table',
				schema: 'public',
				name: 'users'
			},
			options: {
				concurrently: true,
				verbose: true
			}
		}
	}
});
```

### Playwright MCP Testing

```typescript
// Open VACUUM dialog from admin dashboard
await mcp__playwright__browser_click({
	element: 'Vacuum button',
	ref: 'button:has-text("Vacuum"):first'
});

// Verify dialog opens
await mcp__playwright__browser_wait_for({
	text: 'VACUUM'
});

// Configure options
await mcp__playwright__browser_click({
	element: 'ANALYZE checkbox',
	ref: 'input[type="checkbox"]:near(:text("ANALYZE"))'
});

// Take screenshot
await mcp__playwright__browser_take_screenshot({
	filename: 'vacuum-dialog.png'
});

// Run vacuum
await mcp__playwright__browser_click({
	element: 'Run VACUUM button',
	ref: 'button:has-text("Run VACUUM")'
});

// Verify progress dialog
await mcp__playwright__browser_wait_for({
	text: 'Maintenance Jobs'
});
```
