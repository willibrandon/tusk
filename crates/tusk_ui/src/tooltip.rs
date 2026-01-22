//! Tooltip component for displaying hover hints.
//!
//! This module provides a simple tooltip component that displays text
//! when hovering over an element.

use gpui::{div, prelude::*, px, AnyView, Context, IntoElement, Render, SharedString, Window};

use crate::layout::spacing;
use crate::TuskTheme;

/// A simple text tooltip.
pub struct Tooltip {
    text: SharedString,
}

impl Tooltip {
    /// Create a tooltip builder function that displays the given text.
    pub fn text(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut gpui::App) -> AnyView {
        let text = text.into();
        move |_, cx| {
            cx.new(|_cx| Tooltip { text: text.clone() })
                .into()
        }
    }
}

impl Render for Tooltip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        // Offset from cursor position
        div()
            .pl(spacing::XS)
            .pt(spacing::XS)
            .child(
                div()
                    .bg(theme.colors.elevated_surface_background)
                    .border_1()
                    .border_color(theme.colors.border)
                    .rounded(px(4.0))
                    .shadow_sm()
                    .py(spacing::XS)
                    .px(spacing::SM)
                    .text_sm()
                    .text_color(theme.colors.text)
                    .max_w(px(400.0))
                    .child(self.text.clone()),
            )
    }
}
