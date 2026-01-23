//! SQL query editor component with service integration.
//!
//! The QueryEditor provides:
//! - SQL text editing (placeholder until full editor is implemented)
//! - Query execution via TuskState (FR-010)
//! - Streaming results to ResultsPanel (FR-011, FR-012)
//! - Query cancellation support (FR-013)

use gpui::{
    div, prelude::*, px, App, Context, Entity, FocusHandle, Focusable, Render, Task, Window,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::icon::{Icon, IconName, IconSize};
use crate::key_bindings::{CancelQuery, RunQuery};
use crate::panels::{Message, MessagesPanel, ResultsPanel};
use crate::spinner::{Spinner, SpinnerSize};
use crate::text_input::{TextInput, TextInputEvent};
use crate::TuskTheme;

#[cfg(feature = "persistence")]
use tusk_core::{QueryHandle, TuskState};

#[cfg(feature = "persistence")]
use tokio::sync::mpsc;

/// Status of the query editor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum QueryEditorStatus {
    /// Ready for input, no query executing.
    #[default]
    Idle,
    /// Query is currently executing.
    Executing,
    /// Query cancellation has been requested.
    Cancelled,
}

impl QueryEditorStatus {
    /// Check if the editor is idle.
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    /// Check if a query is executing.
    pub fn is_executing(&self) -> bool {
        matches!(self, Self::Executing)
    }

    /// Check if cancellation is in progress.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }
}

/// State for the query editor component.
pub struct QueryEditorState {
    /// Current connection ID for this editor.
    pub connection_id: Option<Uuid>,
    /// Currently executing query handle (if any).
    #[cfg(feature = "persistence")]
    pub active_query: Option<Arc<QueryHandle>>,
    #[cfg(not(feature = "persistence"))]
    pub active_query: Option<()>,
    /// Current execution status.
    pub status: QueryEditorStatus,
}

impl Default for QueryEditorState {
    fn default() -> Self {
        Self { connection_id: None, active_query: None, status: QueryEditorStatus::Idle }
    }
}

impl QueryEditorState {
    /// Create a new editor state with a connection.
    pub fn with_connection(connection_id: Uuid) -> Self {
        Self {
            connection_id: Some(connection_id),
            active_query: None,
            status: QueryEditorStatus::Idle,
        }
    }

    /// Check if the editor has an active connection.
    pub fn has_connection(&self) -> bool {
        self.connection_id.is_some()
    }

    /// Check if a query is currently executing.
    pub fn is_executing(&self) -> bool {
        self.status.is_executing()
    }
}

/// SQL query editor component.
///
/// This component provides SQL editing and execution capabilities.
/// It integrates with TuskState for query execution and ResultsPanel
/// for displaying streaming results.
pub struct QueryEditor {
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// Editor state (connection, active query, status).
    state: QueryEditorState,
    /// Current SQL content.
    content: String,
    /// SQL input field.
    sql_input: Entity<TextInput>,
    /// Results panel to send query results to.
    results_panel: Option<Entity<ResultsPanel>>,
    /// Messages panel for notifications (e.g., query cancelled).
    messages_panel: Option<Entity<MessagesPanel>>,
    /// Background task for query execution (dropped on new query = automatic cancellation).
    _execution_task: Option<Task<()>>,
}

impl QueryEditor {
    /// Create a new query editor.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let sql_input =
            cx.new(|cx| TextInput::new("Enter SQL query (e.g., SELECT * FROM users)", cx));

        // Subscribe to text input changes
        cx.subscribe(&sql_input, Self::on_sql_input_event).detach();

        Self {
            focus_handle: cx.focus_handle(),
            state: QueryEditorState::default(),
            content: String::new(),
            sql_input,
            results_panel: None,
            messages_panel: None,
            _execution_task: None,
        }
    }

    /// Create a new query editor with a connection.
    pub fn with_connection(connection_id: Uuid, cx: &mut Context<Self>) -> Self {
        let sql_input =
            cx.new(|cx| TextInput::new("Enter SQL query (e.g., SELECT * FROM users)", cx));

        // Subscribe to text input changes
        cx.subscribe(&sql_input, Self::on_sql_input_event).detach();

        Self {
            focus_handle: cx.focus_handle(),
            state: QueryEditorState::with_connection(connection_id),
            content: String::new(),
            sql_input,
            results_panel: None,
            messages_panel: None,
            _execution_task: None,
        }
    }

    /// Handle events from the SQL input field.
    fn on_sql_input_event(
        &mut self,
        _input: Entity<TextInput>,
        event: &TextInputEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            TextInputEvent::Changed(text) => {
                self.content = text.clone();
                cx.notify();
            }
            TextInputEvent::Submitted(_) => {
                // Execute query on Enter (in addition to Cmd+Enter)
                self.execute_query(cx);
            }
            _ => {}
        }
    }

    /// Set the results panel to receive query results.
    pub fn set_results_panel(&mut self, panel: Entity<ResultsPanel>) {
        self.results_panel = Some(panel);
    }

    /// Set the messages panel for notifications.
    pub fn set_messages_panel(&mut self, panel: Entity<MessagesPanel>) {
        self.messages_panel = Some(panel);
    }

    /// Get the current connection ID.
    pub fn connection_id(&self) -> Option<Uuid> {
        self.state.connection_id
    }

    /// Set the connection ID for this editor.
    pub fn set_connection_id(&mut self, connection_id: Uuid, _cx: &mut Context<Self>) {
        self.state.connection_id = Some(connection_id);
    }

    /// Get the current SQL content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Set the SQL content.
    pub fn set_content(&mut self, content: impl Into<String>, cx: &mut Context<Self>) {
        self.content = content.into();
        cx.notify();
    }

    /// Get the current status.
    pub fn status(&self) -> &QueryEditorStatus {
        &self.state.status
    }

    /// Check if a query is executing.
    pub fn is_executing(&self) -> bool {
        self.state.is_executing()
    }

    /// Execute the current SQL query (FR-010, FR-011, FR-012).
    ///
    /// This method:
    /// 1. Creates an mpsc channel for streaming results
    /// 2. Starts the results panel streaming
    /// 3. Spawns the query execution on the tokio runtime
    /// 4. Stores the query handle for cancellation support
    #[cfg(feature = "persistence")]
    pub fn execute_query(&mut self, cx: &mut Context<Self>) {
        use tusk_core::services::QueryService;

        // Validate we have a connection
        let Some(connection_id) = self.state.connection_id else {
            tracing::warn!("Cannot execute query: no connection");
            return;
        };

        // Get the SQL to execute
        let sql = self.content.clone();
        if sql.trim().is_empty() {
            tracing::debug!("Cannot execute query: empty SQL");
            return;
        }

        // Access TuskState synchronously to get what we need
        let Some(state) = cx.try_global::<TuskState>() else {
            tracing::error!("TuskState not available");
            return;
        };

        // Get the connection pool and runtime handle (both are Clone)
        let Some(pool) = state.get_connection(&connection_id) else {
            tracing::warn!(connection_id = %connection_id, "Connection not found");
            return;
        };
        let runtime_handle = state.runtime().handle().clone();

        // Create and register query handle
        let handle = QueryHandle::new(connection_id, sql.clone());
        let handle = state.register_query(handle);
        self.state.active_query = Some(handle.clone());

        // Update status to executing
        self.state.status = QueryEditorStatus::Executing;

        // Create channel for streaming results (bounded with backpressure)
        let (tx, rx) = mpsc::channel(100);

        // Start the results panel streaming
        if let Some(results_panel) = &self.results_panel {
            results_panel.update(cx, |panel, cx| {
                panel.start_streaming(rx, cx);
            });
        }

        // Spawn the query execution task
        // Replacing _execution_task will drop the old task, automatically cancelling it
        self._execution_task = Some(cx.spawn(async move |this, cx| {
            // Execute the query with streaming using QueryService directly
            let result = runtime_handle
                .spawn(async move {
                    // Get a connection from the pool
                    let conn = pool.get().await?;
                    QueryService::execute_streaming(&conn, &sql, &handle, tx).await
                })
                .await;

            // Update the editor when query completes
            let _ = this.update(cx, |editor: &mut QueryEditor, cx| {
                match result {
                    Ok(Ok(())) => {
                        tracing::debug!("Query execution completed");
                        editor.state.status = QueryEditorStatus::Idle;
                    }
                    Ok(Err(e)) => {
                        // Check if this is a connection lost error (T049)
                        if e.is_connection_lost() {
                            tracing::warn!(error = %e, "Connection lost during query execution");
                            // Show connection lost message in messages panel
                            if let Some(messages_panel) = &editor.messages_panel {
                                messages_panel.update(cx, |panel, cx| {
                                    panel.add_message(
                                        Message::error("Connection lost during query execution"),
                                        cx,
                                    );
                                    panel.add_message(
                                        Message::info("Try reconnecting to the database"),
                                        cx,
                                    );
                                });
                            }
                        } else {
                            tracing::warn!(error = %e, "Query execution failed");
                        }
                        editor.state.status = QueryEditorStatus::Idle;
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Query task panicked");
                        editor.state.status = QueryEditorStatus::Idle;
                    }
                }
                cx.notify();
            });
        }));

        cx.notify();
    }

    /// Execute query placeholder for non-persistence builds.
    #[cfg(not(feature = "persistence"))]
    pub fn execute_query(&mut self, cx: &mut Context<Self>) {
        tracing::warn!("Query execution requires persistence feature");
        cx.notify();
    }

    /// Cancel the currently executing query (FR-013).
    #[cfg(feature = "persistence")]
    pub fn cancel_query(&mut self, cx: &mut Context<Self>) {
        if let Some(ref handle) = self.state.active_query {
            tracing::debug!(query_id = %handle.id(), "Cancelling query");

            // Signal cancellation
            handle.cancel();

            // Update status
            self.state.status = QueryEditorStatus::Cancelled;

            // Also cancel through TuskState to send PostgreSQL cancel
            let query_id = handle.id();
            if let Some(state) = cx.try_global::<TuskState>() {
                state.cancel_query(&query_id);
            }

            // Show "Query cancelled" notification in messages panel (T036)
            if let Some(messages_panel) = &self.messages_panel {
                messages_panel.update(cx, |panel, cx| {
                    panel.add_message(Message::info("Query cancelled"), cx);
                });
            }

            cx.notify();
        }
    }

    /// Cancel query placeholder for non-persistence builds.
    #[cfg(not(feature = "persistence"))]
    pub fn cancel_query(&mut self, cx: &mut Context<Self>) {
        let _ = cx;
    }

    /// Reset the editor to idle state after query completion.
    pub fn on_query_complete(&mut self, cx: &mut Context<Self>) {
        self.state.status = QueryEditorStatus::Idle;
        self.state.active_query = None;
        self._execution_task = None;
        cx.notify();
    }

    /// Handle the RunQuery action (Cmd+Enter).
    fn on_run_query(&mut self, _: &RunQuery, _window: &mut Window, cx: &mut Context<Self>) {
        self.execute_query(cx);
    }

    /// Handle the CancelQuery action (Escape).
    fn on_cancel_query(&mut self, _: &CancelQuery, _window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_query(cx);
    }

    /// Render the toolbar with execute/cancel button.
    fn render_toolbar(&self, theme: &TuskTheme, cx: &mut Context<Self>) -> impl IntoElement {
        let is_executing = self.state.status.is_executing();
        let has_connection = self.state.connection_id.is_some();
        let has_content = !self.content.trim().is_empty();
        let can_execute = has_connection && has_content && !is_executing;

        div()
            .h(px(36.0))
            .w_full()
            .flex()
            .items_center()
            .px(px(8.0))
            .gap(px(8.0))
            .border_b_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.panel_background)
            .child(if is_executing {
                // Cancel button while executing
                div()
                    .id("cancel-button")
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .px(px(12.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .bg(theme.colors.error.opacity(0.1))
                    .hover(|s| s.bg(theme.colors.error.opacity(0.2)))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.cancel_query(cx);
                    }))
                    .child(Spinner::new().size(SpinnerSize::Small))
                    .child(div().text_size(px(12.0)).text_color(theme.colors.error).child("Cancel"))
                    .into_any_element()
            } else {
                // Execute button when idle
                div()
                    .id("execute-button")
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .px(px(12.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .when(can_execute, |s| {
                        s.bg(theme.colors.accent.opacity(0.1))
                            .hover(|s| s.bg(theme.colors.accent.opacity(0.2)))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.execute_query(cx);
                            }))
                    })
                    .when(!can_execute, |s| s.opacity(0.5).cursor_not_allowed())
                    .child(Icon::new(IconName::Play).size(IconSize::Small).color(if can_execute {
                        theme.colors.accent
                    } else {
                        theme.colors.text_muted
                    }))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(if can_execute {
                                theme.colors.accent
                            } else {
                                theme.colors.text_muted
                            })
                            .child("Execute"),
                    )
                    .into_any_element()
            })
            // Connection status indicator
            .child(div().flex_1().flex().justify_end().child(
                div().text_size(px(11.0)).text_color(theme.colors.text_muted).child(
                    if has_connection {
                        "Connected".to_string()
                    } else {
                        "Not connected".to_string()
                    },
                ),
            ))
    }

    /// Render the editor content area.
    fn render_content(&self, theme: &TuskTheme) -> impl IntoElement {
        div().flex_1().p(px(12.0)).bg(theme.colors.editor_background).child(
            div()
                .size_full()
                .flex()
                .flex_col()
                .gap(px(8.0))
                // SQL input field
                .child(self.sql_input.clone())
                // Help text
                .child(div().text_color(theme.colors.text_muted).text_size(px(11.0)).child(
                    if cfg!(target_os = "macos") {
                        "Press Cmd+Enter to execute, or click Execute button"
                    } else {
                        "Press Ctrl+Enter to execute, or click Execute button"
                    },
                )),
        )
    }
}

impl Focusable for QueryEditor {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for QueryEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>().clone();

        div()
            .id("query-editor")
            .key_context("QueryEditor")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_run_query))
            .on_action(cx.listener(Self::on_cancel_query))
            .size_full()
            .flex()
            .flex_col()
            .child(self.render_toolbar(&theme, cx))
            .child(self.render_content(&theme))
    }
}
