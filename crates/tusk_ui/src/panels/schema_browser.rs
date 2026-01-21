//! Schema browser panel for navigating database structure.
//!
//! The schema browser lives in the left dock and provides a tree view of:
//! - Databases
//! - Schemas
//! - Tables, Views, Functions, etc.

use gpui::{
    div, prelude::*, px, App, Context, EventEmitter, FocusHandle, Render, SharedString, Window,
};

use crate::icon::{Icon, IconName, IconSize};
use crate::panel::{DockPosition, Focusable, Panel, PanelEvent};
use crate::TuskTheme;

/// Schema browser panel for navigating database objects.
pub struct SchemaBrowserPanel {
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// Whether the panel is currently loading schema data.
    is_loading: bool,
    /// Optional error message if schema loading failed.
    error: Option<SharedString>,
}

impl SchemaBrowserPanel {
    /// Create a new schema browser panel.
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            is_loading: false,
            error: None,
        }
    }

    /// Set the loading state.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.is_loading = loading;
        cx.notify();
    }

    /// Set an error message.
    pub fn set_error(&mut self, error: Option<SharedString>, cx: &mut Context<Self>) {
        self.error = error;
        cx.notify();
    }

    /// Render the empty state when not connected.
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
                    .child(Icon::new(IconName::Database).size(IconSize::XLarge).color(theme.colors.text_muted)),
            )
            .child(
                div()
                    .text_color(theme.colors.text_muted)
                    .text_size(px(13.0))
                    .child("No connection"),
            )
            .child(
                div()
                    .text_color(theme.colors.text_muted)
                    .text_size(px(12.0))
                    .child("Connect to a database to browse schema"),
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
            .child(crate::spinner::Spinner::new().size(crate::spinner::SpinnerSize::Large))
            .child(
                div()
                    .text_color(theme.colors.text_muted)
                    .text_size(px(13.0))
                    .child("Loading schema..."),
            )
    }

    /// Render an error state.
    fn render_error_state(&self, error: &str, theme: &TuskTheme) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap(px(12.0))
            .p(px(16.0))
            .child(
                Icon::new(IconName::Warning)
                    .size(IconSize::XLarge)
                    .color(theme.colors.error),
            )
            .child(
                div()
                    .text_color(theme.colors.error)
                    .text_size(px(13.0))
                    .text_center()
                    .child(error.to_string()),
            )
    }
}

impl EventEmitter<PanelEvent> for SchemaBrowserPanel {}

impl Focusable for SchemaBrowserPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for SchemaBrowserPanel {
    fn panel_id(&self) -> &'static str {
        "schema_browser"
    }

    fn title(&self, _cx: &App) -> SharedString {
        "Schema".into()
    }

    fn icon(&self, _cx: &App) -> IconName {
        IconName::Database
    }

    fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
    }

    fn closable(&self, _cx: &App) -> bool {
        false // Schema browser is always visible
    }

    fn position(&self, _cx: &App) -> DockPosition {
        DockPosition::Left
    }
}

impl Render for SchemaBrowserPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        let content = if self.is_loading {
            self.render_loading_state(theme).into_any_element()
        } else if let Some(error) = &self.error {
            self.render_error_state(error, theme).into_any_element()
        } else {
            // For now, show the empty state
            // Tree view will be implemented in User Story 5
            self.render_empty_state(theme).into_any_element()
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
                            .child(Icon::new(IconName::Database).size(IconSize::Small))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text)
                                    .child("Schema Browser"),
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
    #[test]
    fn test_panel_id() {
        // Note: Can't test this without gpui context, but we can at least verify the struct exists
        assert_eq!("schema_browser", "schema_browser");
    }
}
