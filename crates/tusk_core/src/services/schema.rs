//! Schema introspection service.
//!
//! Queries PostgreSQL system catalogs to retrieve database schema information
//! including schemas, tables, views, functions, and columns.

use std::collections::HashMap;

use crate::error::TuskError;
use crate::models::schema::{
    ColumnDetail, DatabaseSchema, FunctionInfo, SchemaInfo, TableInfo, ViewInfo,
};
use crate::services::connection::PooledConnection;

/// Schema introspection service.
///
/// Provides methods to query PostgreSQL system catalogs and retrieve
/// schema information for the schema browser.
pub struct SchemaService;

impl SchemaService {
    /// Load complete schema information for the connected database.
    ///
    /// This loads schemas, tables, views, functions, and all columns.
    pub async fn load_schema(conn: &PooledConnection) -> Result<DatabaseSchema, TuskError> {
        let schemas = Self::load_schemas(conn).await?;
        let tables = Self::load_tables(conn).await?;
        let views = Self::load_views(conn).await?;
        let functions = Self::load_functions(conn).await?;

        // Load columns for all tables and views
        let mut table_columns: HashMap<(String, String), Vec<ColumnDetail>> = HashMap::new();
        let mut view_columns: HashMap<(String, String), Vec<ColumnDetail>> = HashMap::new();

        for table in &tables {
            let columns = Self::load_columns(conn, &table.schema, &table.name).await?;
            table_columns.insert((table.schema.clone(), table.name.clone()), columns);
        }

        for view in &views {
            let columns = Self::load_columns(conn, &view.schema, &view.name).await?;
            view_columns.insert((view.schema.clone(), view.name.clone()), columns);
        }

        Ok(DatabaseSchema { schemas, tables, views, functions, table_columns, view_columns })
    }

    /// Load all schemas (excluding system schemas by default).
    pub async fn load_schemas(conn: &PooledConnection) -> Result<Vec<SchemaInfo>, TuskError> {
        let rows = conn
            .query(
                r#"
                SELECT
                    n.nspname AS name,
                    pg_get_userbyid(n.nspowner) AS owner
                FROM pg_catalog.pg_namespace n
                WHERE n.nspname NOT LIKE 'pg_%'
                  AND n.nspname != 'information_schema'
                ORDER BY n.nspname
                "#,
                &[],
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| SchemaInfo { name: row.get("name"), owner: row.get("owner") })
            .collect())
    }

    /// Load all tables in the database.
    pub async fn load_tables(conn: &PooledConnection) -> Result<Vec<TableInfo>, TuskError> {
        let rows = conn
            .query(
                r#"
                SELECT
                    n.nspname AS schema,
                    c.relname AS name,
                    pg_get_userbyid(c.relowner) AS owner,
                    c.reltuples::bigint AS estimated_rows,
                    pg_table_size(c.oid) AS size_bytes
                FROM pg_catalog.pg_class c
                JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
                WHERE c.relkind = 'r'
                  AND n.nspname NOT LIKE 'pg_%'
                  AND n.nspname != 'information_schema'
                ORDER BY n.nspname, c.relname
                "#,
                &[],
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| TableInfo {
                schema: row.get("schema"),
                name: row.get("name"),
                owner: row.get("owner"),
                estimated_rows: row.get("estimated_rows"),
                size_bytes: row.get("size_bytes"),
            })
            .collect())
    }

    /// Load all views in the database.
    pub async fn load_views(conn: &PooledConnection) -> Result<Vec<ViewInfo>, TuskError> {
        let rows = conn
            .query(
                r#"
                SELECT
                    n.nspname AS schema,
                    c.relname AS name,
                    pg_get_userbyid(c.relowner) AS owner,
                    c.relkind = 'm' AS is_materialized
                FROM pg_catalog.pg_class c
                JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
                WHERE c.relkind IN ('v', 'm')
                  AND n.nspname NOT LIKE 'pg_%'
                  AND n.nspname != 'information_schema'
                ORDER BY n.nspname, c.relname
                "#,
                &[],
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| ViewInfo {
                schema: row.get("schema"),
                name: row.get("name"),
                owner: row.get("owner"),
                is_materialized: row.get("is_materialized"),
            })
            .collect())
    }

    /// Load all functions in the database.
    pub async fn load_functions(conn: &PooledConnection) -> Result<Vec<FunctionInfo>, TuskError> {
        let rows = conn
            .query(
                r#"
                SELECT
                    n.nspname AS schema,
                    p.proname AS name,
                    pg_get_function_result(p.oid) AS return_type,
                    pg_get_function_identity_arguments(p.oid) AS arguments,
                    CASE p.provolatile
                        WHEN 'i' THEN 'IMMUTABLE'
                        WHEN 's' THEN 'STABLE'
                        WHEN 'v' THEN 'VOLATILE'
                    END AS volatility
                FROM pg_catalog.pg_proc p
                JOIN pg_catalog.pg_namespace n ON n.oid = p.pronamespace
                WHERE n.nspname NOT LIKE 'pg_%'
                  AND n.nspname != 'information_schema'
                  AND p.prokind = 'f'
                ORDER BY n.nspname, p.proname
                "#,
                &[],
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| FunctionInfo {
                schema: row.get("schema"),
                name: row.get("name"),
                return_type: row.get("return_type"),
                arguments: row.get("arguments"),
                volatility: row.get("volatility"),
            })
            .collect())
    }

    /// Load columns for a specific table or view.
    pub async fn load_columns(
        conn: &PooledConnection,
        schema: &str,
        table: &str,
    ) -> Result<Vec<ColumnDetail>, TuskError> {
        let rows = conn
            .query(
                r#"
                SELECT
                    a.attname AS name,
                    pg_catalog.format_type(a.atttypid, a.atttypmod) AS data_type,
                    NOT a.attnotnull AS is_nullable,
                    COALESCE(
                        (SELECT TRUE FROM pg_catalog.pg_constraint c
                         WHERE c.conrelid = a.attrelid
                           AND c.contype = 'p'
                           AND a.attnum = ANY(c.conkey)),
                        FALSE
                    ) AS is_primary_key,
                    pg_get_expr(d.adbin, d.adrelid) AS default_value,
                    a.attnum::integer AS ordinal_position
                FROM pg_catalog.pg_attribute a
                JOIN pg_catalog.pg_class c ON c.oid = a.attrelid
                JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
                LEFT JOIN pg_catalog.pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
                WHERE n.nspname = $1
                  AND c.relname = $2
                  AND a.attnum > 0
                  AND NOT a.attisdropped
                ORDER BY a.attnum
                "#,
                &[&schema, &table],
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| ColumnDetail {
                name: row.get("name"),
                data_type: row.get("data_type"),
                is_nullable: row.get("is_nullable"),
                is_primary_key: row.get("is_primary_key"),
                default_value: row.get("default_value"),
                ordinal_position: row.get("ordinal_position"),
            })
            .collect())
    }
}
