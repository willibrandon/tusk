//! Schema introspection models.
//!
//! Data structures representing PostgreSQL database objects for the schema browser.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

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

/// Default schema cache time-to-live (5 minutes).
const DEFAULT_SCHEMA_CACHE_TTL_SECS: u64 = 300;

/// Cached database schema with TTL support (FR-016, FR-017, FR-018).
///
/// Caches schema data per connection with automatic expiration.
/// The cache is invalidated when:
/// - TTL expires (default 5 minutes)
/// - User explicitly refreshes
/// - Connection is closed
#[derive(Debug, Clone)]
pub struct SchemaCache {
    /// Connection this cache belongs to
    connection_id: Uuid,
    /// Cached schema data
    schema: DatabaseSchema,
    /// When cache was populated
    loaded_at: Instant,
    /// Time-to-live for cache validity
    ttl: Duration,
}

impl SchemaCache {
    /// Create a new schema cache with default TTL (5 minutes).
    pub fn new(connection_id: Uuid, schema: DatabaseSchema) -> Self {
        Self {
            connection_id,
            schema,
            loaded_at: Instant::now(),
            ttl: Duration::from_secs(DEFAULT_SCHEMA_CACHE_TTL_SECS),
        }
    }

    /// Create a schema cache with custom TTL.
    pub fn with_ttl(connection_id: Uuid, schema: DatabaseSchema, ttl: Duration) -> Self {
        Self { connection_id, schema, loaded_at: Instant::now(), ttl }
    }

    /// Get the connection ID this cache belongs to.
    pub fn connection_id(&self) -> Uuid {
        self.connection_id
    }

    /// Get the cached schema data.
    pub fn schema(&self) -> &DatabaseSchema {
        &self.schema
    }

    /// Get when the cache was loaded.
    pub fn loaded_at(&self) -> Instant {
        self.loaded_at
    }

    /// Get the cache TTL.
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Get elapsed time since cache was loaded.
    pub fn elapsed(&self) -> Duration {
        self.loaded_at.elapsed()
    }

    /// Check if the cache has expired (FR-018).
    pub fn is_expired(&self) -> bool {
        self.loaded_at.elapsed() > self.ttl
    }

    /// Check if the cache is still valid.
    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }

    /// Get time remaining until expiration.
    pub fn time_remaining(&self) -> Option<Duration> {
        let elapsed = self.loaded_at.elapsed();
        if elapsed > self.ttl {
            None
        } else {
            Some(self.ttl - elapsed)
        }
    }

    /// Refresh the cache with new schema data (FR-017).
    ///
    /// Resets the loaded_at timestamp.
    pub fn refresh(&mut self, schema: DatabaseSchema) {
        self.schema = schema;
        self.loaded_at = Instant::now();
    }

    /// Consume the cache and return the schema.
    pub fn into_schema(self) -> DatabaseSchema {
        self.schema
    }
}
