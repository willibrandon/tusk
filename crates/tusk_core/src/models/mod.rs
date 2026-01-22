//! Data models for Tusk PostgreSQL client.
//!
//! This module contains all core data structures:
//! - `connection` - ConnectionConfig, SslMode, SshTunnelConfig, PoolStatus
//! - `query` - QueryHandle, QueryResult, QueryType, ColumnInfo
//! - `history` - QueryHistoryEntry
//! - `schema` - Schema introspection models

pub mod connection;
pub mod history;
pub mod query;
pub mod schema;

pub use connection::{
    ConnectionConfig, ConnectionOptions, PoolStatus, SshAuthMethod, SshTunnelConfig, SslMode,
};
pub use history::QueryHistoryEntry;
pub use query::{ColumnInfo, QueryHandle, QueryResult, QueryType};
pub use schema::{ColumnDetail, DatabaseSchema, FunctionInfo, SchemaInfo, TableInfo, ViewInfo};
