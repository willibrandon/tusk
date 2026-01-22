//! Panel implementations for docks.
//!
//! This module contains concrete panel implementations that live inside docks:
//! - Schema browser panel (left dock)
//! - Results panel (bottom dock)
//! - Messages panel (bottom dock)

pub mod messages;
pub mod results;
pub mod schema_browser;

pub use messages::{Message, MessageSeverity, MessagesPanel};
pub use results::{
    DisplayColumn, DisplayError, DisplayRow, ResultsPanel, ResultsPanelState, ResultsState,
    ResultsStatus,
};
pub use schema_browser::{
    database_schema_to_tree, SchemaBrowserEvent, SchemaBrowserPanel, SchemaItem,
};
