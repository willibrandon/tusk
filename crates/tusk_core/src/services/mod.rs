//! Backend services for Tusk PostgreSQL client.
//!
//! This module contains all service layer abstractions:
//! - `connection` - Database connection pooling with deadpool-postgres
//! - `query` - Query execution with cancellation support
//! - `credentials` - OS keychain integration for secure credential storage
//! - `storage` - Local SQLite storage for metadata and preferences
//! - `schema` - Schema introspection for the schema browser

pub mod connection;
pub mod credentials;
pub mod query;
pub mod schema;
pub mod storage;

pub use connection::ConnectionPool;
pub use credentials::CredentialService;
pub use query::QueryService;
pub use schema::SchemaService;
pub use storage::LocalStorage;
