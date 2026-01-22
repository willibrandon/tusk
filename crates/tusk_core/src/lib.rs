//! Core types and utilities for Tusk PostgreSQL client.
//!
//! This crate provides the backend service layer for Tusk:
//!
//! - **error**: Error handling with PostgreSQL-specific details
//! - **models**: Data structures for connections, queries, and history
//! - **services**: Connection pooling, query execution, credentials, storage
//! - **state**: Application state management
//! - **logging**: Structured logging setup

pub mod error;
pub mod logging;
pub mod models;
pub mod services;
pub mod state;

pub use error::TuskError;
pub use models::{
    ColumnDetail, ColumnInfo, ConnectionConfig, ConnectionOptions, DatabaseSchema, FunctionInfo,
    PoolStatus, QueryHandle, QueryHistoryEntry, QueryResult, QueryType, SchemaInfo, SshAuthMethod,
    SshTunnelConfig, SslMode, TableInfo, ViewInfo,
};
pub use services::{ConnectionPool, CredentialService, LocalStorage, QueryService, SchemaService};
pub use state::TuskState;
