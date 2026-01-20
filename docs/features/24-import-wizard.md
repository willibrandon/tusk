# Feature 24: Import Wizard

## Overview

The Import Wizard provides a step-by-step interface for importing data from CSV and JSON files into PostgreSQL tables, with column mapping, type conversion, data transformation, and conflict handling options. Built entirely in Rust with GPUI for the UI layer.

## Goals

- Support CSV and JSON (array and newline-delimited) formats
- Auto-detect file format, delimiter, and encoding
- Preview data before import
- Map source columns to target table columns
- Apply transformations (trim, case conversion, date parsing)
- Handle conflicts (error, skip, upsert)
- Use COPY for optimal performance
- Provide progress and error reporting

## Dependencies

- Feature 07: Connection Pool Management
- Feature 10: Schema Cache (for table/column metadata)

## Technical Specification

### 24.1 Import Data Models

```rust
// src/import/types.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for an import job
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImportId(pub String);

impl ImportId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

/// Status of an import job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportStatus {
    Configuring,
    Validating,
    Importing,
    Completed,
    Failed,
    Cancelled,
}

/// Complete import job configuration and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportJob {
    pub id: ImportId,
    pub status: ImportStatus,
    pub source: ImportSource,
    pub target: Option<ImportTarget>,
    pub mappings: Vec<ColumnMapping>,
    pub options: ImportOptions,
    pub progress: Option<ImportProgress>,
    pub result: Option<ImportResult>,
}

/// Source file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSource {
    pub file_path: String,
    pub file_type: FileType,
    pub file_size: u64,
    pub encoding: String,
    pub csv_options: Option<CsvOptions>,
    pub preview: PreviewData,
}

/// Supported file types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Csv,
    Json,
    JsonLines,
}

impl FileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileType::Csv => "CSV",
            FileType::Json => "JSON",
            FileType::JsonLines => "JSONL",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "csv" | "tsv" => Some(FileType::Csv),
            "json" => Some(FileType::Json),
            "jsonl" | "ndjson" => Some(FileType::JsonLines),
            _ => None,
        }
    }
}

/// CSV parsing options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvOptions {
    pub delimiter: char,
    pub quote_char: char,
    pub escape_char: char,
    pub has_header: bool,
    pub null_string: String,
    pub skip_rows: usize,
}

impl Default for CsvOptions {
    fn default() -> Self {
        Self {
            delimiter: ',',
            quote_char: '"',
            escape_char: '\\',
            has_header: true,
            null_string: "\\N".to_string(),
            skip_rows: 0,
        }
    }
}

/// Preview of source data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewData {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_rows: i64,
    pub detected_types: Vec<ColumnTypeHint>,
}

/// Type hint for a source column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnTypeHint {
    pub column: String,
    pub suggested_type: String,
    pub sample_values: Vec<String>,
    pub null_count: i32,
}

/// Import target table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportTarget {
    pub target_type: TargetType,
    pub schema: String,
    pub table: String,
    pub columns: Vec<TableColumn>,
}

/// Whether importing to existing or new table
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetType {
    Existing,
    New,
}

/// Table column information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub has_default: bool,
}

/// Mapping from source to target column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMapping {
    pub source_column: String,
    pub target_column: Option<String>,
    pub transform: Option<ColumnTransform>,
}

/// Column transformation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnTransform {
    pub transform_type: TransformType,
    pub options: HashMap<String, String>,
}

/// Available transformation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransformType {
    None,
    Trim,
    Uppercase,
    Lowercase,
    ParseDate,
    ParseBoolean,
    ParseNumber,
    CustomSql,
}

impl TransformType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransformType::None => "None",
            TransformType::Trim => "Trim",
            TransformType::Uppercase => "Uppercase",
            TransformType::Lowercase => "Lowercase",
            TransformType::ParseDate => "Parse Date",
            TransformType::ParseBoolean => "Parse Boolean",
            TransformType::ParseNumber => "Parse Number",
            TransformType::CustomSql => "Custom SQL",
        }
    }

    pub fn all() -> &'static [TransformType] {
        &[
            TransformType::None,
            TransformType::Trim,
            TransformType::Uppercase,
            TransformType::Lowercase,
            TransformType::ParseDate,
            TransformType::ParseBoolean,
            TransformType::ParseNumber,
            TransformType::CustomSql,
        ]
    }
}

/// Import options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOptions {
    pub conflict_handling: ConflictHandling,
    pub conflict_columns: Vec<String>,
    pub update_columns: Vec<String>,
    pub batch_size: usize,
    pub use_transaction: bool,
    pub use_copy: bool,
    pub truncate_first: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            conflict_handling: ConflictHandling::Error,
            conflict_columns: Vec::new(),
            update_columns: Vec::new(),
            batch_size: 1000,
            use_transaction: true,
            use_copy: true,
            truncate_first: false,
        }
    }
}

/// Conflict handling strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictHandling {
    Error,
    Skip,
    Update,
}

impl ConflictHandling {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConflictHandling::Error => "Error on Conflict",
            ConflictHandling::Skip => "Skip Duplicates",
            ConflictHandling::Update => "Update Existing",
        }
    }

    pub fn all() -> &'static [ConflictHandling] {
        &[ConflictHandling::Error, ConflictHandling::Skip, ConflictHandling::Update]
    }
}

/// Import progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub phase: ImportPhase,
    pub rows_read: i64,
    pub rows_processed: i64,
    pub rows_inserted: i64,
    pub rows_skipped: i64,
    pub rows_failed: i64,
    pub current_batch: i32,
    pub total_batches: i32,
    pub elapsed_ms: i64,
    pub estimated_remaining_ms: Option<i64>,
}

/// Current phase of import
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportPhase {
    Reading,
    Validating,
    Inserting,
}

impl ImportPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            ImportPhase::Reading => "Reading file...",
            ImportPhase::Validating => "Validating data...",
            ImportPhase::Inserting => "Inserting rows...",
        }
    }
}

/// Import result summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub rows_inserted: i64,
    pub rows_updated: i64,
    pub rows_skipped: i64,
    pub rows_failed: i64,
    pub errors: Vec<ImportError>,
    pub duration_ms: i64,
}

/// Individual import error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
    pub row: i64,
    pub column: Option<String>,
    pub value: Option<String>,
    pub message: String,
    pub sql_state: Option<String>,
}
```

### 24.2 Import Service

```rust
// src/import/service.rs

use crate::import::types::*;
use crate::connection::ConnectionPool;
use crate::error::{Result, TuskError};
use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use tokio_postgres::Client;
use regex::Regex;

pub struct ImportService;

impl ImportService {
    /// Analyze a file and return source information with preview
    pub async fn analyze_file(file_path: &str) -> Result<ImportSource> {
        let path = Path::new(file_path);

        // Detect file type from extension
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| TuskError::Import("Unknown file extension".into()))?;

        let file_type = match extension.to_lowercase().as_str() {
            "csv" | "tsv" => FileType::Csv,
            "json" => Self::detect_json_type(file_path)?,
            "jsonl" | "ndjson" => FileType::JsonLines,
            _ => return Err(TuskError::Import("Unsupported file format".into())),
        };

        let metadata = std::fs::metadata(file_path)?;
        let file_size = metadata.len();

        // Detect encoding (simplified - assume UTF-8)
        let encoding = Self::detect_encoding(file_path).unwrap_or("UTF-8".to_string());

        // Parse preview based on file type
        let (csv_options, preview) = match file_type {
            FileType::Csv => {
                let (opts, prev) = Self::analyze_csv(file_path)?;
                (Some(opts), prev)
            }
            FileType::Json => (None, Self::analyze_json(file_path)?),
            FileType::JsonLines => (None, Self::analyze_jsonl(file_path)?),
        };

        Ok(ImportSource {
            file_path: file_path.to_string(),
            file_type,
            file_size,
            encoding,
            csv_options,
            preview,
        })
    }

    /// Detect whether JSON file is array or newline-delimited
    fn detect_json_type(file_path: &str) -> Result<FileType> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                return if trimmed.starts_with('[') {
                    Ok(FileType::Json)
                } else if trimmed.starts_with('{') {
                    Ok(FileType::JsonLines)
                } else {
                    Err(TuskError::Import("Invalid JSON structure".into()))
                };
            }
        }

        Err(TuskError::Import("Empty file".into()))
    }

    /// Detect file encoding using BOM or heuristics
    fn detect_encoding(file_path: &str) -> Option<String> {
        let file = File::open(file_path).ok()?;
        let mut reader = BufReader::new(file);
        let mut bom = [0u8; 4];

        use std::io::Read;
        let bytes_read = reader.read(&mut bom).ok()?;

        if bytes_read >= 3 && bom[0..3] == [0xEF, 0xBB, 0xBF] {
            return Some("UTF-8".to_string());
        }
        if bytes_read >= 2 && bom[0..2] == [0xFE, 0xFF] {
            return Some("UTF-16BE".to_string());
        }
        if bytes_read >= 2 && bom[0..2] == [0xFF, 0xFE] {
            return Some("UTF-16LE".to_string());
        }

        Some("UTF-8".to_string())
    }

    /// Analyze CSV file and return options and preview
    fn analyze_csv(file_path: &str) -> Result<(CsvOptions, PreviewData)> {
        use csv::ReaderBuilder;

        // Detect delimiter from first line
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let mut first_line = String::new();
        reader.read_line(&mut first_line)?;

        let delimiter = Self::detect_delimiter(&first_line);

        let csv_options = CsvOptions {
            delimiter,
            ..Default::default()
        };

        // Parse with detected options
        let file = File::open(file_path)?;
        let mut csv_reader = ReaderBuilder::new()
            .delimiter(delimiter as u8)
            .has_headers(true)
            .from_reader(file);

        let headers: Vec<String> = csv_reader.headers()?
            .iter()
            .map(|s| s.to_string())
            .collect();

        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut type_hints: Vec<ColumnTypeHint> = headers
            .iter()
            .map(|h| ColumnTypeHint {
                column: h.clone(),
                suggested_type: "text".to_string(),
                sample_values: Vec::new(),
                null_count: 0,
            })
            .collect();

        let mut total_rows = 0i64;

        for result in csv_reader.records() {
            let record = result?;
            total_rows += 1;

            // Collect first 5 rows for preview
            if rows.len() < 5 {
                rows.push(record.iter().map(|s| s.to_string()).collect());
            }

            // Analyze types from first 100 rows
            if total_rows <= 100 {
                for (i, value) in record.iter().enumerate() {
                    if i < type_hints.len() {
                        if value.is_empty() || value == "\\N" {
                            type_hints[i].null_count += 1;
                        } else if type_hints[i].sample_values.len() < 5 {
                            type_hints[i].sample_values.push(value.to_string());
                        }
                    }
                }
            }
        }

        // Infer types from samples
        for hint in &mut type_hints {
            hint.suggested_type = Self::infer_type(&hint.sample_values);
        }

        Ok((csv_options, PreviewData {
            columns: headers,
            rows,
            total_rows,
            detected_types: type_hints,
        }))
    }

    /// Detect CSV delimiter from sample line
    fn detect_delimiter(line: &str) -> char {
        let delimiters = [',', '\t', ';', '|'];
        let mut max_count = 0;
        let mut detected = ',';

        for &d in &delimiters {
            let count = line.matches(d).count();
            if count > max_count {
                max_count = count;
                detected = d;
            }
        }

        detected
    }

    /// Infer PostgreSQL type from sample values
    fn infer_type(samples: &[String]) -> String {
        if samples.is_empty() {
            return "text".to_string();
        }

        // Check if all samples match a type
        let all_int = samples.iter().all(|s| s.parse::<i64>().is_ok());
        if all_int {
            return "bigint".to_string();
        }

        let all_float = samples.iter().all(|s| s.parse::<f64>().is_ok());
        if all_float {
            return "numeric".to_string();
        }

        let all_bool = samples.iter().all(|s| {
            matches!(s.to_lowercase().as_str(),
                "true" | "false" | "t" | "f" | "yes" | "no" | "1" | "0")
        });
        if all_bool {
            return "boolean".to_string();
        }

        // Check for date patterns
        let date_pattern = Regex::new(r"^\d{4}-\d{2}-\d{2}").unwrap();
        let all_date = samples.iter().all(|s| date_pattern.is_match(s));
        if all_date {
            if samples.iter().any(|s| s.contains('T') || s.contains(' ')) {
                return "timestamp".to_string();
            }
            return "date".to_string();
        }

        // Check for UUID
        let uuid_pattern = Regex::new(
            r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"
        ).unwrap();
        let all_uuid = samples.iter().all(|s| uuid_pattern.is_match(s));
        if all_uuid {
            return "uuid".to_string();
        }

        "text".to_string()
    }

    /// Analyze JSON array file
    fn analyze_json(file_path: &str) -> Result<PreviewData> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let data: serde_json::Value = serde_json::from_reader(reader)?;

        let array = data.as_array()
            .ok_or_else(|| TuskError::Import("JSON must be an array".into()))?;

        let total_rows = array.len() as i64;

        // Get columns from first object
        let columns: Vec<String> = if let Some(first) = array.first() {
            if let Some(obj) = first.as_object() {
                obj.keys().cloned().collect()
            } else {
                return Err(TuskError::Import("JSON array must contain objects".into()));
            }
        } else {
            Vec::new()
        };

        // Get preview rows
        let rows: Vec<Vec<String>> = array.iter()
            .take(5)
            .filter_map(|v| {
                v.as_object().map(|obj| {
                    columns.iter()
                        .map(|col| Self::json_value_to_string(obj.get(col)))
                        .collect()
                })
            })
            .collect();

        // Detect types
        let detected_types = columns.iter()
            .map(|col| {
                let samples: Vec<String> = array.iter()
                    .take(100)
                    .filter_map(|v| {
                        v.as_object()?.get(col).map(|v| Self::json_value_to_string(Some(v)))
                    })
                    .filter(|s| !s.is_empty())
                    .take(5)
                    .collect();

                ColumnTypeHint {
                    column: col.clone(),
                    suggested_type: Self::infer_type(&samples),
                    sample_values: samples,
                    null_count: 0,
                }
            })
            .collect();

        Ok(PreviewData {
            columns,
            rows,
            total_rows,
            detected_types,
        })
    }

    /// Analyze JSON Lines file
    fn analyze_jsonl(file_path: &str) -> Result<PreviewData> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        let mut columns: Vec<String> = Vec::new();
        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut total_rows = 0i64;

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            total_rows += 1;

            let obj: serde_json::Value = serde_json::from_str(&line)?;
            let obj = obj.as_object()
                .ok_or_else(|| TuskError::Import("Each line must be a JSON object".into()))?;

            // Get columns from first row
            if columns.is_empty() {
                columns = obj.keys().cloned().collect();
            }

            // Collect preview rows
            if rows.len() < 5 {
                let row: Vec<String> = columns.iter()
                    .map(|col| Self::json_value_to_string(obj.get(col)))
                    .collect();
                rows.push(row);
            }
        }

        let detected_types = columns.iter()
            .map(|col| ColumnTypeHint {
                column: col.clone(),
                suggested_type: "text".to_string(),
                sample_values: Vec::new(),
                null_count: 0,
            })
            .collect();

        Ok(PreviewData {
            columns,
            rows,
            total_rows,
            detected_types,
        })
    }

    fn json_value_to_string(value: Option<&serde_json::Value>) -> String {
        match value {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Null) => String::new(),
            Some(other) => other.to_string(),
            None => String::new(),
        }
    }

    /// Get tables available for import target
    pub async fn get_available_tables(
        pool: &ConnectionPool,
        schema: &str,
    ) -> Result<Vec<(String, Vec<TableColumn>)>> {
        let client = pool.get().await?;

        let rows = client.query(
            "SELECT c.table_name, c.column_name, c.data_type,
                    c.is_nullable = 'YES' as nullable,
                    c.column_default IS NOT NULL as has_default
             FROM information_schema.columns c
             JOIN information_schema.tables t
               ON c.table_schema = t.table_schema
              AND c.table_name = t.table_name
             WHERE c.table_schema = $1
               AND t.table_type = 'BASE TABLE'
             ORDER BY c.table_name, c.ordinal_position",
            &[&schema],
        ).await?;

        let mut tables: Vec<(String, Vec<TableColumn>)> = Vec::new();
        let mut current_table: Option<(String, Vec<TableColumn>)> = None;

        for row in rows {
            let table_name: String = row.get(0);
            let column = TableColumn {
                name: row.get(1),
                data_type: row.get(2),
                nullable: row.get(3),
                has_default: row.get(4),
            };

            match &mut current_table {
                Some((name, columns)) if name == &table_name => {
                    columns.push(column);
                }
                _ => {
                    if let Some(prev) = current_table.take() {
                        tables.push(prev);
                    }
                    current_table = Some((table_name, vec![column]));
                }
            }
        }

        if let Some(last) = current_table {
            tables.push(last);
        }

        Ok(tables)
    }

    /// Create auto-mappings based on column name matching
    pub fn create_auto_mappings(
        source: &ImportSource,
        target: &ImportTarget,
    ) -> Vec<ColumnMapping> {
        source.preview.columns.iter()
            .map(|src_col| {
                let target_col = target.columns.iter()
                    .find(|tc| tc.name.to_lowercase() == src_col.to_lowercase())
                    .map(|tc| tc.name.clone());

                ColumnMapping {
                    source_column: src_col.clone(),
                    target_column: target_col,
                    transform: None,
                }
            })
            .collect()
    }

    /// Execute the import operation
    pub async fn execute_import(
        pool: &ConnectionPool,
        job: &ImportJob,
        progress_callback: impl Fn(ImportProgress) + Send + 'static,
    ) -> Result<ImportResult> {
        let start = std::time::Instant::now();
        let client = pool.get().await?;

        let target = job.target.as_ref()
            .ok_or_else(|| TuskError::Import("No target specified".into()))?;

        // Truncate if requested
        if job.options.truncate_first {
            let truncate_sql = format!(
                "TRUNCATE TABLE {}.{}",
                Self::quote_ident(&target.schema),
                Self::quote_ident(&target.table)
            );
            client.execute(&truncate_sql, &[]).await?;
        }

        // Build target columns list
        let target_columns: Vec<&str> = job.mappings.iter()
            .filter_map(|m| m.target_column.as_deref())
            .collect();

        if target_columns.is_empty() {
            return Err(TuskError::Import("No columns mapped".into()));
        }

        // Choose import method based on options
        let result = if job.options.use_copy &&
                        job.options.conflict_handling == ConflictHandling::Error {
            Self::import_with_copy(
                &client,
                job,
                target,
                &target_columns,
                progress_callback,
            ).await?
        } else {
            Self::import_with_insert(
                &client,
                job,
                target,
                &target_columns,
                progress_callback,
            ).await?
        };

        let mut final_result = result;
        final_result.duration_ms = start.elapsed().as_millis() as i64;
        final_result.success = final_result.errors.is_empty() || final_result.rows_inserted > 0;

        Ok(final_result)
    }

    /// Import using PostgreSQL COPY protocol for best performance
    async fn import_with_copy(
        client: &Client,
        job: &ImportJob,
        target: &ImportTarget,
        target_columns: &[&str],
        progress_callback: impl Fn(ImportProgress) + Send + 'static,
    ) -> Result<ImportResult> {
        use csv::ReaderBuilder;
        use tokio_postgres::types::ToSql;

        let mut result = ImportResult {
            success: true,
            rows_inserted: 0,
            rows_updated: 0,
            rows_skipped: 0,
            rows_failed: 0,
            errors: Vec::new(),
            duration_ms: 0,
        };

        // Build COPY command
        let columns_sql = target_columns.iter()
            .map(|c| Self::quote_ident(c))
            .collect::<Vec<_>>()
            .join(", ");

        let copy_sql = format!(
            "COPY {}.{} ({}) FROM STDIN WITH (FORMAT csv, HEADER false, NULL '\\N')",
            Self::quote_ident(&target.schema),
            Self::quote_ident(&target.table),
            columns_sql
        );

        // Start COPY
        let sink = client.copy_in(&copy_sql).await?;
        let mut writer = std::pin::pin!(sink);

        let source = &job.source;
        let mappings = &job.mappings;

        match source.file_type {
            FileType::Csv => {
                let opts = source.csv_options.as_ref().unwrap();
                let file = File::open(&source.file_path)?;
                let mut csv_reader = ReaderBuilder::new()
                    .delimiter(opts.delimiter as u8)
                    .has_headers(opts.has_header)
                    .from_reader(file);

                let mut row_num = 0i64;
                let mut csv_buffer = Vec::new();

                for record_result in csv_reader.records() {
                    row_num += 1;

                    match record_result {
                        Ok(record) => {
                            // Apply mappings and build CSV row
                            let values: Vec<String> = mappings.iter()
                                .filter(|m| m.target_column.is_some())
                                .map(|m| {
                                    let idx = source.preview.columns.iter()
                                        .position(|c| c == &m.source_column)
                                        .unwrap_or(0);
                                    let value = record.get(idx).unwrap_or("");
                                    Self::apply_transform(value, m.transform.as_ref())
                                })
                                .collect();

                            // Write CSV line to buffer
                            csv_buffer.clear();
                            for (i, val) in values.iter().enumerate() {
                                if i > 0 {
                                    csv_buffer.push(b',');
                                }
                                // Quote values containing special chars
                                if val.contains(',') || val.contains('"') || val.contains('\n') {
                                    csv_buffer.push(b'"');
                                    for c in val.bytes() {
                                        if c == b'"' {
                                            csv_buffer.push(b'"');
                                        }
                                        csv_buffer.push(c);
                                    }
                                    csv_buffer.push(b'"');
                                } else if val.is_empty() {
                                    csv_buffer.extend_from_slice(b"\\N");
                                } else {
                                    csv_buffer.extend_from_slice(val.as_bytes());
                                }
                            }
                            csv_buffer.push(b'\n');

                            // Write to COPY stream
                            use futures::SinkExt;
                            if let Err(e) = writer.as_mut().send(bytes::Bytes::copy_from_slice(&csv_buffer)).await {
                                result.rows_failed += 1;
                                result.errors.push(ImportError {
                                    row: row_num,
                                    column: None,
                                    value: None,
                                    message: e.to_string(),
                                    sql_state: None,
                                });
                            } else {
                                result.rows_inserted += 1;
                            }
                        }
                        Err(e) => {
                            result.rows_failed += 1;
                            result.errors.push(ImportError {
                                row: row_num,
                                column: None,
                                value: None,
                                message: e.to_string(),
                                sql_state: None,
                            });
                        }
                    }

                    // Report progress
                    if row_num % 1000 == 0 {
                        progress_callback(ImportProgress {
                            phase: ImportPhase::Inserting,
                            rows_read: row_num,
                            rows_processed: row_num,
                            rows_inserted: result.rows_inserted,
                            rows_skipped: result.rows_skipped,
                            rows_failed: result.rows_failed,
                            current_batch: 0,
                            total_batches: 0,
                            elapsed_ms: 0,
                            estimated_remaining_ms: None,
                        });
                    }
                }

                // Finish COPY
                use futures::SinkExt;
                writer.close().await?;
            }
            FileType::Json | FileType::JsonLines => {
                // For JSON, convert to CSV format for COPY
                Self::import_json_with_copy(
                    &mut writer,
                    source,
                    mappings,
                    &mut result,
                    &progress_callback,
                ).await?;
            }
        }

        Ok(result)
    }

    /// Import JSON data using COPY
    async fn import_json_with_copy(
        writer: &mut std::pin::Pin<&mut tokio_postgres::CopyInSink<bytes::Bytes>>,
        source: &ImportSource,
        mappings: &[ColumnMapping],
        result: &mut ImportResult,
        progress_callback: &impl Fn(ImportProgress),
    ) -> Result<()> {
        use futures::SinkExt;

        let file = File::open(&source.file_path)?;
        let reader = BufReader::new(file);

        let rows: Vec<serde_json::Value> = match source.file_type {
            FileType::Json => {
                let data: serde_json::Value = serde_json::from_reader(reader)?;
                data.as_array().cloned().unwrap_or_default()
            }
            FileType::JsonLines => {
                reader.lines()
                    .filter_map(|line| line.ok())
                    .filter(|line| !line.trim().is_empty())
                    .filter_map(|line| serde_json::from_str(&line).ok())
                    .collect()
            }
            _ => Vec::new(),
        };

        let mut row_num = 0i64;
        let mut csv_buffer = Vec::new();

        for row in rows {
            row_num += 1;

            if let Some(obj) = row.as_object() {
                // Build CSV row from JSON object
                csv_buffer.clear();
                let mut first = true;

                for mapping in mappings {
                    if mapping.target_column.is_none() {
                        continue;
                    }

                    if !first {
                        csv_buffer.push(b',');
                    }
                    first = false;

                    let value = obj.get(&mapping.source_column)
                        .map(|v| Self::json_value_to_string(Some(v)))
                        .unwrap_or_default();

                    let transformed = Self::apply_transform(&value, mapping.transform.as_ref());

                    if transformed.contains(',') || transformed.contains('"') || transformed.contains('\n') {
                        csv_buffer.push(b'"');
                        for c in transformed.bytes() {
                            if c == b'"' {
                                csv_buffer.push(b'"');
                            }
                            csv_buffer.push(c);
                        }
                        csv_buffer.push(b'"');
                    } else if transformed.is_empty() {
                        csv_buffer.extend_from_slice(b"\\N");
                    } else {
                        csv_buffer.extend_from_slice(transformed.as_bytes());
                    }
                }
                csv_buffer.push(b'\n');

                if let Err(e) = writer.as_mut().send(bytes::Bytes::copy_from_slice(&csv_buffer)).await {
                    result.rows_failed += 1;
                    result.errors.push(ImportError {
                        row: row_num,
                        column: None,
                        value: None,
                        message: e.to_string(),
                        sql_state: None,
                    });
                } else {
                    result.rows_inserted += 1;
                }
            }

            if row_num % 1000 == 0 {
                progress_callback(ImportProgress {
                    phase: ImportPhase::Inserting,
                    rows_read: row_num,
                    rows_processed: row_num,
                    rows_inserted: result.rows_inserted,
                    rows_skipped: result.rows_skipped,
                    rows_failed: result.rows_failed,
                    current_batch: 0,
                    total_batches: 0,
                    elapsed_ms: 0,
                    estimated_remaining_ms: None,
                });
            }
        }

        writer.close().await?;
        Ok(())
    }

    /// Import using INSERT statements for conflict handling
    async fn import_with_insert(
        client: &Client,
        job: &ImportJob,
        target: &ImportTarget,
        target_columns: &[&str],
        progress_callback: impl Fn(ImportProgress) + Send + 'static,
    ) -> Result<ImportResult> {
        use csv::ReaderBuilder;

        let mut result = ImportResult {
            success: true,
            rows_inserted: 0,
            rows_updated: 0,
            rows_skipped: 0,
            rows_failed: 0,
            errors: Vec::new(),
            duration_ms: 0,
        };

        // Build INSERT statement
        let columns_sql = target_columns.iter()
            .map(|c| Self::quote_ident(c))
            .collect::<Vec<_>>()
            .join(", ");

        let placeholders: Vec<String> = (1..=target_columns.len())
            .map(|i| format!("${}", i))
            .collect();

        let mut insert_sql = format!(
            "INSERT INTO {}.{} ({}) VALUES ({})",
            Self::quote_ident(&target.schema),
            Self::quote_ident(&target.table),
            columns_sql,
            placeholders.join(", ")
        );

        // Add ON CONFLICT clause if needed
        match job.options.conflict_handling {
            ConflictHandling::Skip => {
                if !job.options.conflict_columns.is_empty() {
                    let conflict_sql = job.options.conflict_columns.iter()
                        .map(|c| Self::quote_ident(c))
                        .collect::<Vec<_>>()
                        .join(", ");
                    insert_sql.push_str(&format!(" ON CONFLICT ({}) DO NOTHING", conflict_sql));
                }
            }
            ConflictHandling::Update => {
                if !job.options.conflict_columns.is_empty() && !job.options.update_columns.is_empty() {
                    let conflict_sql = job.options.conflict_columns.iter()
                        .map(|c| Self::quote_ident(c))
                        .collect::<Vec<_>>()
                        .join(", ");

                    let update_sql = job.options.update_columns.iter()
                        .map(|c| format!("{} = EXCLUDED.{}", Self::quote_ident(c), Self::quote_ident(c)))
                        .collect::<Vec<_>>()
                        .join(", ");

                    insert_sql.push_str(&format!(
                        " ON CONFLICT ({}) DO UPDATE SET {}",
                        conflict_sql,
                        update_sql
                    ));
                }
            }
            ConflictHandling::Error => {}
        }

        // Prepare statement
        let stmt = client.prepare(&insert_sql).await?;

        // Begin transaction if requested
        if job.options.use_transaction {
            client.execute("BEGIN", &[]).await?;
        }

        let source = &job.source;
        let mappings = &job.mappings;

        // Process rows based on file type
        let rows_to_process = Self::read_all_rows(source)?;
        let total_rows = rows_to_process.len() as i64;
        let batch_size = job.options.batch_size;
        let total_batches = ((total_rows as usize + batch_size - 1) / batch_size) as i32;

        for (batch_num, batch) in rows_to_process.chunks(batch_size).enumerate() {
            for (row_idx, row_values) in batch.iter().enumerate() {
                let row_num = (batch_num * batch_size + row_idx + 1) as i64;

                // Apply mappings
                let values: Vec<String> = mappings.iter()
                    .filter(|m| m.target_column.is_some())
                    .map(|m| {
                        let idx = source.preview.columns.iter()
                            .position(|c| c == &m.source_column)
                            .unwrap_or(0);
                        let value = row_values.get(idx).map(|s| s.as_str()).unwrap_or("");
                        Self::apply_transform(value, m.transform.as_ref())
                    })
                    .collect();

                // Execute insert
                let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = values.iter()
                    .map(|v| v as &(dyn tokio_postgres::types::ToSql + Sync))
                    .collect();

                match client.execute(&stmt, &params).await {
                    Ok(count) => {
                        if count > 0 {
                            result.rows_inserted += 1;
                        } else {
                            result.rows_skipped += 1;
                        }
                    }
                    Err(e) => {
                        result.rows_failed += 1;
                        result.errors.push(ImportError {
                            row: row_num,
                            column: None,
                            value: None,
                            message: e.to_string(),
                            sql_state: e.code().map(|c| c.code().to_string()),
                        });
                    }
                }
            }

            // Report progress
            progress_callback(ImportProgress {
                phase: ImportPhase::Inserting,
                rows_read: total_rows,
                rows_processed: ((batch_num + 1) * batch_size).min(total_rows as usize) as i64,
                rows_inserted: result.rows_inserted,
                rows_skipped: result.rows_skipped,
                rows_failed: result.rows_failed,
                current_batch: batch_num as i32 + 1,
                total_batches,
                elapsed_ms: 0,
                estimated_remaining_ms: None,
            });
        }

        // Commit or rollback transaction
        if job.options.use_transaction {
            if result.errors.is_empty() {
                client.execute("COMMIT", &[]).await?;
            } else {
                client.execute("ROLLBACK", &[]).await?;
            }
        }

        Ok(result)
    }

    /// Read all rows from source file
    fn read_all_rows(source: &ImportSource) -> Result<Vec<Vec<String>>> {
        use csv::ReaderBuilder;

        match source.file_type {
            FileType::Csv => {
                let opts = source.csv_options.as_ref().unwrap();
                let file = File::open(&source.file_path)?;
                let mut csv_reader = ReaderBuilder::new()
                    .delimiter(opts.delimiter as u8)
                    .has_headers(opts.has_header)
                    .from_reader(file);

                let mut rows = Vec::new();
                for result in csv_reader.records() {
                    let record = result?;
                    rows.push(record.iter().map(|s| s.to_string()).collect());
                }
                Ok(rows)
            }
            FileType::Json => {
                let file = File::open(&source.file_path)?;
                let reader = BufReader::new(file);
                let data: serde_json::Value = serde_json::from_reader(reader)?;

                let array = data.as_array()
                    .ok_or_else(|| TuskError::Import("Invalid JSON".into()))?;

                let columns = &source.preview.columns;
                Ok(array.iter()
                    .filter_map(|v| {
                        v.as_object().map(|obj| {
                            columns.iter()
                                .map(|col| Self::json_value_to_string(obj.get(col)))
                                .collect()
                        })
                    })
                    .collect())
            }
            FileType::JsonLines => {
                let file = File::open(&source.file_path)?;
                let reader = BufReader::new(file);
                let columns = &source.preview.columns;

                Ok(reader.lines()
                    .filter_map(|line| line.ok())
                    .filter(|line| !line.trim().is_empty())
                    .filter_map(|line| serde_json::from_str::<serde_json::Value>(&line).ok())
                    .filter_map(|v| {
                        v.as_object().map(|obj| {
                            columns.iter()
                                .map(|col| Self::json_value_to_string(obj.get(col)))
                                .collect()
                        })
                    })
                    .collect())
            }
        }
    }

    /// Apply transformation to a value
    fn apply_transform(value: &str, transform: Option<&ColumnTransform>) -> String {
        let Some(t) = transform else {
            return value.to_string();
        };

        match t.transform_type {
            TransformType::None => value.to_string(),
            TransformType::Trim => value.trim().to_string(),
            TransformType::Uppercase => value.to_uppercase(),
            TransformType::Lowercase => value.to_lowercase(),
            TransformType::ParseBoolean => {
                match value.to_lowercase().as_str() {
                    "true" | "t" | "yes" | "y" | "1" => "true".to_string(),
                    "false" | "f" | "no" | "n" | "0" => "false".to_string(),
                    _ => value.to_string(),
                }
            }
            TransformType::ParseDate => {
                // Try to parse and normalize date format
                if let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%m/%d/%Y") {
                    date.format("%Y-%m-%d").to_string()
                } else if let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%d/%m/%Y") {
                    date.format("%Y-%m-%d").to_string()
                } else {
                    value.to_string()
                }
            }
            TransformType::ParseNumber => {
                // Remove thousands separators
                value.replace(",", "").replace(" ", "")
            }
            TransformType::CustomSql => {
                // Custom SQL is handled at insert time, not transform time
                value.to_string()
            }
        }
    }

    /// Quote identifier for PostgreSQL
    fn quote_ident(s: &str) -> String {
        if s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
           && !s.is_empty()
           && !s.chars().next().unwrap().is_ascii_digit() {
            s.to_string()
        } else {
            format!("\"{}\"", s.replace('"', "\"\""))
        }
    }

    /// Cancel a running import job
    pub async fn cancel_import(job_id: &ImportId) -> Result<()> {
        // Implementation would signal cancellation via shared state
        // For now, this is a placeholder
        Ok(())
    }
}
```

### 24.3 Import State Management

```rust
// src/import/state.rs

use crate::import::types::*;
use gpui::Global;
use parking_lot::RwLock;
use std::sync::Arc;
use std::collections::HashMap;

/// Global import state
pub struct ImportState {
    inner: Arc<RwLock<ImportStateInner>>,
}

struct ImportStateInner {
    /// Active import jobs by ID
    jobs: HashMap<ImportId, ImportJob>,
    /// Currently open wizard connection ID
    active_wizard_conn: Option<String>,
    /// Wizard step (1-5)
    wizard_step: usize,
}

impl Global for ImportState {}

impl ImportState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(ImportStateInner {
                jobs: HashMap::new(),
                active_wizard_conn: None,
                wizard_step: 1,
            })),
        }
    }

    /// Start a new import wizard for a connection
    pub fn start_wizard(&self, conn_id: String) -> ImportId {
        let mut inner = self.inner.write();
        inner.active_wizard_conn = Some(conn_id);
        inner.wizard_step = 1;

        let job_id = ImportId::new();
        // Job will be created when source is selected
        job_id
    }

    /// Get current wizard step
    pub fn wizard_step(&self) -> usize {
        self.inner.read().wizard_step
    }

    /// Set wizard step
    pub fn set_wizard_step(&self, step: usize) {
        self.inner.write().wizard_step = step;
    }

    /// Get active wizard connection
    pub fn active_wizard_conn(&self) -> Option<String> {
        self.inner.read().active_wizard_conn.clone()
    }

    /// Close the wizard
    pub fn close_wizard(&self) {
        let mut inner = self.inner.write();
        inner.active_wizard_conn = None;
        inner.wizard_step = 1;
    }

    /// Create a new import job
    pub fn create_job(&self, source: ImportSource) -> ImportId {
        let mut inner = self.inner.write();
        let id = ImportId::new();

        let job = ImportJob {
            id: id.clone(),
            status: ImportStatus::Configuring,
            source,
            target: None,
            mappings: Vec::new(),
            options: ImportOptions::default(),
            progress: None,
            result: None,
        };

        inner.jobs.insert(id.clone(), job);
        id
    }

    /// Get a job by ID
    pub fn get_job(&self, id: &ImportId) -> Option<ImportJob> {
        self.inner.read().jobs.get(id).cloned()
    }

    /// Update job target
    pub fn set_job_target(&self, id: &ImportId, target: ImportTarget) {
        let mut inner = self.inner.write();
        if let Some(job) = inner.jobs.get_mut(id) {
            job.target = Some(target);
        }
    }

    /// Update job mappings
    pub fn set_job_mappings(&self, id: &ImportId, mappings: Vec<ColumnMapping>) {
        let mut inner = self.inner.write();
        if let Some(job) = inner.jobs.get_mut(id) {
            job.mappings = mappings;
        }
    }

    /// Update job options
    pub fn set_job_options(&self, id: &ImportId, options: ImportOptions) {
        let mut inner = self.inner.write();
        if let Some(job) = inner.jobs.get_mut(id) {
            job.options = options;
        }
    }

    /// Update job status
    pub fn set_job_status(&self, id: &ImportId, status: ImportStatus) {
        let mut inner = self.inner.write();
        if let Some(job) = inner.jobs.get_mut(id) {
            job.status = status;
        }
    }

    /// Update job progress
    pub fn set_job_progress(&self, id: &ImportId, progress: ImportProgress) {
        let mut inner = self.inner.write();
        if let Some(job) = inner.jobs.get_mut(id) {
            job.progress = Some(progress);
        }
    }

    /// Set job result
    pub fn set_job_result(&self, id: &ImportId, result: ImportResult) {
        let mut inner = self.inner.write();
        if let Some(job) = inner.jobs.get_mut(id) {
            job.result = Some(result);
            job.status = if result.success {
                ImportStatus::Completed
            } else {
                ImportStatus::Failed
            };
        }
    }

    /// Remove a job
    pub fn remove_job(&self, id: &ImportId) {
        self.inner.write().jobs.remove(id);
    }

    /// Get all active jobs
    pub fn active_jobs(&self) -> Vec<ImportJob> {
        self.inner.read().jobs.values()
            .filter(|j| matches!(j.status, ImportStatus::Configuring | ImportStatus::Importing))
            .cloned()
            .collect()
    }
}
```

### 24.4 Import Wizard Components

```rust
// src/import/wizard.rs

use crate::import::types::*;
use crate::import::service::ImportService;
use crate::import::state::ImportState;
use crate::connection::ConnectionPool;
use crate::ui::{Button, Modal, Select, Checkbox, Input, ProgressBar, Table};
use gpui::*;

/// Import wizard dialog with multi-step flow
pub struct ImportWizard {
    conn_id: String,
    job_id: Option<ImportId>,
    step: usize,

    // Step 1: Source
    source: Option<ImportSource>,
    analyzing: bool,
    analyze_error: Option<String>,

    // Step 2: Target
    available_tables: Vec<(String, Vec<TableColumn>)>,
    selected_schema: String,
    selected_table: Option<String>,
    create_new_table: bool,
    new_table_name: String,
    loading_tables: bool,

    // Step 3: Mappings
    mappings: Vec<ColumnMapping>,

    // Step 4: Options
    options: ImportOptions,

    // Step 5: Execute
    executing: bool,
    progress: Option<ImportProgress>,
    result: Option<ImportResult>,

    focus_handle: FocusHandle,
}

impl ImportWizard {
    pub fn new(conn_id: String, cx: &mut Context<Self>) -> Self {
        Self {
            conn_id,
            job_id: None,
            step: 1,
            source: None,
            analyzing: false,
            analyze_error: None,
            available_tables: Vec::new(),
            selected_schema: "public".to_string(),
            selected_table: None,
            create_new_table: false,
            new_table_name: String::new(),
            loading_tables: false,
            mappings: Vec::new(),
            options: ImportOptions::default(),
            executing: false,
            progress: None,
            result: None,
            focus_handle: cx.focus_handle(),
        }
    }

    fn step_title(&self) -> &'static str {
        match self.step {
            1 => "Select Source File",
            2 => "Select Target Table",
            3 => "Map Columns",
            4 => "Import Options",
            5 => "Execute Import",
            _ => "",
        }
    }

    fn can_continue(&self) -> bool {
        match self.step {
            1 => self.source.is_some(),
            2 => self.selected_table.is_some() || (self.create_new_table && !self.new_table_name.is_empty()),
            3 => self.mappings.iter().any(|m| m.target_column.is_some()),
            4 => true,
            5 => false,
            _ => false,
        }
    }

    fn select_file(&mut self, cx: &mut Context<Self>) {
        cx.spawn(|this, mut cx| async move {
            // Use native file dialog
            let path = rfd::AsyncFileDialog::new()
                .add_filter("Data Files", &["csv", "tsv", "json", "jsonl", "ndjson"])
                .add_filter("CSV", &["csv", "tsv"])
                .add_filter("JSON", &["json", "jsonl", "ndjson"])
                .pick_file()
                .await;

            if let Some(path) = path {
                let file_path = path.path().to_string_lossy().to_string();

                this.update(&mut cx, |this, cx| {
                    this.analyzing = true;
                    this.analyze_error = None;
                    cx.notify();
                }).ok();

                // Analyze file
                match ImportService::analyze_file(&file_path).await {
                    Ok(source) => {
                        this.update(&mut cx, |this, cx| {
                            this.source = Some(source);
                            this.analyzing = false;
                            cx.notify();
                        }).ok();
                    }
                    Err(e) => {
                        this.update(&mut cx, |this, cx| {
                            this.analyze_error = Some(e.to_string());
                            this.analyzing = false;
                            cx.notify();
                        }).ok();
                    }
                }
            }
        }).detach();
    }

    fn load_tables(&mut self, cx: &mut Context<Self>) {
        let schema = self.selected_schema.clone();
        let conn_id = self.conn_id.clone();

        self.loading_tables = true;
        cx.notify();

        cx.spawn(|this, mut cx| async move {
            // Get connection pool from global state
            let pool = cx.update(|cx| {
                cx.global::<ConnectionPool>().clone()
            }).ok();

            if let Some(pool) = pool {
                match ImportService::get_available_tables(&pool, &schema).await {
                    Ok(tables) => {
                        this.update(&mut cx, |this, cx| {
                            this.available_tables = tables;
                            this.loading_tables = false;
                            cx.notify();
                        }).ok();
                    }
                    Err(e) => {
                        this.update(&mut cx, |this, cx| {
                            this.loading_tables = false;
                            // Could show error
                            cx.notify();
                        }).ok();
                    }
                }
            }
        }).detach();
    }

    fn next_step(&mut self, cx: &mut Context<Self>) {
        match self.step {
            1 => {
                // Moving to target selection - load tables
                self.step = 2;
                self.load_tables(cx);
            }
            2 => {
                // Build target and auto-map columns
                if let Some(source) = &self.source {
                    let target = if self.create_new_table {
                        // For new table, create columns from source
                        let columns = source.preview.detected_types.iter()
                            .map(|hint| TableColumn {
                                name: hint.column.clone(),
                                data_type: hint.suggested_type.clone(),
                                nullable: hint.null_count > 0,
                                has_default: false,
                            })
                            .collect();

                        ImportTarget {
                            target_type: TargetType::New,
                            schema: self.selected_schema.clone(),
                            table: self.new_table_name.clone(),
                            columns,
                        }
                    } else if let Some(table_name) = &self.selected_table {
                        let columns = self.available_tables.iter()
                            .find(|(name, _)| name == table_name)
                            .map(|(_, cols)| cols.clone())
                            .unwrap_or_default();

                        ImportTarget {
                            target_type: TargetType::Existing,
                            schema: self.selected_schema.clone(),
                            table: table_name.clone(),
                            columns,
                        }
                    } else {
                        return;
                    };

                    // Create auto-mappings
                    self.mappings = ImportService::create_auto_mappings(source, &target);

                    // Store target in job
                    if let Some(job_id) = &self.job_id {
                        cx.global::<ImportState>().set_job_target(job_id, target);
                    }
                }
                self.step = 3;
            }
            3 => {
                // Store mappings
                if let Some(job_id) = &self.job_id {
                    cx.global::<ImportState>().set_job_mappings(job_id, self.mappings.clone());
                }
                self.step = 4;
            }
            4 => {
                // Store options
                if let Some(job_id) = &self.job_id {
                    cx.global::<ImportState>().set_job_options(job_id, self.options.clone());
                }
                self.step = 5;
            }
            _ => {}
        }
        cx.notify();
    }

    fn prev_step(&mut self, cx: &mut Context<Self>) {
        if self.step > 1 {
            self.step -= 1;
            cx.notify();
        }
    }

    fn execute_import(&mut self, cx: &mut Context<Self>) {
        let Some(job_id) = self.job_id.clone() else { return };

        self.executing = true;
        self.progress = Some(ImportProgress {
            phase: ImportPhase::Reading,
            rows_read: 0,
            rows_processed: 0,
            rows_inserted: 0,
            rows_skipped: 0,
            rows_failed: 0,
            current_batch: 0,
            total_batches: 0,
            elapsed_ms: 0,
            estimated_remaining_ms: None,
        });
        cx.notify();

        cx.spawn(|this, mut cx| async move {
            // Get job and pool
            let (job, pool) = cx.update(|cx| {
                let state = cx.global::<ImportState>();
                let job = state.get_job(&job_id);
                let pool = cx.global::<ConnectionPool>().clone();
                (job, pool)
            }).ok().flatten().unzip();

            let Some(job) = job else { return };
            let Some(pool) = pool else { return };

            // Execute import with progress callback
            let this_clone = this.clone();
            let job_id_clone = job_id.clone();

            let result = ImportService::execute_import(
                &pool,
                &job,
                move |progress| {
                    let this = this_clone.clone();
                    let job_id = job_id_clone.clone();
                    // Update progress
                    // Note: In real impl, would use channel for async update
                },
            ).await;

            match result {
                Ok(result) => {
                    this.update(&mut cx, |this, cx| {
                        this.result = Some(result.clone());
                        this.executing = false;

                        // Update global state
                        cx.global::<ImportState>().set_job_result(&job_id, result);
                        cx.notify();
                    }).ok();
                }
                Err(e) => {
                    this.update(&mut cx, |this, cx| {
                        this.result = Some(ImportResult {
                            success: false,
                            rows_inserted: 0,
                            rows_updated: 0,
                            rows_skipped: 0,
                            rows_failed: 0,
                            errors: vec![ImportError {
                                row: 0,
                                column: None,
                                value: None,
                                message: e.to_string(),
                                sql_state: None,
                            }],
                            duration_ms: 0,
                        });
                        this.executing = false;
                        cx.notify();
                    }).ok();
                }
            }
        }).detach();
    }

    fn render_step_indicator(&self, cx: &Context<Self>) -> impl IntoElement {
        let steps = ["Source", "Target", "Mapping", "Options", "Execute"];

        div()
            .flex()
            .items_center()
            .gap_2()
            .children(steps.iter().enumerate().map(|(i, title)| {
                let step_num = i + 1;
                let is_completed = step_num < self.step;
                let is_current = step_num == self.step;

                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w_8()
                            .h_8()
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_sm()
                            .font_medium()
                            .when(is_completed, |el| {
                                el.bg(rgb(0x22C55E))
                                    .text_color(rgb(0xFFFFFF))
                            })
                            .when(is_current, |el| {
                                el.bg(rgb(0x3B82F6))
                                    .text_color(rgb(0xFFFFFF))
                            })
                            .when(!is_completed && !is_current, |el| {
                                el.bg(rgb(0xE5E7EB))
                                    .text_color(rgb(0x6B7280))
                            })
                            .child(if is_completed { "✓".to_string() } else { step_num.to_string() })
                    )
                    .child(
                        div()
                            .text_sm()
                            .when(is_current, |el| el.font_medium())
                            .when(!is_current, |el| el.text_color(rgb(0x6B7280)))
                            .child(*title)
                    )
                    .when(i < steps.len() - 1, |el| {
                        el.child(
                            div()
                                .w_8()
                                .h_px()
                                .mx_2()
                                .bg(rgb(0xE5E7EB))
                        )
                    })
            }))
    }

    fn render_step_1(&self, cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                // File selection
                div()
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .mb_2()
                            .child("Source File")
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Input::new("file-path")
                                    .placeholder("Select a file...")
                                    .value(self.source.as_ref().map(|s| s.file_path.clone()).unwrap_or_default())
                                    .readonly(true)
                                    .flex_1()
                            )
                            .child(
                                Button::new("browse")
                                    .label("Browse...")
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.select_file(cx);
                                    }))
                            )
                    )
            )
            .when(self.analyzing, |el| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .py_8()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child("Analyzing file...")
                        )
                )
            })
            .when(self.analyze_error.is_some(), |el| {
                el.child(
                    div()
                        .p_4()
                        .rounded_md()
                        .bg(rgb(0xFEE2E2))
                        .border_1()
                        .border_color(rgb(0xFCA5A5))
                        .text_color(rgb(0xB91C1C))
                        .child(self.analyze_error.clone().unwrap_or_default())
                )
            })
            .when(self.source.is_some(), |el| {
                let source = self.source.as_ref().unwrap();
                el.child(self.render_file_info(source, cx))
                    .child(self.render_preview_table(source, cx))
            })
    }

    fn render_file_info(&self, source: &ImportSource, cx: &Context<Self>) -> impl IntoElement {
        div()
            .grid()
            .grid_cols_4()
            .gap_4()
            .p_4()
            .rounded_md()
            .bg(rgb(0xF9FAFB))
            .child(
                div()
                    .child(div().text_xs().text_color(rgb(0x6B7280)).child("Type"))
                    .child(div().font_medium().child(source.file_type.as_str()))
            )
            .child(
                div()
                    .child(div().text_xs().text_color(rgb(0x6B7280)).child("Size"))
                    .child(div().font_medium().child(format_size(source.file_size)))
            )
            .child(
                div()
                    .child(div().text_xs().text_color(rgb(0x6B7280)).child("Encoding"))
                    .child(div().font_medium().child(&source.encoding))
            )
            .child(
                div()
                    .child(div().text_xs().text_color(rgb(0x6B7280)).child("Rows"))
                    .child(div().font_medium().child(format!("{}", source.preview.total_rows)))
            )
    }

    fn render_preview_table(&self, source: &ImportSource, cx: &Context<Self>) -> impl IntoElement {
        div()
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .mb_2()
                    .child("Preview (first 5 rows)")
            )
            .child(
                div()
                    .overflow_auto()
                    .border_1()
                    .border_color(rgb(0xE5E7EB))
                    .rounded_md()
                    .child(
                        // Table header
                        div()
                            .flex()
                            .bg(rgb(0xF9FAFB))
                            .border_b_1()
                            .border_color(rgb(0xE5E7EB))
                            .children(source.preview.columns.iter().map(|col| {
                                div()
                                    .px_3()
                                    .py_2()
                                    .text_xs()
                                    .font_medium()
                                    .text_color(rgb(0x6B7280))
                                    .min_w(px(120.))
                                    .child(col.clone())
                            }))
                    )
                    .children(source.preview.rows.iter().map(|row| {
                        div()
                            .flex()
                            .border_b_1()
                            .border_color(rgb(0xE5E7EB))
                            .children(row.iter().map(|cell| {
                                div()
                                    .px_3()
                                    .py_2()
                                    .text_xs()
                                    .font_family("monospace")
                                    .min_w(px(120.))
                                    .max_w(px(200.))
                                    .truncate()
                                    .child(if cell.is_empty() { "<empty>" } else { cell.as_str() })
                            }))
                    }))
            )
    }

    fn render_step_2(&self, cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                // Schema selector
                div()
                    .child(div().text_sm().font_medium().mb_2().child("Schema"))
                    .child(
                        Select::new("schema")
                            .value(self.selected_schema.clone())
                            .options(vec![
                                ("public".to_string(), "public".to_string()),
                            ])
                            .on_change(cx.listener(|this, value: String, cx| {
                                this.selected_schema = value;
                                this.load_tables(cx);
                            }))
                    )
            )
            .child(
                // Create new table option
                div()
                    .child(
                        Checkbox::new("create-new")
                            .label("Create new table")
                            .checked(self.create_new_table)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.create_new_table = checked;
                                if checked {
                                    this.selected_table = None;
                                }
                                cx.notify();
                            }))
                    )
            )
            .when(self.create_new_table, |el| {
                el.child(
                    div()
                        .child(div().text_sm().font_medium().mb_2().child("New Table Name"))
                        .child(
                            Input::new("new-table-name")
                                .placeholder("Enter table name...")
                                .value(self.new_table_name.clone())
                                .on_change(cx.listener(|this, value: String, cx| {
                                    this.new_table_name = value;
                                    cx.notify();
                                }))
                        )
                )
            })
            .when(!self.create_new_table, |el| {
                el.child(
                    div()
                        .child(div().text_sm().font_medium().mb_2().child("Select Table"))
                        .when(self.loading_tables, |el| {
                            el.child(div().text_sm().text_color(rgb(0x6B7280)).child("Loading tables..."))
                        })
                        .when(!self.loading_tables, |el| {
                            let table_options: Vec<(String, String)> = self.available_tables.iter()
                                .map(|(name, _)| (name.clone(), name.clone()))
                                .collect();

                            el.child(
                                Select::new("table")
                                    .value(self.selected_table.clone().unwrap_or_default())
                                    .options(table_options)
                                    .on_change(cx.listener(|this, value: String, cx| {
                                        this.selected_table = Some(value);
                                        cx.notify();
                                    }))
                            )
                        })
                )
            })
            // Show target columns preview
            .when(self.selected_table.is_some() && !self.create_new_table, |el| {
                let table_name = self.selected_table.as_ref().unwrap();
                let columns = self.available_tables.iter()
                    .find(|(name, _)| name == table_name)
                    .map(|(_, cols)| cols.clone())
                    .unwrap_or_default();

                el.child(
                    div()
                        .child(div().text_sm().font_medium().mb_2().child("Target Columns"))
                        .child(
                            div()
                                .border_1()
                                .border_color(rgb(0xE5E7EB))
                                .rounded_md()
                                .max_h(px(200.))
                                .overflow_auto()
                                .children(columns.iter().map(|col| {
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .px_3()
                                        .py_2()
                                        .border_b_1()
                                        .border_color(rgb(0xE5E7EB))
                                        .child(
                                            div()
                                                .font_medium()
                                                .child(&col.name)
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(rgb(0x6B7280))
                                                .child(&col.data_type)
                                        )
                                }))
                        )
                )
            })
    }

    fn render_step_3(&self, cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x6B7280))
                    .child("Map source columns to target columns. Columns without a mapping will be skipped.")
            )
            .child(
                div()
                    .border_1()
                    .border_color(rgb(0xE5E7EB))
                    .rounded_md()
                    .overflow_auto()
                    .max_h(px(400.))
                    // Header
                    .child(
                        div()
                            .flex()
                            .bg(rgb(0xF9FAFB))
                            .border_b_1()
                            .border_color(rgb(0xE5E7EB))
                            .px_3()
                            .py_2()
                            .child(div().flex_1().text_sm().font_medium().child("Source Column"))
                            .child(div().w(px(48.)).text_center().text_sm().font_medium().child("→"))
                            .child(div().flex_1().text_sm().font_medium().child("Target Column"))
                            .child(div().w(px(150.)).text_sm().font_medium().child("Transform"))
                    )
                    // Mappings
                    .children(self.mappings.iter().enumerate().map(|(i, mapping)| {
                        self.render_mapping_row(i, mapping, cx)
                    }))
            )
    }

    fn render_mapping_row(&self, index: usize, mapping: &ColumnMapping, cx: &Context<Self>) -> impl IntoElement {
        let target_options = if let Some(source) = &self.source {
            if let Some(job_id) = &self.job_id {
                if let Some(job) = cx.global::<ImportState>().get_job(job_id) {
                    if let Some(target) = &job.target {
                        let mut opts = vec![("".to_string(), "(Skip)".to_string())];
                        opts.extend(target.columns.iter().map(|c| (c.name.clone(), c.name.clone())));
                        opts
                    } else {
                        vec![("".to_string(), "(Skip)".to_string())]
                    }
                } else {
                    vec![("".to_string(), "(Skip)".to_string())]
                }
            } else {
                vec![("".to_string(), "(Skip)".to_string())]
            }
        } else {
            vec![("".to_string(), "(Skip)".to_string())]
        };

        let transform_options: Vec<(String, String)> = TransformType::all().iter()
            .map(|t| (format!("{:?}", t), t.as_str().to_string()))
            .collect();

        div()
            .flex()
            .items_center()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(rgb(0xE5E7EB))
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .font_family("monospace")
                    .child(&mapping.source_column)
            )
            .child(
                div()
                    .w(px(48.))
                    .text_center()
                    .text_color(rgb(0x9CA3AF))
                    .child("→")
            )
            .child(
                div()
                    .flex_1()
                    .child(
                        Select::new(format!("target-{}", index))
                            .value(mapping.target_column.clone().unwrap_or_default())
                            .options(target_options)
                            .size_sm()
                            .on_change(cx.listener(move |this, value: String, cx| {
                                if index < this.mappings.len() {
                                    this.mappings[index].target_column = if value.is_empty() {
                                        None
                                    } else {
                                        Some(value)
                                    };
                                    cx.notify();
                                }
                            }))
                    )
            )
            .child(
                div()
                    .w(px(150.))
                    .child(
                        Select::new(format!("transform-{}", index))
                            .value(mapping.transform.as_ref()
                                .map(|t| format!("{:?}", t.transform_type))
                                .unwrap_or_else(|| "None".to_string()))
                            .options(transform_options)
                            .size_sm()
                            .on_change(cx.listener(move |this, value: String, cx| {
                                if index < this.mappings.len() {
                                    let transform_type = match value.as_str() {
                                        "None" => TransformType::None,
                                        "Trim" => TransformType::Trim,
                                        "Uppercase" => TransformType::Uppercase,
                                        "Lowercase" => TransformType::Lowercase,
                                        "ParseDate" => TransformType::ParseDate,
                                        "ParseBoolean" => TransformType::ParseBoolean,
                                        "ParseNumber" => TransformType::ParseNumber,
                                        _ => TransformType::None,
                                    };

                                    this.mappings[index].transform = if transform_type == TransformType::None {
                                        None
                                    } else {
                                        Some(ColumnTransform {
                                            transform_type,
                                            options: std::collections::HashMap::new(),
                                        })
                                    };
                                    cx.notify();
                                }
                            }))
                    )
            )
    }

    fn render_step_4(&self, cx: &Context<Self>) -> impl IntoElement {
        let conflict_options: Vec<(String, String)> = ConflictHandling::all().iter()
            .map(|c| (format!("{:?}", c), c.as_str().to_string()))
            .collect();

        div()
            .flex()
            .flex_col()
            .gap_6()
            // Conflict handling
            .child(
                div()
                    .child(div().text_sm().font_medium().mb_2().child("Conflict Handling"))
                    .child(
                        Select::new("conflict-handling")
                            .value(format!("{:?}", self.options.conflict_handling))
                            .options(conflict_options)
                            .on_change(cx.listener(|this, value: String, cx| {
                                this.options.conflict_handling = match value.as_str() {
                                    "Skip" => ConflictHandling::Skip,
                                    "Update" => ConflictHandling::Update,
                                    _ => ConflictHandling::Error,
                                };
                                cx.notify();
                            }))
                    )
            )
            // Batch size
            .child(
                div()
                    .child(div().text_sm().font_medium().mb_2().child("Batch Size"))
                    .child(
                        Input::new("batch-size")
                            .value(self.options.batch_size.to_string())
                            .on_change(cx.listener(|this, value: String, cx| {
                                if let Ok(size) = value.parse::<usize>() {
                                    this.options.batch_size = size;
                                    cx.notify();
                                }
                            }))
                    )
            )
            // Checkboxes
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        Checkbox::new("use-transaction")
                            .label("Wrap in transaction (rollback on error)")
                            .checked(self.options.use_transaction)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.options.use_transaction = checked;
                                cx.notify();
                            }))
                    )
                    .child(
                        Checkbox::new("use-copy")
                            .label("Use COPY for faster import (requires 'Error on Conflict')")
                            .checked(self.options.use_copy)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.options.use_copy = checked;
                                cx.notify();
                            }))
                    )
                    .child(
                        Checkbox::new("truncate-first")
                            .label("Truncate table before import")
                            .checked(self.options.truncate_first)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.options.truncate_first = checked;
                                cx.notify();
                            }))
                    )
            )
    }

    fn render_step_5(&self, cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_6()
            // Summary
            .when(!self.executing && self.result.is_none(), |el| {
                el.child(self.render_import_summary(cx))
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .child(
                                Button::new("execute")
                                    .label("Start Import")
                                    .variant_primary()
                                    .on_click(cx.listener(|this, _, cx| {
                                        this.execute_import(cx);
                                    }))
                            )
                    )
            })
            // Progress
            .when(self.executing, |el| {
                el.child(self.render_progress(cx))
            })
            // Result
            .when(self.result.is_some(), |el| {
                el.child(self.render_result(cx))
            })
    }

    fn render_import_summary(&self, cx: &Context<Self>) -> impl IntoElement {
        let source = self.source.as_ref();
        let mapped_count = self.mappings.iter().filter(|m| m.target_column.is_some()).count();

        div()
            .p_4()
            .rounded_md()
            .bg(rgb(0xF9FAFB))
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .mb_4()
                    .child("Import Summary")
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .text_sm()
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child("Source file:")
                            .child(source.map(|s| s.file_path.clone()).unwrap_or_default())
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child("Total rows:")
                            .child(source.map(|s| format!("{}", s.preview.total_rows)).unwrap_or_default())
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child("Columns to import:")
                            .child(format!("{}", mapped_count))
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child("Conflict handling:")
                            .child(self.options.conflict_handling.as_str())
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child("Import method:")
                            .child(if self.options.use_copy { "COPY (fast)" } else { "INSERT" })
                    )
            )
    }

    fn render_progress(&self, cx: &Context<Self>) -> impl IntoElement {
        let progress = self.progress.as_ref();

        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_4()
            .py_8()
            .child(
                div()
                    .text_lg()
                    .font_medium()
                    .child(progress.map(|p| p.phase.as_str()).unwrap_or("Importing..."))
            )
            .child(
                div()
                    .w_full()
                    .max_w(px(400.))
                    .child(
                        ProgressBar::new()
                            .value(progress.map(|p| {
                                if p.rows_read > 0 {
                                    (p.rows_processed as f64 / p.rows_read as f64 * 100.0) as i32
                                } else {
                                    0
                                }
                            }).unwrap_or(0))
                    )
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x6B7280))
                    .child(progress.map(|p| {
                        format!("{} / {} rows processed", p.rows_processed, p.rows_read)
                    }).unwrap_or_default())
            )
    }

    fn render_result(&self, cx: &Context<Self>) -> impl IntoElement {
        let result = self.result.as_ref().unwrap();

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .w_12()
                            .h_12()
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_2xl()
                            .when(result.success, |el| {
                                el.bg(rgb(0xDCFCE7)).child("✓")
                            })
                            .when(!result.success, |el| {
                                el.bg(rgb(0xFEE2E2)).child("✗")
                            })
                    )
                    .child(
                        div()
                            .child(
                                div()
                                    .text_lg()
                                    .font_medium()
                                    .child(if result.success { "Import Complete" } else { "Import Failed" })
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x6B7280))
                                    .child(format!("Completed in {:.2}s", result.duration_ms as f64 / 1000.0))
                            )
                    )
            )
            // Stats
            .child(
                div()
                    .grid()
                    .grid_cols_4()
                    .gap_4()
                    .child(
                        div()
                            .p_3()
                            .rounded_md()
                            .bg(rgb(0xDCFCE7))
                            .child(div().text_2xl().font_bold().child(format!("{}", result.rows_inserted)))
                            .child(div().text_xs().text_color(rgb(0x166534)).child("Inserted"))
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_md()
                            .bg(rgb(0xDBEAFE))
                            .child(div().text_2xl().font_bold().child(format!("{}", result.rows_updated)))
                            .child(div().text_xs().text_color(rgb(0x1E40AF)).child("Updated"))
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_md()
                            .bg(rgb(0xFEF3C7))
                            .child(div().text_2xl().font_bold().child(format!("{}", result.rows_skipped)))
                            .child(div().text_xs().text_color(rgb(0x92400E)).child("Skipped"))
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_md()
                            .bg(rgb(0xFEE2E2))
                            .child(div().text_2xl().font_bold().child(format!("{}", result.rows_failed)))
                            .child(div().text_xs().text_color(rgb(0xB91C1C)).child("Failed"))
                    )
            )
            // Errors
            .when(!result.errors.is_empty(), |el| {
                el.child(
                    div()
                        .child(div().text_sm().font_medium().mb_2().child("Errors"))
                        .child(
                            div()
                                .border_1()
                                .border_color(rgb(0xFCA5A5))
                                .rounded_md()
                                .max_h(px(200.))
                                .overflow_auto()
                                .children(result.errors.iter().take(100).map(|err| {
                                    div()
                                        .px_3()
                                        .py_2()
                                        .border_b_1()
                                        .border_color(rgb(0xFEE2E2))
                                        .text_sm()
                                        .child(
                                            div()
                                                .flex()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .text_color(rgb(0xB91C1C))
                                                        .child(format!("Row {}", err.row))
                                                )
                                                .child(err.message.clone())
                                        )
                                }))
                        )
                )
            })
    }
}

impl FocusableView for ImportWizard {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ImportWizard {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        Modal::new("import-wizard")
            .title("Import Data")
            .width(px(800.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_6()
                    // Step indicator
                    .child(self.render_step_indicator(cx))
                    // Step content
                    .child(
                        div()
                            .min_h(px(400.))
                            .child(match self.step {
                                1 => self.render_step_1(cx).into_any_element(),
                                2 => self.render_step_2(cx).into_any_element(),
                                3 => self.render_step_3(cx).into_any_element(),
                                4 => self.render_step_4(cx).into_any_element(),
                                5 => self.render_step_5(cx).into_any_element(),
                                _ => div().into_any_element(),
                            })
                    )
            )
            .footer(
                div()
                    .flex()
                    .justify_between()
                    .child(
                        Button::new("back")
                            .label("Back")
                            .disabled(self.step == 1 || self.executing)
                            .on_click(cx.listener(|this, _, cx| {
                                this.prev_step(cx);
                            }))
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("cancel")
                                    .label("Cancel")
                                    .disabled(self.executing)
                                    .on_click(cx.listener(|this, _, cx| {
                                        cx.emit(ImportWizardEvent::Cancel);
                                    }))
                            )
                            .when(self.step < 5, |el| {
                                el.child(
                                    Button::new("next")
                                        .label(if self.step == 4 { "Review" } else { "Continue" })
                                        .variant_primary()
                                        .disabled(!self.can_continue())
                                        .on_click(cx.listener(|this, _, cx| {
                                            this.next_step(cx);
                                        }))
                                )
                            })
                            .when(self.step == 5 && self.result.is_some(), |el| {
                                el.child(
                                    Button::new("close")
                                        .label("Close")
                                        .variant_primary()
                                        .on_click(cx.listener(|this, _, cx| {
                                            cx.emit(ImportWizardEvent::Complete);
                                        }))
                                )
                            })
                    )
            )
    }
}

/// Events emitted by the import wizard
pub enum ImportWizardEvent {
    Complete,
    Cancel,
}

impl EventEmitter<ImportWizardEvent> for ImportWizard {}

/// Format file size for display
fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
```

### 24.5 Import Panel Integration

```rust
// src/import/panel.rs

use crate::import::wizard::{ImportWizard, ImportWizardEvent};
use crate::import::state::ImportState;
use crate::ui::Button;
use gpui::*;

/// Panel for accessing import functionality
pub struct ImportPanel {
    conn_id: Option<String>,
    wizard: Option<Entity<ImportWizard>>,
}

impl ImportPanel {
    pub fn new() -> Self {
        Self {
            conn_id: None,
            wizard: None,
        }
    }

    pub fn set_connection(&mut self, conn_id: String) {
        self.conn_id = Some(conn_id);
    }

    fn open_wizard(&mut self, cx: &mut Context<Self>) {
        let Some(conn_id) = self.conn_id.clone() else { return };

        let wizard = cx.new(|cx| ImportWizard::new(conn_id, cx));

        cx.subscribe(&wizard, |this, _, event: &ImportWizardEvent, cx| {
            match event {
                ImportWizardEvent::Complete | ImportWizardEvent::Cancel => {
                    this.wizard = None;
                    cx.notify();
                }
            }
        }).detach();

        self.wizard = Some(wizard);
        cx.notify();
    }
}

impl Render for ImportPanel {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            // Import button
            .child(
                Button::new("import")
                    .label("Import Data...")
                    .icon("upload")
                    .disabled(self.conn_id.is_none())
                    .on_click(cx.listener(|this, _, cx| {
                        this.open_wizard(cx);
                    }))
            )
            // Active imports list
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .child("Active Imports")
            )
            .child({
                let active_jobs = cx.global::<ImportState>().active_jobs();
                if active_jobs.is_empty() {
                    div()
                        .text_sm()
                        .text_color(rgb(0x6B7280))
                        .child("No active imports")
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .children(active_jobs.iter().map(|job| {
                            div()
                                .p_3()
                                .rounded_md()
                                .bg(rgb(0xF9FAFB))
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_medium()
                                                .child(job.target.as_ref()
                                                    .map(|t| format!("{}.{}", t.schema, t.table))
                                                    .unwrap_or_else(|| "Configuring...".to_string()))
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0x6B7280))
                                                .child(format!("{:?}", job.status))
                                        )
                                )
                                .when(job.progress.is_some(), |el| {
                                    let progress = job.progress.as_ref().unwrap();
                                    el.child(
                                        div()
                                            .mt_2()
                                            .text_xs()
                                            .text_color(rgb(0x6B7280))
                                            .child(format!("{} rows processed", progress.rows_processed))
                                    )
                                })
                        }))
                        .into_any_element()
                }
            })
            // Render wizard modal if open
            .when_some(self.wizard.clone(), |el, wizard| {
                el.child(wizard)
            })
    }
}
```

## Acceptance Criteria

1. **File Analysis**
   - [ ] Support CSV, TSV, JSON array, and JSON Lines formats
   - [ ] Auto-detect delimiter, encoding, and data types
   - [ ] Preview first 5 rows with column headers
   - [ ] Show file metadata (size, row count, columns)
   - [ ] Handle large files without loading entirely into memory

2. **Target Selection**
   - [ ] List available tables in selected schema
   - [ ] Option to create new table with inferred schema
   - [ ] Display target column types and constraints
   - [ ] Validate target table permissions

3. **Column Mapping**
   - [ ] Auto-map columns by name (case-insensitive)
   - [ ] Allow manual mapping via dropdown selection
   - [ ] Skip columns option (no mapping)
   - [ ] Apply transformations (trim, case, date parse, etc.)
   - [ ] Visual feedback for mapped/unmapped columns

4. **Import Options**
   - [ ] Conflict handling (error, skip, upsert)
   - [ ] Batch size configuration
   - [ ] Transaction mode toggle
   - [ ] COPY vs INSERT selection
   - [ ] Truncate before import option

5. **Execution**
   - [ ] Real-time progress reporting with row counts
   - [ ] Error collection and display (row-level)
   - [ ] Summary of imported/updated/skipped/failed rows
   - [ ] Cancel capability with proper cleanup
   - [ ] Duration and performance metrics

## Performance Considerations

1. **File Analysis**: Stream files rather than loading entirely into memory
2. **COPY Protocol**: Use PostgreSQL COPY for maximum throughput
3. **Batch Processing**: Process rows in configurable batches
4. **Progress Updates**: Throttle UI updates to avoid performance impact
5. **Large Files**: Handle multi-GB files gracefully

## Testing Instructions

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delimiter_detection() {
        assert_eq!(ImportService::detect_delimiter("a,b,c"), ',');
        assert_eq!(ImportService::detect_delimiter("a\tb\tc"), '\t');
        assert_eq!(ImportService::detect_delimiter("a;b;c"), ';');
        assert_eq!(ImportService::detect_delimiter("a|b|c"), '|');
    }

    #[test]
    fn test_type_inference() {
        assert_eq!(ImportService::infer_type(&["1", "2", "3"]), "bigint");
        assert_eq!(ImportService::infer_type(&["1.5", "2.5"]), "numeric");
        assert_eq!(ImportService::infer_type(&["true", "false"]), "boolean");
        assert_eq!(ImportService::infer_type(&["2024-01-15"]), "date");
        assert_eq!(ImportService::infer_type(&["hello"]), "text");
    }

    #[test]
    fn test_transform_trim() {
        let transform = ColumnTransform {
            transform_type: TransformType::Trim,
            options: HashMap::new(),
        };
        assert_eq!(ImportService::apply_transform("  hello  ", Some(&transform)), "hello");
    }

    #[test]
    fn test_transform_boolean() {
        let transform = ColumnTransform {
            transform_type: TransformType::ParseBoolean,
            options: HashMap::new(),
        };
        assert_eq!(ImportService::apply_transform("yes", Some(&transform)), "true");
        assert_eq!(ImportService::apply_transform("no", Some(&transform)), "false");
        assert_eq!(ImportService::apply_transform("1", Some(&transform)), "true");
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_csv_import() {
        // Create test CSV file
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "id,name,email").unwrap();
        writeln!(file, "1,Alice,alice@example.com").unwrap();
        writeln!(file, "2,Bob,bob@example.com").unwrap();

        // Analyze file
        let source = ImportService::analyze_file(file.path().to_str().unwrap())
            .await
            .unwrap();

        assert_eq!(source.file_type, FileType::Csv);
        assert_eq!(source.preview.columns, vec!["id", "name", "email"]);
        assert_eq!(source.preview.total_rows, 2);
    }

    #[tokio::test]
    async fn test_json_import() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"[{{"id":1,"name":"Alice"}},{{"id":2,"name":"Bob"}}]"#).unwrap();

        let source = ImportService::analyze_file(file.path().to_str().unwrap())
            .await
            .unwrap();

        assert_eq!(source.file_type, FileType::Json);
        assert_eq!(source.preview.total_rows, 2);
    }
}
```

### Manual Testing

1. **CSV Import Flow**:
   - Select a CSV file with headers
   - Verify delimiter auto-detection
   - Check preview shows correct data
   - Map columns to existing table
   - Execute import and verify row counts

2. **JSON Import Flow**:
   - Test both JSON array and JSONL formats
   - Verify column extraction from first object
   - Test with nested objects (should flatten first level)

3. **Error Handling**:
   - Import with duplicate key constraint violation
   - Import with type mismatch
   - Import file with malformed rows
   - Cancel mid-import

4. **Performance**:
   - Import 100K+ row CSV file
   - Verify COPY vs INSERT performance difference
   - Monitor memory usage during large imports
