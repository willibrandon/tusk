//! Data models for Tusk PostgreSQL client.
//!
//! This module contains all core data structures:
//! - `connection` - ConnectionConfig, ConnectionStatus, SslMode, SshTunnelConfig, PoolStatus
//! - `query` - QueryHandle, QueryResult, QueryEvent, QueryType, ColumnInfo
//! - `history` - QueryHistoryEntry
//! - `schema` - Schema introspection models, SchemaCache

pub mod connection;
pub mod history;
pub mod query;
pub mod schema;

pub use connection::{
    ConnectionConfig, ConnectionOptions, ConnectionStatus, PoolStatus, SshAuthMethod,
    SshTunnelConfig, SslMode,
};
pub use history::QueryHistoryEntry;
pub use query::{ColumnInfo, QueryEvent, QueryHandle, QueryResult, QueryType};
pub use schema::{
    ColumnDetail, DatabaseSchema, FunctionInfo, SchemaCache, SchemaInfo, TableInfo, ViewInfo,
};
