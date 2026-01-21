//! Status bar component for displaying connection and execution state.
//!
//! The status bar sits at the bottom of the workspace and shows:
//! - Connection status (left side)
//! - Execution state and timing (right side)

use gpui::{div, prelude::*, px, App, IntoElement, RenderOnce, SharedString, Window};

use crate::icon::{Icon, IconName, IconSize};
use crate::layout::sizes::STATUS_BAR_HEIGHT;
use crate::spinner::{Spinner, SpinnerSize};
use crate::TuskTheme;

/// Connection status for the status bar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConnectionStatus {
    /// Not connected to any database.
    #[default]
    Disconnected,
    /// Connected to a database.
    Connected {
        /// Database name.
        database: SharedString,
        /// Server host.
        host: SharedString,
    },
    /// Currently connecting.
    Connecting,
    /// Connection failed.
    Error(SharedString),
}

/// Execution state for the status bar.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ExecutionState {
    /// Idle, no query running.
    #[default]
    Idle,
    /// Query is executing.
    Executing,
    /// Query completed with results.
    Completed {
        /// Number of rows returned.
        rows: usize,
        /// Execution time in milliseconds.
        elapsed_ms: u64,
    },
    /// Query failed with error.
    Failed(SharedString),
}

/// Status bar component displaying connection and execution state.
#[derive(IntoElement)]
pub struct StatusBar {
    /// Current connection status.
    connection_status: ConnectionStatus,
    /// Current execution state.
    execution_state: ExecutionState,
}

impl StatusBar {
    /// Create a new status bar with default state.
    pub fn new() -> Self {
        Self {
            connection_status: ConnectionStatus::default(),
            execution_state: ExecutionState::default(),
        }
    }

    /// Set the connection status.
    pub fn connection_status(mut self, status: ConnectionStatus) -> Self {
        self.connection_status = status;
        self
    }

    /// Set the execution state.
    pub fn execution_state(mut self, state: ExecutionState) -> Self {
        self.execution_state = state;
        self
    }

    /// Render the connection status section (left side).
    fn render_connection_status(&self, theme: &TuskTheme) -> impl IntoElement {
        let (icon, text, color): (IconName, String, gpui::Hsla) = match &self.connection_status {
            ConnectionStatus::Disconnected => (
                IconName::Database,
                "Not connected".to_string(),
                theme.colors.text_muted,
            ),
            ConnectionStatus::Connected { database, host } => (
                IconName::Database,
                format!("{} @ {}", database, host),
                theme.colors.success,
            ),
            ConnectionStatus::Connecting => (
                IconName::Database,
                "Connecting...".to_string(),
                theme.colors.warning,
            ),
            ConnectionStatus::Error(msg) => (
                IconName::Database,
                format!("Error: {}", msg),
                theme.colors.error,
            ),
        };

        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(Icon::new(icon).size(IconSize::Small).color(color))
            .child(
                div()
                    .text_color(color)
                    .child(text),
            )
    }

    /// Render the execution state section (right side).
    fn render_execution_state(&self, theme: &TuskTheme) -> impl IntoElement {
        match &self.execution_state {
            ExecutionState::Idle => {
                div()
                    .flex()
                    .items_center()
                    .text_color(theme.colors.text_muted)
                    .child("Ready")
            }
            ExecutionState::Executing => {
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .text_color(theme.colors.accent)
                    .child(Spinner::new().size(SpinnerSize::Small))
                    .child("Executing...")
            }
            ExecutionState::Completed { rows, elapsed_ms } => {
                let row_text = if *rows == 1 { "row" } else { "rows" };
                let elapsed = format_elapsed(*elapsed_ms);

                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .text_color(theme.colors.success)
                            .child(Icon::new(IconName::Check).size(IconSize::Small).color(theme.colors.success))
                            .child(format!("{} {}", rows, row_text)),
                    )
                    .child(
                        div()
                            .text_color(theme.colors.text_muted)
                            .child(elapsed),
                    )
            }
            ExecutionState::Failed(msg) => {
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .text_color(theme.colors.error)
                    .child(Icon::new(IconName::Warning).size(IconSize::Small).color(theme.colors.error))
                    .child(msg.clone())
            }
        }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for StatusBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        div()
            .h(STATUS_BAR_HEIGHT)
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(12.0))
            .bg(theme.colors.status_bar_background)
            .border_t_1()
            .border_color(theme.colors.border)
            .text_size(px(12.0))
            // Left side: connection status
            .child(self.render_connection_status(theme))
            // Right side: execution state
            .child(self.render_execution_state(theme))
    }
}

/// Format elapsed time in a human-readable way.
fn format_elapsed(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        let secs = ms as f64 / 1000.0;
        format!("{:.2}s", secs)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{}m {}s", mins, secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_elapsed() {
        assert_eq!(format_elapsed(50), "50ms");
        assert_eq!(format_elapsed(1500), "1.50s");
        assert_eq!(format_elapsed(65000), "1m 5s");
    }

    #[test]
    fn test_status_bar_construction() {
        let status_bar = StatusBar::new()
            .connection_status(ConnectionStatus::Connected {
                database: "postgres".into(),
                host: "localhost".into(),
            })
            .execution_state(ExecutionState::Completed {
                rows: 100,
                elapsed_ms: 150,
            });

        assert!(matches!(
            status_bar.connection_status,
            ConnectionStatus::Connected { .. }
        ));
        assert!(matches!(
            status_bar.execution_state,
            ExecutionState::Completed { .. }
        ));
    }
}
