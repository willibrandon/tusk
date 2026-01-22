//! Results panel for displaying query results with streaming support.
//!
//! The results panel lives in the bottom dock and displays query output
//! including data grids, affected row counts, and execution information.
//!
//! Features:
//! - Streaming results via mpsc channel (FR-011, FR-012)
//! - Column metadata display (FR-014)
//! - Execution time and row count (FR-015)
//! - Error display with details

use gpui::{
    div, prelude::*, px, App, Context, EventEmitter, FocusHandle, Render, SharedString, Task,
    Window,
};

use crate::icon::{Icon, IconName, IconSize};
use crate::panel::{DockPosition, Focusable, Panel, PanelEvent};
use crate::spinner::{Spinner, SpinnerSize};
use crate::TuskTheme;

#[cfg(feature = "persistence")]
use tusk_core::{ColumnInfo, QueryEvent, TuskError};

#[cfg(feature = "persistence")]
use tokio::sync::mpsc;

/// Status of the results panel (FR-014, FR-015).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ResultsStatus {
    /// No query has been executed yet.
    #[default]
    Empty,
    /// Waiting for first batch of results.
    Loading,
    /// Receiving batches of results.
    Streaming,
    /// Query completed successfully.
    Complete,
    /// Query failed with an error.
    Error,
}

impl ResultsStatus {
    /// Check if the panel is empty.
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Check if the panel is loading.
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// Check if the panel is streaming results.
    pub fn is_streaming(&self) -> bool {
        matches!(self, Self::Streaming)
    }

    /// Check if the query completed.
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }

    /// Check if there was an error.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }

    /// Check if actively receiving data.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Loading | Self::Streaming)
    }
}

/// Column information for display in the results grid.
#[derive(Debug, Clone)]
pub struct DisplayColumn {
    /// Column name
    pub name: String,
    /// PostgreSQL type name
    pub type_name: String,
}

#[cfg(feature = "persistence")]
impl From<ColumnInfo> for DisplayColumn {
    fn from(col: ColumnInfo) -> Self {
        Self { name: col.name, type_name: col.type_name }
    }
}

/// Error information for display.
#[derive(Debug, Clone)]
pub struct DisplayError {
    /// Error message
    pub message: String,
    /// Optional hint
    pub hint: Option<String>,
    /// Optional error code
    pub code: Option<String>,
    /// Error position in query (1-indexed, for T063)
    pub position: Option<usize>,
    /// Whether this was a query cancellation
    pub is_cancelled: bool,
}

#[cfg(feature = "persistence")]
impl From<TuskError> for DisplayError {
    fn from(err: TuskError) -> Self {
        Self {
            is_cancelled: err.is_cancelled(),
            message: err.to_string(),
            hint: err.hint().map(|s| s.to_string()),
            code: err.pg_code().map(|s| s.to_string()),
            position: err.position(),
        }
    }
}

/// Represents a row of data for display.
/// Each cell is pre-converted to a String for rendering.
#[derive(Debug, Clone)]
pub struct DisplayRow {
    /// Cell values as strings
    pub cells: Vec<String>,
}

/// State for the results panel (FR-011, FR-012, FR-014, FR-015).
pub struct ResultsPanelState {
    /// Column metadata from the query
    pub columns: Vec<DisplayColumn>,
    /// Accumulated result rows
    pub rows: Vec<DisplayRow>,
    /// Total rows received so far
    pub total_rows: usize,
    /// Query execution time in milliseconds
    pub execution_time_ms: Option<u64>,
    /// Rows affected (for INSERT/UPDATE/DELETE)
    pub rows_affected: Option<u64>,
    /// Current status
    pub status: ResultsStatus,
    /// Error information if status is Error
    pub error: Option<DisplayError>,
}

impl Default for ResultsPanelState {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            total_rows: 0,
            execution_time_ms: None,
            rows_affected: None,
            status: ResultsStatus::Empty,
            error: None,
        }
    }
}

impl ResultsPanelState {
    /// Clear all data and reset to empty state.
    pub fn clear(&mut self) {
        self.columns.clear();
        self.rows.clear();
        self.total_rows = 0;
        self.execution_time_ms = None;
        self.rows_affected = None;
        self.status = ResultsStatus::Empty;
        self.error = None;
    }

    /// Set to loading state (clear previous results).
    pub fn set_loading(&mut self) {
        self.clear();
        self.status = ResultsStatus::Loading;
    }
}

/// Results panel for displaying query output (FR-011, FR-012, FR-014, FR-015).
///
/// This panel shows query results in the bottom dock. It supports:
/// - Empty state (no query executed)
/// - Loading state with spinner during execution
/// - Streaming state showing partial results
/// - Success state showing complete results with timing
/// - Error state showing error message
pub struct ResultsPanel {
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// Panel state with columns, rows, status, and error.
    state: ResultsPanelState,
    /// Background task for receiving streaming events.
    _stream_task: Option<Task<()>>,
}

impl ResultsPanel {
    /// Create a new results panel.
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            state: ResultsPanelState::default(),
            _stream_task: None,
        }
    }

    /// Get the current state.
    pub fn state(&self) -> &ResultsPanelState {
        &self.state
    }

    /// Get the current status.
    pub fn status(&self) -> &ResultsStatus {
        &self.state.status
    }

    /// Get column count.
    pub fn column_count(&self) -> usize {
        self.state.columns.len()
    }

    /// Get row count.
    pub fn row_count(&self) -> usize {
        self.state.rows.len()
    }

    /// Set the panel to loading state.
    pub fn set_loading(&mut self, cx: &mut Context<Self>) {
        self.state.set_loading();
        cx.notify();
    }

    /// Set the panel to error state.
    pub fn set_error(&mut self, message: impl Into<String>, cx: &mut Context<Self>) {
        self.state.status = ResultsStatus::Error;
        self.state.error = Some(DisplayError {
            message: message.into(),
            hint: None,
            code: None,
            position: None,
            is_cancelled: false,
        });
        cx.notify();
    }

    /// Clear the panel back to empty state.
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.state.clear();
        self._stream_task = None;
        cx.notify();
    }

    /// Start receiving streaming query events (FR-011, FR-012).
    ///
    /// This method:
    /// 1. Clears previous results
    /// 2. Sets status to Loading
    /// 3. Spawns a background task to receive QueryEvents
    /// 4. Updates the UI as events arrive
    #[cfg(feature = "persistence")]
    pub fn start_streaming(&mut self, mut rx: mpsc::Receiver<QueryEvent>, cx: &mut Context<Self>) {
        // Clear and set to loading
        self.state.set_loading();

        // Spawn background task to receive events
        self._stream_task = Some(cx.spawn(async move |this, cx| {
            while let Some(event) = rx.recv().await {
                let is_terminal = event.is_terminal();

                // Update state with the event
                let result = this.update(cx, |panel: &mut ResultsPanel, cx| {
                    panel.handle_event(event, cx);
                });

                if result.is_err() || is_terminal {
                    break;
                }
            }
        }));

        cx.notify();
    }

    /// Start streaming placeholder for non-persistence builds.
    #[cfg(not(feature = "persistence"))]
    pub fn start_streaming<T>(&mut self, _rx: T, cx: &mut Context<Self>) {
        self.state.status = ResultsStatus::Error;
        self.state.error = Some(DisplayError {
            message: "Streaming requires persistence feature".to_string(),
            hint: None,
            code: None,
            is_cancelled: false,
        });
        cx.notify();
    }

    /// Handle a query event (FR-014, FR-015).
    #[cfg(feature = "persistence")]
    pub fn handle_event(&mut self, event: QueryEvent, cx: &mut Context<Self>) {
        match event {
            QueryEvent::Columns(columns) => {
                self.state.columns = columns.into_iter().map(DisplayColumn::from).collect();
                self.state.status = ResultsStatus::Streaming;
                tracing::debug!(column_count = self.state.columns.len(), "Received columns");
            }
            QueryEvent::Rows { rows, total_so_far } => {
                // Convert tokio_postgres::Row to DisplayRow
                for row in rows {
                    let cells: Vec<String> = (0..self.state.columns.len())
                        .map(|i| Self::format_cell(&row, i))
                        .collect();
                    self.state.rows.push(DisplayRow { cells });
                }
                self.state.total_rows = total_so_far;
                tracing::trace!(total_rows = total_so_far, "Received rows batch");
            }
            QueryEvent::Progress { rows_so_far } => {
                self.state.total_rows = rows_so_far;
            }
            QueryEvent::Complete { total_rows, execution_time_ms, rows_affected } => {
                self.state.total_rows = total_rows;
                self.state.execution_time_ms = Some(execution_time_ms);
                self.state.rows_affected = rows_affected;
                self.state.status = ResultsStatus::Complete;
                tracing::debug!(
                    total_rows,
                    execution_time_ms,
                    rows_affected = ?rows_affected,
                    "Query completed"
                );
            }
            QueryEvent::Error(err) => {
                let display_error = DisplayError::from(err);

                // T035: Preserve already-received results when query is cancelled
                if display_error.is_cancelled {
                    // Keep existing results, just mark as complete (cancelled)
                    // Don't clear columns or rows
                    self.state.status = ResultsStatus::Complete;
                    self.state.error = Some(display_error);
                    tracing::debug!(
                        rows_preserved = self.state.rows.len(),
                        "Query cancelled, preserving received results"
                    );
                } else {
                    // Regular error - show error state
                    self.state.status = ResultsStatus::Error;
                    self.state.error = Some(display_error);
                    tracing::debug!("Query error received");
                }
            }
        }
        cx.notify();
    }

    /// Format a cell value from a tokio_postgres::Row.
    #[cfg(feature = "persistence")]
    fn format_cell(row: &tokio_postgres::Row, index: usize) -> String {
        use tokio_postgres::types::Type;

        let column = &row.columns()[index];
        let type_ = column.type_();

        // Handle NULL values
        if row.try_get::<_, Option<String>>(index).ok().flatten().is_none() {
            // Try to detect if it's actually NULL vs a type mismatch
            // For simplicity, try common types
            match *type_ {
                Type::BOOL => {
                    if let Ok(Some(v)) = row.try_get::<_, Option<bool>>(index) {
                        return v.to_string();
                    }
                }
                Type::INT2 => {
                    if let Ok(Some(v)) = row.try_get::<_, Option<i16>>(index) {
                        return v.to_string();
                    }
                }
                Type::INT4 => {
                    if let Ok(Some(v)) = row.try_get::<_, Option<i32>>(index) {
                        return v.to_string();
                    }
                }
                Type::INT8 => {
                    if let Ok(Some(v)) = row.try_get::<_, Option<i64>>(index) {
                        return v.to_string();
                    }
                }
                Type::FLOAT4 => {
                    if let Ok(Some(v)) = row.try_get::<_, Option<f32>>(index) {
                        return v.to_string();
                    }
                }
                Type::FLOAT8 => {
                    if let Ok(Some(v)) = row.try_get::<_, Option<f64>>(index) {
                        return v.to_string();
                    }
                }
                Type::TEXT | Type::VARCHAR | Type::CHAR | Type::NAME => {
                    if let Ok(Some(v)) = row.try_get::<_, Option<String>>(index) {
                        return v;
                    }
                }
                _ => {}
            }
            // If we couldn't get a value, it's likely NULL
            return "NULL".to_string();
        }

        // Non-NULL string value
        row.try_get::<_, Option<String>>(index)
            .ok()
            .flatten()
            .unwrap_or_else(|| "NULL".to_string())
    }

    /// Render the empty state.
    fn render_empty_state(&self, theme: &TuskTheme) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap(px(12.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(48.0))
                    .rounded(px(8.0))
                    .bg(theme.colors.element_background)
                    .child(
                        Icon::new(IconName::Table)
                            .size(IconSize::XLarge)
                            .color(theme.colors.text_muted),
                    ),
            )
            .child(
                div().text_color(theme.colors.text_muted).text_size(px(13.0)).child("No results"),
            )
            .child(
                div()
                    .text_color(theme.colors.text_muted)
                    .text_size(px(12.0))
                    .child("Execute a query to see results here"),
            )
    }

    /// Render the loading state.
    fn render_loading_state(&self, theme: &TuskTheme) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap(px(12.0))
            .child(Spinner::new().size(SpinnerSize::Large))
            .child(
                div()
                    .text_color(theme.colors.text_muted)
                    .text_size(px(13.0))
                    .child("Executing query..."),
            )
    }

    /// Render the streaming/complete state with results.
    fn render_results_state(&self, theme: &TuskTheme) -> impl IntoElement {
        let is_streaming = self.state.status.is_streaming();
        let row_count = self.state.rows.len();
        let total_rows = self.state.total_rows;
        let was_cancelled = self.state.error.as_ref().map(|e| e.is_cancelled).unwrap_or(false);

        div()
            .flex()
            .flex_col()
            .size_full()
            // Results header with column names
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(px(28.0))
                    .px(px(8.0))
                    .bg(theme.colors.element_background)
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .children(self.state.columns.iter().map(|col| {
                        div()
                            .flex_1()
                            .min_w(px(100.0))
                            .px(px(8.0))
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.colors.text_muted)
                            .overflow_hidden()
                            .child(col.name.clone())
                    })),
            )
            // Results body with rows (simplified - no virtualization yet)
            .child(
                div()
                    .id("results-body")
                    .flex_1()
                    .overflow_y_scroll()
                    .children(self.state.rows.iter().take(100).enumerate().map(|(i, row)| {
                        let bg = if i % 2 == 0 {
                            theme.colors.panel_background
                        } else {
                            theme.colors.element_background
                        };
                        div()
                            .flex()
                            .items_center()
                            .h(px(24.0))
                            .px(px(8.0))
                            .bg(bg)
                            .children(row.cells.iter().map(|cell| {
                                div()
                                    .flex_1()
                                    .min_w(px(100.0))
                                    .px(px(8.0))
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text)
                                    .overflow_hidden()
                                    .child(cell.clone())
                            }))
                    })),
            )
            // Status bar
            .child(
                div()
                    .h(px(24.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .px(px(12.0))
                    .gap(px(16.0))
                    .border_t_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.panel_background)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .when(is_streaming, |s| s.child(Spinner::new().size(SpinnerSize::Small)))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .child(if is_streaming {
                                        format!("Streaming... {} rows", total_rows)
                                    } else if was_cancelled {
                                        // T035: Show cancelled status with preserved row count
                                        format!(
                                            "{} row{} (cancelled)",
                                            row_count,
                                            if row_count == 1 { "" } else { "s" }
                                        )
                                    } else if row_count > 100 {
                                        format!("{} rows (showing first 100)", total_rows)
                                    } else {
                                        format!(
                                            "{} row{}",
                                            total_rows,
                                            if total_rows == 1 { "" } else { "s" }
                                        )
                                    }),
                            ),
                    )
                    .when(self.state.execution_time_ms.is_some(), |s| {
                        s.child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted)
                                .child(format!(
                                    "{}ms",
                                    self.state.execution_time_ms.unwrap_or(0)
                                )),
                        )
                    })
                    .when(self.state.rows_affected.is_some(), |s| {
                        s.child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted)
                                .child(format!(
                                    "{} affected",
                                    self.state.rows_affected.unwrap_or(0)
                                )),
                        )
                    }),
            )
    }

    /// Render the error state.
    fn render_error_state(&self, theme: &TuskTheme) -> impl IntoElement {
        let error = self.state.error.as_ref();
        let message = error.map(|e| e.message.as_str()).unwrap_or("Unknown error");
        let hint = error.and_then(|e| e.hint.as_ref());
        let code = error.and_then(|e| e.code.as_ref());

        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap(px(12.0))
            .p(px(16.0))
            .child(Icon::new(IconName::Error).size(IconSize::XLarge).color(theme.colors.error))
            .child(
                div()
                    .text_color(theme.colors.error)
                    .text_size(px(13.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child("Query failed"),
            )
            .child(
                div()
                    .text_color(theme.colors.text_muted)
                    .text_size(px(12.0))
                    .text_center()
                    .max_w(px(400.0))
                    .child(message.to_string()),
            )
            .when_some(code.cloned(), |s, c| {
                s.child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.colors.text_muted)
                        .child(format!("Error code: {}", c)),
                )
            })
            .when_some(hint.cloned(), |s, h| {
                s.child(
                    div()
                        .mt(px(8.0))
                        .px(px(12.0))
                        .py(px(8.0))
                        .rounded(px(4.0))
                        .bg(theme.colors.element_background)
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text)
                                .child(format!("Hint: {}", h)),
                        ),
                )
            })
    }
}

impl EventEmitter<PanelEvent> for ResultsPanel {}

impl Focusable for ResultsPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for ResultsPanel {
    fn panel_id(&self) -> &'static str {
        "results"
    }

    fn title(&self, _cx: &App) -> SharedString {
        "Results".into()
    }

    fn icon(&self, _cx: &App) -> IconName {
        IconName::Table
    }

    fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
    }

    fn closable(&self, _cx: &App) -> bool {
        false // Results panel is always visible when bottom dock is open
    }

    fn position(&self, _cx: &App) -> DockPosition {
        DockPosition::Bottom
    }
}

impl Render for ResultsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        let content = match &self.state.status {
            ResultsStatus::Empty => self.render_empty_state(theme).into_any_element(),
            ResultsStatus::Loading => self.render_loading_state(theme).into_any_element(),
            ResultsStatus::Streaming | ResultsStatus::Complete => {
                self.render_results_state(theme).into_any_element()
            }
            ResultsStatus::Error => self.render_error_state(theme).into_any_element(),
        };

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.panel_background)
            .child(
                // Panel header
                div()
                    .h(px(32.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(Icon::new(IconName::Table).size(IconSize::Small))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text)
                                    .child("Results"),
                            ),
                    ),
            )
            .child(
                // Panel content
                div().flex_1().overflow_hidden().child(content),
            )
    }
}

// Keep the old ResultsState for backwards compatibility
pub use ResultsStatus as ResultsState;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_results_status_default() {
        let status = ResultsStatus::default();
        assert!(status.is_empty());
    }

    #[test]
    fn test_results_panel_state_default() {
        let state = ResultsPanelState::default();
        assert!(state.columns.is_empty());
        assert!(state.rows.is_empty());
        assert_eq!(state.total_rows, 0);
        assert!(state.execution_time_ms.is_none());
        assert!(state.status.is_empty());
    }

    #[test]
    fn test_results_panel_state_clear() {
        let mut state = ResultsPanelState::default();
        state.columns.push(DisplayColumn { name: "id".to_string(), type_name: "int4".to_string() });
        state.total_rows = 100;
        state.status = ResultsStatus::Complete;

        state.clear();

        assert!(state.columns.is_empty());
        assert_eq!(state.total_rows, 0);
        assert!(state.status.is_empty());
    }
}
