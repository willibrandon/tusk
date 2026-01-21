//! Dock component for side and bottom panels.
//!
//! Docks are containers that hold panels at fixed positions around the workspace.
//! Each dock can contain multiple panels with tab switching.

use std::sync::Arc;

use gpui::{
    deferred, div, prelude::*, px, App, Context, EventEmitter, FocusHandle, IntoElement, Pixels,
    Render, Subscription, Window,
};

use crate::icon::IconName;
use crate::layout::sizes::{DOCK_MAX_BOTTOM, DOCK_MAX_SIDE, DOCK_MIN, RESIZER_SIZE};
use crate::panel::{DockPosition, PanelEntry, PanelHandle};
use crate::TuskTheme;

/// Events emitted by the dock.
#[derive(Debug, Clone)]
pub enum DockEvent {
    /// Dock was resized.
    Resized { size: Pixels },
    /// Visibility toggled.
    VisibilityChanged { visible: bool },
    /// Active panel changed.
    PanelChanged { index: usize },
}

/// Marker type for dock drag operations.
///
/// This is used with `on_drag` to initiate dock resizing.
/// The actual resize calculations happen in the Workspace via `on_drag_move`.
#[derive(Clone)]
pub struct DraggedDock(pub DockPosition);

impl Render for DraggedDock {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Invisible drag visual - the actual drag feedback comes from cursor changes
        gpui::Empty
    }
}

/// A dockable panel container.
///
/// Docks are positioned at the edges of the workspace and can contain
/// multiple panels with tab-based switching.
pub struct Dock {
    position: DockPosition,
    panels: Vec<PanelEntry>,
    active_panel_index: usize,
    size: Pixels,
    is_visible: bool,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl Dock {
    /// Create a new dock at the specified position.
    pub fn new(position: DockPosition, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let default_size = match position {
            DockPosition::Left | DockPosition::Right => px(250.0),
            DockPosition::Bottom => px(200.0),
        };

        Self {
            position,
            panels: Vec::new(),
            active_panel_index: 0,
            size: default_size,
            is_visible: true,
            focus_handle,
            _subscriptions: Vec::new(),
        }
    }

    /// Get the dock position.
    pub fn position(&self) -> DockPosition {
        self.position
    }

    /// Get current size in pixels.
    pub fn size(&self) -> Pixels {
        self.size
    }

    /// Set size (clamped to min/max).
    pub fn set_size(&mut self, size: Pixels, cx: &mut Context<Self>) {
        let (min, max) = self.size_constraints();
        let clamped = size.max(min).min(max);
        if clamped != self.size {
            self.size = clamped;
            cx.emit(DockEvent::Resized { size: clamped });
            cx.notify();
        }
    }

    /// Check if dock is visible (not collapsed).
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    /// Set visibility.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if visible != self.is_visible {
            self.is_visible = visible;
            cx.emit(DockEvent::VisibilityChanged { visible });
            cx.notify();
        }
    }

    /// Toggle visibility.
    pub fn toggle_visibility(&mut self, cx: &mut Context<Self>) {
        self.set_visible(!self.is_visible, cx);
    }

    /// Get registered panels.
    pub fn panels(&self) -> &[PanelEntry] {
        &self.panels
    }

    /// Check if the dock has any panels.
    pub fn has_panels(&self) -> bool {
        !self.panels.is_empty()
    }

    /// Add a panel to the dock.
    pub fn add_panel(&mut self, panel: Arc<dyn PanelHandle>, cx: &mut Context<Self>) {
        self.panels.push(PanelEntry { panel });
        cx.notify();
    }

    /// Remove a panel by index.
    pub fn remove_panel(&mut self, index: usize, cx: &mut Context<Self>) -> Option<PanelEntry> {
        if index < self.panels.len() {
            let entry = self.panels.remove(index);
            // Adjust active index if needed
            if self.active_panel_index >= self.panels.len() && !self.panels.is_empty() {
                self.active_panel_index = self.panels.len() - 1;
            }
            cx.notify();
            Some(entry)
        } else {
            None
        }
    }

    /// Get the active panel index.
    pub fn active_panel_index(&self) -> usize {
        self.active_panel_index
    }

    /// Get the active panel.
    pub fn active_panel(&self) -> Option<&Arc<dyn PanelHandle>> {
        self.panels.get(self.active_panel_index).map(|e| &e.panel)
    }

    /// Activate a panel by index.
    pub fn activate_panel(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.panels.len() && index != self.active_panel_index {
            self.active_panel_index = index;
            cx.emit(DockEvent::PanelChanged { index });
            cx.notify();
        }
    }

    /// Get size constraints for this dock position.
    pub fn size_constraints(&self) -> (Pixels, Pixels) {
        match self.position {
            DockPosition::Left | DockPosition::Right => (DOCK_MIN, DOCK_MAX_SIDE),
            DockPosition::Bottom => (DOCK_MIN, DOCK_MAX_BOTTOM),
        }
    }

    /// Focus the dock (focuses the active panel).
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        if let Some(panel) = self.active_panel() {
            panel.focus(window, cx);
        }
    }

    /// Render the panel tabs.
    fn render_tabs(&self, cx: &App) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        div()
            .flex()
            .items_center()
            .h(px(32.0))
            .px(px(8.0))
            .gap(px(4.0))
            .bg(theme.colors.tab_bar_background)
            .border_b_1()
            .border_color(theme.colors.border)
            .children(self.panels.iter().enumerate().map(|(index, entry)| {
                let is_active = index == self.active_panel_index;
                let title = entry.panel.title(cx);
                let icon = entry.panel.icon(cx);

                let bg = if is_active {
                    theme.colors.tab_active_background
                } else {
                    gpui::transparent_black()
                };

                let text_color = if is_active {
                    theme.colors.text
                } else {
                    theme.colors.text_muted
                };

                div()
                    .id(("dock-tab", index))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .px(px(8.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .bg(bg)
                    .text_color(text_color)
                    .text_size(px(12.0))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.colors.element_hover))
                    .child(self.render_icon(icon, text_color))
                    .child(title.to_string())
            }))
    }

    /// Render an icon with a unicode fallback.
    fn render_icon(&self, icon: IconName, color: gpui::Hsla) -> impl IntoElement {
        use crate::icon::{Icon, IconSize};
        Icon::new(icon).size(IconSize::Small).color(color)
    }

    /// Render the active panel content.
    fn render_content(&self, cx: &App) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        if let Some(panel) = self.active_panel() {
            div()
                .flex_1()
                .overflow_hidden()
                .bg(theme.colors.panel_background)
                .child(panel.to_any())
        } else {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.colors.panel_background)
                .text_color(theme.colors.text_muted)
                .text_size(px(14.0))
                .child("No panels")
        }
    }

    /// Create the resize handle element for this dock.
    ///
    /// Following Zed's pattern, this handle:
    /// - Uses `on_drag` with `DraggedDock` to initiate dragging
    /// - Returns a new entity that renders to `gpui::Empty`
    /// - The actual resize calculations happen in Workspace's `on_drag_move`
    fn create_resize_handle(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let position = self.position;

        // The drag value that will be passed to on_drag_move handlers
        let drag_value = DraggedDock(position);

        let handle = div()
            .id("dock-resize-handle")
            .on_drag(drag_value, |dock, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| dock.clone())
            })
            .occlude();

        // Position the handle based on dock position
        match position {
            DockPosition::Left => deferred(
                handle
                    .absolute()
                    .right(-RESIZER_SIZE / 2.)
                    .top(px(0.))
                    .h_full()
                    .w(RESIZER_SIZE)
                    .cursor_col_resize(),
            ),
            DockPosition::Right => deferred(
                handle
                    .absolute()
                    .left(-RESIZER_SIZE / 2.)
                    .top(px(0.))
                    .h_full()
                    .w(RESIZER_SIZE)
                    .cursor_col_resize(),
            ),
            DockPosition::Bottom => deferred(
                handle
                    .absolute()
                    .top(-RESIZER_SIZE / 2.)
                    .left(px(0.))
                    .w_full()
                    .h(RESIZER_SIZE)
                    .cursor_row_resize(),
            ),
        }
    }
}

impl EventEmitter<DockEvent> for Dock {}

impl Render for Dock {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        if !self.is_visible {
            return div().into_any_element();
        }

        let position = self.position;
        let size = self.size;

        // Create the dock container with appropriate dimensions
        let mut dock = div()
            .relative()
            .flex()
            .flex_shrink_0()
            .bg(theme.colors.panel_background)
            .border_color(theme.colors.border);

        // Set size and border based on position
        match position {
            DockPosition::Left => {
                dock = dock.flex_col().w(size).h_full().border_r_1();
            }
            DockPosition::Right => {
                dock = dock.flex_col().w(size).h_full().border_l_1();
            }
            DockPosition::Bottom => {
                dock = dock.flex_col().h(size).w_full().border_t_1();
            }
        }

        // Build content with tabs and panel
        let content = div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_tabs(cx))
            .child(self.render_content(cx));

        // Add the resize handle
        let resize_handle = self.create_resize_handle(cx);

        dock.child(content).child(resize_handle).into_any_element()
    }
}

/// Focusable implementation for Dock.
impl crate::panel::Focusable for Dock {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_constraints() {
        // Can't easily test without GPUI context, but verify constants
        assert!(DOCK_MIN < DOCK_MAX_SIDE);
        assert!(DOCK_MIN < DOCK_MAX_BOTTOM);
    }
}
