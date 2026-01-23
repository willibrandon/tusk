# Feature 10: Schema Introspection

## Overview

Implement the schema introspection service that queries Postgres system catalogs to retrieve schema metadata (tables, views, columns, indexes, functions, etc.). Includes in-memory caching with fast synchronous access, LISTEN/NOTIFY for real-time schema change detection, and integration with GPUI state management.

## Goals

- Query all schema objects defined in design doc Section 3.2
- Implement efficient caching with memory-indexed structures
- Support incremental refresh via LISTEN/NOTIFY
- Provide fast autocomplete data access (< 50ms)
- Handle large schemas (1000+ tables) efficiently
- Thread-safe synchronous access for GPUI components

## Dependencies

- 07-connection-management.md (ConnectionPool, ConnectionService)

## Technical Specification

### 1. Schema Data Models

```rust
// src/models/schema.rs

use serde::{Deserialize, Serialize};

/// Complete database schema including all objects
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DatabaseSchema {
    pub schemas: Vec<Schema>,
    pub extensions: Vec<Extension>,
    pub roles: Vec<Role>,
    pub loaded_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// A database schema (namespace) with all contained objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub name: String,
    pub tables: Vec<Table>,
    pub views: Vec<View>,
    pub materialized_views: Vec<MaterializedView>,
    pub functions: Vec<Function>,
    pub sequences: Vec<Sequence>,
    pub types: Vec<CustomType>,
}

/// Table metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub oid: i64,
    pub schema: String,
    pub name: String,
    pub columns: Vec<Column>,
    pub primary_key: Option<Constraint>,
    pub foreign_keys: Vec<ForeignKey>,
    pub unique_constraints: Vec<Constraint>,
    pub check_constraints: Vec<CheckConstraint>,
    pub indexes: Vec<Index>,
    pub triggers: Vec<Trigger>,
    pub policies: Vec<Policy>,
    pub row_count_estimate: i64,
    pub size_bytes: i64,
    pub comment: Option<String>,
}

impl Table {
    /// Get the fully qualified name
    pub fn qualified_name(&self) -> String {
        format!("{}.{}", self.schema, self.name)
    }

    /// Get primary key column names
    pub fn primary_key_columns(&self) -> Vec<&str> {
        self.primary_key
            .as_ref()
            .map(|pk| pk.columns.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }
}

/// Column metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub ordinal: i32,
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub base_type: String,
    pub nullable: bool,
    pub default: Option<String>,
    pub is_identity: bool,
    pub identity_generation: Option<String>,
    pub is_generated: bool,
    pub generation_expression: Option<String>,
    pub comment: Option<String>,
}

impl Column {
    /// Check if column has a default value
    pub fn has_default(&self) -> bool {
        self.default.is_some() || self.is_identity || self.is_generated
    }

    /// Check if column is auto-generated (identity, serial, generated)
    pub fn is_auto(&self) -> bool {
        self.is_identity || self.is_generated ||
        self.default.as_ref().map(|d| d.contains("nextval")).unwrap_or(false)
    }
}

/// Index metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub oid: i64,
    pub name: String,
    pub columns: Vec<String>,
    pub include_columns: Vec<String>,
    pub is_unique: bool,
    pub is_primary: bool,
    pub is_partial: bool,
    pub predicate: Option<String>,
    pub method: String, // btree, hash, gist, gin, brin
    pub size_bytes: i64,
    pub definition: String,
}

/// Foreign key constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKey {
    pub name: String,
    pub columns: Vec<String>,
    pub referenced_schema: String,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_delete: ForeignKeyAction,
    pub on_update: ForeignKeyAction,
    pub deferrable: bool,
    pub initially_deferred: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForeignKeyAction {
    NoAction,
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
}

impl ForeignKeyAction {
    pub fn from_pg_char(c: char) -> Self {
        match c {
            'a' => Self::NoAction,
            'r' => Self::Restrict,
            'c' => Self::Cascade,
            'n' => Self::SetNull,
            'd' => Self::SetDefault,
            _ => Self::NoAction,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoAction => "NO ACTION",
            Self::Restrict => "RESTRICT",
            Self::Cascade => "CASCADE",
            Self::SetNull => "SET NULL",
            Self::SetDefault => "SET DEFAULT",
        }
    }
}

/// Unique or primary key constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub name: String,
    pub columns: Vec<String>,
}

/// Check constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckConstraint {
    pub name: String,
    pub expression: String,
}

/// View metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View {
    pub oid: i64,
    pub schema: String,
    pub name: String,
    pub columns: Vec<Column>,
    pub definition: String,
    pub is_updatable: bool,
    pub comment: Option<String>,
}

/// Materialized view metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializedView {
    pub oid: i64,
    pub schema: String,
    pub name: String,
    pub columns: Vec<Column>,
    pub definition: String,
    pub indexes: Vec<Index>,
    pub is_populated: bool,
    pub size_bytes: i64,
    pub comment: Option<String>,
}

/// Function/procedure metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub oid: i64,
    pub schema: String,
    pub name: String,
    pub kind: FunctionKind,
    pub arguments: Vec<FunctionArgument>,
    pub return_type: String,
    pub language: String,
    pub volatility: FunctionVolatility,
    pub is_strict: bool,
    pub is_security_definer: bool,
    pub source: String,
    pub comment: Option<String>,
}

impl Function {
    /// Get function signature for display
    pub fn signature(&self) -> String {
        let args: Vec<_> = self.arguments.iter()
            .map(|a| {
                let name = a.name.as_ref().map(|n| format!("{} ", n)).unwrap_or_default();
                format!("{}{}", name, a.data_type)
            })
            .collect();
        format!("{}({})", self.name, args.join(", "))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionKind {
    Function,
    Procedure,
    Aggregate,
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionVolatility {
    Immutable,
    Stable,
    Volatile,
}

/// Function argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionArgument {
    pub name: Option<String>,
    pub data_type: String,
    pub mode: ArgumentMode,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArgumentMode {
    In,
    Out,
    InOut,
    Variadic,
    Table,
}

/// Sequence metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sequence {
    pub oid: i64,
    pub schema: String,
    pub name: String,
    pub data_type: String,
    pub start_value: i64,
    pub increment: i64,
    pub min_value: i64,
    pub max_value: i64,
    pub cache_size: i64,
    pub is_cyclic: bool,
    pub owned_by: Option<String>,
}

/// Custom type (enum, composite, domain, range)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomType {
    pub oid: i64,
    pub schema: String,
    pub name: String,
    pub type_type: TypeKind,
    pub enum_values: Option<Vec<String>>,
    pub composite_attributes: Option<Vec<Column>>,
    pub domain_base_type: Option<String>,
    pub domain_constraint: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeKind {
    Enum,
    Composite,
    Domain,
    Range,
    Base,
}

/// Trigger metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub name: String,
    pub timing: TriggerTiming,
    pub events: Vec<TriggerEvent>,
    pub function_schema: String,
    pub function_name: String,
    pub is_enabled: bool,
    pub for_each: TriggerScope,
    pub when_clause: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerTiming {
    Before,
    After,
    InsteadOf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerEvent {
    Insert,
    Update,
    Delete,
    Truncate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerScope {
    Row,
    Statement,
}

/// Row Level Security policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub name: String,
    pub command: PolicyCommand,
    pub roles: Vec<String>,
    pub using_expression: Option<String>,
    pub with_check: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyCommand {
    All,
    Select,
    Insert,
    Update,
    Delete,
}

/// Extension metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extension {
    pub name: String,
    pub version: String,
    pub schema: String,
    pub relocatable: bool,
    pub comment: Option<String>,
}

/// Role/user metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub is_superuser: bool,
    pub can_login: bool,
    pub can_create_db: bool,
    pub can_create_role: bool,
    pub can_bypass_rls: bool,
    pub connection_limit: i32,
    pub valid_until: Option<String>,
    pub member_of: Vec<String>,
    pub config: Vec<String>,
}
```

### 2. Schema Service

```rust
// src/services/schema.rs

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::runtime::Handle;
use tokio_postgres::Client;
use uuid::Uuid;

use crate::error::{Result, TuskError};
use crate::models::schema::*;
use crate::services::connection::ConnectionPool;

/// Service for introspecting database schema
pub struct SchemaService {
    /// Schema caches per connection
    caches: RwLock<HashMap<Uuid, Arc<SchemaCache>>>,
    /// Tokio runtime handle for async operations
    runtime: Handle,
    /// Active schema change listeners
    listeners: RwLock<HashMap<Uuid, SchemaChangeListener>>,
}

impl SchemaService {
    pub fn new(runtime: Handle) -> Self {
        Self {
            caches: RwLock::new(HashMap::new()),
            runtime,
            listeners: RwLock::new(HashMap::new()),
        }
    }

    /// Get or create schema cache for a connection
    pub fn get_cache(&self, connection_id: &Uuid) -> Option<Arc<SchemaCache>> {
        self.caches.read().get(connection_id).cloned()
    }

    /// Initialize schema for a connection (fetch and cache)
    pub fn initialize(&self, connection_id: Uuid, pool: Arc<ConnectionPool>) -> Result<Arc<SchemaCache>> {
        let cache = Arc::new(SchemaCache::new());

        // Fetch schema synchronously via runtime
        let schema = self.runtime.block_on(async {
            let client = pool.get().await?;
            Self::fetch_schema(&client).await
        })?;

        cache.update(schema);
        self.caches.write().insert(connection_id, cache.clone());

        Ok(cache)
    }

    /// Refresh schema for a connection
    pub fn refresh(&self, connection_id: &Uuid, pool: Arc<ConnectionPool>) -> Result<()> {
        let cache = self.caches.read().get(connection_id).cloned();

        if let Some(cache) = cache {
            let schema = self.runtime.block_on(async {
                let client = pool.get().await?;
                Self::fetch_schema(&client).await
            })?;

            cache.update(schema);
        }

        Ok(())
    }

    /// Start listening for schema changes
    pub fn start_listener(
        &self,
        connection_id: Uuid,
        pool: Arc<ConnectionPool>,
        on_change: Box<dyn Fn() + Send + Sync>,
    ) -> Result<()> {
        let cache = self.caches.read().get(&connection_id).cloned()
            .ok_or_else(|| TuskError::SchemaNotLoaded)?;

        let listener = SchemaChangeListener::start(
            connection_id,
            pool,
            cache,
            on_change,
            self.runtime.clone(),
        )?;

        self.listeners.write().insert(connection_id, listener);
        Ok(())
    }

    /// Stop listening for schema changes
    pub fn stop_listener(&self, connection_id: &Uuid) {
        if let Some(mut listener) = self.listeners.write().remove(connection_id) {
            listener.stop();
        }
    }

    /// Remove cache for a connection
    pub fn remove_cache(&self, connection_id: &Uuid) {
        self.stop_listener(connection_id);
        self.caches.write().remove(connection_id);
    }

    /// Fetch complete schema from database
    pub async fn fetch_schema(client: &Client) -> Result<DatabaseSchema> {
        let start = std::time::Instant::now();

        let schemas = Self::fetch_schemas(client).await?;
        let extensions = Self::fetch_extensions(client).await?;
        let roles = Self::fetch_roles(client).await?;

        tracing::info!("Schema fetched in {:?}", start.elapsed());

        Ok(DatabaseSchema {
            schemas,
            extensions,
            roles,
            loaded_at: Some(chrono::Utc::now()),
        })
    }

    async fn fetch_schemas(client: &Client) -> Result<Vec<Schema>> {
        // Get list of schemas (excluding system schemas)
        let rows = client.query(
            "SELECT nspname FROM pg_namespace
             WHERE nspname NOT IN ('pg_catalog', 'information_schema', 'pg_toast')
               AND nspname NOT LIKE 'pg_temp_%'
               AND nspname NOT LIKE 'pg_toast_temp_%'
             ORDER BY nspname",
            &[],
        ).await?;

        let mut schemas = Vec::with_capacity(rows.len());
        for row in rows {
            let schema_name: String = row.get(0);
            let schema = Self::fetch_schema_objects(client, &schema_name).await?;
            schemas.push(schema);
        }

        Ok(schemas)
    }

    async fn fetch_schema_objects(client: &Client, schema_name: &str) -> Result<Schema> {
        // Fetch all objects in parallel
        let (tables, views, mat_views, functions, sequences, types) = tokio::try_join!(
            Self::fetch_tables(client, schema_name),
            Self::fetch_views(client, schema_name),
            Self::fetch_materialized_views(client, schema_name),
            Self::fetch_functions(client, schema_name),
            Self::fetch_sequences(client, schema_name),
            Self::fetch_types(client, schema_name),
        )?;

        Ok(Schema {
            name: schema_name.to_string(),
            tables,
            views,
            materialized_views: mat_views,
            functions,
            sequences,
            types,
        })
    }

    async fn fetch_tables(client: &Client, schema: &str) -> Result<Vec<Table>> {
        let rows = client.query(
            r#"
            SELECT
                c.oid,
                c.relname AS name,
                COALESCE(c.reltuples::bigint, 0) AS row_count_estimate,
                COALESCE(pg_total_relation_size(c.oid), 0) AS size_bytes,
                obj_description(c.oid, 'pg_class') AS comment
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE c.relkind = 'r'
              AND n.nspname = $1
            ORDER BY c.relname
            "#,
            &[&schema],
        ).await?;

        let mut tables = Vec::with_capacity(rows.len());
        for row in rows {
            let oid: i64 = row.get::<_, i32>(0) as i64;
            let name: String = row.get(1);

            // Fetch related objects in parallel
            let (columns, indexes, foreign_keys, constraints, check_constraints, triggers, policies) = tokio::try_join!(
                Self::fetch_columns(client, oid),
                Self::fetch_indexes(client, oid),
                Self::fetch_foreign_keys(client, oid),
                Self::fetch_unique_constraints(client, oid),
                Self::fetch_check_constraints(client, oid),
                Self::fetch_triggers(client, schema, &name),
                Self::fetch_policies(client, schema, &name),
            )?;

            // Extract primary key from constraints (has _pkey suffix or is marked primary in indexes)
            let primary_key = constraints.iter()
                .find(|c| indexes.iter().any(|i| i.is_primary && i.name == c.name))
                .cloned()
                .or_else(|| {
                    indexes.iter()
                        .find(|i| i.is_primary)
                        .map(|i| Constraint {
                            name: i.name.clone(),
                            columns: i.columns.clone(),
                        })
                });

            let unique_constraints = constraints.into_iter()
                .filter(|c| !primary_key.as_ref().map(|pk| pk.name == c.name).unwrap_or(false))
                .collect();

            tables.push(Table {
                oid,
                schema: schema.to_string(),
                name,
                columns,
                primary_key,
                foreign_keys,
                unique_constraints,
                check_constraints,
                indexes,
                triggers,
                policies,
                row_count_estimate: row.get(2),
                size_bytes: row.get(3),
                comment: row.get(4),
            });
        }

        Ok(tables)
    }

    async fn fetch_columns(client: &Client, table_oid: i64) -> Result<Vec<Column>> {
        let rows = client.query(
            r#"
            SELECT
                a.attnum AS ordinal,
                a.attname AS name,
                pg_catalog.format_type(a.atttypid, a.atttypmod) AS type,
                t.typname AS base_type,
                NOT a.attnotnull AS nullable,
                pg_get_expr(d.adbin, d.adrelid) AS default,
                a.attidentity != '' AS is_identity,
                CASE a.attidentity
                    WHEN 'a' THEN 'ALWAYS'
                    WHEN 'd' THEN 'BY DEFAULT'
                END AS identity_generation,
                a.attgenerated != '' AS is_generated,
                CASE WHEN a.attgenerated != ''
                    THEN pg_get_expr(d.adbin, d.adrelid)
                END AS generation_expression,
                col_description(a.attrelid, a.attnum) AS comment
            FROM pg_attribute a
            JOIN pg_type t ON t.oid = a.atttypid
            LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
            WHERE a.attrelid = $1
              AND a.attnum > 0
              AND NOT a.attisdropped
            ORDER BY a.attnum
            "#,
            &[&(table_oid as i32)],
        ).await?;

        Ok(rows.iter().map(|row| {
            Column {
                ordinal: row.get(0),
                name: row.get(1),
                data_type: row.get(2),
                base_type: row.get(3),
                nullable: row.get(4),
                default: row.get(5),
                is_identity: row.get(6),
                identity_generation: row.get(7),
                is_generated: row.get(8),
                generation_expression: row.get(9),
                comment: row.get(10),
            }
        }).collect())
    }

    async fn fetch_indexes(client: &Client, table_oid: i64) -> Result<Vec<Index>> {
        let rows = client.query(
            r#"
            SELECT
                i.indexrelid AS oid,
                ic.relname AS name,
                ARRAY(
                    SELECT a.attname
                    FROM unnest(i.indkey) WITH ORDINALITY AS u(attnum, ord)
                    JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = u.attnum
                    ORDER BY u.ord
                ) AS columns,
                COALESCE(ARRAY(
                    SELECT a.attname
                    FROM unnest(i.indkey[(array_length(i.indkey, 1) - i.indnkeyatts + 1):]) AS u(attnum)
                    JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = u.attnum
                ), ARRAY[]::text[]) AS include_columns,
                i.indisunique AS is_unique,
                i.indisprimary AS is_primary,
                i.indpred IS NOT NULL AS is_partial,
                pg_get_expr(i.indpred, i.indrelid) AS predicate,
                am.amname AS method,
                pg_relation_size(i.indexrelid) AS size_bytes,
                pg_get_indexdef(i.indexrelid) AS definition
            FROM pg_index i
            JOIN pg_class ic ON ic.oid = i.indexrelid
            JOIN pg_am am ON am.oid = ic.relam
            WHERE i.indrelid = $1
            ORDER BY ic.relname
            "#,
            &[&(table_oid as i32)],
        ).await?;

        Ok(rows.iter().map(|row| {
            Index {
                oid: row.get::<_, i32>(0) as i64,
                name: row.get(1),
                columns: row.get(2),
                include_columns: row.get(3),
                is_unique: row.get(4),
                is_primary: row.get(5),
                is_partial: row.get(6),
                predicate: row.get(7),
                method: row.get(8),
                size_bytes: row.get(9),
                definition: row.get(10),
            }
        }).collect())
    }

    async fn fetch_foreign_keys(client: &Client, table_oid: i64) -> Result<Vec<ForeignKey>> {
        let rows = client.query(
            r#"
            SELECT
                c.conname AS name,
                ARRAY(
                    SELECT a.attname
                    FROM unnest(c.conkey) WITH ORDINALITY AS u(attnum, ord)
                    JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = u.attnum
                    ORDER BY u.ord
                ) AS columns,
                n2.nspname AS referenced_schema,
                c2.relname AS referenced_table,
                ARRAY(
                    SELECT a.attname
                    FROM unnest(c.confkey) WITH ORDINALITY AS u(attnum, ord)
                    JOIN pg_attribute a ON a.attrelid = c.confrelid AND a.attnum = u.attnum
                    ORDER BY u.ord
                ) AS referenced_columns,
                c.confupdtype AS on_update,
                c.confdeltype AS on_delete,
                c.condeferrable AS deferrable,
                c.condeferred AS initially_deferred
            FROM pg_constraint c
            JOIN pg_class c2 ON c2.oid = c.confrelid
            JOIN pg_namespace n2 ON n2.oid = c2.relnamespace
            WHERE c.conrelid = $1
              AND c.contype = 'f'
            ORDER BY c.conname
            "#,
            &[&(table_oid as i32)],
        ).await?;

        Ok(rows.iter().map(|row| {
            let on_update_char: i8 = row.get(5);
            let on_delete_char: i8 = row.get(6);

            ForeignKey {
                name: row.get(0),
                columns: row.get(1),
                referenced_schema: row.get(2),
                referenced_table: row.get(3),
                referenced_columns: row.get(4),
                on_update: ForeignKeyAction::from_pg_char(on_update_char as u8 as char),
                on_delete: ForeignKeyAction::from_pg_char(on_delete_char as u8 as char),
                deferrable: row.get(7),
                initially_deferred: row.get(8),
            }
        }).collect())
    }

    async fn fetch_unique_constraints(client: &Client, table_oid: i64) -> Result<Vec<Constraint>> {
        let rows = client.query(
            r#"
            SELECT
                c.conname AS name,
                ARRAY(
                    SELECT a.attname
                    FROM unnest(c.conkey) WITH ORDINALITY AS u(attnum, ord)
                    JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = u.attnum
                    ORDER BY u.ord
                ) AS columns
            FROM pg_constraint c
            WHERE c.conrelid = $1
              AND c.contype IN ('p', 'u')
            ORDER BY c.conname
            "#,
            &[&(table_oid as i32)],
        ).await?;

        Ok(rows.iter().map(|row| {
            Constraint {
                name: row.get(0),
                columns: row.get(1),
            }
        }).collect())
    }

    async fn fetch_check_constraints(client: &Client, table_oid: i64) -> Result<Vec<CheckConstraint>> {
        let rows = client.query(
            r#"
            SELECT
                c.conname AS name,
                pg_get_constraintdef(c.oid) AS expression
            FROM pg_constraint c
            WHERE c.conrelid = $1
              AND c.contype = 'c'
            ORDER BY c.conname
            "#,
            &[&(table_oid as i32)],
        ).await?;

        Ok(rows.iter().map(|row| {
            CheckConstraint {
                name: row.get(0),
                expression: row.get(1),
            }
        }).collect())
    }

    async fn fetch_triggers(client: &Client, schema: &str, table: &str) -> Result<Vec<Trigger>> {
        let rows = client.query(
            r#"
            SELECT
                t.tgname AS name,
                CASE
                    WHEN t.tgtype & 2 = 2 THEN 'BEFORE'
                    WHEN t.tgtype & 64 = 64 THEN 'INSTEAD OF'
                    ELSE 'AFTER'
                END AS timing,
                ARRAY_REMOVE(ARRAY[
                    CASE WHEN t.tgtype & 4 = 4 THEN 'INSERT' END,
                    CASE WHEN t.tgtype & 8 = 8 THEN 'DELETE' END,
                    CASE WHEN t.tgtype & 16 = 16 THEN 'UPDATE' END,
                    CASE WHEN t.tgtype & 32 = 32 THEN 'TRUNCATE' END
                ], NULL) AS events,
                np.nspname AS function_schema,
                p.proname AS function_name,
                t.tgenabled != 'D' AS is_enabled,
                CASE WHEN t.tgtype & 1 = 1 THEN 'ROW' ELSE 'STATEMENT' END AS for_each,
                pg_get_triggerdef(t.oid) AS definition
            FROM pg_trigger t
            JOIN pg_class c ON c.oid = t.tgrelid
            JOIN pg_namespace n ON n.oid = c.relnamespace
            JOIN pg_proc p ON p.oid = t.tgfoid
            JOIN pg_namespace np ON np.oid = p.pronamespace
            WHERE n.nspname = $1
              AND c.relname = $2
              AND NOT t.tgisinternal
            ORDER BY t.tgname
            "#,
            &[&schema, &table],
        ).await?;

        Ok(rows.iter().map(|row| {
            let timing_str: String = row.get(1);
            let events_arr: Vec<String> = row.get(2);
            let for_each_str: String = row.get(6);

            Trigger {
                name: row.get(0),
                timing: match timing_str.as_str() {
                    "BEFORE" => TriggerTiming::Before,
                    "INSTEAD OF" => TriggerTiming::InsteadOf,
                    _ => TriggerTiming::After,
                },
                events: events_arr.into_iter().map(|e| {
                    match e.as_str() {
                        "INSERT" => TriggerEvent::Insert,
                        "DELETE" => TriggerEvent::Delete,
                        "UPDATE" => TriggerEvent::Update,
                        _ => TriggerEvent::Truncate,
                    }
                }).collect(),
                function_schema: row.get(3),
                function_name: row.get(4),
                is_enabled: row.get(5),
                for_each: if for_each_str == "ROW" {
                    TriggerScope::Row
                } else {
                    TriggerScope::Statement
                },
                when_clause: None, // Could parse from definition if needed
            }
        }).collect())
    }

    async fn fetch_policies(client: &Client, schema: &str, table: &str) -> Result<Vec<Policy>> {
        let rows = client.query(
            r#"
            SELECT
                p.polname AS name,
                p.polcmd AS command,
                ARRAY(SELECT r.rolname FROM pg_roles r WHERE r.oid = ANY(p.polroles)) AS roles,
                pg_get_expr(p.polqual, p.polrelid) AS using_expression,
                pg_get_expr(p.polwithcheck, p.polrelid) AS with_check
            FROM pg_policy p
            JOIN pg_class c ON c.oid = p.polrelid
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE n.nspname = $1
              AND c.relname = $2
            ORDER BY p.polname
            "#,
            &[&schema, &table],
        ).await?;

        Ok(rows.iter().map(|row| {
            let cmd_char: i8 = row.get(1);
            Policy {
                name: row.get(0),
                command: match cmd_char as u8 as char {
                    '*' => PolicyCommand::All,
                    'r' => PolicyCommand::Select,
                    'a' => PolicyCommand::Insert,
                    'w' => PolicyCommand::Update,
                    'd' => PolicyCommand::Delete,
                    _ => PolicyCommand::All,
                },
                roles: row.get(2),
                using_expression: row.get(3),
                with_check: row.get(4),
            }
        }).collect())
    }

    async fn fetch_views(client: &Client, schema: &str) -> Result<Vec<View>> {
        let rows = client.query(
            r#"
            SELECT
                c.oid,
                c.relname AS name,
                pg_get_viewdef(c.oid, true) AS definition,
                (SELECT EXISTS (
                    SELECT 1 FROM information_schema.views v
                    WHERE v.table_schema = $1 AND v.table_name = c.relname
                    AND v.is_updatable = 'YES'
                )) AS is_updatable,
                obj_description(c.oid, 'pg_class') AS comment
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE c.relkind = 'v'
              AND n.nspname = $1
            ORDER BY c.relname
            "#,
            &[&schema],
        ).await?;

        let mut views = Vec::with_capacity(rows.len());
        for row in rows {
            let oid: i64 = row.get::<_, i32>(0) as i64;
            let columns = Self::fetch_columns(client, oid).await?;

            views.push(View {
                oid,
                schema: schema.to_string(),
                name: row.get(1),
                columns,
                definition: row.get(2),
                is_updatable: row.get(3),
                comment: row.get(4),
            });
        }

        Ok(views)
    }

    async fn fetch_materialized_views(client: &Client, schema: &str) -> Result<Vec<MaterializedView>> {
        let rows = client.query(
            r#"
            SELECT
                c.oid,
                c.relname AS name,
                pg_get_viewdef(c.oid, true) AS definition,
                c.relispopulated AS is_populated,
                pg_total_relation_size(c.oid) AS size_bytes,
                obj_description(c.oid, 'pg_class') AS comment
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE c.relkind = 'm'
              AND n.nspname = $1
            ORDER BY c.relname
            "#,
            &[&schema],
        ).await?;

        let mut mat_views = Vec::with_capacity(rows.len());
        for row in rows {
            let oid: i64 = row.get::<_, i32>(0) as i64;
            let columns = Self::fetch_columns(client, oid).await?;
            let indexes = Self::fetch_indexes(client, oid).await?;

            mat_views.push(MaterializedView {
                oid,
                schema: schema.to_string(),
                name: row.get(1),
                columns,
                definition: row.get(2),
                indexes,
                is_populated: row.get(3),
                size_bytes: row.get(4),
                comment: row.get(5),
            });
        }

        Ok(mat_views)
    }

    async fn fetch_functions(client: &Client, schema: &str) -> Result<Vec<Function>> {
        let rows = client.query(
            r#"
            SELECT
                p.oid,
                p.proname AS name,
                CASE p.prokind
                    WHEN 'f' THEN 'function'
                    WHEN 'p' THEN 'procedure'
                    WHEN 'a' THEN 'aggregate'
                    WHEN 'w' THEN 'window'
                    ELSE 'function'
                END AS kind,
                pg_get_function_arguments(p.oid) AS arguments_str,
                pg_get_function_result(p.oid) AS return_type,
                l.lanname AS language,
                CASE p.provolatile
                    WHEN 'i' THEN 'immutable'
                    WHEN 's' THEN 'stable'
                    ELSE 'volatile'
                END AS volatility,
                p.proisstrict AS is_strict,
                p.prosecdef AS is_security_definer,
                p.prosrc AS source,
                obj_description(p.oid, 'pg_proc') AS comment
            FROM pg_proc p
            JOIN pg_namespace n ON n.oid = p.pronamespace
            JOIN pg_language l ON l.oid = p.prolang
            WHERE n.nspname = $1
              AND p.prokind IN ('f', 'p', 'a', 'w')
            ORDER BY p.proname, pg_get_function_arguments(p.oid)
            "#,
            &[&schema],
        ).await?;

        Ok(rows.iter().map(|row| {
            let kind_str: String = row.get(2);
            let vol_str: String = row.get(6);

            Function {
                oid: row.get::<_, i32>(0) as i64,
                schema: schema.to_string(),
                name: row.get(1),
                kind: match kind_str.as_str() {
                    "procedure" => FunctionKind::Procedure,
                    "aggregate" => FunctionKind::Aggregate,
                    "window" => FunctionKind::Window,
                    _ => FunctionKind::Function,
                },
                arguments: parse_function_arguments(row.get(3)),
                return_type: row.get(4),
                language: row.get(5),
                volatility: match vol_str.as_str() {
                    "immutable" => FunctionVolatility::Immutable,
                    "stable" => FunctionVolatility::Stable,
                    _ => FunctionVolatility::Volatile,
                },
                is_strict: row.get(7),
                is_security_definer: row.get(8),
                source: row.get(9),
                comment: row.get(10),
            }
        }).collect())
    }

    async fn fetch_sequences(client: &Client, schema: &str) -> Result<Vec<Sequence>> {
        let rows = client.query(
            r#"
            SELECT
                c.oid,
                c.relname AS name,
                format_type(s.seqtypid, NULL) AS data_type,
                s.seqstart AS start_value,
                s.seqincrement AS increment,
                s.seqmin AS min_value,
                s.seqmax AS max_value,
                s.seqcache AS cache_size,
                s.seqcycle AS is_cyclic,
                pg_get_serial_sequence(
                    quote_ident(d.refobjid::regclass::text),
                    a.attname
                ) AS owned_by
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            JOIN pg_sequence s ON s.seqrelid = c.oid
            LEFT JOIN pg_depend d ON d.objid = c.oid AND d.deptype = 'a'
            LEFT JOIN pg_attribute a ON a.attrelid = d.refobjid AND a.attnum = d.refobjsubid
            WHERE c.relkind = 'S'
              AND n.nspname = $1
            ORDER BY c.relname
            "#,
            &[&schema],
        ).await?;

        Ok(rows.iter().map(|row| {
            Sequence {
                oid: row.get::<_, i32>(0) as i64,
                schema: schema.to_string(),
                name: row.get(1),
                data_type: row.get(2),
                start_value: row.get(3),
                increment: row.get(4),
                min_value: row.get(5),
                max_value: row.get(6),
                cache_size: row.get(7),
                is_cyclic: row.get(8),
                owned_by: row.get(9),
            }
        }).collect())
    }

    async fn fetch_types(client: &Client, schema: &str) -> Result<Vec<CustomType>> {
        let rows = client.query(
            r#"
            SELECT
                t.oid,
                t.typname AS name,
                t.typtype AS type_type,
                ARRAY(
                    SELECT e.enumlabel
                    FROM pg_enum e
                    WHERE e.enumtypid = t.oid
                    ORDER BY e.enumsortorder
                ) AS enum_values,
                obj_description(t.oid, 'pg_type') AS comment
            FROM pg_type t
            JOIN pg_namespace n ON n.oid = t.typnamespace
            WHERE n.nspname = $1
              AND t.typtype IN ('e', 'c', 'd', 'r')
              AND NOT EXISTS (
                  SELECT 1 FROM pg_class c WHERE c.reltype = t.oid
              )
            ORDER BY t.typname
            "#,
            &[&schema],
        ).await?;

        Ok(rows.iter().map(|row| {
            let type_char: i8 = row.get(2);
            let enum_values: Vec<String> = row.get(3);

            CustomType {
                oid: row.get::<_, i32>(0) as i64,
                schema: schema.to_string(),
                name: row.get(1),
                type_type: match type_char as u8 as char {
                    'e' => TypeKind::Enum,
                    'c' => TypeKind::Composite,
                    'd' => TypeKind::Domain,
                    'r' => TypeKind::Range,
                    _ => TypeKind::Base,
                },
                enum_values: if enum_values.is_empty() { None } else { Some(enum_values) },
                composite_attributes: None, // Could fetch if needed
                domain_base_type: None,
                domain_constraint: None,
                comment: row.get(4),
            }
        }).collect())
    }

    async fn fetch_extensions(client: &Client) -> Result<Vec<Extension>> {
        let rows = client.query(
            r#"
            SELECT
                e.extname AS name,
                e.extversion AS version,
                n.nspname AS schema,
                e.extrelocatable AS relocatable,
                obj_description(e.oid, 'pg_extension') AS comment
            FROM pg_extension e
            JOIN pg_namespace n ON n.oid = e.extnamespace
            ORDER BY e.extname
            "#,
            &[],
        ).await?;

        Ok(rows.iter().map(|row| {
            Extension {
                name: row.get(0),
                version: row.get(1),
                schema: row.get(2),
                relocatable: row.get(3),
                comment: row.get(4),
            }
        }).collect())
    }

    async fn fetch_roles(client: &Client) -> Result<Vec<Role>> {
        let rows = client.query(
            r#"
            SELECT
                r.rolname AS name,
                r.rolsuper AS is_superuser,
                r.rolcanlogin AS can_login,
                r.rolcreatedb AS can_create_db,
                r.rolcreaterole AS can_create_role,
                r.rolbypassrls AS can_bypass_rls,
                r.rolconnlimit AS connection_limit,
                r.rolvaliduntil::text AS valid_until,
                ARRAY(
                    SELECT g.rolname
                    FROM pg_auth_members m
                    JOIN pg_roles g ON g.oid = m.roleid
                    WHERE m.member = r.oid
                ) AS member_of,
                r.rolconfig AS config
            FROM pg_roles r
            WHERE r.rolname NOT LIKE 'pg_%'
            ORDER BY r.rolname
            "#,
            &[],
        ).await?;

        Ok(rows.iter().map(|row| {
            Role {
                name: row.get(0),
                is_superuser: row.get(1),
                can_login: row.get(2),
                can_create_db: row.get(3),
                can_create_role: row.get(4),
                can_bypass_rls: row.get(5),
                connection_limit: row.get(6),
                valid_until: row.get(7),
                member_of: row.get(8),
                config: row.get::<_, Option<Vec<String>>>(9).unwrap_or_default(),
            }
        }).collect())
    }
}

/// Parse function arguments string from pg_get_function_arguments
fn parse_function_arguments(args_str: String) -> Vec<FunctionArgument> {
    if args_str.is_empty() {
        return Vec::new();
    }

    // Simple parser for "name type, name type DEFAULT value" format
    args_str.split(',')
        .map(|arg| {
            let arg = arg.trim();
            let parts: Vec<_> = arg.splitn(2, ' ').collect();

            // Check for mode prefix
            let (mode, rest) = if parts[0].eq_ignore_ascii_case("IN")
                || parts[0].eq_ignore_ascii_case("OUT")
                || parts[0].eq_ignore_ascii_case("INOUT")
                || parts[0].eq_ignore_ascii_case("VARIADIC")
            {
                let mode = match parts[0].to_uppercase().as_str() {
                    "OUT" => ArgumentMode::Out,
                    "INOUT" => ArgumentMode::InOut,
                    "VARIADIC" => ArgumentMode::Variadic,
                    _ => ArgumentMode::In,
                };
                (mode, parts.get(1).map(|s| *s).unwrap_or(""))
            } else {
                (ArgumentMode::In, arg)
            };

            // Split name and type
            let type_parts: Vec<_> = rest.splitn(2, ' ').collect();
            let (name, data_type) = if type_parts.len() == 2 {
                (Some(type_parts[0].to_string()), type_parts[1].to_string())
            } else {
                (None, type_parts[0].to_string())
            };

            // Check for DEFAULT
            let (data_type, default) = if let Some(idx) = data_type.to_uppercase().find(" DEFAULT ") {
                let (dt, def) = data_type.split_at(idx);
                (dt.to_string(), Some(def[9..].to_string()))
            } else {
                (data_type, None)
            };

            FunctionArgument {
                name,
                data_type,
                mode,
                default,
            }
        })
        .collect()
}
```

### 3. Schema Cache

```rust
// src/services/schema.rs (continued)

/// In-memory cache for schema data with fast lookup indices
pub struct SchemaCache {
    /// Complete schema
    schema: RwLock<Option<DatabaseSchema>>,

    /// Quick lookup: table name -> (schema, table)
    tables_by_name: RwLock<HashMap<String, Vec<(String, String)>>>,

    /// Quick lookup: schema.table -> columns
    columns_by_table: RwLock<HashMap<String, Vec<String>>>,

    /// Quick lookup: function name -> vec of (schema, signature)
    functions_by_name: RwLock<HashMap<String, Vec<(String, String)>>>,

    /// Quick lookup: all keywords for autocomplete
    all_identifiers: RwLock<Vec<String>>,

    /// Last update timestamp
    last_updated: RwLock<std::time::Instant>,
}

impl SchemaCache {
    pub fn new() -> Self {
        Self {
            schema: RwLock::new(None),
            tables_by_name: RwLock::new(HashMap::new()),
            columns_by_table: RwLock::new(HashMap::new()),
            functions_by_name: RwLock::new(HashMap::new()),
            all_identifiers: RwLock::new(Vec::new()),
            last_updated: RwLock::new(std::time::Instant::now()),
        }
    }

    /// Update cache with new schema data
    pub fn update(&self, schema: DatabaseSchema) {
        let mut tables = HashMap::new();
        let mut columns = HashMap::new();
        let mut functions = HashMap::new();
        let mut identifiers = Vec::new();

        for s in &schema.schemas {
            identifiers.push(s.name.clone());

            for table in &s.tables {
                // Index by name (may have multiple schemas)
                tables.entry(table.name.clone())
                    .or_insert_with(Vec::new)
                    .push((s.name.clone(), table.name.clone()));

                // Index columns
                let key = format!("{}.{}", s.name, table.name);
                columns.insert(
                    key,
                    table.columns.iter().map(|c| c.name.clone()).collect(),
                );

                identifiers.push(table.name.clone());
                for col in &table.columns {
                    identifiers.push(col.name.clone());
                }
            }

            for view in &s.views {
                tables.entry(view.name.clone())
                    .or_insert_with(Vec::new)
                    .push((s.name.clone(), view.name.clone()));

                let key = format!("{}.{}", s.name, view.name);
                columns.insert(
                    key,
                    view.columns.iter().map(|c| c.name.clone()).collect(),
                );

                identifiers.push(view.name.clone());
            }

            for mat_view in &s.materialized_views {
                tables.entry(mat_view.name.clone())
                    .or_insert_with(Vec::new)
                    .push((s.name.clone(), mat_view.name.clone()));

                identifiers.push(mat_view.name.clone());
            }

            for func in &s.functions {
                functions.entry(func.name.clone())
                    .or_insert_with(Vec::new)
                    .push((s.name.clone(), func.signature()));

                identifiers.push(func.name.clone());
            }

            for seq in &s.sequences {
                identifiers.push(seq.name.clone());
            }

            for typ in &s.types {
                identifiers.push(typ.name.clone());
                if let Some(ref values) = typ.enum_values {
                    identifiers.extend(values.iter().cloned());
                }
            }
        }

        // Deduplicate identifiers
        identifiers.sort();
        identifiers.dedup();

        *self.tables_by_name.write() = tables;
        *self.columns_by_table.write() = columns;
        *self.functions_by_name.write() = functions;
        *self.all_identifiers.write() = identifiers;
        *self.schema.write() = Some(schema);
        *self.last_updated.write() = std::time::Instant::now();
    }

    /// Get the full schema
    pub fn get(&self) -> Option<DatabaseSchema> {
        self.schema.read().clone()
    }

    /// Get table names for autocomplete (returns all matching tables)
    pub fn get_table_names(&self, prefix: &str) -> Vec<(String, String)> {
        let prefix_lower = prefix.to_lowercase();
        self.tables_by_name.read()
            .iter()
            .filter(|(name, _)| name.to_lowercase().starts_with(&prefix_lower))
            .flat_map(|(_, schemas)| schemas.iter().cloned())
            .collect()
    }

    /// Get column names for a specific table
    pub fn get_columns(&self, schema: &str, table: &str) -> Vec<String> {
        let key = format!("{}.{}", schema, table);
        self.columns_by_table.read()
            .get(&key)
            .cloned()
            .unwrap_or_default()
    }

    /// Get columns for a table by name only (searches all schemas)
    pub fn get_columns_by_table_name(&self, table: &str) -> Vec<String> {
        let schemas = self.tables_by_name.read();
        if let Some(matches) = schemas.get(table) {
            if let Some((schema, table)) = matches.first() {
                return self.get_columns(schema, table);
            }
        }
        Vec::new()
    }

    /// Get function names for autocomplete
    pub fn get_function_names(&self, prefix: &str) -> Vec<(String, String)> {
        let prefix_lower = prefix.to_lowercase();
        self.functions_by_name.read()
            .iter()
            .filter(|(name, _)| name.to_lowercase().starts_with(&prefix_lower))
            .flat_map(|(_, schemas)| schemas.iter().cloned())
            .collect()
    }

    /// Get all identifiers matching a prefix
    pub fn get_identifiers(&self, prefix: &str) -> Vec<String> {
        let prefix_lower = prefix.to_lowercase();
        self.all_identifiers.read()
            .iter()
            .filter(|id| id.to_lowercase().starts_with(&prefix_lower))
            .take(50)
            .cloned()
            .collect()
    }

    /// Search for objects matching a query
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        if let Some(ref schema) = *self.schema.read() {
            for s in &schema.schemas {
                // Search tables
                for table in &s.tables {
                    if table.name.to_lowercase().contains(&query_lower) {
                        results.push(SearchResult {
                            object_type: ObjectType::Table,
                            schema: s.name.clone(),
                            name: table.name.clone(),
                            path: format!("{}.{}", s.name, table.name),
                        });
                    }
                    // Search columns
                    for col in &table.columns {
                        if col.name.to_lowercase().contains(&query_lower) {
                            results.push(SearchResult {
                                object_type: ObjectType::Column,
                                schema: s.name.clone(),
                                name: col.name.clone(),
                                path: format!("{}.{}.{}", s.name, table.name, col.name),
                            });
                        }
                    }
                }

                // Search views
                for view in &s.views {
                    if view.name.to_lowercase().contains(&query_lower) {
                        results.push(SearchResult {
                            object_type: ObjectType::View,
                            schema: s.name.clone(),
                            name: view.name.clone(),
                            path: format!("{}.{}", s.name, view.name),
                        });
                    }
                }

                // Search functions
                for func in &s.functions {
                    if func.name.to_lowercase().contains(&query_lower) {
                        results.push(SearchResult {
                            object_type: ObjectType::Function,
                            schema: s.name.clone(),
                            name: func.name.clone(),
                            path: format!("{}.{}", s.name, func.signature()),
                        });
                    }
                }

                // Search types
                for typ in &s.types {
                    if typ.name.to_lowercase().contains(&query_lower) {
                        results.push(SearchResult {
                            object_type: ObjectType::Type,
                            schema: s.name.clone(),
                            name: typ.name.clone(),
                            path: format!("{}.{}", s.name, typ.name),
                        });
                    }
                }
            }
        }

        // Sort by relevance
        results.sort_by(|a, b| {
            let a_exact = a.name.to_lowercase() == query_lower;
            let b_exact = b.name.to_lowercase() == query_lower;
            let a_starts = a.name.to_lowercase().starts_with(&query_lower);
            let b_starts = b.name.to_lowercase().starts_with(&query_lower);

            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => match (a_starts, b_starts) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            }
        });

        results.truncate(100);
        results
    }

    /// Get a specific table by name
    pub fn get_table(&self, schema: &str, name: &str) -> Option<Table> {
        self.schema.read().as_ref()?.schemas.iter()
            .find(|s| s.name == schema)?
            .tables.iter()
            .find(|t| t.name == name)
            .cloned()
    }

    /// Get a specific view by name
    pub fn get_view(&self, schema: &str, name: &str) -> Option<View> {
        self.schema.read().as_ref()?.schemas.iter()
            .find(|s| s.name == schema)?
            .views.iter()
            .find(|v| v.name == name)
            .cloned()
    }

    /// Get a specific function by name
    pub fn get_function(&self, schema: &str, name: &str) -> Option<Function> {
        self.schema.read().as_ref()?.schemas.iter()
            .find(|s| s.name == schema)?
            .functions.iter()
            .find(|f| f.name == name)
            .cloned()
    }

    /// Get time since last update
    pub fn age(&self) -> std::time::Duration {
        self.last_updated.read().elapsed()
    }
}

/// Search result
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub object_type: ObjectType,
    pub schema: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ObjectType {
    Table,
    View,
    MaterializedView,
    Function,
    Sequence,
    Type,
    Column,
    Index,
    Trigger,
    Extension,
    Role,
}

impl ObjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::View => "view",
            Self::MaterializedView => "materialized view",
            Self::Function => "function",
            Self::Sequence => "sequence",
            Self::Type => "type",
            Self::Column => "column",
            Self::Index => "index",
            Self::Trigger => "trigger",
            Self::Extension => "extension",
            Self::Role => "role",
        }
    }
}
```

### 4. Schema Change Listener

```rust
// src/services/schema.rs (continued)

use tokio::sync::oneshot;
use futures_util::StreamExt;

/// Listens for schema changes via LISTEN/NOTIFY
pub struct SchemaChangeListener {
    connection_id: Uuid,
    cancel_tx: Option<oneshot::Sender<()>>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl SchemaChangeListener {
    /// Start listening for schema changes
    pub fn start(
        connection_id: Uuid,
        pool: Arc<ConnectionPool>,
        cache: Arc<SchemaCache>,
        on_change: Box<dyn Fn() + Send + Sync>,
        runtime: Handle,
    ) -> Result<Self> {
        let (cancel_tx, cancel_rx) = oneshot::channel();

        let task_handle = runtime.spawn(async move {
            Self::listen_loop(pool, cache, on_change, cancel_rx).await;
        });

        Ok(Self {
            connection_id,
            cancel_tx: Some(cancel_tx),
            task_handle: Some(task_handle),
        })
    }

    async fn listen_loop(
        pool: Arc<ConnectionPool>,
        cache: Arc<SchemaCache>,
        on_change: Box<dyn Fn() + Send + Sync>,
        mut cancel_rx: oneshot::Receiver<()>,
    ) {
        // Get dedicated connection for LISTEN
        let client = match pool.get_dedicated().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to get connection for schema listener: {}", e);
                return;
            }
        };

        // Set up event trigger if we have permission
        let _ = Self::setup_event_trigger(&client).await;

        // Listen for schema changes
        if let Err(e) = client.execute("LISTEN schema_change", &[]).await {
            tracing::warn!("Failed to LISTEN for schema changes: {}", e);
            return;
        }

        tracing::info!("Schema change listener started");

        // Also listen for built-in DDL events if available
        let _ = client.execute("LISTEN ddl_command_end", &[]).await;

        loop {
            tokio::select! {
                _ = &mut cancel_rx => {
                    tracing::info!("Schema change listener stopped");
                    break;
                }
                notification = client.notifications().next() => {
                    match notification {
                        Some(Ok(notification)) => {
                            tracing::debug!(
                                "Schema change notification: channel={} payload={}",
                                notification.channel(),
                                notification.payload()
                            );

                            // Refresh schema
                            if let Err(e) = Self::refresh_schema(&pool, &cache).await {
                                tracing::error!("Failed to refresh schema: {}", e);
                            } else {
                                // Notify UI
                                on_change();
                            }
                        }
                        Some(Err(e)) => {
                            tracing::error!("Notification error: {}", e);
                            break;
                        }
                        None => {
                            tracing::warn!("Notification stream ended");
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Try to set up an event trigger for DDL changes
    async fn setup_event_trigger(client: &Client) -> Result<()> {
        // Create function if not exists
        client.execute(
            r#"
            CREATE OR REPLACE FUNCTION notify_schema_change()
            RETURNS event_trigger
            LANGUAGE plpgsql
            AS $$
            BEGIN
                NOTIFY schema_change;
            END;
            $$
            "#,
            &[],
        ).await?;

        // Create event trigger if not exists
        let _ = client.execute(
            r#"
            DO $$
            BEGIN
                IF NOT EXISTS (
                    SELECT 1 FROM pg_event_trigger WHERE evtname = 'tusk_schema_change'
                ) THEN
                    CREATE EVENT TRIGGER tusk_schema_change
                    ON ddl_command_end
                    EXECUTE FUNCTION notify_schema_change();
                END IF;
            END;
            $$
            "#,
            &[],
        ).await;

        Ok(())
    }

    async fn refresh_schema(pool: &ConnectionPool, cache: &SchemaCache) -> Result<()> {
        let client = pool.get().await?;
        let schema = SchemaService::fetch_schema(&client).await?;
        cache.update(schema);
        Ok(())
    }

    /// Stop the listener
    pub fn stop(&mut self) {
        if let Some(tx) = self.cancel_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

impl Drop for SchemaChangeListener {
    fn drop(&mut self) {
        self.stop();
    }
}
```

### 5. DDL Generation

```rust
// src/services/ddl.rs

use crate::models::schema::*;
use crate::error::{Result, TuskError};
use tokio_postgres::Client;

/// Service for generating DDL statements
pub struct DdlService;

impl DdlService {
    /// Generate CREATE TABLE statement
    pub fn generate_table_ddl(table: &Table) -> String {
        let mut ddl = format!("CREATE TABLE {}.{} (\n", table.schema, table.name);

        // Columns
        let column_defs: Vec<String> = table.columns.iter().map(|col| {
            let mut def = format!("    {} {}", col.name, col.data_type);

            if !col.nullable {
                def.push_str(" NOT NULL");
            }

            if let Some(ref default) = col.default {
                if !col.is_identity && !col.is_generated {
                    def.push_str(&format!(" DEFAULT {}", default));
                }
            }

            if col.is_identity {
                let gen = col.identity_generation.as_deref().unwrap_or("BY DEFAULT");
                def.push_str(&format!(" GENERATED {} AS IDENTITY", gen));
            }

            if col.is_generated {
                if let Some(ref expr) = col.generation_expression {
                    def.push_str(&format!(" GENERATED ALWAYS AS ({}) STORED", expr));
                }
            }

            def
        }).collect();

        ddl.push_str(&column_defs.join(",\n"));

        // Primary key
        if let Some(ref pk) = table.primary_key {
            ddl.push_str(&format!(
                ",\n    CONSTRAINT {} PRIMARY KEY ({})",
                pk.name,
                pk.columns.join(", ")
            ));
        }

        // Unique constraints
        for uc in &table.unique_constraints {
            ddl.push_str(&format!(
                ",\n    CONSTRAINT {} UNIQUE ({})",
                uc.name,
                uc.columns.join(", ")
            ));
        }

        // Check constraints
        for cc in &table.check_constraints {
            ddl.push_str(&format!(
                ",\n    CONSTRAINT {} CHECK {}",
                cc.name,
                cc.expression
            ));
        }

        // Foreign keys
        for fk in &table.foreign_keys {
            ddl.push_str(&format!(
                ",\n    CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}.{} ({})",
                fk.name,
                fk.columns.join(", "),
                fk.referenced_schema,
                fk.referenced_table,
                fk.referenced_columns.join(", ")
            ));

            if fk.on_update != ForeignKeyAction::NoAction {
                ddl.push_str(&format!(" ON UPDATE {}", fk.on_update.as_str()));
            }
            if fk.on_delete != ForeignKeyAction::NoAction {
                ddl.push_str(&format!(" ON DELETE {}", fk.on_delete.as_str()));
            }
            if fk.deferrable {
                ddl.push_str(" DEFERRABLE");
                if fk.initially_deferred {
                    ddl.push_str(" INITIALLY DEFERRED");
                }
            }
        }

        ddl.push_str("\n);\n");

        // Comments
        if let Some(ref comment) = table.comment {
            ddl.push_str(&format!(
                "\nCOMMENT ON TABLE {}.{} IS '{}';\n",
                table.schema,
                table.name,
                comment.replace('\'', "''")
            ));
        }

        for col in &table.columns {
            if let Some(ref comment) = col.comment {
                ddl.push_str(&format!(
                    "COMMENT ON COLUMN {}.{}.{} IS '{}';\n",
                    table.schema,
                    table.name,
                    col.name,
                    comment.replace('\'', "''")
                ));
            }
        }

        // Indexes (non-primary)
        for idx in &table.indexes {
            if !idx.is_primary {
                ddl.push_str(&format!("\n{};\n", idx.definition));
            }
        }

        // Triggers
        for trigger in &table.triggers {
            ddl.push_str(&format!(
                "\nCREATE TRIGGER {} {} {} ON {}.{}\n    FOR EACH {} EXECUTE FUNCTION {}.{}();\n",
                trigger.name,
                match trigger.timing {
                    TriggerTiming::Before => "BEFORE",
                    TriggerTiming::After => "AFTER",
                    TriggerTiming::InsteadOf => "INSTEAD OF",
                },
                trigger.events.iter().map(|e| match e {
                    TriggerEvent::Insert => "INSERT",
                    TriggerEvent::Update => "UPDATE",
                    TriggerEvent::Delete => "DELETE",
                    TriggerEvent::Truncate => "TRUNCATE",
                }).collect::<Vec<_>>().join(" OR "),
                table.schema,
                table.name,
                match trigger.for_each {
                    TriggerScope::Row => "ROW",
                    TriggerScope::Statement => "STATEMENT",
                },
                trigger.function_schema,
                trigger.function_name,
            ));
        }

        ddl
    }

    /// Generate CREATE VIEW statement
    pub fn generate_view_ddl(view: &View) -> String {
        format!(
            "CREATE OR REPLACE VIEW {}.{} AS\n{};\n",
            view.schema,
            view.name,
            view.definition
        )
    }

    /// Generate CREATE FUNCTION statement
    pub fn generate_function_ddl(func: &Function) -> String {
        let args: Vec<String> = func.arguments.iter().map(|arg| {
            let mode = match arg.mode {
                ArgumentMode::In => "",
                ArgumentMode::Out => "OUT ",
                ArgumentMode::InOut => "INOUT ",
                ArgumentMode::Variadic => "VARIADIC ",
                ArgumentMode::Table => "TABLE ",
            };
            let name = arg.name.as_ref().map(|n| format!("{} ", n)).unwrap_or_default();
            let default = arg.default.as_ref().map(|d| format!(" DEFAULT {}", d)).unwrap_or_default();
            format!("{}{}{}{}", mode, name, arg.data_type, default)
        }).collect();

        let volatility = match func.volatility {
            FunctionVolatility::Immutable => "IMMUTABLE",
            FunctionVolatility::Stable => "STABLE",
            FunctionVolatility::Volatile => "VOLATILE",
        };

        let kind_str = match func.kind {
            FunctionKind::Procedure => "PROCEDURE",
            _ => "FUNCTION",
        };

        let returns = if func.kind == FunctionKind::Procedure {
            String::new()
        } else {
            format!("\n    RETURNS {}", func.return_type)
        };

        format!(
            "CREATE OR REPLACE {} {}.{}({}){}\n    LANGUAGE {}\n    {}{}{}\nAS $function$\n{}\n$function$;\n",
            kind_str,
            func.schema,
            func.name,
            args.join(", "),
            returns,
            func.language,
            volatility,
            if func.is_strict { " STRICT" } else { "" },
            if func.is_security_definer { " SECURITY DEFINER" } else { "" },
            func.source,
        )
    }

    /// Generate DDL from database (using pg_dump style queries)
    pub async fn fetch_ddl(
        client: &Client,
        object_type: &str,
        schema: &str,
        name: &str,
    ) -> Result<String> {
        match object_type {
            "table" => {
                // Try pg_get_tabledef extension first
                let result = client.query_opt(
                    "SELECT pg_get_tabledef($1::regclass::oid)",
                    &[&format!("{}.{}", schema, name)],
                ).await?;

                if let Some(row) = result {
                    return Ok(row.get(0));
                }

                // Fall back to manual generation
                Err(TuskError::NotImplemented("Manual table DDL generation".into()))
            }
            "view" => {
                let row = client.query_one(
                    "SELECT pg_get_viewdef($1::regclass, true)",
                    &[&format!("{}.{}", schema, name)],
                ).await?;
                let def: String = row.get(0);
                Ok(format!("CREATE OR REPLACE VIEW {}.{} AS\n{}", schema, name, def))
            }
            "materialized_view" => {
                let row = client.query_one(
                    "SELECT pg_get_viewdef($1::regclass, true)",
                    &[&format!("{}.{}", schema, name)],
                ).await?;
                let def: String = row.get(0);
                Ok(format!("CREATE MATERIALIZED VIEW {}.{} AS\n{}", schema, name, def))
            }
            "function" | "procedure" => {
                let row = client.query_one(
                    "SELECT pg_get_functiondef(p.oid)
                     FROM pg_proc p
                     JOIN pg_namespace n ON n.oid = p.pronamespace
                     WHERE n.nspname = $1 AND p.proname = $2
                     LIMIT 1",
                    &[&schema, &name],
                ).await?;
                Ok(row.get(0))
            }
            "index" => {
                let row = client.query_one(
                    "SELECT indexdef FROM pg_indexes WHERE schemaname = $1 AND indexname = $2",
                    &[&schema, &name],
                ).await?;
                Ok(row.get(0))
            }
            "trigger" => {
                let row = client.query_one(
                    "SELECT pg_get_triggerdef(t.oid)
                     FROM pg_trigger t
                     JOIN pg_class c ON c.oid = t.tgrelid
                     JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = $1 AND t.tgname = $2",
                    &[&schema, &name],
                ).await?;
                Ok(row.get(0))
            }
            "sequence" => {
                // Build sequence DDL
                let row = client.query_one(
                    r#"
                    SELECT
                        format_type(s.seqtypid, NULL),
                        s.seqstart,
                        s.seqincrement,
                        s.seqmin,
                        s.seqmax,
                        s.seqcache,
                        s.seqcycle
                    FROM pg_class c
                    JOIN pg_namespace n ON n.oid = c.relnamespace
                    JOIN pg_sequence s ON s.seqrelid = c.oid
                    WHERE n.nspname = $1 AND c.relname = $2
                    "#,
                    &[&schema, &name],
                ).await?;

                let data_type: String = row.get(0);
                let start: i64 = row.get(1);
                let increment: i64 = row.get(2);
                let min: i64 = row.get(3);
                let max: i64 = row.get(4);
                let cache: i64 = row.get(5);
                let cycle: bool = row.get(6);

                Ok(format!(
                    "CREATE SEQUENCE {}.{}\n    AS {}\n    START WITH {}\n    INCREMENT BY {}\n    MINVALUE {}\n    MAXVALUE {}\n    CACHE {}\n    {};",
                    schema, name, data_type, start, increment, min, max, cache,
                    if cycle { "CYCLE" } else { "NO CYCLE" }
                ))
            }
            _ => Err(TuskError::InvalidInput(format!("Unknown object type: {}", object_type))),
        }
    }
}
```

### 6. GPUI State Integration

```rust
// src/state.rs (schema additions)

use crate::services::schema::{SchemaService, SchemaCache};

impl TuskState {
    /// Get schema service
    pub fn schema_service(&self) -> &SchemaService {
        &self.schema_service
    }

    /// Get schema cache for a connection
    pub fn get_schema_cache(&self, connection_id: &Uuid) -> Option<Arc<SchemaCache>> {
        self.schema_service.get_cache(connection_id)
    }

    /// Initialize schema for a connection
    pub fn initialize_schema(&self, connection_id: Uuid, pool: Arc<ConnectionPool>) -> Result<()> {
        self.schema_service.initialize(connection_id, pool)?;
        Ok(())
    }

    /// Refresh schema for a connection
    pub fn refresh_schema(&self, connection_id: &Uuid) -> Result<()> {
        if let Some(pool) = self.connection_service.get_pool(connection_id) {
            self.schema_service.refresh(connection_id, pool)?;
        }
        Ok(())
    }
}

// Schema state for GPUI entities
pub struct SchemaState {
    pub connection_id: Option<Uuid>,
    pub schema: Option<DatabaseSchema>,
    pub loading: bool,
    pub error: Option<String>,
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
}

impl SchemaState {
    pub fn new() -> Self {
        Self {
            connection_id: None,
            schema: None,
            loading: false,
            error: None,
            search_query: String::new(),
            search_results: Vec::new(),
        }
    }

    pub fn load(&mut self, cx: &mut Context<Self>) {
        if let Some(conn_id) = self.connection_id {
            self.loading = true;
            self.error = None;
            cx.notify();

            let tusk_state = cx.global::<TuskState>();
            if let Some(cache) = tusk_state.get_schema_cache(&conn_id) {
                if let Some(schema) = cache.get() {
                    self.schema = Some(schema);
                    self.loading = false;
                    cx.notify();
                    return;
                }
            }

            // Need to fetch
            if let Some(pool) = tusk_state.connection_service.get_pool(&conn_id) {
                match tusk_state.schema_service.initialize(conn_id, pool) {
                    Ok(cache) => {
                        self.schema = cache.get();
                        self.loading = false;
                    }
                    Err(e) => {
                        self.error = Some(e.to_string());
                        self.loading = false;
                    }
                }
                cx.notify();
            }
        }
    }

    pub fn search(&mut self, query: String, cx: &mut Context<Self>) {
        self.search_query = query.clone();

        if let Some(conn_id) = self.connection_id {
            let tusk_state = cx.global::<TuskState>();
            if let Some(cache) = tusk_state.get_schema_cache(&conn_id) {
                self.search_results = cache.search(&query);
            }
        }

        cx.notify();
    }
}
```

## Acceptance Criteria

1. [ ] All schema object types fetched correctly (tables, views, functions, etc.)
2. [ ] Schema cached in memory for fast access
3. [ ] Autocomplete data indexed for < 50ms response
4. [ ] LISTEN/NOTIFY detects schema changes
5. [ ] Schema search returns ranked results
6. [ ] DDL generation works for tables, views, functions
7. [ ] Large schemas (1000+ tables) load in < 500ms
8. [ ] Cache updates automatically on schema change
9. [ ] Thread-safe synchronous access from GPUI components

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_cache_lookup() {
        let cache = SchemaCache::new();

        let schema = DatabaseSchema {
            schemas: vec![Schema {
                name: "public".to_string(),
                tables: vec![Table {
                    name: "users".to_string(),
                    schema: "public".to_string(),
                    columns: vec![
                        Column {
                            name: "id".to_string(),
                            ordinal: 1,
                            data_type: "integer".to_string(),
                            base_type: "int4".to_string(),
                            nullable: false,
                            default: None,
                            is_identity: true,
                            identity_generation: Some("BY DEFAULT".to_string()),
                            is_generated: false,
                            generation_expression: None,
                            comment: None,
                        },
                        Column {
                            name: "email".to_string(),
                            ordinal: 2,
                            data_type: "text".to_string(),
                            base_type: "text".to_string(),
                            nullable: false,
                            default: None,
                            is_identity: false,
                            identity_generation: None,
                            is_generated: false,
                            generation_expression: None,
                            comment: None,
                        },
                    ],
                    ..Default::default()
                }],
                ..Default::default()
            }],
            extensions: vec![],
            roles: vec![],
            loaded_at: None,
        };

        cache.update(schema);

        // Test table lookup
        let tables = cache.get_table_names("us");
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0], ("public".to_string(), "users".to_string()));

        // Test column lookup
        let columns = cache.get_columns("public", "users");
        assert_eq!(columns, vec!["id", "email"]);

        // Test search
        let results = cache.search("email");
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.name == "email"));
    }

    #[test]
    fn test_parse_function_arguments() {
        let args = parse_function_arguments("name text, age integer DEFAULT 0".to_string());
        assert_eq!(args.len(), 2);
        assert_eq!(args[0].name, Some("name".to_string()));
        assert_eq!(args[0].data_type, "text");
        assert_eq!(args[1].name, Some("age".to_string()));
        assert_eq!(args[1].default, Some("0".to_string()));
    }

    #[test]
    fn test_ddl_generation() {
        let table = Table {
            oid: 1,
            schema: "public".to_string(),
            name: "users".to_string(),
            columns: vec![
                Column {
                    ordinal: 1,
                    name: "id".to_string(),
                    data_type: "integer".to_string(),
                    base_type: "int4".to_string(),
                    nullable: false,
                    default: None,
                    is_identity: true,
                    identity_generation: Some("BY DEFAULT".to_string()),
                    is_generated: false,
                    generation_expression: None,
                    comment: None,
                },
            ],
            primary_key: Some(Constraint {
                name: "users_pkey".to_string(),
                columns: vec!["id".to_string()],
            }),
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            indexes: vec![],
            triggers: vec![],
            policies: vec![],
            row_count_estimate: 0,
            size_bytes: 0,
            comment: None,
        };

        let ddl = DdlService::generate_table_ddl(&table);
        assert!(ddl.contains("CREATE TABLE public.users"));
        assert!(ddl.contains("id integer NOT NULL"));
        assert!(ddl.contains("GENERATED BY DEFAULT AS IDENTITY"));
        assert!(ddl.contains("PRIMARY KEY (id)"));
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use gpui::TestAppContext;

    #[gpui::test]
    async fn test_schema_introspection(cx: &mut TestAppContext) {
        // This would require a real database connection
        // For now, verify the service initializes correctly

        let app = cx.new(|cx| {
            let runtime = tokio::runtime::Handle::current();
            let schema_service = SchemaService::new(runtime);
            TestApp { schema_service }
        });

        // Service should start with no caches
        app.read(cx, |app, _| {
            assert!(app.schema_service.get_cache(&Uuid::new_v4()).is_none());
        });
    }
}
```

## Performance Considerations

1. **Parallel Fetching**: All schema objects for each schema are fetched in parallel using `tokio::try_join!`
2. **Indexed Lookups**: Cache maintains HashMap indices for O(1) lookups by name
3. **Prefix Search**: Autocomplete uses prefix matching for fast filtering
4. **Incremental Updates**: LISTEN/NOTIFY triggers full refresh only when changes detected
5. **Memory Efficiency**: Schema is stored once with indices pointing to same data

## Dependencies on Other Features

- 07-connection-management.md (ConnectionPool, ConnectionService)

## Dependent Features

- 12-sql-editor.md (autocomplete)
- 16-schema-browser.md (tree view)
- 17-table-data-viewer.md (table metadata)
- 26-er-diagram.md (relationships visualization)
