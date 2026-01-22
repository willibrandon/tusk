//! Query history models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::QueryResult;

/// Record of a previously executed query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHistoryEntry {
    /// Auto-increment ID from database
    pub id: i64,
    /// Associated connection
    pub connection_id: Uuid,
    /// The executed SQL
    pub sql: String,
    /// Time to execute (None if not completed)
    pub execution_time_ms: Option<i64>,
    /// Rows returned/affected
    pub row_count: Option<i64>,
    /// Error message if query failed
    pub error_message: Option<String>,
    /// Execution timestamp
    pub executed_at: DateTime<Utc>,
}

impl QueryHistoryEntry {
    /// Create a history entry from a successful query result.
    pub fn from_result(connection_id: Uuid, sql: impl Into<String>, result: &QueryResult) -> Self {
        Self {
            id: 0, // Set by database
            connection_id,
            sql: sql.into(),
            execution_time_ms: Some(result.execution_time_ms as i64),
            row_count: Some(result.rows.len() as i64),
            error_message: None,
            executed_at: Utc::now(),
        }
    }

    /// Create a history entry from a failed query.
    pub fn from_error(
        connection_id: Uuid,
        sql: impl Into<String>,
        error: impl std::fmt::Display,
    ) -> Self {
        Self {
            id: 0, // Set by database
            connection_id,
            sql: sql.into(),
            execution_time_ms: None,
            row_count: None,
            error_message: Some(error.to_string()),
            executed_at: Utc::now(),
        }
    }

    /// Create a new history entry.
    pub fn new(connection_id: Uuid, sql: impl Into<String>) -> Self {
        Self {
            id: 0,
            connection_id,
            sql: sql.into(),
            execution_time_ms: None,
            row_count: None,
            error_message: None,
            executed_at: Utc::now(),
        }
    }

    /// Check if this entry represents a successful query.
    pub fn is_success(&self) -> bool {
        self.error_message.is_none()
    }

    /// Check if this entry represents a failed query.
    pub fn is_error(&self) -> bool {
        self.error_message.is_some()
    }

    /// Get a truncated version of the SQL for display.
    pub fn sql_preview(&self, max_len: usize) -> &str {
        if self.sql.len() <= max_len {
            &self.sql
        } else {
            &self.sql[..max_len]
        }
    }
}
