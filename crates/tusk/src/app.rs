//! Tusk application root component.

use gpui::{div, prelude::*, Context, IntoElement, Render, Window};
use tusk_ui::TuskTheme;

/// Root application component that manages the main window.
pub struct TuskApp;

impl TuskApp {
    /// Create a new TuskApp instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TuskApp {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for TuskApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();
        let colors = &theme.colors;

        div().flex().flex_col().size_full().bg(colors.background).text_color(colors.text).child(
            div().flex().flex_1().items_center().justify_center().child("Tusk - PostgreSQL Client"),
        )
    }
}
