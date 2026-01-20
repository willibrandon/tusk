# Feature 10: Schema Introspection

## Overview

Implement the schema introspection service that queries Postgres system catalogs to retrieve schema metadata (tables, views, columns, indexes, functions, etc.). Includes in-memory caching and LISTEN/NOTIFY for real-time schema change detection.

## Goals

- Query all schema objects defined in design doc Section 3.2
- Implement efficient caching with memory-indexed structures
- Support incremental refresh via LISTEN/NOTIFY
- Provide fast autocomplete data access
- Handle large schemas (1000+ tables) efficiently

## Technical Specification

### 1. Schema Data Models

```rust
// models/schema.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    pub schemas: Vec<Schema>,
    pub extensions: Vec<Extension>,
    pub roles: Vec<Role>,
}

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
    pub method: String,
    pub size_bytes: i64,
    pub definition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKey {
    pub name: String,
    pub columns: Vec<String>,
    pub referenced_schema: String,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_delete: String,
    pub on_update: String,
    pub deferrable: bool,
    pub initially_deferred: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub name: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckConstraint {
    pub name: String,
    pub expression: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializedView {
    pub oid: i64,
    pub schema: String,
    pub name: String,
    pub columns: Vec<Column>,
    pub definition: String,
    pub is_populated: bool,
    pub size_bytes: i64,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub oid: i64,
    pub schema: String,
    pub name: String,
    pub arguments: Vec<FunctionArgument>,
    pub return_type: String,
    pub language: String,
    pub volatility: String,
    pub is_strict: bool,
    pub is_security_definer: bool,
    pub source: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionArgument {
    pub name: Option<String>,
    pub data_type: String,
    pub mode: String,
    pub default: Option<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomType {
    pub oid: i64,
    pub schema: String,
    pub name: String,
    pub type_type: String, // enum, composite, domain, range
    pub enum_values: Option<Vec<String>>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub name: String,
    pub timing: String,
    pub events: Vec<String>,
    pub function_name: String,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub name: String,
    pub command: String,
    pub roles: Vec<String>,
    pub using_expression: Option<String>,
    pub with_check: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extension {
    pub name: String,
    pub version: String,
    pub schema: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub is_superuser: bool,
    pub can_login: bool,
    pub can_create_db: bool,
    pub can_create_role: bool,
    pub connection_limit: i32,
    pub valid_until: Option<String>,
    pub member_of: Vec<String>,
}
```

### 2. Schema Service

```rust
// services/schema.rs
use std::collections::HashMap;
use tokio_postgres::Client;
use crate::error::{Result, TuskError};
use crate::models::schema::*;

pub struct SchemaService;

impl SchemaService {
    /// Fetch complete schema for a database
    pub async fn fetch_schema(client: &Client) -> Result<DatabaseSchema> {
        let schemas = Self::fetch_schemas(client).await?;
        let extensions = Self::fetch_extensions(client).await?;
        let roles = Self::fetch_roles(client).await?;

        Ok(DatabaseSchema {
            schemas,
            extensions,
            roles,
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

        let mut schemas = Vec::new();
        for row in rows {
            let schema_name: String = row.get(0);
            let schema = Self::fetch_schema_objects(client, &schema_name).await?;
            schemas.push(schema);
        }

        Ok(schemas)
    }

    async fn fetch_schema_objects(client: &Client, schema_name: &str) -> Result<Schema> {
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
        // Query from Appendix A of design doc
        let rows = client.query(
            r#"
            SELECT
                c.oid,
                c.relname AS name,
                c.reltuples::bigint AS row_count_estimate,
                pg_total_relation_size(c.oid) AS size_bytes,
                obj_description(c.oid) AS comment
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE c.relkind = 'r'
              AND n.nspname = $1
            ORDER BY c.relname
            "#,
            &[&schema],
        ).await?;

        let mut tables = Vec::new();
        for row in rows {
            let oid: i64 = row.get::<_, i32>(0) as i64;
            let name: String = row.get(1);

            // Fetch related objects
            let (columns, indexes, foreign_keys, constraints, triggers, policies) = tokio::try_join!(
                Self::fetch_columns(client, oid),
                Self::fetch_indexes(client, oid),
                Self::fetch_foreign_keys(client, oid),
                Self::fetch_constraints(client, oid),
                Self::fetch_triggers(client, oid),
                Self::fetch_policies(client, oid),
            )?;

            let primary_key = constraints.iter()
                .find(|c| c.name.ends_with("_pkey"))
                .cloned();

            let unique_constraints = constraints.iter()
                .filter(|c| !c.name.ends_with("_pkey"))
                .cloned()
                .collect();

            tables.push(Table {
                oid,
                schema: schema.to_string(),
                name,
                columns,
                primary_key,
                foreign_keys,
                unique_constraints,
                check_constraints: Vec::new(), // TODO: implement
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
        // Query from Appendix A of design doc
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
                CASE a.attidentity WHEN 'a' THEN 'ALWAYS' WHEN 'd' THEN 'BY DEFAULT' END AS identity_generation,
                a.attgenerated != '' AS is_generated,
                CASE WHEN a.attgenerated != '' THEN pg_get_expr(d.adbin, d.adrelid) END AS generation_expression,
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

        let columns = rows.iter().map(|row| {
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
        }).collect();

        Ok(columns)
    }

    async fn fetch_indexes(client: &Client, table_oid: i64) -> Result<Vec<Index>> {
        let rows = client.query(
            r#"
            SELECT
                i.indexrelid AS oid,
                c.relname AS name,
                array_agg(a.attname ORDER BY x.ordinality) AS columns,
                i.indisunique AS is_unique,
                i.indisprimary AS is_primary,
                i.indpred IS NOT NULL AS is_partial,
                pg_get_expr(i.indpred, i.indrelid) AS predicate,
                am.amname AS method,
                pg_relation_size(i.indexrelid) AS size_bytes,
                pg_get_indexdef(i.indexrelid) AS definition
            FROM pg_index i
            JOIN pg_class c ON c.oid = i.indexrelid
            JOIN pg_am am ON am.oid = c.relam
            CROSS JOIN LATERAL unnest(i.indkey) WITH ORDINALITY AS x(attnum, ordinality)
            JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = x.attnum
            WHERE i.indrelid = $1
            GROUP BY i.indexrelid, c.relname, i.indisunique, i.indisprimary, i.indpred, i.indrelid, am.amname
            ORDER BY c.relname
            "#,
            &[&(table_oid as i32)],
        ).await?;

        let indexes = rows.iter().map(|row| {
            Index {
                oid: row.get::<_, i32>(0) as i64,
                name: row.get(1),
                columns: row.get(2),
                include_columns: Vec::new(),
                is_unique: row.get(3),
                is_primary: row.get(4),
                is_partial: row.get(5),
                predicate: row.get(6),
                method: row.get(7),
                size_bytes: row.get(8),
                definition: row.get(9),
            }
        }).collect();

        Ok(indexes)
    }

    async fn fetch_foreign_keys(client: &Client, table_oid: i64) -> Result<Vec<ForeignKey>> {
        let rows = client.query(
            r#"
            SELECT
                c.conname AS name,
                array_agg(a1.attname ORDER BY x.ordinality) AS columns,
                n2.nspname AS referenced_schema,
                c2.relname AS referenced_table,
                array_agg(a2.attname ORDER BY x.ordinality) AS referenced_columns,
                CASE c.confupdtype
                    WHEN 'a' THEN 'NO ACTION'
                    WHEN 'r' THEN 'RESTRICT'
                    WHEN 'c' THEN 'CASCADE'
                    WHEN 'n' THEN 'SET NULL'
                    WHEN 'd' THEN 'SET DEFAULT'
                END AS on_update,
                CASE c.confdeltype
                    WHEN 'a' THEN 'NO ACTION'
                    WHEN 'r' THEN 'RESTRICT'
                    WHEN 'c' THEN 'CASCADE'
                    WHEN 'n' THEN 'SET NULL'
                    WHEN 'd' THEN 'SET DEFAULT'
                END AS on_delete,
                c.condeferrable AS deferrable,
                c.condeferred AS initially_deferred
            FROM pg_constraint c
            JOIN pg_class c2 ON c2.oid = c.confrelid
            JOIN pg_namespace n2 ON n2.oid = c2.relnamespace
            CROSS JOIN LATERAL unnest(c.conkey, c.confkey) WITH ORDINALITY AS x(attnum1, attnum2, ordinality)
            JOIN pg_attribute a1 ON a1.attrelid = c.conrelid AND a1.attnum = x.attnum1
            JOIN pg_attribute a2 ON a2.attrelid = c.confrelid AND a2.attnum = x.attnum2
            WHERE c.conrelid = $1
              AND c.contype = 'f'
            GROUP BY c.conname, n2.nspname, c2.relname, c.confupdtype, c.confdeltype, c.condeferrable, c.condeferred
            "#,
            &[&(table_oid as i32)],
        ).await?;

        let fks = rows.iter().map(|row| {
            ForeignKey {
                name: row.get(0),
                columns: row.get(1),
                referenced_schema: row.get(2),
                referenced_table: row.get(3),
                referenced_columns: row.get(4),
                on_update: row.get(5),
                on_delete: row.get(6),
                deferrable: row.get(7),
                initially_deferred: row.get(8),
            }
        }).collect();

        Ok(fks)
    }

    // Additional fetch methods...
    async fn fetch_constraints(client: &Client, table_oid: i64) -> Result<Vec<Constraint>> {
        // Implementation
        Ok(Vec::new())
    }

    async fn fetch_triggers(client: &Client, table_oid: i64) -> Result<Vec<Trigger>> {
        // Implementation
        Ok(Vec::new())
    }

    async fn fetch_policies(client: &Client, table_oid: i64) -> Result<Vec<Policy>> {
        // Implementation
        Ok(Vec::new())
    }

    async fn fetch_views(client: &Client, schema: &str) -> Result<Vec<View>> {
        // Implementation
        Ok(Vec::new())
    }

    async fn fetch_materialized_views(client: &Client, schema: &str) -> Result<Vec<MaterializedView>> {
        // Implementation
        Ok(Vec::new())
    }

    async fn fetch_functions(client: &Client, schema: &str) -> Result<Vec<Function>> {
        let rows = client.query(
            r#"
            SELECT
                p.oid,
                p.proname AS name,
                pg_get_function_arguments(p.oid) AS arguments,
                pg_get_function_result(p.oid) AS return_type,
                l.lanname AS language,
                CASE p.provolatile
                    WHEN 'i' THEN 'IMMUTABLE'
                    WHEN 's' THEN 'STABLE'
                    WHEN 'v' THEN 'VOLATILE'
                END AS volatility,
                p.proisstrict AS is_strict,
                p.prosecdef AS is_security_definer,
                p.prosrc AS source,
                obj_description(p.oid) AS comment
            FROM pg_proc p
            JOIN pg_namespace n ON n.oid = p.pronamespace
            JOIN pg_language l ON l.oid = p.prolang
            WHERE n.nspname = $1
              AND p.prokind = 'f'
            ORDER BY p.proname
            "#,
            &[&schema],
        ).await?;

        let functions = rows.iter().map(|row| {
            Function {
                oid: row.get::<_, i32>(0) as i64,
                schema: schema.to_string(),
                name: row.get(1),
                arguments: Vec::new(), // TODO: parse arguments
                return_type: row.get(3),
                language: row.get(4),
                volatility: row.get(5),
                is_strict: row.get(6),
                is_security_definer: row.get(7),
                source: row.get(8),
                comment: row.get(9),
            }
        }).collect();

        Ok(functions)
    }

    async fn fetch_sequences(client: &Client, schema: &str) -> Result<Vec<Sequence>> {
        // Implementation
        Ok(Vec::new())
    }

    async fn fetch_types(client: &Client, schema: &str) -> Result<Vec<CustomType>> {
        // Implementation
        Ok(Vec::new())
    }

    async fn fetch_extensions(client: &Client) -> Result<Vec<Extension>> {
        let rows = client.query(
            r#"
            SELECT
                e.extname AS name,
                e.extversion AS version,
                n.nspname AS schema,
                obj_description(e.oid, 'pg_extension') AS comment
            FROM pg_extension e
            JOIN pg_namespace n ON n.oid = e.extnamespace
            ORDER BY e.extname
            "#,
            &[],
        ).await?;

        let extensions = rows.iter().map(|row| {
            Extension {
                name: row.get(0),
                version: row.get(1),
                schema: row.get(2),
                comment: row.get(3),
            }
        }).collect();

        Ok(extensions)
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
                r.rolconnlimit AS connection_limit,
                r.rolvaliduntil::text AS valid_until,
                ARRAY(
                    SELECT g.rolname
                    FROM pg_auth_members m
                    JOIN pg_roles g ON g.oid = m.roleid
                    WHERE m.member = r.oid
                ) AS member_of
            FROM pg_roles r
            WHERE r.rolname NOT LIKE 'pg_%'
            ORDER BY r.rolname
            "#,
            &[],
        ).await?;

        let roles = rows.iter().map(|row| {
            Role {
                name: row.get(0),
                is_superuser: row.get(1),
                can_login: row.get(2),
                can_create_db: row.get(3),
                can_create_role: row.get(4),
                connection_limit: row.get(5),
                valid_until: row.get(6),
                member_of: row.get(7),
            }
        }).collect();

        Ok(roles)
    }
}
```

### 3. Schema Cache

```rust
// services/schema.rs (continued)

use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SchemaCache {
    schema: RwLock<Option<DatabaseSchema>>,
    /// Quick lookup maps for autocomplete
    tables_by_name: RwLock<HashMap<String, (String, String)>>, // name -> (schema, name)
    columns_by_table: RwLock<HashMap<String, Vec<String>>>,    // schema.table -> columns
    functions_by_name: RwLock<HashMap<String, String>>,        // name -> schema
    last_updated: RwLock<std::time::Instant>,
}

impl SchemaCache {
    pub fn new() -> Self {
        Self {
            schema: RwLock::new(None),
            tables_by_name: RwLock::new(HashMap::new()),
            columns_by_table: RwLock::new(HashMap::new()),
            functions_by_name: RwLock::new(HashMap::new()),
            last_updated: RwLock::new(std::time::Instant::now()),
        }
    }

    pub async fn update(&self, schema: DatabaseSchema) {
        // Build lookup maps
        let mut tables = HashMap::new();
        let mut columns = HashMap::new();
        let mut functions = HashMap::new();

        for s in &schema.schemas {
            for table in &s.tables {
                tables.insert(table.name.clone(), (s.name.clone(), table.name.clone()));
                let key = format!("{}.{}", s.name, table.name);
                columns.insert(
                    key,
                    table.columns.iter().map(|c| c.name.clone()).collect(),
                );
            }
            for view in &s.views {
                tables.insert(view.name.clone(), (s.name.clone(), view.name.clone()));
            }
            for func in &s.functions {
                functions.insert(func.name.clone(), s.name.clone());
            }
        }

        *self.tables_by_name.write().await = tables;
        *self.columns_by_table.write().await = columns;
        *self.functions_by_name.write().await = functions;
        *self.schema.write().await = Some(schema);
        *self.last_updated.write().await = std::time::Instant::now();
    }

    pub async fn get(&self) -> Option<DatabaseSchema> {
        self.schema.read().await.clone()
    }

    /// Get table names for autocomplete
    pub async fn get_table_names(&self) -> Vec<(String, String)> {
        self.tables_by_name.read().await
            .values()
            .cloned()
            .collect()
    }

    /// Get column names for a table
    pub async fn get_columns(&self, schema: &str, table: &str) -> Vec<String> {
        let key = format!("{}.{}", schema, table);
        self.columns_by_table.read().await
            .get(&key)
            .cloned()
            .unwrap_or_default()
    }

    /// Get function names for autocomplete
    pub async fn get_function_names(&self) -> Vec<(String, String)> {
        self.functions_by_name.read().await
            .iter()
            .map(|(name, schema)| (schema.clone(), name.clone()))
            .collect()
    }

    /// Search for objects matching a pattern
    pub async fn search(&self, query: &str) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        if let Some(ref schema) = *self.schema.read().await {
            for s in &schema.schemas {
                for table in &s.tables {
                    if table.name.to_lowercase().contains(&query_lower) {
                        results.push(SearchResult {
                            object_type: "table".to_string(),
                            schema: s.name.clone(),
                            name: table.name.clone(),
                            path: format!("{}.{}", s.name, table.name),
                        });
                    }
                    for col in &table.columns {
                        if col.name.to_lowercase().contains(&query_lower) {
                            results.push(SearchResult {
                                object_type: "column".to_string(),
                                schema: s.name.clone(),
                                name: col.name.clone(),
                                path: format!("{}.{}.{}", s.name, table.name, col.name),
                            });
                        }
                    }
                }
                for view in &s.views {
                    if view.name.to_lowercase().contains(&query_lower) {
                        results.push(SearchResult {
                            object_type: "view".to_string(),
                            schema: s.name.clone(),
                            name: view.name.clone(),
                            path: format!("{}.{}", s.name, view.name),
                        });
                    }
                }
                for func in &s.functions {
                    if func.name.to_lowercase().contains(&query_lower) {
                        results.push(SearchResult {
                            object_type: "function".to_string(),
                            schema: s.name.clone(),
                            name: func.name.clone(),
                            path: format!("{}.{}", s.name, func.name),
                        });
                    }
                }
            }
        }

        // Sort by relevance (exact match first, then starts with, then contains)
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

        results.truncate(50); // Limit results
        results
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub object_type: String,
    pub schema: String,
    pub name: String,
    pub path: String,
}
```

### 4. LISTEN/NOTIFY for Schema Changes

```rust
// services/schema.rs (continued)

pub struct SchemaChangeListener {
    connection_id: Uuid,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl SchemaChangeListener {
    pub async fn start(
        pool: Arc<ConnectionPool>,
        cache: Arc<SchemaCache>,
        app: AppHandle,
        connection_id: Uuid,
    ) -> Result<Self> {
        let (cancel_tx, mut cancel_rx) = oneshot::channel();

        // Get dedicated connection for LISTEN
        let client = pool.get_client().await?;

        // Set up LISTEN for DDL changes
        client.execute("LISTEN ddl_command_end", &[]).await?;

        // Spawn listener task
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut cancel_rx => {
                        tracing::info!("Schema change listener stopped");
                        break;
                    }
                    notification = client.notifications().next() => {
                        if let Some(Ok(notification)) = notification {
                            tracing::debug!("Schema change notification: {:?}", notification);

                            // Refresh schema
                            match Self::refresh_schema(&pool, &cache).await {
                                Ok(_) => {
                                    // Notify frontend
                                    let _ = app.emit("schema:changed", SchemaChangedEvent {
                                        connection_id,
                                    });
                                }
                                Err(e) => {
                                    tracing::error!("Failed to refresh schema: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            connection_id,
            cancel_tx: Some(cancel_tx),
        })
    }

    async fn refresh_schema(pool: &ConnectionPool, cache: &SchemaCache) -> Result<()> {
        let client = pool.get_client().await?;
        let schema = SchemaService::fetch_schema(&client).await?;
        cache.update(schema).await;
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.cancel_tx.take() {
            let _ = tx.send(());
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct SchemaChangedEvent {
    connection_id: Uuid,
}
```

### 5. Schema Commands

```rust
// commands/schema.rs
use tauri::{command, State};
use uuid::Uuid;

use crate::state::AppState;
use crate::error::Result;
use crate::models::schema::*;
use crate::services::schema::{SchemaService, SearchResult};

#[command]
pub async fn get_schema(
    state: State<'_, AppState>,
    connection_id: Uuid,
) -> Result<DatabaseSchema> {
    // Check cache first
    if let Some(cache) = state.get_schema_cache(&connection_id).await {
        if let Some(schema) = cache.get().await {
            return Ok(schema);
        }
    }

    // Fetch from database
    let pool = state.get_connection(&connection_id).await
        .ok_or(TuskError::ConnectionNotFound { id: connection_id.to_string() })?;

    let client = pool.get_client().await?;
    let schema = SchemaService::fetch_schema(&client).await?;

    // Update cache
    let cache = SchemaCache::new();
    cache.update(schema.clone()).await;
    state.set_schema_cache(connection_id, cache).await;

    Ok(schema)
}

#[command]
pub async fn refresh_schema(
    state: State<'_, AppState>,
    connection_id: Uuid,
) -> Result<DatabaseSchema> {
    let pool = state.get_connection(&connection_id).await
        .ok_or(TuskError::ConnectionNotFound { id: connection_id.to_string() })?;

    let client = pool.get_client().await?;
    let schema = SchemaService::fetch_schema(&client).await?;

    // Update cache
    if let Some(cache) = state.get_schema_cache(&connection_id).await {
        cache.update(schema.clone()).await;
    }

    Ok(schema)
}

#[command]
pub async fn search_schema(
    state: State<'_, AppState>,
    connection_id: Uuid,
    query: String,
) -> Result<Vec<SearchResult>> {
    let cache = state.get_schema_cache(&connection_id).await
        .ok_or(TuskError::NoActiveConnection)?;

    Ok(cache.search(&query).await)
}

#[command]
pub async fn get_table_columns(
    state: State<'_, AppState>,
    connection_id: Uuid,
    schema: String,
    table: String,
) -> Result<Vec<Column>> {
    let pool = state.get_connection(&connection_id).await
        .ok_or(TuskError::ConnectionNotFound { id: connection_id.to_string() })?;

    let client = pool.get_client().await?;

    // Get table OID
    let row = client.query_one(
        "SELECT c.oid FROM pg_class c
         JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = $1 AND c.relname = $2",
        &[&schema, &table],
    ).await?;

    let oid: i32 = row.get(0);
    SchemaService::fetch_columns(&client, oid as i64).await
}

#[command]
pub async fn generate_ddl(
    state: State<'_, AppState>,
    connection_id: Uuid,
    object_type: String,
    schema: String,
    name: String,
) -> Result<String> {
    let pool = state.get_connection(&connection_id).await
        .ok_or(TuskError::ConnectionNotFound { id: connection_id.to_string() })?;

    let client = pool.get_client().await?;

    let ddl = match object_type.as_str() {
        "table" => {
            let row = client.query_one(
                "SELECT pg_get_tabledef($1::regclass::oid)",
                &[&format!("{}.{}", schema, name)],
            ).await;

            // Fallback to manual DDL generation if pg_get_tabledef not available
            match row {
                Ok(r) => r.get(0),
                Err(_) => generate_table_ddl(&client, &schema, &name).await?,
            }
        }
        "view" => {
            let row = client.query_one(
                "SELECT pg_get_viewdef($1::regclass, true)",
                &[&format!("{}.{}", schema, name)],
            ).await?;
            format!("CREATE OR REPLACE VIEW {}.{} AS\n{}", schema, name, row.get::<_, String>(0))
        }
        "function" => {
            let row = client.query_one(
                "SELECT pg_get_functiondef(p.oid)
                 FROM pg_proc p
                 JOIN pg_namespace n ON n.oid = p.pronamespace
                 WHERE n.nspname = $1 AND p.proname = $2
                 LIMIT 1",
                &[&schema, &name],
            ).await?;
            row.get(0)
        }
        "index" => {
            let row = client.query_one(
                "SELECT indexdef FROM pg_indexes WHERE schemaname = $1 AND indexname = $2",
                &[&schema, &name],
            ).await?;
            row.get(0)
        }
        _ => return Err(TuskError::InvalidInput(format!("Unknown object type: {}", object_type))),
    };

    Ok(ddl)
}

async fn generate_table_ddl(client: &Client, schema: &str, table: &str) -> Result<String> {
    // Manual DDL generation for tables
    // Implementation...
    Ok(format!("-- DDL for {}.{}", schema, table))
}
```

## Acceptance Criteria

1. [ ] All schema object types fetched correctly
2. [ ] Schema cached in memory for fast access
3. [ ] Autocomplete data indexed for < 50ms response
4. [ ] LISTEN/NOTIFY detects schema changes
5. [ ] Schema search returns ranked results
6. [ ] DDL generation works for tables, views, functions
7. [ ] Large schemas (1000+ tables) load in < 500ms
8. [ ] Cache updates incrementally on schema change

## Testing with MCP

```
1. Start app: npm run tauri dev
2. Connect: driver_session action=start
3. Connect to database with tables
4. Fetch schema: ipc_execute_command command="get_schema"
5. Verify all objects returned
6. Search: ipc_execute_command command="search_schema" args={query: "user"}
7. Create table in psql
8. Verify schema:changed event
9. Test DDL: ipc_execute_command command="generate_ddl" args={...}
```

## Dependencies on Other Features

- 07-connection-management.md

## Dependent Features

- 12-monaco-editor.md (autocomplete)
- 16-schema-browser.md (tree view)
- 26-er-diagram.md (visualization)
