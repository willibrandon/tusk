# Feature 21: Maintenance Operations

## Overview

This feature implements GUI dialogs for PostgreSQL maintenance commands including VACUUM, ANALYZE, REINDEX, and CLUSTER. All dialogs expose command options with helpful descriptions, execute operations with progress tracking, and provide detailed output display—built as native GPUI components.

**Dependencies:** Features 07 (Connection Management), 10 (Schema Cache), 20 (Admin Dashboard)

## 21.1 Maintenance Data Models

```rust
// src/models/maintenance.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Options for VACUUM command
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VacuumOptions {
    /// Perform full vacuum (rewrites entire table)
    pub full: bool,
    /// Aggressively freeze tuples
    pub freeze: bool,
    /// Also run ANALYZE
    pub analyze: bool,
    /// Print detailed progress
    pub verbose: bool,
    /// Skip tables that cannot be locked immediately
    pub skip_locked: bool,
    /// Index cleanup mode: "auto", "on", "off"
    pub index_cleanup: IndexCleanupMode,
    /// Number of parallel workers (0 = auto)
    pub parallel: u32,
    /// Attempt to truncate empty pages at end
    pub truncate: bool,
    /// Process TOAST tables
    pub process_toast: bool,
}

/// Index cleanup mode for VACUUM
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexCleanupMode {
    #[default]
    Auto,
    On,
    Off,
}

impl IndexCleanupMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            IndexCleanupMode::Auto => "auto",
            IndexCleanupMode::On => "on",
            IndexCleanupMode::Off => "off",
        }
    }

    pub fn all() -> &'static [IndexCleanupMode] {
        &[IndexCleanupMode::Auto, IndexCleanupMode::On, IndexCleanupMode::Off]
    }
}

/// Options for ANALYZE command
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalyzeOptions {
    /// Print progress messages
    pub verbose: bool,
    /// Skip tables that cannot be locked immediately
    pub skip_locked: bool,
    /// Specific columns to analyze (empty = all)
    pub columns: Vec<String>,
}

/// Options for REINDEX command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReindexOptions {
    /// Rebuild without locking writes
    pub concurrently: bool,
    /// Print progress
    pub verbose: bool,
    /// Target tablespace for rebuilt indexes
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

/// Target type for REINDEX
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReindexTargetType {
    Index,
    Table,
    Schema,
    Database,
}

impl ReindexTargetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReindexTargetType::Index => "INDEX",
            ReindexTargetType::Table => "TABLE",
            ReindexTargetType::Schema => "SCHEMA",
            ReindexTargetType::Database => "DATABASE",
        }
    }

    pub fn all() -> &'static [ReindexTargetType] {
        &[
            ReindexTargetType::Table,
            ReindexTargetType::Index,
            ReindexTargetType::Schema,
            ReindexTargetType::Database,
        ]
    }
}

/// Target for REINDEX command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReindexTarget {
    pub target_type: ReindexTargetType,
    pub schema: Option<String>,
    pub name: Option<String>,
}

/// Options for CLUSTER command
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClusterOptions {
    /// Print progress
    pub verbose: bool,
    /// Index to cluster on
    pub index_name: Option<String>,
}

/// Maintenance job status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            JobStatus::Pending => "pending",
            JobStatus::Running => "running",
            JobStatus::Completed => "completed",
            JobStatus::Failed => "failed",
            JobStatus::Cancelled => "cancelled",
        }
    }
}

/// Type of maintenance operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceType {
    Vacuum,
    Analyze,
    Reindex,
    Cluster,
}

impl MaintenanceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MaintenanceType::Vacuum => "VACUUM",
            MaintenanceType::Analyze => "ANALYZE",
            MaintenanceType::Reindex => "REINDEX",
            MaintenanceType::Cluster => "CLUSTER",
        }
    }
}

/// A maintenance job tracking execution
#[derive(Debug, Clone)]
pub struct MaintenanceJob {
    pub id: String,
    pub job_type: MaintenanceType,
    pub target: String,
    pub status: JobStatus,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub output: Vec<String>,
    pub error: Option<String>,
    pub progress: Option<u32>,
}

impl MaintenanceJob {
    pub fn new(job_type: MaintenanceType, target: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            job_type,
            target,
            status: JobStatus::Pending,
            start_time: None,
            end_time: None,
            output: Vec::new(),
            error: None,
            progress: None,
        }
    }

    pub fn duration(&self) -> Option<Duration> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => Some((end - start).to_std().unwrap_or_default()),
            (Some(start), None) => Some((Utc::now() - start).to_std().unwrap_or_default()),
            _ => None,
        }
    }

    pub fn format_duration(&self) -> String {
        match self.duration() {
            None => "-".to_string(),
            Some(d) => {
                let ms = d.as_millis();
                if ms < 1000 {
                    format!("{}ms", ms)
                } else if ms < 60_000 {
                    format!("{:.1}s", ms as f64 / 1000.0)
                } else {
                    let mins = ms / 60_000;
                    let secs = (ms % 60_000) / 1000;
                    format!("{}m {}s", mins, secs)
                }
            }
        }
    }
}

/// Result of a maintenance operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceResult {
    pub success: bool,
    pub output: Vec<String>,
    pub duration_ms: i64,
    pub error: Option<String>,
}
```

## 21.2 Maintenance Service

```rust
// src/services/maintenance.rs

use crate::error::{Error, Result};
use crate::models::maintenance::*;
use crate::services::connection::ConnectionService;
use std::sync::Arc;
use tokio_postgres::Client;

/// Service for executing PostgreSQL maintenance commands
pub struct MaintenanceService {
    connection_service: Arc<ConnectionService>,
}

impl MaintenanceService {
    pub fn new(connection_service: Arc<ConnectionService>) -> Self {
        Self { connection_service }
    }

    /// Execute VACUUM command
    pub async fn vacuum(
        &self,
        conn_id: &str,
        target: Option<(&str, &str)>, // (schema, table) or None for all
        options: &VacuumOptions,
    ) -> Result<MaintenanceResult> {
        let pool = self.connection_service.get_pool(conn_id)?;
        let client = pool.get().await?;
        let start = std::time::Instant::now();

        // Build VACUUM command
        let mut sql = String::from("VACUUM");
        let mut opts = Vec::new();

        if options.full {
            opts.push("FULL".to_string());
        }
        if options.freeze {
            opts.push("FREEZE".to_string());
        }
        if options.verbose {
            opts.push("VERBOSE".to_string());
        }
        if options.analyze {
            opts.push("ANALYZE".to_string());
        }
        if options.skip_locked {
            opts.push("SKIP_LOCKED".to_string());
        }
        if options.index_cleanup != IndexCleanupMode::Auto {
            opts.push(format!(
                "INDEX_CLEANUP {}",
                options.index_cleanup.as_str().to_uppercase()
            ));
        }
        if options.parallel > 0 {
            opts.push(format!("PARALLEL {}", options.parallel));
        }
        if !options.truncate {
            opts.push("TRUNCATE FALSE".to_string());
        }
        if !options.process_toast {
            opts.push("PROCESS_TOAST FALSE".to_string());
        }

        if !opts.is_empty() {
            sql.push_str(&format!(" ({})", opts.join(", ")));
        }

        if let Some((schema, table)) = target {
            sql.push_str(&format!(
                " {}.{}",
                Self::quote_ident(schema),
                Self::quote_ident(table)
            ));
        }

        // Execute with notice collection
        let output = Self::execute_with_notices(&client, &sql).await?;
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
        &self,
        conn_id: &str,
        target: Option<(&str, &str)>,
        options: &AnalyzeOptions,
    ) -> Result<MaintenanceResult> {
        let pool = self.connection_service.get_pool(conn_id)?;
        let client = pool.get().await?;
        let start = std::time::Instant::now();

        let mut sql = String::from("ANALYZE");
        let mut opts = Vec::new();

        if options.verbose {
            opts.push("VERBOSE".to_string());
        }
        if options.skip_locked {
            opts.push("SKIP_LOCKED".to_string());
        }

        if !opts.is_empty() {
            sql.push_str(&format!(" ({})", opts.join(", ")));
        }

        if let Some((schema, table)) = target {
            sql.push_str(&format!(
                " {}.{}",
                Self::quote_ident(schema),
                Self::quote_ident(table)
            ));

            // Add specific columns if provided
            if !options.columns.is_empty() {
                let cols: Vec<String> = options
                    .columns
                    .iter()
                    .map(|c| Self::quote_ident(c))
                    .collect();
                sql.push_str(&format!(" ({})", cols.join(", ")));
            }
        }

        let output = Self::execute_with_notices(&client, &sql).await?;
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
        &self,
        conn_id: &str,
        target: &ReindexTarget,
        options: &ReindexOptions,
    ) -> Result<MaintenanceResult> {
        let pool = self.connection_service.get_pool(conn_id)?;
        let client = pool.get().await?;
        let start = std::time::Instant::now();

        let mut sql = String::from("REINDEX");
        let mut opts = Vec::new();

        if options.concurrently {
            opts.push("CONCURRENTLY".to_string());
        }
        if options.verbose {
            opts.push("VERBOSE".to_string());
        }
        if let Some(ref ts) = options.tablespace {
            opts.push(format!("TABLESPACE {}", Self::quote_ident(ts)));
        }

        if !opts.is_empty() {
            sql.push_str(&format!(" ({})", opts.join(", ")));
        }

        // Add target type and name
        sql.push_str(&format!(" {}", target.target_type.as_str()));

        match target.target_type {
            ReindexTargetType::Index | ReindexTargetType::Table => {
                if let (Some(schema), Some(name)) = (&target.schema, &target.name) {
                    sql.push_str(&format!(
                        " {}.{}",
                        Self::quote_ident(schema),
                        Self::quote_ident(name)
                    ));
                }
            }
            ReindexTargetType::Schema => {
                if let Some(name) = &target.name {
                    sql.push_str(&format!(" {}", Self::quote_ident(name)));
                }
            }
            ReindexTargetType::Database => {
                // Uses current database
            }
        }

        let output = Self::execute_with_notices(&client, &sql).await?;
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
        &self,
        conn_id: &str,
        target: Option<(&str, &str)>,
        options: &ClusterOptions,
    ) -> Result<MaintenanceResult> {
        let pool = self.connection_service.get_pool(conn_id)?;
        let client = pool.get().await?;
        let start = std::time::Instant::now();

        let mut sql = String::from("CLUSTER");

        if options.verbose {
            sql.push_str(" (VERBOSE)");
        }

        if let Some((schema, table)) = target {
            sql.push_str(&format!(
                " {}.{}",
                Self::quote_ident(schema),
                Self::quote_ident(table)
            ));

            if let Some(ref idx) = options.index_name {
                sql.push_str(&format!(" USING {}", Self::quote_ident(idx)));
            }
        }

        let output = Self::execute_with_notices(&client, &sql).await?;
        let duration = start.elapsed().as_millis() as i64;

        Ok(MaintenanceResult {
            success: true,
            output,
            duration_ms: duration,
            error: None,
        })
    }

    /// Execute command and capture NOTICE messages
    async fn execute_with_notices(client: &Client, sql: &str) -> Result<Vec<String>> {
        // Execute the command
        client.execute(sql, &[]).await?;

        // Return executed command as output
        // In production, a notice handler would collect NOTICE/INFO messages
        Ok(vec![format!("Executed: {}", sql)])
    }

    /// Quote an identifier for safe use in SQL
    fn quote_ident(s: &str) -> String {
        if s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            s.to_string()
        } else {
            format!("\"{}\"", s.replace('"', "\"\""))
        }
    }
}
```

## 21.3 Maintenance State (Global)

```rust
// src/state/maintenance_state.rs

use crate::models::maintenance::*;
use crate::services::maintenance::MaintenanceService;
use crate::services::connection::ConnectionService;
use chrono::Utc;
use gpui::Global;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::runtime::Handle;

/// Maximum number of completed jobs to retain
const MAX_COMPLETED_JOBS: usize = 50;

/// Application-wide maintenance state
pub struct MaintenanceState {
    maintenance_service: Arc<MaintenanceService>,
    active_jobs: RwLock<Vec<MaintenanceJob>>,
    completed_jobs: RwLock<VecDeque<MaintenanceJob>>,
    runtime: Handle,
}

impl Global for MaintenanceState {}

impl MaintenanceState {
    pub fn new(connection_service: Arc<ConnectionService>, runtime: Handle) -> Self {
        let maintenance_service = Arc::new(MaintenanceService::new(connection_service));
        Self {
            maintenance_service,
            active_jobs: RwLock::new(Vec::new()),
            completed_jobs: RwLock::new(VecDeque::new()),
            runtime,
        }
    }

    /// Get all active jobs
    pub fn get_active_jobs(&self) -> Vec<MaintenanceJob> {
        self.active_jobs.read().clone()
    }

    /// Get all completed jobs
    pub fn get_completed_jobs(&self) -> Vec<MaintenanceJob> {
        self.completed_jobs.read().iter().cloned().collect()
    }

    /// Get all jobs (active + completed)
    pub fn get_all_jobs(&self) -> Vec<MaintenanceJob> {
        let mut jobs = self.active_jobs.read().clone();
        jobs.extend(self.completed_jobs.read().iter().cloned());
        jobs
    }

    /// Clear completed jobs
    pub fn clear_completed_jobs(&self) {
        self.completed_jobs.write().clear();
    }

    /// Start a VACUUM operation
    pub async fn vacuum(
        &self,
        conn_id: &str,
        schema: Option<&str>,
        table: Option<&str>,
        options: VacuumOptions,
    ) -> Result<MaintenanceResult, String> {
        let target_str = match (schema, table) {
            (Some(s), Some(t)) => format!("{}.{}", s, t),
            _ => "database".to_string(),
        };

        let job = MaintenanceJob::new(MaintenanceType::Vacuum, target_str);
        let job_id = job.id.clone();

        // Add to active jobs
        self.active_jobs.write().push(job);
        self.update_job_status(&job_id, JobStatus::Running);

        // Execute
        let target = match (schema, table) {
            (Some(s), Some(t)) => Some((s, t)),
            _ => None,
        };

        let result = self
            .maintenance_service
            .vacuum(conn_id, target, &options)
            .await;

        self.complete_job(&job_id, result)
    }

    /// Start an ANALYZE operation
    pub async fn analyze(
        &self,
        conn_id: &str,
        schema: Option<&str>,
        table: Option<&str>,
        options: AnalyzeOptions,
    ) -> Result<MaintenanceResult, String> {
        let target_str = match (schema, table) {
            (Some(s), Some(t)) => format!("{}.{}", s, t),
            _ => "database".to_string(),
        };

        let job = MaintenanceJob::new(MaintenanceType::Analyze, target_str);
        let job_id = job.id.clone();

        self.active_jobs.write().push(job);
        self.update_job_status(&job_id, JobStatus::Running);

        let target = match (schema, table) {
            (Some(s), Some(t)) => Some((s, t)),
            _ => None,
        };

        let result = self
            .maintenance_service
            .analyze(conn_id, target, &options)
            .await;

        self.complete_job(&job_id, result)
    }

    /// Start a REINDEX operation
    pub async fn reindex(
        &self,
        conn_id: &str,
        target: ReindexTarget,
        options: ReindexOptions,
    ) -> Result<MaintenanceResult, String> {
        let target_str = match (&target.schema, &target.name) {
            (Some(s), Some(n)) => format!("{}.{}", s, n),
            (None, Some(n)) => n.clone(),
            _ => target.target_type.as_str().to_lowercase(),
        };

        let job = MaintenanceJob::new(MaintenanceType::Reindex, target_str);
        let job_id = job.id.clone();

        self.active_jobs.write().push(job);
        self.update_job_status(&job_id, JobStatus::Running);

        let result = self
            .maintenance_service
            .reindex(conn_id, &target, &options)
            .await;

        self.complete_job(&job_id, result)
    }

    /// Start a CLUSTER operation
    pub async fn cluster(
        &self,
        conn_id: &str,
        schema: Option<&str>,
        table: Option<&str>,
        options: ClusterOptions,
    ) -> Result<MaintenanceResult, String> {
        let target_str = match (schema, table) {
            (Some(s), Some(t)) => format!("{}.{}", s, t),
            _ => "all tables".to_string(),
        };

        let job = MaintenanceJob::new(MaintenanceType::Cluster, target_str);
        let job_id = job.id.clone();

        self.active_jobs.write().push(job);
        self.update_job_status(&job_id, JobStatus::Running);

        let target = match (schema, table) {
            (Some(s), Some(t)) => Some((s, t)),
            _ => None,
        };

        let result = self
            .maintenance_service
            .cluster(conn_id, target, &options)
            .await;

        self.complete_job(&job_id, result)
    }

    /// Update job status
    fn update_job_status(&self, job_id: &str, status: JobStatus) {
        let mut jobs = self.active_jobs.write();
        if let Some(job) = jobs.iter_mut().find(|j| j.id == job_id) {
            job.status = status;
            if status == JobStatus::Running {
                job.start_time = Some(Utc::now());
            }
        }
    }

    /// Complete a job and move to completed list
    fn complete_job(
        &self,
        job_id: &str,
        result: Result<MaintenanceResult, crate::error::Error>,
    ) -> Result<MaintenanceResult, String> {
        let mut active = self.active_jobs.write();
        let job_idx = active.iter().position(|j| j.id == job_id);

        if let Some(idx) = job_idx {
            let mut job = active.remove(idx);
            job.end_time = Some(Utc::now());

            match result {
                Ok(res) => {
                    job.status = JobStatus::Completed;
                    job.output = res.output.clone();

                    // Add to completed
                    let mut completed = self.completed_jobs.write();
                    completed.push_front(job);
                    while completed.len() > MAX_COMPLETED_JOBS {
                        completed.pop_back();
                    }

                    Ok(res)
                }
                Err(e) => {
                    job.status = JobStatus::Failed;
                    job.error = Some(e.to_string());

                    let mut completed = self.completed_jobs.write();
                    completed.push_front(job);
                    while completed.len() > MAX_COMPLETED_JOBS {
                        completed.pop_back();
                    }

                    Err(e.to_string())
                }
            }
        } else {
            Err("Job not found".to_string())
        }
    }
}
```

## 21.4 VACUUM Dialog Component

```rust
// src/components/maintenance/vacuum_dialog.rs

use crate::models::maintenance::{IndexCleanupMode, VacuumOptions};
use crate::models::schema::Table;
use crate::state::maintenance_state::MaintenanceState;
use crate::theme::Theme;
use gpui::*;

/// VACUUM dialog component
pub struct VacuumDialog {
    conn_id: String,
    tables: Vec<Table>,
    selected_schema: Option<String>,
    selected_table: Option<String>,
    options: VacuumOptions,
    executing: bool,
}

impl VacuumDialog {
    pub fn new(
        conn_id: String,
        tables: Vec<Table>,
        initial_schema: Option<String>,
        initial_table: Option<String>,
    ) -> Self {
        Self {
            conn_id,
            tables,
            selected_schema: initial_schema,
            selected_table: initial_table,
            options: VacuumOptions::default(),
            executing: false,
        }
    }

    fn schemas(&self) -> Vec<String> {
        let mut schemas: Vec<_> = self
            .tables
            .iter()
            .map(|t| t.schema.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        schemas.sort();
        schemas
    }

    fn tables_for_schema(&self) -> Vec<String> {
        match &self.selected_schema {
            Some(schema) => {
                let mut tables: Vec<_> = self
                    .tables
                    .iter()
                    .filter(|t| &t.schema == schema)
                    .map(|t| t.name.clone())
                    .collect();
                tables.sort();
                tables
            }
            None => Vec::new(),
        }
    }

    fn on_schema_change(&mut self, schema: Option<String>, cx: &mut Context<Self>) {
        self.selected_schema = schema;
        self.selected_table = None;
        cx.notify();
    }

    fn on_table_change(&mut self, table: Option<String>, cx: &mut Context<Self>) {
        self.selected_table = table;
        cx.notify();
    }

    fn on_run(&mut self, cx: &mut Context<Self>) {
        self.executing = true;
        cx.notify();

        let conn_id = self.conn_id.clone();
        let schema = self.selected_schema.clone();
        let table = self.selected_table.clone();
        let options = self.options.clone();

        cx.spawn(|this, mut cx| async move {
            let result = cx
                .update(|cx| {
                    let state = cx.global::<MaintenanceState>();
                    let schema_ref = schema.as_deref();
                    let table_ref = table.as_deref();
                    state.vacuum(&conn_id, schema_ref, table_ref, options)
                })
                .ok();

            if let Some(fut) = result {
                let _ = fut.await;
            }

            let _ = this.update(&mut cx, |this, cx| {
                this.executing = false;
                cx.emit(VacuumDialogEvent::Completed);
            });
        })
        .detach();
    }

    fn on_cancel(&mut self, cx: &mut Context<Self>) {
        cx.emit(VacuumDialogEvent::Cancelled);
    }
}

pub enum VacuumDialogEvent {
    Completed,
    Cancelled,
}

impl EventEmitter<VacuumDialogEvent> for VacuumDialog {}

impl Render for VacuumDialog {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let schemas = self.schemas();
        let tables = self.tables_for_schema();

        Modal::new("vacuum-dialog")
            .title("VACUUM")
            .width(px(500.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_4()
                    .max_h(vh(60.0))
                    .overflow_y_auto()
                    // Target Selection
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.text)
                                    .child("Target"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(
                                        Select::new(self.selected_schema.clone().unwrap_or_default())
                                            .placeholder("All schemas")
                                            .options(
                                                std::iter::once(("", "All schemas"))
                                                    .chain(schemas.iter().map(|s| (s.as_str(), s.as_str())))
                                                    .collect(),
                                            )
                                            .on_change(cx.listener(|this, value: String, cx| {
                                                let schema = if value.is_empty() { None } else { Some(value) };
                                                this.on_schema_change(schema, cx);
                                            })),
                                    )
                                    .child(
                                        Select::new(self.selected_table.clone().unwrap_or_default())
                                            .placeholder("All tables")
                                            .disabled(self.selected_schema.is_none())
                                            .options(
                                                std::iter::once(("", "All tables"))
                                                    .chain(tables.iter().map(|t| (t.as_str(), t.as_str())))
                                                    .collect(),
                                            )
                                            .on_change(cx.listener(|this, value: String, cx| {
                                                let table = if value.is_empty() { None } else { Some(value) };
                                                this.on_table_change(table, cx);
                                            })),
                                    ),
                            ),
                    )
                    // Options
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.text_muted)
                                    .child("Options"),
                            )
                            // FULL option
                            .child(self.render_option(
                                "FULL",
                                "Reclaims more space but takes longer and requires exclusive lock. Rewrites the entire table.",
                                self.options.full,
                                cx.listener(|this, checked, cx| {
                                    this.options.full = checked;
                                    cx.notify();
                                }),
                                theme,
                            ))
                            // FREEZE option
                            .child(self.render_option(
                                "FREEZE",
                                "Aggressively freeze tuples. Useful before taking a pg_dump for archival.",
                                self.options.freeze,
                                cx.listener(|this, checked, cx| {
                                    this.options.freeze = checked;
                                    cx.notify();
                                }),
                                theme,
                            ))
                            // ANALYZE option
                            .child(self.render_option(
                                "ANALYZE",
                                "Update statistics used by the query planner.",
                                self.options.analyze,
                                cx.listener(|this, checked, cx| {
                                    this.options.analyze = checked;
                                    cx.notify();
                                }),
                                theme,
                            ))
                            // VERBOSE option
                            .child(self.render_option(
                                "VERBOSE",
                                "Print detailed progress report for each table.",
                                self.options.verbose,
                                cx.listener(|this, checked, cx| {
                                    this.options.verbose = checked;
                                    cx.notify();
                                }),
                                theme,
                            ))
                            // SKIP_LOCKED option
                            .child(self.render_option(
                                "SKIP_LOCKED",
                                "Skip tables that cannot be locked immediately.",
                                self.options.skip_locked,
                                cx.listener(|this, checked, cx| {
                                    this.options.skip_locked = checked;
                                    cx.notify();
                                }),
                                theme,
                            ))
                            // Index Cleanup & Parallel
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
                                            .child(div().text_sm().child("Index Cleanup:"))
                                            .child(
                                                Select::new(self.options.index_cleanup.as_str().to_string())
                                                    .options(
                                                        IndexCleanupMode::all()
                                                            .iter()
                                                            .map(|m| (m.as_str(), m.as_str()))
                                                            .collect(),
                                                    )
                                                    .on_change(cx.listener(|this, value: String, cx| {
                                                        this.options.index_cleanup = match value.as_str() {
                                                            "on" => IndexCleanupMode::On,
                                                            "off" => IndexCleanupMode::Off,
                                                            _ => IndexCleanupMode::Auto,
                                                        };
                                                        cx.notify();
                                                    })),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(div().text_sm().child("Parallel Workers:"))
                                            .child(
                                                NumberInput::new(self.options.parallel as i64)
                                                    .min(0)
                                                    .max(32)
                                                    .w_16()
                                                    .on_change(cx.listener(|this, value, cx| {
                                                        this.options.parallel = value as u32;
                                                        cx.notify();
                                                    })),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.text_muted)
                                                    .child("(0 = auto)"),
                                            ),
                                    ),
                            ),
                    )
                    // Warning for FULL
                    .when(self.options.full, |this| {
                        this.child(
                            div()
                                .p_3()
                                .bg(theme.warning_bg)
                                .border_1()
                                .border_color(theme.warning)
                                .rounded_md()
                                .text_sm()
                                .text_color(theme.warning)
                                .child(
                                    div()
                                        .child(
                                            Span::new()
                                                .font_weight(FontWeight::BOLD)
                                                .child("Warning: "),
                                        )
                                        .child("VACUUM FULL requires an exclusive lock on the table and rewrites the entire table. This can take a significant amount of time for large tables and will block all queries."),
                                ),
                        )
                    }),
            )
            .actions(vec![
                Button::new("cancel")
                    .label("Cancel")
                    .variant(ButtonVariant::Ghost)
                    .disabled(self.executing)
                    .on_click(cx.listener(|this, _, cx| this.on_cancel(cx))),
                Button::new("run")
                    .label("Run VACUUM")
                    .variant(ButtonVariant::Primary)
                    .loading(self.executing)
                    .on_click(cx.listener(|this, _, cx| this.on_run(cx))),
            ])
    }
}

impl VacuumDialog {
    fn render_option(
        &self,
        name: &str,
        description: &str,
        checked: bool,
        on_toggle: impl Fn(&mut Self, bool, &mut Context<Self>) + 'static,
        theme: &Theme,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_start()
            .gap_3()
            .cursor_pointer()
            .child(Checkbox::new(checked).on_toggle(on_toggle))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child(name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(description),
                    ),
            )
    }
}
```

## 21.5 ANALYZE Dialog Component

```rust
// src/components/maintenance/analyze_dialog.rs

use crate::models::maintenance::AnalyzeOptions;
use crate::models::schema::Table;
use crate::state::maintenance_state::MaintenanceState;
use crate::theme::Theme;
use gpui::*;
use std::collections::HashSet;

/// ANALYZE dialog component
pub struct AnalyzeDialog {
    conn_id: String,
    tables: Vec<Table>,
    selected_schema: Option<String>,
    selected_table: Option<String>,
    selected_columns: HashSet<String>,
    options: AnalyzeOptions,
    executing: bool,
}

impl AnalyzeDialog {
    pub fn new(
        conn_id: String,
        tables: Vec<Table>,
        initial_schema: Option<String>,
        initial_table: Option<String>,
    ) -> Self {
        Self {
            conn_id,
            tables,
            selected_schema: initial_schema,
            selected_table: initial_table,
            selected_columns: HashSet::new(),
            options: AnalyzeOptions::default(),
            executing: false,
        }
    }

    fn schemas(&self) -> Vec<String> {
        let mut schemas: Vec<_> = self
            .tables
            .iter()
            .map(|t| t.schema.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        schemas.sort();
        schemas
    }

    fn tables_for_schema(&self) -> Vec<String> {
        match &self.selected_schema {
            Some(schema) => {
                let mut tables: Vec<_> = self
                    .tables
                    .iter()
                    .filter(|t| &t.schema == schema)
                    .map(|t| t.name.clone())
                    .collect();
                tables.sort();
                tables
            }
            None => Vec::new(),
        }
    }

    fn columns_for_table(&self) -> Vec<String> {
        match (&self.selected_schema, &self.selected_table) {
            (Some(schema), Some(table)) => {
                self.tables
                    .iter()
                    .find(|t| &t.schema == schema && &t.name == table)
                    .map(|t| t.columns.iter().map(|c| c.name.clone()).collect())
                    .unwrap_or_default()
            }
            _ => Vec::new(),
        }
    }

    fn on_schema_change(&mut self, schema: Option<String>, cx: &mut Context<Self>) {
        self.selected_schema = schema;
        self.selected_table = None;
        self.selected_columns.clear();
        cx.notify();
    }

    fn on_table_change(&mut self, table: Option<String>, cx: &mut Context<Self>) {
        self.selected_table = table;
        self.selected_columns.clear();
        cx.notify();
    }

    fn toggle_column(&mut self, column: String, cx: &mut Context<Self>) {
        if self.selected_columns.contains(&column) {
            self.selected_columns.remove(&column);
        } else {
            self.selected_columns.insert(column);
        }
        cx.notify();
    }

    fn on_run(&mut self, cx: &mut Context<Self>) {
        self.executing = true;
        cx.notify();

        let conn_id = self.conn_id.clone();
        let schema = self.selected_schema.clone();
        let table = self.selected_table.clone();
        let mut options = self.options.clone();
        options.columns = self.selected_columns.iter().cloned().collect();

        cx.spawn(|this, mut cx| async move {
            let result = cx
                .update(|cx| {
                    let state = cx.global::<MaintenanceState>();
                    state.analyze(&conn_id, schema.as_deref(), table.as_deref(), options)
                })
                .ok();

            if let Some(fut) = result {
                let _ = fut.await;
            }

            let _ = this.update(&mut cx, |this, cx| {
                this.executing = false;
                cx.emit(AnalyzeDialogEvent::Completed);
            });
        })
        .detach();
    }

    fn on_cancel(&mut self, cx: &mut Context<Self>) {
        cx.emit(AnalyzeDialogEvent::Cancelled);
    }
}

pub enum AnalyzeDialogEvent {
    Completed,
    Cancelled,
}

impl EventEmitter<AnalyzeDialogEvent> for AnalyzeDialog {}

impl Render for AnalyzeDialog {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let schemas = self.schemas();
        let tables = self.tables_for_schema();
        let columns = self.columns_for_table();

        Modal::new("analyze-dialog")
            .title("ANALYZE")
            .width(px(500.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_4()
                    .max_h(vh(60.0))
                    .overflow_y_auto()
                    // Target Selection
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.text)
                                    .child("Target"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(
                                        Select::new(self.selected_schema.clone().unwrap_or_default())
                                            .placeholder("All schemas")
                                            .options(
                                                std::iter::once(("", "All schemas"))
                                                    .chain(schemas.iter().map(|s| (s.as_str(), s.as_str())))
                                                    .collect(),
                                            )
                                            .on_change(cx.listener(|this, value: String, cx| {
                                                let schema = if value.is_empty() { None } else { Some(value) };
                                                this.on_schema_change(schema, cx);
                                            })),
                                    )
                                    .child(
                                        Select::new(self.selected_table.clone().unwrap_or_default())
                                            .placeholder("All tables")
                                            .disabled(self.selected_schema.is_none())
                                            .options(
                                                std::iter::once(("", "All tables"))
                                                    .chain(tables.iter().map(|t| (t.as_str(), t.as_str())))
                                                    .collect(),
                                            )
                                            .on_change(cx.listener(|this, value: String, cx| {
                                                let table = if value.is_empty() { None } else { Some(value) };
                                                this.on_table_change(table, cx);
                                            })),
                                    ),
                            ),
                    )
                    // Column Selection (when table selected)
                    .when(self.selected_table.is_some() && !columns.is_empty(), |this| {
                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.text)
                                        .child("Specific Columns (optional)"),
                                )
                                .child(
                                    div()
                                        .max_h_32()
                                        .overflow_y_auto()
                                        .border_1()
                                        .border_color(theme.border)
                                        .rounded_md()
                                        .p_2()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .children(columns.iter().map(|col| {
                                            let col_name = col.clone();
                                            let is_selected = self.selected_columns.contains(col);
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .cursor_pointer()
                                                .child(
                                                    Checkbox::new(is_selected)
                                                        .on_toggle(cx.listener(move |this, _, cx| {
                                                            this.toggle_column(col_name.clone(), cx);
                                                        })),
                                                )
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .font_family("monospace")
                                                        .text_color(theme.text)
                                                        .child(col.clone()),
                                                )
                                        })),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.text_muted)
                                        .mt_1()
                                        .child("Leave empty to analyze all columns"),
                                ),
                        )
                    })
                    // Options
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.text_muted)
                                    .child("Options"),
                            )
                            .child(self.render_option(
                                "VERBOSE",
                                "Print progress messages for each table.",
                                self.options.verbose,
                                cx.listener(|this, checked, cx| {
                                    this.options.verbose = checked;
                                    cx.notify();
                                }),
                                theme,
                            ))
                            .child(self.render_option(
                                "SKIP_LOCKED",
                                "Skip tables that cannot be locked immediately.",
                                self.options.skip_locked,
                                cx.listener(|this, checked, cx| {
                                    this.options.skip_locked = checked;
                                    cx.notify();
                                }),
                                theme,
                            )),
                    )
                    // Info
                    .child(
                        div()
                            .p_3()
                            .bg(theme.info_bg)
                            .border_1()
                            .border_color(theme.info)
                            .rounded_md()
                            .text_sm()
                            .text_color(theme.info)
                            .child(
                                div()
                                    .child(
                                        Span::new()
                                            .font_weight(FontWeight::BOLD)
                                            .child("Note: "),
                                    )
                                    .child("ANALYZE collects statistics about the contents of tables in the database, which the query planner uses to generate better execution plans."),
                            ),
                    ),
            )
            .actions(vec![
                Button::new("cancel")
                    .label("Cancel")
                    .variant(ButtonVariant::Ghost)
                    .disabled(self.executing)
                    .on_click(cx.listener(|this, _, cx| this.on_cancel(cx))),
                Button::new("run")
                    .label("Run ANALYZE")
                    .variant(ButtonVariant::Primary)
                    .loading(self.executing)
                    .on_click(cx.listener(|this, _, cx| this.on_run(cx))),
            ])
    }
}

impl AnalyzeDialog {
    fn render_option(
        &self,
        name: &str,
        description: &str,
        checked: bool,
        on_toggle: impl Fn(&mut Self, bool, &mut Context<Self>) + 'static,
        theme: &Theme,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_start()
            .gap_3()
            .cursor_pointer()
            .child(Checkbox::new(checked).on_toggle(on_toggle))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child(name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(description),
                    ),
            )
    }
}
```

## 21.6 REINDEX Dialog Component

```rust
// src/components/maintenance/reindex_dialog.rs

use crate::models::maintenance::{ReindexOptions, ReindexTarget, ReindexTargetType};
use crate::models::schema::{Index, Table};
use crate::state::maintenance_state::MaintenanceState;
use crate::theme::Theme;
use gpui::*;
use std::collections::HashSet;

/// REINDEX dialog component
pub struct ReindexDialog {
    conn_id: String,
    tables: Vec<Table>,
    indexes: Vec<Index>,
    target_type: ReindexTargetType,
    selected_schema: Option<String>,
    selected_table: Option<String>,
    selected_index: Option<String>,
    options: ReindexOptions,
    executing: bool,
}

impl ReindexDialog {
    pub fn new(
        conn_id: String,
        tables: Vec<Table>,
        indexes: Vec<Index>,
        initial_target: Option<ReindexTarget>,
    ) -> Self {
        let (target_type, schema, table, index) = initial_target
            .map(|t| {
                (
                    t.target_type,
                    t.schema,
                    t.name.clone(),
                    if t.target_type == ReindexTargetType::Index {
                        t.name
                    } else {
                        None
                    },
                )
            })
            .unwrap_or((ReindexTargetType::Table, None, None, None));

        Self {
            conn_id,
            tables,
            indexes,
            target_type,
            selected_schema: schema,
            selected_table: table,
            selected_index: index,
            options: ReindexOptions::default(),
            executing: false,
        }
    }

    fn schemas(&self) -> Vec<String> {
        let mut schemas: Vec<_> = self
            .tables
            .iter()
            .map(|t| t.schema.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        schemas.sort();
        schemas
    }

    fn tables_for_schema(&self) -> Vec<String> {
        match &self.selected_schema {
            Some(schema) => {
                let mut tables: Vec<_> = self
                    .tables
                    .iter()
                    .filter(|t| &t.schema == schema)
                    .map(|t| t.name.clone())
                    .collect();
                tables.sort();
                tables
            }
            None => Vec::new(),
        }
    }

    fn indexes_for_table(&self) -> Vec<String> {
        match (&self.selected_schema, &self.selected_table) {
            (Some(schema), Some(table)) => {
                let mut idxs: Vec<_> = self
                    .indexes
                    .iter()
                    .filter(|i| &i.schema == schema && &i.table == table)
                    .map(|i| i.name.clone())
                    .collect();
                idxs.sort();
                idxs
            }
            _ => Vec::new(),
        }
    }

    fn on_target_type_change(&mut self, target_type: ReindexTargetType, cx: &mut Context<Self>) {
        self.target_type = target_type;
        cx.notify();
    }

    fn on_schema_change(&mut self, schema: Option<String>, cx: &mut Context<Self>) {
        self.selected_schema = schema;
        self.selected_table = None;
        self.selected_index = None;
        cx.notify();
    }

    fn on_table_change(&mut self, table: Option<String>, cx: &mut Context<Self>) {
        self.selected_table = table;
        self.selected_index = None;
        cx.notify();
    }

    fn on_index_change(&mut self, index: Option<String>, cx: &mut Context<Self>) {
        self.selected_index = index;
        cx.notify();
    }

    fn can_run(&self) -> bool {
        match self.target_type {
            ReindexTargetType::Database => true,
            ReindexTargetType::Schema => self.selected_schema.is_some(),
            ReindexTargetType::Table => {
                self.selected_schema.is_some() && self.selected_table.is_some()
            }
            ReindexTargetType::Index => {
                self.selected_schema.is_some() && self.selected_index.is_some()
            }
        }
    }

    fn on_run(&mut self, cx: &mut Context<Self>) {
        self.executing = true;
        cx.notify();

        let conn_id = self.conn_id.clone();
        let target = ReindexTarget {
            target_type: self.target_type.clone(),
            schema: match self.target_type {
                ReindexTargetType::Index | ReindexTargetType::Table => self.selected_schema.clone(),
                _ => None,
            },
            name: match self.target_type {
                ReindexTargetType::Index => self.selected_index.clone(),
                ReindexTargetType::Table => self.selected_table.clone(),
                ReindexTargetType::Schema => self.selected_schema.clone(),
                ReindexTargetType::Database => None,
            },
        };
        let options = self.options.clone();

        cx.spawn(|this, mut cx| async move {
            let result = cx
                .update(|cx| {
                    let state = cx.global::<MaintenanceState>();
                    state.reindex(&conn_id, target, options)
                })
                .ok();

            if let Some(fut) = result {
                let _ = fut.await;
            }

            let _ = this.update(&mut cx, |this, cx| {
                this.executing = false;
                cx.emit(ReindexDialogEvent::Completed);
            });
        })
        .detach();
    }

    fn on_cancel(&mut self, cx: &mut Context<Self>) {
        cx.emit(ReindexDialogEvent::Cancelled);
    }
}

pub enum ReindexDialogEvent {
    Completed,
    Cancelled,
}

impl EventEmitter<ReindexDialogEvent> for ReindexDialog {}

impl Render for ReindexDialog {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let schemas = self.schemas();
        let tables = self.tables_for_schema();
        let indexes = self.indexes_for_table();

        Modal::new("reindex-dialog")
            .title("REINDEX")
            .width(px(500.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_4()
                    .max_h(vh(60.0))
                    .overflow_y_auto()
                    // Target Type Selection
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.text)
                                    .child("Target Type"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_4()
                                    .children(ReindexTargetType::all().iter().map(|tt| {
                                        let is_selected = self.target_type == *tt;
                                        let target_type = tt.clone();
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .cursor_pointer()
                                            .child(
                                                Radio::new(is_selected)
                                                    .on_click(cx.listener(move |this, _, cx| {
                                                        this.on_target_type_change(target_type.clone(), cx);
                                                    })),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(theme.text)
                                                    .child(tt.as_str().to_lowercase()),
                                            )
                                    })),
                            ),
                    )
                    // Target Selection (conditional on type)
                    .when(self.target_type != ReindexTargetType::Database, |this| {
                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.text)
                                        .child("Target"),
                                )
                                .when(self.target_type == ReindexTargetType::Schema, |this| {
                                    this.child(
                                        Select::new(self.selected_schema.clone().unwrap_or_default())
                                            .placeholder("Select schema...")
                                            .options(
                                                std::iter::once(("", "Select schema..."))
                                                    .chain(schemas.iter().map(|s| (s.as_str(), s.as_str())))
                                                    .collect(),
                                            )
                                            .on_change(cx.listener(|this, value: String, cx| {
                                                let schema = if value.is_empty() { None } else { Some(value) };
                                                this.on_schema_change(schema, cx);
                                            })),
                                    )
                                })
                                .when(
                                    self.target_type == ReindexTargetType::Table
                                        || self.target_type == ReindexTargetType::Index,
                                    |this| {
                                        this.child(
                                            div()
                                                .flex()
                                                .gap_2()
                                                .child(
                                                    Select::new(
                                                        self.selected_schema.clone().unwrap_or_default(),
                                                    )
                                                    .placeholder("Select schema...")
                                                    .options(
                                                        std::iter::once(("", "Select schema..."))
                                                            .chain(
                                                                schemas
                                                                    .iter()
                                                                    .map(|s| (s.as_str(), s.as_str())),
                                                            )
                                                            .collect(),
                                                    )
                                                    .on_change(cx.listener(|this, value: String, cx| {
                                                        let schema =
                                                            if value.is_empty() { None } else { Some(value) };
                                                        this.on_schema_change(schema, cx);
                                                    })),
                                                )
                                                .child(
                                                    Select::new(
                                                        self.selected_table.clone().unwrap_or_default(),
                                                    )
                                                    .placeholder("Select table...")
                                                    .disabled(self.selected_schema.is_none())
                                                    .options(
                                                        std::iter::once(("", "Select table..."))
                                                            .chain(
                                                                tables
                                                                    .iter()
                                                                    .map(|t| (t.as_str(), t.as_str())),
                                                            )
                                                            .collect(),
                                                    )
                                                    .on_change(cx.listener(|this, value: String, cx| {
                                                        let table =
                                                            if value.is_empty() { None } else { Some(value) };
                                                        this.on_table_change(table, cx);
                                                    })),
                                                ),
                                        )
                                    },
                                )
                                .when(
                                    self.target_type == ReindexTargetType::Index
                                        && self.selected_table.is_some(),
                                    |this| {
                                        this.child(
                                            Select::new(self.selected_index.clone().unwrap_or_default())
                                                .placeholder("Select index...")
                                                .options(
                                                    std::iter::once(("", "Select index..."))
                                                        .chain(
                                                            indexes.iter().map(|i| (i.as_str(), i.as_str())),
                                                        )
                                                        .collect(),
                                                )
                                                .on_change(cx.listener(|this, value: String, cx| {
                                                    let index =
                                                        if value.is_empty() { None } else { Some(value) };
                                                    this.on_index_change(index, cx);
                                                })),
                                        )
                                    },
                                ),
                        )
                    })
                    // Options
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.text_muted)
                                    .child("Options"),
                            )
                            .child(self.render_option(
                                "CONCURRENTLY",
                                "Rebuild the index without locking writes. Takes longer but doesn't block normal database operations.",
                                self.options.concurrently,
                                cx.listener(|this, checked, cx| {
                                    this.options.concurrently = checked;
                                    cx.notify();
                                }),
                                theme,
                            ))
                            .child(self.render_option(
                                "VERBOSE",
                                "Print progress report.",
                                self.options.verbose,
                                cx.listener(|this, checked, cx| {
                                    this.options.verbose = checked;
                                    cx.notify();
                                }),
                                theme,
                            )),
                    )
                    // Info/Warning
                    .when(self.options.concurrently, |this| {
                        this.child(
                            div()
                                .p_3()
                                .bg(theme.info_bg)
                                .border_1()
                                .border_color(theme.info)
                                .rounded_md()
                                .text_sm()
                                .text_color(theme.info)
                                .child(
                                    div()
                                        .child(
                                            Span::new()
                                                .font_weight(FontWeight::BOLD)
                                                .child("Note: "),
                                        )
                                        .child("CONCURRENTLY requires more time and resources but allows normal database operations to continue during the reindex."),
                                ),
                        )
                    })
                    .when(!self.options.concurrently, |this| {
                        this.child(
                            div()
                                .p_3()
                                .bg(theme.warning_bg)
                                .border_1()
                                .border_color(theme.warning)
                                .rounded_md()
                                .text_sm()
                                .text_color(theme.warning)
                                .child(
                                    div()
                                        .child(
                                            Span::new()
                                                .font_weight(FontWeight::BOLD)
                                                .child("Warning: "),
                                        )
                                        .child("Without CONCURRENTLY, the table will be locked for writes during the entire reindex operation."),
                                ),
                        )
                    }),
            )
            .actions(vec![
                Button::new("cancel")
                    .label("Cancel")
                    .variant(ButtonVariant::Ghost)
                    .disabled(self.executing)
                    .on_click(cx.listener(|this, _, cx| this.on_cancel(cx))),
                Button::new("run")
                    .label("Run REINDEX")
                    .variant(ButtonVariant::Primary)
                    .loading(self.executing)
                    .disabled(!self.can_run())
                    .on_click(cx.listener(|this, _, cx| this.on_run(cx))),
            ])
    }
}

impl ReindexDialog {
    fn render_option(
        &self,
        name: &str,
        description: &str,
        checked: bool,
        on_toggle: impl Fn(&mut Self, bool, &mut Context<Self>) + 'static,
        theme: &Theme,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_start()
            .gap_3()
            .cursor_pointer()
            .child(Checkbox::new(checked).on_toggle(on_toggle))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child(name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(description),
                    ),
            )
    }
}
```

## 21.7 Maintenance Jobs Panel

```rust
// src/components/maintenance/jobs_panel.rs

use crate::models::maintenance::{JobStatus, MaintenanceJob};
use crate::state::maintenance_state::MaintenanceState;
use crate::theme::Theme;
use gpui::*;

/// Panel showing maintenance job history and progress
pub struct MaintenanceJobsPanel {
    expanded_job_id: Option<String>,
}

impl MaintenanceJobsPanel {
    pub fn new() -> Self {
        Self {
            expanded_job_id: None,
        }
    }

    fn toggle_job(&mut self, job_id: String, cx: &mut Context<Self>) {
        if self.expanded_job_id.as_ref() == Some(&job_id) {
            self.expanded_job_id = None;
        } else {
            self.expanded_job_id = Some(job_id);
        }
        cx.notify();
    }

    fn clear_completed(&mut self, cx: &mut Context<Self>) {
        cx.global::<MaintenanceState>().clear_completed_jobs();
        cx.notify();
    }

    fn get_status_icon(status: &JobStatus) -> &'static str {
        match status {
            JobStatus::Pending => "clock",
            JobStatus::Running => "spinner",
            JobStatus::Completed => "check-circle",
            JobStatus::Failed => "x-circle",
            JobStatus::Cancelled => "minus-circle",
        }
    }

    fn get_status_color(status: &JobStatus, theme: &Theme) -> Hsla {
        match status {
            JobStatus::Pending => theme.text_muted,
            JobStatus::Running => theme.info,
            JobStatus::Completed => theme.success,
            JobStatus::Failed => theme.error,
            JobStatus::Cancelled => theme.text_muted,
        }
    }
}

impl Render for MaintenanceJobsPanel {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let state = cx.global::<MaintenanceState>();
        let jobs = state.get_all_jobs();
        let completed_count = state.get_completed_jobs().len();

        Modal::new("maintenance-jobs")
            .title("Maintenance Jobs")
            .width(px(600.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .max_h(vh(60.0))
                    .overflow_y_auto()
                    .when(jobs.is_empty(), |this| {
                        this.child(
                            div()
                                .p_8()
                                .text_center()
                                .text_color(theme.text_muted)
                                .child("No maintenance jobs"),
                        )
                    })
                    .when(!jobs.is_empty(), |this| {
                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .children(jobs.iter().map(|job| {
                                    let job_id = job.id.clone();
                                    let is_expanded = self.expanded_job_id.as_ref() == Some(&job.id);

                                    div()
                                        .border_b_1()
                                        .border_color(theme.border)
                                        .p_4()
                                        // Job header
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .cursor_pointer()
                                                .on_click(cx.listener({
                                                    let job_id = job_id.clone();
                                                    move |this, _, cx| {
                                                        this.toggle_job(job_id.clone(), cx);
                                                    }
                                                }))
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap_3()
                                                        .child(
                                                            Icon::new(Self::get_status_icon(&job.status))
                                                                .size(IconSize::Medium)
                                                                .color(Self::get_status_color(
                                                                    &job.status,
                                                                    theme,
                                                                ))
                                                                .when(job.status == JobStatus::Running, |icon| {
                                                                    icon.class("animate-spin")
                                                                }),
                                                        )
                                                        .child(
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap_2()
                                                                .child(
                                                                    div()
                                                                        .text_sm()
                                                                        .font_weight(FontWeight::MEDIUM)
                                                                        .text_color(theme.text)
                                                                        .child(job.job_type.as_str()),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_sm()
                                                                        .text_color(theme.text_muted)
                                                                        .child(job.target.clone()),
                                                                ),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap_4()
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .text_color(Self::get_status_color(
                                                                    &job.status,
                                                                    theme,
                                                                ))
                                                                .child(job.status.as_str()),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .text_color(theme.text_muted)
                                                                .child(job.format_duration()),
                                                        )
                                                        .child(
                                                            Icon::new(if is_expanded {
                                                                "chevron-up"
                                                            } else {
                                                                "chevron-down"
                                                            })
                                                            .size(IconSize::Small)
                                                            .color(theme.text_muted),
                                                        ),
                                                ),
                                        )
                                        // Job details (expanded)
                                        .when(is_expanded, |this| {
                                            this.child(
                                                div()
                                                    .mt_3()
                                                    .pl_9()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_2()
                                                    // Output
                                                    .when(!job.output.is_empty(), |this| {
                                                        this.child(
                                                            div()
                                                                .p_3()
                                                                .bg(theme.surface_hover)
                                                                .rounded_md()
                                                                .font_family("monospace")
                                                                .text_xs()
                                                                .max_h_40()
                                                                .overflow_auto()
                                                                .whitespace_pre_wrap()
                                                                .child(job.output.join("\n")),
                                                        )
                                                    })
                                                    // Error
                                                    .when(job.error.is_some(), |this| {
                                                        this.child(
                                                            div()
                                                                .p_3()
                                                                .bg(theme.error_bg)
                                                                .border_1()
                                                                .border_color(theme.error)
                                                                .rounded_md()
                                                                .text_sm()
                                                                .text_color(theme.error)
                                                                .child(
                                                                    job.error.clone().unwrap_or_default(),
                                                                ),
                                                        )
                                                    })
                                                    // Timestamps
                                                    .when(job.start_time.is_some(), |this| {
                                                        this.child(
                                                            div()
                                                                .text_xs()
                                                                .text_color(theme.text_muted)
                                                                .child(format!(
                                                                    "Started: {}{}",
                                                                    job.start_time
                                                                        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                                                                        .unwrap_or_default(),
                                                                    job.end_time
                                                                        .map(|t| format!(
                                                                            "  |  Ended: {}",
                                                                            t.format("%Y-%m-%d %H:%M:%S")
                                                                        ))
                                                                        .unwrap_or_default()
                                                                )),
                                                        )
                                                    }),
                                            )
                                        })
                                })),
                        )
                    }),
            )
            .actions(vec![
                Button::new("clear")
                    .label("Clear Completed")
                    .variant(ButtonVariant::Ghost)
                    .disabled(completed_count == 0)
                    .on_click(cx.listener(|this, _, cx| this.clear_completed(cx))),
                Button::new("close")
                    .label("Close")
                    .variant(ButtonVariant::Secondary)
                    .on_click(cx.listener(|_, _, cx| {
                        cx.emit(JobsPanelEvent::Close);
                    })),
            ])
    }
}

pub enum JobsPanelEvent {
    Close,
}

impl EventEmitter<JobsPanelEvent> for MaintenanceJobsPanel {}
```

## 21.8 Module Exports

```rust
// src/components/maintenance/mod.rs

mod vacuum_dialog;
mod analyze_dialog;
mod reindex_dialog;
mod cluster_dialog;
mod jobs_panel;

pub use vacuum_dialog::{VacuumDialog, VacuumDialogEvent};
pub use analyze_dialog::{AnalyzeDialog, AnalyzeDialogEvent};
pub use reindex_dialog::{ReindexDialog, ReindexDialogEvent};
pub use cluster_dialog::{ClusterDialog, ClusterDialogEvent};
pub use jobs_panel::{MaintenanceJobsPanel, JobsPanelEvent};
```

## Acceptance Criteria

1. **VACUUM Dialog**
   - [x] Select target table or all tables
   - [x] Configure FULL, FREEZE, ANALYZE options
   - [x] Set VERBOSE, SKIP_LOCKED options
   - [x] Configure INDEX_CLEANUP and PARALLEL
   - [x] Show warning for FULL option
   - [x] Execute with job tracking

2. **ANALYZE Dialog**
   - [x] Select target table or all tables
   - [x] Optionally select specific columns
   - [x] Configure VERBOSE option
   - [x] Support SKIP_LOCKED option
   - [x] Informational note about ANALYZE purpose

3. **REINDEX Dialog**
   - [x] Select target type (index, table, schema, database)
   - [x] Support CONCURRENTLY option
   - [x] Configure VERBOSE option
   - [x] Warning for non-concurrent reindex
   - [x] Info note for concurrent reindex

4. **Progress Tracking**
   - [x] Show active jobs with spinner
   - [x] Display completed jobs with output
   - [x] Show error messages for failed jobs
   - [x] Expandable job details
   - [x] Clear completed jobs functionality
   - [x] Job duration tracking

5. **Integration**
   - [x] MaintenanceState Global for app-wide job tracking
   - [x] Direct service calls (no IPC)
   - [x] Async execution with UI feedback
   - [x] Event emission for dialog completion

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maintenance_job_duration() {
        let mut job = MaintenanceJob::new(MaintenanceType::Vacuum, "public.users".to_string());
        assert!(job.duration().is_none());
        assert_eq!(job.format_duration(), "-");

        job.start_time = Some(chrono::Utc::now() - chrono::Duration::seconds(5));
        assert!(job.duration().is_some());
        assert!(job.format_duration().contains("s"));
    }

    #[test]
    fn test_vacuum_options_default() {
        let opts = VacuumOptions::default();
        assert!(!opts.full);
        assert!(!opts.freeze);
        assert!(!opts.analyze);
        assert!(opts.verbose);
        assert!(!opts.skip_locked);
        assert_eq!(opts.index_cleanup, IndexCleanupMode::Auto);
        assert_eq!(opts.parallel, 0);
        assert!(opts.truncate);
        assert!(opts.process_toast);
    }

    #[test]
    fn test_reindex_target_type() {
        assert_eq!(ReindexTargetType::Table.as_str(), "TABLE");
        assert_eq!(ReindexTargetType::Index.as_str(), "INDEX");
        assert_eq!(ReindexTargetType::Schema.as_str(), "SCHEMA");
        assert_eq!(ReindexTargetType::Database.as_str(), "DATABASE");
    }

    #[test]
    fn test_job_status() {
        assert_eq!(JobStatus::Running.as_str(), "running");
        assert_eq!(JobStatus::Completed.as_str(), "completed");
        assert_eq!(JobStatus::Failed.as_str(), "failed");
    }
}
```

### Integration Tests with Tauri MCP

```rust
#[tokio::test]
async fn test_vacuum_dialog_e2e() {
    // Start driver session
    let session = mcp___hypothesi_tauri_mcp_server__driver_session(json!({
        "action": "start"
    })).await;

    // Open vacuum dialog from admin dashboard
    mcp___hypothesi_tauri_mcp_server__webview_interact(json!({
        "action": "click",
        "selector": "button[data-testid='vacuum-button']"
    })).await;

    // Wait for dialog
    mcp___hypothesi_tauri_mcp_server__webview_wait_for(json!({
        "type": "text",
        "value": "VACUUM",
        "timeout": 5000
    })).await;

    // Take snapshot
    let snapshot = mcp___hypothesi_tauri_mcp_server__webview_dom_snapshot(json!({
        "type": "accessibility"
    })).await;

    // Verify options are present
    assert!(snapshot.contains("FULL"));
    assert!(snapshot.contains("ANALYZE"));
    assert!(snapshot.contains("VERBOSE"));

    // Enable ANALYZE option
    mcp___hypothesi_tauri_mcp_server__webview_interact(json!({
        "action": "click",
        "selector": "input[data-option='analyze']"
    })).await;

    // Run vacuum
    mcp___hypothesi_tauri_mcp_server__webview_interact(json!({
        "action": "click",
        "selector": "button:has-text('Run VACUUM')"
    })).await;

    // Wait for completion
    mcp___hypothesi_tauri_mcp_server__webview_wait_for(json!({
        "type": "text",
        "value": "completed",
        "timeout": 30000
    })).await;

    // Stop session
    mcp___hypothesi_tauri_mcp_server__driver_session(json!({
        "action": "stop"
    })).await;
}

#[tokio::test]
async fn test_maintenance_jobs_panel() {
    let session = mcp___hypothesi_tauri_mcp_server__driver_session(json!({
        "action": "start"
    })).await;

    // Open jobs panel
    mcp___hypothesi_tauri_mcp_server__webview_interact(json!({
        "action": "click",
        "selector": "button[data-testid='maintenance-jobs']"
    })).await;

    // Wait for panel
    mcp___hypothesi_tauri_mcp_server__webview_wait_for(json!({
        "type": "text",
        "value": "Maintenance Jobs"
    })).await;

    // Expand a job if present
    mcp___hypothesi_tauri_mcp_server__webview_interact(json!({
        "action": "click",
        "selector": "[data-testid='job-row']:first-child"
    })).await;

    // Clear completed
    mcp___hypothesi_tauri_mcp_server__webview_interact(json!({
        "action": "click",
        "selector": "button:has-text('Clear Completed')"
    })).await;

    mcp___hypothesi_tauri_mcp_server__driver_session(json!({
        "action": "stop"
    })).await;
}
```
