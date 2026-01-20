# Feature 03: Frontend Architecture

> **Status:** Not Started
> **Dependencies:** 01-project-initialization
> **Estimated Complexity:** High

## Overview

This document specifies the GPUI frontend architecture for Tusk, implementing a workspace-based UI with docks, panes, and panels. The architecture follows patterns established in Zed while adapting them for a database client workflow.

## Goals

1. Implement workspace shell with sidebar, content area, and status bar
2. Create resizable dock system for panels (schema browser, query results)
3. Build tab management for multiple query editors
4. Establish component patterns using GPUI's Render trait
5. Implement keyboard-driven navigation throughout

## Non-Goals

1. Plugin/extension UI (out of scope)
2. Multiple window support (single window application)
3. Custom window decorations (use native)

---

## 1. Application Shell Architecture

### 1.1 Workspace Structure

The workspace is the root UI component containing all visual elements:

```
┌─────────────────────────────────────────────────────────────────┐
│                         Title Bar                                │
├─────────┬───────────────────────────────────────┬───────────────┤
│         │              Tab Bar                   │               │
│         ├───────────────────────────────────────┤               │
│  Left   │                                       │    Right      │
│  Dock   │           Content Area                │    Dock       │
│         │         (Editor Panes)                │               │
│         │                                       │               │
├─────────┴───────────────────────────────────────┴───────────────┤
│                        Bottom Dock                               │
│                    (Results / Messages)                          │
├─────────────────────────────────────────────────────────────────┤
│                        Status Bar                                │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Core Workspace Implementation

```rust
// crates/tusk_ui/src/workspace.rs

use gpui::*;
use std::sync::Arc;

use crate::{
    dock::{Dock, DockPosition},
    pane::{Pane, PaneGroup},
    status_bar::StatusBar,
    tab_bar::TabBar,
    theme::Theme,
};
use tusk_core::TuskState;

/// Root workspace containing all UI elements
pub struct Workspace {
    /// Application state
    state: Entity<TuskState>,

    /// Left dock (schema browser, connections)
    left_dock: Entity<Dock>,

    /// Right dock (object details, inspector)
    right_dock: Option<Entity<Dock>>,

    /// Bottom dock (query results, messages)
    bottom_dock: Entity<Dock>,

    /// Main content area with editor panes
    center: Entity<PaneGroup>,

    /// Status bar at bottom
    status_bar: Entity<StatusBar>,

    /// Currently focused element
    focus_handle: FocusHandle,

    /// Workspace dimensions for layout calculations
    bounds: Bounds<Pixels>,
}

impl Workspace {
    pub fn new(state: Entity<TuskState>, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Create docks
        let left_dock = cx.new(|cx| {
            Dock::new(DockPosition::Left, px(280.0), cx)
        });

        let bottom_dock = cx.new(|cx| {
            Dock::new(DockPosition::Bottom, px(300.0), cx)
        });

        // Create center pane group with initial pane
        let center = cx.new(|cx| {
            PaneGroup::new(cx)
        });

        // Create status bar
        let status_bar = cx.new(|cx| {
            StatusBar::new(state.clone(), cx)
        });

        Self {
            state,
            left_dock,
            right_dock: None,
            bottom_dock,
            center,
            status_bar,
            focus_handle,
            bounds: Bounds::default(),
        }
    }

    /// Get the active pane in the center area
    pub fn active_pane(&self, cx: &App) -> Entity<Pane> {
        self.center.read(cx).active_pane()
    }

    /// Toggle dock visibility
    pub fn toggle_dock(&mut self, position: DockPosition, cx: &mut Context<Self>) {
        match position {
            DockPosition::Left => {
                self.left_dock.update(cx, |dock, cx| {
                    dock.toggle_visibility(cx);
                });
            }
            DockPosition::Right => {
                if let Some(dock) = &self.right_dock {
                    dock.update(cx, |dock, cx| {
                        dock.toggle_visibility(cx);
                    });
                }
            }
            DockPosition::Bottom => {
                self.bottom_dock.update(cx, |dock, cx| {
                    dock.toggle_visibility(cx);
                });
            }
        }
        cx.notify();
    }

    /// Focus the center editor area
    pub fn focus_center(&mut self, cx: &mut Context<Self>) {
        self.center.update(cx, |center, cx| {
            center.focus_active_pane(cx);
        });
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id("workspace")
            .key_context("Workspace")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_toggle_left_dock))
            .on_action(cx.listener(Self::handle_toggle_bottom_dock))
            .on_action(cx.listener(Self::handle_new_query_tab))
            .on_action(cx.listener(Self::handle_close_tab))
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.background)
            .text_color(theme.colors.text)
            .font_family(theme.ui_font_family.clone())
            .font_size(theme.ui_font_size)
            .child(self.render_main_area(window, cx))
            .child(self.status_bar.clone())
    }
}

impl Workspace {
    fn render_main_area(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let left_dock_visible = self.left_dock.read(cx).is_visible();
        let bottom_dock_visible = self.bottom_dock.read(cx).is_visible();

        div()
            .id("main-area")
            .flex_1()
            .flex()
            .flex_row()
            .overflow_hidden()
            // Left dock
            .when(left_dock_visible, |this| {
                this.child(self.left_dock.clone())
            })
            // Center + Bottom
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    // Center content area
                    .child(self.render_center_area(window, cx))
                    // Bottom dock
                    .when(bottom_dock_visible, |this| {
                        this.child(self.bottom_dock.clone())
                    })
            )
    }

    fn render_center_area(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id("center-area")
            .flex_1()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(theme.colors.editor_background)
            .child(self.center.clone())
    }

    // Action handlers
    fn handle_toggle_left_dock(
        &mut self,
        _: &ToggleLeftDock,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_dock(DockPosition::Left, cx);
    }

    fn handle_toggle_bottom_dock(
        &mut self,
        _: &ToggleBottomDock,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_dock(DockPosition::Bottom, cx);
    }

    fn handle_new_query_tab(
        &mut self,
        _: &NewQueryTab,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.center.update(cx, |center, cx| {
            center.add_query_tab(cx);
        });
    }

    fn handle_close_tab(
        &mut self,
        _: &CloseTab,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.center.update(cx, |center, cx| {
            center.close_active_tab(cx);
        });
    }
}

// Actions
actions!(workspace, [
    ToggleLeftDock,
    ToggleBottomDock,
    ToggleRightDock,
    NewQueryTab,
    CloseTab,
    FocusEditor,
    FocusResults,
    FocusSidebar,
]);
```

---

## 2. Dock System

### 2.1 Dock Component

Docks are resizable containers that hold panels:

```rust
// crates/tusk_ui/src/dock.rs

use gpui::*;
use crate::{panel::Panel, resizer::Resizer, theme::Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockPosition {
    Left,
    Right,
    Bottom,
}

pub struct Dock {
    position: DockPosition,
    size: Pixels,
    min_size: Pixels,
    max_size: Pixels,
    visible: bool,
    panels: Vec<Entity<dyn Panel>>,
    active_panel_index: usize,
    focus_handle: FocusHandle,
}

impl Dock {
    pub fn new(position: DockPosition, initial_size: Pixels, cx: &mut Context<Self>) -> Self {
        let (min_size, max_size) = match position {
            DockPosition::Left | DockPosition::Right => (px(200.0), px(600.0)),
            DockPosition::Bottom => (px(150.0), px(500.0)),
        };

        Self {
            position,
            size: initial_size,
            min_size,
            max_size,
            visible: true,
            panels: Vec::new(),
            active_panel_index: 0,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn toggle_visibility(&mut self, cx: &mut Context<Self>) {
        self.visible = !self.visible;
        cx.notify();
    }

    pub fn add_panel(&mut self, panel: Entity<dyn Panel>, cx: &mut Context<Self>) {
        self.panels.push(panel);
        cx.notify();
    }

    pub fn set_size(&mut self, size: Pixels, cx: &mut Context<Self>) {
        self.size = size.clamp(self.min_size, self.max_size);
        cx.notify();
    }

    pub fn active_panel(&self) -> Option<&Entity<dyn Panel>> {
        self.panels.get(self.active_panel_index)
    }
}

impl Render for Dock {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().into_any_element();
        }

        let theme = cx.global::<Theme>();
        let position = self.position;
        let size = self.size;

        let resizer = cx.new(|cx| {
            Resizer::new(position, cx)
        });

        let content = div()
            .flex_1()
            .overflow_hidden()
            .children(self.panels.iter().enumerate().filter_map(|(i, panel)| {
                if i == self.active_panel_index {
                    Some(panel.clone().into_any_element())
                } else {
                    None
                }
            }));

        match position {
            DockPosition::Left => {
                div()
                    .id("left-dock")
                    .flex()
                    .flex_row()
                    .h_full()
                    .w(size)
                    .bg(theme.colors.panel_background)
                    .border_r_1()
                    .border_color(theme.colors.border)
                    .child(content)
                    .child(resizer)
            }
            DockPosition::Right => {
                div()
                    .id("right-dock")
                    .flex()
                    .flex_row()
                    .h_full()
                    .w(size)
                    .bg(theme.colors.panel_background)
                    .border_l_1()
                    .border_color(theme.colors.border)
                    .child(resizer)
                    .child(content)
            }
            DockPosition::Bottom => {
                div()
                    .id("bottom-dock")
                    .flex()
                    .flex_col()
                    .w_full()
                    .h(size)
                    .bg(theme.colors.panel_background)
                    .border_t_1()
                    .border_color(theme.colors.border)
                    .child(resizer)
                    .child(content)
            }
        }
        .into_any_element()
    }
}
```

### 2.2 Resizer Component

```rust
// crates/tusk_ui/src/resizer.rs

use gpui::*;
use crate::{dock::DockPosition, theme::Theme};

pub struct Resizer {
    position: DockPosition,
    dragging: bool,
    hover: bool,
}

impl Resizer {
    pub fn new(position: DockPosition, _cx: &mut Context<Self>) -> Self {
        Self {
            position,
            dragging: false,
            hover: false,
        }
    }
}

impl Render for Resizer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let position = self.position;
        let is_active = self.dragging || self.hover;

        let (width, height, cursor) = match position {
            DockPosition::Left | DockPosition::Right => {
                (px(4.0), relative(1.0), CursorStyle::ResizeLeftRight)
            }
            DockPosition::Bottom => {
                (relative(1.0), px(4.0), CursorStyle::ResizeUpDown)
            }
        };

        div()
            .id("resizer")
            .w(width)
            .h(height)
            .cursor(cursor)
            .bg(if is_active {
                theme.colors.accent
            } else {
                theme.colors.border
            })
            .hover(|style| style.bg(theme.colors.accent.opacity(0.5)))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.dragging = true;
                cx.notify();
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.dragging = false;
                cx.notify();
            }))
            .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
                if this.dragging {
                    // Emit resize event to parent dock
                    cx.emit(ResizeEvent {
                        position,
                        delta: match position {
                            DockPosition::Left => event.position.x,
                            DockPosition::Right => -event.position.x,
                            DockPosition::Bottom => -event.position.y,
                        },
                    });
                }
            }))
            .on_hover(cx.listener(|this, hovering, _, cx| {
                this.hover = *hovering;
                cx.notify();
            }))
    }
}

#[derive(Debug, Clone)]
pub struct ResizeEvent {
    pub position: DockPosition,
    pub delta: Pixels,
}

impl EventEmitter<ResizeEvent> for Resizer {}
```

---

## 3. Pane and Tab System

### 3.1 Pane Group

The pane group manages multiple panes with split support:

```rust
// crates/tusk_ui/src/pane.rs

use gpui::*;
use smallvec::SmallVec;
use crate::{
    tab_bar::TabBar,
    theme::Theme,
};
use tusk_editor::QueryEditor;

/// Axis for pane splits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// A group of panes that can be split
pub struct PaneGroup {
    root: PaneNode,
    active_pane: Entity<Pane>,
}

enum PaneNode {
    Single(Entity<Pane>),
    Split {
        axis: Axis,
        children: SmallVec<[PaneNode; 2]>,
        ratios: SmallVec<[f32; 2]>,
    },
}

impl PaneGroup {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let initial_pane = cx.new(|cx| Pane::new(cx));

        Self {
            root: PaneNode::Single(initial_pane.clone()),
            active_pane: initial_pane,
        }
    }

    pub fn active_pane(&self) -> Entity<Pane> {
        self.active_pane.clone()
    }

    pub fn focus_active_pane(&self, cx: &mut App) {
        self.active_pane.update(cx, |pane, cx| {
            pane.focus(cx);
        });
    }

    pub fn add_query_tab(&mut self, cx: &mut Context<Self>) {
        self.active_pane.update(cx, |pane, cx| {
            pane.add_query_tab(None, cx);
        });
    }

    pub fn close_active_tab(&mut self, cx: &mut Context<Self>) {
        self.active_pane.update(cx, |pane, cx| {
            pane.close_active_tab(cx);
        });
    }

    /// Split the active pane along an axis
    pub fn split_pane(&mut self, axis: Axis, cx: &mut Context<Self>) {
        let new_pane = cx.new(|cx| Pane::new(cx));

        // Find and split the node containing the active pane
        self.root = self.split_node(
            std::mem::replace(&mut self.root, PaneNode::Single(new_pane.clone())),
            &self.active_pane,
            axis,
            new_pane.clone(),
        );

        self.active_pane = new_pane;
        cx.notify();
    }

    fn split_node(
        &self,
        node: PaneNode,
        target: &Entity<Pane>,
        axis: Axis,
        new_pane: Entity<Pane>,
    ) -> PaneNode {
        match node {
            PaneNode::Single(pane) if &pane == target => {
                PaneNode::Split {
                    axis,
                    children: smallvec![
                        PaneNode::Single(pane),
                        PaneNode::Single(new_pane),
                    ],
                    ratios: smallvec![0.5, 0.5],
                }
            }
            PaneNode::Single(pane) => PaneNode::Single(pane),
            PaneNode::Split { axis: existing_axis, children, ratios } => {
                PaneNode::Split {
                    axis: existing_axis,
                    children: children
                        .into_iter()
                        .map(|child| self.split_node(child, target, axis, new_pane.clone()))
                        .collect(),
                    ratios,
                }
            }
        }
    }
}

impl Render for PaneGroup {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_node(&self.root, window, cx)
    }
}

impl PaneGroup {
    fn render_node(
        &self,
        node: &PaneNode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        match node {
            PaneNode::Single(pane) => {
                div()
                    .size_full()
                    .child(pane.clone())
                    .into_any_element()
            }
            PaneNode::Split { axis, children, ratios } => {
                let mut container = div().size_full().flex();

                container = match axis {
                    Axis::Horizontal => container.flex_row(),
                    Axis::Vertical => container.flex_col(),
                };

                for (i, (child, ratio)) in children.iter().zip(ratios.iter()).enumerate() {
                    let child_element = self.render_node(child, window, cx);

                    container = container.child(
                        div()
                            .flex_basis(relative(*ratio))
                            .flex_grow()
                            .flex_shrink()
                            .overflow_hidden()
                            .child(child_element)
                    );

                    // Add resizer between panes (except after last)
                    if i < children.len() - 1 {
                        container = container.child(self.render_pane_resizer(*axis, cx));
                    }
                }

                container.into_any_element()
            }
        }
    }

    fn render_pane_resizer(&self, axis: Axis, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let (width, height, cursor) = match axis {
            Axis::Horizontal => (px(4.0), relative(1.0), CursorStyle::ResizeLeftRight),
            Axis::Vertical => (relative(1.0), px(4.0), CursorStyle::ResizeUpDown),
        };

        div()
            .w(width)
            .h(height)
            .cursor(cursor)
            .bg(theme.colors.border)
            .hover(|style| style.bg(theme.colors.accent))
    }
}
```

### 3.2 Pane Component

```rust
// crates/tusk_ui/src/pane.rs (continued)

use uuid::Uuid;
use std::collections::HashMap;

/// Tab item data
#[derive(Clone)]
pub struct TabItem {
    pub id: Uuid,
    pub title: SharedString,
    pub icon: Option<IconName>,
    pub dirty: bool,
    pub closable: bool,
}

/// A single pane containing tabs
pub struct Pane {
    tabs: Vec<TabItem>,
    editors: HashMap<Uuid, Entity<QueryEditor>>,
    active_tab_index: usize,
    focus_handle: FocusHandle,
}

impl Pane {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut pane = Self {
            tabs: Vec::new(),
            editors: HashMap::new(),
            active_tab_index: 0,
            focus_handle: cx.focus_handle(),
        };

        // Start with one empty query tab
        pane.add_query_tab(None, cx);
        pane
    }

    pub fn focus(&self, cx: &mut App) {
        self.focus_handle.focus(cx);
    }

    pub fn add_query_tab(&mut self, connection_id: Option<Uuid>, cx: &mut Context<Self>) {
        let id = Uuid::new_v4();
        let tab_number = self.tabs.len() + 1;

        let editor = cx.new(|cx| {
            QueryEditor::new(connection_id, cx)
        });

        self.tabs.push(TabItem {
            id,
            title: format!("Query {}", tab_number).into(),
            icon: Some(IconName::Code),
            dirty: false,
            closable: true,
        });

        self.editors.insert(id, editor);
        self.active_tab_index = self.tabs.len() - 1;
        cx.notify();
    }

    pub fn close_active_tab(&mut self, cx: &mut Context<Self>) {
        if self.tabs.len() <= 1 {
            return; // Keep at least one tab
        }

        let tab = self.tabs.remove(self.active_tab_index);
        self.editors.remove(&tab.id);

        if self.active_tab_index >= self.tabs.len() {
            self.active_tab_index = self.tabs.len().saturating_sub(1);
        }

        cx.notify();
    }

    pub fn close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.tabs.len() <= 1 || index >= self.tabs.len() {
            return;
        }

        let tab = self.tabs.remove(index);
        self.editors.remove(&tab.id);

        if self.active_tab_index >= self.tabs.len() {
            self.active_tab_index = self.tabs.len().saturating_sub(1);
        } else if index < self.active_tab_index {
            self.active_tab_index -= 1;
        }

        cx.notify();
    }

    pub fn activate_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.tabs.len() {
            self.active_tab_index = index;
            cx.notify();
        }
    }

    fn active_editor(&self) -> Option<&Entity<QueryEditor>> {
        self.tabs.get(self.active_tab_index)
            .and_then(|tab| self.editors.get(&tab.id))
    }
}

impl Render for Pane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id("pane")
            .key_context("Pane")
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.editor_background)
            .child(self.render_tab_bar(cx))
            .child(self.render_content(cx))
    }
}

impl Pane {
    fn render_tab_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id("tab-bar")
            .w_full()
            .h(px(36.0))
            .flex()
            .flex_row()
            .items_center()
            .bg(theme.colors.tab_bar_background)
            .border_b_1()
            .border_color(theme.colors.border)
            .children(
                self.tabs.iter().enumerate().map(|(index, tab)| {
                    self.render_tab(tab, index, cx)
                })
            )
            .child(self.render_new_tab_button(cx))
    }

    fn render_tab(&self, tab: &TabItem, index: usize, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let is_active = index == self.active_tab_index;
        let tab_id = tab.id;

        div()
            .id(ElementId::Name(format!("tab-{}", index).into()))
            .h_full()
            .px_3()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .bg(if is_active {
                theme.colors.tab_active_background
            } else {
                theme.colors.tab_inactive_background
            })
            .border_r_1()
            .border_color(theme.colors.border)
            .hover(|style| {
                if !is_active {
                    style.bg(theme.colors.tab_hover_background)
                } else {
                    style
                }
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.activate_tab(index, cx);
            }))
            // Tab icon
            .when_some(tab.icon, |this, icon| {
                this.child(
                    Icon::new(icon)
                        .size(IconSize::Small)
                        .color(if is_active {
                            theme.colors.text
                        } else {
                            theme.colors.text_muted
                        })
                )
            })
            // Tab title
            .child(
                div()
                    .text_sm()
                    .text_color(if is_active {
                        theme.colors.text
                    } else {
                        theme.colors.text_muted
                    })
                    .child(tab.title.clone())
            )
            // Dirty indicator
            .when(tab.dirty, |this| {
                this.child(
                    div()
                        .size_2()
                        .rounded_full()
                        .bg(theme.colors.warning)
                )
            })
            // Close button
            .when(tab.closable, |this| {
                this.child(
                    div()
                        .id("close-button")
                        .size_4()
                        .rounded_sm()
                        .flex()
                        .items_center()
                        .justify_center()
                        .hover(|style| style.bg(theme.colors.ghost_element_hover))
                        .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                            event.stop_propagation();
                            this.close_tab(index, cx);
                        }))
                        .child(Icon::new(IconName::Close).size(IconSize::XSmall))
                )
            })
    }

    fn render_new_tab_button(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id("new-tab-button")
            .size_7()
            .mx_1()
            .rounded_md()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(theme.colors.ghost_element_hover))
            .on_click(cx.listener(|this, _, _, cx| {
                this.add_query_tab(None, cx);
            }))
            .child(Icon::new(IconName::Plus).size(IconSize::Small))
    }

    fn render_content(&self, cx: &Context<Self>) -> impl IntoElement {
        div()
            .id("pane-content")
            .flex_1()
            .overflow_hidden()
            .when_some(self.active_editor(), |this, editor| {
                this.child(editor.clone())
            })
    }
}
```

---

## 4. Status Bar

### 4.1 Status Bar Component

```rust
// crates/tusk_ui/src/status_bar.rs

use gpui::*;
use crate::theme::Theme;
use tusk_core::TuskState;

pub struct StatusBar {
    state: Entity<TuskState>,
}

impl StatusBar {
    pub fn new(state: Entity<TuskState>, _cx: &mut Context<Self>) -> Self {
        Self { state }
    }
}

impl Render for StatusBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let state = self.state.read(cx);

        div()
            .id("status-bar")
            .w_full()
            .h(px(24.0))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px_2()
            .bg(theme.colors.status_bar_background)
            .border_t_1()
            .border_color(theme.colors.border)
            .text_xs()
            .text_color(theme.colors.text_muted)
            .child(self.render_left_items(state, cx))
            .child(self.render_right_items(state, cx))
    }
}

impl StatusBar {
    fn render_left_items(
        &self,
        state: &TuskState,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_3()
            // Connection status
            .child(self.render_connection_status(state, cx))
            // Query status (if running)
            .when_some(state.active_query_status(), |this, status| {
                this.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .child(Spinner::new().size(SpinnerSize::Small))
                        .child(status)
                )
            })
    }

    fn render_connection_status(
        &self,
        state: &TuskState,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        if let Some(conn) = state.active_connection() {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .size_2()
                        .rounded_full()
                        .bg(theme.colors.success)
                )
                .child(format!(
                    "{}@{}:{}/{}",
                    conn.user, conn.host, conn.port, conn.database
                ))
        } else {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .size_2()
                        .rounded_full()
                        .bg(theme.colors.text_muted)
                )
                .child("Not connected")
        }
    }

    fn render_right_items(
        &self,
        state: &TuskState,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_3()
            // Row count (if results visible)
            .when_some(state.last_query_row_count(), |this, count| {
                this.child(format!("{} rows", count))
            })
            // Execution time
            .when_some(state.last_query_duration(), |this, duration| {
                this.child(format!("{:.2}s", duration.as_secs_f64()))
            })
            // Cursor position
            .when_some(state.cursor_position(), |this, pos| {
                this.child(format!("Ln {}, Col {}", pos.line, pos.column))
            })
    }
}
```

---

## 5. Panel Trait

### 5.1 Panel Trait Definition

All dock panels implement this trait:

```rust
// crates/tusk_ui/src/panel.rs

use gpui::*;

/// Trait for panels that can be placed in docks
pub trait Panel: Render + EventEmitter<PanelEvent> {
    /// Unique identifier for this panel type
    fn panel_id(&self) -> &'static str;

    /// Display name shown in panel header
    fn title(&self, cx: &App) -> SharedString;

    /// Icon for panel tab
    fn icon(&self) -> IconName;

    /// Whether this panel can be closed
    fn closable(&self) -> bool {
        true
    }

    /// Focus the panel's primary element
    fn focus(&self, cx: &mut App);

    /// Whether the panel has unsaved changes
    fn is_dirty(&self, cx: &App) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
pub enum PanelEvent {
    Focus,
    Close,
    ActivateTab(usize),
}
```

### 5.2 Example Panel: Schema Browser

```rust
// crates/tusk_ui/src/panels/schema_browser.rs

use gpui::*;
use crate::{
    panel::{Panel, PanelEvent},
    theme::Theme,
    tree::{Tree, TreeItem},
};
use tusk_core::TuskState;
use tusk_db::schema::SchemaObject;

pub struct SchemaBrowserPanel {
    state: Entity<TuskState>,
    tree: Entity<Tree<SchemaObject>>,
    search_query: String,
    focus_handle: FocusHandle,
}

impl SchemaBrowserPanel {
    pub fn new(state: Entity<TuskState>, cx: &mut Context<Self>) -> Self {
        let tree = cx.new(|cx| Tree::new(cx));

        Self {
            state,
            tree,
            search_query: String::new(),
            focus_handle: cx.focus_handle(),
        }
    }

    fn refresh_schema(&mut self, cx: &mut Context<Self>) {
        let state = self.state.clone();

        cx.spawn(async move |this, cx| {
            let schema = state.read(&cx).schema_service()
                .load_schema()
                .await?;

            this.update(&cx, |this, cx| {
                this.tree.update(cx, |tree, cx| {
                    tree.set_items(schema.to_tree_items(), cx);
                });
            })?;

            Ok(())
        }).detach();
    }
}

impl Panel for SchemaBrowserPanel {
    fn panel_id(&self) -> &'static str {
        "schema_browser"
    }

    fn title(&self, _cx: &App) -> SharedString {
        "Schema".into()
    }

    fn icon(&self) -> IconName {
        IconName::Database
    }

    fn focus(&self, cx: &mut App) {
        self.focus_handle.focus(cx);
    }
}

impl EventEmitter<PanelEvent> for SchemaBrowserPanel {}

impl Render for SchemaBrowserPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id("schema-browser")
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.panel_background)
            // Search bar
            .child(self.render_search_bar(cx))
            // Schema tree
            .child(
                div()
                    .flex_1()
                    .overflow_y_scroll()
                    .child(self.tree.clone())
            )
    }
}

impl SchemaBrowserPanel {
    fn render_search_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .w_full()
            .p_2()
            .child(
                div()
                    .w_full()
                    .h_7()
                    .px_2()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .rounded_md()
                    .bg(theme.colors.input_background)
                    .border_1()
                    .border_color(theme.colors.border)
                    .child(Icon::new(IconName::Search).size(IconSize::Small))
                    .child(
                        input()
                            .flex_1()
                            .bg(transparent())
                            .border_0()
                            .placeholder("Filter objects...")
                            .on_input(cx.listener(|this, text, _, cx| {
                                this.search_query = text.to_string();
                                cx.notify();
                            }))
                    )
            )
    }
}
```

---

## 6. Tree Component

### 6.1 Generic Tree View

```rust
// crates/tusk_ui/src/tree.rs

use gpui::*;
use std::collections::HashSet;
use std::hash::Hash;
use crate::theme::Theme;

/// Item that can be displayed in a tree
pub trait TreeItem: Clone + 'static {
    type Id: Clone + Eq + Hash;

    fn id(&self) -> Self::Id;
    fn label(&self) -> SharedString;
    fn icon(&self) -> Option<IconName>;
    fn children(&self) -> Option<&[Self]>;
    fn is_expandable(&self) -> bool {
        self.children().map(|c| !c.is_empty()).unwrap_or(false)
    }
}

pub struct Tree<T: TreeItem> {
    items: Vec<T>,
    expanded: HashSet<T::Id>,
    selected: Option<T::Id>,
    focus_handle: FocusHandle,
}

impl<T: TreeItem> Tree<T> {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            items: Vec::new(),
            expanded: HashSet::new(),
            selected: None,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn set_items(&mut self, items: Vec<T>, cx: &mut Context<Self>) {
        self.items = items;
        cx.notify();
    }

    pub fn toggle_expanded(&mut self, id: &T::Id, cx: &mut Context<Self>) {
        if self.expanded.contains(id) {
            self.expanded.remove(id);
        } else {
            self.expanded.insert(id.clone());
        }
        cx.notify();
    }

    pub fn select(&mut self, id: T::Id, cx: &mut Context<Self>) {
        self.selected = Some(id);
        cx.notify();
    }
}

impl<T: TreeItem> Render for Tree<T> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id("tree")
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .children(
                self.items.iter().map(|item| {
                    self.render_item(item, 0, cx)
                })
            )
    }
}

impl<T: TreeItem> Tree<T> {
    fn render_item(
        &self,
        item: &T,
        depth: usize,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let id = item.id();
        let is_expanded = self.expanded.contains(&id);
        let is_selected = self.selected.as_ref() == Some(&id);
        let is_expandable = item.is_expandable();

        let indent = px(depth as f32 * 16.0 + 4.0);

        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .id(ElementId::Name(format!("tree-item-{:?}", id).into()))
                    .w_full()
                    .h_6()
                    .pl(indent)
                    .pr_2()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .cursor_pointer()
                    .rounded_sm()
                    .bg(if is_selected {
                        theme.colors.list_active_selection_background
                    } else {
                        transparent()
                    })
                    .hover(|style| {
                        if !is_selected {
                            style.bg(theme.colors.list_hover_background)
                        } else {
                            style
                        }
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if is_expandable {
                            this.toggle_expanded(&id, cx);
                        }
                        this.select(id.clone(), cx);
                    }))
                    // Expand/collapse indicator
                    .child(
                        div()
                            .size_4()
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(is_expandable, |this| {
                                this.child(
                                    Icon::new(if is_expanded {
                                        IconName::ChevronDown
                                    } else {
                                        IconName::ChevronRight
                                    })
                                    .size(IconSize::XSmall)
                                    .color(theme.colors.text_muted)
                                )
                            })
                    )
                    // Item icon
                    .when_some(item.icon(), |this, icon| {
                        this.child(
                            Icon::new(icon)
                                .size(IconSize::Small)
                                .color(theme.colors.text_muted)
                        )
                    })
                    // Item label
                    .child(
                        div()
                            .text_sm()
                            .truncate()
                            .child(item.label())
                    )
            )
            // Render children if expanded
            .when(is_expanded && is_expandable, |this| {
                if let Some(children) = item.children() {
                    this.children(
                        children.iter().map(|child| {
                            self.render_item(child, depth + 1, cx)
                        })
                    )
                } else {
                    this
                }
            })
    }
}
```

---

## 7. Icon System

### 7.1 Icon Component

```rust
// crates/tusk_ui/src/icon.rs

use gpui::*;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconName {
    // Navigation
    ChevronRight,
    ChevronDown,
    ChevronLeft,
    ChevronUp,

    // Actions
    Plus,
    Close,
    Search,
    Refresh,
    Play,
    Stop,
    Save,
    Copy,
    Paste,

    // Objects
    Database,
    Table,
    Column,
    Key,
    Index,
    View,
    Function,
    Schema,
    Folder,
    File,
    Code,

    // Status
    Check,
    Warning,
    Error,
    Info,

    // UI
    Menu,
    Settings,
    VerticalDots,
    HorizontalDots,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconSize {
    XSmall,  // 12px
    Small,   // 14px
    Medium,  // 16px
    Large,   // 20px
    XLarge,  // 24px
}

impl IconSize {
    fn pixels(self) -> Pixels {
        match self {
            IconSize::XSmall => px(12.0),
            IconSize::Small => px(14.0),
            IconSize::Medium => px(16.0),
            IconSize::Large => px(20.0),
            IconSize::XLarge => px(24.0),
        }
    }
}

pub struct Icon {
    name: IconName,
    size: IconSize,
    color: Option<Hsla>,
}

impl Icon {
    pub fn new(name: IconName) -> Self {
        Self {
            name,
            size: IconSize::Medium,
            color: None,
        }
    }

    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let size = self.size.pixels();
        let color = self.color.unwrap_or(theme.colors.text);

        // SVG path data for each icon
        let path = self.name.svg_path();

        svg()
            .size(size)
            .text_color(color)
            .path(path)
    }
}

impl IconName {
    fn svg_path(self) -> &'static str {
        match self {
            IconName::ChevronRight => "M9 18l6-6-6-6",
            IconName::ChevronDown => "M6 9l6 6 6-6",
            IconName::ChevronLeft => "M15 18l-6-6 6-6",
            IconName::ChevronUp => "M18 15l-6-6-6 6",
            IconName::Plus => "M12 5v14m-7-7h14",
            IconName::Close => "M6 18L18 6M6 6l12 12",
            IconName::Search => "M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z",
            IconName::Database => "M4 7v10c0 2 4 4 8 4s8-2 8-4V7M4 7c0 2 4 4 8 4s8-2 8-4M4 7c0-2 4-4 8-4s8 2 8 4m0 5c0 2-4 4-8 4s-8-2-8-4",
            IconName::Table => "M3 10h18M3 14h18M10 3v18M14 3v18M3 6a3 3 0 013-3h12a3 3 0 013 3v12a3 3 0 01-3 3H6a3 3 0 01-3-3V6z",
            IconName::Play => "M5 3l14 9-14 9V3z",
            IconName::Stop => "M6 4h4v16H6V4zm8 0h4v16h-4V4z",
            IconName::Code => "M16 18l6-6-6-6M8 6l-6 6 6 6",
            IconName::Refresh => "M4 4v5h5M20 20v-5h-5M20.49 9A9 9 0 005.64 5.64L4 4m16 16l-1.64-1.64A9 9 0 013.51 15",
            IconName::Save => "M17 21H7a2 2 0 01-2-2V5a2 2 0 012-2h7l5 5v11a2 2 0 01-2 2zM15 3v4a2 2 0 002 2h4",
            IconName::Copy => "M16 4h2a2 2 0 012 2v14a2 2 0 01-2 2H6a2 2 0 01-2-2V6a2 2 0 012-2h2m4-2h4a1 1 0 011 1v2a1 1 0 01-1 1h-4a1 1 0 01-1-1V3a1 1 0 011-1z",
            IconName::Paste => "M16 4h2a2 2 0 012 2v1m-4-3V3a1 1 0 00-1-1h-4a1 1 0 00-1 1v1m6 0H8m8 0v1H8V4m-4 2a2 2 0 012-2h2M4 6v14a2 2 0 002 2h12a2 2 0 002-2v-1",
            IconName::Column => "M9 3v18m6-18v18M3 9h18M3 15h18",
            IconName::Key => "M21 2l-2 2m-7.61 7.61a5.5 5.5 0 11-7.778 7.778 5.5 5.5 0 017.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4",
            IconName::Index => "M4 6h16M4 12h10M4 18h6",
            IconName::View => "M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8zm11 3a3 3 0 100-6 3 3 0 000 6z",
            IconName::Function => "M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8l-6-6zm-1 9h-2v2H9v-2H7v-2h2V7h2v2h2v2z",
            IconName::Schema => "M4 7v10c0 2 4 4 8 4s8-2 8-4V7c0-2-4-4-8-4s-8 2-8 4z",
            IconName::Folder => "M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z",
            IconName::File => "M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8l-6-6z",
            IconName::Check => "M20 6L9 17l-5-5",
            IconName::Warning => "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z",
            IconName::Error => "M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z",
            IconName::Info => "M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
            IconName::Menu => "M4 6h16M4 12h16m-7 6h7",
            IconName::Settings => "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z",
            IconName::VerticalDots => "M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z",
            IconName::HorizontalDots => "M5 12h.01M12 12h.01M19 12h.01M6 12a1 1 0 11-2 0 1 1 0 012 0zm7 0a1 1 0 11-2 0 1 1 0 012 0zm7 0a1 1 0 11-2 0 1 1 0 012 0z",
        }
    }
}
```

---

## 8. Input Components

### 8.1 Text Input

```rust
// crates/tusk_ui/src/input.rs

use gpui::*;
use crate::theme::Theme;

pub struct TextInput {
    value: String,
    placeholder: SharedString,
    disabled: bool,
    password: bool,
    focus_handle: FocusHandle,
    on_change: Option<Box<dyn Fn(&str, &mut App) + 'static>>,
    on_submit: Option<Box<dyn Fn(&str, &mut App) + 'static>>,
}

impl TextInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            value: String::new(),
            placeholder: SharedString::default(),
            disabled: false,
            password: false,
            focus_handle: cx.focus_handle(),
            on_change: None,
            on_submit: None,
        }
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(&str, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    pub fn on_submit(mut self, handler: impl Fn(&str, &mut App) + 'static) -> Self {
        self.on_submit = Some(Box::new(handler));
        self
    }

    fn handle_input(&mut self, event: &InputEvent, cx: &mut Context<Self>) {
        self.value = event.text.to_string();
        if let Some(on_change) = &self.on_change {
            on_change(&self.value, cx);
        }
        cx.notify();
    }

    fn handle_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if event.keystroke.key == "enter" {
            if let Some(on_submit) = &self.on_submit {
                on_submit(&self.value, cx);
            }
        }
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let is_focused = self.focus_handle.is_focused(cx);

        div()
            .id("text-input")
            .track_focus(&self.focus_handle)
            .w_full()
            .h_8()
            .px_3()
            .flex()
            .items_center()
            .rounded_md()
            .bg(theme.colors.input_background)
            .border_1()
            .border_color(if is_focused {
                theme.colors.accent
            } else {
                theme.colors.border
            })
            .when(self.disabled, |this| {
                this.opacity(0.5).cursor_not_allowed()
            })
            .on_key_down(cx.listener(Self::handle_key))
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(theme.colors.text)
                    .when(self.value.is_empty(), |this| {
                        this.child(
                            div()
                                .text_color(theme.colors.text_muted)
                                .child(self.placeholder.clone())
                        )
                    })
                    .when(!self.value.is_empty(), |this| {
                        this.child(if self.password {
                            "*".repeat(self.value.len())
                        } else {
                            self.value.clone()
                        })
                    })
            )
    }
}
```

### 8.2 Select/Dropdown

```rust
// crates/tusk_ui/src/select.rs

use gpui::*;
use crate::theme::Theme;

pub struct SelectOption<T: Clone> {
    pub value: T,
    pub label: SharedString,
    pub disabled: bool,
}

pub struct Select<T: Clone + PartialEq + 'static> {
    options: Vec<SelectOption<T>>,
    selected: Option<T>,
    placeholder: SharedString,
    open: bool,
    focus_handle: FocusHandle,
    on_change: Option<Box<dyn Fn(&T, &mut App) + 'static>>,
}

impl<T: Clone + PartialEq + 'static> Select<T> {
    pub fn new(options: Vec<SelectOption<T>>, cx: &mut Context<Self>) -> Self {
        Self {
            options,
            selected: None,
            placeholder: "Select...".into(),
            open: false,
            focus_handle: cx.focus_handle(),
            on_change: None,
        }
    }

    pub fn selected(mut self, value: T) -> Self {
        self.selected = Some(value);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn on_change(mut self, handler: impl Fn(&T, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    fn toggle_open(&mut self, cx: &mut Context<Self>) {
        self.open = !self.open;
        cx.notify();
    }

    fn select_option(&mut self, value: T, cx: &mut Context<Self>) {
        self.selected = Some(value.clone());
        self.open = false;
        if let Some(on_change) = &self.on_change {
            on_change(&value, cx);
        }
        cx.notify();
    }

    fn selected_label(&self) -> SharedString {
        self.selected.as_ref()
            .and_then(|v| {
                self.options.iter()
                    .find(|o| &o.value == v)
                    .map(|o| o.label.clone())
            })
            .unwrap_or_else(|| self.placeholder.clone())
    }
}

impl<T: Clone + PartialEq + 'static> Render for Select<T> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id("select")
            .relative()
            .w_full()
            .child(self.render_trigger(cx))
            .when(self.open, |this| {
                this.child(self.render_dropdown(cx))
            })
    }
}

impl<T: Clone + PartialEq + 'static> Select<T> {
    fn render_trigger(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let has_selection = self.selected.is_some();

        div()
            .id("select-trigger")
            .w_full()
            .h_8()
            .px_3()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .rounded_md()
            .bg(theme.colors.input_background)
            .border_1()
            .border_color(if self.open {
                theme.colors.accent
            } else {
                theme.colors.border
            })
            .on_click(cx.listener(|this, _, _, cx| {
                this.toggle_open(cx);
            }))
            .child(
                div()
                    .text_sm()
                    .text_color(if has_selection {
                        theme.colors.text
                    } else {
                        theme.colors.text_muted
                    })
                    .child(self.selected_label())
            )
            .child(
                Icon::new(if self.open {
                    IconName::ChevronUp
                } else {
                    IconName::ChevronDown
                })
                .size(IconSize::Small)
                .color(theme.colors.text_muted)
            )
    }

    fn render_dropdown(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id("select-dropdown")
            .absolute()
            .top_full()
            .left_0()
            .right_0()
            .mt_1()
            .rounded_md()
            .bg(theme.colors.elevated_surface_background)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .overflow_hidden()
            .z_index(100)
            .children(
                self.options.iter().map(|option| {
                    self.render_option(option, cx)
                })
            )
    }

    fn render_option(&self, option: &SelectOption<T>, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let is_selected = self.selected.as_ref() == Some(&option.value);
        let value = option.value.clone();

        div()
            .w_full()
            .px_3()
            .py_2()
            .cursor_pointer()
            .bg(if is_selected {
                theme.colors.list_active_selection_background
            } else {
                transparent()
            })
            .hover(|style| {
                if !is_selected {
                    style.bg(theme.colors.list_hover_background)
                } else {
                    style
                }
            })
            .when(option.disabled, |this| {
                this.opacity(0.5).cursor_not_allowed()
            })
            .when(!option.disabled, |this| {
                this.on_click(cx.listener(move |this, _, _, cx| {
                    this.select_option(value.clone(), cx);
                }))
            })
            .child(
                div()
                    .text_sm()
                    .child(option.label.clone())
            )
    }
}
```

---

## 9. Button Component

### 9.1 Button Variants

```rust
// crates/tusk_ui/src/button.rs

use gpui::*;
use crate::{icon::{Icon, IconName, IconSize}, theme::Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    Small,
    #[default]
    Medium,
    Large,
}

pub struct Button {
    label: Option<SharedString>,
    icon: Option<IconName>,
    icon_position: IconPosition,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    loading: bool,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut App) + 'static>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconPosition {
    #[default]
    Left,
    Right,
}

impl Button {
    pub fn new() -> Self {
        Self {
            label: None,
            icon: None,
            icon_position: IconPosition::Left,
            variant: ButtonVariant::Primary,
            size: ButtonSize::Medium,
            disabled: false,
            loading: false,
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn icon_position(mut self, position: IconPosition) -> Self {
        self.icon_position = position;
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let (height, padding, font_size, icon_size) = match self.size {
            ButtonSize::Small => (px(28.0), px(8.0), px(12.0), IconSize::XSmall),
            ButtonSize::Medium => (px(32.0), px(12.0), px(14.0), IconSize::Small),
            ButtonSize::Large => (px(40.0), px(16.0), px(16.0), IconSize::Medium),
        };

        let (bg, hover_bg, text_color) = match self.variant {
            ButtonVariant::Primary => (
                theme.colors.accent,
                theme.colors.accent_hover,
                theme.colors.on_accent,
            ),
            ButtonVariant::Secondary => (
                theme.colors.element_background,
                theme.colors.element_hover,
                theme.colors.text,
            ),
            ButtonVariant::Ghost => (
                transparent(),
                theme.colors.ghost_element_hover,
                theme.colors.text,
            ),
            ButtonVariant::Danger => (
                theme.colors.error,
                theme.colors.error.opacity(0.8),
                white(),
            ),
        };

        let disabled = self.disabled || self.loading;

        div()
            .id("button")
            .h(height)
            .px(padding)
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap_2()
            .rounded_md()
            .bg(bg)
            .text_color(text_color)
            .font_size(font_size)
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .when(disabled, |this| {
                this.opacity(0.5).cursor_not_allowed()
            })
            .when(!disabled, |this| {
                this.hover(|style| style.bg(hover_bg))
                    .when_some(self.on_click, |this, handler| {
                        this.on_click(move |event, _, cx| {
                            handler(event, cx);
                        })
                    })
            })
            // Loading spinner
            .when(self.loading, |this| {
                this.child(Spinner::new().size(SpinnerSize::Small))
            })
            // Icon (left)
            .when(!self.loading && self.icon.is_some() && self.icon_position == IconPosition::Left, |this| {
                this.child(Icon::new(self.icon.unwrap()).size(icon_size))
            })
            // Label
            .when_some(self.label.clone(), |this, label| {
                this.child(label)
            })
            // Icon (right)
            .when(!self.loading && self.icon.is_some() && self.icon_position == IconPosition::Right, |this| {
                this.child(Icon::new(self.icon.unwrap()).size(icon_size))
            })
    }
}
```

---

## 10. Dialog/Modal System

### 10.1 Modal Component

```rust
// crates/tusk_ui/src/modal.rs

use gpui::*;
use crate::{button::{Button, ButtonVariant}, icon::{Icon, IconName, IconSize}, theme::Theme};

pub struct Modal {
    title: SharedString,
    content: AnyElement,
    actions: Vec<ModalAction>,
    width: Pixels,
    closable: bool,
    focus_handle: FocusHandle,
}

pub struct ModalAction {
    pub label: SharedString,
    pub variant: ButtonVariant,
    pub handler: Box<dyn Fn(&mut App)>,
}

impl Modal {
    pub fn new(
        title: impl Into<SharedString>,
        content: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            title: title.into(),
            content: content.into_any_element(),
            actions: Vec::new(),
            width: px(480.0),
            closable: true,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    pub fn action(
        mut self,
        label: impl Into<SharedString>,
        variant: ButtonVariant,
        handler: impl Fn(&mut App) + 'static,
    ) -> Self {
        self.actions.push(ModalAction {
            label: label.into(),
            variant,
            handler: Box::new(handler),
        });
        self
    }
}

impl Render for Modal {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        // Overlay background
        div()
            .id("modal-overlay")
            .fixed()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(black().opacity(0.5))
            .z_index(1000)
            .on_click(cx.listener(|this, _, _, cx| {
                if this.closable {
                    cx.emit(ModalEvent::Close);
                }
            }))
            // Modal content
            .child(
                div()
                    .id("modal-content")
                    .track_focus(&self.focus_handle)
                    .w(self.width)
                    .max_h(vh(0.9))
                    .flex()
                    .flex_col()
                    .rounded_lg()
                    .bg(theme.colors.elevated_surface_background)
                    .border_1()
                    .border_color(theme.colors.border)
                    .shadow_xl()
                    .on_click(|event, _, _| {
                        event.stop_propagation();
                    })
                    // Header
                    .child(self.render_header(cx))
                    // Content
                    .child(
                        div()
                            .flex_1()
                            .p_4()
                            .overflow_y_auto()
                            .child(self.content.clone())
                    )
                    // Footer with actions
                    .when(!self.actions.is_empty(), |this| {
                        this.child(self.render_footer(cx))
                    })
            )
    }
}

impl Modal {
    fn render_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .w_full()
            .h_12()
            .px_4()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(theme.colors.border)
            .child(
                div()
                    .text_base()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.title.clone())
            )
            .when(self.closable, |this| {
                this.child(
                    div()
                        .size_7()
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.colors.ghost_element_hover))
                        .on_click(cx.listener(|_, _, _, cx| {
                            cx.emit(ModalEvent::Close);
                        }))
                        .child(Icon::new(IconName::Close).size(IconSize::Small))
                )
            })
    }

    fn render_footer(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .w_full()
            .px_4()
            .py_3()
            .flex()
            .flex_row()
            .items_center()
            .justify_end()
            .gap_2()
            .border_t_1()
            .border_color(theme.colors.border)
            .children(
                self.actions.iter().map(|action| {
                    Button::new()
                        .label(action.label.clone())
                        .variant(action.variant)
                })
            )
    }
}

#[derive(Debug, Clone)]
pub enum ModalEvent {
    Close,
}

impl EventEmitter<ModalEvent> for Modal {}
```

---

## 11. Context Menu

### 11.1 Context Menu Component

```rust
// crates/tusk_ui/src/context_menu.rs

use gpui::*;
use crate::{icon::{Icon, IconName, IconSize}, theme::Theme};

pub struct ContextMenu {
    items: Vec<ContextMenuItem>,
    position: Point<Pixels>,
    focus_handle: FocusHandle,
}

pub enum ContextMenuItem {
    Action {
        label: SharedString,
        icon: Option<IconName>,
        shortcut: Option<SharedString>,
        disabled: bool,
        handler: Box<dyn Fn(&mut App)>,
    },
    Separator,
    Submenu {
        label: SharedString,
        icon: Option<IconName>,
        items: Vec<ContextMenuItem>,
    },
}

impl ContextMenu {
    pub fn new(position: Point<Pixels>, cx: &mut Context<Self>) -> Self {
        Self {
            items: Vec::new(),
            position,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn items(mut self, items: Vec<ContextMenuItem>) -> Self {
        self.items = items;
        self
    }

    pub fn action(
        label: impl Into<SharedString>,
        handler: impl Fn(&mut App) + 'static,
    ) -> ContextMenuItem {
        ContextMenuItem::Action {
            label: label.into(),
            icon: None,
            shortcut: None,
            disabled: false,
            handler: Box::new(handler),
        }
    }

    pub fn separator() -> ContextMenuItem {
        ContextMenuItem::Separator
    }
}

impl Render for ContextMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id("context-menu")
            .track_focus(&self.focus_handle)
            .fixed()
            .left(self.position.x)
            .top(self.position.y)
            .min_w(px(180.0))
            .py_1()
            .rounded_md()
            .bg(theme.colors.elevated_surface_background)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .z_index(2000)
            .children(
                self.items.iter().map(|item| {
                    self.render_item(item, cx)
                })
            )
    }
}

impl ContextMenu {
    fn render_item(&self, item: &ContextMenuItem, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        match item {
            ContextMenuItem::Separator => {
                div()
                    .w_full()
                    .h(px(1.0))
                    .my_1()
                    .bg(theme.colors.border)
                    .into_any_element()
            }
            ContextMenuItem::Action { label, icon, shortcut, disabled, handler: _ } => {
                div()
                    .w_full()
                    .px_2()
                    .py_1()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .rounded_sm()
                    .cursor_pointer()
                    .when(*disabled, |this| {
                        this.opacity(0.5).cursor_not_allowed()
                    })
                    .when(!*disabled, |this| {
                        this.hover(|style| style.bg(theme.colors.list_hover_background))
                    })
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .when_some(*icon, |this, icon| {
                                this.child(
                                    Icon::new(icon)
                                        .size(IconSize::Small)
                                        .color(theme.colors.text_muted)
                                )
                            })
                            .child(
                                div().text_sm().child(label.clone())
                            )
                    )
                    .when_some(shortcut.clone(), |this, shortcut| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(theme.colors.text_muted)
                                .child(shortcut)
                        )
                    })
                    .into_any_element()
            }
            ContextMenuItem::Submenu { label, icon, items: _ } => {
                // Submenu implementation with hover-expand
                div()
                    .w_full()
                    .px_2()
                    .py_1()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.colors.list_hover_background))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .when_some(*icon, |this, icon| {
                                this.child(
                                    Icon::new(icon)
                                        .size(IconSize::Small)
                                        .color(theme.colors.text_muted)
                                )
                            })
                            .child(div().text_sm().child(label.clone()))
                    )
                    .child(
                        Icon::new(IconName::ChevronRight)
                            .size(IconSize::XSmall)
                            .color(theme.colors.text_muted)
                    )
                    .into_any_element()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ContextMenuEvent {
    Close,
}

impl EventEmitter<ContextMenuEvent> for ContextMenu {}
```

---

## 12. Keyboard Navigation

### 12.1 Key Bindings

```rust
// crates/tusk_ui/src/key_bindings.rs

use gpui::*;

pub fn register_key_bindings(cx: &mut App) {
    // Global workspace bindings
    cx.bind_keys([
        // Dock toggles
        KeyBinding::new("cmd-b", ToggleLeftDock, Some("Workspace")),
        KeyBinding::new("cmd-j", ToggleBottomDock, Some("Workspace")),

        // Tab management
        KeyBinding::new("cmd-t", NewQueryTab, Some("Workspace")),
        KeyBinding::new("cmd-w", CloseTab, Some("Workspace")),
        KeyBinding::new("cmd-shift-t", ReopenClosedTab, Some("Workspace")),

        // Tab navigation
        KeyBinding::new("cmd-1", ActivateTab1, Some("Workspace")),
        KeyBinding::new("cmd-2", ActivateTab2, Some("Workspace")),
        KeyBinding::new("cmd-3", ActivateTab3, Some("Workspace")),
        KeyBinding::new("cmd-4", ActivateTab4, Some("Workspace")),
        KeyBinding::new("cmd-5", ActivateTab5, Some("Workspace")),
        KeyBinding::new("cmd-6", ActivateTab6, Some("Workspace")),
        KeyBinding::new("cmd-7", ActivateTab7, Some("Workspace")),
        KeyBinding::new("cmd-8", ActivateTab8, Some("Workspace")),
        KeyBinding::new("cmd-9", ActivateLastTab, Some("Workspace")),
        KeyBinding::new("ctrl-tab", NextTab, Some("Workspace")),
        KeyBinding::new("ctrl-shift-tab", PreviousTab, Some("Workspace")),

        // Focus
        KeyBinding::new("cmd-shift-e", FocusSidebar, Some("Workspace")),
        KeyBinding::new("cmd-shift-r", FocusResults, Some("Workspace")),
        KeyBinding::new("escape", FocusEditor, Some("Workspace")),
    ]);

    // Editor bindings
    cx.bind_keys([
        KeyBinding::new("cmd-enter", ExecuteQuery, Some("QueryEditor")),
        KeyBinding::new("cmd-shift-enter", ExecuteSelectedQuery, Some("QueryEditor")),
        KeyBinding::new("cmd-.", CancelQuery, Some("QueryEditor")),
        KeyBinding::new("f5", RefreshSchema, Some("QueryEditor")),
    ]);

    // Results grid bindings
    cx.bind_keys([
        KeyBinding::new("cmd-c", CopySelection, Some("ResultsGrid")),
        KeyBinding::new("cmd-shift-c", CopyWithHeaders, Some("ResultsGrid")),
        KeyBinding::new("cmd-a", SelectAll, Some("ResultsGrid")),
    ]);

    // Tree navigation
    cx.bind_keys([
        KeyBinding::new("up", SelectPrevious, Some("Tree")),
        KeyBinding::new("down", SelectNext, Some("Tree")),
        KeyBinding::new("left", CollapseNode, Some("Tree")),
        KeyBinding::new("right", ExpandNode, Some("Tree")),
        KeyBinding::new("enter", ActivateNode, Some("Tree")),
    ]);
}

// Define all actions
actions!(workspace, [
    ToggleLeftDock,
    ToggleBottomDock,
    NewQueryTab,
    CloseTab,
    ReopenClosedTab,
    ActivateTab1,
    ActivateTab2,
    ActivateTab3,
    ActivateTab4,
    ActivateTab5,
    ActivateTab6,
    ActivateTab7,
    ActivateTab8,
    ActivateLastTab,
    NextTab,
    PreviousTab,
    FocusSidebar,
    FocusResults,
    FocusEditor,
]);

actions!(editor, [
    ExecuteQuery,
    ExecuteSelectedQuery,
    CancelQuery,
    RefreshSchema,
]);

actions!(grid, [
    CopySelection,
    CopyWithHeaders,
    SelectAll,
]);

actions!(tree, [
    SelectPrevious,
    SelectNext,
    CollapseNode,
    ExpandNode,
    ActivateNode,
]);
```

---

## 13. Layout Utilities

### 13.1 Common Layout Patterns

```rust
// crates/tusk_ui/src/layout.rs

use gpui::*;
use crate::{icon::{Icon, IconName, IconSize}, theme::Theme};

/// Horizontal stack with gap
pub fn h_stack() -> Div {
    div().flex().flex_row().items_center()
}

/// Vertical stack with gap
pub fn v_stack() -> Div {
    div().flex().flex_col()
}

/// Centered content
pub fn centered() -> Div {
    div().flex().items_center().justify_center()
}

/// Card container
pub fn card(cx: &App) -> Div {
    let theme = cx.global::<Theme>();

    div()
        .rounded_lg()
        .bg(theme.colors.elevated_surface_background)
        .border_1()
        .border_color(theme.colors.border)
        .shadow_sm()
}

/// Section with header
pub fn section(title: impl Into<SharedString>, cx: &App) -> Div {
    let theme = cx.global::<Theme>();

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.colors.text_muted)
                .child(title.into())
        )
}

/// Divider line
pub fn divider(cx: &App) -> Div {
    let theme = cx.global::<Theme>();

    div()
        .w_full()
        .h(px(1.0))
        .bg(theme.colors.border)
}

/// Empty state placeholder
pub fn empty_state(
    icon: IconName,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    cx: &App,
) -> Div {
    let theme = cx.global::<Theme>();

    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .child(
            Icon::new(icon)
                .size(IconSize::XLarge)
                .color(theme.colors.text_muted)
        )
        .child(
            div()
                .text_base()
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.colors.text)
                .child(title.into())
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.colors.text_muted)
                .child(description.into())
        )
}
```

---

## 14. Spinner Component

### 14.1 Loading Spinner

```rust
// crates/tusk_ui/src/spinner.rs

use gpui::*;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinnerSize {
    Small,   // 14px
    #[default]
    Medium,  // 20px
    Large,   // 32px
}

impl SpinnerSize {
    fn pixels(self) -> Pixels {
        match self {
            SpinnerSize::Small => px(14.0),
            SpinnerSize::Medium => px(20.0),
            SpinnerSize::Large => px(32.0),
        }
    }
}

pub struct Spinner {
    size: SpinnerSize,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            size: SpinnerSize::Medium,
        }
    }

    pub fn size(mut self, size: SpinnerSize) -> Self {
        self.size = size;
        self
    }
}

impl RenderOnce for Spinner {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let size = self.size.pixels();

        div()
            .size(size)
            .rounded_full()
            .border_2()
            .border_color(theme.colors.border)
            .border_t_color(theme.colors.accent)
            .with_animation(
                "spin",
                Animation::new(Duration::from_secs(1))
                    .repeat()
                    .with_easing(Easing::Linear),
                move |this, progress| {
                    this.rotate(Angle::from_radians(progress * std::f32::consts::TAU))
                },
            )
    }
}
```

---

## 15. Acceptance Criteria

### 15.1 Workspace Shell

- [ ] Workspace renders with left dock, center area, bottom dock, and status bar
- [ ] Docks can be toggled via keyboard shortcuts (Cmd+B, Cmd+J)
- [ ] Docks resize via drag handle
- [ ] Minimum and maximum dock sizes enforced

### 15.2 Tab System

- [ ] New tabs can be created (Cmd+T)
- [ ] Tabs can be closed (Cmd+W)
- [ ] Tab navigation via Cmd+1-9
- [ ] Active tab visually distinguished
- [ ] Dirty indicator shows unsaved changes

### 15.3 Pane Splitting

- [ ] Panes can be split horizontally and vertically
- [ ] Pane ratios adjustable via drag
- [ ] Focus moves between panes with keyboard

### 15.4 Panel System

- [ ] Panels register with docks
- [ ] Panel tabs show in dock header
- [ ] Active panel content renders in dock

### 15.5 Components

- [ ] Button renders all variants (primary, secondary, ghost, danger)
- [ ] TextInput handles text entry, placeholder, disabled state
- [ ] Select dropdown opens/closes, allows selection
- [ ] Modal displays with overlay, header, content, actions
- [ ] Context menu positions correctly, handles item selection

### 15.6 Keyboard Navigation

- [ ] All key bindings registered and functional
- [ ] Focus moves correctly between workspace regions
- [ ] Tab navigation works within forms

### 15.7 Tree Component

- [ ] Tree renders items with proper indentation
- [ ] Expand/collapse works on expandable items
- [ ] Selection highlighted
- [ ] Keyboard navigation (up/down/left/right/enter)

---

## 16. Dependencies

```toml
# crates/tusk_ui/Cargo.toml
[dependencies]
gpui = { git = "https://github.com/zed-industries/zed" }
smallvec = { version = "1.11", features = ["union"] }
uuid = { version = "1.6", features = ["v4"] }
parking_lot = "0.12"

tusk_core = { path = "../tusk_core" }
tusk_editor = { path = "../tusk_editor" }
```

---

## 17. File Structure

```
crates/tusk_ui/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── workspace.rs
    ├── dock.rs
    ├── pane.rs
    ├── resizer.rs
    ├── status_bar.rs
    ├── panel.rs
    ├── tree.rs
    ├── icon.rs
    ├── button.rs
    ├── input.rs
    ├── select.rs
    ├── modal.rs
    ├── context_menu.rs
    ├── spinner.rs
    ├── key_bindings.rs
    ├── layout.rs
    ├── theme.rs
    └── panels/
        ├── mod.rs
        ├── schema_browser.rs
        ├── results.rs
        └── messages.rs
```

---

## 18. Testing Strategy

### 18.1 Component Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn test_workspace_creation(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let state = cx.new(|cx| TuskState::new(cx));
            let workspace = cx.new(|cx| Workspace::new(state, cx));

            // Verify initial state
            let workspace = workspace.read(cx);
            assert!(workspace.left_dock.read(cx).is_visible());
            assert!(workspace.bottom_dock.read(cx).is_visible());
        });
    }

    #[gpui::test]
    fn test_tab_management(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let pane = cx.new(|cx| Pane::new(cx));

            // Initial tab
            assert_eq!(pane.read(cx).tabs.len(), 1);

            // Add tab
            pane.update(cx, |pane, cx| pane.add_query_tab(None, cx));
            assert_eq!(pane.read(cx).tabs.len(), 2);

            // Close tab
            pane.update(cx, |pane, cx| pane.close_active_tab(cx));
            assert_eq!(pane.read(cx).tabs.len(), 1);
        });
    }

    #[gpui::test]
    fn test_dock_toggle(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let dock = cx.new(|cx| Dock::new(DockPosition::Left, px(280.0), cx));

            assert!(dock.read(cx).is_visible());

            dock.update(cx, |dock, cx| dock.toggle_visibility(cx));
            assert!(!dock.read(cx).is_visible());

            dock.update(cx, |dock, cx| dock.toggle_visibility(cx));
            assert!(dock.read(cx).is_visible());
        });
    }

    #[gpui::test]
    fn test_tree_expand_collapse(cx: &mut TestAppContext) {
        #[derive(Clone)]
        struct TestItem {
            id: usize,
            label: String,
            children: Vec<TestItem>,
        }

        impl TreeItem for TestItem {
            type Id = usize;
            fn id(&self) -> usize { self.id }
            fn label(&self) -> SharedString { self.label.clone().into() }
            fn icon(&self) -> Option<IconName> { None }
            fn children(&self) -> Option<&[Self]> { Some(&self.children) }
        }

        cx.update(|cx| {
            let tree = cx.new(|cx| Tree::<TestItem>::new(cx));

            let items = vec![TestItem {
                id: 1,
                label: "Root".to_string(),
                children: vec![TestItem {
                    id: 2,
                    label: "Child".to_string(),
                    children: vec![],
                }],
            }];

            tree.update(cx, |tree, cx| {
                tree.set_items(items, cx);
            });

            // Initially not expanded
            assert!(!tree.read(cx).expanded.contains(&1));

            // Expand
            tree.update(cx, |tree, cx| tree.toggle_expanded(&1, cx));
            assert!(tree.read(cx).expanded.contains(&1));

            // Collapse
            tree.update(cx, |tree, cx| tree.toggle_expanded(&1, cx));
            assert!(!tree.read(cx).expanded.contains(&1));
        });
    }
}
```

### 18.2 Integration Tests

Integration tests use the Tauri MCP server to test the full UI:

```rust
// tests/ui_integration.rs

// Note: These tests require a running Tusk application instance
// and use the Tauri MCP server for automation

#[test]
fn test_workspace_keyboard_navigation() {
    // Use Tauri MCP to:
    // 1. Start app
    // 2. Press Cmd+B to toggle left dock
    // 3. Verify dock visibility changed via DOM snapshot
    // 4. Press Cmd+T to create new tab
    // 5. Verify tab count increased
}

#[test]
fn test_tab_creation_and_close() {
    // 1. Connect to running app via driver_session
    // 2. Get initial DOM snapshot
    // 3. Click new tab button
    // 4. Verify new tab appears
    // 5. Click close button on tab
    // 6. Verify tab is removed
}
```

---

## 19. Migration Notes

This document replaces the previous Svelte-based frontend architecture. Key changes:

| Previous (Svelte)       | New (GPUI)                       |
| ----------------------- | -------------------------------- |
| Svelte components       | Rust structs implementing Render |
| Svelte stores           | Entity<T> state management       |
| CSS/Tailwind            | Styled trait fluent API          |
| DOM events              | GPUI event system                |
| JavaScript runtime      | Pure Rust compilation            |
| Virtual DOM             | Immediate mode rendering         |
| `svelte-dnd-action`     | Custom drag handlers             |
| `localStorage`          | rusqlite persistence             |

The component patterns and layout remain similar, but implementation is entirely in Rust using GPUI's element system.
