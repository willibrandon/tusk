# Feature 15: Export and Copy Functionality

## Overview

Export and copy functionality enables users to extract query results in multiple formats for use in other applications. This includes clipboard operations (copy as TSV, CSV, JSON, INSERT statements) and file exports (CSV, JSON, SQL, Excel, Markdown). Built entirely in Rust with GPUI for the export dialog.

## Goals

- Copy selected cells or entire result sets to clipboard
- Export results to various file formats
- Support format-specific options (delimiters, headers, etc.)
- Generate SQL statements from data (INSERT, UPDATE, COPY)
- Handle large exports efficiently with streaming
- Native cross-platform file dialogs

## Dependencies

- Feature 14: Results Grid (data selection)
- Feature 11: Query Execution (column metadata)
- Feature 03: Frontend Architecture (GPUI components)

## Technical Specification

### 15.1 Export Models

```rust
// src/export/models.rs

use serde::{Deserialize, Serialize};

/// Supported export formats
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Csv,
    Tsv,
    Json,
    JsonLines,
    Sql,
    SqlCopy,
    Markdown,
    Excel,
}

impl ExportFormat {
    pub fn file_extension(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Tsv => "tsv",
            ExportFormat::Json => "json",
            ExportFormat::JsonLines => "jsonl",
            ExportFormat::Sql => "sql",
            ExportFormat::SqlCopy => "sql",
            ExportFormat::Markdown => "md",
            ExportFormat::Excel => "xlsx",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            ExportFormat::Csv | ExportFormat::Tsv => "text/csv",
            ExportFormat::Json | ExportFormat::JsonLines => "application/json",
            ExportFormat::Sql | ExportFormat::SqlCopy => "application/sql",
            ExportFormat::Markdown => "text/markdown",
            ExportFormat::Excel => {
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            }
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "CSV",
            ExportFormat::Tsv => "TSV",
            ExportFormat::Json => "JSON",
            ExportFormat::JsonLines => "JSON Lines",
            ExportFormat::Sql => "SQL INSERT",
            ExportFormat::SqlCopy => "SQL COPY",
            ExportFormat::Markdown => "Markdown",
            ExportFormat::Excel => "Excel",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "Comma-separated values",
            ExportFormat::Tsv => "Tab-separated values",
            ExportFormat::Json => "JSON array of objects",
            ExportFormat::JsonLines => "Newline-delimited JSON",
            ExportFormat::Sql => "INSERT statements",
            ExportFormat::SqlCopy => "PostgreSQL COPY format",
            ExportFormat::Markdown => "Markdown table",
            ExportFormat::Excel => "XLSX spreadsheet",
        }
    }

    pub fn all() -> Vec<ExportFormat> {
        vec![
            ExportFormat::Csv,
            ExportFormat::Tsv,
            ExportFormat::Json,
            ExportFormat::JsonLines,
            ExportFormat::Sql,
            ExportFormat::SqlCopy,
            ExportFormat::Markdown,
            ExportFormat::Excel,
        ]
    }
}

/// Export configuration options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub include_headers: bool,
    pub null_string: String,
    pub csv_options: Option<CsvOptions>,
    pub json_options: Option<JsonOptions>,
    pub sql_options: Option<SqlOptions>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Csv,
            include_headers: true,
            null_string: String::new(),
            csv_options: Some(CsvOptions::default()),
            json_options: None,
            sql_options: None,
        }
    }
}

impl ExportOptions {
    pub fn for_format(format: ExportFormat) -> Self {
        match format {
            ExportFormat::Csv => Self {
                format,
                include_headers: true,
                null_string: String::new(),
                csv_options: Some(CsvOptions::default()),
                json_options: None,
                sql_options: None,
            },
            ExportFormat::Tsv => Self {
                format,
                include_headers: true,
                null_string: String::new(),
                csv_options: Some(CsvOptions {
                    delimiter: '\t',
                    ..Default::default()
                }),
                json_options: None,
                sql_options: None,
            },
            ExportFormat::Json => Self {
                format,
                include_headers: false,
                null_string: String::new(),
                csv_options: None,
                json_options: Some(JsonOptions::default()),
                sql_options: None,
            },
            ExportFormat::JsonLines => Self {
                format,
                include_headers: false,
                null_string: String::new(),
                csv_options: None,
                json_options: Some(JsonOptions {
                    pretty_print: false,
                    ..Default::default()
                }),
                sql_options: None,
            },
            ExportFormat::Sql | ExportFormat::SqlCopy => Self {
                format: format.clone(),
                include_headers: false,
                null_string: if format == ExportFormat::SqlCopy {
                    "\\N".to_string()
                } else {
                    String::new()
                },
                csv_options: None,
                json_options: None,
                sql_options: Some(SqlOptions::default()),
            },
            ExportFormat::Markdown => Self {
                format,
                include_headers: true,
                null_string: String::new(),
                csv_options: None,
                json_options: None,
                sql_options: None,
            },
            ExportFormat::Excel => Self {
                format,
                include_headers: true,
                null_string: String::new(),
                csv_options: None,
                json_options: None,
                sql_options: None,
            },
        }
    }
}

/// CSV-specific options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CsvOptions {
    pub delimiter: char,
    pub quote_char: char,
    pub escape_char: Option<char>,
    pub line_terminator: String,
}

impl Default for CsvOptions {
    fn default() -> Self {
        Self {
            delimiter: ',',
            quote_char: '"',
            escape_char: None,
            line_terminator: "\n".to_string(),
        }
    }
}

/// JSON-specific options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonOptions {
    /// true = array of objects, false = object with columns and rows arrays
    pub array_format: bool,
    pub pretty_print: bool,
}

impl Default for JsonOptions {
    fn default() -> Self {
        Self {
            array_format: true,
            pretty_print: true,
        }
    }
}

/// SQL-specific options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SqlOptions {
    pub table_name: String,
    pub schema_name: Option<String>,
    pub batch_size: usize,
    pub include_column_names: bool,
    pub on_conflict: Option<OnConflictOption>,
}

impl Default for SqlOptions {
    fn default() -> Self {
        Self {
            table_name: "table_name".to_string(),
            schema_name: None,
            batch_size: 1000,
            include_column_names: true,
            on_conflict: None,
        }
    }
}

/// ON CONFLICT clause options for SQL INSERT
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OnConflictOption {
    DoNothing,
    DoUpdate { columns: Vec<String> },
}

/// Export progress for large exports
#[derive(Clone, Debug)]
pub struct ExportProgress {
    pub total_rows: usize,
    pub exported_rows: usize,
    pub bytes_written: u64,
    pub is_complete: bool,
    pub error: Option<String>,
}

impl ExportProgress {
    pub fn new(total_rows: usize) -> Self {
        Self {
            total_rows,
            exported_rows: 0,
            bytes_written: 0,
            is_complete: false,
            error: None,
        }
    }

    pub fn percentage(&self) -> f32 {
        if self.total_rows == 0 {
            100.0
        } else {
            (self.exported_rows as f32 / self.total_rows as f32) * 100.0
        }
    }
}

/// Data to export - references grid data
#[derive(Clone, Debug)]
pub struct ExportData {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Vec<crate::query::Value>>,
}

/// Column information for export
#[derive(Clone, Debug)]
pub struct ColumnInfo {
    pub name: String,
    pub type_name: String,
    pub type_oid: u32,
}
```

### 15.2 Export Service

```rust
// src/export/service.rs

use std::io::Write;
use std::path::PathBuf;
use std::fs::File;
use std::sync::Arc;
use parking_lot::RwLock;
use csv::WriterBuilder;
use tokio::runtime::Handle;

use crate::error::{Error, Result};
use crate::query::Value;
use super::models::*;

/// Export service handles all export and copy operations
pub struct ExportService {
    runtime: Handle,
    progress: Arc<RwLock<Option<ExportProgress>>>,
}

impl ExportService {
    pub fn new(runtime: Handle) -> Self {
        Self {
            runtime,
            progress: Arc::new(RwLock::new(None)),
        }
    }

    /// Get current export progress
    pub fn progress(&self) -> Option<ExportProgress> {
        self.progress.read().clone()
    }

    /// Export data to file
    pub fn export_to_file(
        &self,
        data: &ExportData,
        path: PathBuf,
        options: ExportOptions,
    ) -> Result<u64> {
        // Initialize progress
        {
            let mut progress = self.progress.write();
            *progress = Some(ExportProgress::new(data.rows.len()));
        }

        let file = File::create(&path)?;
        let result = self.export_to_writer(data, file, options);

        // Mark complete
        {
            let mut progress = self.progress.write();
            if let Some(ref mut p) = *progress {
                p.is_complete = true;
                if let Err(ref e) = result {
                    p.error = Some(e.to_string());
                }
            }
        }

        result
    }

    /// Export data to a writer (file, buffer, etc.)
    pub fn export_to_writer<W: Write>(
        &self,
        data: &ExportData,
        writer: W,
        options: ExportOptions,
    ) -> Result<u64> {
        match options.format {
            ExportFormat::Csv | ExportFormat::Tsv => {
                self.export_csv(data, writer, &options)
            }
            ExportFormat::Json => {
                self.export_json(data, writer, &options)
            }
            ExportFormat::JsonLines => {
                self.export_json_lines(data, writer, &options)
            }
            ExportFormat::Sql => {
                self.export_sql_insert(data, writer, &options)
            }
            ExportFormat::SqlCopy => {
                self.export_sql_copy(data, writer, &options)
            }
            ExportFormat::Markdown => {
                self.export_markdown(data, writer, &options)
            }
            ExportFormat::Excel => {
                self.export_excel(data, writer, &options)
            }
        }
    }

    /// Export to string (for clipboard operations)
    pub fn export_to_string(
        &self,
        data: &ExportData,
        options: ExportOptions,
    ) -> Result<String> {
        let mut buffer = Vec::new();
        self.export_to_writer(data, &mut buffer, options)?;
        Ok(String::from_utf8(buffer)?)
    }

    /// Copy data to clipboard in specified format
    pub fn copy_to_clipboard(
        &self,
        data: &ExportData,
        options: ExportOptions,
    ) -> Result<()> {
        let text = self.export_to_string(data, options)?;

        // Use arboard for cross-platform clipboard access
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| Error::Export(format!("Failed to access clipboard: {}", e)))?;

        clipboard.set_text(&text)
            .map_err(|e| Error::Export(format!("Failed to copy to clipboard: {}", e)))?;

        Ok(())
    }

    /// Quick copy as TSV (default for cells)
    pub fn copy_as_tsv(&self, data: &ExportData, include_headers: bool) -> Result<()> {
        let options = ExportOptions {
            format: ExportFormat::Tsv,
            include_headers,
            null_string: String::new(),
            csv_options: Some(CsvOptions {
                delimiter: '\t',
                ..Default::default()
            }),
            json_options: None,
            sql_options: None,
        };
        self.copy_to_clipboard(data, options)
    }

    /// Quick copy as CSV
    pub fn copy_as_csv(&self, data: &ExportData, include_headers: bool) -> Result<()> {
        let options = ExportOptions {
            format: ExportFormat::Csv,
            include_headers,
            ..Default::default()
        };
        self.copy_to_clipboard(data, options)
    }

    /// Quick copy as JSON
    pub fn copy_as_json(&self, data: &ExportData) -> Result<()> {
        let options = ExportOptions::for_format(ExportFormat::Json);
        self.copy_to_clipboard(data, options)
    }

    /// Copy as SQL INSERT statements
    pub fn copy_as_insert(
        &self,
        data: &ExportData,
        table_name: &str,
        schema_name: Option<&str>,
    ) -> Result<()> {
        let options = ExportOptions {
            format: ExportFormat::Sql,
            include_headers: false,
            null_string: String::new(),
            csv_options: None,
            json_options: None,
            sql_options: Some(SqlOptions {
                table_name: table_name.to_string(),
                schema_name: schema_name.map(|s| s.to_string()),
                batch_size: 1000,
                include_column_names: true,
                on_conflict: None,
            }),
        };
        self.copy_to_clipboard(data, options)
    }

    /// Generate UPDATE statement for a single row
    pub fn copy_as_update(
        &self,
        data: &ExportData,
        row_index: usize,
        table_name: &str,
        primary_key_column: &str,
        schema_name: Option<&str>,
    ) -> Result<()> {
        if row_index >= data.rows.len() {
            return Err(Error::Export("Row index out of bounds".to_string()));
        }

        let pk_index = data.columns.iter()
            .position(|c| c.name == primary_key_column)
            .ok_or_else(|| Error::Export("Primary key column not found".to_string()))?;

        let row = &data.rows[row_index];
        let pk_value = self.format_value_for_sql(&row[pk_index], &data.columns[pk_index].type_name);

        let set_clause: Vec<String> = data.columns.iter()
            .enumerate()
            .filter(|(i, _)| *i != pk_index)
            .map(|(i, col)| {
                let value = self.format_value_for_sql(&row[i], &col.type_name);
                format!("\"{}\" = {}", col.name, value)
            })
            .collect();

        let table_ref = if let Some(schema) = schema_name {
            format!("\"{}\".\"{}\"", schema, table_name)
        } else {
            format!("\"{}\"", table_name)
        };

        let sql = format!(
            "UPDATE {}\nSET {}\nWHERE \"{}\" = {};",
            table_ref,
            set_clause.join(", "),
            primary_key_column,
            pk_value
        );

        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| Error::Export(format!("Failed to access clipboard: {}", e)))?;

        clipboard.set_text(&sql)
            .map_err(|e| Error::Export(format!("Failed to copy to clipboard: {}", e)))?;

        Ok(())
    }

    // ========== Format-specific exporters ==========

    fn export_csv<W: Write>(
        &self,
        data: &ExportData,
        writer: W,
        options: &ExportOptions,
    ) -> Result<u64> {
        let csv_opts = options.csv_options.as_ref()
            .cloned()
            .unwrap_or_default();

        let delimiter = if matches!(options.format, ExportFormat::Tsv) {
            b'\t'
        } else {
            csv_opts.delimiter as u8
        };

        let mut csv_writer = WriterBuilder::new()
            .delimiter(delimiter)
            .quote(csv_opts.quote_char as u8)
            .terminator(csv::Terminator::Any(
                csv_opts.line_terminator.as_bytes()[0]
            ))
            .from_writer(writer);

        // Write header
        if options.include_headers {
            let headers: Vec<&str> = data.columns.iter()
                .map(|c| c.name.as_str())
                .collect();
            csv_writer.write_record(&headers)?;
        }

        // Write rows with progress updates
        let progress = self.progress.clone();
        for (i, row) in data.rows.iter().enumerate() {
            let record: Vec<String> = row.iter()
                .map(|v| self.format_value_for_csv(v, &options.null_string))
                .collect();
            csv_writer.write_record(&record)?;

            // Update progress every 1000 rows
            if i % 1000 == 0 {
                if let Some(ref mut p) = *progress.write() {
                    p.exported_rows = i + 1;
                }
            }
        }

        csv_writer.flush()?;

        // Final progress update
        if let Some(ref mut p) = *progress.write() {
            p.exported_rows = data.rows.len();
        }

        Ok(0)
    }

    fn export_json<W: Write>(
        &self,
        data: &ExportData,
        mut writer: W,
        options: &ExportOptions,
    ) -> Result<u64> {
        let json_opts = options.json_options.as_ref()
            .cloned()
            .unwrap_or_default();

        if json_opts.array_format {
            // Array of objects: [{"col1": val1, "col2": val2}, ...]
            let objects: Vec<serde_json::Map<String, serde_json::Value>> = data.rows
                .iter()
                .map(|row| {
                    let mut obj = serde_json::Map::new();
                    for (i, col) in data.columns.iter().enumerate() {
                        let value = self.value_to_json(&row[i]);
                        obj.insert(col.name.clone(), value);
                    }
                    obj
                })
                .collect();

            let json_str = if json_opts.pretty_print {
                serde_json::to_string_pretty(&objects)?
            } else {
                serde_json::to_string(&objects)?
            };

            writer.write_all(json_str.as_bytes())?;
        } else {
            // Object with columns and rows: {"columns": [...], "rows": [[...], ...]}
            let arrays: Vec<Vec<serde_json::Value>> = data.rows
                .iter()
                .map(|row| row.iter().map(|v| self.value_to_json(v)).collect())
                .collect();

            let output = serde_json::json!({
                "columns": data.columns.iter().map(|c| &c.name).collect::<Vec<_>>(),
                "rows": arrays
            });

            let json_str = if json_opts.pretty_print {
                serde_json::to_string_pretty(&output)?
            } else {
                serde_json::to_string(&output)?
            };

            writer.write_all(json_str.as_bytes())?;
        }

        Ok(0)
    }

    fn export_json_lines<W: Write>(
        &self,
        data: &ExportData,
        mut writer: W,
        _options: &ExportOptions,
    ) -> Result<u64> {
        for row in &data.rows {
            let mut obj = serde_json::Map::new();
            for (i, col) in data.columns.iter().enumerate() {
                let value = self.value_to_json(&row[i]);
                obj.insert(col.name.clone(), value);
            }

            let line = serde_json::to_string(&obj)?;
            writeln!(writer, "{}", line)?;
        }

        Ok(0)
    }

    fn export_sql_insert<W: Write>(
        &self,
        data: &ExportData,
        mut writer: W,
        options: &ExportOptions,
    ) -> Result<u64> {
        let sql_opts = options.sql_options.as_ref()
            .cloned()
            .unwrap_or_default();

        let table_name = if let Some(schema) = &sql_opts.schema_name {
            format!("\"{}\".\"{}\"", schema, sql_opts.table_name)
        } else {
            format!("\"{}\"", sql_opts.table_name)
        };

        let column_names: Vec<String> = data.columns.iter()
            .map(|c| format!("\"{}\"", c.name))
            .collect();
        let columns_clause = column_names.join(", ");

        // Write in batches
        for chunk in data.rows.chunks(sql_opts.batch_size) {
            write!(writer, "INSERT INTO {} ({})\nVALUES\n", table_name, columns_clause)?;

            for (i, row) in chunk.iter().enumerate() {
                let values: Vec<String> = row.iter()
                    .zip(data.columns.iter())
                    .map(|(v, c)| self.format_value_for_sql(v, &c.type_name))
                    .collect();

                if i > 0 {
                    write!(writer, ",\n")?;
                }
                write!(writer, "  ({})", values.join(", "))?;
            }

            // ON CONFLICT clause
            if let Some(on_conflict) = &sql_opts.on_conflict {
                match on_conflict {
                    OnConflictOption::DoNothing => {
                        write!(writer, "\nON CONFLICT DO NOTHING")?;
                    }
                    OnConflictOption::DoUpdate { columns: update_cols } => {
                        let updates: Vec<String> = update_cols.iter()
                            .map(|c| format!("\"{}\" = EXCLUDED.\"{}\"", c, c))
                            .collect();
                        write!(writer, "\nON CONFLICT DO UPDATE SET {}", updates.join(", "))?;
                    }
                }
            }

            writeln!(writer, ";")?;
        }

        Ok(0)
    }

    fn export_sql_copy<W: Write>(
        &self,
        data: &ExportData,
        mut writer: W,
        options: &ExportOptions,
    ) -> Result<u64> {
        let sql_opts = options.sql_options.as_ref()
            .cloned()
            .unwrap_or_default();

        let table_name = if let Some(schema) = &sql_opts.schema_name {
            format!("\"{}\".\"{}\"", schema, sql_opts.table_name)
        } else {
            format!("\"{}\"", sql_opts.table_name)
        };

        let column_names: Vec<String> = data.columns.iter()
            .map(|c| format!("\"{}\"", c.name))
            .collect();

        writeln!(writer, "COPY {} ({}) FROM stdin;", table_name, column_names.join(", "))?;

        for row in &data.rows {
            let values: Vec<String> = row.iter()
                .map(|v| self.format_value_for_copy(v, &options.null_string))
                .collect();
            writeln!(writer, "{}", values.join("\t"))?;
        }

        writeln!(writer, "\\.")?;

        Ok(0)
    }

    fn export_markdown<W: Write>(
        &self,
        data: &ExportData,
        mut writer: W,
        options: &ExportOptions,
    ) -> Result<u64> {
        // Header row
        let headers: Vec<&str> = data.columns.iter()
            .map(|c| c.name.as_str())
            .collect();
        writeln!(writer, "| {} |", headers.join(" | "))?;

        // Separator row with alignment
        let separators: Vec<String> = data.columns.iter()
            .map(|c| {
                match c.type_name.as_str() {
                    "int2" | "int4" | "int8" | "float4" | "float8" | "numeric" | "money" => {
                        "---:".to_string()
                    }
                    _ => "---".to_string(),
                }
            })
            .collect();
        writeln!(writer, "| {} |", separators.join(" | "))?;

        // Data rows
        for row in &data.rows {
            let values: Vec<String> = row.iter()
                .map(|v| self.format_value_for_markdown(v, &options.null_string))
                .collect();
            writeln!(writer, "| {} |", values.join(" | "))?;
        }

        Ok(0)
    }

    fn export_excel<W: Write>(
        &self,
        data: &ExportData,
        mut writer: W,
        _options: &ExportOptions,
    ) -> Result<u64> {
        use rust_xlsxwriter::{Workbook, Format};

        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        let header_format = Format::new().set_bold();

        // Write headers
        for (col, column) in data.columns.iter().enumerate() {
            worksheet.write_with_format(0, col as u16, &column.name, &header_format)?;
        }

        // Write data
        for (row_idx, row) in data.rows.iter().enumerate() {
            for (col_idx, value) in row.iter().enumerate() {
                self.write_excel_value(worksheet, row_idx + 1, col_idx, value)?;
            }
        }

        // Auto-fit columns (approximate width)
        for (col, _) in data.columns.iter().enumerate() {
            worksheet.set_column_width(col as u16, 15)?;
        }

        let buffer = workbook.save_to_buffer()?;
        writer.write_all(&buffer)?;

        Ok(buffer.len() as u64)
    }

    fn write_excel_value(
        &self,
        worksheet: &mut rust_xlsxwriter::Worksheet,
        row: usize,
        col: usize,
        value: &Value,
    ) -> Result<()> {
        match value {
            Value::Null => {
                worksheet.write_string(row as u32, col as u16, "")?;
            }
            Value::Bool(b) => {
                worksheet.write_boolean(row as u32, col as u16, *b)?;
            }
            Value::Int16(n) => {
                worksheet.write_number(row as u32, col as u16, *n as f64)?;
            }
            Value::Int32(n) => {
                worksheet.write_number(row as u32, col as u16, *n as f64)?;
            }
            Value::Int64(n) => {
                worksheet.write_number(row as u32, col as u16, *n as f64)?;
            }
            Value::Float32(f) => {
                worksheet.write_number(row as u32, col as u16, *f as f64)?;
            }
            Value::Float64(f) => {
                worksheet.write_number(row as u32, col as u16, *f)?;
            }
            Value::Numeric(s) => {
                if let Ok(n) = s.parse::<f64>() {
                    worksheet.write_number(row as u32, col as u16, n)?;
                } else {
                    worksheet.write_string(row as u32, col as u16, s)?;
                }
            }
            Value::String(s) | Value::Text(s) => {
                worksheet.write_string(row as u32, col as u16, s)?;
            }
            Value::Json(j) | Value::Jsonb(j) => {
                worksheet.write_string(row as u32, col as u16, &j.to_string())?;
            }
            Value::Timestamp(ts) | Value::TimestampTz(ts) => {
                worksheet.write_string(row as u32, col as u16, ts)?;
            }
            Value::Date(d) => {
                worksheet.write_string(row as u32, col as u16, d)?;
            }
            Value::Time(t) | Value::TimeTz(t) => {
                worksheet.write_string(row as u32, col as u16, t)?;
            }
            Value::Uuid(u) => {
                worksheet.write_string(row as u32, col as u16, u)?;
            }
            Value::Bytea(hex) => {
                worksheet.write_string(row as u32, col as u16, &format!("\\x{}", hex))?;
            }
            Value::Array(arr) => {
                let formatted: Vec<String> = arr.iter()
                    .map(|v| self.format_value_for_csv(v, ""))
                    .collect();
                worksheet.write_string(row as u32, col as u16, &format!("{{{}}}", formatted.join(",")))?;
            }
            Value::Interval(iso) => {
                worksheet.write_string(row as u32, col as u16, iso)?;
            }
            Value::Point { x, y } => {
                worksheet.write_string(row as u32, col as u16, &format!("({},{})", x, y))?;
            }
            Value::Inet(addr) | Value::Cidr(addr) | Value::MacAddr(addr) => {
                worksheet.write_string(row as u32, col as u16, addr)?;
            }
            Value::Unknown(text) => {
                worksheet.write_string(row as u32, col as u16, text)?;
            }
        }
        Ok(())
    }

    // ========== Value formatting helpers ==========

    fn format_value_for_csv(&self, value: &Value, null_string: &str) -> String {
        match value {
            Value::Null => null_string.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int16(n) => n.to_string(),
            Value::Int32(n) => n.to_string(),
            Value::Int64(n) => n.to_string(),
            Value::Float32(f) => f.to_string(),
            Value::Float64(f) => f.to_string(),
            Value::Numeric(s) => s.clone(),
            Value::String(s) | Value::Text(s) => s.clone(),
            Value::Json(j) | Value::Jsonb(j) => j.to_string(),
            Value::Timestamp(ts) | Value::TimestampTz(ts) => ts.clone(),
            Value::Date(d) => d.clone(),
            Value::Time(t) | Value::TimeTz(t) => t.clone(),
            Value::Uuid(u) => u.clone(),
            Value::Bytea(hex) => format!("\\x{}", hex),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter()
                    .map(|v| self.format_value_for_csv(v, null_string))
                    .collect();
                format!("{{{}}}", items.join(","))
            }
            Value::Interval(iso) => iso.clone(),
            Value::Point { x, y } => format!("({},{})", x, y),
            Value::Inet(addr) | Value::Cidr(addr) | Value::MacAddr(addr) => addr.clone(),
            Value::Unknown(text) => text.clone(),
        }
    }

    fn format_value_for_sql(&self, value: &Value, _type_name: &str) -> String {
        match value {
            Value::Null => "NULL".to_string(),
            Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            Value::Int16(n) => n.to_string(),
            Value::Int32(n) => n.to_string(),
            Value::Int64(n) => n.to_string(),
            Value::Float32(f) => f.to_string(),
            Value::Float64(f) => f.to_string(),
            Value::Numeric(s) => s.clone(),
            Value::String(s) | Value::Text(s) => format!("'{}'", s.replace('\'', "''")),
            Value::Json(j) | Value::Jsonb(j) => {
                format!("'{}'", j.to_string().replace('\'', "''"))
            }
            Value::Timestamp(ts) => format!("'{}'::timestamp", ts),
            Value::TimestampTz(ts) => format!("'{}'::timestamptz", ts),
            Value::Date(d) => format!("'{}'::date", d),
            Value::Time(t) => format!("'{}'::time", t),
            Value::TimeTz(t) => format!("'{}'::timetz", t),
            Value::Uuid(u) => format!("'{}'::uuid", u),
            Value::Bytea(hex) => format!("'\\x{}'::bytea", hex),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter()
                    .map(|v| self.format_value_for_sql(v, ""))
                    .collect();
                format!("ARRAY[{}]", items.join(", "))
            }
            Value::Interval(iso) => format!("'{}'::interval", iso),
            Value::Point { x, y } => format!("point({}, {})", x, y),
            Value::Inet(addr) => format!("'{}'::inet", addr),
            Value::Cidr(addr) => format!("'{}'::cidr", addr),
            Value::MacAddr(addr) => format!("'{}'::macaddr", addr),
            Value::Unknown(text) => format!("'{}'", text.replace('\'', "''")),
        }
    }

    fn format_value_for_copy(&self, value: &Value, null_string: &str) -> String {
        match value {
            Value::Null => null_string.to_string(),
            Value::String(s) | Value::Text(s) => s
                .replace('\\', "\\\\")
                .replace('\t', "\\t")
                .replace('\n', "\\n")
                .replace('\r', "\\r"),
            _ => self.format_value_for_csv(value, null_string),
        }
    }

    fn format_value_for_markdown(&self, value: &Value, null_string: &str) -> String {
        match value {
            Value::Null => format!("*{}*", if null_string.is_empty() { "NULL" } else { null_string }),
            Value::String(s) | Value::Text(s) => s.replace('|', "\\|"),
            _ => self.format_value_for_csv(value, null_string).replace('|', "\\|"),
        }
    }

    fn value_to_json(&self, value: &Value) -> serde_json::Value {
        match value {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Int16(n) => serde_json::json!(n),
            Value::Int32(n) => serde_json::json!(n),
            Value::Int64(n) => serde_json::json!(n),
            Value::Float32(f) => serde_json::json!(f),
            Value::Float64(f) => serde_json::json!(f),
            Value::Numeric(s) => {
                if let Ok(n) = s.parse::<f64>() {
                    serde_json::json!(n)
                } else {
                    serde_json::Value::String(s.clone())
                }
            }
            Value::String(s) | Value::Text(s) => serde_json::Value::String(s.clone()),
            Value::Json(j) | Value::Jsonb(j) => j.clone(),
            Value::Timestamp(ts) | Value::TimestampTz(ts) => serde_json::Value::String(ts.clone()),
            Value::Date(d) => serde_json::Value::String(d.clone()),
            Value::Time(t) | Value::TimeTz(t) => serde_json::Value::String(t.clone()),
            Value::Uuid(u) => serde_json::Value::String(u.clone()),
            Value::Bytea(hex) => serde_json::json!({ "type": "bytea", "hex": hex }),
            Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| self.value_to_json(v)).collect())
            }
            Value::Interval(iso) => serde_json::json!({ "type": "interval", "value": iso }),
            Value::Point { x, y } => serde_json::json!({ "type": "point", "x": x, "y": y }),
            Value::Inet(addr) => serde_json::Value::String(addr.clone()),
            Value::Cidr(addr) => serde_json::Value::String(addr.clone()),
            Value::MacAddr(addr) => serde_json::Value::String(addr.clone()),
            Value::Unknown(text) => serde_json::Value::String(text.clone()),
        }
    }
}
```

### 15.3 File Dialog Integration

```rust
// src/export/dialog.rs

use std::path::PathBuf;
use rfd::{FileDialog, AsyncFileDialog};

use super::models::ExportFormat;
use crate::error::{Error, Result};

/// Show native save file dialog
pub fn show_save_dialog(
    default_name: &str,
    format: ExportFormat,
) -> Option<PathBuf> {
    let (filter_name, extensions) = get_file_filter(format);

    FileDialog::new()
        .set_file_name(default_name)
        .add_filter(filter_name, &extensions)
        .save_file()
}

/// Show native save file dialog (async version)
pub async fn show_save_dialog_async(
    default_name: &str,
    format: ExportFormat,
) -> Option<PathBuf> {
    let (filter_name, extensions) = get_file_filter(format);

    let handle = AsyncFileDialog::new()
        .set_file_name(default_name)
        .add_filter(filter_name, &extensions)
        .save_file()
        .await?;

    Some(handle.path().to_path_buf())
}

fn get_file_filter(format: ExportFormat) -> (&'static str, Vec<&'static str>) {
    match format {
        ExportFormat::Csv => ("CSV Files", vec!["csv"]),
        ExportFormat::Tsv => ("TSV Files", vec!["tsv", "txt"]),
        ExportFormat::Json => ("JSON Files", vec!["json"]),
        ExportFormat::JsonLines => ("JSON Lines Files", vec!["jsonl", "json"]),
        ExportFormat::Sql | ExportFormat::SqlCopy => ("SQL Files", vec!["sql"]),
        ExportFormat::Markdown => ("Markdown Files", vec!["md", "markdown"]),
        ExportFormat::Excel => ("Excel Files", vec!["xlsx"]),
    }
}

/// Quick export to file with dialog
pub fn quick_export(
    export_service: &super::service::ExportService,
    data: &super::models::ExportData,
    format: ExportFormat,
) -> Result<bool> {
    let default_name = format!("export.{}", format.file_extension());

    let path = match show_save_dialog(&default_name, format.clone()) {
        Some(p) => p,
        None => return Ok(false), // User cancelled
    };

    let options = super::models::ExportOptions::for_format(format);
    export_service.export_to_file(data, path, options)?;

    Ok(true)
}
```

### 15.4 Export State (Global)

```rust
// src/export/state.rs

use gpui::Global;
use std::sync::Arc;
use parking_lot::RwLock;

use super::models::*;
use super::service::ExportService;

/// Global export state
pub struct ExportState {
    pub service: Arc<ExportService>,
    pub current_export: RwLock<Option<CurrentExport>>,
    pub last_options: RwLock<ExportOptions>,
}

impl Global for ExportState {}

/// Current export operation state
#[derive(Clone)]
pub struct CurrentExport {
    pub data: ExportData,
    pub options: ExportOptions,
    pub progress: ExportProgress,
}

impl ExportState {
    pub fn new(runtime: tokio::runtime::Handle) -> Self {
        Self {
            service: Arc::new(ExportService::new(runtime)),
            current_export: RwLock::new(None),
            last_options: RwLock::new(ExportOptions::default()),
        }
    }

    /// Start an export operation
    pub fn start_export(&self, data: ExportData, options: ExportOptions) {
        let progress = ExportProgress::new(data.rows.len());
        *self.current_export.write() = Some(CurrentExport {
            data,
            options: options.clone(),
            progress,
        });
        *self.last_options.write() = options;
    }

    /// Update export progress
    pub fn update_progress(&self, exported_rows: usize, bytes_written: u64) {
        if let Some(ref mut export) = *self.current_export.write() {
            export.progress.exported_rows = exported_rows;
            export.progress.bytes_written = bytes_written;
        }
    }

    /// Complete export
    pub fn complete_export(&self, error: Option<String>) {
        if let Some(ref mut export) = *self.current_export.write() {
            export.progress.is_complete = true;
            export.progress.error = error;
        }
    }

    /// Clear current export
    pub fn clear_export(&self) {
        *self.current_export.write() = None;
    }

    /// Get last used options
    pub fn last_options(&self) -> ExportOptions {
        self.last_options.read().clone()
    }
}
```

### 15.5 Export Dialog GPUI Component

```rust
// src/ui/export_dialog.rs

use gpui::*;
use std::sync::Arc;

use crate::export::models::*;
use crate::export::state::ExportState;
use crate::export::dialog::show_save_dialog;
use crate::theme::Theme;

/// Events emitted by the export dialog
pub enum ExportDialogEvent {
    Exported(std::path::PathBuf),
    Cancelled,
}

impl EventEmitter<ExportDialogEvent> for ExportDialog {}

/// Export dialog component
pub struct ExportDialog {
    data: ExportData,
    table_name: String,
    schema_name: Option<String>,

    // Current selections
    format: ExportFormat,
    include_headers: bool,
    null_string: String,

    // CSV options
    csv_delimiter: char,
    csv_quote_char: char,

    // JSON options
    json_array_format: bool,
    json_pretty_print: bool,

    // SQL options
    sql_table_name: String,
    sql_schema_name: String,
    sql_batch_size: usize,
    sql_on_conflict: OnConflictSelection,

    // State
    is_exporting: bool,
}

#[derive(Clone, PartialEq)]
enum OnConflictSelection {
    None,
    DoNothing,
    DoUpdate,
}

impl ExportDialog {
    pub fn new(
        data: ExportData,
        table_name: Option<String>,
        schema_name: Option<String>,
    ) -> Self {
        let table = table_name.unwrap_or_else(|| "data".to_string());
        Self {
            data,
            table_name: table.clone(),
            schema_name: schema_name.clone(),

            format: ExportFormat::Csv,
            include_headers: true,
            null_string: String::new(),

            csv_delimiter: ',',
            csv_quote_char: '"',

            json_array_format: true,
            json_pretty_print: true,

            sql_table_name: table,
            sql_schema_name: schema_name.unwrap_or_default(),
            sql_batch_size: 1000,
            sql_on_conflict: OnConflictSelection::None,

            is_exporting: false,
        }
    }

    fn build_options(&self) -> ExportOptions {
        let mut options = ExportOptions::for_format(self.format.clone());
        options.include_headers = self.include_headers;
        options.null_string = self.null_string.clone();

        if matches!(self.format, ExportFormat::Csv) {
            options.csv_options = Some(CsvOptions {
                delimiter: self.csv_delimiter,
                quote_char: self.csv_quote_char,
                ..Default::default()
            });
        }

        if matches!(self.format, ExportFormat::Json | ExportFormat::JsonLines) {
            options.json_options = Some(JsonOptions {
                array_format: self.json_array_format,
                pretty_print: self.json_pretty_print,
            });
        }

        if matches!(self.format, ExportFormat::Sql | ExportFormat::SqlCopy) {
            let on_conflict = match self.sql_on_conflict {
                OnConflictSelection::None => None,
                OnConflictSelection::DoNothing => Some(OnConflictOption::DoNothing),
                OnConflictSelection::DoUpdate => Some(OnConflictOption::DoUpdate {
                    columns: self.data.columns.iter().map(|c| c.name.clone()).collect(),
                }),
            };

            options.sql_options = Some(SqlOptions {
                table_name: self.sql_table_name.clone(),
                schema_name: if self.sql_schema_name.is_empty() {
                    None
                } else {
                    Some(self.sql_schema_name.clone())
                },
                batch_size: self.sql_batch_size,
                include_column_names: true,
                on_conflict,
            });
        }

        options
    }

    fn do_export(&mut self, cx: &mut Context<Self>) {
        self.is_exporting = true;
        cx.notify();

        let options = self.build_options();
        let default_name = format!("export.{}", self.format.file_extension());

        // Show save dialog
        let path = match show_save_dialog(&default_name, self.format.clone()) {
            Some(p) => p,
            None => {
                self.is_exporting = false;
                cx.notify();
                return;
            }
        };

        // Perform export
        let export_state = cx.global::<ExportState>();
        match export_state.service.export_to_file(&self.data, path.clone(), options) {
            Ok(_) => {
                cx.emit(ExportDialogEvent::Exported(path));
            }
            Err(e) => {
                // Show error (would be handled by error display system)
                eprintln!("Export error: {}", e);
            }
        }

        self.is_exporting = false;
        cx.notify();
    }

    fn render_format_grid(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_wrap()
            .gap_2()
            .children(ExportFormat::all().into_iter().map(|format| {
                let is_selected = self.format == format;
                let display_name = format.display_name();
                let description = format.description();

                div()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .border_1()
                    .when(is_selected, |div| {
                        div
                            .bg(theme.primary)
                            .text_color(theme.on_primary)
                            .border_color(theme.primary)
                    })
                    .when(!is_selected, |div| {
                        div
                            .bg(theme.surface)
                            .text_color(theme.text)
                            .border_color(theme.border)
                            .hover(|div| div.border_color(theme.primary))
                    })
                    .on_click({
                        let format = format.clone();
                        cx.listener(move |this, _, cx| {
                            this.format = format.clone();
                            cx.notify();
                        })
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(display_name)
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .when(is_selected, |div| div.text_color(rgba(0xffffffcc)))
                                    .when(!is_selected, |div| div.text_color(theme.text_muted))
                                    .child(description)
                            )
                    )
            }))
    }

    fn render_csv_options(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                self.render_select_row(
                    "Delimiter",
                    vec![
                        ("Comma (,)", ','),
                        ("Semicolon (;)", ';'),
                        ("Pipe (|)", '|'),
                    ],
                    self.csv_delimiter,
                    cx.listener(|this, value: &char, cx| {
                        this.csv_delimiter = *value;
                        cx.notify();
                    }),
                    cx,
                )
            )
            .child(
                self.render_select_row(
                    "Quote character",
                    vec![
                        ("Double quote (\")", '"'),
                        ("Single quote (')", '\''),
                    ],
                    self.csv_quote_char,
                    cx.listener(|this, value: &char, cx| {
                        this.csv_quote_char = *value;
                        cx.notify();
                    }),
                    cx,
                )
            )
    }

    fn render_json_options(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                self.render_checkbox(
                    "Array of objects (vs object with columns/rows)",
                    self.json_array_format,
                    cx.listener(|this, _, cx| {
                        this.json_array_format = !this.json_array_format;
                        cx.notify();
                    }),
                    cx,
                )
            )
            .when(self.format == ExportFormat::Json, |div| {
                div.child(
                    self.render_checkbox(
                        "Pretty print",
                        self.json_pretty_print,
                        cx.listener(|this, _, cx| {
                            this.json_pretty_print = !this.json_pretty_print;
                            cx.notify();
                        }),
                        cx,
                    )
                )
            })
    }

    fn render_sql_options(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                self.render_text_input(
                    "Schema name",
                    &self.sql_schema_name,
                    "Optional",
                    cx.listener(|this, value: &String, cx| {
                        this.sql_schema_name = value.clone();
                        cx.notify();
                    }),
                    cx,
                )
            )
            .child(
                self.render_text_input(
                    "Table name",
                    &self.sql_table_name,
                    "table_name",
                    cx.listener(|this, value: &String, cx| {
                        this.sql_table_name = value.clone();
                        cx.notify();
                    }),
                    cx,
                )
            )
            .when(self.format == ExportFormat::Sql, |div| {
                div
                    .child(
                        self.render_number_input(
                            "Batch size",
                            self.sql_batch_size,
                            1,
                            10000,
                            cx.listener(|this, value: &usize, cx| {
                                this.sql_batch_size = *value;
                                cx.notify();
                            }),
                            cx,
                        )
                    )
                    .child(
                        self.render_select_row(
                            "ON CONFLICT",
                            vec![
                                ("None", OnConflictSelection::None),
                                ("DO NOTHING", OnConflictSelection::DoNothing),
                                ("DO UPDATE", OnConflictSelection::DoUpdate),
                            ],
                            self.sql_on_conflict.clone(),
                            cx.listener(|this, value: &OnConflictSelection, cx| {
                                this.sql_on_conflict = value.clone();
                                cx.notify();
                            }),
                            cx,
                        )
                    )
            })
    }

    fn render_checkbox<F>(
        &self,
        label: &str,
        checked: bool,
        on_click: F,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        F: Fn(&mut Self, &ClickEvent, &mut Context<Self>) + 'static,
    {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .on_click(on_click)
            .child(
                div()
                    .size_4()
                    .rounded_sm()
                    .border_1()
                    .border_color(theme.border)
                    .when(checked, |div| {
                        div
                            .bg(theme.primary)
                            .child(
                                svg()
                                    .path("icons/check.svg")
                                    .size_3()
                                    .text_color(theme.on_primary)
                            )
                    })
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text)
                    .child(label.to_string())
            )
    }

    fn render_text_input<F>(
        &self,
        label: &str,
        value: &str,
        placeholder: &str,
        on_change: F,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        F: Fn(&mut Self, &String, &mut Context<Self>) + 'static,
    {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text)
                    .min_w_24()
                    .child(label.to_string())
            )
            .child(
                div()
                    .flex_1()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.surface)
                    .child(value.to_string())
                    // In a real implementation, this would use a TextInput component
            )
    }

    fn render_number_input<F>(
        &self,
        label: &str,
        value: usize,
        min: usize,
        max: usize,
        on_change: F,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        F: Fn(&mut Self, &usize, &mut Context<Self>) + 'static,
    {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text)
                    .min_w_24()
                    .child(label.to_string())
            )
            .child(
                div()
                    .flex_1()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.surface)
                    .child(value.to_string())
            )
    }

    fn render_select_row<T, F>(
        &self,
        label: &str,
        options: Vec<(&str, T)>,
        current: T,
        on_select: F,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        T: Clone + PartialEq + 'static,
        F: Fn(&mut Self, &T, &mut Context<Self>) + 'static,
    {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text)
                    .min_w_24()
                    .child(label.to_string())
            )
            .child(
                div()
                    .flex_1()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.surface)
                    .child(
                        options.iter()
                            .find(|(_, v)| *v == current)
                            .map(|(label, _)| label.to_string())
                            .unwrap_or_default()
                    )
                    // In a real implementation, this would use a Dropdown component
            )
    }
}

impl Render for ExportDialog {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        // Overlay
        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(|_, _, cx| {
                cx.emit(ExportDialogEvent::Cancelled);
            }))
            .child(
                // Dialog
                div()
                    .w_128()
                    .max_h(vh(90.0))
                    .bg(theme.surface)
                    .rounded_lg()
                    .shadow_xl()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .on_click(|_, _| {}) // Prevent click through
                    .child(self.render_header(cx))
                    .child(self.render_body(cx))
                    .child(self.render_footer(cx))
            )
    }
}

impl ExportDialog {
    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text)
                    .child("Export Results")
            )
            .child(
                div()
                    .p_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(|div| div.bg(theme.hover))
                    .on_click(cx.listener(|_, _, cx| {
                        cx.emit(ExportDialogEvent::Cancelled);
                    }))
                    .child(
                        svg()
                            .path("icons/x.svg")
                            .size_5()
                            .text_color(theme.text_muted)
                    )
            )
    }

    fn render_body(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex_1()
            .overflow_y_auto()
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            // Format selection
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child("Format")
                    )
                    .child(self.render_format_grid(cx))
            )
            // Options section
            .child(
                div()
                    .pt_4()
                    .border_t_1()
                    .border_color(theme.border)
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child("Options")
                    )
                    // Include headers checkbox
                    .child(
                        self.render_checkbox(
                            "Include column headers",
                            self.include_headers,
                            cx.listener(|this, _, cx| {
                                this.include_headers = !this.include_headers;
                                cx.notify();
                            }),
                            cx,
                        )
                    )
                    // NULL string input
                    .child(
                        self.render_text_input(
                            "NULL string",
                            &self.null_string,
                            "Empty string",
                            cx.listener(|this, value: &String, cx| {
                                this.null_string = value.clone();
                                cx.notify();
                            }),
                            cx,
                        )
                    )
                    // Format-specific options
                    .when(self.format == ExportFormat::Csv, |div| {
                        div.child(self.render_csv_options(cx))
                    })
                    .when(
                        matches!(self.format, ExportFormat::Json | ExportFormat::JsonLines),
                        |div| div.child(self.render_json_options(cx))
                    )
                    .when(
                        matches!(self.format, ExportFormat::Sql | ExportFormat::SqlCopy),
                        |div| div.child(self.render_sql_options(cx))
                    )
            )
            // Summary
            .child(
                div()
                    .mt_4()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(theme.surface_variant)
                    .flex()
                    .gap_2()
                    .text_sm()
                    .text_color(theme.text_muted)
                    .child(format!("{} rows", self.data.rows.len().to_formatted_string(&num_format::Locale::en)))
                    .child("•")
                    .child(format!("{} columns", self.data.columns.len()))
            )
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .justify_end()
            .gap_2()
            .px_4()
            .py_3()
            .border_t_1()
            .border_color(theme.border)
            .child(
                // Cancel button
                div()
                    .px_4()
                    .py_2()
                    .rounded_md()
                    .bg(theme.surface_variant)
                    .text_color(theme.text)
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_pointer()
                    .hover(|div| div.bg(theme.hover))
                    .on_click(cx.listener(|_, _, cx| {
                        cx.emit(ExportDialogEvent::Cancelled);
                    }))
                    .child("Cancel")
            )
            .child(
                // Export button
                div()
                    .px_4()
                    .py_2()
                    .rounded_md()
                    .bg(theme.primary)
                    .text_color(theme.on_primary)
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_pointer()
                    .when(!self.is_exporting, |div| {
                        div.hover(|div| div.bg(theme.primary_hover))
                    })
                    .when(self.is_exporting, |div| {
                        div.opacity(0.5).cursor_not_allowed()
                    })
                    .on_click(cx.listener(|this, _, cx| {
                        if !this.is_exporting {
                            this.do_export(cx);
                        }
                    }))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        svg()
                            .path("icons/download.svg")
                            .size_4()
                    )
                    .child(if self.is_exporting { "Exporting..." } else { "Export" })
            )
    }
}
```

### 15.6 Context Menu Integration

```rust
// src/ui/grid/context_menu.rs

use gpui::*;
use crate::export::models::*;
use crate::export::state::ExportState;
use crate::theme::Theme;

/// Grid context menu for copy/export operations
pub struct GridContextMenu {
    position: Point<Pixels>,
    selection: GridSelection,
    table_name: Option<String>,
    schema_name: Option<String>,
}

#[derive(Clone)]
pub struct GridSelection {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Vec<crate::query::Value>>,
    pub primary_key_column: Option<String>,
}

pub enum GridContextMenuEvent {
    CopyAsTsv,
    CopyAsCsv,
    CopyAsJson,
    CopyAsInsert,
    CopyAsUpdate,
    ExportToFile,
    Dismissed,
}

impl EventEmitter<GridContextMenuEvent> for GridContextMenu {}

impl GridContextMenu {
    pub fn new(
        position: Point<Pixels>,
        selection: GridSelection,
        table_name: Option<String>,
        schema_name: Option<String>,
    ) -> Self {
        Self {
            position,
            selection,
            table_name,
            schema_name,
        }
    }

    fn handle_copy(&mut self, format: &str, cx: &mut Context<Self>) {
        let export_state = cx.global::<ExportState>();
        let data = ExportData {
            columns: self.selection.columns.clone(),
            rows: self.selection.rows.clone(),
        };

        let result = match format {
            "tsv" => export_state.service.copy_as_tsv(&data, false),
            "csv" => export_state.service.copy_as_csv(&data, false),
            "json" => export_state.service.copy_as_json(&data),
            "insert" => {
                let table = self.table_name.as_deref().unwrap_or("table_name");
                export_state.service.copy_as_insert(&data, table, self.schema_name.as_deref())
            }
            _ => Ok(()),
        };

        if let Err(e) = result {
            eprintln!("Copy failed: {}", e);
        }

        cx.emit(GridContextMenuEvent::Dismissed);
    }
}

impl Render for GridContextMenu {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let has_single_row = self.selection.rows.len() == 1;
        let has_pk = self.selection.primary_key_column.is_some();

        div()
            .absolute()
            .top(self.position.y)
            .left(self.position.x)
            .min_w_48()
            .bg(theme.surface_elevated)
            .rounded_md()
            .shadow_lg()
            .border_1()
            .border_color(theme.border)
            .py_1()
            .flex()
            .flex_col()
            // Copy section
            .child(
                self.render_menu_item("Copy as TSV", Some("Ctrl+C"), cx.listener(|this, _, cx| {
                    this.handle_copy("tsv", cx);
                }), cx)
            )
            .child(
                self.render_menu_item("Copy as CSV", None, cx.listener(|this, _, cx| {
                    this.handle_copy("csv", cx);
                }), cx)
            )
            .child(
                self.render_menu_item("Copy as JSON", None, cx.listener(|this, _, cx| {
                    this.handle_copy("json", cx);
                }), cx)
            )
            .child(self.render_separator(cx))
            // SQL section
            .child(
                self.render_menu_item("Copy as INSERT", None, cx.listener(|this, _, cx| {
                    this.handle_copy("insert", cx);
                }), cx)
            )
            .when(has_single_row && has_pk, |div| {
                div.child(
                    self.render_menu_item("Copy as UPDATE", None, cx.listener(|this, _, cx| {
                        cx.emit(GridContextMenuEvent::CopyAsUpdate);
                    }), cx)
                )
            })
            .child(self.render_separator(cx))
            // Export section
            .child(
                self.render_menu_item("Export to file...", Some("Ctrl+Shift+E"), cx.listener(|_, _, cx| {
                    cx.emit(GridContextMenuEvent::ExportToFile);
                }), cx)
            )
    }
}

impl GridContextMenu {
    fn render_menu_item<F>(
        &self,
        label: &str,
        shortcut: Option<&str>,
        on_click: F,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        F: Fn(&mut Self, &ClickEvent, &mut Context<Self>) + 'static,
    {
        let theme = cx.global::<Theme>();

        div()
            .px_3()
            .py_1p5()
            .flex()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .hover(|div| div.bg(theme.hover))
            .on_click(on_click)
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text)
                    .child(label.to_string())
            )
            .when_some(shortcut, |div, shortcut| {
                div.child(
                    div()
                        .text_xs()
                        .text_color(theme.text_muted)
                        .child(shortcut.to_string())
                )
            })
    }

    fn render_separator(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .my_1()
            .h_px()
            .bg(theme.border)
    }
}
```

### 15.7 Keyboard Shortcuts

```rust
// src/export/shortcuts.rs

use gpui::*;

/// Register export-related keyboard shortcuts
pub fn register_export_shortcuts(cx: &mut AppContext) {
    // Copy as TSV (default copy)
    cx.bind_keys([
        KeyBinding::new("ctrl-c", CopySelection, Some("ResultsGrid")),
        KeyBinding::new("cmd-c", CopySelection, Some("ResultsGrid")),
    ]);

    // Copy with headers
    cx.bind_keys([
        KeyBinding::new("ctrl-shift-c", CopyWithHeaders, Some("ResultsGrid")),
        KeyBinding::new("cmd-shift-c", CopyWithHeaders, Some("ResultsGrid")),
    ]);

    // Export to file
    cx.bind_keys([
        KeyBinding::new("ctrl-shift-e", ExportToFile, Some("ResultsGrid")),
        KeyBinding::new("cmd-shift-e", ExportToFile, Some("ResultsGrid")),
    ]);
}

/// Copy selection action
#[derive(Clone, PartialEq)]
pub struct CopySelection;

impl_actions!(export, [CopySelection]);

/// Copy with headers action
#[derive(Clone, PartialEq)]
pub struct CopyWithHeaders;

impl_actions!(export, [CopyWithHeaders]);

/// Export to file action
#[derive(Clone, PartialEq)]
pub struct ExportToFile;

impl_actions!(export, [ExportToFile]);
```

### 15.8 Streaming Export for Large Data

```rust
// src/export/streaming.rs

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use parking_lot::RwLock;

use crate::error::Result;
use crate::query::Value;
use super::models::*;

/// Streaming exporter for large datasets
pub struct StreamingExporter {
    progress: Arc<RwLock<ExportProgress>>,
}

impl StreamingExporter {
    pub fn new(total_rows: usize) -> Self {
        Self {
            progress: Arc::new(RwLock::new(ExportProgress::new(total_rows))),
        }
    }

    pub fn progress(&self) -> ExportProgress {
        self.progress.read().clone()
    }

    /// Export data with streaming - receives rows via channel
    pub async fn export_streaming<W: Write + Send + 'static>(
        &self,
        columns: Vec<ColumnInfo>,
        mut row_receiver: mpsc::Receiver<Vec<Value>>,
        writer: W,
        options: ExportOptions,
    ) -> Result<u64> {
        let progress = self.progress.clone();

        // Spawn export task
        let handle = tokio::task::spawn_blocking(move || {
            let mut writer = writer;
            let mut bytes_written = 0u64;
            let mut row_count = 0usize;

            // Write header if needed
            if options.include_headers {
                match options.format {
                    ExportFormat::Csv | ExportFormat::Tsv => {
                        let header = columns.iter()
                            .map(|c| c.name.as_str())
                            .collect::<Vec<_>>()
                            .join(if options.format == ExportFormat::Tsv { "\t" } else { "," });
                        writeln!(writer, "{}", header)?;
                    }
                    ExportFormat::Markdown => {
                        let headers = columns.iter()
                            .map(|c| c.name.as_str())
                            .collect::<Vec<_>>();
                        writeln!(writer, "| {} |", headers.join(" | "))?;

                        let separators: Vec<&str> = columns.iter()
                            .map(|_| "---")
                            .collect();
                        writeln!(writer, "| {} |", separators.join(" | "))?;
                    }
                    _ => {}
                }
            }

            // Process rows as they arrive
            while let Some(row) = row_receiver.blocking_recv() {
                // Format and write row based on format
                let line = format_row(&columns, &row, &options);
                writeln!(writer, "{}", line)?;

                row_count += 1;
                bytes_written += line.len() as u64 + 1;

                // Update progress every 1000 rows
                if row_count % 1000 == 0 {
                    let mut p = progress.write();
                    p.exported_rows = row_count;
                    p.bytes_written = bytes_written;
                }
            }

            // Final progress update
            {
                let mut p = progress.write();
                p.exported_rows = row_count;
                p.bytes_written = bytes_written;
                p.is_complete = true;
            }

            Ok::<u64, crate::error::Error>(bytes_written)
        });

        handle.await?
    }
}

fn format_row(columns: &[ColumnInfo], row: &[Value], options: &ExportOptions) -> String {
    match options.format {
        ExportFormat::Csv => {
            let values: Vec<String> = row.iter()
                .map(|v| format_csv_value(v, &options.null_string))
                .collect();
            values.join(",")
        }
        ExportFormat::Tsv => {
            let values: Vec<String> = row.iter()
                .map(|v| format_csv_value(v, &options.null_string))
                .collect();
            values.join("\t")
        }
        ExportFormat::JsonLines => {
            let mut obj = serde_json::Map::new();
            for (i, col) in columns.iter().enumerate() {
                obj.insert(col.name.clone(), value_to_json(&row[i]));
            }
            serde_json::to_string(&obj).unwrap_or_default()
        }
        ExportFormat::Markdown => {
            let values: Vec<String> = row.iter()
                .map(|v| format_markdown_value(v, &options.null_string))
                .collect();
            format!("| {} |", values.join(" | "))
        }
        _ => String::new(), // Other formats don't support streaming
    }
}

fn format_csv_value(value: &Value, null_string: &str) -> String {
    match value {
        Value::Null => null_string.to_string(),
        Value::String(s) | Value::Text(s) => {
            if s.contains(',') || s.contains('"') || s.contains('\n') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.clone()
            }
        }
        Value::Bool(b) => b.to_string(),
        Value::Int16(n) => n.to_string(),
        Value::Int32(n) => n.to_string(),
        Value::Int64(n) => n.to_string(),
        Value::Float32(f) => f.to_string(),
        Value::Float64(f) => f.to_string(),
        Value::Numeric(s) => s.clone(),
        Value::Json(j) | Value::Jsonb(j) => j.to_string(),
        _ => format!("{:?}", value),
    }
}

fn format_markdown_value(value: &Value, null_string: &str) -> String {
    match value {
        Value::Null => format!("*{}*", if null_string.is_empty() { "NULL" } else { null_string }),
        Value::String(s) | Value::Text(s) => s.replace('|', "\\|"),
        _ => format_csv_value(value, null_string).replace('|', "\\|"),
    }
}

fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int16(n) => serde_json::json!(n),
        Value::Int32(n) => serde_json::json!(n),
        Value::Int64(n) => serde_json::json!(n),
        Value::Float32(f) => serde_json::json!(f),
        Value::Float64(f) => serde_json::json!(f),
        Value::Numeric(s) => {
            s.parse::<f64>().map(|n| serde_json::json!(n))
                .unwrap_or_else(|_| serde_json::Value::String(s.clone()))
        }
        Value::String(s) | Value::Text(s) => serde_json::Value::String(s.clone()),
        Value::Json(j) | Value::Jsonb(j) => j.clone(),
        Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(value_to_json).collect())
        }
        _ => serde_json::Value::String(format!("{:?}", value)),
    }
}
```

## Acceptance Criteria

1. **Clipboard Copy**
   - Copy selected cells as TSV (default)
   - Copy as CSV, JSON, INSERT statements
   - Preserve NULL values correctly
   - Handle special characters (quotes, newlines)
   - Cross-platform clipboard support via arboard

2. **File Export**
   - CSV with configurable delimiter, quote char
   - TSV format
   - JSON (array of objects or arrays)
   - JSON Lines (newline-delimited)
   - SQL INSERT with batch size
   - SQL COPY format
   - Markdown table
   - Excel XLSX

3. **Export Dialog**
   - Native GPUI dialog implementation
   - Format selection with descriptions
   - Format-specific options
   - Preview row/column count
   - Native file save dialog via rfd

4. **SQL Generation**
   - Valid INSERT statements
   - Proper value escaping for all PostgreSQL types
   - ON CONFLICT support (DO NOTHING, DO UPDATE)
   - Batch size configuration
   - Schema-qualified table names
   - UPDATE statement generation for single rows

5. **Large Data Handling**
   - Streaming export for large datasets
   - Progress indication with row count and bytes
   - Memory-efficient processing
   - Cancellation support

6. **Keyboard Shortcuts**
   - Ctrl/Cmd+C: Copy as TSV
   - Ctrl/Cmd+Shift+C: Copy with headers
   - Ctrl/Cmd+Shift+E: Export to file

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_export() {
        let service = ExportService::new(tokio::runtime::Handle::current());
        let data = ExportData {
            columns: vec![
                ColumnInfo { name: "id".to_string(), type_name: "int4".to_string(), type_oid: 23 },
                ColumnInfo { name: "name".to_string(), type_name: "text".to_string(), type_oid: 25 },
            ],
            rows: vec![
                vec![Value::Int32(1), Value::String("Alice".to_string())],
                vec![Value::Int32(2), Value::String("Bob".to_string())],
            ],
        };

        let result = service.export_to_string(&data, ExportOptions::for_format(ExportFormat::Csv))
            .unwrap();

        assert!(result.contains("id,name"));
        assert!(result.contains("1,Alice"));
        assert!(result.contains("2,Bob"));
    }

    #[test]
    fn test_sql_insert_export() {
        let service = ExportService::new(tokio::runtime::Handle::current());
        let data = ExportData {
            columns: vec![
                ColumnInfo { name: "id".to_string(), type_name: "int4".to_string(), type_oid: 23 },
                ColumnInfo { name: "value".to_string(), type_name: "text".to_string(), type_oid: 25 },
            ],
            rows: vec![
                vec![Value::Int32(1), Value::String("test".to_string())],
            ],
        };

        let options = ExportOptions {
            format: ExportFormat::Sql,
            sql_options: Some(SqlOptions {
                table_name: "my_table".to_string(),
                schema_name: Some("public".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = service.export_to_string(&data, options).unwrap();

        assert!(result.contains("INSERT INTO \"public\".\"my_table\""));
        assert!(result.contains("(\"id\", \"value\")"));
        assert!(result.contains("(1, 'test')"));
    }

    #[test]
    fn test_json_export() {
        let service = ExportService::new(tokio::runtime::Handle::current());
        let data = ExportData {
            columns: vec![
                ColumnInfo { name: "id".to_string(), type_name: "int4".to_string(), type_oid: 23 },
            ],
            rows: vec![
                vec![Value::Int32(42)],
            ],
        };

        let result = service.export_to_string(&data, ExportOptions::for_format(ExportFormat::Json))
            .unwrap();

        let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["id"], 42);
    }

    #[test]
    fn test_null_handling() {
        let service = ExportService::new(tokio::runtime::Handle::current());
        let data = ExportData {
            columns: vec![
                ColumnInfo { name: "value".to_string(), type_name: "text".to_string(), type_oid: 25 },
            ],
            rows: vec![
                vec![Value::Null],
            ],
        };

        // CSV with empty null string
        let result = service.export_to_string(&data, ExportOptions::for_format(ExportFormat::Csv))
            .unwrap();
        assert!(result.contains("value\n\n") || result.contains("value\r\n\r\n"));

        // CSV with custom null string
        let mut options = ExportOptions::for_format(ExportFormat::Csv);
        options.null_string = "\\N".to_string();
        let result = service.export_to_string(&data, options).unwrap();
        assert!(result.contains("\\N"));
    }

    #[test]
    fn test_special_characters_in_csv() {
        let service = ExportService::new(tokio::runtime::Handle::current());
        let data = ExportData {
            columns: vec![
                ColumnInfo { name: "text".to_string(), type_name: "text".to_string(), type_oid: 25 },
            ],
            rows: vec![
                vec![Value::String("hello, world".to_string())],
                vec![Value::String("say \"hello\"".to_string())],
                vec![Value::String("line1\nline2".to_string())],
            ],
        };

        let result = service.export_to_string(&data, ExportOptions::for_format(ExportFormat::Csv))
            .unwrap();

        assert!(result.contains("\"hello, world\""));
        assert!(result.contains("\"say \"\"hello\"\"\""));
    }
}
```

## Dependencies

### Rust Crates

```toml
[dependencies]
# CSV handling
csv = "1.3"

# Excel export
rust_xlsxwriter = "0.79"

# JSON
serde_json = "1.0"

# Clipboard
arboard = "3.4"

# File dialogs
rfd = "0.15"

# Number formatting
num-format = "0.4"
```

## Module Structure

```
src/
├── export/
│   ├── mod.rs
│   ├── models.rs        # Export format types and options
│   ├── service.rs       # Core export service
│   ├── dialog.rs        # File dialog integration
│   ├── state.rs         # Global export state
│   ├── streaming.rs     # Large data streaming export
│   └── shortcuts.rs     # Keyboard shortcut registration
├── ui/
│   ├── export_dialog.rs # GPUI export dialog component
│   └── grid/
│       └── context_menu.rs  # Grid context menu
```
