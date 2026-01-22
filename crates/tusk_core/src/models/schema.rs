//! Schema introspection models.
//!
//! Data structures representing PostgreSQL database objects for the schema browser.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A PostgreSQL schema (namespace).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    /// Schema name (e.g., "public", "pg_catalog").
    pub name: String,
    /// Schema owner.
    pub owner: String,
}

/// A PostgreSQL table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    /// Schema name containing this table.
    pub schema: String,
    /// Table name.
    pub name: String,
    /// Table owner.
    pub owner: String,
    /// Estimated row count from pg_class.reltuples.
    pub estimated_rows: i64,
    /// Table size in bytes.
    pub size_bytes: i64,
}

/// A PostgreSQL view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewInfo {
    /// Schema name containing this view.
    pub schema: String,
    /// View name.
    pub name: String,
    /// View owner.
    pub owner: String,
    /// Whether this is a materialized view.
    pub is_materialized: bool,
}

/// A PostgreSQL function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    /// Schema name containing this function.
    pub schema: String,
    /// Function name.
    pub name: String,
    /// Return type.
    pub return_type: String,
    /// Argument types as a formatted string.
    pub arguments: String,
    /// Function volatility (IMMUTABLE, STABLE, VOLATILE).
    pub volatility: String,
}

/// A PostgreSQL column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDetail {
    /// Column name.
    pub name: String,
    /// Data type (e.g., "integer", "varchar(255)").
    pub data_type: String,
    /// Whether the column allows NULL values.
    pub is_nullable: bool,
    /// Whether this column is part of the primary key.
    pub is_primary_key: bool,
    /// Default value expression, if any.
    pub default_value: Option<String>,
    /// Column position (1-based ordinal).
    pub ordinal_position: i32,
}

/// Complete schema information for a database.
#[derive(Debug, Clone, Default)]
pub struct DatabaseSchema {
    /// All schemas in the database.
    pub schemas: Vec<SchemaInfo>,
    /// All tables in the database.
    pub tables: Vec<TableInfo>,
    /// All views in the database.
    pub views: Vec<ViewInfo>,
    /// All functions in the database.
    pub functions: Vec<FunctionInfo>,
    /// Columns for each table, keyed by (schema, table_name).
    pub table_columns: HashMap<(String, String), Vec<ColumnDetail>>,
    /// Columns for each view, keyed by (schema, view_name).
    pub view_columns: HashMap<(String, String), Vec<ColumnDetail>>,
}
