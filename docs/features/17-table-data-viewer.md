# Feature 17: Table Data Viewer

## Overview

The table data viewer provides a dedicated interface for browsing and filtering table data. It combines the results grid with a visual filter builder, sortable columns, and pagination. This is the primary way users explore table contents without writing SQL. Built entirely in Rust using GPUI for native performance.

## Goals

- Display table data with all grid features
- Provide visual filter builder with type-aware operators
- Support multi-column sorting
- Paginate large tables efficiently
- Show table metadata (columns, constraints, indexes)
- Enable transition to SQL query for complex filters

## Dependencies

- Feature 14: Results Grid (data display)
- Feature 11: Query Execution (data fetching)
- Feature 10: Schema Introspection (column metadata)

## Technical Specification

### 17.1 Table Viewer Models

```rust
// src/models/table_viewer.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Filter operator types categorized by data type compatibility
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterOperator {
    // Universal operators
    Equal,
    NotEqual,
    IsNull,
    IsNotNull,
    In,
    NotIn,

    // Comparison operators (numeric, date, text)
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Between,
    NotBetween,

    // Text operators
    Like,
    NotLike,
    ILike,
    NotILike,
    SimilarTo,
    NotSimilarTo,
    RegexMatch,
    RegexMatchInsensitive,
    NotRegexMatch,
    StartsWith,
    EndsWith,
    Contains,

    // Boolean operators
    IsTrue,
    IsFalse,
    IsNotTrue,
    IsNotFalse,
    IsUnknown,
    IsNotUnknown,

    // JSON/JSONB operators
    JsonContains,        // @>
    JsonContainedBy,     // <@
    JsonHasKey,          // ?
    JsonHasAnyKey,       // ?|
    JsonHasAllKeys,      // ?&
    JsonPathExists,      // @?
    JsonPathMatch,       // @@

    // Array operators
    ArrayContains,       // @>
    ArrayContainedBy,    // <@
    ArrayOverlaps,       // &&
    ArrayAnyEqual,       // = ANY()
    ArrayAllEqual,       // = ALL()

    // Range operators
    RangeContains,
    RangeContainedBy,
    RangeOverlaps,
    RangeAdjacentTo,
    RangeLeftOf,
    RangeRightOf,

    // Network operators (inet, cidr)
    NetworkContains,
    NetworkContainedBy,
    NetworkContainsOrEquals,
    NetworkContainedByOrEquals,

    // Full-text search
    TextSearchMatch,     // @@
    TextSearchMatchPhrase,
}

impl FilterOperator {
    /// Get display label for the operator
    pub fn label(&self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::NotEqual => "≠",
            Self::IsNull => "IS NULL",
            Self::IsNotNull => "IS NOT NULL",
            Self::In => "IN",
            Self::NotIn => "NOT IN",
            Self::LessThan => "<",
            Self::LessThanOrEqual => "≤",
            Self::GreaterThan => ">",
            Self::GreaterThanOrEqual => "≥",
            Self::Between => "BETWEEN",
            Self::NotBetween => "NOT BETWEEN",
            Self::Like => "LIKE",
            Self::NotLike => "NOT LIKE",
            Self::ILike => "ILIKE",
            Self::NotILike => "NOT ILIKE",
            Self::SimilarTo => "SIMILAR TO",
            Self::NotSimilarTo => "NOT SIMILAR TO",
            Self::RegexMatch => "~",
            Self::RegexMatchInsensitive => "~*",
            Self::NotRegexMatch => "!~",
            Self::StartsWith => "starts with",
            Self::EndsWith => "ends with",
            Self::Contains => "contains",
            Self::IsTrue => "IS TRUE",
            Self::IsFalse => "IS FALSE",
            Self::IsNotTrue => "IS NOT TRUE",
            Self::IsNotFalse => "IS NOT FALSE",
            Self::IsUnknown => "IS UNKNOWN",
            Self::IsNotUnknown => "IS NOT UNKNOWN",
            Self::JsonContains => "@> (contains)",
            Self::JsonContainedBy => "<@ (contained by)",
            Self::JsonHasKey => "? (has key)",
            Self::JsonHasAnyKey => "?| (has any key)",
            Self::JsonHasAllKeys => "?& (has all keys)",
            Self::JsonPathExists => "@? (path exists)",
            Self::JsonPathMatch => "@@ (path match)",
            Self::ArrayContains => "@> (contains)",
            Self::ArrayContainedBy => "<@ (contained by)",
            Self::ArrayOverlaps => "&& (overlaps)",
            Self::ArrayAnyEqual => "= ANY",
            Self::ArrayAllEqual => "= ALL",
            Self::RangeContains => "@> (contains)",
            Self::RangeContainedBy => "<@ (contained by)",
            Self::RangeOverlaps => "&& (overlaps)",
            Self::RangeAdjacentTo => "-|- (adjacent)",
            Self::RangeLeftOf => "<< (left of)",
            Self::RangeRightOf => ">> (right of)",
            Self::NetworkContains => ">> (contains)",
            Self::NetworkContainedBy => "<< (contained by)",
            Self::NetworkContainsOrEquals => ">>= (contains or equals)",
            Self::NetworkContainedByOrEquals => "<<= (contained by or equals)",
            Self::TextSearchMatch => "@@ (matches)",
            Self::TextSearchMatchPhrase => "@@ phrase",
        }
    }

    /// Check if this operator requires a value
    pub fn requires_value(&self) -> bool {
        !matches!(
            self,
            Self::IsNull | Self::IsNotNull |
            Self::IsTrue | Self::IsFalse |
            Self::IsNotTrue | Self::IsNotFalse |
            Self::IsUnknown | Self::IsNotUnknown
        )
    }

    /// Check if this operator takes two values (BETWEEN)
    pub fn is_range_operator(&self) -> bool {
        matches!(self, Self::Between | Self::NotBetween)
    }

    /// Check if this operator takes a list of values (IN)
    pub fn is_list_operator(&self) -> bool {
        matches!(self, Self::In | Self::NotIn | Self::JsonHasAnyKey | Self::JsonHasAllKeys)
    }

    /// Get operators available for a given PostgreSQL type
    pub fn for_type(pg_type: &str) -> Vec<Self> {
        let base_type = pg_type.trim_end_matches("[]");
        let is_array = pg_type.ends_with("[]");

        if is_array {
            return vec![
                Self::Equal,
                Self::NotEqual,
                Self::IsNull,
                Self::IsNotNull,
                Self::ArrayContains,
                Self::ArrayContainedBy,
                Self::ArrayOverlaps,
                Self::ArrayAnyEqual,
            ];
        }

        match base_type {
            // Numeric types
            "int2" | "int4" | "int8" | "float4" | "float8" | "numeric" | "money" => vec![
                Self::Equal,
                Self::NotEqual,
                Self::LessThan,
                Self::LessThanOrEqual,
                Self::GreaterThan,
                Self::GreaterThanOrEqual,
                Self::Between,
                Self::NotBetween,
                Self::In,
                Self::NotIn,
                Self::IsNull,
                Self::IsNotNull,
            ],

            // Text types
            "text" | "varchar" | "char" | "bpchar" | "name" => vec![
                Self::Equal,
                Self::NotEqual,
                Self::Like,
                Self::NotLike,
                Self::ILike,
                Self::NotILike,
                Self::SimilarTo,
                Self::RegexMatch,
                Self::RegexMatchInsensitive,
                Self::StartsWith,
                Self::EndsWith,
                Self::Contains,
                Self::In,
                Self::NotIn,
                Self::IsNull,
                Self::IsNotNull,
            ],

            // Date/time types
            "date" | "timestamp" | "timestamptz" | "time" | "timetz" | "interval" => vec![
                Self::Equal,
                Self::NotEqual,
                Self::LessThan,
                Self::LessThanOrEqual,
                Self::GreaterThan,
                Self::GreaterThanOrEqual,
                Self::Between,
                Self::NotBetween,
                Self::IsNull,
                Self::IsNotNull,
            ],

            // Boolean
            "bool" => vec![
                Self::IsTrue,
                Self::IsFalse,
                Self::IsNotTrue,
                Self::IsNotFalse,
                Self::IsNull,
                Self::IsNotNull,
            ],

            // JSON/JSONB
            "json" | "jsonb" => vec![
                Self::Equal,
                Self::NotEqual,
                Self::JsonContains,
                Self::JsonContainedBy,
                Self::JsonHasKey,
                Self::JsonHasAnyKey,
                Self::JsonHasAllKeys,
                Self::JsonPathExists,
                Self::JsonPathMatch,
                Self::IsNull,
                Self::IsNotNull,
            ],

            // UUID
            "uuid" => vec![
                Self::Equal,
                Self::NotEqual,
                Self::In,
                Self::NotIn,
                Self::IsNull,
                Self::IsNotNull,
            ],

            // Network types
            "inet" | "cidr" => vec![
                Self::Equal,
                Self::NotEqual,
                Self::LessThan,
                Self::LessThanOrEqual,
                Self::GreaterThan,
                Self::GreaterThanOrEqual,
                Self::NetworkContains,
                Self::NetworkContainedBy,
                Self::NetworkContainsOrEquals,
                Self::NetworkContainedByOrEquals,
                Self::IsNull,
                Self::IsNotNull,
            ],

            // Range types
            "int4range" | "int8range" | "numrange" | "daterange" | "tsrange" | "tstzrange" => vec![
                Self::Equal,
                Self::NotEqual,
                Self::RangeContains,
                Self::RangeContainedBy,
                Self::RangeOverlaps,
                Self::RangeAdjacentTo,
                Self::RangeLeftOf,
                Self::RangeRightOf,
                Self::IsNull,
                Self::IsNotNull,
            ],

            // Full-text search
            "tsvector" => vec![
                Self::TextSearchMatch,
                Self::IsNull,
                Self::IsNotNull,
            ],

            // Geometric types (basic support)
            "point" | "line" | "lseg" | "box" | "path" | "polygon" | "circle" => vec![
                Self::Equal,
                Self::NotEqual,
                Self::IsNull,
                Self::IsNotNull,
            ],

            // Binary
            "bytea" => vec![
                Self::Equal,
                Self::NotEqual,
                Self::IsNull,
                Self::IsNotNull,
            ],

            // Default
            _ => vec![
                Self::Equal,
                Self::NotEqual,
                Self::IsNull,
                Self::IsNotNull,
            ],
        }
    }
}

/// A single filter condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableFilter {
    pub id: Uuid,
    pub column: String,
    pub column_type: String,
    pub operator: FilterOperator,
    pub value: Option<String>,
    pub value2: Option<String>,  // For BETWEEN operator
    pub enabled: bool,
}

impl TableFilter {
    pub fn new(column: String, column_type: String, operator: FilterOperator) -> Self {
        Self {
            id: Uuid::new_v4(),
            column,
            column_type,
            operator,
            value: None,
            value2: None,
            enabled: true,
        }
    }

    /// Convert filter to SQL WHERE clause fragment
    pub fn to_sql(&self, param_index: &mut i32) -> (String, Vec<String>) {
        let col = format!("\"{}\"", self.column);
        let mut params = Vec::new();

        let sql = match &self.operator {
            FilterOperator::IsNull => format!("{} IS NULL", col),
            FilterOperator::IsNotNull => format!("{} IS NOT NULL", col),
            FilterOperator::IsTrue => format!("{} IS TRUE", col),
            FilterOperator::IsFalse => format!("{} IS FALSE", col),
            FilterOperator::IsNotTrue => format!("{} IS NOT TRUE", col),
            FilterOperator::IsNotFalse => format!("{} IS NOT FALSE", col),
            FilterOperator::IsUnknown => format!("{} IS UNKNOWN", col),
            FilterOperator::IsNotUnknown => format!("{} IS NOT UNKNOWN", col),

            FilterOperator::Equal => {
                if let Some(v) = &self.value {
                    params.push(v.clone());
                    *param_index += 1;
                    format!("{} = ${}", col, *param_index)
                } else {
                    format!("{} IS NULL", col)
                }
            }
            FilterOperator::NotEqual => {
                if let Some(v) = &self.value {
                    params.push(v.clone());
                    *param_index += 1;
                    format!("{} <> ${}", col, *param_index)
                } else {
                    format!("{} IS NOT NULL", col)
                }
            }
            FilterOperator::LessThan => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} < ${}", col, *param_index)
            }
            FilterOperator::LessThanOrEqual => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} <= ${}", col, *param_index)
            }
            FilterOperator::GreaterThan => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} > ${}", col, *param_index)
            }
            FilterOperator::GreaterThanOrEqual => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} >= ${}", col, *param_index)
            }
            FilterOperator::Between => {
                params.push(self.value.clone().unwrap_or_default());
                params.push(self.value2.clone().unwrap_or_default());
                *param_index += 1;
                let p1 = *param_index;
                *param_index += 1;
                let p2 = *param_index;
                format!("{} BETWEEN ${} AND ${}", col, p1, p2)
            }
            FilterOperator::NotBetween => {
                params.push(self.value.clone().unwrap_or_default());
                params.push(self.value2.clone().unwrap_or_default());
                *param_index += 1;
                let p1 = *param_index;
                *param_index += 1;
                let p2 = *param_index;
                format!("{} NOT BETWEEN ${} AND ${}", col, p1, p2)
            }
            FilterOperator::In => {
                let values: Vec<&str> = self.value
                    .as_ref()
                    .map(|v| v.split(',').map(|s| s.trim()).collect())
                    .unwrap_or_default();
                let placeholders: Vec<String> = values.iter().map(|v| {
                    params.push(v.to_string());
                    *param_index += 1;
                    format!("${}", *param_index)
                }).collect();
                format!("{} IN ({})", col, placeholders.join(", "))
            }
            FilterOperator::NotIn => {
                let values: Vec<&str> = self.value
                    .as_ref()
                    .map(|v| v.split(',').map(|s| s.trim()).collect())
                    .unwrap_or_default();
                let placeholders: Vec<String> = values.iter().map(|v| {
                    params.push(v.to_string());
                    *param_index += 1;
                    format!("${}", *param_index)
                }).collect();
                format!("{} NOT IN ({})", col, placeholders.join(", "))
            }
            FilterOperator::Like => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} LIKE ${}", col, *param_index)
            }
            FilterOperator::NotLike => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} NOT LIKE ${}", col, *param_index)
            }
            FilterOperator::ILike => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} ILIKE ${}", col, *param_index)
            }
            FilterOperator::NotILike => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} NOT ILIKE ${}", col, *param_index)
            }
            FilterOperator::SimilarTo => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} SIMILAR TO ${}", col, *param_index)
            }
            FilterOperator::NotSimilarTo => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} NOT SIMILAR TO ${}", col, *param_index)
            }
            FilterOperator::RegexMatch => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} ~ ${}", col, *param_index)
            }
            FilterOperator::RegexMatchInsensitive => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} ~* ${}", col, *param_index)
            }
            FilterOperator::NotRegexMatch => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} !~ ${}", col, *param_index)
            }
            FilterOperator::StartsWith => {
                let pattern = format!("{}%", self.value.clone().unwrap_or_default());
                params.push(pattern);
                *param_index += 1;
                format!("{} LIKE ${}", col, *param_index)
            }
            FilterOperator::EndsWith => {
                let pattern = format!("%{}", self.value.clone().unwrap_or_default());
                params.push(pattern);
                *param_index += 1;
                format!("{} LIKE ${}", col, *param_index)
            }
            FilterOperator::Contains => {
                let pattern = format!("%{}%", self.value.clone().unwrap_or_default());
                params.push(pattern);
                *param_index += 1;
                format!("{} ILIKE ${}", col, *param_index)
            }
            FilterOperator::JsonContains => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} @> ${}::jsonb", col, *param_index)
            }
            FilterOperator::JsonContainedBy => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} <@ ${}::jsonb", col, *param_index)
            }
            FilterOperator::JsonHasKey => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} ? ${}", col, *param_index)
            }
            FilterOperator::JsonHasAnyKey => {
                let keys: Vec<&str> = self.value
                    .as_ref()
                    .map(|v| v.split(',').map(|s| s.trim()).collect())
                    .unwrap_or_default();
                let array_literal = format!("ARRAY[{}]",
                    keys.iter().map(|k| format!("'{}'", k)).collect::<Vec<_>>().join(", "));
                format!("{} ?| {}", col, array_literal)
            }
            FilterOperator::JsonHasAllKeys => {
                let keys: Vec<&str> = self.value
                    .as_ref()
                    .map(|v| v.split(',').map(|s| s.trim()).collect())
                    .unwrap_or_default();
                let array_literal = format!("ARRAY[{}]",
                    keys.iter().map(|k| format!("'{}'", k)).collect::<Vec<_>>().join(", "));
                format!("{} ?& {}", col, array_literal)
            }
            FilterOperator::JsonPathExists => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} @? ${}::jsonpath", col, *param_index)
            }
            FilterOperator::JsonPathMatch => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} @@ ${}::jsonpath", col, *param_index)
            }
            FilterOperator::ArrayContains => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} @> ${}", col, *param_index)
            }
            FilterOperator::ArrayContainedBy => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} <@ ${}", col, *param_index)
            }
            FilterOperator::ArrayOverlaps => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} && ${}", col, *param_index)
            }
            FilterOperator::ArrayAnyEqual => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("${} = ANY({})", *param_index, col)
            }
            FilterOperator::ArrayAllEqual => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("${} = ALL({})", *param_index, col)
            }
            FilterOperator::RangeContains => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} @> ${}", col, *param_index)
            }
            FilterOperator::RangeContainedBy => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} <@ ${}", col, *param_index)
            }
            FilterOperator::RangeOverlaps => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} && ${}", col, *param_index)
            }
            FilterOperator::RangeAdjacentTo => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} -|- ${}", col, *param_index)
            }
            FilterOperator::RangeLeftOf => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} << ${}", col, *param_index)
            }
            FilterOperator::RangeRightOf => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} >> ${}", col, *param_index)
            }
            FilterOperator::NetworkContains => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} >> ${}::inet", col, *param_index)
            }
            FilterOperator::NetworkContainedBy => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} << ${}::inet", col, *param_index)
            }
            FilterOperator::NetworkContainsOrEquals => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} >>= ${}::inet", col, *param_index)
            }
            FilterOperator::NetworkContainedByOrEquals => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} <<= ${}::inet", col, *param_index)
            }
            FilterOperator::TextSearchMatch => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} @@ plainto_tsquery(${})::tsquery", col, *param_index)
            }
            FilterOperator::TextSearchMatchPhrase => {
                params.push(self.value.clone().unwrap_or_default());
                *param_index += 1;
                format!("{} @@ phraseto_tsquery(${})::tsquery", col, *param_index)
            }
        };

        (sql, params)
    }

    /// Convert filter to display-friendly SQL (for showing in UI)
    pub fn to_display_sql(&self) -> String {
        let col = &self.column;

        match &self.operator {
            FilterOperator::IsNull => format!("{} IS NULL", col),
            FilterOperator::IsNotNull => format!("{} IS NOT NULL", col),
            FilterOperator::IsTrue => format!("{} IS TRUE", col),
            FilterOperator::IsFalse => format!("{} IS FALSE", col),
            FilterOperator::Between | FilterOperator::NotBetween => {
                let op = if matches!(self.operator, FilterOperator::Between) { "BETWEEN" } else { "NOT BETWEEN" };
                format!("{} {} '{}' AND '{}'", col, op,
                    self.value.as_deref().unwrap_or("?"),
                    self.value2.as_deref().unwrap_or("?"))
            }
            _ => {
                let op_str = self.operator.label();
                let val = self.value.as_deref().unwrap_or("?");
                format!("{} {} '{}'", col, op_str, val)
            }
        }
    }
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn toggle(&self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }

    pub fn sql(&self) -> &'static str {
        match self {
            Self::Ascending => "ASC",
            Self::Descending => "DESC",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Ascending => "↑",
            Self::Descending => "↓",
        }
    }
}

/// Null handling for sorting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NullsPosition {
    First,
    Last,
    Default,
}

impl NullsPosition {
    pub fn sql(&self) -> &'static str {
        match self {
            Self::First => " NULLS FIRST",
            Self::Last => " NULLS LAST",
            Self::Default => "",
        }
    }
}

/// A sort specification for a column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSort {
    pub column: String,
    pub direction: SortDirection,
    pub nulls: NullsPosition,
    pub priority: usize,  // Lower = higher priority in multi-sort
}

impl TableSort {
    pub fn new(column: String, direction: SortDirection) -> Self {
        Self {
            column,
            direction,
            nulls: NullsPosition::Default,
            priority: 0,
        }
    }

    pub fn to_sql(&self) -> String {
        format!(
            "\"{}\" {}{}",
            self.column,
            self.direction.sql(),
            self.nulls.sql()
        )
    }
}

/// Pagination state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub current_page: usize,
    pub page_size: usize,
    pub total_rows: usize,
    pub available_page_sizes: Vec<usize>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            current_page: 1,
            page_size: 1000,
            total_rows: 0,
            available_page_sizes: vec![100, 250, 500, 1000, 2500, 5000],
        }
    }
}

impl Pagination {
    pub fn total_pages(&self) -> usize {
        if self.total_rows == 0 {
            1
        } else {
            (self.total_rows + self.page_size - 1) / self.page_size
        }
    }

    pub fn offset(&self) -> usize {
        (self.current_page - 1) * self.page_size
    }

    pub fn first_row(&self) -> usize {
        if self.total_rows == 0 {
            0
        } else {
            self.offset() + 1
        }
    }

    pub fn last_row(&self) -> usize {
        std::cmp::min(self.offset() + self.page_size, self.total_rows)
    }

    pub fn can_go_previous(&self) -> bool {
        self.current_page > 1
    }

    pub fn can_go_next(&self) -> bool {
        self.current_page < self.total_pages()
    }

    pub fn go_first(&mut self) {
        self.current_page = 1;
    }

    pub fn go_previous(&mut self) {
        if self.can_go_previous() {
            self.current_page -= 1;
        }
    }

    pub fn go_next(&mut self) {
        if self.can_go_next() {
            self.current_page += 1;
        }
    }

    pub fn go_last(&mut self) {
        self.current_page = self.total_pages();
    }

    pub fn go_to_page(&mut self, page: usize) {
        self.current_page = page.clamp(1, self.total_pages());
    }

    pub fn set_page_size(&mut self, size: usize) {
        let first_row = self.first_row();
        self.page_size = size;
        // Try to keep the user roughly at the same position
        self.current_page = if first_row == 0 { 1 } else { (first_row - 1) / self.page_size + 1 };
    }
}

/// Key for table viewer state lookup
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TableViewerKey {
    pub connection_id: Uuid,
    pub schema: String,
    pub table: String,
}

impl TableViewerKey {
    pub fn new(connection_id: Uuid, schema: impl Into<String>, table: impl Into<String>) -> Self {
        Self {
            connection_id,
            schema: schema.into(),
            table: table.into(),
        }
    }
}

/// State for a single table viewer instance
#[derive(Debug, Clone)]
pub struct TableViewerInstance {
    pub key: TableViewerKey,
    pub filters: Vec<TableFilter>,
    pub sorts: Vec<TableSort>,
    pub pagination: Pagination,
    pub edit_mode: bool,
    pub show_filter_builder: bool,
    pub is_loading: bool,
    pub is_counting: bool,
    pub error: Option<String>,
    pub last_refresh: Option<DateTime<Utc>>,
}

impl TableViewerInstance {
    pub fn new(key: TableViewerKey) -> Self {
        Self {
            key,
            filters: Vec::new(),
            sorts: Vec::new(),
            pagination: Pagination::default(),
            edit_mode: false,
            show_filter_builder: false,
            is_loading: false,
            is_counting: false,
            error: None,
            last_refresh: None,
        }
    }

    /// Get active (enabled) filters
    pub fn active_filters(&self) -> impl Iterator<Item = &TableFilter> {
        self.filters.iter().filter(|f| f.enabled)
    }

    /// Build the WHERE clause for active filters
    pub fn build_where_clause(&self) -> (String, Vec<String>) {
        let active: Vec<_> = self.active_filters().collect();
        if active.is_empty() {
            return (String::new(), Vec::new());
        }

        let mut param_index = 0;
        let mut all_params = Vec::new();
        let mut clauses = Vec::new();

        for filter in active {
            let (sql, params) = filter.to_sql(&mut param_index);
            clauses.push(sql);
            all_params.extend(params);
        }

        (format!("WHERE {}", clauses.join(" AND ")), all_params)
    }

    /// Build the ORDER BY clause
    pub fn build_order_by_clause(&self) -> String {
        if self.sorts.is_empty() {
            return String::new();
        }

        let mut sorted: Vec<_> = self.sorts.iter().collect();
        sorted.sort_by_key(|s| s.priority);

        let clauses: Vec<_> = sorted.iter().map(|s| s.to_sql()).collect();
        format!("ORDER BY {}", clauses.join(", "))
    }

    /// Generate full SELECT SQL with current filters, sorts, and pagination
    pub fn build_data_sql(&self) -> (String, Vec<String>) {
        let table_ref = format!("\"{}\".\"{}\"", self.key.schema, self.key.table);
        let (where_clause, params) = self.build_where_clause();
        let order_by = self.build_order_by_clause();

        let sql = format!(
            "SELECT * FROM {} {} {} LIMIT {} OFFSET {}",
            table_ref,
            where_clause,
            order_by,
            self.pagination.page_size,
            self.pagination.offset()
        );

        (sql.trim().replace("  ", " "), params)
    }

    /// Generate COUNT SQL for pagination
    pub fn build_count_sql(&self) -> (String, Vec<String>) {
        let table_ref = format!("\"{}\".\"{}\"", self.key.schema, self.key.table);
        let (where_clause, params) = self.build_where_clause();

        let sql = format!(
            "SELECT COUNT(*) FROM {} {}",
            table_ref,
            where_clause
        );

        (sql.trim().to_string(), params)
    }

    /// Generate formatted SQL for "Open as SQL" feature
    pub fn build_formatted_sql(&self) -> String {
        let table_ref = format!("\"{}\".\"{}\"", self.key.schema, self.key.table);

        let mut parts = vec![format!("SELECT *\nFROM {}", table_ref)];

        let active: Vec<_> = self.active_filters().collect();
        if !active.is_empty() {
            let clauses: Vec<_> = active.iter().map(|f| f.to_display_sql()).collect();
            parts.push(format!("WHERE {}", clauses.join("\n  AND ")));
        }

        if !self.sorts.is_empty() {
            let mut sorted: Vec<_> = self.sorts.iter().collect();
            sorted.sort_by_key(|s| s.priority);
            let clauses: Vec<_> = sorted.iter().map(|s| s.to_sql()).collect();
            parts.push(format!("ORDER BY {}", clauses.join(", ")));
        }

        parts.push(format!("LIMIT {};", self.pagination.page_size));

        parts.join("\n")
    }
}
```

### 17.2 Table Viewer State

```rust
// src/state/table_viewer.rs

use gpui::Global;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Handle;
use uuid::Uuid;

use crate::models::table_viewer::{
    TableViewerKey, TableViewerInstance, TableFilter, TableSort,
    FilterOperator, SortDirection, Pagination,
};
use crate::models::schema::Column;
use crate::services::query::QueryService;
use crate::services::schema::SchemaService;

/// Global state for all table viewer instances
pub struct TableViewerState {
    query_service: Arc<QueryService>,
    schema_service: Arc<SchemaService>,
    instances: RwLock<HashMap<TableViewerKey, TableViewerInstance>>,
    runtime: Handle,
}

impl Global for TableViewerState {}

impl TableViewerState {
    pub fn new(
        query_service: Arc<QueryService>,
        schema_service: Arc<SchemaService>,
        runtime: Handle,
    ) -> Self {
        Self {
            query_service,
            schema_service,
            instances: RwLock::new(HashMap::new()),
            runtime,
        }
    }

    /// Initialize or get a table viewer instance
    pub fn init_viewer(&self, key: TableViewerKey) -> TableViewerInstance {
        let mut instances = self.instances.write();
        instances.entry(key.clone())
            .or_insert_with(|| TableViewerInstance::new(key.clone()))
            .clone()
    }

    /// Get viewer state for a table
    pub fn get_viewer(&self, key: &TableViewerKey) -> Option<TableViewerInstance> {
        self.instances.read().get(key).cloned()
    }

    /// Get mutable access to viewer and run update function
    fn update_viewer<F, R>(&self, key: &TableViewerKey, f: F) -> Option<R>
    where
        F: FnOnce(&mut TableViewerInstance) -> R,
    {
        let mut instances = self.instances.write();
        instances.get_mut(key).map(f)
    }

    /// Fetch data for the table viewer
    pub fn fetch_data(&self, key: &TableViewerKey) -> Result<(), String> {
        // Mark as loading
        self.update_viewer(key, |instance| {
            instance.is_loading = true;
            instance.error = None;
        });

        // Get SQL and params
        let (sql, _params) = {
            let instances = self.instances.read();
            let instance = instances.get(key).ok_or("Viewer not found")?;
            instance.build_data_sql()
        };

        // Execute query
        let query_service = self.query_service.clone();
        let key_clone = key.clone();

        let result = self.runtime.block_on(async {
            query_service.execute_query(
                key_clone.connection_id,
                &sql,
                None, // Use statement timeout from connection settings
            ).await
        });

        match result {
            Ok(query_result) => {
                self.update_viewer(key, |instance| {
                    instance.is_loading = false;
                    instance.last_refresh = Some(chrono::Utc::now());
                });

                // Data is handled by grid state - this service just coordinates
                Ok(())
            }
            Err(e) => {
                self.update_viewer(key, |instance| {
                    instance.is_loading = false;
                    instance.error = Some(e.to_string());
                });
                Err(e.to_string())
            }
        }
    }

    /// Fetch row count for pagination
    pub fn fetch_count(&self, key: &TableViewerKey) -> Result<usize, String> {
        self.update_viewer(key, |instance| {
            instance.is_counting = true;
        });

        let (sql, _params) = {
            let instances = self.instances.read();
            let instance = instances.get(key).ok_or("Viewer not found")?;
            instance.build_count_sql()
        };

        let query_service = self.query_service.clone();
        let key_clone = key.clone();

        let result = self.runtime.block_on(async {
            query_service.execute_query(
                key_clone.connection_id,
                &sql,
                Some(30000), // 30 second timeout for COUNT
            ).await
        });

        match result {
            Ok(query_result) => {
                // Extract count from first row, first column
                let count = query_result.rows
                    .first()
                    .and_then(|row| row.first())
                    .and_then(|val| val.as_str())
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0);

                self.update_viewer(key, |instance| {
                    instance.is_counting = false;
                    instance.pagination.total_rows = count;
                });

                Ok(count)
            }
            Err(e) => {
                self.update_viewer(key, |instance| {
                    instance.is_counting = false;
                });
                Err(e.to_string())
            }
        }
    }

    /// Add a filter
    pub fn add_filter(&self, key: &TableViewerKey, filter: TableFilter) {
        self.update_viewer(key, |instance| {
            instance.filters.push(filter);
            instance.pagination.current_page = 1; // Reset to first page
        });
    }

    /// Remove a filter by ID
    pub fn remove_filter(&self, key: &TableViewerKey, filter_id: Uuid) {
        self.update_viewer(key, |instance| {
            instance.filters.retain(|f| f.id != filter_id);
            instance.pagination.current_page = 1;
        });
    }

    /// Toggle filter enabled state
    pub fn toggle_filter(&self, key: &TableViewerKey, filter_id: Uuid) {
        self.update_viewer(key, |instance| {
            if let Some(filter) = instance.filters.iter_mut().find(|f| f.id == filter_id) {
                filter.enabled = !filter.enabled;
            }
            instance.pagination.current_page = 1;
        });
    }

    /// Update filter value
    pub fn update_filter_value(&self, key: &TableViewerKey, filter_id: Uuid, value: Option<String>, value2: Option<String>) {
        self.update_viewer(key, |instance| {
            if let Some(filter) = instance.filters.iter_mut().find(|f| f.id == filter_id) {
                filter.value = value;
                filter.value2 = value2;
            }
        });
    }

    /// Clear all filters
    pub fn clear_filters(&self, key: &TableViewerKey) {
        self.update_viewer(key, |instance| {
            instance.filters.clear();
            instance.pagination.current_page = 1;
        });
    }

    /// Toggle sort on a column
    pub fn toggle_sort(&self, key: &TableViewerKey, column: String) {
        self.update_viewer(key, |instance| {
            if let Some(pos) = instance.sorts.iter().position(|s| s.column == column) {
                let sort = &mut instance.sorts[pos];
                if sort.direction == SortDirection::Ascending {
                    sort.direction = SortDirection::Descending;
                } else {
                    // Remove sort on second toggle of DESC
                    instance.sorts.remove(pos);
                    // Re-index priorities
                    for (i, s) in instance.sorts.iter_mut().enumerate() {
                        s.priority = i;
                    }
                }
            } else {
                // Add new sort
                let priority = instance.sorts.len();
                instance.sorts.push(TableSort {
                    column,
                    direction: SortDirection::Ascending,
                    nulls: crate::models::table_viewer::NullsPosition::Default,
                    priority,
                });
            }
        });
    }

    /// Set sort for column (replacing any existing)
    pub fn set_sort(&self, key: &TableViewerKey, column: String, direction: SortDirection) {
        self.update_viewer(key, |instance| {
            instance.sorts.clear();
            instance.sorts.push(TableSort::new(column, direction));
        });
    }

    /// Clear all sorts
    pub fn clear_sorts(&self, key: &TableViewerKey) {
        self.update_viewer(key, |instance| {
            instance.sorts.clear();
        });
    }

    /// Navigate to page
    pub fn go_to_page(&self, key: &TableViewerKey, page: usize) {
        self.update_viewer(key, |instance| {
            instance.pagination.go_to_page(page);
        });
    }

    /// Navigate to first page
    pub fn go_first(&self, key: &TableViewerKey) {
        self.update_viewer(key, |instance| {
            instance.pagination.go_first();
        });
    }

    /// Navigate to previous page
    pub fn go_previous(&self, key: &TableViewerKey) {
        self.update_viewer(key, |instance| {
            instance.pagination.go_previous();
        });
    }

    /// Navigate to next page
    pub fn go_next(&self, key: &TableViewerKey) {
        self.update_viewer(key, |instance| {
            instance.pagination.go_next();
        });
    }

    /// Navigate to last page
    pub fn go_last(&self, key: &TableViewerKey) {
        self.update_viewer(key, |instance| {
            instance.pagination.go_last();
        });
    }

    /// Set page size
    pub fn set_page_size(&self, key: &TableViewerKey, size: usize) {
        self.update_viewer(key, |instance| {
            instance.pagination.set_page_size(size);
        });
    }

    /// Toggle edit mode
    pub fn toggle_edit_mode(&self, key: &TableViewerKey) {
        self.update_viewer(key, |instance| {
            instance.edit_mode = !instance.edit_mode;
        });
    }

    /// Toggle filter builder visibility
    pub fn toggle_filter_builder(&self, key: &TableViewerKey) {
        self.update_viewer(key, |instance| {
            instance.show_filter_builder = !instance.show_filter_builder;
        });
    }

    /// Get columns for the table
    pub fn get_columns(&self, key: &TableViewerKey) -> Vec<Column> {
        let schema_service = self.schema_service.clone();

        self.runtime.block_on(async {
            schema_service.get_columns(key.connection_id, &key.schema, &key.table)
                .await
                .unwrap_or_default()
        })
    }

    /// Get formatted SQL for "Open as SQL" feature
    pub fn get_formatted_sql(&self, key: &TableViewerKey) -> Option<String> {
        self.instances.read().get(key).map(|i| i.build_formatted_sql())
    }

    /// Cleanup viewer state when tab closes
    pub fn cleanup(&self, key: &TableViewerKey) {
        self.instances.write().remove(key);
    }
}
```

### 17.3 Table Viewer Component

```rust
// src/ui/components/table_viewer.rs

use gpui::*;
use uuid::Uuid;

use crate::models::table_viewer::{
    TableViewerKey, TableFilter, FilterOperator, SortDirection,
};
use crate::state::table_viewer::TableViewerState;
use crate::state::tabs::TabsState;
use crate::ui::components::results_grid::ResultsGrid;
use crate::ui::components::filter_builder::FilterBuilder;
use crate::theme::Theme;

/// Events emitted by TableViewer
pub enum TableViewerEvent {
    OpenAsSql(String),
    Error(String),
}

impl EventEmitter<TableViewerEvent> for TableViewer {}

/// Table viewer component
pub struct TableViewer {
    key: TableViewerKey,
    results_grid: Entity<ResultsGrid>,
    filter_builder: Option<Entity<FilterBuilder>>,
}

impl TableViewer {
    pub fn new(
        cx: &mut Context<Self>,
        connection_id: Uuid,
        schema: String,
        table: String,
    ) -> Self {
        let key = TableViewerKey::new(connection_id, schema.clone(), table.clone());

        // Initialize viewer state
        let viewer_state = cx.global::<TableViewerState>();
        viewer_state.init_viewer(key.clone());

        // Create results grid
        let results_grid = cx.new(|cx| {
            ResultsGrid::new(cx)
        });

        // Initial data fetch
        let key_clone = key.clone();
        cx.spawn(|this, mut cx| async move {
            // Fetch count first
            cx.update_global::<TableViewerState, _>(|state, _| {
                let _ = state.fetch_count(&key_clone);
            }).ok();

            // Then fetch data
            cx.update_global::<TableViewerState, _>(|state, _| {
                let _ = state.fetch_data(&key_clone);
            }).ok();
        }).detach();

        Self {
            key,
            results_grid,
            filter_builder: None,
        }
    }

    fn toggle_filter_builder(&mut self, cx: &mut Context<Self>) {
        let viewer_state = cx.global::<TableViewerState>();
        viewer_state.toggle_filter_builder(&self.key);

        let instance = viewer_state.get_viewer(&self.key);
        if instance.map(|i| i.show_filter_builder).unwrap_or(false) {
            // Create filter builder
            let columns = viewer_state.get_columns(&self.key);
            let key = self.key.clone();

            self.filter_builder = Some(cx.new(|cx| {
                FilterBuilder::new(cx, columns, move |filter, cx| {
                    cx.global::<TableViewerState>().add_filter(&key, filter);
                })
            }));
        } else {
            self.filter_builder = None;
        }

        cx.notify();
    }

    fn remove_filter(&mut self, filter_id: Uuid, cx: &mut Context<Self>) {
        let viewer_state = cx.global::<TableViewerState>();
        viewer_state.remove_filter(&self.key, filter_id);
        self.refresh_data(cx);
    }

    fn clear_filters(&mut self, cx: &mut Context<Self>) {
        let viewer_state = cx.global::<TableViewerState>();
        viewer_state.clear_filters(&self.key);
        self.refresh_data(cx);
    }

    fn toggle_sort(&mut self, column: String, cx: &mut Context<Self>) {
        let viewer_state = cx.global::<TableViewerState>();
        viewer_state.toggle_sort(&self.key, column);
        self.refresh_data(cx);
    }

    fn set_page(&mut self, page: usize, cx: &mut Context<Self>) {
        let viewer_state = cx.global::<TableViewerState>();
        viewer_state.go_to_page(&self.key, page);
        self.refresh_data(cx);
    }

    fn go_first(&mut self, cx: &mut Context<Self>) {
        let viewer_state = cx.global::<TableViewerState>();
        viewer_state.go_first(&self.key);
        self.refresh_data(cx);
    }

    fn go_previous(&mut self, cx: &mut Context<Self>) {
        let viewer_state = cx.global::<TableViewerState>();
        viewer_state.go_previous(&self.key);
        self.refresh_data(cx);
    }

    fn go_next(&mut self, cx: &mut Context<Self>) {
        let viewer_state = cx.global::<TableViewerState>();
        viewer_state.go_next(&self.key);
        self.refresh_data(cx);
    }

    fn go_last(&mut self, cx: &mut Context<Self>) {
        let viewer_state = cx.global::<TableViewerState>();
        viewer_state.go_last(&self.key);
        self.refresh_data(cx);
    }

    fn set_page_size(&mut self, size: usize, cx: &mut Context<Self>) {
        let viewer_state = cx.global::<TableViewerState>();
        viewer_state.set_page_size(&self.key, size);
        self.refresh_data(cx);
    }

    fn toggle_edit_mode(&mut self, cx: &mut Context<Self>) {
        let viewer_state = cx.global::<TableViewerState>();
        viewer_state.toggle_edit_mode(&self.key);
        cx.notify();
    }

    fn refresh_data(&mut self, cx: &mut Context<Self>) {
        let key = self.key.clone();

        cx.spawn(|this, mut cx| async move {
            // Fetch count
            cx.update_global::<TableViewerState, _>(|state, _| {
                let _ = state.fetch_count(&key);
            }).ok();

            // Fetch data
            let result = cx.update_global::<TableViewerState, _>(|state, _| {
                state.fetch_data(&key)
            });

            if let Ok(Some(Err(e))) = result {
                this.update(&mut cx, |this, cx| {
                    cx.emit(TableViewerEvent::Error(e));
                }).ok();
            }
        }).detach();
    }

    fn open_as_sql(&mut self, cx: &mut Context<Self>) {
        let viewer_state = cx.global::<TableViewerState>();
        if let Some(sql) = viewer_state.get_formatted_sql(&self.key) {
            cx.emit(TableViewerEvent::OpenAsSql(sql));
        }
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let viewer_state = cx.global::<TableViewerState>();
        let instance = viewer_state.get_viewer(&self.key);

        let schema = self.key.schema.clone();
        let table = self.key.table.clone();
        let filter_count = instance.as_ref().map(|i| i.active_filters().count()).unwrap_or(0);
        let show_filter_builder = instance.as_ref().map(|i| i.show_filter_builder).unwrap_or(false);
        let edit_mode = instance.as_ref().map(|i| i.edit_mode).unwrap_or(false);
        let row_estimate = instance.as_ref().map(|i| i.pagination.total_rows).unwrap_or(0);

        div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .child(
                // Table info
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        Icon::new(IconName::Database)
                            .size_4()
                            .color(theme.text_muted)
                    )
                    .child(
                        span()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child(schema)
                    )
                    .child(
                        span()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child(".")
                    )
                    .child(
                        span()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text)
                            .child(table)
                    )
                    .child(
                        div()
                            .ml_2()
                            .px_2()
                            .py_px()
                            .rounded_full()
                            .bg(theme.surface_secondary)
                            .child(
                                span()
                                    .text_xs()
                                    .text_color(theme.text_muted)
                                    .child(format!("~{} rows", format_number(row_estimate)))
                            )
                    )
            )
            .child(
                // Actions
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        self.render_button(
                            "Filter",
                            Some(IconName::Filter),
                            show_filter_builder || filter_count > 0,
                            filter_count,
                            cx.listener(|this, _, cx| this.toggle_filter_builder(cx)),
                            cx,
                        )
                    )
                    .child(
                        self.render_button(
                            "Open as SQL",
                            None,
                            false,
                            0,
                            cx.listener(|this, _, cx| this.open_as_sql(cx)),
                            cx,
                        )
                    )
                    .child(
                        self.render_button(
                            "Edit Mode",
                            Some(IconName::Pencil),
                            edit_mode,
                            0,
                            cx.listener(|this, _, cx| this.toggle_edit_mode(cx)),
                            cx,
                        )
                    )
                    .child(
                        self.render_button(
                            "Refresh",
                            Some(IconName::RefreshCw),
                            false,
                            0,
                            cx.listener(|this, _, cx| this.refresh_data(cx)),
                            cx,
                        )
                    )
            )
    }

    fn render_button(
        &self,
        label: &str,
        icon: Option<IconName>,
        active: bool,
        badge: usize,
        on_click: impl Fn(&ClickEvent, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let bg = if active { theme.primary } else { theme.transparent };
        let text = if active { theme.on_primary } else { theme.text };
        let border = if active { theme.primary } else { theme.border };

        div()
            .id(SharedString::from(label.to_string()))
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(border)
            .bg(bg)
            .cursor_pointer()
            .hover(|s| s.bg(if active { theme.primary_hover } else { theme.hover }))
            .on_click(on_click)
            .when_some(icon, |el, icon| {
                el.child(Icon::new(icon).size_4().color(text))
            })
            .child(
                span()
                    .text_sm()
                    .text_color(text)
                    .child(label.to_string())
            )
            .when(badge > 0, |el| {
                el.child(
                    div()
                        .px_1()
                        .rounded_full()
                        .bg(if active { theme.on_primary } else { theme.primary })
                        .child(
                            span()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(if active { theme.primary } else { theme.on_primary })
                                .child(badge.to_string())
                        )
                )
            })
    }

    fn render_filter_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let viewer_state = cx.global::<TableViewerState>();
        let instance = viewer_state.get_viewer(&self.key);

        let filters: Vec<_> = instance.as_ref()
            .map(|i| i.filters.clone())
            .unwrap_or_default();

        let show_builder = instance.as_ref().map(|i| i.show_filter_builder).unwrap_or(false);

        if filters.is_empty() && !show_builder {
            return div().into_any_element();
        }

        div()
            .w_full()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.surface_secondary)
            .child(
                // Active filters
                div()
                    .when(!filters.is_empty(), |el| {
                        el.flex()
                            .flex_wrap()
                            .gap_2()
                            .items_center()
                            .mb_2()
                            .children(filters.iter().map(|filter| {
                                self.render_filter_chip(filter, cx)
                            }))
                            .child(
                                div()
                                    .cursor_pointer()
                                    .text_sm()
                                    .text_color(theme.text_muted)
                                    .hover(|s| s.text_color(theme.primary))
                                    .on_click(cx.listener(|this, _, cx| this.clear_filters(cx)))
                                    .child("Clear all")
                            )
                    })
            )
            .when_some(self.filter_builder.as_ref(), |el, filter_builder| {
                el.child(filter_builder.clone())
            })
            .into_any_element()
    }

    fn render_filter_chip(&self, filter: &TableFilter, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let filter_id = filter.id;
        let display_sql = filter.to_display_sql();
        let enabled = filter.enabled;

        div()
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(if enabled { theme.surface } else { theme.surface_secondary })
            .when(!enabled, |el| el.opacity(0.5))
            .child(
                span()
                    .text_sm()
                    .font_family("monospace")
                    .text_color(theme.text)
                    .child(display_sql)
            )
            .child(
                div()
                    .id(SharedString::from(format!("remove-filter-{}", filter_id)))
                    .ml_1()
                    .p_px()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.hover))
                    .on_click(cx.listener(move |this, _, cx| {
                        this.remove_filter(filter_id, cx);
                    }))
                    .child(
                        Icon::new(IconName::X)
                            .size_3()
                            .color(theme.text_muted)
                    )
            )
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let viewer_state = cx.global::<TableViewerState>();
        let instance = viewer_state.get_viewer(&self.key);

        let pagination = instance.as_ref()
            .map(|i| i.pagination.clone())
            .unwrap_or_default();

        let sorts: Vec<_> = instance.as_ref()
            .map(|i| i.sorts.clone())
            .unwrap_or_default();

        div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px_3()
            .py_2()
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .child(
                // Pagination controls
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(self.render_page_button("First", !pagination.can_go_previous(),
                        cx.listener(|this, _, cx| this.go_first(cx)), cx))
                    .child(self.render_page_button_icon(IconName::ChevronLeft, !pagination.can_go_previous(),
                        cx.listener(|this, _, cx| this.go_previous(cx)), cx))
                    .child(
                        span()
                            .px_2()
                            .text_sm()
                            .text_color(theme.text_muted)
                            .child(format!(
                                "Page {} of {}",
                                pagination.current_page,
                                pagination.total_pages()
                            ))
                    )
                    .child(self.render_page_button_icon(IconName::ChevronRight, !pagination.can_go_next(),
                        cx.listener(|this, _, cx| this.go_next(cx)), cx))
                    .child(self.render_page_button("Last", !pagination.can_go_next(),
                        cx.listener(|this, _, cx| this.go_last(cx)), cx))
                    .child(
                        // Page size selector
                        self.render_page_size_selector(&pagination, cx)
                    )
            )
            .child(
                // Row info
                span()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child(format!(
                        "Showing {} - {} of {} rows",
                        format_number(pagination.first_row()),
                        format_number(pagination.last_row()),
                        format_number(pagination.total_rows)
                    ))
            )
            .child(
                // Sort info
                div()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .when(!sorts.is_empty(), |el| {
                        let sort_text = sorts.iter()
                            .map(|s| format!("{} {}", s.column, s.direction.symbol()))
                            .collect::<Vec<_>>()
                            .join(", ");
                        el.child(format!("Sort: {}", sort_text))
                    })
            )
    }

    fn render_page_button(
        &self,
        label: &str,
        disabled: bool,
        on_click: impl Fn(&ClickEvent, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id(SharedString::from(format!("page-btn-{}", label)))
            .flex()
            .items_center()
            .justify_center()
            .min_w_7()
            .h_7()
            .px_2()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .cursor(if disabled { CursorStyle::Default } else { CursorStyle::PointingHand })
            .when(disabled, |el| el.opacity(0.5))
            .when(!disabled, |el| {
                el.hover(|s| s.bg(theme.hover))
                    .on_click(on_click)
            })
            .child(
                span()
                    .text_sm()
                    .text_color(theme.text)
                    .child(label.to_string())
            )
    }

    fn render_page_button_icon(
        &self,
        icon: IconName,
        disabled: bool,
        on_click: impl Fn(&ClickEvent, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id(SharedString::from(format!("page-btn-{:?}", icon)))
            .flex()
            .items_center()
            .justify_center()
            .w_7()
            .h_7()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .cursor(if disabled { CursorStyle::Default } else { CursorStyle::PointingHand })
            .when(disabled, |el| el.opacity(0.5))
            .when(!disabled, |el| {
                el.hover(|s| s.bg(theme.hover))
                    .on_click(on_click)
            })
            .child(Icon::new(icon).size_4().color(theme.text))
    }

    fn render_page_size_selector(&self, pagination: &crate::models::table_viewer::Pagination, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let current_size = pagination.page_size;
        let sizes = pagination.available_page_sizes.clone();

        div()
            .ml_4()
            .flex()
            .items_center()
            .gap_1()
            .child(
                span()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child("Rows:")
            )
            .child(
                div()
                    .relative()
                    .child(
                        // Simple dropdown implementation
                        select()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.surface)
                            .text_sm()
                            .text_color(theme.text)
                            .on_change(cx.listener(move |this, event: &ChangeEvent, cx| {
                                if let Some(size) = event.value.parse::<usize>().ok() {
                                    this.set_page_size(size, cx);
                                }
                            }))
                            .children(sizes.iter().map(|&size| {
                                option()
                                    .value(size.to_string())
                                    .selected(size == current_size)
                                    .child(format_number(size))
                            }))
                    )
            )
    }

    fn render_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let viewer_state = cx.global::<TableViewerState>();
        let instance = viewer_state.get_viewer(&self.key);

        let is_loading = instance.as_ref().map(|i| i.is_loading).unwrap_or(false);
        let error = instance.as_ref().and_then(|i| i.error.clone());

        div()
            .flex_1()
            .overflow_hidden()
            .child(
                if is_loading {
                    div()
                        .w_full()
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            span()
                                .text_color(theme.text_muted)
                                .child("Loading data...")
                        )
                        .into_any_element()
                } else if let Some(err) = error {
                    div()
                        .w_full()
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            span()
                                .text_color(theme.error)
                                .child(err)
                        )
                        .into_any_element()
                } else {
                    self.results_grid.clone().into_any_element()
                }
            )
    }
}

impl Render for TableViewer {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .child(self.render_header(cx))
            .child(self.render_filter_section(cx))
            .child(self.render_content(cx))
            .child(self.render_footer(cx))
    }
}

/// Format a number with thousands separators
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}
```

### 17.4 Filter Builder Component

```rust
// src/ui/components/filter_builder.rs

use gpui::*;
use uuid::Uuid;

use crate::models::table_viewer::{TableFilter, FilterOperator};
use crate::models::schema::Column;
use crate::theme::Theme;

/// Events emitted by FilterBuilder
pub enum FilterBuilderEvent {
    FilterAdded(TableFilter),
    Close,
}

impl EventEmitter<FilterBuilderEvent> for FilterBuilder {}

/// Filter builder component for adding filters
pub struct FilterBuilder {
    columns: Vec<Column>,
    selected_column_index: usize,
    selected_operator_index: usize,
    value: String,
    value2: String,  // For BETWEEN operator
    available_operators: Vec<FilterOperator>,
    on_add: Box<dyn Fn(TableFilter, &mut WindowContext) + 'static>,
}

impl FilterBuilder {
    pub fn new(
        cx: &mut Context<Self>,
        columns: Vec<Column>,
        on_add: impl Fn(TableFilter, &mut WindowContext) + 'static,
    ) -> Self {
        let available_operators = if let Some(col) = columns.first() {
            FilterOperator::for_type(&col.data_type)
        } else {
            vec![FilterOperator::Equal, FilterOperator::IsNull]
        };

        Self {
            columns,
            selected_column_index: 0,
            selected_operator_index: 0,
            value: String::new(),
            value2: String::new(),
            available_operators,
            on_add: Box::new(on_add),
        }
    }

    fn selected_column(&self) -> Option<&Column> {
        self.columns.get(self.selected_column_index)
    }

    fn selected_operator(&self) -> Option<&FilterOperator> {
        self.available_operators.get(self.selected_operator_index)
    }

    fn select_column(&mut self, index: usize, cx: &mut Context<Self>) {
        self.selected_column_index = index;

        // Update available operators for new column type
        if let Some(col) = self.columns.get(index) {
            self.available_operators = FilterOperator::for_type(&col.data_type);
            self.selected_operator_index = 0;
        }

        cx.notify();
    }

    fn select_operator(&mut self, index: usize, cx: &mut Context<Self>) {
        self.selected_operator_index = index;
        cx.notify();
    }

    fn set_value(&mut self, value: String, cx: &mut Context<Self>) {
        self.value = value;
        cx.notify();
    }

    fn set_value2(&mut self, value: String, cx: &mut Context<Self>) {
        self.value2 = value;
        cx.notify();
    }

    fn add_filter(&mut self, cx: &mut Context<Self>) {
        let column = match self.selected_column() {
            Some(col) => col.clone(),
            None => return,
        };

        let operator = match self.selected_operator() {
            Some(op) => op.clone(),
            None => return,
        };

        // Validate value if required
        if operator.requires_value() && self.value.trim().is_empty() {
            return;
        }

        if operator.is_range_operator() && self.value2.trim().is_empty() {
            return;
        }

        let filter = TableFilter {
            id: Uuid::new_v4(),
            column: column.name.clone(),
            column_type: column.data_type.clone(),
            operator,
            value: if self.value.trim().is_empty() { None } else { Some(self.value.trim().to_string()) },
            value2: if self.value2.trim().is_empty() { None } else { Some(self.value2.trim().to_string()) },
            enabled: true,
        };

        // Call the callback
        cx.window_context().update(|cx| {
            (self.on_add)(filter.clone(), cx);
        });

        // Reset values
        self.value.clear();
        self.value2.clear();

        cx.emit(FilterBuilderEvent::FilterAdded(filter));
        cx.notify();
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        cx.emit(FilterBuilderEvent::Close);
    }

    fn handle_keydown(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "enter" => self.add_filter(cx),
            "escape" => self.close(cx),
            _ => {}
        }
    }
}

impl Render for FilterBuilder {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let operator = self.selected_operator().cloned();
        let needs_value = operator.as_ref().map(|o| o.requires_value()).unwrap_or(false);
        let is_range = operator.as_ref().map(|o| o.is_range_operator()).unwrap_or(false);

        div()
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .flex_wrap()
            .on_key_down(cx.listener(Self::handle_keydown))
            .child(
                // Column selector
                self.render_column_selector(cx)
            )
            .child(
                // Operator selector
                self.render_operator_selector(cx)
            )
            .when(needs_value, |el| {
                el.child(
                    // Value input
                    self.render_value_input("Value", &self.value.clone(), false, cx)
                )
            })
            .when(is_range, |el| {
                el.child(
                    span()
                        .text_sm()
                        .text_color(theme.text_muted)
                        .child("and")
                )
                .child(
                    self.render_value_input("Max value", &self.value2.clone(), true, cx)
                )
            })
            .child(
                // Add button
                div()
                    .id("add-filter-btn")
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .bg(theme.primary)
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.primary_hover))
                    .on_click(cx.listener(|this, _, cx| this.add_filter(cx)))
                    .child(
                        Icon::new(IconName::Plus)
                            .size_4()
                            .color(theme.on_primary)
                    )
                    .child(
                        span()
                            .text_sm()
                            .text_color(theme.on_primary)
                            .child("Add")
                    )
            )
    }
}

impl FilterBuilder {
    fn render_column_selector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        select()
            .min_w(px(150.0))
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .text_sm()
            .text_color(theme.text)
            .on_change(cx.listener(|this, event: &ChangeEvent, cx| {
                if let Some(index) = event.value.parse::<usize>().ok() {
                    this.select_column(index, cx);
                }
            }))
            .children(self.columns.iter().enumerate().map(|(i, col)| {
                option()
                    .value(i.to_string())
                    .selected(i == self.selected_column_index)
                    .child(format!("{} ({})", col.name, col.data_type))
            }))
    }

    fn render_operator_selector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        select()
            .min_w(px(100.0))
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .text_sm()
            .text_color(theme.text)
            .on_change(cx.listener(|this, event: &ChangeEvent, cx| {
                if let Some(index) = event.value.parse::<usize>().ok() {
                    this.select_operator(index, cx);
                }
            }))
            .children(self.available_operators.iter().enumerate().map(|(i, op)| {
                option()
                    .value(i.to_string())
                    .selected(i == self.selected_operator_index)
                    .child(op.label())
            }))
    }

    fn render_value_input(&self, placeholder: &str, value: &str, is_value2: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let value_owned = value.to_string();

        input()
            .min_w(px(150.0))
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .text_sm()
            .text_color(theme.text)
            .placeholder(placeholder)
            .value(value_owned)
            .on_input(cx.listener(move |this, event: &InputEvent, cx| {
                if is_value2 {
                    this.set_value2(event.value.clone(), cx);
                } else {
                    this.set_value(event.value.clone(), cx);
                }
            }))
            .on_key_down(cx.listener(Self::handle_keydown))
    }
}
```

### 17.5 Column Header Sort Integration

```rust
// src/ui/components/sortable_column_header.rs

use gpui::*;

use crate::models::table_viewer::SortDirection;
use crate::theme::Theme;

/// Props for sortable column header
pub struct SortableColumnHeaderProps {
    pub name: String,
    pub sort_direction: Option<SortDirection>,
    pub sort_priority: Option<usize>,
}

/// A sortable column header component
pub struct SortableColumnHeader {
    props: SortableColumnHeaderProps,
}

impl SortableColumnHeader {
    pub fn new(props: SortableColumnHeaderProps) -> Self {
        Self { props }
    }
}

impl Render for SortableColumnHeader {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let name = self.props.name.clone();

        div()
            .w_full()
            .h_full()
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .cursor_pointer()
            .hover(|s| s.bg(theme.hover))
            .child(
                span()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text)
                    .child(name)
            )
            .when_some(self.props.sort_direction, |el, direction| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_px()
                        .child(
                            span()
                                .text_xs()
                                .text_color(theme.primary)
                                .child(direction.symbol())
                        )
                        .when_some(self.props.sort_priority, |el, priority| {
                            el.when(priority > 0, |el| {
                                el.child(
                                    span()
                                        .text_xs()
                                        .text_color(theme.text_muted)
                                        .child(format!("{}", priority + 1))
                                )
                            })
                        })
                )
            })
    }
}

/// Utility to create column header with sort info
pub fn sortable_header(
    name: impl Into<String>,
    sort_direction: Option<SortDirection>,
    sort_priority: Option<usize>,
) -> SortableColumnHeader {
    SortableColumnHeader::new(SortableColumnHeaderProps {
        name: name.into(),
        sort_direction,
        sort_priority,
    })
}
```

### 17.6 Quick Filter Bar (Inline Search)

```rust
// src/ui/components/quick_filter.rs

use gpui::*;
use std::time::{Duration, Instant};

use crate::theme::Theme;

/// Events emitted by QuickFilter
pub enum QuickFilterEvent {
    FilterChanged(String),
    Clear,
}

impl EventEmitter<QuickFilterEvent> for QuickFilter {}

/// Quick filter input for instant row filtering
pub struct QuickFilter {
    value: String,
    placeholder: String,
    debounce_ms: u64,
    last_change: Option<Instant>,
    pending_value: Option<String>,
}

impl QuickFilter {
    pub fn new(placeholder: impl Into<String>) -> Self {
        Self {
            value: String::new(),
            placeholder: placeholder.into(),
            debounce_ms: 300,
            last_change: None,
            pending_value: None,
        }
    }

    pub fn with_debounce(mut self, ms: u64) -> Self {
        self.debounce_ms = ms;
        self
    }

    fn set_value(&mut self, value: String, cx: &mut Context<Self>) {
        self.value = value.clone();
        self.pending_value = Some(value);
        self.last_change = Some(Instant::now());

        // Schedule debounced emit
        let debounce_ms = self.debounce_ms;
        cx.spawn(|this, mut cx| async move {
            smol::Timer::after(Duration::from_millis(debounce_ms)).await;

            this.update(&mut cx, |this, cx| {
                if let Some(pending) = this.pending_value.take() {
                    if this.last_change
                        .map(|t| t.elapsed() >= Duration::from_millis(this.debounce_ms))
                        .unwrap_or(true)
                    {
                        cx.emit(QuickFilterEvent::FilterChanged(pending));
                    }
                }
            }).ok();
        }).detach();

        cx.notify();
    }

    fn clear(&mut self, cx: &mut Context<Self>) {
        self.value.clear();
        self.pending_value = None;
        cx.emit(QuickFilterEvent::Clear);
        cx.notify();
    }

    fn handle_keydown(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if event.keystroke.key.as_str() == "escape" {
            self.clear(cx);
        }
    }
}

impl Render for QuickFilter {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let has_value = !self.value.is_empty();

        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .child(
                Icon::new(IconName::Search)
                    .size_4()
                    .color(theme.text_muted)
            )
            .child(
                input()
                    .flex_1()
                    .border_none()
                    .bg(theme.transparent)
                    .text_sm()
                    .text_color(theme.text)
                    .placeholder(&self.placeholder)
                    .value(self.value.clone())
                    .on_input(cx.listener(|this, event: &InputEvent, cx| {
                        this.set_value(event.value.clone(), cx);
                    }))
                    .on_key_down(cx.listener(Self::handle_keydown))
            )
            .when(has_value, |el| {
                el.child(
                    div()
                        .id("clear-filter")
                        .p_1()
                        .rounded_sm()
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.hover))
                        .on_click(cx.listener(|this, _, cx| this.clear(cx)))
                        .child(
                            Icon::new(IconName::X)
                                .size_3()
                                .color(theme.text_muted)
                        )
                )
            })
    }
}
```

### 17.7 Keyboard Shortcuts

```rust
// Keyboard shortcuts for table viewer

use gpui::*;

/// Register table viewer keyboard shortcuts
pub fn register_table_viewer_shortcuts(cx: &mut AppContext) {
    // Refresh data
    cx.bind_keys([
        KeyBinding::new("cmd-r", RefreshData, Some("TableViewer")),
        KeyBinding::new("f5", RefreshData, Some("TableViewer")),
    ]);

    // Toggle filter builder
    cx.bind_keys([
        KeyBinding::new("cmd-f", ToggleFilterBuilder, Some("TableViewer")),
        KeyBinding::new("ctrl-f", ToggleFilterBuilder, Some("TableViewer")),
    ]);

    // Clear all filters
    cx.bind_keys([
        KeyBinding::new("cmd-shift-f", ClearFilters, Some("TableViewer")),
    ]);

    // Toggle edit mode
    cx.bind_keys([
        KeyBinding::new("cmd-e", ToggleEditMode, Some("TableViewer")),
    ]);

    // Open as SQL
    cx.bind_keys([
        KeyBinding::new("cmd-shift-s", OpenAsSql, Some("TableViewer")),
    ]);

    // Pagination
    cx.bind_keys([
        KeyBinding::new("cmd-left", GoFirstPage, Some("TableViewer")),
        KeyBinding::new("cmd-right", GoLastPage, Some("TableViewer")),
        KeyBinding::new("pageup", GoPreviousPage, Some("TableViewer")),
        KeyBinding::new("pagedown", GoNextPage, Some("TableViewer")),
    ]);

    // Quick filter focus
    cx.bind_keys([
        KeyBinding::new("/", FocusQuickFilter, Some("TableViewer")),
    ]);
}

// Action definitions
actions!(
    table_viewer,
    [
        RefreshData,
        ToggleFilterBuilder,
        ClearFilters,
        ToggleEditMode,
        OpenAsSql,
        GoFirstPage,
        GoLastPage,
        GoPreviousPage,
        GoNextPage,
        FocusQuickFilter,
    ]
);
```

## Acceptance Criteria

1. **Data Display**
   - Show table data in grid with all column types
   - Display row count and table info in header
   - Support virtual scrolling for large results
   - Show loading state during data fetch

2. **Filtering**
   - Visual filter builder with column selection
   - Type-appropriate operators (text: LIKE, ILIKE; numeric: <, >; JSON: @>, ?; etc.)
   - Multiple active filters with AND logic
   - Filter chips showing active filters with disable/remove
   - Clear individual or all filters
   - Parameterized queries for all filter values (SQL injection safe)

3. **Sorting**
   - Click column header to sort (toggle asc/desc/none)
   - Multi-column sorting with priority indicator
   - Sort indicator (↑/↓) in column header
   - Sort info in footer

4. **Pagination**
   - Navigate pages with first/prev/next/last buttons
   - Show current page and total pages
   - Display row range (1-1,000 of 50,000)
   - Configurable page size (100, 250, 500, 1000, 2500, 5000)
   - Efficient COUNT queries for total rows

5. **Edit Mode Toggle**
   - Toggle button to enable editing
   - Visual indicator when edit mode active
   - Integrates with Feature 18 (Inline Editing)

6. **SQL Export**
   - "Open as SQL" generates equivalent query
   - Opens in new query tab
   - Includes filters and sorting
   - Properly formatted multi-line SQL

7. **Keyboard Shortcuts**
   - Cmd/Ctrl+R or F5: Refresh
   - Cmd/Ctrl+F: Toggle filter builder
   - Cmd/Ctrl+E: Toggle edit mode
   - PageUp/PageDown: Previous/next page
   - /: Focus quick filter

## Testing Instructions

### Using Tauri MCP (Integration Testing)

```rust
// Test table viewer initialization
#[test]
fn test_table_viewer_init() {
    let key = TableViewerKey::new(conn_id, "public", "users");
    let instance = viewer_state.init_viewer(key.clone());

    assert_eq!(instance.pagination.current_page, 1);
    assert_eq!(instance.pagination.page_size, 1000);
    assert!(instance.filters.is_empty());
}

// Test filter SQL generation
#[test]
fn test_filter_to_sql() {
    let filter = TableFilter {
        id: Uuid::new_v4(),
        column: "status".to_string(),
        column_type: "text".to_string(),
        operator: FilterOperator::Equal,
        value: Some("active".to_string()),
        value2: None,
        enabled: true,
    };

    let mut param_idx = 0;
    let (sql, params) = filter.to_sql(&mut param_idx);

    assert_eq!(sql, "\"status\" = $1");
    assert_eq!(params, vec!["active"]);
}

// Test pagination
#[test]
fn test_pagination() {
    let mut pagination = Pagination::default();
    pagination.total_rows = 5500;
    pagination.page_size = 1000;

    assert_eq!(pagination.total_pages(), 6);
    assert_eq!(pagination.first_row(), 1);
    assert_eq!(pagination.last_row(), 1000);

    pagination.go_next();
    assert_eq!(pagination.current_page, 2);
    assert_eq!(pagination.first_row(), 1001);

    pagination.go_last();
    assert_eq!(pagination.current_page, 6);
    assert_eq!(pagination.last_row(), 5500);
}

// Test multi-sort
#[test]
fn test_multi_sort() {
    let mut instance = TableViewerInstance::new(key);

    instance.sorts.push(TableSort::new("created_at".to_string(), SortDirection::Descending));
    instance.sorts.push(TableSort {
        column: "name".to_string(),
        direction: SortDirection::Ascending,
        nulls: NullsPosition::Last,
        priority: 1,
    });

    let order_by = instance.build_order_by_clause();
    assert_eq!(order_by, "ORDER BY \"created_at\" DESC, \"name\" ASC NULLS LAST");
}
```

### Using Playwright MCP (UI Testing)

```typescript
// Test filter builder UI
await mcp.browser_navigate({ url: 'http://localhost:1420' });

// Open table viewer
await mcp.browser_click({ element: 'Users table in schema browser', ref: 'users-table' });

// Click filter button
await mcp.browser_click({ element: 'Filter button', ref: 'filter-btn' });

// Select column
await mcp.browser_select_option({
    element: 'Column selector',
    ref: 'column-select',
    values: ['status']
});

// Select operator
await mcp.browser_select_option({
    element: 'Operator selector',
    ref: 'operator-select',
    values: ['1']  // Equal
});

// Enter value
await mcp.browser_type({
    element: 'Filter value input',
    ref: 'value-input',
    text: 'active'
});

// Add filter
await mcp.browser_click({ element: 'Add filter button', ref: 'add-filter-btn' });

// Verify filter chip appears
const snapshot = await mcp.browser_snapshot();
assert(snapshot.includes('status = \'active\''));

// Test pagination
await mcp.browser_click({ element: 'Next page button', ref: 'next-page' });
await mcp.browser_wait_for({ text: 'Page 2' });
```

## Performance Considerations

1. **Efficient COUNT Queries**
   - Use separate COUNT query with same WHERE clause
   - Consider caching count for unchanged filters
   - Add timeout to prevent runaway counts on huge tables

2. **Parameterized Queries**
   - All filter values passed as parameters
   - Prevents SQL injection
   - Enables query plan caching

3. **Lazy Loading**
   - Only fetch current page data
   - Don't prefetch adjacent pages
   - Stream large results if needed

4. **Debounced Quick Filter**
   - 300ms debounce on quick filter input
   - Client-side filtering for current page
   - Server-side for full table search

## Dependencies

- Feature 14: Results Grid (data display component)
- Feature 11: Query Execution (executing SELECT queries)
- Feature 10: Schema Introspection (column metadata for operators)
