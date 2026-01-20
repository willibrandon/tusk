# Feature 23: Extension Manager

## Overview

The Extension Manager provides a GUI for managing PostgreSQL extensions, including viewing installed extensions, installing new extensions, upgrading to newer versions, and viewing extension details and dependencies. Built with GPUI for native performance and cross-platform support.

## Goals

- List all available and installed extensions
- Install extensions with schema selection
- Upgrade extensions to newer versions
- Uninstall extensions (with CASCADE option)
- Display extension details, dependencies, and objects
- Generate SQL for extension operations

## Dependencies

- Feature 07: Connection Pool Management
- Feature 10: Schema Cache (for schema list)

## Technical Specification

### 23.1 Extension Data Models

```rust
// src/models/extension.rs

use serde::{Deserialize, Serialize};

/// A PostgreSQL extension (available or installed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extension {
    pub name: String,
    pub installed_version: Option<String>,
    pub default_version: String,
    pub available_versions: Vec<String>,
    pub schema: Option<String>,
    pub relocatable: bool,
    pub comment: Option<String>,
    pub requires: Vec<String>,
    pub is_installed: bool,
}

impl Extension {
    /// Check if an upgrade is available
    pub fn has_upgrade(&self) -> bool {
        if let Some(ref installed) = self.installed_version {
            installed != &self.default_version
        } else {
            false
        }
    }

    /// Get the upgrade target version (if available)
    pub fn upgrade_version(&self) -> Option<&str> {
        if self.has_upgrade() {
            Some(&self.default_version)
        } else {
            None
        }
    }
}

/// Detailed information about an installed extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionDetail {
    pub name: String,
    pub version: String,
    pub schema: String,
    pub description: String,
    pub requires: Vec<String>,
    pub objects: Vec<ExtensionObject>,
    pub config: Vec<ExtensionConfig>,
}

impl ExtensionDetail {
    /// Get objects grouped by type
    pub fn objects_by_type(&self) -> std::collections::BTreeMap<String, Vec<&ExtensionObject>> {
        let mut groups = std::collections::BTreeMap::new();
        for obj in &self.objects {
            groups
                .entry(obj.object_type.clone())
                .or_insert_with(Vec::new)
                .push(obj);
        }
        groups
    }

    /// Get total object count
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }
}

/// An object created by an extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionObject {
    pub object_type: String, // function, type, operator, table, etc.
    pub schema: String,
    pub name: String,
    pub identity: String, // Full qualified name with signature
}

/// A configuration parameter for an extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionConfig {
    pub name: String,
    pub value: String,
    pub description: String,
    pub unit: Option<String>,
    pub vartype: String,
    pub enum_vals: Option<Vec<String>>,
    pub min_val: Option<String>,
    pub max_val: Option<String>,
}

impl ExtensionConfig {
    /// Format the value with unit for display
    pub fn display_value(&self) -> String {
        if let Some(ref unit) = self.unit {
            format!("{} {}", self.value, unit)
        } else {
            self.value.clone()
        }
    }
}

/// Options for installing an extension
#[derive(Debug, Clone, Default)]
pub struct InstallExtensionOptions {
    pub name: String,
    pub version: Option<String>,
    pub schema: Option<String>,
    pub cascade: bool,
}

impl InstallExtensionOptions {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cascade: true, // Default to cascade for convenience
            ..Default::default()
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    pub fn cascade(mut self, cascade: bool) -> Self {
        self.cascade = cascade;
        self
    }
}

/// Options for upgrading an extension
#[derive(Debug, Clone, Default)]
pub struct UpgradeExtensionOptions {
    pub name: String,
    pub target_version: Option<String>,
}

impl UpgradeExtensionOptions {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            target_version: None,
        }
    }

    pub fn to_version(mut self, version: impl Into<String>) -> Self {
        self.target_version = Some(version.into());
        self
    }
}
```

### 23.2 Extension Service

```rust
// src/services/extension.rs

use crate::models::extension::{
    Extension, ExtensionConfig, ExtensionDetail, ExtensionObject,
    InstallExtensionOptions, UpgradeExtensionOptions,
};
use deadpool_postgres::Pool;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtensionError {
    #[error("Database error: {0}")]
    Database(#[from] tokio_postgres::Error),

    #[error("Pool error: {0}")]
    Pool(#[from] deadpool_postgres::PoolError),

    #[error("Extension not found: {0}")]
    NotFound(String),

    #[error("Extension not installed: {0}")]
    NotInstalled(String),

    #[error("Extension already installed: {0}")]
    AlreadyInstalled(String),

    #[error("Cannot uninstall: {0}")]
    CannotUninstall(String),
}

pub struct ExtensionService;

impl ExtensionService {
    /// Get all available and installed extensions
    pub async fn get_extensions(pool: &Pool) -> Result<Vec<Extension>, ExtensionError> {
        let client = pool.get().await?;

        let rows = client
            .query(
                r#"
                SELECT
                    a.name,
                    e.extversion AS installed_version,
                    a.default_version,
                    n.nspname AS schema,
                    a.relocatable,
                    a.comment,
                    COALESCE(a.requires, ARRAY[]::name[]) AS requires,
                    e.oid IS NOT NULL AS is_installed
                FROM pg_available_extensions a
                LEFT JOIN pg_extension e ON e.extname = a.name
                LEFT JOIN pg_namespace n ON n.oid = e.extnamespace
                ORDER BY a.name
                "#,
                &[],
            )
            .await?;

        let extensions = rows
            .iter()
            .map(|row| {
                let requires: Vec<String> = row.get("requires");

                Extension {
                    name: row.get("name"),
                    installed_version: row.get("installed_version"),
                    default_version: row.get("default_version"),
                    available_versions: vec![], // Populated separately on demand
                    schema: row.get("schema"),
                    relocatable: row.get("relocatable"),
                    comment: row.get("comment"),
                    requires,
                    is_installed: row.get("is_installed"),
                }
            })
            .collect();

        Ok(extensions)
    }

    /// Get available versions for an extension
    pub async fn get_available_versions(
        pool: &Pool,
        extension_name: &str,
    ) -> Result<Vec<String>, ExtensionError> {
        let client = pool.get().await?;

        let rows = client
            .query(
                r#"
                SELECT version
                FROM pg_available_extension_versions
                WHERE name = $1
                ORDER BY version DESC
                "#,
                &[&extension_name],
            )
            .await?;

        let versions = rows.iter().map(|row| row.get("version")).collect();
        Ok(versions)
    }

    /// Get detailed information about an installed extension
    pub async fn get_extension_detail(
        pool: &Pool,
        extension_name: &str,
    ) -> Result<ExtensionDetail, ExtensionError> {
        let client = pool.get().await?;

        // Get basic info
        let info_row = client
            .query_opt(
                r#"
                SELECT
                    e.extname AS name,
                    e.extversion AS version,
                    n.nspname AS schema,
                    COALESCE(a.comment, '') AS description,
                    COALESCE(a.requires, ARRAY[]::name[]) AS requires
                FROM pg_extension e
                JOIN pg_namespace n ON n.oid = e.extnamespace
                LEFT JOIN pg_available_extensions a ON a.name = e.extname
                WHERE e.extname = $1
                "#,
                &[&extension_name],
            )
            .await?
            .ok_or_else(|| ExtensionError::NotInstalled(extension_name.to_string()))?;

        let name: String = info_row.get("name");
        let version: String = info_row.get("version");
        let schema: String = info_row.get("schema");
        let description: String = info_row.get("description");
        let requires: Vec<String> = info_row.get("requires");

        // Get objects created by extension
        let object_rows = client
            .query(
                r#"
                SELECT
                    CASE classid
                        WHEN 'pg_proc'::regclass THEN 'function'
                        WHEN 'pg_type'::regclass THEN 'type'
                        WHEN 'pg_operator'::regclass THEN 'operator'
                        WHEN 'pg_class'::regclass THEN
                            CASE (SELECT relkind FROM pg_class WHERE oid = objid)
                                WHEN 'r' THEN 'table'
                                WHEN 'i' THEN 'index'
                                WHEN 'S' THEN 'sequence'
                                WHEN 'v' THEN 'view'
                                WHEN 'm' THEN 'materialized view'
                                ELSE 'relation'
                            END
                        WHEN 'pg_cast'::regclass THEN 'cast'
                        WHEN 'pg_opclass'::regclass THEN 'operator class'
                        WHEN 'pg_opfamily'::regclass THEN 'operator family'
                        WHEN 'pg_am'::regclass THEN 'access method'
                        WHEN 'pg_aggregate'::regclass THEN 'aggregate'
                        WHEN 'pg_collation'::regclass THEN 'collation'
                        WHEN 'pg_conversion'::regclass THEN 'conversion'
                        WHEN 'pg_ts_config'::regclass THEN 'text search config'
                        WHEN 'pg_ts_dict'::regclass THEN 'text search dictionary'
                        WHEN 'pg_ts_parser'::regclass THEN 'text search parser'
                        WHEN 'pg_ts_template'::regclass THEN 'text search template'
                        ELSE 'other'
                    END AS object_type,
                    COALESCE(n.nspname, '') AS schema,
                    pg_describe_object(classid, objid, objsubid) AS identity
                FROM pg_depend d
                JOIN pg_extension e ON e.oid = d.refobjid
                LEFT JOIN pg_class c ON c.oid = d.objid
                LEFT JOIN pg_namespace n ON n.oid = c.relnamespace
                WHERE e.extname = $1
                  AND d.deptype = 'e'
                ORDER BY object_type, identity
                "#,
                &[&extension_name],
            )
            .await?;

        let objects = object_rows
            .iter()
            .map(|row| {
                let identity: String = row.get("identity");
                let name = identity
                    .split('.')
                    .last()
                    .unwrap_or(&identity)
                    .split('(')
                    .next()
                    .unwrap_or(&identity)
                    .to_string();

                ExtensionObject {
                    object_type: row.get("object_type"),
                    schema: row.get("schema"),
                    name,
                    identity,
                }
            })
            .collect();

        // Get extension configuration parameters
        let config = Self::get_extension_config(&client, &name)
            .await
            .unwrap_or_default();

        Ok(ExtensionDetail {
            name,
            version,
            schema,
            description,
            requires,
            objects,
            config,
        })
    }

    /// Get configuration parameters for an extension
    async fn get_extension_config(
        client: &tokio_postgres::Client,
        extension_name: &str,
    ) -> Result<Vec<ExtensionConfig>, ExtensionError> {
        let rows = client
            .query(
                r#"
                SELECT
                    name,
                    setting AS value,
                    short_desc AS description,
                    unit,
                    vartype,
                    enumvals AS enum_vals,
                    min_val,
                    max_val
                FROM pg_settings
                WHERE name LIKE $1 || '.%'
                ORDER BY name
                "#,
                &[&extension_name],
            )
            .await?;

        let config = rows
            .iter()
            .map(|row| ExtensionConfig {
                name: row.get("name"),
                value: row.get("value"),
                description: row
                    .get::<_, Option<String>>("description")
                    .unwrap_or_default(),
                unit: row.get("unit"),
                vartype: row.get("vartype"),
                enum_vals: row.get("enum_vals"),
                min_val: row.get("min_val"),
                max_val: row.get("max_val"),
            })
            .collect();

        Ok(config)
    }

    /// Install an extension
    pub async fn install_extension(
        pool: &Pool,
        options: &InstallExtensionOptions,
    ) -> Result<(), ExtensionError> {
        let client = pool.get().await?;
        let sql = Self::build_install_sql(options);
        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Build CREATE EXTENSION SQL
    pub fn build_install_sql(options: &InstallExtensionOptions) -> String {
        let mut sql = format!(
            "CREATE EXTENSION IF NOT EXISTS {}",
            quote_ident(&options.name)
        );

        if let Some(ref schema) = options.schema {
            sql.push_str(&format!(" SCHEMA {}", quote_ident(schema)));
        }

        if let Some(ref version) = options.version {
            sql.push_str(&format!(" VERSION '{}'", escape_string(version)));
        }

        if options.cascade {
            sql.push_str(" CASCADE");
        }

        sql
    }

    /// Upgrade an extension
    pub async fn upgrade_extension(
        pool: &Pool,
        options: &UpgradeExtensionOptions,
    ) -> Result<(), ExtensionError> {
        let client = pool.get().await?;
        let sql = Self::build_upgrade_sql(options);
        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Build ALTER EXTENSION UPDATE SQL
    pub fn build_upgrade_sql(options: &UpgradeExtensionOptions) -> String {
        let mut sql = format!(
            "ALTER EXTENSION {} UPDATE",
            quote_ident(&options.name)
        );

        if let Some(ref version) = options.target_version {
            sql.push_str(&format!(" TO '{}'", escape_string(version)));
        }

        sql
    }

    /// Uninstall an extension
    pub async fn uninstall_extension(
        pool: &Pool,
        extension_name: &str,
        cascade: bool,
    ) -> Result<(), ExtensionError> {
        let client = pool.get().await?;
        let sql = Self::build_uninstall_sql(extension_name, cascade);
        client.execute(&sql, &[]).await?;
        Ok(())
    }

    /// Build DROP EXTENSION SQL
    pub fn build_uninstall_sql(extension_name: &str, cascade: bool) -> String {
        let mut sql = format!(
            "DROP EXTENSION IF EXISTS {}",
            quote_ident(extension_name)
        );

        if cascade {
            sql.push_str(" CASCADE");
        }

        sql
    }

    /// Check if an extension is installed
    pub async fn is_installed(pool: &Pool, extension_name: &str) -> Result<bool, ExtensionError> {
        let client = pool.get().await?;

        let row = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = $1)",
                &[&extension_name],
            )
            .await?;

        Ok(row.get(0))
    }

    /// Get extensions that depend on a given extension
    pub async fn get_dependent_extensions(
        pool: &Pool,
        extension_name: &str,
    ) -> Result<Vec<String>, ExtensionError> {
        let client = pool.get().await?;

        let rows = client
            .query(
                r#"
                SELECT DISTINCT e2.extname
                FROM pg_extension e1
                JOIN pg_depend d ON d.refobjid = e1.oid
                JOIN pg_extension e2 ON e2.oid = d.objid
                WHERE e1.extname = $1
                  AND d.deptype = 'n'
                "#,
                &[&extension_name],
            )
            .await?;

        let deps = rows.iter().map(|row| row.get("extname")).collect();
        Ok(deps)
    }
}

/// Quote an identifier for safe use in SQL
fn quote_ident(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !s.is_empty()
        && !s.chars().next().unwrap().is_ascii_digit()
    {
        s.to_string()
    } else {
        format!("\"{}\"", s.replace('"', "\"\""))
    }
}

/// Escape a string for safe use in SQL
fn escape_string(s: &str) -> String {
    s.replace('\'', "''")
}
```

### 23.3 Extension State (GPUI Global)

```rust
// src/state/extension_state.rs

use crate::models::extension::{
    Extension, ExtensionDetail, InstallExtensionOptions, UpgradeExtensionOptions,
};
use crate::services::extension::{ExtensionError, ExtensionService};
use deadpool_postgres::Pool;
use gpui::Global;
use parking_lot::RwLock;
use std::sync::Arc;

/// Application-wide extension management state
pub struct ExtensionState {
    inner: Arc<RwLock<ExtensionStateInner>>,
}

struct ExtensionStateInner {
    /// All available and installed extensions
    extensions: Vec<Extension>,

    /// Currently selected extension (for detail view)
    selected_extension: Option<String>,

    /// Detail of selected extension (if installed)
    extension_detail: Option<ExtensionDetail>,

    /// Filter text
    filter: String,

    /// Show only installed extensions
    show_installed_only: bool,

    /// Loading state
    loading: bool,

    /// Error message
    error: Option<String>,

    /// Connection pool reference
    pool: Option<Pool>,
}

impl Global for ExtensionState {}

impl ExtensionState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(ExtensionStateInner {
                extensions: Vec::new(),
                selected_extension: None,
                extension_detail: None,
                filter: String::new(),
                show_installed_only: false,
                loading: false,
                error: None,
                pool: None,
            })),
        }
    }

    /// Set the connection pool
    pub fn set_pool(&self, pool: Pool) {
        self.inner.write().pool = Some(pool);
    }

    /// Get all extensions (unfiltered)
    pub fn extensions(&self) -> Vec<Extension> {
        self.inner.read().extensions.clone()
    }

    /// Get filtered extensions based on current filter settings
    pub fn filtered_extensions(&self) -> Vec<Extension> {
        let inner = self.inner.read();
        let filter = inner.filter.to_lowercase();

        inner
            .extensions
            .iter()
            .filter(|ext| {
                // Filter by installed status
                if inner.show_installed_only && !ext.is_installed {
                    return false;
                }

                // Filter by search text
                if !filter.is_empty() {
                    let name_match = ext.name.to_lowercase().contains(&filter);
                    let comment_match = ext
                        .comment
                        .as_ref()
                        .map(|c| c.to_lowercase().contains(&filter))
                        .unwrap_or(false);

                    if !name_match && !comment_match {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect()
    }

    /// Get installed extensions count
    pub fn installed_count(&self) -> usize {
        self.inner
            .read()
            .extensions
            .iter()
            .filter(|e| e.is_installed)
            .count()
    }

    /// Get selected extension name
    pub fn selected_extension(&self) -> Option<String> {
        self.inner.read().selected_extension.clone()
    }

    /// Get extension detail
    pub fn extension_detail(&self) -> Option<ExtensionDetail> {
        self.inner.read().extension_detail.clone()
    }

    /// Get filter text
    pub fn filter(&self) -> String {
        self.inner.read().filter.clone()
    }

    /// Set filter text
    pub fn set_filter(&self, filter: String) {
        self.inner.write().filter = filter;
    }

    /// Get show installed only flag
    pub fn show_installed_only(&self) -> bool {
        self.inner.read().show_installed_only
    }

    /// Set show installed only flag
    pub fn set_show_installed_only(&self, value: bool) {
        self.inner.write().show_installed_only = value;
    }

    /// Check if loading
    pub fn is_loading(&self) -> bool {
        self.inner.read().loading
    }

    /// Get error message
    pub fn error(&self) -> Option<String> {
        self.inner.read().error.clone()
    }

    /// Clear error
    pub fn clear_error(&self) {
        self.inner.write().error = None;
    }

    /// Load all extensions
    pub async fn load_extensions(&self) -> Result<(), ExtensionError> {
        let pool = {
            let inner = self.inner.read();
            inner.pool.clone().ok_or_else(|| {
                ExtensionError::Database(tokio_postgres::Error::__private_api_error(
                    "No connection pool",
                ))
            })?
        };

        self.inner.write().loading = true;
        self.inner.write().error = None;

        match ExtensionService::get_extensions(&pool).await {
            Ok(extensions) => {
                let mut inner = self.inner.write();
                inner.extensions = extensions;
                inner.loading = false;
                Ok(())
            }
            Err(e) => {
                let mut inner = self.inner.write();
                inner.loading = false;
                inner.error = Some(e.to_string());
                Err(e)
            }
        }
    }

    /// Select an extension for viewing
    pub async fn select_extension(&self, name: &str) -> Result<(), ExtensionError> {
        let pool = {
            let inner = self.inner.read();
            inner.pool.clone()
        };

        self.inner.write().selected_extension = Some(name.to_string());

        // Check if installed
        let is_installed = self
            .inner
            .read()
            .extensions
            .iter()
            .find(|e| e.name == name)
            .map(|e| e.is_installed)
            .unwrap_or(false);

        if is_installed {
            if let Some(pool) = pool {
                match ExtensionService::get_extension_detail(&pool, name).await {
                    Ok(detail) => {
                        self.inner.write().extension_detail = Some(detail);
                    }
                    Err(e) => {
                        self.inner.write().error = Some(e.to_string());
                        return Err(e);
                    }
                }
            }
        } else {
            self.inner.write().extension_detail = None;
        }

        Ok(())
    }

    /// Clear selection
    pub fn clear_selection(&self) {
        let mut inner = self.inner.write();
        inner.selected_extension = None;
        inner.extension_detail = None;
    }

    /// Get available versions for an extension
    pub async fn get_available_versions(&self, name: &str) -> Result<Vec<String>, ExtensionError> {
        let pool = {
            let inner = self.inner.read();
            inner.pool.clone().ok_or_else(|| {
                ExtensionError::NotFound("No connection pool".to_string())
            })?
        };

        ExtensionService::get_available_versions(&pool, name).await
    }

    /// Install an extension
    pub async fn install_extension(
        &self,
        options: &InstallExtensionOptions,
    ) -> Result<(), ExtensionError> {
        let pool = {
            let inner = self.inner.read();
            inner.pool.clone().ok_or_else(|| {
                ExtensionError::NotFound("No connection pool".to_string())
            })?
        };

        ExtensionService::install_extension(&pool, options).await?;
        self.load_extensions().await?;

        // If this was the selected extension, reload its detail
        if self.inner.read().selected_extension.as_deref() == Some(&options.name) {
            self.select_extension(&options.name).await?;
        }

        Ok(())
    }

    /// Upgrade an extension
    pub async fn upgrade_extension(
        &self,
        options: &UpgradeExtensionOptions,
    ) -> Result<(), ExtensionError> {
        let pool = {
            let inner = self.inner.read();
            inner.pool.clone().ok_or_else(|| {
                ExtensionError::NotFound("No connection pool".to_string())
            })?
        };

        ExtensionService::upgrade_extension(&pool, options).await?;
        self.load_extensions().await?;

        // Reload detail if this was the selected extension
        if self.inner.read().selected_extension.as_deref() == Some(&options.name) {
            self.select_extension(&options.name).await?;
        }

        Ok(())
    }

    /// Uninstall an extension
    pub async fn uninstall_extension(
        &self,
        name: &str,
        cascade: bool,
    ) -> Result<(), ExtensionError> {
        let pool = {
            let inner = self.inner.read();
            inner.pool.clone().ok_or_else(|| {
                ExtensionError::NotFound("No connection pool".to_string())
            })?
        };

        ExtensionService::uninstall_extension(&pool, name, cascade).await?;

        // Clear detail if this was the selected extension
        if self.inner.read().selected_extension.as_deref() == Some(name) {
            self.inner.write().extension_detail = None;
        }

        self.load_extensions().await
    }

    /// Generate install SQL
    pub fn generate_install_sql(&self, options: &InstallExtensionOptions) -> String {
        ExtensionService::build_install_sql(options)
    }

    /// Generate upgrade SQL
    pub fn generate_upgrade_sql(&self, options: &UpgradeExtensionOptions) -> String {
        ExtensionService::build_upgrade_sql(options)
    }

    /// Generate uninstall SQL
    pub fn generate_uninstall_sql(&self, name: &str, cascade: bool) -> String {
        ExtensionService::build_uninstall_sql(name, cascade)
    }
}

impl Default for ExtensionState {
    fn default() -> Self {
        Self::new()
    }
}
```

### 23.4 Extension List View

```rust
// src/components/extensions/extension_list.rs

use crate::models::extension::Extension;
use crate::state::extension_state::ExtensionState;
use crate::ui::{
    Button, ButtonVariant, Checkbox, Icon, IconName, Input, ScrollView, Table,
    TableColumn, TableRow, Tooltip,
};
use gpui::{
    div, px, AppContext, Context, Element, EventEmitter, FocusHandle, FocusableView,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled,
    View, ViewContext, VisualContext,
};

/// Events emitted by the extension list
pub enum ExtensionListEvent {
    Select(String),
    Install(Extension),
    Upgrade(Extension),
    Uninstall(String),
}

pub struct ExtensionListView {
    focus_handle: FocusHandle,
}

impl EventEmitter<ExtensionListEvent> for ExtensionListView {}

impl ExtensionListView {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    fn handle_filter_change(&mut self, text: String, cx: &mut ViewContext<Self>) {
        let ext_state = cx.global::<ExtensionState>();
        ext_state.set_filter(text);
        cx.notify();
    }

    fn toggle_installed_filter(&mut self, cx: &mut ViewContext<Self>) {
        let ext_state = cx.global::<ExtensionState>();
        let current = ext_state.show_installed_only();
        ext_state.set_show_installed_only(!current);
        cx.notify();
    }

    fn handle_refresh(&mut self, cx: &mut ViewContext<Self>) {
        let ext_state = cx.global::<ExtensionState>().clone();
        cx.spawn(|_, _| async move {
            let _ = ext_state.load_extensions().await;
        })
        .detach();
    }

    fn handle_select(&mut self, name: String, cx: &mut ViewContext<Self>) {
        cx.emit(ExtensionListEvent::Select(name.clone()));

        let ext_state = cx.global::<ExtensionState>().clone();
        cx.spawn(|_, _| async move {
            let _ = ext_state.select_extension(&name).await;
        })
        .detach();
    }

    fn handle_install(&mut self, ext: Extension, cx: &mut ViewContext<Self>) {
        cx.emit(ExtensionListEvent::Install(ext));
    }

    fn handle_upgrade(&mut self, ext: Extension, cx: &mut ViewContext<Self>) {
        cx.emit(ExtensionListEvent::Upgrade(ext));
    }

    fn handle_uninstall(&mut self, name: String, cx: &mut ViewContext<Self>) {
        cx.emit(ExtensionListEvent::Uninstall(name));
    }
}

impl FocusableView for ExtensionListView {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ExtensionListView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let ext_state = cx.global::<ExtensionState>();
        let extensions = ext_state.filtered_extensions();
        let filter = ext_state.filter();
        let show_installed_only = ext_state.show_installed_only();
        let is_loading = ext_state.is_loading();
        let selected = ext_state.selected_extension();
        let installed_count = ext_state.installed_count();
        let total_count = ext_state.extensions().len();

        div()
            .flex()
            .flex_col()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(
                // Toolbar
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_4()
                    .border_b_1()
                    .border_color(gpui::rgb(0xe5e7eb))
                    .child(
                        Input::new("search-extensions")
                            .placeholder("Search extensions...")
                            .value(filter)
                            .on_change(cx.listener(|this, text: &String, cx| {
                                this.handle_filter_change(text.clone(), cx);
                            }))
                            .flex_1(),
                    )
                    .child(
                        Checkbox::new("installed-only")
                            .checked(show_installed_only)
                            .label("Installed only")
                            .on_toggle(cx.listener(|this, _, cx| {
                                this.toggle_installed_filter(cx);
                            })),
                    )
                    .child(
                        Button::new("refresh")
                            .icon(IconName::Refresh)
                            .variant(ButtonVariant::Ghost)
                            .tooltip("Refresh extensions")
                            .on_click(cx.listener(|this, _, cx| {
                                this.handle_refresh(cx);
                            })),
                    ),
            )
            .child(
                // Stats bar
                div()
                    .px_4()
                    .py_2()
                    .bg(gpui::rgb(0xf9fafb))
                    .border_b_1()
                    .border_color(gpui::rgb(0xe5e7eb))
                    .text_sm()
                    .text_color(gpui::rgb(0x6b7280))
                    .child(format!(
                        "{} installed of {} available",
                        installed_count, total_count
                    )),
            )
            .child(
                // Extension table
                ScrollView::new("extension-list-scroll")
                    .flex_1()
                    .child(
                        Table::new("extensions-table")
                            .header(vec![
                                TableColumn::new("Extension").flex(1.0),
                                TableColumn::new("Version").width(px(150.0)).center(),
                                TableColumn::new("Schema").width(px(120.0)),
                                TableColumn::new("Actions").width(px(120.0)).right(),
                            ])
                            .loading(is_loading)
                            .empty_message(if filter.is_empty() {
                                "No extensions found"
                            } else {
                                "No extensions match the filter"
                            })
                            .children(extensions.iter().map(|ext| {
                                let name = ext.name.clone();
                                let is_selected = selected.as_ref() == Some(&name);
                                let ext_clone = ext.clone();
                                let name_for_select = name.clone();
                                let name_for_uninstall = name.clone();

                                TableRow::new(format!("ext-{}", name))
                                    .selected(is_selected)
                                    .on_click(cx.listener(move |this, _, cx| {
                                        this.handle_select(name_for_select.clone(), cx);
                                    }))
                                    .child(
                                        // Extension name column
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .w(px(8.0))
                                                    .h(px(8.0))
                                                    .rounded_full()
                                                    .bg(if ext.is_installed {
                                                        gpui::rgb(0x22c55e)
                                                    } else {
                                                        gpui::rgb(0xd1d5db)
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .child(
                                                        div()
                                                            .font_weight(gpui::FontWeight::MEDIUM)
                                                            .child(ext.name.clone()),
                                                    )
                                                    .when_some(
                                                        ext.comment.as_ref(),
                                                        |this, comment| {
                                                            this.child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(gpui::rgb(0x6b7280))
                                                                    .truncate()
                                                                    .max_w(px(300.0))
                                                                    .child(comment.clone()),
                                                            )
                                                        },
                                                    ),
                                            ),
                                    )
                                    .child(
                                        // Version column
                                        div()
                                            .flex()
                                            .justify_center()
                                            .items_center()
                                            .gap_1()
                                            .child(if ext.is_installed {
                                                div()
                                                    .font_family("monospace")
                                                    .text_sm()
                                                    .child(
                                                        ext.installed_version
                                                            .clone()
                                                            .unwrap_or_default(),
                                                    )
                                            } else {
                                                div()
                                                    .text_color(gpui::rgb(0x9ca3af))
                                                    .text_sm()
                                                    .child(ext.default_version.clone())
                                            })
                                            .when(ext.has_upgrade(), |this| {
                                                this.child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(gpui::rgb(0xd97706))
                                                        .child(format!(
                                                            "→ {}",
                                                            ext.default_version
                                                        )),
                                                )
                                            }),
                                    )
                                    .child(
                                        // Schema column
                                        if let Some(ref schema) = ext.schema {
                                            div()
                                                .font_family("monospace")
                                                .text_xs()
                                                .px_1()
                                                .py_px()
                                                .bg(gpui::rgb(0xf3f4f6))
                                                .rounded(px(4.0))
                                                .child(schema.clone())
                                        } else {
                                            div()
                                                .text_color(gpui::rgb(0x9ca3af))
                                                .child("-")
                                        },
                                    )
                                    .child(
                                        // Actions column
                                        div()
                                            .flex()
                                            .justify_end()
                                            .gap_1()
                                            .when(ext.is_installed && ext.has_upgrade(), |this| {
                                                let ext_for_upgrade = ext_clone.clone();
                                                this.child(
                                                    Tooltip::new("Upgrade extension").child(
                                                        Button::new(format!("upgrade-{}", name))
                                                            .icon(IconName::ArrowUp)
                                                            .variant(ButtonVariant::Ghost)
                                                            .small()
                                                            .on_click(cx.listener(
                                                                move |this, _, cx| {
                                                                    this.handle_upgrade(
                                                                        ext_for_upgrade.clone(),
                                                                        cx,
                                                                    );
                                                                },
                                                            )),
                                                    ),
                                                )
                                            })
                                            .child(if ext.is_installed {
                                                Tooltip::new("Uninstall extension").child(
                                                    Button::new(format!("uninstall-{}", name))
                                                        .icon(IconName::Trash)
                                                        .variant(ButtonVariant::Ghost)
                                                        .small()
                                                        .danger()
                                                        .on_click(cx.listener(
                                                            move |this, _, cx| {
                                                                this.handle_uninstall(
                                                                    name_for_uninstall.clone(),
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                )
                                            } else {
                                                let ext_for_install = ext_clone.clone();
                                                Tooltip::new("Install extension").child(
                                                    Button::new(format!("install-{}", name))
                                                        .icon(IconName::Download)
                                                        .variant(ButtonVariant::Ghost)
                                                        .small()
                                                        .on_click(cx.listener(
                                                            move |this, _, cx| {
                                                                this.handle_install(
                                                                    ext_for_install.clone(),
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                )
                                            }),
                                    )
                            })),
                    ),
            )
    }
}
```

### 23.5 Extension Detail Panel

```rust
// src/components/extensions/extension_detail.rs

use crate::models::extension::ExtensionDetail;
use crate::state::extension_state::ExtensionState;
use crate::ui::{
    Button, ButtonVariant, EmptyState, Icon, IconName, ScrollView, TabBar, TabItem,
};
use gpui::{
    div, px, AppContext, Context, Element, EventEmitter, FocusHandle, FocusableView,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled,
    View, ViewContext, VisualContext,
};

/// Events emitted by the detail panel
pub enum ExtensionDetailEvent {
    Upgrade(String),
    Uninstall(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DetailTab {
    Objects,
    Config,
}

pub struct ExtensionDetailPanel {
    focus_handle: FocusHandle,
    active_tab: DetailTab,
}

impl EventEmitter<ExtensionDetailEvent> for ExtensionDetailPanel {}

impl ExtensionDetailPanel {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            active_tab: DetailTab::Objects,
        }
    }

    fn set_tab(&mut self, tab: DetailTab, cx: &mut ViewContext<Self>) {
        self.active_tab = tab;
        cx.notify();
    }

    fn handle_upgrade(&mut self, name: String, cx: &mut ViewContext<Self>) {
        cx.emit(ExtensionDetailEvent::Upgrade(name));
    }

    fn handle_uninstall(&mut self, name: String, cx: &mut ViewContext<Self>) {
        cx.emit(ExtensionDetailEvent::Uninstall(name));
    }

    fn render_objects_tab(&self, detail: &ExtensionDetail) -> impl IntoElement {
        let objects_by_type = detail.objects_by_type();

        if detail.objects.is_empty() {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    EmptyState::new("no-objects")
                        .icon(IconName::Package)
                        .title("No objects")
                        .description("This extension has no objects"),
                );
        }

        ScrollView::new("objects-scroll")
            .p_4()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .children(objects_by_type.into_iter().map(|(obj_type, objects)| {
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(gpui::rgb(0x374151))
                                    .child(format!(
                                        "{}s ({})",
                                        capitalize(&obj_type),
                                        objects.len()
                                    )),
                            )
                            .child(
                                div()
                                    .bg(gpui::rgb(0xf9fafb))
                                    .rounded(px(6.0))
                                    .p_2()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .children(objects.into_iter().map(|obj| {
                                        div()
                                            .text_sm()
                                            .font_family("monospace")
                                            .py_1()
                                            .px_2()
                                            .rounded(px(4.0))
                                            .hover(|s| s.bg(gpui::rgb(0xf3f4f6)))
                                            .child(obj.identity.clone())
                                    })),
                            )
                    })),
            )
    }

    fn render_config_tab(&self, detail: &ExtensionDetail) -> impl IntoElement {
        if detail.config.is_empty() {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    EmptyState::new("no-config")
                        .icon(IconName::Settings)
                        .title("No configuration")
                        .description("This extension has no configuration parameters"),
                );
        }

        ScrollView::new("config-scroll")
            .p_4()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .children(detail.config.iter().map(|param| {
                        div()
                            .bg(gpui::rgb(0xf9fafb))
                            .rounded(px(6.0))
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .font_family("monospace")
                                            .text_sm()
                                            .child(param.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .font_family("monospace")
                                            .text_sm()
                                            .text_color(gpui::rgb(0x2563eb))
                                            .child(param.display_value()),
                                    ),
                            )
                            .when(!param.description.is_empty(), |this| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(gpui::rgb(0x6b7280))
                                        .child(param.description.clone()),
                                )
                            })
                    })),
            )
    }
}

impl FocusableView for ExtensionDetailPanel {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ExtensionDetailPanel {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let ext_state = cx.global::<ExtensionState>();
        let detail = ext_state.extension_detail();
        let selected = ext_state.selected_extension();

        div()
            .flex()
            .flex_col()
            .size_full()
            .track_focus(&self.focus_handle)
            .when(detail.is_none(), |this| {
                this.child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            EmptyState::new("no-selection")
                                .icon(IconName::Puzzle)
                                .title(if selected.is_some() {
                                    "Extension not installed"
                                } else {
                                    "No extension selected"
                                })
                                .description(if selected.is_some() {
                                    "Install the extension to view its details"
                                } else {
                                    "Select an extension from the list"
                                }),
                        ),
                )
            })
            .when_some(detail.clone(), |this, detail| {
                let name_for_upgrade = detail.name.clone();
                let name_for_uninstall = detail.name.clone();

                this.child(
                    // Header
                    div()
                        .p_4()
                        .border_b_1()
                        .border_color(gpui::rgb(0xe5e7eb))
                        .child(
                            div()
                                .flex()
                                .items_start()
                                .justify_between()
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_lg()
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .child(detail.name.clone()),
                                        )
                                        .when(!detail.description.is_empty(), |this| {
                                            this.child(
                                                div()
                                                    .text_sm()
                                                    .text_color(gpui::rgb(0x6b7280))
                                                    .child(detail.description.clone()),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .gap_2()
                                        .child(
                                            Button::new("upgrade-btn")
                                                .label("Upgrade")
                                                .variant(ButtonVariant::Secondary)
                                                .small()
                                                .on_click(cx.listener(
                                                    move |this, _, cx| {
                                                        this.handle_upgrade(
                                                            name_for_upgrade.clone(),
                                                            cx,
                                                        );
                                                    },
                                                )),
                                        )
                                        .child(
                                            Button::new("uninstall-btn")
                                                .label("Uninstall")
                                                .variant(ButtonVariant::Danger)
                                                .small()
                                                .on_click(cx.listener(
                                                    move |this, _, cx| {
                                                        this.handle_uninstall(
                                                            name_for_uninstall.clone(),
                                                            cx,
                                                        );
                                                    },
                                                )),
                                        ),
                                ),
                        )
                        .child(
                            // Info row
                            div()
                                .flex()
                                .items_center()
                                .gap_6()
                                .mt_3()
                                .text_sm()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_color(gpui::rgb(0x6b7280))
                                                .child("Version:"),
                                        )
                                        .child(
                                            div()
                                                .font_family("monospace")
                                                .child(detail.version.clone()),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_color(gpui::rgb(0x6b7280))
                                                .child("Schema:"),
                                        )
                                        .child(
                                            div()
                                                .font_family("monospace")
                                                .child(detail.schema.clone()),
                                        ),
                                )
                                .when(!detail.requires.is_empty(), |this| {
                                    this.child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_color(gpui::rgb(0x6b7280))
                                                    .child("Requires:"),
                                            )
                                            .child(
                                                div().child(detail.requires.join(", ")),
                                            ),
                                    )
                                }),
                        ),
                )
                .child(
                    // Tabs
                    TabBar::new("detail-tabs")
                        .child(
                            TabItem::new("objects")
                                .label(format!("Objects ({})", detail.object_count()))
                                .selected(self.active_tab == DetailTab::Objects)
                                .on_click(cx.listener(|this, _, cx| {
                                    this.set_tab(DetailTab::Objects, cx);
                                })),
                        )
                        .child(
                            TabItem::new("config")
                                .label(format!("Configuration ({})", detail.config.len()))
                                .selected(self.active_tab == DetailTab::Config)
                                .on_click(cx.listener(|this, _, cx| {
                                    this.set_tab(DetailTab::Config, cx);
                                })),
                        ),
                )
                .child(
                    // Tab content
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .child(match self.active_tab {
                            DetailTab::Objects => self.render_objects_tab(&detail),
                            DetailTab::Config => self.render_config_tab(&detail),
                        }),
                )
            })
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(c).collect(),
    }
}
```

### 23.6 Install Extension Dialog

```rust
// src/components/extensions/install_dialog.rs

use crate::models::extension::{Extension, InstallExtensionOptions};
use crate::state::extension_state::ExtensionState;
use crate::ui::{
    Button, ButtonVariant, Checkbox, Modal, ModalFooter, Section, Select, SelectOption,
};
use gpui::{
    div, px, AppContext, Context, Element, EventEmitter, FocusHandle, FocusableView,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled,
    View, ViewContext, VisualContext,
};

/// Events emitted by the install dialog
pub enum InstallDialogEvent {
    Installed,
    Cancelled,
}

pub struct InstallExtensionDialog {
    focus_handle: FocusHandle,
    extension: Extension,
    available_versions: Vec<String>,
    schemas: Vec<String>,
    selected_version: String,
    selected_schema: String,
    cascade: bool,
    installing: bool,
    error: Option<String>,
}

impl EventEmitter<InstallDialogEvent> for InstallExtensionDialog {}

impl InstallExtensionDialog {
    pub fn new(extension: Extension, schemas: Vec<String>, cx: &mut ViewContext<Self>) -> Self {
        let selected_version = extension.default_version.clone();
        let ext_name = extension.name.clone();

        let mut dialog = Self {
            focus_handle: cx.focus_handle(),
            extension,
            available_versions: vec![selected_version.clone()],
            schemas,
            selected_version,
            selected_schema: "public".to_string(),
            cascade: true,
            installing: false,
            error: None,
        };

        // Load available versions
        let ext_state = cx.global::<ExtensionState>().clone();
        cx.spawn(|this, mut cx| async move {
            if let Ok(versions) = ext_state.get_available_versions(&ext_name).await {
                this.update(&mut cx, |this, cx| {
                    this.available_versions = versions;
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();

        dialog
    }

    fn generate_sql(&self) -> String {
        let options = InstallExtensionOptions {
            name: self.extension.name.clone(),
            version: if self.selected_version != self.extension.default_version {
                Some(self.selected_version.clone())
            } else {
                None
            },
            schema: if self.selected_schema != "public" {
                Some(self.selected_schema.clone())
            } else {
                None
            },
            cascade: self.cascade,
        };

        let ext_state = ExtensionState::new(); // Use default for SQL generation
        ext_state.generate_install_sql(&options)
    }

    fn handle_install(&mut self, cx: &mut ViewContext<Self>) {
        self.installing = true;
        self.error = None;
        cx.notify();

        let options = InstallExtensionOptions {
            name: self.extension.name.clone(),
            version: if self.selected_version != self.extension.default_version {
                Some(self.selected_version.clone())
            } else {
                None
            },
            schema: if self.selected_schema != "public" {
                Some(self.selected_schema.clone())
            } else {
                None
            },
            cascade: self.cascade,
        };

        let ext_state = cx.global::<ExtensionState>().clone();

        cx.spawn(|this, mut cx| async move {
            match ext_state.install_extension(&options).await {
                Ok(()) => {
                    this.update(&mut cx, |_, cx| {
                        cx.emit(InstallDialogEvent::Installed);
                    })
                    .ok();
                }
                Err(e) => {
                    this.update(&mut cx, |this, cx| {
                        this.installing = false;
                        this.error = Some(e.to_string());
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    fn handle_cancel(&mut self, cx: &mut ViewContext<Self>) {
        cx.emit(InstallDialogEvent::Cancelled);
    }
}

impl FocusableView for InstallExtensionDialog {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for InstallExtensionDialog {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let generated_sql = self.generate_sql();

        Modal::new("install-extension")
            .title(format!("Install Extension: {}", self.extension.name))
            .width(px(500.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_4()
                    // Description
                    .when_some(self.extension.comment.as_ref(), |this, comment| {
                        this.child(
                            div()
                                .text_sm()
                                .text_color(gpui::rgb(0x6b7280))
                                .child(comment.clone()),
                        )
                    })
                    // Dependencies warning
                    .when(!self.extension.requires.is_empty(), |this| {
                        this.child(
                            div()
                                .p_3()
                                .bg(gpui::rgb(0xeff6ff))
                                .border_1()
                                .border_color(gpui::rgb(0xbfdbfe))
                                .rounded(px(6.0))
                                .text_sm()
                                .child(
                                    div()
                                        .child(SharedString::from("Requires: "))
                                        .child(self.extension.requires.join(", ")),
                                ),
                        )
                    })
                    // Error message
                    .when_some(self.error.clone(), |this, error| {
                        this.child(
                            div()
                                .p_3()
                                .bg(gpui::rgb(0xfef2f2))
                                .border_1()
                                .border_color(gpui::rgb(0xfecaca))
                                .rounded(px(6.0))
                                .text_sm()
                                .text_color(gpui::rgb(0xb91c1c))
                                .child(error),
                        )
                    })
                    // Version selection
                    .child(
                        Section::new("version")
                            .label("Version")
                            .child(
                                Select::new("version-select")
                                    .value(Some(SharedString::from(self.selected_version.clone())))
                                    .options(
                                        self.available_versions
                                            .iter()
                                            .map(|v| {
                                                let label = if v == &self.extension.default_version {
                                                    format!("{} (default)", v)
                                                } else {
                                                    v.clone()
                                                };
                                                SelectOption::new(v.clone(), label)
                                            })
                                            .collect(),
                                    )
                                    .on_change(cx.listener(
                                        |this, value: &Option<SharedString>, cx| {
                                            if let Some(v) = value {
                                                this.selected_version = v.to_string();
                                                cx.notify();
                                            }
                                        },
                                    )),
                            ),
                    )
                    // Schema selection
                    .child(
                        Section::new("schema")
                            .label("Schema")
                            .when(!self.extension.relocatable, |this| {
                                this.hint(
                                    "This extension is not relocatable and will use its default schema",
                                )
                            })
                            .child(
                                Select::new("schema-select")
                                    .value(Some(SharedString::from(self.selected_schema.clone())))
                                    .disabled(!self.extension.relocatable)
                                    .options(
                                        self.schemas
                                            .iter()
                                            .map(|s| SelectOption::new(s.clone(), s.clone()))
                                            .collect(),
                                    )
                                    .on_change(cx.listener(
                                        |this, value: &Option<SharedString>, cx| {
                                            if let Some(v) = value {
                                                this.selected_schema = v.to_string();
                                                cx.notify();
                                            }
                                        },
                                    )),
                            ),
                    )
                    // Cascade option
                    .child(
                        Checkbox::new("cascade")
                            .checked(self.cascade)
                            .label("CASCADE - Automatically install required dependencies")
                            .on_toggle(cx.listener(|this, _, cx| {
                                this.cascade = !this.cascade;
                                cx.notify();
                            })),
                    )
                    // SQL preview
                    .child(
                        Section::new("sql")
                            .label("Generated SQL")
                            .child(
                                div()
                                    .p_3()
                                    .bg(gpui::rgb(0xf3f4f6))
                                    .rounded(px(6.0))
                                    .font_family("monospace")
                                    .text_xs()
                                    .overflow_x_auto()
                                    .child(generated_sql),
                            ),
                    ),
            )
            .footer(
                ModalFooter::new().right(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            Button::new("cancel")
                                .label("Cancel")
                                .variant(ButtonVariant::Secondary)
                                .on_click(cx.listener(|this, _, cx| {
                                    this.handle_cancel(cx);
                                })),
                        )
                        .child(
                            Button::new("install")
                                .label(if self.installing {
                                    "Installing..."
                                } else {
                                    "Install"
                                })
                                .variant(ButtonVariant::Primary)
                                .disabled(self.installing)
                                .on_click(cx.listener(|this, _, cx| {
                                    this.handle_install(cx);
                                })),
                        ),
                ),
            )
    }
}
```

### 23.7 Uninstall Extension Dialog

```rust
// src/components/extensions/uninstall_dialog.rs

use crate::state::extension_state::ExtensionState;
use crate::ui::{Button, ButtonVariant, Checkbox, Modal, ModalFooter, Section};
use gpui::{
    div, px, AppContext, Context, Element, EventEmitter, FocusHandle, FocusableView,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled,
    View, ViewContext, VisualContext,
};

/// Events emitted by the uninstall dialog
pub enum UninstallDialogEvent {
    Uninstalled,
    Cancelled,
}

pub struct UninstallExtensionDialog {
    focus_handle: FocusHandle,
    extension_name: String,
    cascade: bool,
    uninstalling: bool,
    error: Option<String>,
}

impl EventEmitter<UninstallDialogEvent> for UninstallExtensionDialog {}

impl UninstallExtensionDialog {
    pub fn new(extension_name: String, cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            extension_name,
            cascade: false, // Default to no cascade for safety
            uninstalling: false,
            error: None,
        }
    }

    fn generate_sql(&self, ext_state: &ExtensionState) -> String {
        ext_state.generate_uninstall_sql(&self.extension_name, self.cascade)
    }

    fn handle_uninstall(&mut self, cx: &mut ViewContext<Self>) {
        self.uninstalling = true;
        self.error = None;
        cx.notify();

        let ext_state = cx.global::<ExtensionState>().clone();
        let name = self.extension_name.clone();
        let cascade = self.cascade;

        cx.spawn(|this, mut cx| async move {
            match ext_state.uninstall_extension(&name, cascade).await {
                Ok(()) => {
                    this.update(&mut cx, |_, cx| {
                        cx.emit(UninstallDialogEvent::Uninstalled);
                    })
                    .ok();
                }
                Err(e) => {
                    this.update(&mut cx, |this, cx| {
                        this.uninstalling = false;
                        this.error = Some(e.to_string());
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    fn handle_cancel(&mut self, cx: &mut ViewContext<Self>) {
        cx.emit(UninstallDialogEvent::Cancelled);
    }
}

impl FocusableView for UninstallExtensionDialog {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for UninstallExtensionDialog {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let ext_state = cx.global::<ExtensionState>();
        let generated_sql = self.generate_sql(ext_state);

        Modal::new("uninstall-extension")
            .title(format!("Uninstall Extension: {}", self.extension_name))
            .width(px(450.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_4()
                    // Warning message
                    .child(
                        div()
                            .p_3()
                            .bg(gpui::rgb(0xfef3c7))
                            .border_1()
                            .border_color(gpui::rgb(0xfbbf24))
                            .rounded(px(6.0))
                            .text_sm()
                            .text_color(gpui::rgb(0x92400e))
                            .child(
                                "This will remove the extension and all objects it created. This action cannot be undone.",
                            ),
                    )
                    // Error message
                    .when_some(self.error.clone(), |this, error| {
                        this.child(
                            div()
                                .p_3()
                                .bg(gpui::rgb(0xfef2f2))
                                .border_1()
                                .border_color(gpui::rgb(0xfecaca))
                                .rounded(px(6.0))
                                .text_sm()
                                .text_color(gpui::rgb(0xb91c1c))
                                .child(error),
                        )
                    })
                    // Cascade option
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                Checkbox::new("cascade")
                                    .checked(self.cascade)
                                    .label("CASCADE - Also drop dependent objects")
                                    .on_toggle(cx.listener(|this, _, cx| {
                                        this.cascade = !this.cascade;
                                        cx.notify();
                                    })),
                            )
                            .when(self.cascade, |this| {
                                this.child(
                                    div()
                                        .pl_6()
                                        .text_xs()
                                        .text_color(gpui::rgb(0xdc2626))
                                        .child(
                                            "Warning: This will drop all objects that depend on this extension!",
                                        ),
                                )
                            }),
                    )
                    // SQL preview
                    .child(
                        Section::new("sql")
                            .label("Generated SQL")
                            .child(
                                div()
                                    .p_3()
                                    .bg(gpui::rgb(0xf3f4f6))
                                    .rounded(px(6.0))
                                    .font_family("monospace")
                                    .text_xs()
                                    .child(generated_sql),
                            ),
                    ),
            )
            .footer(
                ModalFooter::new().right(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            Button::new("cancel")
                                .label("Cancel")
                                .variant(ButtonVariant::Secondary)
                                .on_click(cx.listener(|this, _, cx| {
                                    this.handle_cancel(cx);
                                })),
                        )
                        .child(
                            Button::new("uninstall")
                                .label(if self.uninstalling {
                                    "Uninstalling..."
                                } else {
                                    "Uninstall"
                                })
                                .variant(ButtonVariant::Danger)
                                .disabled(self.uninstalling)
                                .on_click(cx.listener(|this, _, cx| {
                                    this.handle_uninstall(cx);
                                })),
                        ),
                ),
            )
    }
}
```

### 23.8 Extension Manager Panel (Main Container)

```rust
// src/components/extensions/extension_panel.rs

use crate::components::extensions::{
    ExtensionDetailEvent, ExtensionDetailPanel, ExtensionListEvent, ExtensionListView,
    InstallDialogEvent, InstallExtensionDialog, UninstallDialogEvent,
    UninstallExtensionDialog,
};
use crate::models::extension::Extension;
use crate::state::extension_state::ExtensionState;
use crate::ui::{Panel, SplitView, SplitViewDirection};
use gpui::{
    div, AppContext, Context, Element, FocusHandle, FocusableView, IntoElement,
    ParentElement, Render, Styled, View, ViewContext, VisualContext,
};

pub struct ExtensionPanel {
    focus_handle: FocusHandle,
    list_view: View<ExtensionListView>,
    detail_panel: View<ExtensionDetailPanel>,
    install_dialog: Option<View<InstallExtensionDialog>>,
    uninstall_dialog: Option<View<UninstallExtensionDialog>>,
}

impl ExtensionPanel {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        // Initialize extension state
        let ext_state = ExtensionState::new();
        cx.set_global(ext_state);

        // Create child views
        let list_view = cx.new_view(|cx| ExtensionListView::new(cx));
        let detail_panel = cx.new_view(|cx| ExtensionDetailPanel::new(cx));

        // Subscribe to events
        cx.subscribe(&list_view, Self::handle_list_event).detach();
        cx.subscribe(&detail_panel, Self::handle_detail_event).detach();

        Self {
            focus_handle: cx.focus_handle(),
            list_view,
            detail_panel,
            install_dialog: None,
            uninstall_dialog: None,
        }
    }

    /// Load extensions when panel becomes active
    pub fn load_extensions(&self, cx: &mut ViewContext<Self>) {
        let ext_state = cx.global::<ExtensionState>().clone();

        cx.spawn(|this, mut cx| async move {
            if let Err(e) = ext_state.load_extensions().await {
                log::error!("Failed to load extensions: {}", e);
            }

            this.update(&mut cx, |_, cx| cx.notify()).ok();
        })
        .detach();
    }

    fn handle_list_event(
        &mut self,
        _: View<ExtensionListView>,
        event: &ExtensionListEvent,
        cx: &mut ViewContext<Self>,
    ) {
        match event {
            ExtensionListEvent::Select(_) => {
                cx.notify();
            }
            ExtensionListEvent::Install(ext) => {
                self.show_install_dialog(ext.clone(), cx);
            }
            ExtensionListEvent::Upgrade(ext) => {
                self.handle_upgrade(ext.clone(), cx);
            }
            ExtensionListEvent::Uninstall(name) => {
                self.show_uninstall_dialog(name.clone(), cx);
            }
        }
    }

    fn handle_detail_event(
        &mut self,
        _: View<ExtensionDetailPanel>,
        event: &ExtensionDetailEvent,
        cx: &mut ViewContext<Self>,
    ) {
        match event {
            ExtensionDetailEvent::Upgrade(name) => {
                // Find extension and trigger upgrade
                let ext_state = cx.global::<ExtensionState>();
                if let Some(ext) = ext_state.extensions().into_iter().find(|e| &e.name == name) {
                    self.handle_upgrade(ext, cx);
                }
            }
            ExtensionDetailEvent::Uninstall(name) => {
                self.show_uninstall_dialog(name.clone(), cx);
            }
        }
    }

    fn show_install_dialog(&mut self, ext: Extension, cx: &mut ViewContext<Self>) {
        // Get available schemas from schema cache (would come from another state)
        let schemas = vec![
            "public".to_string(),
            "extensions".to_string(),
        ];

        let dialog = cx.new_view(|cx| InstallExtensionDialog::new(ext, schemas, cx));
        cx.subscribe(&dialog, Self::handle_install_dialog_event).detach();
        self.install_dialog = Some(dialog);
        cx.notify();
    }

    fn show_uninstall_dialog(&mut self, name: String, cx: &mut ViewContext<Self>) {
        let dialog = cx.new_view(|cx| UninstallExtensionDialog::new(name, cx));
        cx.subscribe(&dialog, Self::handle_uninstall_dialog_event).detach();
        self.uninstall_dialog = Some(dialog);
        cx.notify();
    }

    fn handle_upgrade(&mut self, ext: Extension, cx: &mut ViewContext<Self>) {
        let ext_state = cx.global::<ExtensionState>().clone();
        let options = crate::models::extension::UpgradeExtensionOptions::new(&ext.name);

        cx.spawn(|this, mut cx| async move {
            if let Err(e) = ext_state.upgrade_extension(&options).await {
                log::error!("Failed to upgrade extension: {}", e);
            }

            this.update(&mut cx, |_, cx| cx.notify()).ok();
        })
        .detach();
    }

    fn handle_install_dialog_event(
        &mut self,
        _: View<InstallExtensionDialog>,
        event: &InstallDialogEvent,
        cx: &mut ViewContext<Self>,
    ) {
        match event {
            InstallDialogEvent::Installed | InstallDialogEvent::Cancelled => {
                self.install_dialog = None;
                cx.notify();
            }
        }
    }

    fn handle_uninstall_dialog_event(
        &mut self,
        _: View<UninstallExtensionDialog>,
        event: &UninstallDialogEvent,
        cx: &mut ViewContext<Self>,
    ) {
        match event {
            UninstallDialogEvent::Uninstalled | UninstallDialogEvent::Cancelled => {
                self.uninstall_dialog = None;
                cx.notify();
            }
        }
    }
}

impl FocusableView for ExtensionPanel {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ExtensionPanel {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        Panel::new("extension-manager")
            .title("Extension Manager")
            .child(
                SplitView::new("extension-split")
                    .direction(SplitViewDirection::Horizontal)
                    .initial_ratio(0.55)
                    .min_size(300.0)
                    .left(self.list_view.clone())
                    .right(self.detail_panel.clone()),
            )
            // Install dialog overlay
            .when_some(self.install_dialog.clone(), |this, dialog| {
                this.child(dialog)
            })
            // Uninstall dialog overlay
            .when_some(self.uninstall_dialog.clone(), |this, dialog| {
                this.child(dialog)
            })
    }
}
```

## Acceptance Criteria

1. **Extension Listing**
   - [ ] Display all available extensions in a sortable table
   - [ ] Show installed status with visual indicator
   - [ ] Display version with upgrade availability
   - [ ] Filter by name, description, and installed status
   - [ ] Show extension count statistics

2. **Extension Installation**
   - [ ] Select version to install from available versions
   - [ ] Choose target schema (if relocatable)
   - [ ] CASCADE option for dependencies
   - [ ] Preview generated SQL before execution
   - [ ] Handle installation errors gracefully

3. **Extension Details**
   - [ ] Show installed extension info (version, schema, description)
   - [ ] List all objects created by extension grouped by type
   - [ ] Display configuration parameters with values
   - [ ] Show required dependencies

4. **Extension Upgrade**
   - [ ] Upgrade to latest or specific version
   - [ ] Preview upgrade SQL
   - [ ] Handle upgrade errors gracefully

5. **Extension Removal**
   - [ ] Confirm before uninstall with warning
   - [ ] CASCADE option for dependent objects
   - [ ] Preview uninstall SQL
   - [ ] Handle removal errors

6. **State Management**
   - [ ] Use GPUI Global trait for ExtensionState
   - [ ] Thread-safe access with parking_lot::RwLock
   - [ ] Automatic refresh after mutations
   - [ ] Loading and error states

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_sql_basic() {
        let options = InstallExtensionOptions::new("uuid-ossp");
        let sql = ExtensionService::build_install_sql(&options);
        assert_eq!(sql, "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\" CASCADE");
    }

    #[test]
    fn test_install_sql_with_options() {
        let options = InstallExtensionOptions {
            name: "postgis".to_string(),
            version: Some("3.4.0".to_string()),
            schema: Some("extensions".to_string()),
            cascade: true,
        };
        let sql = ExtensionService::build_install_sql(&options);
        assert!(sql.contains("SCHEMA extensions"));
        assert!(sql.contains("VERSION '3.4.0'"));
        assert!(sql.contains("CASCADE"));
    }

    #[test]
    fn test_upgrade_sql() {
        let options = UpgradeExtensionOptions {
            name: "postgis".to_string(),
            target_version: Some("3.5.0".to_string()),
        };
        let sql = ExtensionService::build_upgrade_sql(&options);
        assert_eq!(sql, "ALTER EXTENSION postgis UPDATE TO '3.5.0'");
    }

    #[test]
    fn test_uninstall_sql() {
        assert_eq!(
            ExtensionService::build_uninstall_sql("hstore", false),
            "DROP EXTENSION IF EXISTS hstore"
        );
        assert_eq!(
            ExtensionService::build_uninstall_sql("hstore", true),
            "DROP EXTENSION IF EXISTS hstore CASCADE"
        );
    }

    #[test]
    fn test_extension_has_upgrade() {
        let ext = Extension {
            name: "test".to_string(),
            installed_version: Some("1.0".to_string()),
            default_version: "2.0".to_string(),
            available_versions: vec![],
            schema: None,
            relocatable: true,
            comment: None,
            requires: vec![],
            is_installed: true,
        };
        assert!(ext.has_upgrade());

        let ext_no_upgrade = Extension {
            installed_version: Some("2.0".to_string()),
            ..ext.clone()
        };
        assert!(!ext_no_upgrade.has_upgrade());
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    async fn test_extension_list_filtering(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let state = ExtensionState::new();
            cx.set_global(state);
        });

        let view = cx.new_view(|cx| ExtensionListView::new(cx));

        view.update(cx, |_, cx| {
            let ext_state = cx.global::<ExtensionState>();
            ext_state.set_filter("uuid".to_string());
        });

        view.update(cx, |_, cx| {
            let ext_state = cx.global::<ExtensionState>();
            assert_eq!(ext_state.filter(), "uuid");
        });
    }

    #[gpui::test]
    async fn test_install_dialog_sql_generation(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let state = ExtensionState::new();
            cx.set_global(state);
        });

        let ext = Extension {
            name: "pg_trgm".to_string(),
            installed_version: None,
            default_version: "1.6".to_string(),
            available_versions: vec!["1.6".to_string()],
            schema: None,
            relocatable: true,
            comment: Some("text similarity".to_string()),
            requires: vec![],
            is_installed: false,
        };

        let view = cx.new_view(|cx| {
            InstallExtensionDialog::new(
                ext,
                vec!["public".to_string(), "extensions".to_string()],
                cx,
            )
        });

        view.update(cx, |view, _| {
            let sql = view.generate_sql();
            assert!(sql.contains("CREATE EXTENSION"));
            assert!(sql.contains("pg_trgm"));
        });
    }
}
```
