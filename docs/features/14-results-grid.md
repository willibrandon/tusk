# Feature 14: Results Grid

## Overview

The results grid displays query results in a high-performance, GPU-accelerated, spreadsheet-like interface using GPUI's native rendering. It implements custom virtual scrolling to handle millions of rows efficiently, with type-specific rendering for all PostgreSQL data types, column operations, and cell selection.

## Goals

- Display query results in a performant GPU-accelerated virtualized grid
- Handle 10M+ rows through custom virtual scrolling
- Render all PostgreSQL types with appropriate formatting
- Support column resizing, reordering, sorting, and hiding
- Enable cell and range selection with keyboard navigation
- Provide multiple display modes (grid, transposed, JSON)
- Native Rust implementation with no JavaScript dependencies

## Dependencies

- Feature 03: Frontend Architecture (GPUI component structure)
- Feature 11: Query Execution (result streaming and column metadata)

## Technical Specification

### 14.1 Virtual Scrolling Core

```rust
// src/ui/components/grid/virtual_scroll.rs

use std::ops::Range;

/// State for vertical virtual scrolling
#[derive(Clone, Debug)]
pub struct VerticalScrollState {
    /// Index of first visible row
    pub visible_start: usize,
    /// Index of last visible row (exclusive)
    pub visible_end: usize,
    /// Y offset for positioning
    pub offset_y: f32,
    /// Total scrollable height
    pub total_height: f32,
}

/// State for horizontal virtual scrolling
#[derive(Clone, Debug)]
pub struct HorizontalScrollState {
    /// Index of first visible column
    pub visible_start: usize,
    /// Index of last visible column (exclusive)
    pub visible_end: usize,
    /// X offset for positioning
    pub offset_x: f32,
    /// Total scrollable width
    pub total_width: f32,
}

/// Configuration for virtual scrolling
#[derive(Clone, Debug)]
pub struct VirtualScrollConfig {
    /// Height of each row in pixels
    pub row_height: f32,
    /// Number of extra rows to render outside viewport
    pub overscan_rows: usize,
    /// Number of extra columns to render outside viewport
    pub overscan_columns: usize,
}

impl Default for VirtualScrollConfig {
    fn default() -> Self {
        Self {
            row_height: 28.0,
            overscan_rows: 5,
            overscan_columns: 2,
        }
    }
}

/// Calculate vertical scroll state
pub fn calculate_vertical_scroll(
    scroll_top: f32,
    total_rows: usize,
    viewport_height: f32,
    config: &VirtualScrollConfig,
) -> VerticalScrollState {
    let row_height = config.row_height;
    let overscan = config.overscan_rows;

    let visible_count = (viewport_height / row_height).ceil() as usize;
    let start_index = (scroll_top / row_height).floor() as usize;

    let visible_start = start_index.saturating_sub(overscan);
    let visible_end = (start_index + visible_count + overscan).min(total_rows);

    let offset_y = visible_start as f32 * row_height;
    let total_height = total_rows as f32 * row_height;

    VerticalScrollState {
        visible_start,
        visible_end,
        offset_y,
        total_height,
    }
}

/// Calculate horizontal scroll state
pub fn calculate_horizontal_scroll(
    scroll_left: f32,
    column_widths: &[f32],
    viewport_width: f32,
    overscan: usize,
) -> HorizontalScrollState {
    let total_width: f32 = column_widths.iter().sum();

    if column_widths.is_empty() {
        return HorizontalScrollState {
            visible_start: 0,
            visible_end: 0,
            offset_x: 0.0,
            total_width,
        };
    }

    // Find start index
    let mut accumulated = 0.0;
    let mut visible_start = 0;
    let mut offset_x = 0.0;

    for (i, &width) in column_widths.iter().enumerate() {
        if accumulated + width >= scroll_left {
            visible_start = i.saturating_sub(overscan);
            offset_x = column_widths[..visible_start].iter().sum();
            break;
        }
        accumulated += width;
    }

    // Find end index
    accumulated = offset_x;
    let mut visible_end = visible_start;

    for (i, &width) in column_widths.iter().enumerate().skip(visible_start) {
        accumulated += width;
        visible_end = i + 1;
        if accumulated >= scroll_left + viewport_width {
            visible_end = (visible_end + overscan).min(column_widths.len());
            break;
        }
    }

    HorizontalScrollState {
        visible_start,
        visible_end,
        offset_x,
        total_width,
    }
}

/// Get the visible range of rows
pub fn visible_row_range(state: &VerticalScrollState) -> Range<usize> {
    state.visible_start..state.visible_end
}

/// Get the visible range of columns
pub fn visible_column_range(state: &HorizontalScrollState) -> Range<usize> {
    state.visible_start..state.visible_end
}
```

### 14.2 Grid Data Models

```rust
// src/models/grid.rs

use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::models::query::{Value, ColumnMeta};

/// State for a grid displaying query results
#[derive(Clone, Debug)]
pub struct GridData {
    /// Unique identifier for this result set
    pub query_id: Uuid,
    /// Column metadata
    pub columns: Vec<GridColumn>,
    /// Row data
    pub rows: Vec<Vec<Value>>,
    /// Total row count (may be larger than loaded rows)
    pub total_rows: usize,
    /// Query execution time in milliseconds
    pub elapsed_ms: u64,
    /// Whether more rows are available to load
    pub has_more: bool,
}

impl GridData {
    /// Create empty grid data
    pub fn empty() -> Self {
        Self {
            query_id: Uuid::nil(),
            columns: Vec::new(),
            rows: Vec::new(),
            total_rows: 0,
            elapsed_ms: 0,
            has_more: false,
        }
    }

    /// Create from query result
    pub fn from_result(
        query_id: Uuid,
        columns: Vec<ColumnMeta>,
        rows: Vec<Vec<Value>>,
        total_rows: usize,
        elapsed_ms: u64,
    ) -> Self {
        let grid_columns = columns.iter().enumerate().map(|(i, col)| {
            GridColumn::from_meta(col, &rows, i)
        }).collect();

        Self {
            query_id,
            columns: grid_columns,
            rows,
            total_rows,
            elapsed_ms,
            has_more: false,
        }
    }

    /// Append more rows
    pub fn append_rows(&mut self, new_rows: Vec<Vec<Value>>) {
        self.rows.extend(new_rows);
    }

    /// Get a cell value
    pub fn get_cell(&self, row: usize, col: usize) -> Option<&Value> {
        self.rows.get(row).and_then(|r| r.get(col))
    }

    /// Get a row
    pub fn get_row(&self, row: usize) -> Option<&Vec<Value>> {
        self.rows.get(row)
    }

    /// Get column widths as a vector
    pub fn column_widths(&self) -> Vec<f32> {
        self.columns.iter().map(|c| c.width).collect()
    }
}

/// Column state in the grid
#[derive(Clone, Debug)]
pub struct GridColumn {
    /// Column name
    pub name: String,
    /// PostgreSQL type name
    pub type_name: String,
    /// PostgreSQL type OID
    pub type_oid: u32,
    /// Current width in pixels
    pub width: f32,
    /// Minimum width
    pub min_width: f32,
    /// Maximum width
    pub max_width: f32,
    /// Whether column is hidden
    pub hidden: bool,
    /// Sort direction (if sorted)
    pub sort_direction: Option<SortDirection>,
    /// Sort priority (for multi-column sort)
    pub sort_order: Option<usize>,
}

impl GridColumn {
    /// Create from column metadata with auto-sizing
    pub fn from_meta(meta: &ColumnMeta, rows: &[Vec<Value>], col_index: usize) -> Self {
        let width = Self::calculate_width(&meta.name, &meta.type_name, rows, col_index);

        Self {
            name: meta.name.clone(),
            type_name: meta.type_name.clone(),
            type_oid: meta.type_oid,
            width,
            min_width: 50.0,
            max_width: 500.0,
            hidden: false,
            sort_direction: None,
            sort_order: None,
        }
    }

    /// Calculate optimal width based on content
    fn calculate_width(
        name: &str,
        type_name: &str,
        rows: &[Vec<Value>],
        col_index: usize,
    ) -> f32 {
        // Start with header width (approx 8px per char + padding)
        let header_width = name.len() as f32 * 8.0 + 32.0;
        let mut max_width = header_width;

        // Sample first 100 rows
        for row in rows.iter().take(100) {
            if let Some(value) = row.get(col_index) {
                let display_len = Self::estimate_display_length(value, type_name);
                let cell_width = display_len as f32 * 7.0 + 16.0;
                max_width = max_width.max(cell_width);
            }
        }

        // Clamp to reasonable bounds
        max_width.clamp(60.0, 400.0)
    }

    /// Estimate display length of a value
    fn estimate_display_length(value: &Value, type_name: &str) -> usize {
        match value {
            Value::Null => 4, // "NULL"
            Value::Bool(_) => 1, // ✓ or ✗
            Value::Int16(n) => n.to_string().len(),
            Value::Int32(n) => n.to_string().len(),
            Value::Int64(n) => n.to_string().len(),
            Value::Float32(n) => format!("{:.2}", n).len(),
            Value::Float64(n) => format!("{:.2}", n).len(),
            Value::Numeric(s) => s.len(),
            Value::Text(s) => s.len().min(50),
            Value::Bytea(b) => b.len().min(10) * 2 + 2, // hex representation
            Value::Timestamp(ts) => 19, // "YYYY-MM-DD HH:MM:SS"
            Value::TimestampTz(ts) => 25, // with timezone
            Value::Date(d) => 10, // "YYYY-MM-DD"
            Value::Time(t) => 8, // "HH:MM:SS"
            Value::TimeTz(t) => 14, // with timezone
            Value::Interval { .. } => 15,
            Value::Uuid(u) => 36,
            Value::Json(j) | Value::Jsonb(j) => j.to_string().len().min(50),
            Value::Array(arr) => format!("[{} items]", arr.len()).len(),
            Value::Point { .. } => 15, // "(x, y)"
            Value::Inet(s) | Value::Cidr(s) | Value::MacAddr(s) => s.len(),
            Value::Unknown(s) => s.len().min(30),
        }
    }
}

/// Sort direction
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    /// Toggle direction
    pub fn toggle(&self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }
}

/// Cell selection state
#[derive(Clone, Debug, Default)]
pub struct Selection {
    /// Currently selected cell (anchor)
    pub anchor: Option<CellPosition>,
    /// End of range selection
    pub cursor: Option<CellPosition>,
    /// Selected rows (for row selection mode)
    pub selected_rows: Vec<usize>,
}

impl Selection {
    /// Clear all selection
    pub fn clear(&mut self) {
        self.anchor = None;
        self.cursor = None;
        self.selected_rows.clear();
    }

    /// Select a single cell
    pub fn select_cell(&mut self, row: usize, col: usize) {
        self.anchor = Some(CellPosition { row, col });
        self.cursor = None;
        self.selected_rows.clear();
    }

    /// Extend selection to a cell
    pub fn extend_to(&mut self, row: usize, col: usize) {
        if self.anchor.is_some() {
            self.cursor = Some(CellPosition { row, col });
        }
    }

    /// Select all cells
    pub fn select_all(&mut self, rows: usize, cols: usize) {
        if rows > 0 && cols > 0 {
            self.anchor = Some(CellPosition { row: 0, col: 0 });
            self.cursor = Some(CellPosition { row: rows - 1, col: cols - 1 });
        }
    }

    /// Get the selection range (normalized)
    pub fn range(&self) -> Option<SelectionRange> {
        let anchor = self.anchor?;

        let cursor = self.cursor.unwrap_or(anchor);

        Some(SelectionRange {
            start_row: anchor.row.min(cursor.row),
            end_row: anchor.row.max(cursor.row),
            start_col: anchor.col.min(cursor.col),
            end_col: anchor.col.max(cursor.col),
        })
    }

    /// Check if a cell is selected
    pub fn is_selected(&self, row: usize, col: usize) -> bool {
        if let Some(range) = self.range() {
            row >= range.start_row && row <= range.end_row &&
            col >= range.start_col && col <= range.end_col
        } else {
            false
        }
    }

    /// Check if a cell is the anchor
    pub fn is_anchor(&self, row: usize, col: usize) -> bool {
        self.anchor.map(|a| a.row == row && a.col == col).unwrap_or(false)
    }
}

/// Position of a cell
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellPosition {
    pub row: usize,
    pub col: usize,
}

/// Range of selected cells
#[derive(Clone, Debug)]
pub struct SelectionRange {
    pub start_row: usize,
    pub end_row: usize,
    pub start_col: usize,
    pub end_col: usize,
}

impl SelectionRange {
    /// Get number of selected rows
    pub fn row_count(&self) -> usize {
        self.end_row - self.start_row + 1
    }

    /// Get number of selected columns
    pub fn col_count(&self) -> usize {
        self.end_col - self.start_col + 1
    }
}

/// Display mode for the grid
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DisplayMode {
    #[default]
    Grid,
    Transposed,
    Json,
}
```

### 14.3 Cell Renderer

```rust
// src/ui/components/grid/cell_renderer.rs

use gpui::*;

use crate::models::query::Value;
use crate::ui::theme::Theme;

/// Render a cell value with appropriate styling
pub fn render_cell(
    value: &Value,
    type_name: &str,
    is_selected: bool,
    theme: &Theme,
) -> impl IntoElement {
    let (display, style) = format_value(value, type_name, theme);

    div()
        .px_2()
        .h_full()
        .flex()
        .items_center()
        .overflow_hidden()
        .text_ellipsis()
        .whitespace_nowrap()
        .when(is_selected, |el| el.bg(theme.selected))
        .child(
            div()
                .text_sm()
                .font_family(style.font_family)
                .text_color(style.color)
                .when(style.italic, |el| el.italic())
                .when(style.align_right, |el| el.ml_auto())
                .child(display)
        )
}

/// Style information for cell display
struct CellStyle {
    color: Hsla,
    font_family: &'static str,
    italic: bool,
    align_right: bool,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self {
            color: Hsla::default(),
            font_family: "system-ui",
            italic: false,
            align_right: false,
        }
    }
}

/// Format a value for display
fn format_value(value: &Value, type_name: &str, theme: &Theme) -> (String, CellStyle) {
    match value {
        Value::Null => (
            "NULL".to_string(),
            CellStyle {
                color: theme.text_muted,
                italic: true,
                ..Default::default()
            }
        ),

        Value::Bool(b) => (
            if *b { "✓" } else { "✗" }.to_string(),
            CellStyle {
                color: if *b {
                    hsla(0.38, 0.68, 0.42, 1.0) // green
                } else {
                    hsla(0.0, 0.72, 0.51, 1.0) // red
                },
                ..Default::default()
            }
        ),

        Value::Int16(n) => format_number(*n as f64, theme),
        Value::Int32(n) => format_number(*n as f64, theme),
        Value::Int64(n) => format_number(*n as f64, theme),
        Value::Float32(n) => format_number(*n as f64, theme),
        Value::Float64(n) => format_number(*n, theme),

        Value::Numeric(s) => (
            s.clone(),
            CellStyle {
                font_family: "monospace",
                align_right: true,
                color: theme.text,
                ..Default::default()
            }
        ),

        Value::Text(s) => {
            let truncated = if s.len() > 500 {
                format!("{}…", &s[..500])
            } else {
                s.clone()
            };
            (truncated, CellStyle {
                color: theme.text,
                ..Default::default()
            })
        },

        Value::Bytea(bytes) => {
            let hex: String = bytes.iter().take(20).map(|b| format!("{:02x}", b)).collect();
            let display = if bytes.len() > 20 {
                format!("\\x{}…", hex)
            } else {
                format!("\\x{}", hex)
            };
            (display, CellStyle {
                font_family: "monospace",
                color: hsla(0.78, 0.73, 0.53, 1.0), // purple
                ..Default::default()
            })
        },

        Value::Timestamp(ts) => (
            ts.format("%Y-%m-%d %H:%M:%S").to_string(),
            CellStyle {
                font_family: "monospace",
                color: theme.text,
                ..Default::default()
            }
        ),

        Value::TimestampTz(ts) => (
            ts.format("%Y-%m-%d %H:%M:%S %Z").to_string(),
            CellStyle {
                font_family: "monospace",
                color: theme.text,
                ..Default::default()
            }
        ),

        Value::Date(d) => (
            d.format("%Y-%m-%d").to_string(),
            CellStyle {
                font_family: "monospace",
                color: theme.text,
                ..Default::default()
            }
        ),

        Value::Time(t) => (
            t.format("%H:%M:%S").to_string(),
            CellStyle {
                font_family: "monospace",
                color: theme.text,
                ..Default::default()
            }
        ),

        Value::TimeTz(t) => (
            format!("{}", t),
            CellStyle {
                font_family: "monospace",
                color: theme.text,
                ..Default::default()
            }
        ),

        Value::Interval { months, days, microseconds } => {
            let display = format_interval(*months, *days, *microseconds);
            (display, CellStyle {
                font_family: "monospace",
                color: theme.text,
                ..Default::default()
            })
        },

        Value::Uuid(u) => (
            u.to_string(),
            CellStyle {
                font_family: "monospace",
                color: theme.text,
                ..Default::default()
            }
        ),

        Value::Json(j) | Value::Jsonb(j) => {
            let preview = j.to_string();
            let truncated = if preview.len() > 100 {
                format!("{}…", &preview[..100])
            } else {
                preview
            };
            (truncated, CellStyle {
                font_family: "monospace",
                color: theme.primary,
                ..Default::default()
            })
        },

        Value::Array(arr) => (
            format!("[{} items]", arr.len()),
            CellStyle {
                color: theme.primary,
                italic: true,
                ..Default::default()
            }
        ),

        Value::Point { x, y } => (
            format!("({:.4}, {:.4})", x, y),
            CellStyle {
                font_family: "monospace",
                color: theme.text,
                ..Default::default()
            }
        ),

        Value::Inet(s) | Value::Cidr(s) | Value::MacAddr(s) => (
            s.clone(),
            CellStyle {
                font_family: "monospace",
                color: theme.text,
                ..Default::default()
            }
        ),

        Value::Unknown(s) => (
            s.clone(),
            CellStyle {
                color: theme.text_muted,
                ..Default::default()
            }
        ),
    }
}

/// Format a number for display
fn format_number(n: f64, theme: &Theme) -> (String, CellStyle) {
    let display = if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{:.6}", n).trim_end_matches('0').trim_end_matches('.').to_string()
    };

    (display, CellStyle {
        font_family: "monospace",
        align_right: true,
        color: theme.text,
        ..Default::default()
    })
}

/// Format interval for display
fn format_interval(months: i32, days: i32, microseconds: i64) -> String {
    let mut parts = Vec::new();

    if months != 0 {
        let years = months / 12;
        let remaining_months = months % 12;
        if years != 0 {
            parts.push(format!("{} year{}", years, if years.abs() == 1 { "" } else { "s" }));
        }
        if remaining_months != 0 {
            parts.push(format!("{} month{}", remaining_months, if remaining_months.abs() == 1 { "" } else { "s" }));
        }
    }

    if days != 0 {
        parts.push(format!("{} day{}", days, if days.abs() == 1 { "" } else { "s" }));
    }

    if microseconds != 0 {
        let total_seconds = microseconds / 1_000_000;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let micros = microseconds % 1_000_000;

        if hours != 0 || minutes != 0 || seconds != 0 {
            if micros != 0 {
                parts.push(format!("{:02}:{:02}:{:02}.{:06}", hours, minutes, seconds, micros));
            } else {
                parts.push(format!("{:02}:{:02}:{:02}", hours, minutes, seconds));
            }
        }
    }

    if parts.is_empty() {
        "0".to_string()
    } else {
        parts.join(" ")
    }
}

/// Render cell with expandable indicator
pub fn render_expandable_cell(
    value: &Value,
    type_name: &str,
    is_selected: bool,
    theme: &Theme,
    on_expand: impl Fn() + 'static,
) -> impl IntoElement {
    let is_expandable = matches!(
        value,
        Value::Json(_) | Value::Jsonb(_) | Value::Array(_) | Value::Text(s) if s.len() > 500
    );

    div()
        .w_full()
        .h_full()
        .flex()
        .items_center()
        .cursor(if is_expandable { CursorStyle::PointingHand } else { CursorStyle::Arrow })
        .when(is_expandable, |el| {
            el.on_double_click(move |_event, _cx| {
                on_expand();
            })
        })
        .child(render_cell(value, type_name, is_selected, theme))
        .when(is_expandable, |el| {
            el.child(
                div()
                    .ml_auto()
                    .mr_1()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .child("⋯")
            )
        })
}
```

### 14.4 Grid State

```rust
// src/state/grid_state.rs

use std::sync::Arc;
use gpui::Global;
use parking_lot::RwLock;
use uuid::Uuid;

use crate::models::grid::{GridData, GridColumn, Selection, DisplayMode, SortDirection};
use crate::models::query::Value;

/// Global grid state for GPUI
pub struct GridState {
    /// Current grid data
    data: RwLock<GridData>,
    /// Selection state
    selection: RwLock<Selection>,
    /// Display mode
    display_mode: RwLock<DisplayMode>,
    /// Scroll position
    scroll: RwLock<ScrollPosition>,
    /// Sort configuration
    sort_columns: RwLock<Vec<SortConfig>>,
}

impl Global for GridState {}

#[derive(Clone, Default)]
struct ScrollPosition {
    pub top: f32,
    pub left: f32,
}

#[derive(Clone)]
struct SortConfig {
    pub col_index: usize,
    pub direction: SortDirection,
}

impl GridState {
    /// Create new grid state
    pub fn new() -> Self {
        Self {
            data: RwLock::new(GridData::empty()),
            selection: RwLock::new(Selection::default()),
            display_mode: RwLock::new(DisplayMode::Grid),
            scroll: RwLock::new(ScrollPosition::default()),
            sort_columns: RwLock::new(Vec::new()),
        }
    }

    /// Set grid data from query result
    pub fn set_data(&self, data: GridData) {
        *self.data.write() = data;
        self.selection.write().clear();
        self.scroll.write().top = 0.0;
        self.scroll.write().left = 0.0;
        self.sort_columns.write().clear();
    }

    /// Append rows to existing data
    pub fn append_rows(&self, rows: Vec<Vec<Value>>) {
        self.data.write().append_rows(rows);
    }

    /// Get grid data
    pub fn data(&self) -> GridData {
        self.data.read().clone()
    }

    /// Get selection
    pub fn selection(&self) -> Selection {
        self.selection.read().clone()
    }

    /// Get display mode
    pub fn display_mode(&self) -> DisplayMode {
        *self.display_mode.read()
    }

    /// Set display mode
    pub fn set_display_mode(&self, mode: DisplayMode) {
        *self.display_mode.write() = mode;
    }

    /// Get scroll position
    pub fn scroll_position(&self) -> (f32, f32) {
        let scroll = self.scroll.read();
        (scroll.top, scroll.left)
    }

    /// Set scroll position
    pub fn set_scroll(&self, top: f32, left: f32) {
        let mut scroll = self.scroll.write();
        scroll.top = top;
        scroll.left = left;
    }

    // === Selection operations ===

    /// Select a cell
    pub fn select_cell(&self, row: usize, col: usize) {
        self.selection.write().select_cell(row, col);
    }

    /// Extend selection
    pub fn extend_selection(&self, row: usize, col: usize) {
        self.selection.write().extend_to(row, col);
    }

    /// Select all cells
    pub fn select_all(&self) {
        let data = self.data.read();
        self.selection.write().select_all(data.rows.len(), data.columns.len());
    }

    /// Clear selection
    pub fn clear_selection(&self) {
        self.selection.write().clear();
    }

    /// Check if cell is selected
    pub fn is_selected(&self, row: usize, col: usize) -> bool {
        self.selection.read().is_selected(row, col)
    }

    // === Column operations ===

    /// Resize a column
    pub fn resize_column(&self, col_index: usize, width: f32) {
        let mut data = self.data.write();
        if let Some(col) = data.columns.get_mut(col_index) {
            col.width = width.clamp(col.min_width, col.max_width);
        }
    }

    /// Auto-size a column
    pub fn auto_size_column(&self, col_index: usize) {
        let mut data = self.data.write();
        if col_index < data.columns.len() {
            let col = &data.columns[col_index];
            let width = GridColumn::calculate_width(
                &col.name,
                &col.type_name,
                &data.rows,
                col_index,
            );
            data.columns[col_index].width = width;
        }
    }

    /// Auto-size all columns
    pub fn auto_size_all_columns(&self) {
        let mut data = self.data.write();
        for i in 0..data.columns.len() {
            let col = &data.columns[i];
            let width = GridColumn::calculate_width(
                &col.name,
                &col.type_name,
                &data.rows,
                i,
            );
            data.columns[i].width = width;
        }
    }

    /// Toggle column visibility
    pub fn toggle_column_visibility(&self, col_index: usize) {
        let mut data = self.data.write();
        if let Some(col) = data.columns.get_mut(col_index) {
            col.hidden = !col.hidden;
        }
    }

    // === Sorting ===

    /// Sort by column
    pub fn sort_by_column(&self, col_index: usize, multi: bool) {
        let mut data = self.data.write();
        let mut sort_columns = self.sort_columns.write();

        if !multi {
            // Clear other column sorts
            for (i, col) in data.columns.iter_mut().enumerate() {
                if i != col_index {
                    col.sort_direction = None;
                    col.sort_order = None;
                }
            }
            sort_columns.clear();
        }

        // Toggle sort direction
        let col = &mut data.columns[col_index];
        col.sort_direction = match col.sort_direction {
            None => Some(SortDirection::Ascending),
            Some(SortDirection::Ascending) => Some(SortDirection::Descending),
            Some(SortDirection::Descending) => None,
        };

        // Update sort columns
        if let Some(direction) = col.sort_direction {
            let existing = sort_columns.iter().position(|s| s.col_index == col_index);
            if let Some(idx) = existing {
                sort_columns[idx].direction = direction;
            } else {
                col.sort_order = Some(sort_columns.len());
                sort_columns.push(SortConfig { col_index, direction });
            }
        } else {
            col.sort_order = None;
            sort_columns.retain(|s| s.col_index != col_index);
            // Reindex
            for (i, cfg) in sort_columns.iter().enumerate() {
                data.columns[cfg.col_index].sort_order = Some(i);
            }
        }

        // Apply sort
        if !sort_columns.is_empty() {
            Self::apply_sort(&mut data.rows, &sort_columns);
        }
    }

    fn apply_sort(rows: &mut Vec<Vec<Value>>, sort_columns: &[SortConfig]) {
        rows.sort_by(|a, b| {
            for cfg in sort_columns {
                let cmp = Self::compare_values(
                    a.get(cfg.col_index),
                    b.get(cfg.col_index),
                );
                if cmp != std::cmp::Ordering::Equal {
                    return match cfg.direction {
                        SortDirection::Ascending => cmp,
                        SortDirection::Descending => cmp.reverse(),
                    };
                }
            }
            std::cmp::Ordering::Equal
        });
    }

    fn compare_values(a: Option<&Value>, b: Option<&Value>) -> std::cmp::Ordering {
        match (a, b) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (Some(Value::Null), Some(Value::Null)) => std::cmp::Ordering::Equal,
            (Some(Value::Null), _) => std::cmp::Ordering::Less,
            (_, Some(Value::Null)) => std::cmp::Ordering::Greater,
            (Some(a), Some(b)) => Self::compare_non_null(a, b),
        }
    }

    fn compare_non_null(a: &Value, b: &Value) -> std::cmp::Ordering {
        match (a, b) {
            (Value::Int16(a), Value::Int16(b)) => a.cmp(b),
            (Value::Int32(a), Value::Int32(b)) => a.cmp(b),
            (Value::Int64(a), Value::Int64(b)) => a.cmp(b),
            (Value::Float32(a), Value::Float32(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
            (Value::Float64(a), Value::Float64(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
            (Value::Text(a), Value::Text(b)) => a.cmp(b),
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            (Value::Timestamp(a), Value::Timestamp(b)) => a.cmp(b),
            (Value::TimestampTz(a), Value::TimestampTz(b)) => a.cmp(b),
            (Value::Date(a), Value::Date(b)) => a.cmp(b),
            _ => a.to_string().cmp(&b.to_string()),
        }
    }

    // === Data extraction ===

    /// Get selected data for copying
    pub fn get_selected_data(&self) -> (Vec<String>, Vec<Vec<Value>>) {
        let data = self.data.read();
        let selection = self.selection.read();

        if let Some(range) = selection.range() {
            let columns: Vec<String> = data.columns[range.start_col..=range.end_col]
                .iter()
                .map(|c| c.name.clone())
                .collect();

            let rows: Vec<Vec<Value>> = data.rows[range.start_row..=range.end_row]
                .iter()
                .map(|row| row[range.start_col..=range.end_col].to_vec())
                .collect();

            (columns, rows)
        } else {
            (Vec::new(), Vec::new())
        }
    }
}

impl GridColumn {
    pub fn calculate_width(
        name: &str,
        type_name: &str,
        rows: &[Vec<Value>],
        col_index: usize,
    ) -> f32 {
        let header_width = name.len() as f32 * 8.0 + 32.0;
        let mut max_width = header_width;

        for row in rows.iter().take(100) {
            if let Some(value) = row.get(col_index) {
                let display_len = Self::estimate_display_length(value, type_name);
                let cell_width = display_len as f32 * 7.0 + 16.0;
                max_width = max_width.max(cell_width);
            }
        }

        max_width.clamp(60.0, 400.0)
    }
}
```

### 14.5 Results Grid Component

```rust
// src/ui/components/grid/results_grid.rs

use gpui::*;
use uuid::Uuid;

use crate::models::grid::{GridData, Selection, DisplayMode};
use crate::state::grid_state::GridState;
use crate::ui::theme::Theme;
use crate::ui::components::grid::{
    virtual_scroll::{
        calculate_vertical_scroll, calculate_horizontal_scroll,
        VirtualScrollConfig, VerticalScrollState, HorizontalScrollState,
    },
    cell_renderer::render_cell,
};

/// Events emitted by the results grid
pub enum ResultsGridEvent {
    CellDoubleClicked { row: usize, col: usize },
    ColumnResized { col: usize, width: f32 },
    ColumnSorted { col: usize, multi: bool },
    SelectionChanged,
    CopyRequested,
    ContextMenuRequested { x: f32, y: f32, row: usize, col: usize },
}

/// Results grid component
pub struct ResultsGrid {
    /// Scroll configuration
    config: VirtualScrollConfig,
    /// Cached viewport size
    viewport_size: Size<Pixels>,
    /// Context menu state
    context_menu: Option<ContextMenuState>,
    /// Column being resized
    resizing_column: Option<ResizeState>,
}

struct ContextMenuState {
    position: Point<Pixels>,
    row: usize,
    col: usize,
}

struct ResizeState {
    col_index: usize,
    start_x: f32,
    start_width: f32,
}

impl ResultsGrid {
    pub fn new() -> Self {
        Self {
            config: VirtualScrollConfig::default(),
            viewport_size: Size::default(),
            context_menu: None,
            resizing_column: None,
        }
    }

    /// Handle scroll event
    fn handle_scroll(&mut self, scroll_top: f32, scroll_left: f32, cx: &mut Context<Self>) {
        let grid_state = cx.global::<GridState>();
        grid_state.set_scroll(scroll_top, scroll_left);
        cx.notify();
    }

    /// Handle cell click
    fn handle_cell_click(
        &mut self,
        row: usize,
        col: usize,
        shift: bool,
        ctrl: bool,
        cx: &mut Context<Self>,
    ) {
        let grid_state = cx.global::<GridState>();

        if shift {
            grid_state.extend_selection(row, col);
        } else if ctrl {
            // Toggle row selection
        } else {
            grid_state.select_cell(row, col);
        }

        cx.emit(ResultsGridEvent::SelectionChanged);
        cx.notify();
    }

    /// Handle keyboard navigation
    fn handle_key(&mut self, key: &KeyDownEvent, cx: &mut Context<Self>) -> bool {
        let grid_state = cx.global::<GridState>();
        let data = grid_state.data();
        let selection = grid_state.selection();

        let anchor = match selection.anchor {
            Some(a) => a,
            None => return false,
        };

        let shift = key.modifiers.shift;
        let cmd = key.modifiers.command || key.modifiers.control;

        match &key.keystroke.key {
            key if key == "up" => {
                let new_row = anchor.row.saturating_sub(1);
                if shift {
                    grid_state.extend_selection(new_row, anchor.col);
                } else {
                    grid_state.select_cell(new_row, anchor.col);
                }
                cx.notify();
                true
            }
            key if key == "down" => {
                let new_row = (anchor.row + 1).min(data.rows.len().saturating_sub(1));
                if shift {
                    grid_state.extend_selection(new_row, anchor.col);
                } else {
                    grid_state.select_cell(new_row, anchor.col);
                }
                cx.notify();
                true
            }
            key if key == "left" => {
                let new_col = anchor.col.saturating_sub(1);
                if shift {
                    grid_state.extend_selection(anchor.row, new_col);
                } else {
                    grid_state.select_cell(anchor.row, new_col);
                }
                cx.notify();
                true
            }
            key if key == "right" => {
                let new_col = (anchor.col + 1).min(data.columns.len().saturating_sub(1));
                if shift {
                    grid_state.extend_selection(anchor.row, new_col);
                } else {
                    grid_state.select_cell(anchor.row, new_col);
                }
                cx.notify();
                true
            }
            key if key == "a" && cmd => {
                grid_state.select_all();
                cx.notify();
                true
            }
            key if key == "c" && cmd => {
                cx.emit(ResultsGridEvent::CopyRequested);
                true
            }
            _ => false,
        }
    }

    /// Render column headers
    fn render_headers(
        &self,
        data: &GridData,
        h_state: &HorizontalScrollState,
        row_num_width: f32,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .h_8()
            .bg(theme.surface)
            .border_b_1()
            .border_color(theme.border)
            .sticky()
            .top_0()
            .z_index(10)
            // Row number header
            .child(
                div()
                    .w(px(row_num_width))
                    .h_full()
                    .bg(theme.surface_secondary)
                    .border_r_1()
                    .border_color(theme.border)
                    .sticky()
                    .left_0()
                    .z_index(5)
            )
            // Offset spacer
            .child(div().w(px(h_state.offset_x)))
            // Visible column headers
            .children(
                (h_state.visible_start..h_state.visible_end).map(|col_idx| {
                    let col = &data.columns[col_idx];
                    self.render_column_header(col, col_idx, theme, cx)
                })
            )
    }

    /// Render a single column header
    fn render_column_header(
        &self,
        column: &crate::models::grid::GridColumn,
        col_index: usize,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let width = column.width;
        let name = column.name.clone();
        let sort_dir = column.sort_direction;
        let sort_order = column.sort_order;

        div()
            .id(ElementId::Name(format!("col-header-{}", col_index).into()))
            .w(px(width))
            .h_full()
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .bg(theme.surface)
            .border_r_1()
            .border_color(theme.border)
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            .on_click(cx.listener(move |this, event: &ClickEvent, cx| {
                let multi = event.modifiers.shift;
                cx.emit(ResultsGridEvent::ColumnSorted { col: col_index, multi });
            }))
            // Column name
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_ellipsis()
                    .overflow_hidden()
                    .child(name)
            )
            // Sort indicator
            .when_some(sort_dir, |el, direction| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .text_color(theme.primary)
                        .child(
                            svg()
                                .path(match direction {
                                    crate::models::grid::SortDirection::Ascending => "icons/chevron-up.svg",
                                    crate::models::grid::SortDirection::Descending => "icons/chevron-down.svg",
                                })
                                .size_3p5()
                        )
                        .when_some(sort_order, |el, order| {
                            if order > 0 {
                                el.child(
                                    div()
                                        .text_xs()
                                        .ml_px()
                                        .child(format!("{}", order + 1))
                                )
                            } else {
                                el
                            }
                        })
                )
            })
            // Resize handle
            .child(
                div()
                    .absolute()
                    .right_0()
                    .top_0()
                    .bottom_0()
                    .w_1()
                    .cursor(CursorStyle::ResizeLeftRight)
                    .hover(|style| style.bg(theme.primary))
                    .on_drag_start(cx.listener(move |this, event: &DragStartEvent, _cx| {
                        this.resizing_column = Some(ResizeState {
                            col_index,
                            start_x: event.position.x.0,
                            start_width: width,
                        });
                    }))
            )
    }

    /// Render a data row
    fn render_row(
        &self,
        data: &GridData,
        row_index: usize,
        h_state: &HorizontalScrollState,
        row_num_width: f32,
        is_row_selected: bool,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let row = &data.rows[row_index];

        div()
            .flex()
            .h(px(self.config.row_height))
            .border_b_1()
            .border_color(theme.border)
            .when(is_row_selected, |el| el.bg(theme.selected))
            .hover(|style| style.bg(theme.hover))
            // Row number
            .child(
                div()
                    .w(px(row_num_width))
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_end()
                    .pr_2()
                    .bg(theme.surface_secondary)
                    .text_xs()
                    .text_color(theme.text_muted)
                    .border_r_1()
                    .border_color(theme.border)
                    .sticky()
                    .left_0()
                    .z_index(5)
                    .child(format!("{}", row_index + 1))
            )
            // Offset spacer
            .child(div().w(px(h_state.offset_x)))
            // Visible cells
            .children(
                (h_state.visible_start..h_state.visible_end).map(|col_idx| {
                    let col = &data.columns[col_idx];
                    let value = &row[col_idx];
                    let is_selected = cx.global::<GridState>().is_selected(row_index, col_idx);

                    div()
                        .id(ElementId::Name(format!("cell-{}-{}", row_index, col_idx).into()))
                        .w(px(col.width))
                        .h_full()
                        .border_r_1()
                        .border_color(theme.border)
                        .when(is_selected, |el| {
                            el.bg(theme.selected)
                                .outline_2()
                                .outline_color(theme.primary)
                                .outline_offset(px(-2.0))
                        })
                        .on_click(cx.listener(move |this, event: &ClickEvent, cx| {
                            this.handle_cell_click(
                                row_index,
                                col_idx,
                                event.modifiers.shift,
                                event.modifiers.command || event.modifiers.control,
                                cx,
                            );
                        }))
                        .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &MouseDownEvent, cx| {
                            this.context_menu = Some(ContextMenuState {
                                position: event.position,
                                row: row_index,
                                col: col_idx,
                            });
                            cx.emit(ResultsGridEvent::ContextMenuRequested {
                                x: event.position.x.0,
                                y: event.position.y.0,
                                row: row_index,
                                col: col_idx,
                            });
                            cx.notify();
                        }))
                        .child(render_cell(value, &col.type_name, is_selected, theme))
                })
            )
    }

    /// Render empty state
    fn render_empty(&self, is_loading: bool, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .h_full()
            .text_color(theme.text_muted)
            .child(
                if is_loading {
                    "Loading results..."
                } else {
                    "No results to display"
                }
            )
    }
}

impl EventEmitter<ResultsGridEvent> for ResultsGrid {}

impl Render for ResultsGrid {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let grid_state = cx.global::<GridState>();
        let data = grid_state.data();
        let (scroll_top, scroll_left) = grid_state.scroll_position();

        if data.rows.is_empty() {
            return div()
                .w_full()
                .h_full()
                .bg(theme.background)
                .child(self.render_empty(false, &theme))
                .into_any_element();
        }

        // Calculate row number column width
        let row_num_width = (data.total_rows.to_string().len() as f32 * 8.0 + 16.0).max(50.0);

        // Get column widths (excluding hidden)
        let column_widths: Vec<f32> = data.columns.iter()
            .filter(|c| !c.hidden)
            .map(|c| c.width)
            .collect();

        // Calculate virtual scroll states
        let v_state = calculate_vertical_scroll(
            scroll_top,
            data.rows.len(),
            self.viewport_size.height.0,
            &self.config,
        );

        let h_state = calculate_horizontal_scroll(
            scroll_left,
            &column_widths,
            self.viewport_size.width.0 - row_num_width,
            self.config.overscan_columns,
        );

        div()
            .id("results-grid")
            .w_full()
            .h_full()
            .bg(theme.background)
            .text_sm()
            .overflow_hidden()
            .focusable()
            .on_key_down(cx.listener(|this, event, cx| {
                this.handle_key(event, cx);
            }))
            .child(
                div()
                    .id("grid-scroll")
                    .w_full()
                    .h_full()
                    .overflow_auto()
                    .on_scroll(cx.listener(move |this, event: &ScrollEvent, cx| {
                        this.handle_scroll(event.scroll_position.y.0, event.scroll_position.x.0, cx);
                    }))
                    // Headers
                    .child(self.render_headers(&data, &h_state, row_num_width, &theme, cx))
                    // Body with virtual content height
                    .child(
                        div()
                            .relative()
                            .h(px(v_state.total_height))
                            // Offset spacer
                            .child(div().h(px(v_state.offset_y)))
                            // Visible rows
                            .children(
                                (v_state.visible_start..v_state.visible_end).map(|row_idx| {
                                    self.render_row(
                                        &data,
                                        row_idx,
                                        &h_state,
                                        row_num_width,
                                        false, // TODO: row selection
                                        &theme,
                                        cx,
                                    )
                                })
                            )
                    )
            )
            .into_any_element()
    }
}
```

### 14.6 Results Toolbar

```rust
// src/ui/components/grid/results_toolbar.rs

use gpui::*;

use crate::models::grid::DisplayMode;
use crate::state::grid_state::GridState;
use crate::ui::theme::Theme;

/// Events from results toolbar
pub enum ResultsToolbarEvent {
    ExportRequested,
    DisplayModeChanged(DisplayMode),
    PageChanged(usize),
}

/// Results toolbar component
pub struct ResultsToolbar {
    /// Current page (for paginated results)
    current_page: usize,
    /// Total pages
    total_pages: usize,
    /// Page size
    page_size: usize,
}

impl ResultsToolbar {
    pub fn new() -> Self {
        Self {
            current_page: 1,
            total_pages: 1,
            page_size: 1000,
        }
    }

    /// Update pagination
    pub fn set_pagination(&mut self, total_rows: usize, page_size: usize) {
        self.page_size = page_size;
        self.total_pages = (total_rows + page_size - 1) / page_size;
        self.current_page = self.current_page.min(self.total_pages).max(1);
    }

    /// Format row range display
    fn format_row_range(&self, total_rows: usize) -> String {
        let start = (self.current_page - 1) * self.page_size + 1;
        let end = (self.current_page * self.page_size).min(total_rows);
        format!("{}-{} of {}", start, end, total_rows)
    }

    /// Render display mode toggle
    fn render_mode_toggle(
        &self,
        current_mode: DisplayMode,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .overflow_hidden()
            .child(self.render_mode_button(DisplayMode::Grid, "icons/grid.svg", "Grid view", current_mode, theme, cx))
            .child(div().w_px().bg(theme.border))
            .child(self.render_mode_button(DisplayMode::Transposed, "icons/list.svg", "Transposed view", current_mode, theme, cx))
            .child(div().w_px().bg(theme.border))
            .child(self.render_mode_button(DisplayMode::Json, "icons/braces.svg", "JSON view", current_mode, theme, cx))
    }

    fn render_mode_button(
        &self,
        mode: DisplayMode,
        icon: &'static str,
        tooltip: &'static str,
        current_mode: DisplayMode,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_active = mode == current_mode;

        div()
            .flex()
            .items_center()
            .justify_center()
            .w_7()
            .h_6()
            .cursor_pointer()
            .bg(if is_active { theme.primary } else { Hsla::transparent_black() })
            .text_color(if is_active { Hsla::white() } else { theme.text_muted })
            .hover(|style| {
                if !is_active {
                    style.bg(theme.hover).text_color(theme.text)
                } else {
                    style
                }
            })
            .on_click(cx.listener(move |_this, _event, cx| {
                cx.emit(ResultsToolbarEvent::DisplayModeChanged(mode));
            }))
            .child(svg().path(icon).size_4())
    }

    /// Render pagination controls
    fn render_pagination(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let current = self.current_page;
        let total = self.total_pages;

        div()
            .flex()
            .items_center()
            .gap_1()
            // First page
            .child(self.render_page_button("icons/chevrons-left.svg", current > 1, 1, theme, cx))
            // Previous page
            .child(self.render_page_button("icons/chevron-left.svg", current > 1, current.saturating_sub(1), theme, cx))
            // Page info
            .child(
                div()
                    .text_color(theme.text_muted)
                    .text_sm()
                    .min_w_24()
                    .text_center()
                    .child(format!("Page {} of {}", current, total))
            )
            // Next page
            .child(self.render_page_button("icons/chevron-right.svg", current < total, current + 1, theme, cx))
            // Last page
            .child(self.render_page_button("icons/chevrons-right.svg", current < total, total, theme, cx))
    }

    fn render_page_button(
        &self,
        icon: &'static str,
        enabled: bool,
        target_page: usize,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .w_6()
            .h_6()
            .rounded_sm()
            .cursor(if enabled { CursorStyle::PointingHand } else { CursorStyle::Arrow })
            .text_color(if enabled { theme.text_muted } else { theme.text_muted.opacity(0.5) })
            .when(enabled, |el| {
                el.hover(|style| style.bg(theme.hover).text_color(theme.text))
                    .on_click(cx.listener(move |_this, _event, cx| {
                        cx.emit(ResultsToolbarEvent::PageChanged(target_page));
                    }))
            })
            .child(svg().path(icon).size_4())
    }
}

impl EventEmitter<ResultsToolbarEvent> for ResultsToolbar {}

impl Render for ResultsToolbar {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let grid_state = cx.global::<GridState>();
        let data = grid_state.data();
        let display_mode = grid_state.display_mode();

        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .bg(theme.surface)
            .border_t_1()
            .border_color(theme.border)
            .text_sm()
            // Row count info
            .child(
                div()
                    .text_color(theme.text_muted)
                    .child(self.format_row_range(data.total_rows))
                    .child(" rows")
            )
            // Elapsed time
            .when(data.elapsed_ms > 0, |el| {
                el.child(
                    div()
                        .text_color(theme.text_muted)
                        .child(format!("• {}ms", data.elapsed_ms))
                )
            })
            // Spacer
            .child(div().flex_1())
            // Display mode toggle
            .child(self.render_mode_toggle(display_mode, &theme, cx))
            // Export button
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w_7()
                    .h_7()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .cursor_pointer()
                    .text_color(theme.text_muted)
                    .hover(|style| style.bg(theme.hover).text_color(theme.text))
                    .on_click(cx.listener(|_this, _event, cx| {
                        cx.emit(ResultsToolbarEvent::ExportRequested);
                    }))
                    .child(svg().path("icons/download.svg").size_4())
            )
            // Pagination (if multiple pages)
            .when(self.total_pages > 1, |el| {
                el.child(div().w_px().h_5().bg(theme.border).mx_1())
                    .child(self.render_pagination(&theme, cx))
            })
    }
}
```

### 14.7 Transposed View

```rust
// src/ui/components/grid/transposed_view.rs

use gpui::*;

use crate::models::grid::GridData;
use crate::state::grid_state::GridState;
use crate::ui::theme::Theme;
use crate::ui::components::grid::cell_renderer::render_cell;

/// Transposed view - shows one row at a time as key-value pairs
pub struct TransposedView {
    /// Current row index
    current_row: usize,
}

impl TransposedView {
    pub fn new() -> Self {
        Self { current_row: 0 }
    }

    /// Set current row
    pub fn set_row(&mut self, row: usize) {
        self.current_row = row;
    }

    /// Navigate to next row
    pub fn next_row(&mut self, total_rows: usize) {
        if self.current_row < total_rows.saturating_sub(1) {
            self.current_row += 1;
        }
    }

    /// Navigate to previous row
    pub fn prev_row(&mut self) {
        self.current_row = self.current_row.saturating_sub(1);
    }
}

impl Render for TransposedView {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let grid_state = cx.global::<GridState>();
        let data = grid_state.data();

        if data.rows.is_empty() {
            return div()
                .w_full()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.text_muted)
                .child("No data")
                .into_any_element();
        }

        let row = &data.rows[self.current_row.min(data.rows.len() - 1)];

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            // Navigation header
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_2()
                    .bg(theme.surface)
                    .border_b_1()
                    .border_color(theme.border)
                    // Previous button
                    .child(
                        div()
                            .cursor(if self.current_row > 0 { CursorStyle::PointingHand } else { CursorStyle::Arrow })
                            .text_color(if self.current_row > 0 { theme.text } else { theme.text_muted })
                            .on_click(cx.listener(|this, _event, cx| {
                                this.prev_row();
                                cx.notify();
                            }))
                            .child("← Previous")
                    )
                    // Row indicator
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .child(format!("Row {} of {}", self.current_row + 1, data.rows.len()))
                    )
                    // Next button
                    .child(
                        div()
                            .cursor(if self.current_row < data.rows.len() - 1 { CursorStyle::PointingHand } else { CursorStyle::Arrow })
                            .text_color(if self.current_row < data.rows.len() - 1 { theme.text } else { theme.text_muted })
                            .on_click(cx.listener(move |this, _event, cx| {
                                this.next_row(data.rows.len());
                                cx.notify();
                            }))
                            .child("Next →")
                    )
            )
            // Key-value pairs
            .child(
                div()
                    .flex_1()
                    .overflow_y_auto()
                    .p_4()
                    .children(
                        data.columns.iter().enumerate().map(|(i, col)| {
                            let value = &row[i];

                            div()
                                .flex()
                                .py_2()
                                .border_b_1()
                                .border_color(theme.border)
                                // Column name
                                .child(
                                    div()
                                        .w_48()
                                        .pr_4()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.text_muted)
                                        .child(col.name.clone())
                                )
                                // Value
                                .child(
                                    div()
                                        .flex_1()
                                        .child(render_cell(value, &col.type_name, false, &theme))
                                )
                        })
                    )
            )
            .into_any_element()
    }
}
```

### 14.8 JSON View

```rust
// src/ui/components/grid/json_view.rs

use gpui::*;
use serde_json::json;

use crate::models::query::Value;
use crate::state::grid_state::GridState;
use crate::ui::theme::Theme;

/// JSON view - shows results as formatted JSON
pub struct JsonView {
    /// Formatted JSON string
    formatted_json: String,
    /// Scroll position
    scroll_top: f32,
}

impl JsonView {
    pub fn new() -> Self {
        Self {
            formatted_json: String::new(),
            scroll_top: 0.0,
        }
    }

    /// Update JSON content from grid data
    pub fn update_from_grid(&mut self, cx: &mut Context<Self>) {
        let grid_state = cx.global::<GridState>();
        let data = grid_state.data();

        let json_array: Vec<serde_json::Value> = data.rows.iter().map(|row| {
            let mut obj = serde_json::Map::new();
            for (i, col) in data.columns.iter().enumerate() {
                let value = &row[i];
                obj.insert(col.name.clone(), Self::value_to_json(value));
            }
            serde_json::Value::Object(obj)
        }).collect();

        self.formatted_json = serde_json::to_string_pretty(&json_array)
            .unwrap_or_else(|_| "Error formatting JSON".to_string());

        cx.notify();
    }

    /// Convert Value to serde_json::Value
    fn value_to_json(value: &Value) -> serde_json::Value {
        match value {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => json!(b),
            Value::Int16(n) => json!(n),
            Value::Int32(n) => json!(n),
            Value::Int64(n) => json!(n),
            Value::Float32(n) => json!(n),
            Value::Float64(n) => json!(n),
            Value::Numeric(s) => json!(s),
            Value::Text(s) => json!(s),
            Value::Bytea(b) => json!(format!("\\x{}", hex::encode(b))),
            Value::Timestamp(ts) => json!(ts.to_rfc3339()),
            Value::TimestampTz(ts) => json!(ts.to_rfc3339()),
            Value::Date(d) => json!(d.to_string()),
            Value::Time(t) => json!(t.to_string()),
            Value::TimeTz(t) => json!(t.to_string()),
            Value::Interval { months, days, microseconds } => json!({
                "months": months,
                "days": days,
                "microseconds": microseconds
            }),
            Value::Uuid(u) => json!(u.to_string()),
            Value::Json(j) | Value::Jsonb(j) => j.clone(),
            Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(Self::value_to_json).collect())
            }
            Value::Point { x, y } => json!({ "x": x, "y": y }),
            Value::Inet(s) | Value::Cidr(s) | Value::MacAddr(s) => json!(s),
            Value::Unknown(s) => json!(s),
        }
    }
}

impl Render for JsonView {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();

        div()
            .w_full()
            .h_full()
            .bg(theme.background)
            .overflow_auto()
            .p_4()
            .child(
                div()
                    .font_family("monospace")
                    .text_sm()
                    .whitespace_pre()
                    .text_color(theme.text)
                    .child(self.formatted_json.clone())
            )
    }
}
```

## Acceptance Criteria

1. **Virtual Scrolling**
   - Render only visible rows + overscan buffer
   - Handle 10M+ rows without performance degradation
   - Smooth GPU-accelerated scrolling in both directions
   - Proper row height calculations

2. **Type Rendering**
   - NULL displayed as styled "NULL" text
   - Booleans as checkmarks/crosses with color coding
   - Numbers right-aligned with formatting
   - Timestamps in locale format
   - JSON with preview and expand capability
   - Arrays with item count
   - UUIDs in monospace font
   - Bytea as hex representation

3. **Column Operations**
   - Resize columns by dragging header border
   - Click header to sort (ascending/descending/none)
   - Shift+click for multi-column sort
   - Context menu for hide/autosize

4. **Cell Selection**
   - Click to select single cell
   - Shift+click for range selection
   - Ctrl/Cmd+A to select all
   - Arrow key navigation
   - Ctrl/Cmd+C to copy

5. **Display Modes**
   - Grid mode (default spreadsheet)
   - Transposed mode (single row as key-value with navigation)
   - JSON mode (raw formatted JSON output)

6. **Performance**
   - GPU-accelerated rendering via GPUI
   - Render 1000 visible rows in < 16ms
   - Memory efficient with virtualization
   - Smooth 60fps scrolling

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertical_scroll_calculation() {
        let config = VirtualScrollConfig::default();
        let state = calculate_vertical_scroll(
            100.0, // scroll_top
            1000,  // total_rows
            500.0, // viewport_height
            &config,
        );

        // With 28px row height, scrolled 100px = ~3.5 rows
        assert!(state.visible_start <= 3);
        assert!(state.visible_end >= 21); // 500/28 + overscan
        assert_eq!(state.total_height, 1000.0 * 28.0);
    }

    #[test]
    fn test_selection_range() {
        let mut selection = Selection::default();

        selection.select_cell(5, 3);
        selection.extend_to(10, 7);

        let range = selection.range().unwrap();
        assert_eq!(range.start_row, 5);
        assert_eq!(range.end_row, 10);
        assert_eq!(range.start_col, 3);
        assert_eq!(range.end_col, 7);
    }

    #[test]
    fn test_selection_check() {
        let mut selection = Selection::default();
        selection.select_cell(2, 2);
        selection.extend_to(4, 4);

        assert!(selection.is_selected(3, 3));
        assert!(!selection.is_selected(1, 1));
        assert!(!selection.is_selected(5, 5));
    }

    #[test]
    fn test_column_auto_width() {
        let rows: Vec<Vec<Value>> = vec![
            vec![Value::Text("short".to_string())],
            vec![Value::Text("much longer text here".to_string())],
            vec![Value::Text("mid".to_string())],
        ];

        let width = GridColumn::calculate_width("Column", "text", &rows, 0);
        assert!(width >= 100.0); // Should accommodate longest text
        assert!(width <= 400.0); // Should be capped
    }

    #[test]
    fn test_sort_direction_toggle() {
        let dir = SortDirection::Ascending;
        assert_eq!(dir.toggle(), SortDirection::Descending);
        assert_eq!(dir.toggle().toggle(), SortDirection::Ascending);
    }
}
```

## Dependencies

- Feature 03: Frontend Architecture (GPUI component patterns)
- Feature 11: Query Execution (result data and streaming)
- Feature 06: Settings (display preferences like locale, null display)
