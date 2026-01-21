//! Panel implementations for docks.
//!
//! This module contains concrete panel implementations that live inside docks:
//! - Schema browser panel (left dock)
//! - Results panel (bottom dock, future)
//! - Messages panel (bottom dock, future)

pub mod schema_browser;

pub use schema_browser::{database_schema_to_tree, SchemaItem, SchemaBrowserPanel};
