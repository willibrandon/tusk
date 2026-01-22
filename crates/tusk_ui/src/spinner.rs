//! Spinner component for loading indicators.

use gpui::{
    div, prelude::*, px, Animation, AnimationExt, App, IntoElement, Pixels, RenderOnce, Window,
};
use std::time::Duration;

use crate::TuskTheme;

/// Size variants for the Spinner component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinnerSize {
    /// Small spinner: 14px
    Small,
    /// Medium spinner: 20px (default)
    #[default]
    Medium,
    /// Large spinner: 32px
    Large,
}

impl SpinnerSize {
    /// Get the size in pixels.
    pub fn pixels(&self) -> Pixels {
        match self {
            Self::Small => px(14.0),
            Self::Medium => px(20.0),
            Self::Large => px(32.0),
        }
    }

    /// Get the border width for this size.
    pub fn border_width(&self) -> Pixels {
        match self {
            Self::Small => px(2.0),
            Self::Medium => px(2.0),
            Self::Large => px(3.0),
        }
    }
}

/// A loading spinner component with continuous rotation animation.
#[derive(IntoElement)]
pub struct Spinner {
    size: SpinnerSize,
}

impl Spinner {
    /// Create a new spinner with default size.
    pub fn new() -> Self {
        Self { size: SpinnerSize::default() }
    }

    /// Set the spinner size.
    pub fn size(mut self, size: SpinnerSize) -> Self {
        self.size = size;
        self
    }

    /// Create a small spinner.
    pub fn small() -> Self {
        Self::new().size(SpinnerSize::Small)
    }

    /// Create a medium spinner.
    pub fn medium() -> Self {
        Self::new().size(SpinnerSize::Medium)
    }

    /// Create a large spinner.
    pub fn large() -> Self {
        Self::new().size(SpinnerSize::Large)
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Spinner {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();
        let size = self.size.pixels();
        let border_width = self.size.border_width();

        div()
            .size(size)
            .rounded_full()
            .border(border_width)
            .border_color(theme.colors.border)
            .child(
                // Inner arc that rotates
                div()
                    .absolute()
                    .inset_0()
                    .rounded_full()
                    .border_t(border_width)
                    .border_color(theme.colors.accent)
                    .with_animation(
                        "spinner-rotation",
                        Animation::new(Duration::from_secs(1)).repeat().with_easing(gpui::linear),
                        move |element, _progress| element.occlude(),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_sizes() {
        assert_eq!(SpinnerSize::Small.pixels(), px(14.0));
        assert_eq!(SpinnerSize::Medium.pixels(), px(20.0));
        assert_eq!(SpinnerSize::Large.pixels(), px(32.0));
    }

    #[test]
    fn test_spinner_construction() {
        let spinner = Spinner::new();
        assert_eq!(spinner.size, SpinnerSize::Medium);

        let small = Spinner::small();
        assert_eq!(small.size, SpinnerSize::Small);

        let large = Spinner::large();
        assert_eq!(large.size, SpinnerSize::Large);
    }
}
