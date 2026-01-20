# Feature 15: Export and Copy Functionality

## Overview

Export and copy functionality enables users to extract query results in multiple formats for use in other applications. This includes clipboard operations (copy as TSV, CSV, JSON, INSERT statements) and file exports (CSV, JSON, SQL, Excel, Markdown).

## Goals

- Copy selected cells or entire result sets to clipboard
- Export results to various file formats
- Support format-specific options (delimiters, headers, etc.)
- Generate SQL statements from data (INSERT, UPDATE, COPY)
- Handle large exports efficiently with streaming

## Dependencies

- Feature 14: Results Grid (data selection)
- Feature 11: Query Execution (column metadata)
- Feature 04: IPC Layer (file dialogs and writing)

## Technical Specification

### 15.1 Export Service (Rust)

```rust
// src-tauri/src/services/export.rs

use std::io::Write;
use std::path::PathBuf;
use std::fs::File;
use serde::{Deserialize, Serialize};
use csv::WriterBuilder;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::models::query::{Value, ColumnMeta};

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
            null_string: "".to_string(),
            csv_options: Some(CsvOptions::default()),
            json_options: None,
            sql_options: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonOptions {
    pub array_format: bool,  // true = array of objects, false = array of arrays
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SqlOptions {
    pub table_name: String,
    pub schema_name: Option<String>,
    pub batch_size: usize,
    pub include_column_names: bool,
    pub on_conflict: Option<OnConflictOption>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnConflictOption {
    DoNothing,
    DoUpdate { columns: Vec<String> },
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

pub struct ExportService;

impl ExportService {
    /// Export data to file
    pub fn export_to_file(
        columns: &[ColumnMeta],
        rows: &[Vec<Value>],
        path: PathBuf,
        options: ExportOptions,
    ) -> Result<u64> {
        let file = File::create(&path)?;
        let bytes_written = Self::export_to_writer(columns, rows, file, options)?;
        Ok(bytes_written)
    }

    /// Export data to writer
    pub fn export_to_writer<W: Write>(
        columns: &[ColumnMeta],
        rows: &[Vec<Value>],
        writer: W,
        options: ExportOptions,
    ) -> Result<u64> {
        match options.format {
            ExportFormat::Csv | ExportFormat::Tsv => {
                Self::export_csv(columns, rows, writer, &options)
            }
            ExportFormat::Json => {
                Self::export_json(columns, rows, writer, &options)
            }
            ExportFormat::JsonLines => {
                Self::export_json_lines(columns, rows, writer, &options)
            }
            ExportFormat::Sql => {
                Self::export_sql_insert(columns, rows, writer, &options)
            }
            ExportFormat::SqlCopy => {
                Self::export_sql_copy(columns, rows, writer, &options)
            }
            ExportFormat::Markdown => {
                Self::export_markdown(columns, rows, writer, &options)
            }
            ExportFormat::Excel => {
                Self::export_excel(columns, rows, writer, &options)
            }
        }
    }

    /// Export to string (for clipboard)
    pub fn export_to_string(
        columns: &[ColumnMeta],
        rows: &[Vec<Value>],
        options: ExportOptions,
    ) -> Result<String> {
        let mut buffer = Vec::new();
        Self::export_to_writer(columns, rows, &mut buffer, options)?;
        Ok(String::from_utf8(buffer)?)
    }

    fn export_csv<W: Write>(
        columns: &[ColumnMeta],
        rows: &[Vec<Value>],
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
            let headers: Vec<&str> = columns.iter()
                .map(|c| c.name.as_str())
                .collect();
            csv_writer.write_record(&headers)?;
        }

        // Write rows
        for row in rows {
            let record: Vec<String> = row.iter()
                .map(|v| Self::format_value_for_csv(v, &options.null_string))
                .collect();
            csv_writer.write_record(&record)?;
        }

        csv_writer.flush()?;
        let inner = csv_writer.into_inner()?;
        // Return approximate bytes
        Ok(0) // Would need to track actual bytes
    }

    fn export_json<W: Write>(
        columns: &[ColumnMeta],
        rows: &[Vec<Value>],
        mut writer: W,
        options: &ExportOptions,
    ) -> Result<u64> {
        let json_opts = options.json_options.as_ref()
            .cloned()
            .unwrap_or_default();

        if json_opts.array_format {
            // Array of objects
            let objects: Vec<serde_json::Map<String, serde_json::Value>> = rows
                .iter()
                .map(|row| {
                    let mut obj = serde_json::Map::new();
                    for (i, col) in columns.iter().enumerate() {
                        let value = Self::value_to_json(&row[i]);
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
            // Array of arrays
            let arrays: Vec<Vec<serde_json::Value>> = rows
                .iter()
                .map(|row| row.iter().map(Self::value_to_json).collect())
                .collect();

            let output = serde_json::json!({
                "columns": columns.iter().map(|c| &c.name).collect::<Vec<_>>(),
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
        columns: &[ColumnMeta],
        rows: &[Vec<Value>],
        mut writer: W,
        _options: &ExportOptions,
    ) -> Result<u64> {
        for row in rows {
            let mut obj = serde_json::Map::new();
            for (i, col) in columns.iter().enumerate() {
                let value = Self::value_to_json(&row[i]);
                obj.insert(col.name.clone(), value);
            }

            let line = serde_json::to_string(&obj)?;
            writeln!(writer, "{}", line)?;
        }

        Ok(0)
    }

    fn export_sql_insert<W: Write>(
        columns: &[ColumnMeta],
        rows: &[Vec<Value>],
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

        let column_names: Vec<String> = columns.iter()
            .map(|c| format!("\"{}\"", c.name))
            .collect();
        let columns_clause = column_names.join(", ");

        // Write in batches
        for chunk in rows.chunks(sql_opts.batch_size) {
            write!(writer, "INSERT INTO {} ({})\nVALUES\n", table_name, columns_clause)?;

            for (i, row) in chunk.iter().enumerate() {
                let values: Vec<String> = row.iter()
                    .zip(columns.iter())
                    .map(|(v, c)| Self::format_value_for_sql(v, &c.type_name))
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
        columns: &[ColumnMeta],
        rows: &[Vec<Value>],
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

        let column_names: Vec<String> = columns.iter()
            .map(|c| format!("\"{}\"", c.name))
            .collect();

        writeln!(writer, "COPY {} ({}) FROM stdin;", table_name, column_names.join(", "))?;

        for row in rows {
            let values: Vec<String> = row.iter()
                .map(|v| Self::format_value_for_copy(v, &options.null_string))
                .collect();
            writeln!(writer, "{}", values.join("\t"))?;
        }

        writeln!(writer, "\\.")?;

        Ok(0)
    }

    fn export_markdown<W: Write>(
        columns: &[ColumnMeta],
        rows: &[Vec<Value>],
        mut writer: W,
        options: &ExportOptions,
    ) -> Result<u64> {
        // Header row
        let headers: Vec<&str> = columns.iter()
            .map(|c| c.name.as_str())
            .collect();
        writeln!(writer, "| {} |", headers.join(" | "))?;

        // Separator row
        let separators: Vec<String> = columns.iter()
            .map(|c| {
                match c.type_name.as_str() {
                    "int2" | "int4" | "int8" | "float4" | "float8" | "numeric" | "money" => "---:".to_string(),
                    _ => "---".to_string(),
                }
            })
            .collect();
        writeln!(writer, "| {} |", separators.join(" | "))?;

        // Data rows
        for row in rows {
            let values: Vec<String> = row.iter()
                .map(|v| Self::format_value_for_markdown(v, &options.null_string))
                .collect();
            writeln!(writer, "| {} |", values.join(" | "))?;
        }

        Ok(0)
    }

    fn export_excel<W: Write>(
        columns: &[ColumnMeta],
        rows: &[Vec<Value>],
        mut writer: W,
        options: &ExportOptions,
    ) -> Result<u64> {
        use rust_xlsxwriter::{Workbook, Format};

        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        let header_format = Format::new().set_bold();

        // Write headers
        for (col, column) in columns.iter().enumerate() {
            worksheet.write_with_format(0, col as u16, &column.name, &header_format)?;
        }

        // Write data
        for (row_idx, row) in rows.iter().enumerate() {
            for (col_idx, value) in row.iter().enumerate() {
                Self::write_excel_value(worksheet, row_idx + 1, col_idx, value)?;
            }
        }

        // Auto-fit columns
        for (col, _) in columns.iter().enumerate() {
            worksheet.set_column_width(col as u16, 15)?;
        }

        let buffer = workbook.save_to_buffer()?;
        writer.write_all(&buffer)?;

        Ok(buffer.len() as u64)
    }

    fn write_excel_value(
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
            Value::Number(n) => {
                worksheet.write_number(row as u32, col as u16, *n as f64)?;
            }
            Value::Float(f) => {
                worksheet.write_number(row as u32, col as u16, *f)?;
            }
            Value::String(s) => {
                worksheet.write_string(row as u32, col as u16, s)?;
            }
            Value::Json(j) => {
                worksheet.write_string(row as u32, col as u16, &j.to_string())?;
            }
            _ => {
                worksheet.write_string(row as u32, col as u16, &format!("{:?}", value))?;
            }
        }
        Ok(())
    }

    // Value formatting helpers
    fn format_value_for_csv(value: &Value, null_string: &str) -> String {
        match value {
            Value::Null => null_string.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::Json(j) => j.to_string(),
            Value::Array(arr) => format!("{{{}}}", arr.iter()
                .map(|v| Self::format_value_for_csv(v, null_string))
                .collect::<Vec<_>>()
                .join(",")),
            Value::Bytea { hex } => format!("\\x{}", hex),
            Value::Interval { iso } => iso.clone(),
            Value::Point { x, y } => format!("({},{})", x, y),
            Value::Unknown { text } => text.clone(),
        }
    }

    fn format_value_for_sql(value: &Value, type_name: &str) -> String {
        match value {
            Value::Null => "NULL".to_string(),
            Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            Value::Number(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => format!("'{}'", s.replace('\'', "''")),
            Value::Json(j) => format!("'{}'", j.to_string().replace('\'', "''")),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter()
                    .map(|v| Self::format_value_for_sql(v, ""))
                    .collect();
                format!("ARRAY[{}]", items.join(", "))
            }
            Value::Bytea { hex } => format!("'\\x{}'", hex),
            Value::Interval { iso } => format!("'{}'::interval", iso),
            Value::Point { x, y } => format!("point({}, {})", x, y),
            Value::Unknown { text } => format!("'{}'", text.replace('\'', "''")),
        }
    }

    fn format_value_for_copy(value: &Value, null_string: &str) -> String {
        match value {
            Value::Null => null_string.to_string(),
            Value::String(s) => s.replace('\\', "\\\\")
                .replace('\t', "\\t")
                .replace('\n', "\\n")
                .replace('\r', "\\r"),
            _ => Self::format_value_for_csv(value, null_string),
        }
    }

    fn format_value_for_markdown(value: &Value, null_string: &str) -> String {
        match value {
            Value::Null => format!("*{}*", null_string),
            Value::String(s) => s.replace('|', "\\|"),
            _ => Self::format_value_for_csv(value, null_string).replace('|', "\\|"),
        }
    }

    fn value_to_json(value: &Value) -> serde_json::Value {
        match value {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Number(n) => serde_json::json!(n),
            Value::Float(f) => serde_json::json!(f),
            Value::String(s) => serde_json::Value::String(s.clone()),
            Value::Json(j) => j.clone(),
            Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(Self::value_to_json).collect())
            }
            Value::Bytea { hex } => serde_json::json!({ "type": "bytea", "hex": hex }),
            Value::Interval { iso } => serde_json::json!({ "type": "interval", "iso": iso }),
            Value::Point { x, y } => serde_json::json!({ "type": "point", "x": x, "y": y }),
            Value::Unknown { text } => serde_json::Value::String(text.clone()),
        }
    }
}
```

### 15.2 IPC Commands for Export

```rust
// src-tauri/src/commands/export.rs

use std::path::PathBuf;
use tauri::{State, AppHandle};
use tauri::api::dialog::FileDialogBuilder;

use crate::error::Result;
use crate::models::query::{Value, ColumnMeta};
use crate::services::export::{ExportService, ExportOptions, ExportFormat};
use crate::state::AppState;

#[tauri::command]
pub async fn export_to_file(
    columns: Vec<ColumnMeta>,
    rows: Vec<Vec<Value>>,
    path: String,
    options: ExportOptions,
) -> Result<u64> {
    let path = PathBuf::from(path);
    ExportService::export_to_file(&columns, &rows, path, options)
}

#[tauri::command]
pub async fn export_to_clipboard(
    columns: Vec<ColumnMeta>,
    rows: Vec<Vec<Value>>,
    options: ExportOptions,
) -> Result<String> {
    ExportService::export_to_string(&columns, &rows, options)
}

#[tauri::command]
pub fn get_export_file_extension(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::Csv => "csv",
        ExportFormat::Tsv => "tsv",
        ExportFormat::Json | ExportFormat::JsonLines => "json",
        ExportFormat::Sql | ExportFormat::SqlCopy => "sql",
        ExportFormat::Markdown => "md",
        ExportFormat::Excel => "xlsx",
    }
}

#[tauri::command]
pub fn get_export_mime_type(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::Csv | ExportFormat::Tsv => "text/csv",
        ExportFormat::Json | ExportFormat::JsonLines => "application/json",
        ExportFormat::Sql | ExportFormat::SqlCopy => "application/sql",
        ExportFormat::Markdown => "text/markdown",
        ExportFormat::Excel => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    }
}

#[tauri::command]
pub async fn show_save_dialog(
    app: AppHandle,
    default_name: String,
    filters: Vec<(String, Vec<String>)>,
) -> Result<Option<PathBuf>> {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut dialog = FileDialogBuilder::new();

    dialog = dialog.set_file_name(&default_name);

    for (name, extensions) in filters {
        dialog = dialog.add_filter(&name, &extensions.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    }

    dialog.save_file(move |path| {
        let _ = tx.send(path);
    });

    let path = rx.recv().map_err(|_| crate::error::Error::Cancelled)?;
    Ok(path)
}
```

### 15.3 Frontend Export Service

```typescript
// src/lib/services/export.ts

import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import type { ColumnMeta, Value } from './query';

export type ExportFormat =
	| 'csv'
	| 'tsv'
	| 'json'
	| 'jsonLines'
	| 'sql'
	| 'sqlCopy'
	| 'markdown'
	| 'excel';

export interface ExportOptions {
	format: ExportFormat;
	include_headers: boolean;
	null_string: string;
	csv_options?: CsvOptions;
	json_options?: JsonOptions;
	sql_options?: SqlOptions;
}

export interface CsvOptions {
	delimiter: string;
	quote_char: string;
	escape_char?: string;
	line_terminator: string;
}

export interface JsonOptions {
	array_format: boolean;
	pretty_print: boolean;
}

export interface SqlOptions {
	table_name: string;
	schema_name?: string;
	batch_size: number;
	include_column_names: boolean;
	on_conflict?: OnConflictOption;
}

export type OnConflictOption = { type: 'do_nothing' } | { type: 'do_update'; columns: string[] };

const defaultOptions: Record<ExportFormat, Partial<ExportOptions>> = {
	csv: {
		include_headers: true,
		csv_options: {
			delimiter: ',',
			quote_char: '"',
			line_terminator: '\n'
		}
	},
	tsv: {
		include_headers: true,
		csv_options: {
			delimiter: '\t',
			quote_char: '"',
			line_terminator: '\n'
		}
	},
	json: {
		json_options: {
			array_format: true,
			pretty_print: true
		}
	},
	jsonLines: {
		json_options: {
			array_format: true,
			pretty_print: false
		}
	},
	sql: {
		sql_options: {
			table_name: 'table_name',
			batch_size: 1000,
			include_column_names: true
		}
	},
	sqlCopy: {
		null_string: '\\N',
		sql_options: {
			table_name: 'table_name',
			batch_size: 1000,
			include_column_names: true
		}
	},
	markdown: {
		include_headers: true
	},
	excel: {
		include_headers: true
	}
};

const formatExtensions: Record<ExportFormat, string> = {
	csv: 'csv',
	tsv: 'tsv',
	json: 'json',
	jsonLines: 'jsonl',
	sql: 'sql',
	sqlCopy: 'sql',
	markdown: 'md',
	excel: 'xlsx'
};

const formatFilters: Record<ExportFormat, { name: string; extensions: string[] }> = {
	csv: { name: 'CSV Files', extensions: ['csv'] },
	tsv: { name: 'TSV Files', extensions: ['tsv', 'txt'] },
	json: { name: 'JSON Files', extensions: ['json'] },
	jsonLines: { name: 'JSON Lines Files', extensions: ['jsonl', 'json'] },
	sql: { name: 'SQL Files', extensions: ['sql'] },
	sqlCopy: { name: 'SQL Files', extensions: ['sql'] },
	markdown: { name: 'Markdown Files', extensions: ['md', 'markdown'] },
	excel: { name: 'Excel Files', extensions: ['xlsx'] }
};

class ExportService {
	async exportToFile(
		columns: ColumnMeta[],
		rows: Value[][],
		format: ExportFormat,
		options?: Partial<ExportOptions>
	): Promise<boolean> {
		const mergedOptions: ExportOptions = {
			format,
			include_headers: true,
			null_string: '',
			...defaultOptions[format],
			...options
		};

		// Show save dialog
		const defaultName = `export.${formatExtensions[format]}`;
		const filter = formatFilters[format];

		const path = await save({
			defaultPath: defaultName,
			filters: [filter]
		});

		if (!path) return false;

		// Export to file
		await invoke('export_to_file', {
			columns,
			rows,
			path,
			options: mergedOptions
		});

		return true;
	}

	async exportToClipboard(
		columns: ColumnMeta[],
		rows: Value[][],
		format: ExportFormat,
		options?: Partial<ExportOptions>
	): Promise<void> {
		const mergedOptions: ExportOptions = {
			format,
			include_headers: true,
			null_string: '',
			...defaultOptions[format],
			...options
		};

		const text = await invoke<string>('export_to_clipboard', {
			columns,
			rows,
			options: mergedOptions
		});

		await navigator.clipboard.writeText(text);
	}

	async copyAsTsv(columns: ColumnMeta[], rows: Value[][]): Promise<void> {
		return this.exportToClipboard(columns, rows, 'tsv', {
			include_headers: false
		});
	}

	async copyAsCsv(columns: ColumnMeta[], rows: Value[][]): Promise<void> {
		return this.exportToClipboard(columns, rows, 'csv', {
			include_headers: false
		});
	}

	async copyAsJson(columns: ColumnMeta[], rows: Value[][]): Promise<void> {
		return this.exportToClipboard(columns, rows, 'json');
	}

	async copyAsInsert(
		columns: ColumnMeta[],
		rows: Value[][],
		tableName: string,
		schemaName?: string
	): Promise<void> {
		return this.exportToClipboard(columns, rows, 'sql', {
			sql_options: {
				table_name: tableName,
				schema_name: schemaName,
				batch_size: 1000,
				include_column_names: true
			}
		});
	}

	async copyAsUpdate(
		columns: ColumnMeta[],
		row: Value[],
		tableName: string,
		primaryKeyColumn: string,
		schemaName?: string
	): Promise<void> {
		// Generate UPDATE statement for single row
		const pkIndex = columns.findIndex((c) => c.name === primaryKeyColumn);
		if (pkIndex === -1) throw new Error('Primary key column not found');

		const pkValue = this.formatSqlValue(row[pkIndex], columns[pkIndex].type_name);
		const setClause = columns
			.filter((_, i) => i !== pkIndex)
			.map((col, i) => {
				const actualIndex = i < pkIndex ? i : i + 1;
				return `"${col.name}" = ${this.formatSqlValue(row[actualIndex], col.type_name)}`;
			})
			.join(', ');

		const tableRef = schemaName ? `"${schemaName}"."${tableName}"` : `"${tableName}"`;

		const sql = `UPDATE ${tableRef}\nSET ${setClause}\nWHERE "${primaryKeyColumn}" = ${pkValue};`;

		await navigator.clipboard.writeText(sql);
	}

	private formatSqlValue(value: Value, typeName: string): string {
		if (value === null) return 'NULL';
		if (typeof value === 'boolean') return value ? 'TRUE' : 'FALSE';
		if (typeof value === 'number') return value.toString();
		if (typeof value === 'string') return `'${value.replace(/'/g, "''")}'`;
		if (typeof value === 'object' && 'hex' in value) return `'\\x${value.hex}'`;
		if (typeof value === 'object') return `'${JSON.stringify(value).replace(/'/g, "''")}'`;
		return `'${String(value).replace(/'/g, "''")}'`;
	}
}

export const exportService = new ExportService();
```

### 15.4 Export Dialog Component

```svelte
<!-- src/lib/components/dialogs/ExportDialog.svelte -->
<script lang="ts">
	import { X, Download } from 'lucide-svelte';
	import { exportService, type ExportFormat, type ExportOptions } from '$lib/services/export';
	import type { ColumnMeta, Value } from '$lib/services/query';

	interface Props {
		columns: ColumnMeta[];
		rows: Value[][];
		tableName?: string;
		schemaName?: string;
		onClose: () => void;
	}

	let { columns, rows, tableName = 'data', schemaName, onClose }: Props = $props();

	let format = $state<ExportFormat>('csv');
	let includeHeaders = $state(true);
	let nullString = $state('');

	// CSV options
	let delimiter = $state(',');
	let quoteChar = $state('"');

	// JSON options
	let jsonArrayFormat = $state(true);
	let jsonPrettyPrint = $state(true);

	// SQL options
	let sqlTableName = $state(tableName);
	let sqlSchemaName = $state(schemaName ?? '');
	let sqlBatchSize = $state(1000);
	let sqlOnConflict = $state<'none' | 'doNothing' | 'doUpdate'>('none');

	let isExporting = $state(false);

	const formatOptions: { value: ExportFormat; label: string; description: string }[] = [
		{ value: 'csv', label: 'CSV', description: 'Comma-separated values' },
		{ value: 'tsv', label: 'TSV', description: 'Tab-separated values' },
		{ value: 'json', label: 'JSON', description: 'JSON array' },
		{ value: 'jsonLines', label: 'JSON Lines', description: 'Newline-delimited JSON' },
		{ value: 'sql', label: 'SQL INSERT', description: 'INSERT statements' },
		{ value: 'sqlCopy', label: 'SQL COPY', description: 'PostgreSQL COPY format' },
		{ value: 'markdown', label: 'Markdown', description: 'Markdown table' },
		{ value: 'excel', label: 'Excel', description: 'XLSX spreadsheet' }
	];

	async function handleExport() {
		isExporting = true;

		try {
			const options: Partial<ExportOptions> = {
				include_headers: includeHeaders,
				null_string: nullString
			};

			if (format === 'csv' || format === 'tsv') {
				options.csv_options = {
					delimiter: format === 'tsv' ? '\t' : delimiter,
					quote_char: quoteChar,
					line_terminator: '\n'
				};
			}

			if (format === 'json' || format === 'jsonLines') {
				options.json_options = {
					array_format: jsonArrayFormat,
					pretty_print: jsonPrettyPrint
				};
			}

			if (format === 'sql' || format === 'sqlCopy') {
				options.sql_options = {
					table_name: sqlTableName,
					schema_name: sqlSchemaName || undefined,
					batch_size: sqlBatchSize,
					include_column_names: true,
					on_conflict:
						sqlOnConflict === 'doNothing'
							? { type: 'do_nothing' }
							: sqlOnConflict === 'doUpdate'
								? { type: 'do_update', columns: columns.map((c) => c.name) }
								: undefined
				};
			}

			const success = await exportService.exportToFile(columns, rows, format, options);

			if (success) {
				onClose();
			}
		} finally {
			isExporting = false;
		}
	}
</script>

<div class="dialog-overlay" onclick={onClose}>
	<div class="dialog" onclick={(e) => e.stopPropagation()}>
		<div class="dialog-header">
			<h2>Export Results</h2>
			<button class="close-btn" onclick={onClose}>
				<X size={20} />
			</button>
		</div>

		<div class="dialog-body">
			<div class="form-group">
				<label>Format</label>
				<div class="format-grid">
					{#each formatOptions as opt}
						<button
							class="format-option"
							class:selected={format === opt.value}
							onclick={() => (format = opt.value)}
						>
							<span class="format-label">{opt.label}</span>
							<span class="format-desc">{opt.description}</span>
						</button>
					{/each}
				</div>
			</div>

			<div class="options-section">
				<h3>Options</h3>

				<label class="checkbox-label">
					<input type="checkbox" bind:checked={includeHeaders} />
					Include column headers
				</label>

				<div class="form-row">
					<label>NULL string</label>
					<input type="text" bind:value={nullString} placeholder="Empty string" />
				</div>

				{#if format === 'csv'}
					<div class="form-row">
						<label>Delimiter</label>
						<select bind:value={delimiter}>
							<option value=",">Comma (,)</option>
							<option value=";">Semicolon (;)</option>
							<option value="|">Pipe (|)</option>
						</select>
					</div>

					<div class="form-row">
						<label>Quote character</label>
						<select bind:value={quoteChar}>
							<option value=""">Double quote (")</option>
							<option value="'">Single quote (')</option>
						</select>
					</div>
				{/if}

				{#if format === 'json' || format === 'jsonLines'}
					<label class="checkbox-label">
						<input type="checkbox" bind:checked={jsonArrayFormat} />
						Array of objects (vs array of arrays)
					</label>

					{#if format === 'json'}
						<label class="checkbox-label">
							<input type="checkbox" bind:checked={jsonPrettyPrint} />
							Pretty print
						</label>
					{/if}
				{/if}

				{#if format === 'sql' || format === 'sqlCopy'}
					<div class="form-row">
						<label>Schema name</label>
						<input type="text" bind:value={sqlSchemaName} placeholder="Optional" />
					</div>

					<div class="form-row">
						<label>Table name</label>
						<input type="text" bind:value={sqlTableName} />
					</div>

					{#if format === 'sql'}
						<div class="form-row">
							<label>Batch size</label>
							<input type="number" bind:value={sqlBatchSize} min="1" max="10000" />
						</div>

						<div class="form-row">
							<label>ON CONFLICT</label>
							<select bind:value={sqlOnConflict}>
								<option value="none">None</option>
								<option value="doNothing">DO NOTHING</option>
								<option value="doUpdate">DO UPDATE</option>
							</select>
						</div>
					{/if}
				{/if}
			</div>

			<div class="export-summary">
				<span>{rows.length.toLocaleString()} rows</span>
				<span>•</span>
				<span>{columns.length} columns</span>
			</div>
		</div>

		<div class="dialog-footer">
			<button class="btn btn-secondary" onclick={onClose}> Cancel </button>
			<button class="btn btn-primary" onclick={handleExport} disabled={isExporting}>
				<Download size={16} />
				{isExporting ? 'Exporting...' : 'Export'}
			</button>
		</div>
	</div>
</div>

<style>
	.dialog-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
	}

	.dialog {
		background: var(--surface-color);
		border-radius: 0.5rem;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1);
		width: 500px;
		max-height: 90vh;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.dialog-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem;
		border-bottom: 1px solid var(--border-color);
	}

	.dialog-header h2 {
		margin: 0;
		font-size: 1.125rem;
		font-weight: 600;
	}

	.close-btn {
		display: flex;
		padding: 0.25rem;
		border: none;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
		border-radius: 0.25rem;
	}

	.close-btn:hover {
		background: var(--hover-color);
	}

	.dialog-body {
		padding: 1rem;
		overflow-y: auto;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.form-group > label {
		display: block;
		margin-bottom: 0.5rem;
		font-weight: 500;
		font-size: 0.875rem;
	}

	.format-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 0.5rem;
	}

	.format-option {
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 0.75rem 0.5rem;
		border: 1px solid var(--border-color);
		border-radius: 0.375rem;
		background: none;
		cursor: pointer;
		transition: all 0.15s;
	}

	.format-option:hover {
		border-color: var(--primary-color);
	}

	.format-option.selected {
		border-color: var(--primary-color);
		background: var(--primary-color);
		color: white;
	}

	.format-label {
		font-weight: 500;
		font-size: 0.875rem;
	}

	.format-desc {
		font-size: 0.625rem;
		color: var(--text-muted);
		margin-top: 0.125rem;
	}

	.format-option.selected .format-desc {
		color: rgba(255, 255, 255, 0.8);
	}

	.options-section {
		padding-top: 1rem;
		border-top: 1px solid var(--border-color);
	}

	.options-section h3 {
		font-size: 0.875rem;
		font-weight: 500;
		margin: 0 0 0.75rem 0;
	}

	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.5rem;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.form-row {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.5rem;
	}

	.form-row label {
		font-size: 0.875rem;
		min-width: 100px;
	}

	.form-row input,
	.form-row select {
		flex: 1;
		padding: 0.375rem 0.5rem;
		border: 1px solid var(--border-color);
		border-radius: 0.25rem;
		font-size: 0.875rem;
	}

	.export-summary {
		display: flex;
		gap: 0.5rem;
		margin-top: 1rem;
		padding: 0.75rem;
		background: var(--surface-secondary);
		border-radius: 0.375rem;
		font-size: 0.875rem;
		color: var(--text-muted);
	}

	.dialog-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
		padding: 1rem;
		border-top: 1px solid var(--border-color);
	}

	.btn {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.15s;
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-secondary {
		background: var(--surface-secondary);
		color: var(--text-color);
	}

	.btn-secondary:hover:not(:disabled) {
		background: var(--hover-color);
	}

	.btn-primary {
		background: var(--primary-color);
		color: white;
	}

	.btn-primary:hover:not(:disabled) {
		background: var(--primary-hover);
	}
</style>
```

## Acceptance Criteria

1. **Clipboard Copy**
   - Copy selected cells as TSV (default)
   - Copy as CSV, JSON, INSERT statements
   - Preserve NULL values correctly
   - Handle special characters (quotes, newlines)

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
   - Format selection with descriptions
   - Format-specific options
   - Preview row/column count
   - Save file dialog integration

4. **SQL Generation**
   - Valid INSERT statements
   - Proper value escaping
   - ON CONFLICT support
   - Batch size configuration
   - Schema-qualified table names

5. **Large Data Handling**
   - Stream large exports
   - Progress indication for big files
   - Memory-efficient processing

## MCP Testing Instructions

### Using Tauri MCP

```typescript
// Execute query
await mcp.ipc_execute_command({
	command: 'execute_query',
	args: { connId: connectionId, sql: 'SELECT * FROM users LIMIT 100' }
});

// Test export to clipboard
const csvData = await mcp.ipc_execute_command({
	command: 'export_to_clipboard',
	args: {
		columns,
		rows,
		options: { format: 'csv', include_headers: true, null_string: '' }
	}
});

// Verify CSV format
assert(csvData.includes('id,name,email'));
assert(csvData.split('\n').length === 101); // header + 100 rows

// Test SQL INSERT export
const sqlData = await mcp.ipc_execute_command({
	command: 'export_to_clipboard',
	args: {
		columns,
		rows,
		options: {
			format: 'sql',
			sql_options: { table_name: 'users', batch_size: 10 }
		}
	}
});

// Verify INSERT statements
assert(sqlData.includes('INSERT INTO "users"'));
assert((sqlData.match(/INSERT INTO/g) || []).length === 10); // 10 batches

// Test JSON export
const jsonData = await mcp.ipc_execute_command({
	command: 'export_to_clipboard',
	args: {
		columns,
		rows,
		options: {
			format: 'json',
			json_options: { array_format: true, pretty_print: true }
		}
	}
});

const parsed = JSON.parse(jsonData);
assert(Array.isArray(parsed));
assert(parsed.length === 100);
```

## Dependencies

- csv crate (Rust CSV writing)
- rust_xlsxwriter (Excel export)
- serde_json (JSON serialization)
- @tauri-apps/plugin-dialog (save dialogs)
- @tauri-apps/plugin-fs (file writing)
