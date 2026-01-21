//! Results panel for displaying query results.
//!
//! The results panel lives in the bottom dock and displays query output
//! including data grids, affected row counts, and execution information.

use gpui::{
    div, prelude::*, px, App, Context, EventEmitter, FocusHandle, Render, SharedString, Window,
};

use crate::icon::{Icon, IconName, IconSize};
use crate::panel::{DockPosition, Focusable, Panel, PanelEvent};
use crate::spinner::{Spinner, SpinnerSize};
use crate::TuskTheme;

/// State of the results panel.
#[derive(Debug, Clone, Default)]
pub enum ResultsState {
    /// No query has been executed yet.
    #[default]
    Empty,
    /// Query is currently executing.
    Loading,
    /// Query completed successfully with results.
    Success {
        /// Number of rows returned or affected.
        row_count: usize,
        /// Execution time in milliseconds.
        elapsed_ms: u64,
    },
    /// Query failed with an error.
    Error {
        /// Error message.
        message: String,
    },
}

/// Results panel for displaying query output.
///
/// This panel shows query results in the bottom dock. It supports:
/// - Empty state (no query executed)
/// - Loading state with spinner during execution
/// - Success state showing row count and timing
/// - Error state showing error message
pub struct ResultsPanel {
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// Current state of the results panel.
    state: ResultsState,
}

impl ResultsPanel {
    /// Create a new results panel.
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            state: ResultsState::Empty,
        }
    }

    /// Get the current state.
    pub fn state(&self) -> &ResultsState {
        &self.state
    }

    /// Set the panel to loading state.
    pub fn set_loading(&mut self, cx: &mut Context<Self>) {
        self.state = ResultsState::Loading;
        cx.notify();
    }

    /// Set the panel to success state with results.
    pub fn set_success(&mut self, row_count: usize, elapsed_ms: u64, cx: &mut Context<Self>) {
        self.state = ResultsState::Success {
            row_count,
            elapsed_ms,
        };
        cx.notify();
    }

    /// Set the panel to error state.
    pub fn set_error(&mut self, message: impl Into<String>, cx: &mut Context<Self>) {
        self.state = ResultsState::Error {
            message: message.into(),
        };
        cx.notify();
    }

    /// Clear the panel back to empty state.
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.state = ResultsState::Empty;
        cx.notify();
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
                div()
                    .text_color(theme.colors.text_muted)
                    .text_size(px(13.0))
                    .child("No results"),
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

    /// Render the success state.
    fn render_success_state(
        &self,
        row_count: usize,
        elapsed_ms: u64,
        theme: &TuskTheme,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap(px(12.0))
            .child(
                Icon::new(IconName::Check)
                    .size(IconSize::XLarge)
                    .color(theme.colors.success),
            )
            .child(
                div()
                    .text_color(theme.colors.text)
                    .text_size(px(13.0))
                    .child(format!(
                        "{} row{} returned",
                        row_count,
                        if row_count == 1 { "" } else { "s" }
                    )),
            )
            .child(
                div()
                    .text_color(theme.colors.text_muted)
                    .text_size(px(12.0))
                    .child(format!("Query completed in {}ms", elapsed_ms)),
            )
    }

    /// Render the error state.
    fn render_error_state(&self, message: &str, theme: &TuskTheme) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap(px(12.0))
            .p(px(16.0))
            .child(
                Icon::new(IconName::Error)
                    .size(IconSize::XLarge)
                    .color(theme.colors.error),
            )
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

        let content = match &self.state {
            ResultsState::Empty => self.render_empty_state(theme).into_any_element(),
            ResultsState::Loading => self.render_loading_state(theme).into_any_element(),
            ResultsState::Success {
                row_count,
                elapsed_ms,
            } => self
                .render_success_state(*row_count, *elapsed_ms, theme)
                .into_any_element(),
            ResultsState::Error { message } => {
                self.render_error_state(message, theme).into_any_element()
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_results_state_default() {
        let state = ResultsState::default();
        assert!(matches!(state, ResultsState::Empty));
    }
}
