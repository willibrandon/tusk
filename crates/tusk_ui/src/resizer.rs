//! Resizer component for draggable resize handles.
//!
//! This component provides a visual resize handle that shows the correct
//! cursor style on hover. Actual resize logic is handled via GPUI's
//! on_drag/on_drag_move pattern at the parent level, following Zed's approach.

use gpui::{div, prelude::*, App, Axis, CursorStyle, IntoElement, Length, RenderOnce, Window};

use crate::layout::sizes::RESIZER_SIZE;
use crate::TuskTheme;

/// A visual resize handle for docks and pane splits.
///
/// This is a visual-only component. The actual resize behavior should be
/// implemented using GPUI's on_drag/on_drag_move pattern:
/// - Place `on_drag` on the resize handle with a marker type
/// - Place `on_drag_move` on the parent element to handle the actual resizing
///
/// See `DraggedDock` in dock.rs and `DraggedPaneSplit` in pane.rs for examples.
#[derive(IntoElement)]
pub struct Resizer {
    axis: Axis,
}

impl Resizer {
    /// Create a new resizer for the given axis.
    ///
    /// - `Axis::Horizontal`: Resizes left/right (cursor: ResizeLeftRight)
    /// - `Axis::Vertical`: Resizes up/down (cursor: ResizeUpDown)
    pub fn new(axis: Axis) -> Self {
        Self { axis }
    }

    /// Create a horizontal resizer (for side docks).
    pub fn horizontal() -> Self {
        Self::new(Axis::Horizontal)
    }

    /// Create a vertical resizer (for bottom dock and horizontal splits).
    pub fn vertical() -> Self {
        Self::new(Axis::Vertical)
    }
}

impl RenderOnce for Resizer {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        let (width, height, cursor): (Length, Length, CursorStyle) = match self.axis {
            Axis::Horizontal => {
                (RESIZER_SIZE.into(), gpui::relative(1.0).into(), CursorStyle::ResizeLeftRight)
            }
            Axis::Vertical => {
                (gpui::relative(1.0).into(), RESIZER_SIZE.into(), CursorStyle::ResizeUpDown)
            }
        };

        let border_color = theme.colors.border;

        div()
            .id("resizer")
            .w(width)
            .h(height)
            .flex_shrink_0()
            .cursor(cursor)
            .bg(gpui::transparent_black())
            .hover(|style| style.bg(border_color))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resizer_creation() {
        let horizontal = Resizer::horizontal();
        assert!(matches!(horizontal.axis, Axis::Horizontal));

        let vertical = Resizer::vertical();
        assert!(matches!(vertical.axis, Axis::Vertical));
    }
}
