//! Pane and PaneGroup components for tab management.
//!
//! A Pane contains multiple tabs (TabItems) and displays one at a time.
//! A PaneGroup manages the layout of panes, supporting splits.

use gpui::{
    canvas, deferred, div, prelude::*, px, AnyElement, AnyView, App, Axis, Bounds, ClickEvent,
    Context, CursorStyle, DragMoveEvent, Entity, EntityId, EventEmitter, FocusHandle, IntoElement,
    Pixels, Point, Render, SharedString, Window,
};
use smallvec::SmallVec;
use uuid::Uuid;

use crate::icon::{Icon, IconName, IconSize};
use crate::layout::{sizes, spacing};
use crate::panel::Focusable;
use crate::TuskTheme;

/// Size of the split resize handle.
const SPLIT_HANDLE_SIZE: Pixels = px(6.0);
/// Minimum size for a pane in pixels.
const PANE_MIN_SIZE: Pixels = px(100.0);

// ============================================================================
// TabItem
// ============================================================================

/// A single tab item in a pane.
#[derive(Clone)]
pub struct TabItem {
    /// Unique identifier for this tab.
    pub id: Uuid,
    /// Display title for the tab.
    pub title: SharedString,
    /// Optional icon for the tab.
    pub icon: Option<IconName>,
    /// Whether the tab has unsaved changes.
    pub dirty: bool,
    /// Whether the tab can be closed.
    pub closable: bool,
    /// The content view for this tab.
    pub view: AnyView,
}

impl TabItem {
    /// Create a new tab item with the given title and view.
    pub fn new(title: impl Into<SharedString>, view: impl Into<AnyView>) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            icon: None,
            dirty: false,
            closable: true,
            view: view.into(),
        }
    }

    /// Set the tab icon.
    pub fn with_icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set the dirty state.
    pub fn with_dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }

    /// Set whether the tab is closable.
    pub fn with_closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }
}

// ============================================================================
// Pane Events
// ============================================================================

/// Events emitted by a Pane.
#[derive(Debug, Clone)]
pub enum PaneEvent {
    /// A tab was added.
    TabAdded { tab_id: Uuid },
    /// A tab was closed.
    TabClosed { tab_id: Uuid },
    /// The active tab changed.
    ActiveTabChanged { index: usize },
    /// A tab was reordered.
    TabMoved { from: usize, to: usize },
    /// The pane wants to close (all tabs closed).
    Close,
}

// ============================================================================
// Pane
// ============================================================================

/// A pane containing multiple tabs.
pub struct Pane {
    /// All tabs in this pane.
    tabs: Vec<TabItem>,
    /// Index of the currently active tab.
    active_tab_index: usize,
    /// Focus handle for this pane.
    focus_handle: FocusHandle,
}

impl Pane {
    /// Create a new empty pane.
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_index: 0,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Get all tabs.
    pub fn tabs(&self) -> &[TabItem] {
        &self.tabs
    }

    /// Get the active tab index.
    pub fn active_tab_index(&self) -> usize {
        self.active_tab_index
    }

    /// Get the active tab item.
    pub fn active_tab(&self) -> Option<&TabItem> {
        self.tabs.get(self.active_tab_index)
    }

    /// Check if the pane is empty.
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Check if any tab has unsaved changes.
    pub fn has_dirty_tabs(&self) -> bool {
        self.tabs.iter().any(|tab| tab.dirty)
    }

    /// Add a new tab to the pane.
    pub fn add_tab(&mut self, item: TabItem, cx: &mut Context<Self>) {
        let tab_id = item.id;
        self.tabs.push(item);
        self.active_tab_index = self.tabs.len() - 1;
        cx.emit(PaneEvent::TabAdded { tab_id });
        cx.notify();
    }

    /// Close a tab by index.
    ///
    /// Returns the closed tab if successful.
    pub fn close_tab(&mut self, index: usize, cx: &mut Context<Self>) -> Option<TabItem> {
        if index >= self.tabs.len() {
            return None;
        }

        let tab = self.tabs.remove(index);
        let tab_id = tab.id;

        // Adjust active tab index
        if self.tabs.is_empty() {
            self.active_tab_index = 0;
            cx.emit(PaneEvent::Close);
        } else if self.active_tab_index >= self.tabs.len() {
            self.active_tab_index = self.tabs.len() - 1;
        } else if index < self.active_tab_index {
            self.active_tab_index = self.active_tab_index.saturating_sub(1);
        }

        cx.emit(PaneEvent::TabClosed { tab_id });
        cx.notify();
        Some(tab)
    }

    /// Close the currently active tab.
    pub fn close_active_tab(&mut self, cx: &mut Context<Self>) -> Option<TabItem> {
        if self.tabs.is_empty() {
            return None;
        }
        self.close_tab(self.active_tab_index, cx)
    }

    /// Activate a tab by index.
    pub fn activate_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.tabs.len() && index != self.active_tab_index {
            self.active_tab_index = index;
            cx.emit(PaneEvent::ActiveTabChanged { index });
            cx.notify();
        }
    }

    /// Activate the next tab (wraps around).
    pub fn activate_next_tab(&mut self, cx: &mut Context<Self>) {
        if self.tabs.is_empty() {
            return;
        }
        let next = (self.active_tab_index + 1) % self.tabs.len();
        self.activate_tab(next, cx);
    }

    /// Activate the previous tab (wraps around).
    pub fn activate_previous_tab(&mut self, cx: &mut Context<Self>) {
        if self.tabs.is_empty() {
            return;
        }
        let prev = if self.active_tab_index == 0 {
            self.tabs.len() - 1
        } else {
            self.active_tab_index - 1
        };
        self.activate_tab(prev, cx);
    }

    /// Move a tab from one index to another.
    pub fn move_tab(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
            return;
        }

        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);

        // Adjust active tab index
        if self.active_tab_index == from {
            self.active_tab_index = to;
        } else if from < self.active_tab_index && to >= self.active_tab_index {
            self.active_tab_index = self.active_tab_index.saturating_sub(1);
        } else if from > self.active_tab_index && to <= self.active_tab_index {
            self.active_tab_index += 1;
        }

        cx.emit(PaneEvent::TabMoved { from, to });
        cx.notify();
    }

    /// Render the tab bar.
    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>().clone();
        let tabs: Vec<_> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| self.render_tab(index, tab, &theme, cx))
            .collect();

        div()
            .h(sizes::TAB_BAR_HEIGHT)
            .w_full()
            .flex()
            .items_center()
            .bg(theme.colors.tab_bar_background)
            .border_b_1()
            .border_color(theme.colors.border)
            .children(tabs)
    }

    /// Render a single tab.
    fn render_tab(
        &self,
        index: usize,
        tab: &TabItem,
        theme: &TuskTheme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_active = index == self.active_tab_index;
        let bg = if is_active {
            theme.colors.tab_active_background
        } else {
            theme.colors.tab_inactive_background
        };

        let text_color = if is_active {
            theme.colors.text
        } else {
            theme.colors.text_muted
        };

        let weak_pane = cx.entity().downgrade();
        let tab_icon = tab.icon;
        let tab_title = tab.title.clone();
        let tab_dirty = tab.dirty;
        let tab_closable = tab.closable;
        let hover_bg = theme.colors.tab_hover_background;
        let close_hover_bg = theme.colors.ghost_element_hover;

        let mut tab_div = div()
            .id(("pane-tab", index))
            .h_full()
            .px(spacing::SM)
            .flex()
            .items_center()
            .gap(spacing::XS)
            .bg(bg)
            .hover(|style| style.bg(hover_bg))
            .cursor_pointer()
            .on_click({
                let weak = weak_pane.clone();
                move |_event: &ClickEvent, _window, cx| {
                    if let Some(pane) = weak.upgrade() {
                        pane.update(cx, |pane, cx| pane.activate_tab(index, cx));
                    }
                }
            });

        // Icon
        if let Some(icon) = tab_icon {
            tab_div = tab_div.child(Icon::new(icon).size(IconSize::Small).color(text_color));
        }

        // Title with dirty indicator
        let title_text = if tab_dirty {
            format!("{}*", tab_title)
        } else {
            tab_title.to_string()
        };

        tab_div = tab_div.child(div().text_sm().text_color(text_color).child(title_text));

        // Close button
        if tab_closable {
            let weak = weak_pane.clone();
            tab_div = tab_div.child(
                div()
                    .id(("pane-tab-close", index))
                    .size(px(16.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(2.0))
                    .hover(|style| style.bg(close_hover_bg))
                    .on_click(move |_event: &ClickEvent, _window, cx| {
                        cx.stop_propagation();
                        if let Some(pane) = weak.upgrade() {
                            pane.update(cx, |pane, cx| {
                                pane.close_tab(index, cx);
                            });
                        }
                    })
                    .child(Icon::new(IconName::Close).size(IconSize::XSmall)),
            );
        }

        tab_div
    }

    /// Render the content area.
    fn render_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        if let Some(tab) = self.active_tab() {
            div()
                .flex_1()
                .w_full()
                .bg(theme.colors.editor_background)
                .child(tab.view.clone())
        } else {
            // Empty state
            div()
                .flex_1()
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.colors.editor_background)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(spacing::MD)
                        .child(
                            div()
                                .text_lg()
                                .text_color(theme.colors.text_muted)
                                .child("No tabs open"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.colors.text_muted)
                                .child("Press Cmd+N to create a new query"),
                        ),
                )
        }
    }
}

impl EventEmitter<PaneEvent> for Pane {}

impl Focusable for Pane {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Pane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .track_focus(&self.focus_handle)
            .key_context("Pane")
            .child(self.render_tab_bar(cx))
            .child(self.render_content(cx))
    }
}

// ============================================================================
// DraggedPaneSplit - marker for pane split drag operations
// ============================================================================

/// Marker type for pane split drag operations.
///
/// Contains the split index and axis for the drag.
#[derive(Clone)]
pub struct DraggedPaneSplit {
    /// The index of the split handle being dragged.
    pub split_index: usize,
    /// The axis of the split (Horizontal or Vertical).
    pub axis: Axis,
}

impl Render for DraggedPaneSplit {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Invisible drag visual
        gpui::Empty
    }
}

// ============================================================================
// PaneNode
// ============================================================================

/// A node in the pane tree (either a single pane or a split).
pub enum PaneNode {
    /// A single pane.
    Single(Entity<Pane>),
    /// A split containing multiple children.
    Split {
        /// The axis of the split.
        axis: Axis,
        /// The child nodes (boxed for recursion).
        children: SmallVec<[Box<PaneNode>; 2]>,
        /// The ratios for each child (must sum to 1.0).
        ratios: SmallVec<[f32; 2]>,
    },
}

impl PaneNode {
    /// Get all panes in this node (flattened).
    pub fn panes(&self) -> Vec<Entity<Pane>> {
        match self {
            PaneNode::Single(pane) => vec![pane.clone()],
            PaneNode::Split { children, .. } => {
                children.iter().flat_map(|child| child.panes()).collect()
            }
        }
    }

    /// Find a pane by entity ID.
    pub fn find_pane(&self, id: EntityId) -> Option<Entity<Pane>> {
        match self {
            PaneNode::Single(pane) => {
                if pane.entity_id() == id {
                    Some(pane.clone())
                } else {
                    None
                }
            }
            PaneNode::Split { children, .. } => {
                children.iter().find_map(|child| child.find_pane(id))
            }
        }
    }
}

// ============================================================================
// PaneGroup Events
// ============================================================================

/// Events emitted by a PaneGroup.
#[derive(Debug, Clone)]
pub enum PaneGroupEvent {
    /// A pane was split.
    Split { axis: Axis, new_pane: Entity<Pane> },
    /// A pane was closed.
    PaneClosed { pane: Entity<Pane> },
    /// The active pane changed.
    ActivePaneChanged { pane: Entity<Pane> },
    /// Layout ratios changed.
    RatiosChanged,
}

// ============================================================================
// PaneGroup
// ============================================================================

/// A group of panes with support for splits.
pub struct PaneGroup {
    /// The root node of the pane tree.
    root: PaneNode,
    /// The currently active pane.
    active_pane: Entity<Pane>,
    /// Focus handle for the group.
    focus_handle: FocusHandle,
    /// Current bounds of the pane group (for resize calculations).
    bounds: Bounds<Pixels>,
    /// Previous drag coordinates (to avoid duplicate processing).
    previous_drag_coordinates: Option<Point<Pixels>>,
}

impl PaneGroup {
    /// Create a new pane group with a single pane.
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let pane = cx.new(|cx| Pane::new(cx));
        Self {
            root: PaneNode::Single(pane.clone()),
            active_pane: pane,
            focus_handle: cx.focus_handle(),
            bounds: Bounds::default(),
            previous_drag_coordinates: None,
        }
    }

    /// Get the active pane.
    pub fn active_pane(&self) -> &Entity<Pane> {
        &self.active_pane
    }

    /// Set the active pane.
    pub fn set_active_pane(&mut self, pane: Entity<Pane>, cx: &mut Context<Self>) {
        if self.active_pane.entity_id() != pane.entity_id() {
            self.active_pane = pane.clone();
            cx.emit(PaneGroupEvent::ActivePaneChanged { pane });
            cx.notify();
        }
    }

    /// Get all panes in the group.
    pub fn panes(&self) -> Vec<Entity<Pane>> {
        self.root.panes()
    }

    /// Split the active pane along an axis.
    pub fn split_active_pane(
        &mut self,
        axis: Axis,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<Pane> {
        let new_pane = cx.new(|cx| Pane::new(cx));

        // For simplicity, we replace the root with a split
        // A full implementation would find the active pane in the tree
        let old_root = std::mem::replace(&mut self.root, PaneNode::Single(new_pane.clone()));

        self.root = PaneNode::Split {
            axis,
            children: smallvec::smallvec![
                Box::new(old_root),
                Box::new(PaneNode::Single(new_pane.clone()))
            ],
            ratios: smallvec::smallvec![0.5, 0.5],
        };

        self.active_pane = new_pane.clone();
        cx.emit(PaneGroupEvent::Split {
            axis,
            new_pane: new_pane.clone(),
        });
        cx.notify();

        new_pane
    }

    /// Close a pane.
    pub fn close_pane(&mut self, pane: Entity<Pane>, cx: &mut Context<Self>) {
        // Simplified: if we have a split, collapse to the remaining child
        if let PaneNode::Split { children, .. } = &mut self.root {
            if children.len() == 2 {
                let remaining = if matches!(children[0].as_ref(), PaneNode::Single(p) if p.entity_id() == pane.entity_id())
                {
                    children.remove(1)
                } else {
                    children.remove(0)
                };
                self.root = *remaining;

                // Update active pane
                self.active_pane = self.root.panes().first().unwrap().clone();
                cx.emit(PaneGroupEvent::PaneClosed { pane });
                cx.notify();
            }
        }
    }

    /// Resize a split at the given index based on mouse position.
    pub fn resize_split(
        &mut self,
        split_index: usize,
        axis: Axis,
        mouse_position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if let PaneNode::Split { ratios, axis: split_axis, .. } = &mut self.root {
            if *split_axis != axis || split_index >= ratios.len() - 1 {
                return;
            }

            // Calculate the new ratio based on mouse position relative to bounds
            let total_size = match axis {
                Axis::Horizontal => self.bounds.size.width,
                Axis::Vertical => self.bounds.size.height,
            };

            if total_size <= Pixels::ZERO {
                return;
            }

            let mouse_offset = match axis {
                Axis::Horizontal => mouse_position.x - self.bounds.left(),
                Axis::Vertical => mouse_position.y - self.bounds.top(),
            };

            // Calculate cumulative ratio up to this split
            let cumulative_before: f32 = ratios.iter().take(split_index + 1).sum();

            // Calculate where the split handle should be
            let new_ratio = (mouse_offset / total_size).clamp(0.1, 0.9);

            // Calculate the adjustment needed
            let current_position = cumulative_before;
            let adjustment = new_ratio - current_position;

            // Apply adjustment to the adjacent ratios
            if split_index < ratios.len() - 1 {
                let min_ratio = (PANE_MIN_SIZE / total_size).max(0.1);

                // Adjust ratios
                ratios[split_index] = (ratios[split_index] + adjustment).clamp(min_ratio, 1.0 - min_ratio);
                ratios[split_index + 1] = (ratios[split_index + 1] - adjustment).clamp(min_ratio, 1.0 - min_ratio);

                // Normalize ratios to sum to 1.0
                let total: f32 = ratios.iter().sum();
                if total > 0.0 {
                    for ratio in ratios.iter_mut() {
                        *ratio /= total;
                    }
                }

                cx.emit(PaneGroupEvent::RatiosChanged);
                cx.notify();
            }
        }
    }

    /// Focus the next pane.
    pub fn focus_next_pane(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let panes = self.panes();
        if let Some(pos) = panes
            .iter()
            .position(|p| p.entity_id() == self.active_pane.entity_id())
        {
            let next = (pos + 1) % panes.len();
            self.set_active_pane(panes[next].clone(), cx);
        }
    }

    /// Focus the previous pane.
    pub fn focus_previous_pane(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let panes = self.panes();
        if let Some(pos) = panes
            .iter()
            .position(|p| p.entity_id() == self.active_pane.entity_id())
        {
            let prev = if pos == 0 { panes.len() - 1 } else { pos - 1 };
            self.set_active_pane(panes[prev].clone(), cx);
        }
    }

    /// Render a pane node.
    fn render_node(&self, node: &PaneNode, cx: &mut Context<Self>) -> AnyElement {
        match node {
            PaneNode::Single(pane) => {
                let is_active = pane.entity_id() == self.active_pane.entity_id();
                let weak_group = cx.entity().downgrade();
                let pane_clone = pane.clone();
                let theme = cx.global::<TuskTheme>();
                let accent = theme.colors.accent;

                let mut pane_div = div()
                    .id(("pane-container", pane.entity_id().as_u64()))
                    .size_full();

                if is_active {
                    pane_div = pane_div.border_2().border_color(accent);
                }

                pane_div
                    .on_click(move |_event: &ClickEvent, _window, cx| {
                        if let Some(group) = weak_group.upgrade() {
                            group.update(cx, |group, cx| {
                                group.set_active_pane(pane_clone.clone(), cx);
                            });
                        }
                    })
                    .child(pane.clone())
                    .into_any_element()
            }
            PaneNode::Split {
                axis,
                children,
                ratios,
            } => {
                // Build the split container
                let mut container = match axis {
                    Axis::Horizontal => div().flex().flex_row().size_full(),
                    Axis::Vertical => div().flex().flex_col().size_full(),
                };

                for (i, (child, ratio)) in children.iter().zip(ratios.iter()).enumerate() {
                    // Add the child
                    let child_element = self.render_node(child, cx);
                    container = match axis {
                        Axis::Horizontal => container.child(
                            div()
                                .h_full()
                                .flex_basis(gpui::relative(*ratio))
                                .child(child_element),
                        ),
                        Axis::Vertical => container.child(
                            div()
                                .w_full()
                                .flex_basis(gpui::relative(*ratio))
                                .child(child_element),
                        ),
                    };

                    // Add resize handle between children (not after the last one)
                    if i < children.len() - 1 {
                        let split_axis = *axis;
                        let split_index = i;
                        let theme = cx.global::<TuskTheme>();
                        let border_color = theme.colors.border;
                        let drag_value = DraggedPaneSplit {
                            split_index,
                            axis: split_axis,
                        };

                        let cursor = match split_axis {
                            Axis::Horizontal => CursorStyle::ResizeLeftRight,
                            Axis::Vertical => CursorStyle::ResizeUpDown,
                        };

                        let handle = div()
                            .id(("split-handle", split_index))
                            .flex_shrink_0()
                            .bg(border_color)
                            .cursor(cursor)
                            .on_drag(drag_value, |drag, _, _, cx| {
                                cx.stop_propagation();
                                cx.new(|_| drag.clone())
                            });

                        let sized_handle = match split_axis {
                            Axis::Horizontal => handle.w(SPLIT_HANDLE_SIZE).h_full(),
                            Axis::Vertical => handle.h(SPLIT_HANDLE_SIZE).w_full(),
                        };

                        container = container.child(deferred(sized_handle));
                    }
                }

                container.into_any_element()
            }
        }
    }
}

impl EventEmitter<PaneGroupEvent> for PaneGroup {}

impl Focusable for PaneGroup {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PaneGroup {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Get entity handle for bounds tracking canvas
        let this = cx.entity().clone();

        div()
            .size_full()
            .relative()
            .track_focus(&self.focus_handle)
            .key_context("PaneGroup")
            // Track bounds using canvas element (Zed's pattern)
            .child({
                canvas(
                    {
                        let this = this.clone();
                        move |bounds, _window, cx| {
                            this.update(cx, |pane_group, _cx| {
                                pane_group.bounds = bounds;
                            });
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full()
            })
            // Handle split resize via drag move
            .on_drag_move(cx.listener(
                |this, e: &DragMoveEvent<DraggedPaneSplit>, _window, cx| {
                    // Avoid processing duplicate coordinates
                    if this.previous_drag_coordinates != Some(e.event.position) {
                        this.previous_drag_coordinates = Some(e.event.position);
                        let drag = e.drag(cx);
                        this.resize_split(drag.split_index, drag.axis, e.event.position, cx);
                    }
                },
            ))
            .child(self.render_node(&self.root, cx))
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_tab_item_creation() {
        // Note: We can't test view creation without GPUI context
        // This is a placeholder for the structure test
    }
}
