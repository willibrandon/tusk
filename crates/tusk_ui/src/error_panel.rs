//! Error panel component for detailed error display (T062, T064).
//!
//! The error panel provides a detailed view of errors including:
//! - Error type header
//! - Error message
//! - Position indicator (for query errors)
//! - Technical detail (expandable)
//! - Hint (actionable suggestion)
//! - PostgreSQL error code (if available)
//!
//! Display rules per error-handling.md:
//! - Query errors with position: Show in error panel
//! - Non-recoverable errors: Show in error panel/modal
//! - Includes "Show Details" expansion for technical detail

use gpui::{div, prelude::*, px, App, Context, FocusHandle, Render, SharedString, Window};

use crate::icon::{Icon, IconName, IconSize};
use crate::panel::Focusable;
use crate::TuskTheme;

#[cfg(feature = "persistence")]
use tusk_core::error::ErrorInfo;

/// Error panel for detailed error display.
///
/// This panel is used for query errors with position information
/// and non-recoverable errors that need more detail than a toast.
pub struct ErrorPanel {
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// Current error to display (if any).
    error: Option<ErrorPanelContent>,
    /// Whether the technical detail section is expanded.
    detail_expanded: bool,
}

/// Content for the error panel.
#[derive(Debug, Clone)]
pub struct ErrorPanelContent {
    /// Error type/category (e.g., "Query Error").
    pub error_type: SharedString,
    /// User-friendly error message.
    pub message: SharedString,
    /// Actionable suggestion for the user.
    pub hint: Option<SharedString>,
    /// Technical detail for debugging (shown in expandable section).
    pub technical_detail: Option<SharedString>,
    /// Character position for query errors (1-indexed).
    pub position: Option<usize>,
    /// PostgreSQL error code (e.g., "42P01").
    pub code: Option<SharedString>,
}

impl ErrorPanelContent {
    /// Create new error panel content.
    pub fn new(error_type: impl Into<SharedString>, message: impl Into<SharedString>) -> Self {
        Self {
            error_type: error_type.into(),
            message: message.into(),
            hint: None,
            technical_detail: None,
            position: None,
            code: None,
        }
    }

    /// Set the hint.
    pub fn with_hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Set the technical detail.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.technical_detail = Some(detail.into());
        self
    }

    /// Set the position.
    pub fn with_position(mut self, position: usize) -> Self {
        self.position = Some(position);
        self
    }

    /// Set the PostgreSQL error code.
    pub fn with_code(mut self, code: impl Into<SharedString>) -> Self {
        self.code = Some(code.into());
        self
    }
}

#[cfg(feature = "persistence")]
impl From<ErrorInfo> for ErrorPanelContent {
    fn from(info: ErrorInfo) -> Self {
        Self {
            error_type: info.error_type.into(),
            message: info.message.into(),
            hint: info.hint.map(Into::into),
            technical_detail: info.technical_detail.map(Into::into),
            position: info.position,
            code: info.code.map(Into::into),
        }
    }
}

impl ErrorPanel {
    /// Create a new error panel.
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle(), error: None, detail_expanded: false }
    }

    /// Show an error in the panel.
    pub fn show_error(&mut self, content: ErrorPanelContent, cx: &mut Context<Self>) {
        self.error = Some(content);
        self.detail_expanded = false;
        cx.notify();
    }

    /// Show an error from ErrorInfo.
    #[cfg(feature = "persistence")]
    pub fn show_error_info(&mut self, info: ErrorInfo, cx: &mut Context<Self>) {
        self.show_error(ErrorPanelContent::from(info), cx);
    }

    /// Clear the error.
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        self.detail_expanded = false;
        cx.notify();
    }

    /// Check if there's an error to display.
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    /// Toggle the detail expansion.
    fn toggle_detail(&mut self, cx: &mut Context<Self>) {
        self.detail_expanded = !self.detail_expanded;
        cx.notify();
    }
}

impl Focusable for ErrorPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ErrorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Clone colors upfront to avoid borrow issues with cx
        let theme = cx.global::<TuskTheme>();
        let panel_bg = theme.colors.panel_background;
        let text_muted = theme.colors.text_muted;
        let text_color = theme.colors.text;
        let border_color = theme.colors.border;
        let error_color = theme.colors.error;
        let accent_color = theme.colors.accent;
        let element_bg = theme.colors.element_background;

        let Some(error) = &self.error else {
            // No error - render empty state
            return div()
                .track_focus(&self.focus_handle)
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(panel_bg)
                .child(div().text_size(px(12.0)).text_color(text_muted).child("No errors"))
                .into_any_element();
        };

        // Clone error fields for closures
        let error_type = error.error_type.clone();
        let error_message = error.message.clone();
        let error_code = error.code.clone();
        let error_hint = error.hint.clone();
        let error_position = error.position;
        let error_detail = error.technical_detail.clone();
        let is_expanded = self.detail_expanded;
        let chevron_icon = if is_expanded { IconName::ChevronDown } else { IconName::ChevronRight };

        // Render error content
        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(panel_bg)
            // Header with error type and code
            .child(
                div()
                    .h(px(40.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(16.0))
                    .border_b_1()
                    .border_color(border_color)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                Icon::new(IconName::Error)
                                    .size(IconSize::Medium)
                                    .color(error_color),
                            )
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(error_color)
                                    .child(error_type),
                            ),
                    )
                    // Error code badge (T064)
                    .when_some(error_code.clone(), |s, code| {
                        s.child(
                            div()
                                .px(px(8.0))
                                .py(px(4.0))
                                .rounded(px(4.0))
                                .bg(error_color.opacity(0.1))
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(error_color)
                                        .child(code.to_string()),
                                ),
                        )
                    }),
            )
            // Content area
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    .p(px(16.0))
                    // Error message
                    .child(div().text_size(px(13.0)).text_color(text_color).child(error_message))
                    // Position indicator (T063)
                    .when_some(error_position, |s, pos| {
                        s.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .px(px(12.0))
                                .py(px(8.0))
                                .rounded(px(4.0))
                                .bg(element_bg)
                                .child(
                                    Icon::new(IconName::ChevronRight)
                                        .size(IconSize::Small)
                                        .color(error_color),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(text_color)
                                        .child(format!("Error at position {}", pos)),
                                ),
                        )
                    })
                    // Hint (T064)
                    .when_some(error_hint, |s, hint| {
                        s.child(
                            div()
                                .flex()
                                .items_start()
                                .gap(px(8.0))
                                .px(px(12.0))
                                .py(px(10.0))
                                .rounded(px(4.0))
                                .bg(accent_color.opacity(0.1))
                                .border_l_2()
                                .border_color(accent_color)
                                .child(
                                    Icon::new(IconName::Info)
                                        .size(IconSize::Small)
                                        .color(accent_color),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.0))
                                        .child(
                                            div()
                                                .text_size(px(11.0))
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(accent_color)
                                                .child("Hint"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.0))
                                                .text_color(text_color)
                                                .child(hint.to_string()),
                                        ),
                                ),
                        )
                    })
                    // Technical detail (expandable)
                    .when_some(error_detail, |s, detail| {
                        s.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(8.0))
                                // Header with toggle
                                .child(
                                    div()
                                        .id("detail-toggle")
                                        .flex()
                                        .items_center()
                                        .gap(px(4.0))
                                        .cursor_pointer()
                                        .hover(|s| s.opacity(0.8))
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.toggle_detail(cx)),
                                        )
                                        .child(
                                            Icon::new(chevron_icon)
                                                .size(IconSize::Small)
                                                .color(text_muted),
                                        )
                                        .child(
                                            div().text_size(px(11.0)).text_color(text_muted).child(
                                                if is_expanded {
                                                    "Hide Details"
                                                } else {
                                                    "Show Details"
                                                },
                                            ),
                                        ),
                                )
                                // Detail content (when expanded)
                                .when(is_expanded, |s| {
                                    s.child(
                                        div()
                                            .px(px(12.0))
                                            .py(px(8.0))
                                            .rounded(px(4.0))
                                            .bg(element_bg)
                                            .child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .font_family("monospace")
                                                    .text_color(text_muted)
                                                    .child(detail.to_string()),
                                            ),
                                    )
                                }),
                        )
                    }),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_panel_content_builder() {
        let content = ErrorPanelContent::new("Query Error", "Syntax error")
            .with_hint("Check SQL syntax")
            .with_position(42)
            .with_code("42601");

        assert_eq!(content.error_type.as_ref(), "Query Error");
        assert_eq!(content.message.as_ref(), "Syntax error");
        assert_eq!(content.hint.as_ref().map(|s| s.as_ref()), Some("Check SQL syntax"));
        assert_eq!(content.position, Some(42));
        assert_eq!(content.code.as_ref().map(|s| s.as_ref()), Some("42601"));
    }
}
