# Feature 24: Import Wizard

## Overview

The Import Wizard provides a step-by-step interface for importing data from CSV and JSON files into PostgreSQL tables, with column mapping, type conversion, data transformation, and conflict handling options.

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

```typescript
// src/lib/types/import.ts

export interface ImportJob {
  id: string;
  status: ImportStatus;
  source: ImportSource;
  target: ImportTarget;
  mapping: ColumnMapping[];
  options: ImportOptions;
  progress: ImportProgress;
  result: ImportResult | null;
}

export type ImportStatus = 'configuring' | 'validating' | 'importing' | 'completed' | 'failed' | 'cancelled';

export interface ImportSource {
  filePath: string;
  fileType: 'csv' | 'json' | 'jsonl';
  fileSize: number;
  encoding: string;
  csvOptions?: CsvOptions;
  preview: PreviewData;
}

export interface CsvOptions {
  delimiter: string;
  quoteChar: string;
  escapeChar: string;
  hasHeader: boolean;
  nullString: string;
  skipRows: number;
}

export interface PreviewData {
  columns: string[];
  rows: string[][];
  totalRows: number;
  detectedTypes: ColumnTypeHint[];
}

export interface ColumnTypeHint {
  column: string;
  suggestedType: string;
  sampleValues: string[];
  nullCount: number;
}

export interface ImportTarget {
  type: 'existing' | 'new';
  schema: string;
  table: string;
  columns?: TableColumn[];
}

export interface TableColumn {
  name: string;
  type: string;
  nullable: boolean;
  hasDefault: boolean;
}

export interface ColumnMapping {
  sourceColumn: string;
  targetColumn: string | null; // null = skip this column
  transform: ColumnTransform | null;
}

export interface ColumnTransform {
  type: TransformType;
  options?: Record<string, any>;
}

export type TransformType =
  | 'none'
  | 'trim'
  | 'uppercase'
  | 'lowercase'
  | 'parse_date'
  | 'parse_boolean'
  | 'parse_number'
  | 'custom_sql';

export interface ImportOptions {
  conflictHandling: 'error' | 'skip' | 'update';
  conflictColumns?: string[]; // For upsert
  updateColumns?: string[]; // Columns to update on conflict
  batchSize: number;
  useTransaction: boolean;
  useCopy: boolean;
  truncateFirst: boolean;
}

export interface ImportProgress {
  phase: 'reading' | 'validating' | 'inserting';
  rowsRead: number;
  rowsProcessed: number;
  rowsInserted: number;
  rowsSkipped: number;
  rowsFailed: number;
  currentBatch: number;
  totalBatches: number;
  elapsedMs: number;
  estimatedRemainingMs: number | null;
}

export interface ImportResult {
  success: boolean;
  rowsInserted: number;
  rowsUpdated: number;
  rowsSkipped: number;
  rowsFailed: number;
  errors: ImportError[];
  duration: number;
}

export interface ImportError {
  row: number;
  column?: string;
  value?: string;
  message: string;
  sqlState?: string;
}
```

### 24.2 Import Service (Rust)

```rust
// src-tauri/src/services/import.rs

use serde::{Deserialize, Serialize};
use tokio_postgres::Client;
use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader};
use csv::ReaderBuilder;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSource {
    pub file_path: String,
    pub file_type: String,
    pub file_size: u64,
    pub encoding: String,
    pub csv_options: Option<CsvOptions>,
    pub preview: PreviewData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvOptions {
    pub delimiter: String,
    pub quote_char: String,
    pub escape_char: String,
    pub has_header: bool,
    pub null_string: String,
    pub skip_rows: i32,
}

impl Default for CsvOptions {
    fn default() -> Self {
        Self {
            delimiter: ",".to_string(),
            quote_char: "\"".to_string(),
            escape_char: "\\".to_string(),
            has_header: true,
            null_string: "\\N".to_string(),
            skip_rows: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewData {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_rows: i64,
    pub detected_types: Vec<ColumnTypeHint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnTypeHint {
    pub column: String,
    pub suggested_type: String,
    pub sample_values: Vec<String>,
    pub null_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnMapping {
    pub source_column: String,
    pub target_column: Option<String>,
    pub transform: Option<ColumnTransform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnTransform {
    pub transform_type: String,
    pub options: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOptions {
    pub conflict_handling: String,
    pub conflict_columns: Option<Vec<String>>,
    pub update_columns: Option<Vec<String>>,
    pub batch_size: i32,
    pub use_transaction: bool,
    pub use_copy: bool,
    pub truncate_first: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            conflict_handling: "error".to_string(),
            conflict_columns: None,
            update_columns: None,
            batch_size: 1000,
            use_transaction: true,
            use_copy: true,
            truncate_first: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgress {
    pub phase: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub success: bool,
    pub rows_inserted: i64,
    pub rows_updated: i64,
    pub rows_skipped: i64,
    pub rows_failed: i64,
    pub errors: Vec<ImportError>,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportError {
    pub row: i64,
    pub column: Option<String>,
    pub value: Option<String>,
    pub message: String,
    pub sql_state: Option<String>,
}

pub struct ImportService;

impl ImportService {
    /// Analyze a file and return preview data
    pub async fn analyze_file(file_path: &str) -> Result<ImportSource, ImportServiceError> {
        let path = Path::new(file_path);

        // Detect file type
        let file_type = match path.extension().and_then(|e| e.to_str()) {
            Some("csv") | Some("tsv") => "csv",
            Some("json") => Self::detect_json_type(file_path)?,
            Some("jsonl") | Some("ndjson") => "jsonl",
            _ => return Err(ImportServiceError::UnsupportedFormat),
        };

        let metadata = std::fs::metadata(file_path)?;
        let file_size = metadata.len();

        // Detect encoding (simplified - assume UTF-8)
        let encoding = "UTF-8".to_string();

        // Parse preview
        let (csv_options, preview) = match file_type {
            "csv" => {
                let (opts, prev) = Self::analyze_csv(file_path)?;
                (Some(opts), prev)
            }
            "json" => (None, Self::analyze_json(file_path)?),
            "jsonl" => (None, Self::analyze_jsonl(file_path)?),
            _ => return Err(ImportServiceError::UnsupportedFormat),
        };

        Ok(ImportSource {
            file_path: file_path.to_string(),
            file_type: file_type.to_string(),
            file_size,
            encoding,
            csv_options,
            preview,
        })
    }

    fn detect_json_type(file_path: &str) -> Result<&'static str, ImportServiceError> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        // Read first non-empty character
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                return if trimmed.starts_with('[') {
                    Ok("json")
                } else if trimmed.starts_with('{') {
                    Ok("jsonl")
                } else {
                    Err(ImportServiceError::InvalidJson)
                };
            }
        }

        Err(ImportServiceError::EmptyFile)
    }

    fn analyze_csv(file_path: &str) -> Result<(CsvOptions, PreviewData), ImportServiceError> {
        // Try to detect delimiter
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let mut first_line = String::new();
        reader.read_line(&mut first_line)?;

        let delimiter = Self::detect_delimiter(&first_line);

        let csv_options = CsvOptions {
            delimiter: delimiter.to_string(),
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
                        } else {
                            if type_hints[i].sample_values.len() < 5 {
                                type_hints[i].sample_values.push(value.to_string());
                            }
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
            matches!(s.to_lowercase().as_str(), "true" | "false" | "t" | "f" | "yes" | "no" | "1" | "0")
        });
        if all_bool {
            return "boolean".to_string();
        }

        // Check for date patterns
        let date_pattern = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}").unwrap();
        let all_date = samples.iter().all(|s| date_pattern.is_match(s));
        if all_date {
            if samples.iter().any(|s| s.contains('T') || s.contains(' ')) {
                return "timestamp".to_string();
            }
            return "date".to_string();
        }

        // Check for UUID
        let uuid_pattern = regex::Regex::new(
            r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"
        ).unwrap();
        let all_uuid = samples.iter().all(|s| uuid_pattern.is_match(s));
        if all_uuid {
            return "uuid".to_string();
        }

        "text".to_string()
    }

    fn analyze_json(file_path: &str) -> Result<PreviewData, ImportServiceError> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let data: serde_json::Value = serde_json::from_reader(reader)?;

        let array = data.as_array()
            .ok_or(ImportServiceError::InvalidJson)?;

        let total_rows = array.len() as i64;

        // Get columns from first object
        let columns: Vec<String> = if let Some(first) = array.first() {
            if let Some(obj) = first.as_object() {
                obj.keys().cloned().collect()
            } else {
                return Err(ImportServiceError::InvalidJson);
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
                        .map(|col| {
                            obj.get(col)
                                .map(|v| match v {
                                    serde_json::Value::String(s) => s.clone(),
                                    serde_json::Value::Null => String::new(),
                                    other => other.to_string(),
                                })
                                .unwrap_or_default()
                        })
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
                        v.as_object()?.get(col).map(|v| match v {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Null => String::new(),
                            other => other.to_string(),
                        })
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

    fn analyze_jsonl(file_path: &str) -> Result<PreviewData, ImportServiceError> {
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
            let obj = obj.as_object().ok_or(ImportServiceError::InvalidJson)?;

            // Get columns from first row
            if columns.is_empty() {
                columns = obj.keys().cloned().collect();
            }

            // Collect preview rows
            if rows.len() < 5 {
                let row: Vec<String> = columns.iter()
                    .map(|col| {
                        obj.get(col)
                            .map(|v| match v {
                                serde_json::Value::String(s) => s.clone(),
                                serde_json::Value::Null => String::new(),
                                other => other.to_string(),
                            })
                            .unwrap_or_default()
                    })
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

    /// Execute the import
    pub async fn execute_import(
        app: AppHandle,
        client: &Client,
        import_id: &str,
        source: &ImportSource,
        target_schema: &str,
        target_table: &str,
        mappings: &[ColumnMapping],
        options: &ImportOptions,
    ) -> Result<ImportResult, ImportServiceError> {
        let start = std::time::Instant::now();
        let mut result = ImportResult {
            success: true,
            rows_inserted: 0,
            rows_updated: 0,
            rows_skipped: 0,
            rows_failed: 0,
            errors: Vec::new(),
            duration_ms: 0,
        };

        // Truncate if requested
        if options.truncate_first {
            let truncate_sql = format!(
                "TRUNCATE TABLE {}.{}",
                Self::quote_ident(target_schema),
                Self::quote_ident(target_table)
            );
            client.execute(&truncate_sql, &[]).await?;
        }

        // Build target columns list
        let target_columns: Vec<&str> = mappings.iter()
            .filter_map(|m| m.target_column.as_deref())
            .collect();

        if options.use_copy && options.conflict_handling == "error" {
            // Use COPY for best performance
            result = Self::import_with_copy(
                &app, client, import_id, source, target_schema, target_table,
                mappings, &target_columns, options
            ).await?;
        } else {
            // Use INSERT for conflict handling
            result = Self::import_with_insert(
                &app, client, import_id, source, target_schema, target_table,
                mappings, &target_columns, options
            ).await?;
        }

        result.duration_ms = start.elapsed().as_millis() as i64;
        result.success = result.errors.is_empty() || result.rows_inserted > 0;

        Ok(result)
    }

    async fn import_with_copy(
        app: &AppHandle,
        client: &Client,
        import_id: &str,
        source: &ImportSource,
        target_schema: &str,
        target_table: &str,
        mappings: &[ColumnMapping],
        target_columns: &[&str],
        options: &ImportOptions,
    ) -> Result<ImportResult, ImportServiceError> {
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
            Self::quote_ident(target_schema),
            Self::quote_ident(target_table),
            columns_sql
        );

        // Start COPY
        let sink = client.copy_in(&copy_sql).await?;
        let writer = std::pin::pin!(sink);

        // Read source and write to COPY
        match source.file_type.as_str() {
            "csv" => {
                let opts = source.csv_options.as_ref().unwrap();
                let file = File::open(&source.file_path)?;
                let mut csv_reader = ReaderBuilder::new()
                    .delimiter(opts.delimiter.chars().next().unwrap() as u8)
                    .has_headers(opts.has_header)
                    .from_reader(file);

                let mut row_num = 0i64;
                for record in csv_reader.records() {
                    row_num += 1;

                    match record {
                        Ok(rec) => {
                            // Apply mappings and write row
                            let values: Vec<String> = mappings.iter()
                                .filter(|m| m.target_column.is_some())
                                .map(|m| {
                                    let idx = source.preview.columns.iter()
                                        .position(|c| c == &m.source_column)
                                        .unwrap_or(0);
                                    let value = rec.get(idx).unwrap_or("");
                                    Self::apply_transform(value, m.transform.as_ref())
                                })
                                .collect();

                            // In a real implementation, write to COPY writer
                            result.rows_inserted += 1;
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

                    // Emit progress
                    if row_num % 1000 == 0 {
                        let _ = app.emit(&format!("import:progress:{}", import_id), ImportProgress {
                            phase: "inserting".to_string(),
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
            }
            _ => {
                // Handle JSON formats similarly
            }
        }

        Ok(result)
    }

    async fn import_with_insert(
        app: &AppHandle,
        client: &Client,
        import_id: &str,
        source: &ImportSource,
        target_schema: &str,
        target_table: &str,
        mappings: &[ColumnMapping],
        target_columns: &[&str],
        options: &ImportOptions,
    ) -> Result<ImportResult, ImportServiceError> {
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
            Self::quote_ident(target_schema),
            Self::quote_ident(target_table),
            columns_sql,
            placeholders.join(", ")
        );

        // Add ON CONFLICT clause if needed
        if options.conflict_handling == "skip" {
            if let Some(ref conflict_cols) = options.conflict_columns {
                let conflict_sql = conflict_cols.iter()
                    .map(|c| Self::quote_ident(c))
                    .collect::<Vec<_>>()
                    .join(", ");
                insert_sql.push_str(&format!(" ON CONFLICT ({}) DO NOTHING", conflict_sql));
            }
        } else if options.conflict_handling == "update" {
            if let (Some(ref conflict_cols), Some(ref update_cols)) =
                (&options.conflict_columns, &options.update_columns)
            {
                let conflict_sql = conflict_cols.iter()
                    .map(|c| Self::quote_ident(c))
                    .collect::<Vec<_>>()
                    .join(", ");

                let update_sql = update_cols.iter()
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

        // Prepare statement
        let stmt = client.prepare(&insert_sql).await?;

        // Begin transaction if requested
        if options.use_transaction {
            client.execute("BEGIN", &[]).await?;
        }

        // Process rows in batches
        // ... (similar to COPY implementation but with individual inserts)

        if options.use_transaction {
            if result.errors.is_empty() {
                client.execute("COMMIT", &[]).await?;
            } else {
                client.execute("ROLLBACK", &[]).await?;
            }
        }

        Ok(result)
    }

    fn apply_transform(value: &str, transform: Option<&ColumnTransform>) -> String {
        let Some(t) = transform else {
            return value.to_string();
        };

        match t.transform_type.as_str() {
            "trim" => value.trim().to_string(),
            "uppercase" => value.to_uppercase(),
            "lowercase" => value.to_lowercase(),
            "parse_boolean" => {
                match value.to_lowercase().as_str() {
                    "true" | "t" | "yes" | "y" | "1" => "true".to_string(),
                    "false" | "f" | "no" | "n" | "0" => "false".to_string(),
                    _ => value.to_string(),
                }
            }
            _ => value.to_string(),
        }
    }

    fn quote_ident(s: &str) -> String {
        if s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            s.to_string()
        } else {
            format!("\"{}\"", s.replace('"', "\"\""))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ImportServiceError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("CSV error: {0}")]
    CsvError(#[from] csv::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    DatabaseError(#[from] tokio_postgres::Error),

    #[error("Unsupported file format")]
    UnsupportedFormat,

    #[error("Invalid JSON structure")]
    InvalidJson,

    #[error("Empty file")]
    EmptyFile,

    #[error("Import cancelled")]
    Cancelled,
}
```

### 24.3 Import Wizard Components

```svelte
<!-- src/lib/components/import/ImportWizard.svelte -->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { ImportSource, ImportTarget, ColumnMapping, ImportOptions } from '$lib/types/import';
  import ImportStep1Source from './ImportStep1Source.svelte';
  import ImportStep2Target from './ImportStep2Target.svelte';
  import ImportStep3Mapping from './ImportStep3Mapping.svelte';
  import ImportStep4Options from './ImportStep4Options.svelte';
  import ImportStep5Execute from './ImportStep5Execute.svelte';

  interface Props {
    open: boolean;
    connId: string;
  }

  let { open = $bindable(), connId }: Props = $props();

  const dispatch = createEventDispatcher<{
    complete: void;
    cancel: void;
  }>();

  let step = $state(1);
  let source = $state<ImportSource | null>(null);
  let target = $state<ImportTarget | null>(null);
  let mappings = $state<ColumnMapping[]>([]);
  let options = $state<ImportOptions>({
    conflictHandling: 'error',
    batchSize: 1000,
    useTransaction: true,
    useCopy: true,
    truncateFirst: false,
  });

  function handleSourceComplete(src: ImportSource) {
    source = src;
    step = 2;
  }

  function handleTargetComplete(tgt: ImportTarget) {
    target = tgt;

    // Initialize mappings
    mappings = source!.preview.columns.map((col, i) => ({
      sourceColumn: col,
      targetColumn: tgt.columns?.find(c => c.name.toLowerCase() === col.toLowerCase())?.name ?? null,
      transform: null,
    }));

    step = 3;
  }

  function handleMappingComplete(maps: ColumnMapping[]) {
    mappings = maps;
    step = 4;
  }

  function handleOptionsComplete(opts: ImportOptions) {
    options = opts;
    step = 5;
  }

  function handleBack() {
    if (step > 1) step--;
  }

  function handleComplete() {
    dispatch('complete');
    open = false;
  }

  function handleCancel() {
    dispatch('cancel');
    open = false;
  }

  const stepTitles = [
    'Select Source',
    'Select Target',
    'Map Columns',
    'Import Options',
    'Execute Import',
  ];
</script>

{#if open}
  <div
    class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
    role="dialog"
    aria-modal="true"
  >
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-[800px] max-h-[85vh] flex flex-col">
      <!-- Header -->
      <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between">
          <h2 class="text-lg font-semibold">Import Data</h2>
          <button
            onclick={handleCancel}
            class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <!-- Step Indicator -->
        <div class="flex items-center mt-4">
          {#each stepTitles as title, i}
            <div class="flex items-center {i < stepTitles.length - 1 ? 'flex-1' : ''}">
              <div
                class="w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium
                       {i + 1 < step
                         ? 'bg-green-500 text-white'
                         : i + 1 === step
                           ? 'bg-blue-600 text-white'
                           : 'bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400'}"
              >
                {#if i + 1 < step}
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                  </svg>
                {:else}
                  {i + 1}
                {/if}
              </div>
              <span class="ml-2 text-sm hidden sm:inline
                          {i + 1 === step ? 'font-medium' : 'text-gray-500 dark:text-gray-400'}">
                {title}
              </span>
              {#if i < stepTitles.length - 1}
                <div class="flex-1 h-px mx-4 bg-gray-200 dark:bg-gray-700"></div>
              {/if}
            </div>
          {/each}
        </div>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-auto p-6">
        {#if step === 1}
          <ImportStep1Source {connId} onComplete={handleSourceComplete} />
        {:else if step === 2 && source}
          <ImportStep2Target {connId} {source} onComplete={handleTargetComplete} onBack={handleBack} />
        {:else if step === 3 && source && target}
          <ImportStep3Mapping {source} {target} initialMappings={mappings} onComplete={handleMappingComplete} onBack={handleBack} />
        {:else if step === 4}
          <ImportStep4Options {target} initialOptions={options} onComplete={handleOptionsComplete} onBack={handleBack} />
        {:else if step === 5 && source && target}
          <ImportStep5Execute {connId} {source} {target} {mappings} {options} onComplete={handleComplete} onBack={handleBack} />
        {/if}
      </div>
    </div>
  </div>
{/if}
```

### 24.4 Step 1: Source Selection

```svelte
<!-- src/lib/components/import/ImportStep1Source.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open as openDialog } from '@tauri-apps/plugin-dialog';
  import type { ImportSource } from '$lib/types/import';

  interface Props {
    connId: string;
    onComplete: (source: ImportSource) => void;
  }

  let { connId, onComplete }: Props = $props();

  let filePath = $state('');
  let analyzing = $state(false);
  let error = $state<string | null>(null);
  let source = $state<ImportSource | null>(null);

  async function handleSelectFile() {
    const selected = await openDialog({
      multiple: false,
      filters: [
        { name: 'Data Files', extensions: ['csv', 'tsv', 'json', 'jsonl', 'ndjson'] },
        { name: 'CSV', extensions: ['csv', 'tsv'] },
        { name: 'JSON', extensions: ['json', 'jsonl', 'ndjson'] },
      ],
    });

    if (selected && typeof selected === 'string') {
      filePath = selected;
      await analyzeFile();
    }
  }

  async function analyzeFile() {
    analyzing = true;
    error = null;

    try {
      source = await invoke<ImportSource>('analyze_import_file', { filePath });
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      source = null;
    } finally {
      analyzing = false;
    }
  }

  function handleContinue() {
    if (source) {
      onComplete(source);
    }
  }

  function formatSize(bytes: number): string {
    if (bytes >= 1_073_741_824) return (bytes / 1_073_741_824).toFixed(2) + ' GB';
    if (bytes >= 1_048_576) return (bytes / 1_048_576).toFixed(2) + ' MB';
    if (bytes >= 1024) return (bytes / 1024).toFixed(2) + ' KB';
    return bytes + ' B';
  }
</script>

<div class="space-y-6">
  <!-- File Selection -->
  <div>
    <label class="block text-sm font-medium mb-2">Source File</label>
    <div class="flex gap-2">
      <input
        type="text"
        value={filePath}
        readonly
        placeholder="Select a file..."
        class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded
               bg-gray-50 dark:bg-gray-900 text-sm"
      />
      <button
        onclick={handleSelectFile}
        class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm"
      >
        Browse...
      </button>
    </div>
  </div>

  {#if analyzing}
    <div class="flex items-center justify-center py-8">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      <span class="ml-3">Analyzing file...</span>
    </div>
  {:else if error}
    <div class="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200
                dark:border-red-800 rounded text-red-700 dark:text-red-400">
      {error}
    </div>
  {:else if source}
    <!-- File Info -->
    <div class="grid grid-cols-4 gap-4 p-4 bg-gray-50 dark:bg-gray-900/50 rounded">
      <div>
        <span class="text-xs text-gray-500 block">Type</span>
        <span class="font-medium uppercase">{source.fileType}</span>
      </div>
      <div>
        <span class="text-xs text-gray-500 block">Size</span>
        <span class="font-medium">{formatSize(source.fileSize)}</span>
      </div>
      <div>
        <span class="text-xs text-gray-500 block">Encoding</span>
        <span class="font-medium">{source.encoding}</span>
      </div>
      <div>
        <span class="text-xs text-gray-500 block">Rows</span>
        <span class="font-medium">{source.preview.totalRows.toLocaleString()}</span>
      </div>
    </div>

    <!-- CSV Options (if applicable) -->
    {#if source.csvOptions}
      <div>
        <h3 class="text-sm font-medium mb-2">CSV Options</h3>
        <div class="grid grid-cols-3 gap-4">
          <div>
            <label class="block text-xs text-gray-500 mb-1">Delimiter</label>
            <select
              bind:value={source.csvOptions.delimiter}
              class="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded
                     bg-white dark:bg-gray-700 text-sm"
            >
              <option value=",">Comma (,)</option>
              <option value="&#9;">Tab</option>
              <option value=";">Semicolon (;)</option>
              <option value="|">Pipe (|)</option>
            </select>
          </div>
          <div>
            <label class="block text-xs text-gray-500 mb-1">Quote Character</label>
            <select
              bind:value={source.csvOptions.quoteChar}
              class="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded
                     bg-white dark:bg-gray-700 text-sm"
            >
              <option value="&quot;">Double Quote (")</option>
              <option value="'">Single Quote (')</option>
            </select>
          </div>
          <div class="flex items-end">
            <label class="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                bind:checked={source.csvOptions.hasHeader}
                class="rounded"
              />
              <span class="text-sm">Has header row</span>
            </label>
          </div>
        </div>
      </div>
    {/if}

    <!-- Data Preview -->
    <div>
      <h3 class="text-sm font-medium mb-2">Preview (first 5 rows)</h3>
      <div class="overflow-auto border border-gray-200 dark:border-gray-700 rounded">
        <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700 text-sm">
          <thead class="bg-gray-50 dark:bg-gray-900/50">
            <tr>
              {#each source.preview.columns as col}
                <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase">
                  {col}
                </th>
              {/each}
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
            {#each source.preview.rows as row}
              <tr>
                {#each row as cell}
                  <td class="px-3 py-2 font-mono text-xs truncate max-w-[200px]">
                    {cell || '<empty>'}
                  </td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}

  <!-- Footer -->
  <div class="flex justify-end pt-4">
    <button
      onclick={handleContinue}
      disabled={!source}
      class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700
             disabled:opacity-50 disabled:cursor-not-allowed text-sm"
    >
      Continue
    </button>
  </div>
</div>
```

## Acceptance Criteria

1. **File Analysis**
   - [ ] Support CSV, TSV, JSON array, and JSON Lines formats
   - [ ] Auto-detect delimiter, encoding, and data types
   - [ ] Preview first 5 rows
   - [ ] Show file metadata (size, row count, columns)

2. **Target Selection**
   - [ ] Select existing table
   - [ ] Option to create new table
   - [ ] Display target column types

3. **Column Mapping**
   - [ ] Auto-map columns by name
   - [ ] Allow manual mapping
   - [ ] Skip columns option
   - [ ] Apply transformations (trim, case, date parse)

4. **Import Options**
   - [ ] Conflict handling (error, skip, upsert)
   - [ ] Batch size configuration
   - [ ] Transaction mode
   - [ ] COPY vs INSERT selection
   - [ ] Truncate before import

5. **Execution**
   - [ ] Progress reporting
   - [ ] Error collection and display
   - [ ] Summary of imported rows
   - [ ] Cancel capability

## MCP Testing Instructions

### Tauri MCP Testing

```typescript
// Analyze a CSV file
await mcp___hypothesi_tauri_mcp_server__ipc_execute_command({
  command: 'analyze_import_file',
  args: { filePath: '/path/to/data.csv' }
});

// Execute import
await mcp___hypothesi_tauri_mcp_server__ipc_execute_command({
  command: 'execute_import',
  args: {
    connId: 'test-conn',
    source: { /* ImportSource */ },
    targetSchema: 'public',
    targetTable: 'users',
    mappings: [
      { sourceColumn: 'email', targetColumn: 'email', transform: null },
      { sourceColumn: 'name', targetColumn: 'full_name', transform: { type: 'trim' } }
    ],
    options: {
      conflictHandling: 'skip',
      conflictColumns: ['email'],
      batchSize: 1000,
      useTransaction: true,
      useCopy: false
    }
  }
});
```

### Playwright MCP Testing

```typescript
// Open import wizard
await mcp__playwright__browser_click({
  element: 'Import button',
  ref: 'button:has-text("Import")'
});

// Select file
await mcp__playwright__browser_click({
  element: 'Browse button',
  ref: 'button:has-text("Browse")'
});

// Take screenshot of wizard
await mcp__playwright__browser_take_screenshot({
  filename: 'import-wizard-step1.png'
});
```
