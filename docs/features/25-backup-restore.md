# Feature 25: Backup and Restore

## Overview

Backup and Restore provides GUI interfaces for pg_dump and pg_restore operations, enabling database backup creation and restoration with full control over formats, objects, and options. Built entirely in Rust with GPUI for the UI layer.

## Goals

- Create backups using pg_dump with all format options
- Restore backups using pg_restore
- Select specific objects (schemas, tables) for backup
- Configure compression and parallel jobs
- Show real-time progress and output
- Support backup history and management

## Dependencies

- Feature 07: Connection Pool Management
- Feature 10: Schema Cache (for object selection)

## Technical Specification

### 25.1 Backup/Restore Data Models

```rust
// src/backup/types.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use chrono::{DateTime, Utc};

/// Unique identifier for a backup job
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BackupJobId(pub String);

impl BackupJobId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

/// Backup output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupFormat {
    Custom,
    Plain,
    Directory,
    Tar,
}

impl BackupFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            BackupFormat::Custom => "Custom (.backup)",
            BackupFormat::Plain => "Plain SQL (.sql)",
            BackupFormat::Directory => "Directory",
            BackupFormat::Tar => "Tar Archive (.tar)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            BackupFormat::Custom => "Recommended. Compressed, selective restore.",
            BackupFormat::Plain => "Human-readable SQL script.",
            BackupFormat::Directory => "Parallel dump support.",
            BackupFormat::Tar => "Portable archive format.",
        }
    }

    pub fn pg_dump_flag(&self) -> &'static str {
        match self {
            BackupFormat::Custom => "c",
            BackupFormat::Plain => "p",
            BackupFormat::Directory => "d",
            BackupFormat::Tar => "t",
        }
    }

    pub fn default_extension(&self) -> &'static str {
        match self {
            BackupFormat::Custom => "backup",
            BackupFormat::Plain => "sql",
            BackupFormat::Directory => "",
            BackupFormat::Tar => "tar",
        }
    }

    pub fn all() -> &'static [BackupFormat] {
        &[
            BackupFormat::Custom,
            BackupFormat::Plain,
            BackupFormat::Directory,
            BackupFormat::Tar,
        ]
    }
}

/// Options for creating a backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupOptions {
    /// Connection ID to backup from
    pub connection_id: String,

    /// Output file/directory path
    pub output_path: PathBuf,

    /// Backup format
    pub format: BackupFormat,

    /// Compression level (0-9, 0 = none)
    pub compression: u8,

    /// Include schema definitions
    pub include_schema: bool,

    /// Include table data
    pub include_data: bool,

    /// Include GRANT/REVOKE statements
    pub include_privileges: bool,

    /// Include ownership statements
    pub include_ownership: bool,

    /// Specific schemas to backup (None = all)
    pub schemas: Option<Vec<String>>,

    /// Specific tables to backup (None = all)
    pub tables: Option<Vec<String>>,

    /// Tables to exclude from backup
    pub exclude_tables: Vec<String>,

    /// Tables to exclude data from (schema only)
    pub exclude_table_data: Vec<String>,

    /// Number of parallel dump jobs (directory format only)
    pub jobs: u8,

    /// Lock wait timeout in seconds
    pub lock_wait_timeout: u32,

    /// Skip fsync
    pub no_sync: bool,

    /// Output encoding
    pub encoding: Option<String>,

    /// Additional pg_dump arguments
    pub extra_args: Vec<String>,
}

impl Default for BackupOptions {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            output_path: PathBuf::new(),
            format: BackupFormat::Custom,
            compression: 6,
            include_schema: true,
            include_data: true,
            include_privileges: true,
            include_ownership: true,
            schemas: None,
            tables: None,
            exclude_tables: Vec::new(),
            exclude_table_data: Vec::new(),
            jobs: 4,
            lock_wait_timeout: 30,
            no_sync: false,
            encoding: None,
            extra_args: Vec::new(),
        }
    }
}

/// Options for restoring a backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreOptions {
    /// Connection ID to restore to
    pub connection_id: String,

    /// Input file/directory path
    pub input_path: PathBuf,

    /// Target database (None = current connection database)
    pub target_database: Option<String>,

    /// Create database before restore
    pub create_database: bool,

    /// Restore schema only
    pub schema_only: bool,

    /// Restore data only
    pub data_only: bool,

    /// Specific schemas to restore (None = all)
    pub schemas: Option<Vec<String>>,

    /// Specific tables to restore (None = all)
    pub tables: Option<Vec<String>>,

    /// Drop objects before restore
    pub clean: bool,

    /// Add IF EXISTS to DROP statements
    pub if_exists: bool,

    /// Skip ownership restoration
    pub no_owner: bool,

    /// Skip privilege restoration
    pub no_privileges: bool,

    /// Exit on first error
    pub exit_on_error: bool,

    /// Wrap restore in single transaction
    pub single_transaction: bool,

    /// Number of parallel restore jobs
    pub jobs: u8,

    /// Disable triggers during restore
    pub disable_triggers: bool,

    /// Additional pg_restore arguments
    pub extra_args: Vec<String>,
}

impl Default for RestoreOptions {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            input_path: PathBuf::new(),
            target_database: None,
            create_database: false,
            schema_only: false,
            data_only: false,
            schemas: None,
            tables: None,
            clean: false,
            if_exists: true,
            no_owner: false,
            no_privileges: false,
            exit_on_error: true,
            single_transaction: true,
            jobs: 4,
            disable_triggers: false,
            extra_args: Vec::new(),
        }
    }
}

/// Status of a backup/restore job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Pending => "Pending",
            JobStatus::Running => "Running",
            JobStatus::Completed => "Completed",
            JobStatus::Failed => "Failed",
            JobStatus::Cancelled => "Cancelled",
        }
    }
}

/// A backup or restore job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupJob {
    pub id: BackupJobId,
    pub job_type: JobType,
    pub status: JobStatus,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub output: Vec<String>,
    pub errors: Vec<String>,
    pub progress: Option<BackupProgress>,
}

/// Type of backup job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobType {
    Backup,
    Restore,
}

/// Progress information for a backup/restore job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupProgress {
    pub phase: String,
    pub current_object: Option<String>,
    pub objects_total: Option<i64>,
    pub objects_completed: i64,
    pub bytes_written: i64,
    pub elapsed_ms: i64,
}

/// Information about a backup file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub format: BackupFormat,
    pub size_bytes: u64,
    pub created: DateTime<Utc>,
    pub database: String,
    pub server_version: String,
    pub pg_dump_version: String,
    pub contents: BackupContents,
}

/// Contents of a backup file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

/// Reference to a table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRef {
    pub schema: String,
    pub name: String,
}
```

### 25.2 Backup Service

```rust
// src/backup/service.rs

use crate::backup::types::*;
use crate::connection::ConnectionPool;
use crate::error::{Result, TuskError};
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::mpsc;
use std::thread;

pub struct BackupService;

impl BackupService {
    /// Create a backup using pg_dump
    pub async fn create_backup(
        connection_string: &str,
        options: &BackupOptions,
        progress_callback: impl Fn(BackupProgress) + Send + 'static,
    ) -> Result<BackupJob> {
        let job_id = BackupJobId::new();
        let mut job = BackupJob {
            id: job_id.clone(),
            job_type: JobType::Backup,
            status: JobStatus::Running,
            start_time: Some(chrono::Utc::now()),
            end_time: None,
            output: Vec::new(),
            errors: Vec::new(),
            progress: None,
        };

        // Build pg_dump command
        let mut cmd = Command::new("pg_dump");

        // Connection string
        cmd.arg(connection_string);

        // Format
        cmd.arg("-F").arg(options.format.pg_dump_flag());

        // Output path
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

        // Parallel jobs (directory format only)
        if options.jobs > 1 && options.format == BackupFormat::Directory {
            cmd.arg("-j").arg(options.jobs.to_string());
        }

        // Lock timeout
        if options.lock_wait_timeout > 0 {
            cmd.arg("--lock-wait-timeout")
                .arg(format!("{}s", options.lock_wait_timeout));
        }

        // No sync
        if options.no_sync {
            cmd.arg("--no-sync");
        }

        // Encoding
        if let Some(ref enc) = options.encoding {
            cmd.arg("-E").arg(enc);
        }

        // Extra args
        for arg in &options.extra_args {
            cmd.arg(arg);
        }

        // Verbose output for progress
        cmd.arg("-v");

        // Execute with output capture
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()
            .map_err(|e| TuskError::Backup(format!("Failed to start pg_dump: {}", e)))?;

        // Capture stderr for progress
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let start_time = std::time::Instant::now();

            thread::spawn(move || {
                let mut objects_completed = 0i64;

                for line in reader.lines().flatten() {
                    objects_completed += 1;

                    let progress = BackupProgress {
                        phase: if line.contains("dumping") {
                            "Dumping".to_string()
                        } else {
                            "Processing".to_string()
                        },
                        current_object: Some(line),
                        objects_total: None,
                        objects_completed,
                        bytes_written: 0,
                        elapsed_ms: start_time.elapsed().as_millis() as i64,
                    };

                    progress_callback(progress);
                }
            });
        }

        // Wait for completion
        let status = child.wait()
            .map_err(|e| TuskError::Backup(format!("Failed to wait for pg_dump: {}", e)))?;

        job.end_time = Some(chrono::Utc::now());

        if status.success() {
            job.status = JobStatus::Completed;
        } else {
            job.status = JobStatus::Failed;
            job.errors.push(format!("pg_dump exited with code: {:?}", status.code()));
        }

        Ok(job)
    }

    /// Restore a backup using pg_restore or psql
    pub async fn restore_backup(
        connection_string: &str,
        options: &RestoreOptions,
        progress_callback: impl Fn(BackupProgress) + Send + 'static,
    ) -> Result<BackupJob> {
        let job_id = BackupJobId::new();
        let mut job = BackupJob {
            id: job_id.clone(),
            job_type: JobType::Restore,
            status: JobStatus::Running,
            start_time: Some(chrono::Utc::now()),
            end_time: None,
            output: Vec::new(),
            errors: Vec::new(),
            progress: None,
        };

        // Detect backup format
        let format = Self::detect_backup_format(&options.input_path)?;
        let is_plain = format == BackupFormat::Plain;

        let mut cmd = if is_plain {
            // Use psql for plain SQL files
            let mut c = Command::new("psql");
            c.arg(connection_string);
            c.arg("-f").arg(&options.input_path);
            c
        } else {
            // Use pg_restore for other formats
            let mut c = Command::new("pg_restore");

            // Connection
            c.arg("-d").arg(connection_string);

            // Input path
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

            // Behavior flags
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

        // Execute with output capture
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()
            .map_err(|e| TuskError::Backup(format!("Failed to start pg_restore: {}", e)))?;

        // Capture stderr for progress
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let start_time = std::time::Instant::now();

            thread::spawn(move || {
                let mut objects_completed = 0i64;

                for line in reader.lines().flatten() {
                    objects_completed += 1;

                    let progress = BackupProgress {
                        phase: "Restoring".to_string(),
                        current_object: Some(line),
                        objects_total: None,
                        objects_completed,
                        bytes_written: 0,
                        elapsed_ms: start_time.elapsed().as_millis() as i64,
                    };

                    progress_callback(progress);
                }
            });
        }

        // Wait for completion
        let status = child.wait()
            .map_err(|e| TuskError::Backup(format!("Failed to wait for pg_restore: {}", e)))?;

        job.end_time = Some(chrono::Utc::now());

        if status.success() {
            job.status = JobStatus::Completed;
        } else {
            job.status = JobStatus::Failed;
            job.errors.push(format!("pg_restore exited with code: {:?}", status.code()));
        }

        Ok(job)
    }

    /// Get information about a backup file
    pub fn get_backup_info(path: &Path) -> Result<BackupInfo> {
        let format = Self::detect_backup_format(path)?;

        let metadata = std::fs::metadata(path)?;
        let size_bytes = if path.is_dir() {
            // Calculate directory size
            Self::get_directory_size(path)?
        } else {
            metadata.len()
        };

        // For custom/tar format, use pg_restore -l to get contents
        let contents = if matches!(format, BackupFormat::Custom | BackupFormat::Tar) {
            Self::get_backup_contents(path)?
        } else {
            BackupContents::default()
        };

        Ok(BackupInfo {
            path: path.to_path_buf(),
            format,
            size_bytes,
            created: chrono::Utc::now(), // Would parse from file metadata
            database: String::new(),
            server_version: String::new(),
            pg_dump_version: String::new(),
            contents,
        })
    }

    /// Detect backup format from file
    pub fn detect_backup_format(path: &Path) -> Result<BackupFormat> {
        // Check if directory
        if path.is_dir() {
            return Ok(BackupFormat::Directory);
        }

        // Check extension
        match path.extension().and_then(|e| e.to_str()) {
            Some("sql") => return Ok(BackupFormat::Plain),
            Some("tar") => return Ok(BackupFormat::Tar),
            Some("backup") | Some("dump") => return Ok(BackupFormat::Custom),
            _ => {}
        }

        // Try to detect from file header
        let mut file = std::fs::File::open(path)?;
        let mut header = [0u8; 5];
        use std::io::Read;
        file.read_exact(&mut header)?;

        // PostgreSQL custom format magic
        if &header[0..5] == b"PGDMP" {
            Ok(BackupFormat::Custom)
        } else {
            Ok(BackupFormat::Plain)
        }
    }

    /// Get contents listing from backup file
    fn get_backup_contents(path: &Path) -> Result<BackupContents> {
        let output = Command::new("pg_restore")
            .arg("-l")
            .arg(path)
            .output()
            .map_err(|e| TuskError::Backup(format!("Failed to run pg_restore -l: {}", e)))?;

        let listing = String::from_utf8_lossy(&output.stdout);

        let mut contents = BackupContents::default();

        for line in listing.lines() {
            // Skip comments and empty lines
            if line.starts_with(';') || line.trim().is_empty() {
                continue;
            }

            // Parse table of contents entry
            // Format: ID; OWNER; TYPE; SCHEMA; NAME; ...
            let parts: Vec<&str> = line.split(';').collect();

            if parts.len() >= 3 {
                let obj_type = parts[2].trim();

                match obj_type {
                    "TABLE" | "TABLE DATA" => {
                        if parts.len() >= 5 && obj_type == "TABLE" {
                            contents.tables.push(TableRef {
                                schema: parts[3].trim().to_string(),
                                name: parts[4].trim().split_whitespace().next()
                                    .unwrap_or("").to_string(),
                            });
                        }
                    }
                    "SCHEMA" => {
                        if parts.len() >= 5 {
                            let schema_name = parts[4].trim().split_whitespace().next()
                                .unwrap_or("").to_string();
                            if !contents.schemas.contains(&schema_name) {
                                contents.schemas.push(schema_name);
                            }
                        }
                    }
                    "FUNCTION" => contents.functions += 1,
                    "VIEW" => contents.views += 1,
                    "SEQUENCE" | "SEQUENCE OWNED BY" | "SEQUENCE SET" => contents.sequences += 1,
                    "INDEX" => contents.indexes += 1,
                    "TRIGGER" => contents.triggers += 1,
                    "CONSTRAINT" | "FK CONSTRAINT" | "CHECK CONSTRAINT" => contents.constraints += 1,
                    "BLOB" | "BLOBS" => contents.has_blobs = true,
                    _ => {}
                }
            }
        }

        Ok(contents)
    }

    /// Get total size of a directory
    fn get_directory_size(path: &Path) -> Result<u64> {
        let mut total = 0u64;

        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;

            if metadata.is_dir() {
                total += Self::get_directory_size(&entry.path())?;
            } else {
                total += metadata.len();
            }
        }

        Ok(total)
    }

    /// Cancel a running backup/restore job
    pub fn cancel_job(job_id: &BackupJobId) -> Result<()> {
        // Implementation would signal cancellation via process kill
        // or shared state
        Ok(())
    }
}
```

### 25.3 Backup State Management

```rust
// src/backup/state.rs

use crate::backup::types::*;
use gpui::Global;
use parking_lot::RwLock;
use std::sync::Arc;
use std::collections::HashMap;

/// Global backup state
pub struct BackupState {
    inner: Arc<RwLock<BackupStateInner>>,
}

struct BackupStateInner {
    /// All backup jobs
    jobs: HashMap<BackupJobId, BackupJob>,
    /// Recent backup files
    recent_backups: Vec<BackupInfo>,
}

impl Global for BackupState {}

impl BackupState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(BackupStateInner {
                jobs: HashMap::new(),
                recent_backups: Vec::new(),
            })),
        }
    }

    /// Register a new job
    pub fn register_job(&self, job: BackupJob) {
        let mut inner = self.inner.write();
        inner.jobs.insert(job.id.clone(), job);
    }

    /// Update job status
    pub fn update_job_status(&self, job_id: &BackupJobId, status: JobStatus) {
        let mut inner = self.inner.write();
        if let Some(job) = inner.jobs.get_mut(job_id) {
            job.status = status;
            if matches!(status, JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled) {
                job.end_time = Some(chrono::Utc::now());
            }
        }
    }

    /// Update job progress
    pub fn update_job_progress(&self, job_id: &BackupJobId, progress: BackupProgress) {
        let mut inner = self.inner.write();
        if let Some(job) = inner.jobs.get_mut(job_id) {
            if let Some(ref obj) = progress.current_object {
                job.output.push(obj.clone());
            }
            job.progress = Some(progress);
        }
    }

    /// Add job error
    pub fn add_job_error(&self, job_id: &BackupJobId, error: String) {
        let mut inner = self.inner.write();
        if let Some(job) = inner.jobs.get_mut(job_id) {
            job.errors.push(error);
        }
    }

    /// Get a job by ID
    pub fn get_job(&self, job_id: &BackupJobId) -> Option<BackupJob> {
        self.inner.read().jobs.get(job_id).cloned()
    }

    /// Get all running jobs
    pub fn running_jobs(&self) -> Vec<BackupJob> {
        self.inner.read().jobs.values()
            .filter(|j| j.status == JobStatus::Running)
            .cloned()
            .collect()
    }

    /// Get all jobs
    pub fn all_jobs(&self) -> Vec<BackupJob> {
        self.inner.read().jobs.values().cloned().collect()
    }

    /// Add to recent backups
    pub fn add_recent_backup(&self, info: BackupInfo) {
        let mut inner = self.inner.write();
        // Keep only last 10
        inner.recent_backups.retain(|b| b.path != info.path);
        inner.recent_backups.insert(0, info);
        inner.recent_backups.truncate(10);
    }

    /// Get recent backups
    pub fn recent_backups(&self) -> Vec<BackupInfo> {
        self.inner.read().recent_backups.clone()
    }

    /// Remove a job
    pub fn remove_job(&self, job_id: &BackupJobId) {
        self.inner.write().jobs.remove(job_id);
    }
}
```

### 25.4 Backup Dialog Component

```rust
// src/backup/dialog.rs

use crate::backup::types::*;
use crate::backup::service::BackupService;
use crate::backup::state::BackupState;
use crate::connection::ConnectionState;
use crate::schema::SchemaCache;
use crate::ui::{Button, Modal, Select, Checkbox, Input, ProgressBar};
use gpui::*;
use std::path::PathBuf;

/// Backup creation dialog
pub struct BackupDialog {
    conn_id: String,

    // Options
    output_path: String,
    format: BackupFormat,
    compression: u8,
    include_schema: bool,
    include_data: bool,
    include_privileges: bool,
    include_ownership: bool,
    selected_schemas: Vec<String>,
    exclude_table_data: Vec<String>,
    jobs: u8,

    // Available schemas from cache
    available_schemas: Vec<String>,
    all_tables: Vec<String>,

    // State
    running: bool,
    progress: Option<BackupProgress>,
    output_lines: Vec<String>,
    error: Option<String>,
    job_id: Option<BackupJobId>,

    focus_handle: FocusHandle,
}

impl BackupDialog {
    pub fn new(conn_id: String, cx: &mut Context<Self>) -> Self {
        // Load available schemas from cache
        let available_schemas = cx.global::<SchemaCache>()
            .get_schemas(&conn_id)
            .map(|schemas| schemas.iter().map(|s| s.name.clone()).collect())
            .unwrap_or_default();

        let all_tables = cx.global::<SchemaCache>()
            .get_schemas(&conn_id)
            .map(|schemas| {
                schemas.iter()
                    .flat_map(|s| s.tables.iter().map(|t| format!("{}.{}", s.name, t.name)))
                    .collect()
            })
            .unwrap_or_default();

        Self {
            conn_id,
            output_path: String::new(),
            format: BackupFormat::Custom,
            compression: 6,
            include_schema: true,
            include_data: true,
            include_privileges: true,
            include_ownership: true,
            selected_schemas: Vec::new(),
            exclude_table_data: Vec::new(),
            jobs: 4,
            available_schemas,
            all_tables,
            running: false,
            progress: None,
            output_lines: Vec::new(),
            error: None,
            job_id: None,
            focus_handle: cx.focus_handle(),
        }
    }

    fn select_output_path(&mut self, cx: &mut Context<Self>) {
        let format = self.format;

        cx.spawn(|this, mut cx| async move {
            let date = chrono::Local::now().format("%Y-%m-%d").to_string();
            let default_name = format!("backup_{}", date);

            let path = rfd::AsyncFileDialog::new()
                .set_file_name(&format!("{}.{}", default_name, format.default_extension()))
                .add_filter("Backup Files", &["backup", "dump", "sql", "tar"])
                .save_file()
                .await;

            if let Some(path) = path {
                this.update(&mut cx, |this, cx| {
                    this.output_path = path.path().to_string_lossy().to_string();
                    cx.notify();
                }).ok();
            }
        }).detach();
    }

    fn start_backup(&mut self, cx: &mut Context<Self>) {
        if self.output_path.is_empty() {
            self.error = Some("Please select an output path".to_string());
            cx.notify();
            return;
        }

        self.running = true;
        self.error = None;
        self.output_lines.clear();
        cx.notify();

        let job_id = BackupJobId::new();
        self.job_id = Some(job_id.clone());

        // Build options
        let options = BackupOptions {
            connection_id: self.conn_id.clone(),
            output_path: PathBuf::from(&self.output_path),
            format: self.format,
            compression: self.compression,
            include_schema: self.include_schema,
            include_data: self.include_data,
            include_privileges: self.include_privileges,
            include_ownership: self.include_ownership,
            schemas: if self.selected_schemas.is_empty() {
                None
            } else {
                Some(self.selected_schemas.clone())
            },
            tables: None,
            exclude_tables: Vec::new(),
            exclude_table_data: self.exclude_table_data.clone(),
            jobs: self.jobs,
            lock_wait_timeout: 30,
            no_sync: false,
            encoding: None,
            extra_args: Vec::new(),
        };

        // Get connection string
        let conn_string = cx.global::<ConnectionState>()
            .get_connection_string(&self.conn_id)
            .unwrap_or_default();

        cx.spawn(|this, mut cx| async move {
            // Register job
            cx.update(|cx| {
                let job = BackupJob {
                    id: job_id.clone(),
                    job_type: JobType::Backup,
                    status: JobStatus::Running,
                    start_time: Some(chrono::Utc::now()),
                    end_time: None,
                    output: Vec::new(),
                    errors: Vec::new(),
                    progress: None,
                };
                cx.global::<BackupState>().register_job(job);
            }).ok();

            // Execute backup with progress updates
            let this_clone = this.clone();
            let job_id_clone = job_id.clone();

            let result = BackupService::create_backup(
                &conn_string,
                &options,
                move |progress| {
                    // Update progress in UI
                    let this = this_clone.clone();
                    // Note: In real impl, would use proper async channel
                },
            ).await;

            this.update(&mut cx, |this, cx| {
                this.running = false;

                match result {
                    Ok(job) => {
                        if job.status == JobStatus::Completed {
                            // Add to recent backups
                            if let Ok(info) = BackupService::get_backup_info(
                                std::path::Path::new(&this.output_path)
                            ) {
                                cx.global::<BackupState>().add_recent_backup(info);
                            }
                            cx.emit(BackupDialogEvent::Complete(job));
                        } else {
                            this.error = Some(job.errors.join("\n"));
                        }
                    }
                    Err(e) => {
                        this.error = Some(e.to_string());
                    }
                }

                cx.notify();
            }).ok();
        }).detach();
    }

    fn render_format_options(&self, cx: &Context<Self>) -> impl IntoElement {
        div()
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .mb_2()
                    .child("Format")
            )
            .child(
                div()
                    .grid()
                    .grid_cols_2()
                    .gap_3()
                    .children(BackupFormat::all().iter().map(|format| {
                        let is_selected = self.format == *format;

                        div()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .cursor_pointer()
                            .when(is_selected, |el| {
                                el.border_color(rgb(0x3B82F6))
                                    .bg(rgb(0xEFF6FF))
                            })
                            .when(!is_selected, |el| {
                                el.border_color(rgb(0xE5E7EB))
                                    .hover(|el| el.border_color(rgb(0xD1D5DB)))
                            })
                            .on_click(cx.listener(move |this, _, cx| {
                                this.format = *format;
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .gap_3()
                                    .child(
                                        div()
                                            .w_4()
                                            .h_4()
                                            .mt(px(2.))
                                            .rounded_full()
                                            .border_2()
                                            .when(is_selected, |el| {
                                                el.border_color(rgb(0x3B82F6))
                                                    .bg(rgb(0x3B82F6))
                                            })
                                            .when(!is_selected, |el| {
                                                el.border_color(rgb(0xD1D5DB))
                                            })
                                    )
                                    .child(
                                        div()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_medium()
                                                    .child(format.as_str())
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(0x6B7280))
                                                    .child(format.description())
                                            )
                                    )
                            )
                    }))
            )
    }

    fn render_content_options(&self, cx: &Context<Self>) -> impl IntoElement {
        div()
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .mb_2()
                    .child("Content")
            )
            .child(
                div()
                    .grid()
                    .grid_cols_2()
                    .gap_3()
                    .child(
                        Checkbox::new("include-schema")
                            .label("Schema definitions")
                            .checked(self.include_schema)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.include_schema = checked;
                                cx.notify();
                            }))
                    )
                    .child(
                        Checkbox::new("include-data")
                            .label("Table data")
                            .checked(self.include_data)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.include_data = checked;
                                cx.notify();
                            }))
                    )
                    .child(
                        Checkbox::new("include-privileges")
                            .label("Privileges (GRANT/REVOKE)")
                            .checked(self.include_privileges)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.include_privileges = checked;
                                cx.notify();
                            }))
                    )
                    .child(
                        Checkbox::new("include-ownership")
                            .label("Ownership")
                            .checked(self.include_ownership)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.include_ownership = checked;
                                cx.notify();
                            }))
                    )
            )
    }

    fn render_schema_selection(&self, cx: &Context<Self>) -> impl IntoElement {
        div()
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .mb_2()
                    .child("Schemas")
                    .child(
                        span()
                            .ml_2()
                            .text_xs()
                            .text_color(rgb(0x6B7280))
                            .child("(leave empty for all)")
                    )
            )
            .child(
                div()
                    .max_h(px(120.))
                    .overflow_auto()
                    .border_1()
                    .border_color(rgb(0xE5E7EB))
                    .rounded_md()
                    .p_2()
                    .children(self.available_schemas.iter().map(|schema| {
                        let is_selected = self.selected_schemas.contains(schema);
                        let schema_clone = schema.clone();

                        Checkbox::new(format!("schema-{}", schema))
                            .label(schema)
                            .checked(is_selected)
                            .on_change(cx.listener(move |this, checked: bool, cx| {
                                if checked {
                                    this.selected_schemas.push(schema_clone.clone());
                                } else {
                                    this.selected_schemas.retain(|s| s != &schema_clone);
                                }
                                cx.notify();
                            }))
                    }))
            )
    }

    fn render_progress(&self, cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w_4()
                            .h_4()
                            .rounded_full()
                            .border_2()
                            .border_color(rgb(0x3B82F6))
                            .border_t_color(transparent_black())
                            .animate_spin()
                    )
                    .child(
                        self.progress.as_ref()
                            .map(|p| p.phase.clone())
                            .unwrap_or_else(|| "Starting backup...".to_string())
                    )
            )
            .when(self.progress.as_ref().and_then(|p| p.current_object.as_ref()).is_some(), |el| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x6B7280))
                        .font_family("monospace")
                        .truncate()
                        .child(
                            self.progress.as_ref()
                                .and_then(|p| p.current_object.clone())
                                .unwrap_or_default()
                        )
                )
            })
            .when(!self.output_lines.is_empty(), |el| {
                el.child(
                    div()
                        .max_h(px(120.))
                        .overflow_auto()
                        .bg(rgb(0xF3F4F6))
                        .rounded_md()
                        .p_2()
                        .text_xs()
                        .font_family("monospace")
                        .children(
                            self.output_lines.iter().rev().take(20).rev().map(|line| {
                                div().truncate().child(line.clone())
                            })
                        )
                )
            })
    }
}

impl FocusableView for BackupDialog {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for BackupDialog {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        Modal::new("backup-dialog")
            .title("Backup Database")
            .width(px(600.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_6()
                    // Error display
                    .when(self.error.is_some(), |el| {
                        el.child(
                            div()
                                .p_3()
                                .rounded_md()
                                .bg(rgb(0xFEE2E2))
                                .border_1()
                                .border_color(rgb(0xFCA5A5))
                                .text_sm()
                                .text_color(rgb(0xB91C1C))
                                .child(self.error.clone().unwrap_or_default())
                        )
                    })
                    // Output path
                    .child(
                        div()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .mb_2()
                                    .child("Output")
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(
                                        Input::new("output-path")
                                            .placeholder("Select output path...")
                                            .value(self.output_path.clone())
                                            .readonly(true)
                                            .flex_1()
                                    )
                                    .child(
                                        Button::new("browse")
                                            .label("Browse...")
                                            .on_click(cx.listener(|this, _, cx| {
                                                this.select_output_path(cx);
                                            }))
                                    )
                            )
                    )
                    // Format selection
                    .child(self.render_format_options(cx))
                    // Content options
                    .child(self.render_content_options(cx))
                    // Schema selection
                    .child(self.render_schema_selection(cx))
                    // Advanced options (collapsible)
                    .child(
                        div()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .mb_2()
                                    .cursor_pointer()
                                    .child("Advanced Options")
                            )
                            .child(
                                div()
                                    .grid()
                                    .grid_cols_2()
                                    .gap_4()
                                    .child(
                                        div()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(0x6B7280))
                                                    .mb_1()
                                                    .child("Compression (0-9)")
                                            )
                                            .child(
                                                Input::new("compression")
                                                    .value(self.compression.to_string())
                                                    .on_change(cx.listener(|this, value: String, cx| {
                                                        if let Ok(v) = value.parse::<u8>() {
                                                            if v <= 9 {
                                                                this.compression = v;
                                                                cx.notify();
                                                            }
                                                        }
                                                    }))
                                            )
                                    )
                                    .child(
                                        div()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(0x6B7280))
                                                    .mb_1()
                                                    .child("Parallel Jobs")
                                            )
                                            .child(
                                                Input::new("jobs")
                                                    .value(self.jobs.to_string())
                                                    .disabled(self.format != BackupFormat::Directory)
                                                    .on_change(cx.listener(|this, value: String, cx| {
                                                        if let Ok(v) = value.parse::<u8>() {
                                                            this.jobs = v.max(1).min(32);
                                                            cx.notify();
                                                        }
                                                    }))
                                            )
                                    )
                            )
                    )
                    // Progress
                    .when(self.running, |el| {
                        el.child(self.render_progress(cx))
                    })
            )
            .footer(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel")
                            .label(if self.running { "Cancel" } else { "Close" })
                            .on_click(cx.listener(|this, _, cx| {
                                cx.emit(BackupDialogEvent::Cancel);
                            }))
                    )
                    .child(
                        Button::new("backup")
                            .label(if self.running { "Backing up..." } else { "Create Backup" })
                            .variant_primary()
                            .disabled(self.running || self.output_path.is_empty())
                            .on_click(cx.listener(|this, _, cx| {
                                this.start_backup(cx);
                            }))
                    )
            )
    }
}

/// Events emitted by backup dialog
pub enum BackupDialogEvent {
    Complete(BackupJob),
    Cancel,
}

impl EventEmitter<BackupDialogEvent> for BackupDialog {}
```

### 25.5 Restore Dialog Component

```rust
// src/backup/restore_dialog.rs

use crate::backup::types::*;
use crate::backup::service::BackupService;
use crate::backup::state::BackupState;
use crate::connection::ConnectionState;
use crate::ui::{Button, Modal, Checkbox, Input};
use gpui::*;
use std::path::PathBuf;

/// Restore dialog
pub struct RestoreDialog {
    conn_id: String,

    // Input
    input_path: String,
    backup_info: Option<BackupInfo>,

    // Options
    schema_only: bool,
    data_only: bool,
    clean: bool,
    if_exists: bool,
    no_owner: bool,
    no_privileges: bool,
    exit_on_error: bool,
    single_transaction: bool,
    jobs: u8,
    disable_triggers: bool,

    // State
    analyzing: bool,
    running: bool,
    progress: Option<BackupProgress>,
    output_lines: Vec<String>,
    error: Option<String>,

    focus_handle: FocusHandle,
}

impl RestoreDialog {
    pub fn new(conn_id: String, cx: &mut Context<Self>) -> Self {
        Self {
            conn_id,
            input_path: String::new(),
            backup_info: None,
            schema_only: false,
            data_only: false,
            clean: false,
            if_exists: true,
            no_owner: false,
            no_privileges: false,
            exit_on_error: true,
            single_transaction: true,
            jobs: 4,
            disable_triggers: false,
            analyzing: false,
            running: false,
            progress: None,
            output_lines: Vec::new(),
            error: None,
            focus_handle: cx.focus_handle(),
        }
    }

    fn select_input_file(&mut self, cx: &mut Context<Self>) {
        cx.spawn(|this, mut cx| async move {
            let path = rfd::AsyncFileDialog::new()
                .add_filter("Backup Files", &["backup", "dump", "sql", "tar"])
                .add_filter("All Files", &["*"])
                .pick_file()
                .await;

            if let Some(path) = path {
                let path_str = path.path().to_string_lossy().to_string();

                this.update(&mut cx, |this, cx| {
                    this.input_path = path_str.clone();
                    this.analyzing = true;
                    this.error = None;
                    cx.notify();
                }).ok();

                // Analyze backup file
                match BackupService::get_backup_info(std::path::Path::new(&path_str)) {
                    Ok(info) => {
                        this.update(&mut cx, |this, cx| {
                            this.backup_info = Some(info);
                            this.analyzing = false;
                            cx.notify();
                        }).ok();
                    }
                    Err(e) => {
                        this.update(&mut cx, |this, cx| {
                            this.error = Some(e.to_string());
                            this.analyzing = false;
                            cx.notify();
                        }).ok();
                    }
                }
            }
        }).detach();
    }

    fn start_restore(&mut self, cx: &mut Context<Self>) {
        if self.input_path.is_empty() {
            self.error = Some("Please select a backup file".to_string());
            cx.notify();
            return;
        }

        self.running = true;
        self.error = None;
        self.output_lines.clear();
        cx.notify();

        let options = RestoreOptions {
            connection_id: self.conn_id.clone(),
            input_path: PathBuf::from(&self.input_path),
            target_database: None,
            create_database: false,
            schema_only: self.schema_only,
            data_only: self.data_only,
            schemas: None,
            tables: None,
            clean: self.clean,
            if_exists: self.if_exists,
            no_owner: self.no_owner,
            no_privileges: self.no_privileges,
            exit_on_error: self.exit_on_error,
            single_transaction: self.single_transaction,
            jobs: self.jobs,
            disable_triggers: self.disable_triggers,
            extra_args: Vec::new(),
        };

        let conn_string = cx.global::<ConnectionState>()
            .get_connection_string(&self.conn_id)
            .unwrap_or_default();

        cx.spawn(|this, mut cx| async move {
            let result = BackupService::restore_backup(
                &conn_string,
                &options,
                |_progress| {
                    // Update progress
                },
            ).await;

            this.update(&mut cx, |this, cx| {
                this.running = false;

                match result {
                    Ok(job) => {
                        if job.status == JobStatus::Completed {
                            cx.emit(RestoreDialogEvent::Complete(job));
                        } else {
                            this.error = Some(job.errors.join("\n"));
                        }
                    }
                    Err(e) => {
                        this.error = Some(e.to_string());
                    }
                }

                cx.notify();
            }).ok();
        }).detach();
    }

    fn render_backup_info(&self, info: &BackupInfo, cx: &Context<Self>) -> impl IntoElement {
        div()
            .p_4()
            .rounded_md()
            .bg(rgb(0xF9FAFB))
            .child(
                div()
                    .grid()
                    .grid_cols_3()
                    .gap_4()
                    .text_sm()
                    .child(
                        div()
                            .child(
                                span()
                                    .text_color(rgb(0x6B7280))
                                    .child("Format: ")
                            )
                            .child(
                                span()
                                    .font_medium()
                                    .child(info.format.as_str())
                            )
                    )
                    .child(
                        div()
                            .child(
                                span()
                                    .text_color(rgb(0x6B7280))
                                    .child("Size: ")
                            )
                            .child(
                                span()
                                    .font_medium()
                                    .child(format_size(info.size_bytes))
                            )
                    )
                    .child(
                        div()
                            .child(
                                span()
                                    .text_color(rgb(0x6B7280))
                                    .child("Database: ")
                            )
                            .child(
                                span()
                                    .font_medium()
                                    .child(if info.database.is_empty() {
                                        "Unknown"
                                    } else {
                                        &info.database
                                    })
                            )
                    )
            )
            .when(!info.contents.tables.is_empty(), |el| {
                el.child(
                    div()
                        .mt_2()
                        .text_sm()
                        .child(
                            span()
                                .text_color(rgb(0x6B7280))
                                .child("Contents: ")
                        )
                        .child(
                            span()
                                .child(format!(
                                    "{} schemas, {} tables, {} functions",
                                    info.contents.schemas.len(),
                                    info.contents.tables.len(),
                                    info.contents.functions
                                ))
                        )
                )
            })
    }

    fn render_options(&self, cx: &Context<Self>) -> impl IntoElement {
        div()
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .mb_2()
                    .child("Restore Options")
            )
            .child(
                div()
                    .grid()
                    .grid_cols_2()
                    .gap_3()
                    .child(
                        Checkbox::new("schema-only")
                            .label("Schema only (no data)")
                            .checked(self.schema_only)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.schema_only = checked;
                                if checked { this.data_only = false; }
                                cx.notify();
                            }))
                    )
                    .child(
                        Checkbox::new("data-only")
                            .label("Data only (no schema)")
                            .checked(self.data_only)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.data_only = checked;
                                if checked { this.schema_only = false; }
                                cx.notify();
                            }))
                    )
                    .child(
                        Checkbox::new("clean")
                            .label("Clean (drop objects first)")
                            .checked(self.clean)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.clean = checked;
                                cx.notify();
                            }))
                    )
                    .child(
                        Checkbox::new("no-owner")
                            .label("Skip ownership")
                            .checked(self.no_owner)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.no_owner = checked;
                                cx.notify();
                            }))
                    )
                    .child(
                        Checkbox::new("no-privileges")
                            .label("Skip privileges")
                            .checked(self.no_privileges)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.no_privileges = checked;
                                cx.notify();
                            }))
                    )
                    .child(
                        Checkbox::new("exit-on-error")
                            .label("Exit on error")
                            .checked(self.exit_on_error)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.exit_on_error = checked;
                                cx.notify();
                            }))
                    )
                    .child(
                        Checkbox::new("single-transaction")
                            .label("Single transaction")
                            .checked(self.single_transaction)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.single_transaction = checked;
                                cx.notify();
                            }))
                    )
                    .child(
                        Checkbox::new("disable-triggers")
                            .label("Disable triggers")
                            .checked(self.disable_triggers)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.disable_triggers = checked;
                                cx.notify();
                            }))
                    )
            )
    }
}

impl FocusableView for RestoreDialog {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for RestoreDialog {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        Modal::new("restore-dialog")
            .title("Restore Database")
            .width(px(600.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_6()
                    // Error display
                    .when(self.error.is_some(), |el| {
                        el.child(
                            div()
                                .p_3()
                                .rounded_md()
                                .bg(rgb(0xFEE2E2))
                                .border_1()
                                .border_color(rgb(0xFCA5A5))
                                .text_sm()
                                .text_color(rgb(0xB91C1C))
                                .child(self.error.clone().unwrap_or_default())
                        )
                    })
                    // Input file selection
                    .child(
                        div()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .mb_2()
                                    .child("Source")
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(
                                        Input::new("input-path")
                                            .placeholder("Select backup file...")
                                            .value(self.input_path.clone())
                                            .readonly(true)
                                            .flex_1()
                                    )
                                    .child(
                                        Button::new("browse")
                                            .label("Browse...")
                                            .on_click(cx.listener(|this, _, cx| {
                                                this.select_input_file(cx);
                                            }))
                                    )
                            )
                    )
                    // Backup info
                    .when(self.analyzing, |el| {
                        el.child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x6B7280))
                                .child("Analyzing backup file...")
                        )
                    })
                    .when_some(self.backup_info.clone(), |el, info| {
                        el.child(self.render_backup_info(&info, cx))
                    })
                    // Options
                    .child(self.render_options(cx))
                    // Parallel jobs
                    .child(
                        div()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .mb_2()
                                    .child("Parallel Jobs")
                            )
                            .child(
                                Input::new("jobs")
                                    .value(self.jobs.to_string())
                                    .w(px(100.))
                                    .on_change(cx.listener(|this, value: String, cx| {
                                        if let Ok(v) = value.parse::<u8>() {
                                            this.jobs = v.max(1).min(32);
                                            cx.notify();
                                        }
                                    }))
                            )
                    )
                    // Warning for clean mode
                    .when(self.clean, |el| {
                        el.child(
                            div()
                                .p_3()
                                .rounded_md()
                                .bg(rgb(0xFEF3C7))
                                .border_1()
                                .border_color(rgb(0xFCD34D))
                                .text_sm()
                                .text_color(rgb(0x92400E))
                                .child("Warning: Clean mode will drop existing objects before restoring. This is a destructive operation.")
                        )
                    })
                    // Progress
                    .when(self.running, |el| {
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .w_4()
                                        .h_4()
                                        .rounded_full()
                                        .border_2()
                                        .border_color(rgb(0x3B82F6))
                                        .border_t_color(transparent_black())
                                        .animate_spin()
                                )
                                .child("Restoring...")
                        )
                    })
            )
            .footer(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel")
                            .label(if self.running { "Cancel" } else { "Close" })
                            .on_click(cx.listener(|this, _, cx| {
                                cx.emit(RestoreDialogEvent::Cancel);
                            }))
                    )
                    .child(
                        Button::new("restore")
                            .label(if self.running { "Restoring..." } else { "Restore" })
                            .variant_primary()
                            .disabled(self.running || self.input_path.is_empty())
                            .on_click(cx.listener(|this, _, cx| {
                                this.start_restore(cx);
                            }))
                    )
            )
    }
}

/// Events emitted by restore dialog
pub enum RestoreDialogEvent {
    Complete(BackupJob),
    Cancel,
}

impl EventEmitter<RestoreDialogEvent> for RestoreDialog {}

/// Format file size for display
fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
```

### 25.6 Backup Panel Integration

```rust
// src/backup/panel.rs

use crate::backup::dialog::{BackupDialog, BackupDialogEvent};
use crate::backup::restore_dialog::{RestoreDialog, RestoreDialogEvent};
use crate::backup::state::BackupState;
use crate::backup::types::*;
use crate::ui::Button;
use gpui::*;

/// Panel for backup/restore operations
pub struct BackupPanel {
    conn_id: Option<String>,
    backup_dialog: Option<Entity<BackupDialog>>,
    restore_dialog: Option<Entity<RestoreDialog>>,
}

impl BackupPanel {
    pub fn new() -> Self {
        Self {
            conn_id: None,
            backup_dialog: None,
            restore_dialog: None,
        }
    }

    pub fn set_connection(&mut self, conn_id: String) {
        self.conn_id = Some(conn_id);
    }

    fn open_backup_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(conn_id) = self.conn_id.clone() else { return };

        let dialog = cx.new(|cx| BackupDialog::new(conn_id, cx));

        cx.subscribe(&dialog, |this, _, event: &BackupDialogEvent, cx| {
            match event {
                BackupDialogEvent::Complete(_) | BackupDialogEvent::Cancel => {
                    this.backup_dialog = None;
                    cx.notify();
                }
            }
        }).detach();

        self.backup_dialog = Some(dialog);
        cx.notify();
    }

    fn open_restore_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(conn_id) = self.conn_id.clone() else { return };

        let dialog = cx.new(|cx| RestoreDialog::new(conn_id, cx));

        cx.subscribe(&dialog, |this, _, event: &RestoreDialogEvent, cx| {
            match event {
                RestoreDialogEvent::Complete(_) | RestoreDialogEvent::Cancel => {
                    this.restore_dialog = None;
                    cx.notify();
                }
            }
        }).detach();

        self.restore_dialog = Some(dialog);
        cx.notify();
    }
}

impl Render for BackupPanel {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let running_jobs = cx.global::<BackupState>().running_jobs();
        let recent_backups = cx.global::<BackupState>().recent_backups();

        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            // Action buttons
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("backup")
                            .label("Backup...")
                            .icon("download")
                            .disabled(self.conn_id.is_none())
                            .on_click(cx.listener(|this, _, cx| {
                                this.open_backup_dialog(cx);
                            }))
                    )
                    .child(
                        Button::new("restore")
                            .label("Restore...")
                            .icon("upload")
                            .disabled(self.conn_id.is_none())
                            .on_click(cx.listener(|this, _, cx| {
                                this.open_restore_dialog(cx);
                            }))
                    )
            )
            // Running jobs
            .when(!running_jobs.is_empty(), |el| {
                el.child(
                    div()
                        .child(
                            div()
                                .text_sm()
                                .font_medium()
                                .mb_2()
                                .child("Running Jobs")
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .children(running_jobs.iter().map(|job| {
                                    div()
                                        .p_3()
                                        .rounded_md()
                                        .bg(rgb(0xEFF6FF))
                                        .border_1()
                                        .border_color(rgb(0xBFDBFE))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .w_3()
                                                        .h_3()
                                                        .rounded_full()
                                                        .border_2()
                                                        .border_color(rgb(0x3B82F6))
                                                        .border_t_color(transparent_black())
                                                        .animate_spin()
                                                )
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .child(format!(
                                                            "{} in progress...",
                                                            if job.job_type == JobType::Backup {
                                                                "Backup"
                                                            } else {
                                                                "Restore"
                                                            }
                                                        ))
                                                )
                                        )
                                        .when(job.progress.as_ref().and_then(|p| p.current_object.as_ref()).is_some(), |el| {
                                            el.child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(0x6B7280))
                                                    .mt_1()
                                                    .truncate()
                                                    .child(
                                                        job.progress.as_ref()
                                                            .and_then(|p| p.current_object.clone())
                                                            .unwrap_or_default()
                                                    )
                                            )
                                        })
                                }))
                        )
                )
            })
            // Recent backups
            .when(!recent_backups.is_empty(), |el| {
                el.child(
                    div()
                        .child(
                            div()
                                .text_sm()
                                .font_medium()
                                .mb_2()
                                .child("Recent Backups")
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .children(recent_backups.iter().take(5).map(|backup| {
                                    div()
                                        .p_3()
                                        .rounded_md()
                                        .bg(rgb(0xF9FAFB))
                                        .child(
                                            div()
                                                .flex()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .truncate()
                                                        .max_w(px(200.))
                                                        .child(
                                                            backup.path.file_name()
                                                                .map(|n| n.to_string_lossy().to_string())
                                                                .unwrap_or_else(|| backup.path.to_string_lossy().to_string())
                                                        )
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(0x6B7280))
                                                        .child(format_size(backup.size_bytes))
                                                )
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0x6B7280))
                                                .mt_1()
                                                .child(backup.format.as_str())
                                        )
                                }))
                        )
                )
            })
            // Render dialogs
            .when_some(self.backup_dialog.clone(), |el, dialog| {
                el.child(dialog)
            })
            .when_some(self.restore_dialog.clone(), |el, dialog| {
                el.child(dialog)
            })
    }
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
```

## Acceptance Criteria

1. **Backup Creation**
   - [ ] Support all pg_dump formats (custom, plain, directory, tar)
   - [ ] Configure compression level (0-9)
   - [ ] Select specific schemas for backup
   - [ ] Exclude specific tables from data backup
   - [ ] Configure parallel jobs for directory format
   - [ ] Show real-time progress with current object

2. **Restore**
   - [ ] Auto-detect backup format from file
   - [ ] Show backup file info before restore
   - [ ] Support schema-only and data-only restore
   - [ ] Clean mode with appropriate warnings
   - [ ] Single transaction option for atomic restore
   - [ ] Parallel restore support

3. **Progress and Output**
   - [ ] Real-time progress display with phase indicator
   - [ ] Capture and display pg_dump/pg_restore output
   - [ ] Show errors clearly with context
   - [ ] Support job cancellation

4. **State Management**
   - [ ] Track running jobs with progress
   - [ ] Maintain recent backups list
   - [ ] Persist backup history across sessions

## Testing Instructions

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_backup_format() {
        // Test custom format detection
        let temp = TempDir::new().unwrap();
        let custom_path = temp.path().join("test.backup");
        std::fs::write(&custom_path, b"PGDMP...").unwrap();

        assert!(matches!(
            BackupService::detect_backup_format(&custom_path),
            Ok(BackupFormat::Custom)
        ));

        // Test plain SQL detection
        let sql_path = temp.path().join("test.sql");
        std::fs::write(&sql_path, b"-- PostgreSQL dump").unwrap();

        assert!(matches!(
            BackupService::detect_backup_format(&sql_path),
            Ok(BackupFormat::Plain)
        ));

        // Test directory detection
        let dir_path = temp.path().join("backup_dir");
        std::fs::create_dir(&dir_path).unwrap();

        assert!(matches!(
            BackupService::detect_backup_format(&dir_path),
            Ok(BackupFormat::Directory)
        ));
    }

    #[test]
    fn test_backup_options_defaults() {
        let opts = BackupOptions::default();

        assert_eq!(opts.format, BackupFormat::Custom);
        assert_eq!(opts.compression, 6);
        assert!(opts.include_schema);
        assert!(opts.include_data);
        assert_eq!(opts.jobs, 4);
    }

    #[test]
    fn test_restore_options_defaults() {
        let opts = RestoreOptions::default();

        assert!(!opts.schema_only);
        assert!(!opts.data_only);
        assert!(!opts.clean);
        assert!(opts.if_exists);
        assert!(opts.exit_on_error);
        assert!(opts.single_transaction);
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_backup_restore_cycle() {
        // This test requires a running PostgreSQL instance
        // Skip if not available
        let conn_string = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/test".to_string());

        let temp = tempfile::TempDir::new().unwrap();
        let backup_path = temp.path().join("test.backup");

        // Create backup
        let backup_opts = BackupOptions {
            connection_id: "test".to_string(),
            output_path: backup_path.clone(),
            format: BackupFormat::Custom,
            compression: 6,
            include_schema: true,
            include_data: true,
            ..Default::default()
        };

        let job = BackupService::create_backup(
            &conn_string,
            &backup_opts,
            |_| {},
        ).await.unwrap();

        assert_eq!(job.status, JobStatus::Completed);
        assert!(backup_path.exists());

        // Verify backup info
        let info = BackupService::get_backup_info(&backup_path).unwrap();
        assert_eq!(info.format, BackupFormat::Custom);
    }
}
```

### Manual Testing

1. **Backup Flow**:
   - Connect to a test database
   - Open backup dialog
   - Select custom format with compression level 6
   - Select specific schemas
   - Create backup and verify file is created
   - Check progress updates during backup

2. **Restore Flow**:
   - Select a backup file
   - Verify backup info is displayed correctly
   - Enable "Clean" mode and verify warning
   - Execute restore and verify data

3. **Error Handling**:
   - Attempt backup with invalid path
   - Attempt restore of corrupted file
   - Cancel mid-backup and verify cleanup

4. **Large Database**:
   - Backup database with 100+ tables
   - Use parallel jobs for directory format
   - Monitor memory usage during operation
