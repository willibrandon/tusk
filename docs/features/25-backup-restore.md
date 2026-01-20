# Feature 25: Backup and Restore

## Overview

Backup and Restore provides GUI interfaces for pg_dump and pg_restore operations, enabling database backup creation and restoration with full control over formats, objects, and options.

## Goals

- Create backups using pg_dump with all format options
- Restore backups using pg_restore
- Select specific objects (schemas, tables) for backup
- Configure compression and parallel jobs
- Show real-time progress and output
- Support backup scheduling (future enhancement)

## Dependencies

- Feature 07: Connection Pool Management
- Feature 10: Schema Cache (for object selection)

## Technical Specification

### 25.1 Backup/Restore Data Models

```typescript
// src/lib/types/backup.ts

export interface BackupOptions {
	// Connection
	connectionId: string;

	// Output
	outputPath: string;
	format: BackupFormat;
	compression: number; // 0-9, 0 = none

	// Content
	includeSchema: boolean;
	includeData: boolean;
	includePrivileges: boolean;
	includeOwnership: boolean;

	// Objects
	schemas: string[] | null; // null = all
	tables: string[] | null; // null = all
	excludeTables: string[];
	excludeTableData: string[]; // Tables to exclude data (schema only)

	// Advanced
	jobs: number; // Parallel dump jobs
	lockWaitTimeout: number; // Seconds
	noSync: boolean;
	encoding: string | null;

	// Extras
	extraArgs: string[];
}

export type BackupFormat = 'custom' | 'plain' | 'directory' | 'tar';

export interface RestoreOptions {
	// Connection
	connectionId: string;

	// Input
	inputPath: string;

	// Target
	targetDatabase: string | null; // null = current connection
	createDatabase: boolean;

	// Content
	schemaOnly: boolean;
	dataOnly: boolean;

	// Objects
	schemas: string[] | null;
	tables: string[] | null;

	// Behavior
	clean: boolean; // Drop objects before restore
	ifExists: boolean; // Add IF EXISTS to DROP
	noOwner: boolean;
	noPrivileges: boolean;
	exitOnError: boolean;
	singleTransaction: boolean;

	// Advanced
	jobs: number;
	disableTriggers: boolean;

	// Extras
	extraArgs: string[];
}

export interface BackupJob {
	id: string;
	type: 'backup' | 'restore';
	status: JobStatus;
	options: BackupOptions | RestoreOptions;
	startTime: Date | null;
	endTime: Date | null;
	output: string[];
	errors: string[];
	progress: BackupProgress | null;
}

export type JobStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

export interface BackupProgress {
	phase: string;
	currentObject: string | null;
	objectsTotal: number | null;
	objectsCompleted: number;
	bytesWritten: number;
	elapsedMs: number;
}

export interface BackupInfo {
	path: string;
	format: BackupFormat;
	sizeBytes: number;
	created: Date;
	database: string;
	serverVersion: string;
	pgDumpVersion: string;
	contents: BackupContents;
}

export interface BackupContents {
	schemas: string[];
	tables: Array<{ schema: string; name: string }>;
	functions: number;
	views: number;
	sequences: number;
	indexes: number;
	triggers: number;
	constraints: number;
	hasBlobs: boolean;
}
```

### 25.2 Backup Service (Rust)

```rust
// src-tauri/src/services/backup.rs

use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::path::Path;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupOptions {
    pub connection_id: String,
    pub output_path: String,
    pub format: String,
    pub compression: i32,
    pub include_schema: bool,
    pub include_data: bool,
    pub include_privileges: bool,
    pub include_ownership: bool,
    pub schemas: Option<Vec<String>>,
    pub tables: Option<Vec<String>>,
    pub exclude_tables: Vec<String>,
    pub exclude_table_data: Vec<String>,
    pub jobs: i32,
    pub lock_wait_timeout: i32,
    pub no_sync: bool,
    pub encoding: Option<String>,
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOptions {
    pub connection_id: String,
    pub input_path: String,
    pub target_database: Option<String>,
    pub create_database: bool,
    pub schema_only: bool,
    pub data_only: bool,
    pub schemas: Option<Vec<String>>,
    pub tables: Option<Vec<String>>,
    pub clean: bool,
    pub if_exists: bool,
    pub no_owner: bool,
    pub no_privileges: bool,
    pub exit_on_error: bool,
    pub single_transaction: bool,
    pub jobs: i32,
    pub disable_triggers: bool,
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupJob {
    pub id: String,
    pub job_type: String,
    pub status: String,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub output: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupProgress {
    pub phase: String,
    pub current_object: Option<String>,
    pub objects_total: Option<i64>,
    pub objects_completed: i64,
    pub bytes_written: i64,
    pub elapsed_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub path: String,
    pub format: String,
    pub size_bytes: u64,
    pub created: chrono::DateTime<chrono::Utc>,
    pub database: String,
    pub server_version: String,
    pub pg_dump_version: String,
    pub contents: BackupContents,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupContents {
    pub schemas: Vec<String>,
    pub tables: Vec<TableRef>,
    pub functions: i64,
    pub views: i64,
    pub sequences: i64,
    pub indexes: i64,
    pub triggers: i64,
    pub constraints: i64,
    pub has_blobs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRef {
    pub schema: String,
    pub name: String,
}

pub struct BackupService;

impl BackupService {
    /// Create a backup using pg_dump
    pub async fn create_backup(
        app: AppHandle,
        job_id: &str,
        connection_string: &str,
        options: &BackupOptions,
    ) -> Result<BackupJob, BackupError> {
        let mut job = BackupJob {
            id: job_id.to_string(),
            job_type: "backup".to_string(),
            status: "running".to_string(),
            start_time: Some(chrono::Utc::now()),
            end_time: None,
            output: Vec::new(),
            errors: Vec::new(),
        };

        // Build pg_dump command
        let mut cmd = Command::new("pg_dump");

        // Connection
        cmd.arg(&connection_string);

        // Format
        cmd.arg("-F").arg(match options.format.as_str() {
            "custom" => "c",
            "plain" => "p",
            "directory" => "d",
            "tar" => "t",
            _ => "c",
        });

        // Output
        cmd.arg("-f").arg(&options.output_path);

        // Compression
        if options.compression > 0 {
            cmd.arg("-Z").arg(options.compression.to_string());
        }

        // Content flags
        if !options.include_schema {
            cmd.arg("-a"); // data-only
        }
        if !options.include_data {
            cmd.arg("-s"); // schema-only
        }
        if !options.include_privileges {
            cmd.arg("-x"); // no-privileges
        }
        if !options.include_ownership {
            cmd.arg("-O"); // no-owner
        }

        // Schema selection
        if let Some(ref schemas) = options.schemas {
            for schema in schemas {
                cmd.arg("-n").arg(schema);
            }
        }

        // Table selection
        if let Some(ref tables) = options.tables {
            for table in tables {
                cmd.arg("-t").arg(table);
            }
        }

        // Exclusions
        for table in &options.exclude_tables {
            cmd.arg("-T").arg(table);
        }
        for table in &options.exclude_table_data {
            cmd.arg("--exclude-table-data").arg(table);
        }

        // Parallel jobs
        if options.jobs > 1 && options.format == "directory" {
            cmd.arg("-j").arg(options.jobs.to_string());
        }

        // Lock timeout
        if options.lock_wait_timeout > 0 {
            cmd.arg("--lock-wait-timeout")
                .arg(format!("{}s", options.lock_wait_timeout));
        }

        // Encoding
        if let Some(ref enc) = options.encoding {
            cmd.arg("-E").arg(enc);
        }

        // Extra args
        for arg in &options.extra_args {
            cmd.arg(arg);
        }

        // Verbose output
        cmd.arg("-v");

        // Execute
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| BackupError::ProcessError(e.to_string()))?;

        // Read stderr for progress
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let app_clone = app.clone();
            let job_id_clone = job_id.to_string();

            std::thread::spawn(move || {
                for line in reader.lines() {
                    if let Ok(line) = line {
                        // Parse progress from pg_dump verbose output
                        let progress = BackupProgress {
                            phase: if line.contains("dumping") {
                                "dumping".to_string()
                            } else {
                                "processing".to_string()
                            },
                            current_object: Some(line.clone()),
                            objects_total: None,
                            objects_completed: 0,
                            bytes_written: 0,
                            elapsed_ms: 0,
                        };

                        let _ = app_clone.emit(
                            &format!("backup:progress:{}", job_id_clone),
                            progress,
                        );
                    }
                }
            });
        }

        // Wait for completion
        let status = child.wait().map_err(|e| BackupError::ProcessError(e.to_string()))?;

        job.end_time = Some(chrono::Utc::now());

        if status.success() {
            job.status = "completed".to_string();
        } else {
            job.status = "failed".to_string();
            job.errors.push(format!("pg_dump exited with code: {:?}", status.code()));
        }

        Ok(job)
    }

    /// Restore a backup using pg_restore
    pub async fn restore_backup(
        app: AppHandle,
        job_id: &str,
        connection_string: &str,
        options: &RestoreOptions,
    ) -> Result<BackupJob, BackupError> {
        let mut job = BackupJob {
            id: job_id.to_string(),
            job_type: "restore".to_string(),
            status: "running".to_string(),
            start_time: Some(chrono::Utc::now()),
            end_time: None,
            output: Vec::new(),
            errors: Vec::new(),
        };

        // Determine if we need psql (for plain format) or pg_restore
        let is_plain = Self::detect_backup_format(&options.input_path)
            .map(|f| f == "plain")
            .unwrap_or(false);

        let mut cmd = if is_plain {
            let mut c = Command::new("psql");
            c.arg(&connection_string);
            c.arg("-f").arg(&options.input_path);
            c
        } else {
            let mut c = Command::new("pg_restore");

            // Connection
            c.arg("-d").arg(&connection_string);

            // Input
            c.arg(&options.input_path);

            // Content flags
            if options.schema_only {
                c.arg("-s");
            }
            if options.data_only {
                c.arg("-a");
            }

            // Schema selection
            if let Some(ref schemas) = options.schemas {
                for schema in schemas {
                    c.arg("-n").arg(schema);
                }
            }

            // Table selection
            if let Some(ref tables) = options.tables {
                for table in tables {
                    c.arg("-t").arg(table);
                }
            }

            // Behavior
            if options.clean {
                c.arg("-c");
            }
            if options.if_exists {
                c.arg("--if-exists");
            }
            if options.no_owner {
                c.arg("-O");
            }
            if options.no_privileges {
                c.arg("-x");
            }
            if options.exit_on_error {
                c.arg("-e");
            }
            if options.single_transaction {
                c.arg("-1");
            }

            // Parallel jobs
            if options.jobs > 1 {
                c.arg("-j").arg(options.jobs.to_string());
            }

            // Disable triggers
            if options.disable_triggers {
                c.arg("--disable-triggers");
            }

            // Extra args
            for arg in &options.extra_args {
                c.arg(arg);
            }

            // Verbose
            c.arg("-v");

            c
        };

        // Execute
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| BackupError::ProcessError(e.to_string()))?;

        // Read stderr for progress
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let app_clone = app.clone();
            let job_id_clone = job_id.to_string();

            std::thread::spawn(move || {
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let progress = BackupProgress {
                            phase: "restoring".to_string(),
                            current_object: Some(line.clone()),
                            objects_total: None,
                            objects_completed: 0,
                            bytes_written: 0,
                            elapsed_ms: 0,
                        };

                        let _ = app_clone.emit(
                            &format!("restore:progress:{}", job_id_clone),
                            progress,
                        );
                    }
                }
            });
        }

        // Wait for completion
        let status = child.wait().map_err(|e| BackupError::ProcessError(e.to_string()))?;

        job.end_time = Some(chrono::Utc::now());

        if status.success() {
            job.status = "completed".to_string();
        } else {
            job.status = "failed".to_string();
            job.errors.push(format!("pg_restore exited with code: {:?}", status.code()));
        }

        Ok(job)
    }

    /// Get information about a backup file
    pub fn get_backup_info(path: &str) -> Result<BackupInfo, BackupError> {
        let format = Self::detect_backup_format(path)?;

        let metadata = std::fs::metadata(path)?;
        let size_bytes = metadata.len();

        // For custom format, use pg_restore -l to get contents
        let contents = if format == "custom" || format == "tar" {
            Self::get_backup_contents(path)?
        } else {
            BackupContents {
                schemas: Vec::new(),
                tables: Vec::new(),
                functions: 0,
                views: 0,
                sequences: 0,
                indexes: 0,
                triggers: 0,
                constraints: 0,
                has_blobs: false,
            }
        };

        Ok(BackupInfo {
            path: path.to_string(),
            format,
            size_bytes,
            created: chrono::Utc::now(), // Would parse from file
            database: String::new(),
            server_version: String::new(),
            pg_dump_version: String::new(),
            contents,
        })
    }

    fn detect_backup_format(path: &str) -> Result<String, BackupError> {
        let path = Path::new(path);

        // Check if directory
        if path.is_dir() {
            return Ok("directory".to_string());
        }

        // Check extension
        match path.extension().and_then(|e| e.to_str()) {
            Some("sql") => Ok("plain".to_string()),
            Some("tar") => Ok("tar".to_string()),
            Some("backup") | Some("dump") => Ok("custom".to_string()),
            _ => {
                // Try to detect from file header
                let mut file = std::fs::File::open(path)?;
                let mut header = [0u8; 5];
                std::io::Read::read_exact(&mut file, &mut header)?;

                // PostgreSQL custom format magic
                if &header[0..5] == b"PGDMP" {
                    Ok("custom".to_string())
                } else {
                    Ok("plain".to_string())
                }
            }
        }
    }

    fn get_backup_contents(path: &str) -> Result<BackupContents, BackupError> {
        let output = Command::new("pg_restore")
            .arg("-l")
            .arg(path)
            .output()
            .map_err(|e| BackupError::ProcessError(e.to_string()))?;

        let listing = String::from_utf8_lossy(&output.stdout);

        let mut contents = BackupContents {
            schemas: Vec::new(),
            tables: Vec::new(),
            functions: 0,
            views: 0,
            sequences: 0,
            indexes: 0,
            triggers: 0,
            constraints: 0,
            has_blobs: false,
        };

        for line in listing.lines() {
            if line.contains(" TABLE ") {
                // Parse table entry
                // Format: ID; OWNER; TYPE; SCHEMA; NAME; ...
                let parts: Vec<&str> = line.split(';').collect();
                if parts.len() >= 5 {
                    contents.tables.push(TableRef {
                        schema: parts[3].trim().to_string(),
                        name: parts[4].trim().to_string(),
                    });
                }
            } else if line.contains(" SCHEMA ") {
                let parts: Vec<&str> = line.split(';').collect();
                if parts.len() >= 5 {
                    contents.schemas.push(parts[4].trim().to_string());
                }
            } else if line.contains(" FUNCTION ") {
                contents.functions += 1;
            } else if line.contains(" VIEW ") {
                contents.views += 1;
            } else if line.contains(" SEQUENCE ") {
                contents.sequences += 1;
            } else if line.contains(" INDEX ") {
                contents.indexes += 1;
            } else if line.contains(" TRIGGER ") {
                contents.triggers += 1;
            } else if line.contains(" CONSTRAINT ") {
                contents.constraints += 1;
            } else if line.contains("BLOB") {
                contents.has_blobs = true;
            }
        }

        Ok(contents)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Process error: {0}")]
    ProcessError(String),

    #[error("Invalid backup format")]
    InvalidFormat,

    #[error("Backup cancelled")]
    Cancelled,
}
```

### 25.3 Backup Dialog Component

```svelte
<!-- src/lib/components/backup/BackupDialog.svelte -->
<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { save as saveDialog } from '@tauri-apps/plugin-dialog';
	import { listen } from '@tauri-apps/api/event';
	import type { BackupOptions, BackupProgress, BackupJob } from '$lib/types/backup';
	import type { Schema, Table } from '$lib/types/schema';

	interface Props {
		open: boolean;
		connId: string;
		schemas: Schema[];
	}

	let { open = $bindable(), connId, schemas }: Props = $props();

	const dispatch = createEventDispatcher<{
		complete: BackupJob;
		cancel: void;
	}>();

	// Options
	let outputPath = $state('');
	let format = $state<'custom' | 'plain' | 'directory' | 'tar'>('custom');
	let compression = $state(6);
	let includeSchema = $state(true);
	let includeData = $state(true);
	let includePrivileges = $state(true);
	let includeOwnership = $state(true);
	let selectedSchemas = $state<string[]>([]);
	let excludeTableData = $state<string[]>([]);
	let jobs = $state(4);

	// State
	let running = $state(false);
	let progress = $state<BackupProgress | null>(null);
	let output = $state<string[]>([]);
	let error = $state<string | null>(null);
	let jobId = $state('');

	// Get all tables for exclusion selection
	const allTables = $derived(schemas.flatMap((s) => s.tables.map((t) => `${s.name}.${t.name}`)));

	async function handleSelectPath() {
		const extensions = {
			custom: [{ name: 'PostgreSQL Backup', extensions: ['backup', 'dump'] }],
			plain: [{ name: 'SQL', extensions: ['sql'] }],
			directory: [],
			tar: [{ name: 'Tar Archive', extensions: ['tar'] }]
		};

		const selected = await saveDialog({
			defaultPath: `backup_${new Date().toISOString().split('T')[0]}`,
			filters: extensions[format] || []
		});

		if (selected) {
			outputPath = selected;
		}
	}

	async function handleBackup() {
		if (!outputPath) {
			error = 'Please select an output path';
			return;
		}

		running = true;
		error = null;
		output = [];
		jobId = crypto.randomUUID();

		// Listen for progress
		const unlisten = await listen<BackupProgress>(`backup:progress:${jobId}`, (event) => {
			progress = event.payload;
			if (event.payload.current_object) {
				output = [...output, event.payload.current_object];
			}
		});

		try {
			const options: BackupOptions = {
				connectionId: connId,
				outputPath,
				format,
				compression,
				includeSchema,
				includeData,
				includePrivileges,
				includeOwnership,
				schemas: selectedSchemas.length > 0 ? selectedSchemas : null,
				tables: null,
				excludeTables: [],
				excludeTableData,
				jobs,
				lockWaitTimeout: 30,
				noSync: false,
				encoding: null,
				extraArgs: []
			};

			const job = await invoke<BackupJob>('create_backup', {
				jobId,
				options
			});

			dispatch('complete', job);
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			running = false;
			unlisten();
		}
	}

	function handleCancel() {
		if (running) {
			// Would cancel the job
		}
		dispatch('cancel');
		open = false;
	}

	function formatSize(bytes: number): string {
		if (bytes >= 1_073_741_824) return (bytes / 1_073_741_824).toFixed(2) + ' GB';
		if (bytes >= 1_048_576) return (bytes / 1_048_576).toFixed(2) + ' MB';
		if (bytes >= 1024) return (bytes / 1024).toFixed(2) + ' KB';
		return bytes + ' B';
	}
</script>

{#if open}
	<div
		class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
		role="dialog"
		aria-modal="true"
	>
		<div
			class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-[600px] max-h-[85vh] flex flex-col"
		>
			<!-- Header -->
			<div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
				<h2 class="text-lg font-semibold">Backup Database</h2>
			</div>

			<!-- Body -->
			<div class="flex-1 overflow-auto p-6 space-y-6">
				{#if error}
					<div
						class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200
                      dark:border-red-800 rounded text-red-700 dark:text-red-400 text-sm"
					>
						{error}
					</div>
				{/if}

				<!-- Output Path -->
				<div>
					<label class="block text-sm font-medium mb-1">Output</label>
					<div class="flex gap-2">
						<input
							type="text"
							bind:value={outputPath}
							placeholder="Select output path..."
							class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                     bg-gray-50 dark:bg-gray-900 text-sm"
							readonly
						/>
						<button
							onclick={handleSelectPath}
							class="px-4 py-2 bg-gray-200 dark:bg-gray-700 rounded hover:bg-gray-300
                     dark:hover:bg-gray-600 text-sm"
						>
							Browse...
						</button>
					</div>
				</div>

				<!-- Format -->
				<div>
					<label class="block text-sm font-medium mb-2">Format</label>
					<div class="grid grid-cols-2 gap-3">
						{#each [{ value: 'custom', label: 'Custom (.backup)', desc: 'Recommended. Compressed, selective restore.' }, { value: 'plain', label: 'Plain SQL (.sql)', desc: 'Human-readable SQL script.' }, { value: 'directory', label: 'Directory', desc: 'Parallel dump support.' }, { value: 'tar', label: 'Tar Archive (.tar)', desc: 'Portable archive format.' }] as fmt}
							<label
								class="flex items-start gap-3 p-3 border rounded cursor-pointer
                       {format === fmt.value
									? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
									: 'border-gray-200 dark:border-gray-700'}"
							>
								<input type="radio" bind:group={format} value={fmt.value} class="mt-1" />
								<div>
									<div class="font-medium text-sm">{fmt.label}</div>
									<div class="text-xs text-gray-500">{fmt.desc}</div>
								</div>
							</label>
						{/each}
					</div>
				</div>

				<!-- Content Options -->
				<div>
					<label class="block text-sm font-medium mb-2">Objects</label>
					<div class="grid grid-cols-2 gap-3">
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={includeSchema} class="rounded" />
							<span class="text-sm">Schema definitions</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={includeData} class="rounded" />
							<span class="text-sm">Data</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={includePrivileges} class="rounded" />
							<span class="text-sm">Privileges (GRANT/REVOKE)</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={includeOwnership} class="rounded" />
							<span class="text-sm">Ownership</span>
						</label>
					</div>
				</div>

				<!-- Schema Selection -->
				<div>
					<label class="block text-sm font-medium mb-2">
						Schemas
						<span class="font-normal text-gray-500">(leave empty for all)</span>
					</label>
					<div
						class="max-h-32 overflow-y-auto border border-gray-200 dark:border-gray-700
                      rounded p-2 space-y-1"
					>
						{#each schemas as schema}
							<label class="flex items-center gap-2 cursor-pointer text-sm">
								<input
									type="checkbox"
									checked={selectedSchemas.includes(schema.name)}
									onchange={(e) => {
										if (e.currentTarget.checked) {
											selectedSchemas = [...selectedSchemas, schema.name];
										} else {
											selectedSchemas = selectedSchemas.filter((s) => s !== schema.name);
										}
									}}
									class="rounded"
								/>
								<span>{schema.name}</span>
							</label>
						{/each}
					</div>
				</div>

				<!-- Advanced Options -->
				<details class="group">
					<summary class="cursor-pointer text-sm font-medium text-gray-700 dark:text-gray-300">
						Advanced Options
					</summary>
					<div class="mt-3 space-y-4 pl-4">
						<div class="grid grid-cols-2 gap-4">
							<div>
								<label class="block text-xs text-gray-500 mb-1">Compression (0-9)</label>
								<input type="range" bind:value={compression} min="0" max="9" class="w-full" />
								<div class="text-xs text-gray-500 text-center">{compression}</div>
							</div>
							<div>
								<label class="block text-xs text-gray-500 mb-1">Parallel Jobs</label>
								<input
									type="number"
									bind:value={jobs}
									min="1"
									max="32"
									disabled={format !== 'directory'}
									class="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded
                         text-sm disabled:opacity-50"
								/>
							</div>
						</div>

						<!-- Exclude Table Data -->
						<div>
							<label class="block text-xs text-gray-500 mb-1">
								Exclude data for tables (schema only)
							</label>
							<select
								multiple
								bind:value={excludeTableData}
								class="w-full h-24 px-2 py-1 border border-gray-300 dark:border-gray-600 rounded
                       text-sm bg-white dark:bg-gray-700"
							>
								{#each allTables as table}
									<option value={table}>{table}</option>
								{/each}
							</select>
						</div>
					</div>
				</details>

				<!-- Progress -->
				{#if running}
					<div class="space-y-2">
						<div class="flex items-center gap-2">
							<div class="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600"></div>
							<span class="text-sm">
								{progress?.phase ?? 'Starting backup...'}
							</span>
						</div>
						{#if progress?.current_object}
							<div class="text-xs text-gray-500 font-mono truncate">
								{progress.current_object}
							</div>
						{/if}
						{#if output.length > 0}
							<div
								class="max-h-32 overflow-y-auto bg-gray-100 dark:bg-gray-900 rounded
                          p-2 font-mono text-xs"
							>
								{#each output.slice(-20) as line}
									<div class="truncate">{line}</div>
								{/each}
							</div>
						{/if}
					</div>
				{/if}
			</div>

			<!-- Footer -->
			<div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-2">
				<button
					onclick={handleCancel}
					class="px-4 py-2 text-sm text-gray-700 dark:text-gray-300
                 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
				>
					{running ? 'Cancel' : 'Close'}
				</button>
				<button
					onclick={handleBackup}
					disabled={running || !outputPath}
					class="px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700
                 disabled:opacity-50 disabled:cursor-not-allowed"
				>
					{running ? 'Backing up...' : 'Create Backup'}
				</button>
			</div>
		</div>
	</div>
{/if}
```

### 25.4 Restore Dialog Component

```svelte
<!-- src/lib/components/backup/RestoreDialog.svelte -->
<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import { listen } from '@tauri-apps/api/event';
	import type { RestoreOptions, BackupProgress, BackupJob, BackupInfo } from '$lib/types/backup';

	interface Props {
		open: boolean;
		connId: string;
	}

	let { open = $bindable(), connId }: Props = $props();

	const dispatch = createEventDispatcher<{
		complete: BackupJob;
		cancel: void;
	}>();

	// Options
	let inputPath = $state('');
	let backupInfo = $state<BackupInfo | null>(null);
	let schemaOnly = $state(false);
	let dataOnly = $state(false);
	let clean = $state(false);
	let ifExists = $state(true);
	let noOwner = $state(false);
	let noPrivileges = $state(false);
	let exitOnError = $state(true);
	let singleTransaction = $state(true);
	let jobs = $state(4);
	let disableTriggers = $state(false);

	// State
	let running = $state(false);
	let progress = $state<BackupProgress | null>(null);
	let output = $state<string[]>([]);
	let error = $state<string | null>(null);
	let jobId = $state('');

	async function handleSelectFile() {
		const selected = await openDialog({
			multiple: false,
			filters: [
				{ name: 'Backup Files', extensions: ['backup', 'dump', 'sql', 'tar'] },
				{ name: 'All Files', extensions: ['*'] }
			]
		});

		if (selected && typeof selected === 'string') {
			inputPath = selected;
			await analyzeBackup();
		}
	}

	async function analyzeBackup() {
		try {
			backupInfo = await invoke<BackupInfo>('get_backup_info', { path: inputPath });
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
			backupInfo = null;
		}
	}

	async function handleRestore() {
		if (!inputPath) {
			error = 'Please select a backup file';
			return;
		}

		running = true;
		error = null;
		output = [];
		jobId = crypto.randomUUID();

		// Listen for progress
		const unlisten = await listen<BackupProgress>(`restore:progress:${jobId}`, (event) => {
			progress = event.payload;
			if (event.payload.current_object) {
				output = [...output, event.payload.current_object];
			}
		});

		try {
			const options: RestoreOptions = {
				connectionId: connId,
				inputPath,
				targetDatabase: null,
				createDatabase: false,
				schemaOnly,
				dataOnly,
				schemas: null,
				tables: null,
				clean,
				ifExists,
				noOwner,
				noPrivileges,
				exitOnError,
				singleTransaction,
				jobs,
				disableTriggers,
				extraArgs: []
			};

			const job = await invoke<BackupJob>('restore_backup', {
				jobId,
				options
			});

			dispatch('complete', job);
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			running = false;
			unlisten();
		}
	}

	function handleCancel() {
		dispatch('cancel');
		open = false;
	}

	function formatSize(bytes: number): string {
		if (bytes >= 1_073_741_824) return (bytes / 1_073_741_824).toFixed(2) + ' GB';
		if (bytes >= 1_048_576) return (bytes / 1_048_576).toFixed(2) + ' MB';
		if (bytes >= 1024) return (bytes / 1024).toFixed(2) + ' KB';
		return bytes + ' B';
	}
</script>

{#if open}
	<div
		class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
		role="dialog"
		aria-modal="true"
	>
		<div
			class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-[600px] max-h-[85vh] flex flex-col"
		>
			<!-- Header -->
			<div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
				<h2 class="text-lg font-semibold">Restore Database</h2>
			</div>

			<!-- Body -->
			<div class="flex-1 overflow-auto p-6 space-y-6">
				{#if error}
					<div
						class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200
                      dark:border-red-800 rounded text-red-700 dark:text-red-400 text-sm"
					>
						{error}
					</div>
				{/if}

				<!-- Source File -->
				<div>
					<label class="block text-sm font-medium mb-1">Source</label>
					<div class="flex gap-2">
						<input
							type="text"
							bind:value={inputPath}
							placeholder="Select backup file..."
							class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
                     bg-gray-50 dark:bg-gray-900 text-sm"
							readonly
						/>
						<button
							onclick={handleSelectFile}
							class="px-4 py-2 bg-gray-200 dark:bg-gray-700 rounded hover:bg-gray-300
                     dark:hover:bg-gray-600 text-sm"
						>
							Browse...
						</button>
					</div>
				</div>

				<!-- Backup Info -->
				{#if backupInfo}
					<div class="p-4 bg-gray-50 dark:bg-gray-900/50 rounded space-y-2">
						<div class="grid grid-cols-3 gap-4 text-sm">
							<div>
								<span class="text-gray-500">Format:</span>
								<span class="ml-1 font-medium capitalize">{backupInfo.format}</span>
							</div>
							<div>
								<span class="text-gray-500">Size:</span>
								<span class="ml-1 font-medium">{formatSize(backupInfo.sizeBytes)}</span>
							</div>
							<div>
								<span class="text-gray-500">Database:</span>
								<span class="ml-1 font-medium">{backupInfo.database || 'Unknown'}</span>
							</div>
						</div>
						{#if backupInfo.contents.tables.length > 0}
							<div class="text-sm">
								<span class="text-gray-500">Contents:</span>
								<span class="ml-1">
									{backupInfo.contents.schemas.length} schemas,
									{backupInfo.contents.tables.length} tables,
									{backupInfo.contents.functions} functions
								</span>
							</div>
						{/if}
					</div>
				{/if}

				<!-- Options -->
				<div>
					<label class="block text-sm font-medium mb-2">Restore Options</label>
					<div class="grid grid-cols-2 gap-3">
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={schemaOnly} class="rounded" />
							<span class="text-sm">Schema only (no data)</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={dataOnly} class="rounded" />
							<span class="text-sm">Data only (no schema)</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={clean} class="rounded" />
							<span class="text-sm">Clean (drop objects first)</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={noOwner} class="rounded" />
							<span class="text-sm">Skip ownership</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={noPrivileges} class="rounded" />
							<span class="text-sm">Skip privileges</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={exitOnError} class="rounded" />
							<span class="text-sm">Exit on error</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={singleTransaction} class="rounded" />
							<span class="text-sm">Single transaction</span>
						</label>
						<label class="flex items-center gap-2 cursor-pointer">
							<input type="checkbox" bind:checked={disableTriggers} class="rounded" />
							<span class="text-sm">Disable triggers</span>
						</label>
					</div>
				</div>

				<!-- Parallel Jobs -->
				<div>
					<label class="block text-sm font-medium mb-1">Parallel Jobs</label>
					<input
						type="number"
						bind:value={jobs}
						min="1"
						max="32"
						class="w-24 px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm"
					/>
				</div>

				<!-- Warning -->
				{#if clean}
					<div
						class="p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200
                      dark:border-amber-800 rounded text-amber-800 dark:text-amber-200 text-sm"
					>
						<strong>Warning:</strong> Clean mode will drop existing objects before restoring. This is
						a destructive operation.
					</div>
				{/if}

				<!-- Progress -->
				{#if running}
					<div class="space-y-2">
						<div class="flex items-center gap-2">
							<div class="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600"></div>
							<span class="text-sm">
								{progress?.phase ?? 'Starting restore...'}
							</span>
						</div>
						{#if output.length > 0}
							<div
								class="max-h-32 overflow-y-auto bg-gray-100 dark:bg-gray-900 rounded
                          p-2 font-mono text-xs"
							>
								{#each output.slice(-20) as line}
									<div class="truncate">{line}</div>
								{/each}
							</div>
						{/if}
					</div>
				{/if}
			</div>

			<!-- Footer -->
			<div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-2">
				<button
					onclick={handleCancel}
					class="px-4 py-2 text-sm text-gray-700 dark:text-gray-300
                 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
				>
					{running ? 'Cancel' : 'Close'}
				</button>
				<button
					onclick={handleRestore}
					disabled={running || !inputPath}
					class="px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700
                 disabled:opacity-50 disabled:cursor-not-allowed"
				>
					{running ? 'Restoring...' : 'Restore'}
				</button>
			</div>
		</div>
	</div>
{/if}
```

## Acceptance Criteria

1. **Backup Creation**
   - [ ] Support all pg_dump formats (custom, plain, directory, tar)
   - [ ] Configure compression level
   - [ ] Select schemas and tables
   - [ ] Exclude specific tables from data backup
   - [ ] Configure parallel jobs for directory format
   - [ ] Show real-time progress

2. **Restore**
   - [ ] Auto-detect backup format
   - [ ] Show backup file info before restore
   - [ ] Support schema-only and data-only restore
   - [ ] Clean mode with warnings
   - [ ] Single transaction option
   - [ ] Parallel restore support

3. **Progress and Output**
   - [ ] Real-time progress display
   - [ ] Capture pg_dump/pg_restore output
   - [ ] Show errors clearly
   - [ ] Support cancellation

## MCP Testing Instructions

### Tauri MCP Testing

```typescript
// Create backup
await mcp___hypothesi_tauri_mcp_server__ipc_execute_command({
	command: 'create_backup',
	args: {
		jobId: 'test-job',
		options: {
			connectionId: 'test-conn',
			outputPath: '/tmp/backup.dump',
			format: 'custom',
			compression: 6,
			includeSchema: true,
			includeData: true,
			includePrivileges: true,
			includeOwnership: true,
			schemas: null,
			tables: null,
			excludeTables: [],
			excludeTableData: [],
			jobs: 4,
			lockWaitTimeout: 30,
			noSync: false,
			encoding: null,
			extraArgs: []
		}
	}
});

// Get backup info
await mcp___hypothesi_tauri_mcp_server__ipc_execute_command({
	command: 'get_backup_info',
	args: { path: '/tmp/backup.dump' }
});

// Restore backup
await mcp___hypothesi_tauri_mcp_server__ipc_execute_command({
	command: 'restore_backup',
	args: {
		jobId: 'restore-job',
		options: {
			connectionId: 'test-conn',
			inputPath: '/tmp/backup.dump',
			clean: false,
			noOwner: true,
			exitOnError: true,
			jobs: 4
		}
	}
});
```

### Playwright MCP Testing

```typescript
// Open backup dialog
await mcp__playwright__browser_click({
	element: 'Backup menu item',
	ref: 'button:has-text("Backup")'
});

// Take screenshot
await mcp__playwright__browser_take_screenshot({
	filename: 'backup-dialog.png'
});

// Test restore dialog
await mcp__playwright__browser_click({
	element: 'Restore menu item',
	ref: 'button:has-text("Restore")'
});

await mcp__playwright__browser_take_screenshot({
	filename: 'restore-dialog.png'
});
```
