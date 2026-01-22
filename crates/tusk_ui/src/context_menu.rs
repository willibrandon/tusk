//! Context menu component for right-click menus with submenus.
//!
//! This module provides:
//! - ContextMenu component with keyboard navigation
//! - ContextMenuItem enum (Action, Separator, Submenu)
//! - ContextMenuLayer global for showing/dismissing menus
//! - Viewport overflow handling for menu positioning
//! - Click-outside-to-close behavior

use std::sync::Arc;

use gpui::{
    div, prelude::*, px, App, Context, Entity, EventEmitter, FocusHandle, Global, MouseButton,
    ParentElement, Point, Render, SharedString, Styled, Subscription, Window,
};

use crate::panel::Focusable;

use crate::icon::{Icon, IconName, IconSize};
use crate::key_bindings::context_menu::{
    CloseSubmenu, ConfirmItem, DismissMenu, OpenSubmenu, SelectNextItem, SelectPreviousItem,
};
use crate::TuskTheme;

// ============================================================================
// ContextMenuEvent
// ============================================================================

/// Events emitted by the ContextMenu component.
#[derive(Debug, Clone)]
pub enum ContextMenuEvent {
    /// The menu should be closed.
    Close,
    /// An item was activated.
    ItemActivated { id: SharedString },
}

// ============================================================================
// ContextMenuItem
// ============================================================================

/// Handler type for context menu actions.
pub type ContextMenuHandler = Arc<dyn Fn(&mut App) + Send + Sync + 'static>;

/// Represents an item in a context menu.
#[derive(Clone)]
pub enum ContextMenuItem {
    /// A clickable action item.
    Action {
        id: SharedString,
        label: SharedString,
        icon: Option<IconName>,
        shortcut: Option<SharedString>,
        disabled: bool,
        handler: Option<ContextMenuHandler>,
    },
    /// A visual separator line.
    Separator,
    /// A nested submenu.
    Submenu {
        label: SharedString,
        icon: Option<IconName>,
        items: Vec<ContextMenuItem>,
    },
}

impl ContextMenuItem {
    /// Create an action item with a handler.
    pub fn action(
        label: impl Into<SharedString>,
        handler: impl Fn(&mut App) + Send + Sync + 'static,
    ) -> Self {
        let label: SharedString = label.into();
        Self::Action {
            id: label.clone(),
            label,
            icon: None,
            shortcut: None,
            disabled: false,
            handler: Some(Arc::new(handler)),
        }
    }

    /// Create an action item with full options.
    pub fn action_full(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: Option<IconName>,
        shortcut: Option<impl Into<SharedString>>,
        disabled: bool,
        handler: impl Fn(&mut App) + Send + Sync + 'static,
    ) -> Self {
        Self::Action {
            id: id.into(),
            label: label.into(),
            icon,
            shortcut: shortcut.map(|s| s.into()),
            disabled,
            handler: Some(Arc::new(handler)),
        }
    }

    /// Create a separator item.
    pub fn separator() -> Self {
        Self::Separator
    }

    /// Create a submenu item.
    pub fn submenu(label: impl Into<SharedString>, items: Vec<ContextMenuItem>) -> Self {
        Self::Submenu {
            label: label.into(),
            icon: None,
            items,
        }
    }

    /// Builder: add an icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        match &mut self {
            Self::Action {
                icon: ref mut i, ..
            } => *i = Some(icon),
            Self::Submenu {
                icon: ref mut i, ..
            } => *i = Some(icon),
            Self::Separator => {}
        }
        self
    }

    /// Builder: add a shortcut hint.
    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        if let Self::Action {
            shortcut: ref mut s,
            ..
        } = &mut self
        {
            *s = Some(shortcut.into());
        }
        self
    }

    /// Builder: set disabled state.
    pub fn disabled(mut self, disabled: bool) -> Self {
        if let Self::Action {
            disabled: ref mut d,
            ..
        } = &mut self
        {
            *d = disabled;
        }
        self
    }

    /// Check if this item is a separator.
    pub fn is_separator(&self) -> bool {
        matches!(self, Self::Separator)
    }

    /// Check if this item is a submenu.
    pub fn is_submenu(&self) -> bool {
        matches!(self, Self::Submenu { .. })
    }

    /// Check if this item is selectable (not a separator, not disabled).
    pub fn is_selectable(&self) -> bool {
        match self {
            Self::Action { disabled, .. } => !disabled,
            Self::Submenu { .. } => true,
            Self::Separator => false,
        }
    }

    /// Get the label of this item.
    pub fn label(&self) -> Option<&SharedString> {
        match self {
            Self::Action { label, .. } => Some(label),
            Self::Submenu { label, .. } => Some(label),
            Self::Separator => None,
        }
    }

    /// Get the ID of an action item.
    pub fn id(&self) -> Option<&SharedString> {
        match self {
            Self::Action { id, .. } => Some(id),
            _ => None,
        }
    }

    /// Get submenu items if this is a submenu.
    pub fn submenu_items(&self) -> Option<&[ContextMenuItem]> {
        match self {
            Self::Submenu { items, .. } => Some(items),
            _ => None,
        }
    }
}

// ============================================================================
// ContextMenu
// ============================================================================

/// A context menu component with keyboard navigation.
pub struct ContextMenu {
    /// Menu items.
    items: Vec<ContextMenuItem>,
    /// Position where the menu should appear.
    position: Point<gpui::Pixels>,
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// Currently highlighted item index.
    highlighted_index: Option<usize>,
    /// Active submenu entity and its index.
    active_submenu: Option<(usize, Entity<ContextMenu>)>,
    /// Subscription to submenu events.
    _submenu_subscription: Option<Subscription>,
    /// Whether this is a root menu or a submenu.
    is_submenu: bool,
}

impl ContextMenu {
    /// Create a new context menu at the given position.
    pub fn new(position: Point<gpui::Pixels>, cx: &mut Context<Self>) -> Self {
        Self {
            items: Vec::new(),
            position,
            focus_handle: cx.focus_handle(),
            highlighted_index: None,
            active_submenu: None,
            _submenu_subscription: None,
            is_submenu: false,
        }
    }

    /// Create a new submenu at the given position.
    pub fn new_submenu(
        position: Point<gpui::Pixels>,
        items: Vec<ContextMenuItem>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            items,
            position,
            focus_handle: cx.focus_handle(),
            highlighted_index: None,
            active_submenu: None,
            _submenu_subscription: None,
            is_submenu: true,
        }
    }

    /// Add a menu item.
    pub fn item(mut self, item: ContextMenuItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add multiple items.
    pub fn items(mut self, items: impl IntoIterator<Item = ContextMenuItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Get the menu position.
    pub fn position(&self) -> Point<gpui::Pixels> {
        self.position
    }

    /// Get the menu items.
    pub fn menu_items(&self) -> &[ContextMenuItem] {
        &self.items
    }

    /// Dismiss the menu.
    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        cx.emit(ContextMenuEvent::Close);
    }

    /// Select the next item (skip separators).
    fn select_next(&mut self, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }

        let start = self.highlighted_index.map(|i| i + 1).unwrap_or(0);
        let len = self.items.len();

        for offset in 0..len {
            let idx = (start + offset) % len;
            if self.items[idx].is_selectable() {
                self.highlighted_index = Some(idx);
                self.close_active_submenu(cx);
                cx.notify();
                return;
            }
        }
    }

    /// Select the previous item (skip separators).
    fn select_previous(&mut self, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }

        let len = self.items.len();
        let start = self
            .highlighted_index
            .map(|i| if i == 0 { len - 1 } else { i - 1 })
            .unwrap_or(len - 1);

        for offset in 0..len {
            let idx = (start + len - offset) % len;
            if self.items[idx].is_selectable() {
                self.highlighted_index = Some(idx);
                self.close_active_submenu(cx);
                cx.notify();
                return;
            }
        }
    }

    /// Confirm the highlighted item.
    fn confirm_highlighted(&mut self, cx: &mut Context<Self>) {
        let Some(idx) = self.highlighted_index else {
            return;
        };

        let Some(item) = self.items.get(idx).cloned() else {
            return;
        };

        match item {
            ContextMenuItem::Action {
                id,
                disabled,
                handler,
                ..
            } => {
                if !disabled {
                    // Emit item activated event
                    cx.emit(ContextMenuEvent::ItemActivated { id: id.clone() });
                    // Run the handler
                    if let Some(handler) = handler {
                        // We need to run this after closing the menu
                        cx.spawn(async move |_this, cx| {
                            cx.update(|cx| {
                                handler(cx);
                            });
                        })
                        .detach();
                    }
                    // Close the menu
                    cx.emit(ContextMenuEvent::Close);
                }
            }
            ContextMenuItem::Submenu { items, .. } => {
                // Open the submenu
                self.open_submenu(idx, items, cx);
            }
            ContextMenuItem::Separator => {}
        }
    }

    /// Open a submenu for the item at the given index.
    fn open_submenu(&mut self, idx: usize, items: Vec<ContextMenuItem>, cx: &mut Context<Self>) {
        // Close any existing submenu
        self.close_active_submenu(cx);

        // Calculate submenu position (to the right of the current menu)
        // This is a simplified position; real positioning happens in render
        let submenu_position = Point {
            x: self.position.x + px(200.0), // Menu width
            y: self.position.y + px(idx as f32 * 28.0), // Item height
        };

        let submenu = cx.new(|cx| {
            let mut menu = ContextMenu::new_submenu(submenu_position, items, cx);
            // Auto-select first selectable item
            for (i, item) in menu.items.iter().enumerate() {
                if item.is_selectable() {
                    menu.highlighted_index = Some(i);
                    break;
                }
            }
            menu
        });

        // Subscribe to submenu events
        let subscription = cx.subscribe(&submenu, Self::handle_submenu_event);

        self.active_submenu = Some((idx, submenu));
        self._submenu_subscription = Some(subscription);
        cx.notify();
    }

    /// Handle events from a submenu.
    fn handle_submenu_event(
        &mut self,
        _submenu: Entity<ContextMenu>,
        event: &ContextMenuEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            ContextMenuEvent::Close => {
                // Propagate close to parent
                cx.emit(ContextMenuEvent::Close);
            }
            ContextMenuEvent::ItemActivated { id } => {
                // Propagate item activation
                cx.emit(ContextMenuEvent::ItemActivated { id: id.clone() });
            }
        }
    }

    /// Close the active submenu.
    fn close_active_submenu(&mut self, cx: &mut Context<Self>) {
        if self.active_submenu.take().is_some() {
            self._submenu_subscription = None;
            cx.notify();
        }
    }

    /// Open submenu for currently highlighted item (if it's a submenu).
    fn open_highlighted_submenu(&mut self, cx: &mut Context<Self>) {
        let Some(idx) = self.highlighted_index else {
            return;
        };

        if let Some(ContextMenuItem::Submenu { items, .. }) = self.items.get(idx).cloned() {
            self.open_submenu(idx, items, cx);
        }
    }

    /// Close submenu and return focus to parent.
    fn close_submenu_action(&mut self, cx: &mut Context<Self>) {
        if self.is_submenu {
            // This is a submenu, close it
            cx.emit(ContextMenuEvent::Close);
        } else if self.active_submenu.is_some() {
            // Close the active submenu
            self.close_active_submenu(cx);
        }
    }

    /// Render a single menu item.
    fn render_item(
        &self,
        idx: usize,
        item: &ContextMenuItem,
        is_highlighted: bool,
        theme: &TuskTheme,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        match item {
            ContextMenuItem::Separator => div()
                .h(px(1.0))
                .w_full()
                .my(px(4.0))
                .bg(theme.colors.border)
                .into_any_element(),

            ContextMenuItem::Action {
                label,
                icon,
                shortcut,
                disabled,
                ..
            } => {
                let text_color = if *disabled {
                    theme.colors.text_muted
                } else {
                    theme.colors.text
                };

                let is_disabled = *disabled;
                let shortcut_clone = shortcut.clone();

                div()
                    .id(format!("menu-item-{}", idx))
                    .h(px(28.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .px(px(8.0))
                    .gap(px(8.0))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .when(is_highlighted && !is_disabled, |d| {
                        d.bg(theme.colors.list_active_selection_background)
                    })
                    .when(!is_highlighted && !is_disabled, |d| {
                        d.hover(|s| s.bg(theme.colors.list_hover_background))
                    })
                    // Use on_mouse_move to track hovering (triggers when mouse enters)
                    .on_mouse_move(cx.listener(move |this, _, _window, cx| {
                        if this.highlighted_index != Some(idx) {
                            this.highlighted_index = Some(idx);
                            this.close_active_submenu(cx);
                            cx.notify();
                        }
                    }))
                    .when(!is_disabled, |d: gpui::Stateful<gpui::Div>| {
                        d.on_click(cx.listener(move |this, _, _window, cx| {
                            this.highlighted_index = Some(idx);
                            this.confirm_highlighted(cx);
                        }))
                    })
                    // Icon
                    .child(
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when_some(icon.as_ref(), |d, icon| {
                                d.child(Icon::new(*icon).size(IconSize::Small).color(text_color))
                            }),
                    )
                    // Label
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(text_color)
                            .child(label.clone()),
                    )
                    // Shortcut
                    .when_some(
                        shortcut_clone.as_ref(),
                        |d: gpui::Stateful<gpui::Div>, shortcut: &SharedString| {
                            d.child(
                                div()
                                    .text_xs()
                                    .text_color(theme.colors.text_muted)
                                    .child(shortcut.clone()),
                            )
                        },
                    )
                    .into_any_element()
            }

            ContextMenuItem::Submenu { label, icon, .. } => {
                let is_submenu_open = self
                    .active_submenu
                    .as_ref()
                    .map(|(i, _)| *i == idx)
                    .unwrap_or(false);

                div()
                    .id(format!("submenu-item-{}", idx))
                    .h(px(28.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .px(px(8.0))
                    .gap(px(8.0))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .when(is_highlighted || is_submenu_open, |d| {
                        d.bg(theme.colors.list_active_selection_background)
                    })
                    .when(!is_highlighted && !is_submenu_open, |d| {
                        d.hover(|s| s.bg(theme.colors.list_hover_background))
                    })
                    .on_mouse_move(cx.listener(move |this, _, _window, cx| {
                        if this.highlighted_index != Some(idx) {
                            this.highlighted_index = Some(idx);
                            // Open submenu on hover
                            if let Some(ContextMenuItem::Submenu { items, .. }) =
                                this.items.get(idx).cloned()
                            {
                                this.open_submenu(idx, items, cx);
                            }
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.highlighted_index = Some(idx);
                        this.open_highlighted_submenu(cx);
                    }))
                    // Icon
                    .child(
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when_some(icon.as_ref(), |d, icon| {
                                d.child(
                                    Icon::new(*icon)
                                        .size(IconSize::Small)
                                        .color(theme.colors.text),
                                )
                            }),
                    )
                    // Label
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(theme.colors.text)
                            .child(label.clone()),
                    )
                    // Submenu arrow
                    .child(
                        Icon::new(IconName::ChevronRight)
                            .size(IconSize::Small)
                            .color(theme.colors.text_muted),
                    )
                    .into_any_element()
            }
        }
    }
}

impl EventEmitter<ContextMenuEvent> for ContextMenu {}

impl Focusable for ContextMenu {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ContextMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>().clone();
        let highlighted_index = self.highlighted_index;

        // Menu dimensions
        let menu_width = px(200.0);
        let approx_height = px(
            self.items
                .iter()
                .map(|item| if item.is_separator() { 9.0 } else { 28.0 })
                .sum::<f32>()
                + 8.0,
        ); // padding

        // Calculate position with viewport overflow handling (T099)
        let viewport_size = window.viewport_size();
        let mut pos = self.position;

        // Adjust for right overflow
        if pos.x + menu_width > viewport_size.width {
            pos.x = self.position.x - menu_width;
            if pos.x < px(0.0) {
                pos.x = px(0.0);
            }
        }

        // Adjust for bottom overflow
        if pos.y + approx_height > viewport_size.height {
            pos.y = self.position.y - approx_height;
            if pos.y < px(0.0) {
                pos.y = px(0.0);
            }
        }

        // Clone items for rendering
        let items_clone = self.items.clone();

        div()
            .id("context-menu")
            .key_context("ContextMenu")
            .track_focus(&self.focus_handle)
            .absolute()
            .left(pos.x)
            .top(pos.y)
            .w(menu_width)
            .py(px(4.0))
            .bg(theme.colors.elevated_surface_background)
            .border_1()
            .border_color(theme.colors.border)
            .rounded(px(6.0))
            .shadow_lg()
            // Keyboard navigation
            .on_action(cx.listener(|this, _: &SelectNextItem, _window, cx| {
                this.select_next(cx);
            }))
            .on_action(cx.listener(|this, _: &SelectPreviousItem, _window, cx| {
                this.select_previous(cx);
            }))
            .on_action(cx.listener(|this, _: &ConfirmItem, _window, cx| {
                this.confirm_highlighted(cx);
            }))
            .on_action(cx.listener(|this, _: &DismissMenu, _window, cx| {
                this.dismiss(cx);
            }))
            .on_action(cx.listener(|this, _: &OpenSubmenu, _window, cx| {
                this.open_highlighted_submenu(cx);
            }))
            .on_action(cx.listener(|this, _: &CloseSubmenu, _window, cx| {
                this.close_submenu_action(cx);
            }))
            // Stop propagation on mouse events
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_mouse_down(MouseButton::Right, |_, _, cx| {
                cx.stop_propagation();
            })
            // Menu items
            .children(items_clone.iter().enumerate().map(|(idx, item)| {
                let is_highlighted = highlighted_index == Some(idx);
                self.render_item(idx, item, is_highlighted, &theme, cx)
            }))
            // Active submenu
            .when_some(self.active_submenu.as_ref(), |d, (_, submenu)| {
                d.child(submenu.clone())
            })
    }
}

// ============================================================================
// ContextMenuLayer - Global Context Menu Management
// ============================================================================

/// Entry in the context menu stack.
struct MenuEntry {
    /// The context menu entity.
    menu: Entity<ContextMenu>,
    /// Subscription to menu events.
    _subscription: Subscription,
}

/// Global layer for managing context menu display.
///
/// The ContextMenuLayer maintains a single active context menu (with potential submenus)
/// and ensures it is rendered above all other content.
pub struct ContextMenuLayer {
    /// The active context menu.
    active_menu: Option<MenuEntry>,
}

impl Global for ContextMenuLayer {}

impl ContextMenuLayer {
    /// Create a new empty context menu layer.
    pub fn new() -> Self {
        Self { active_menu: None }
    }

    /// Show a context menu.
    ///
    /// Any existing menu will be dismissed first.
    pub fn show(&mut self, menu: Entity<ContextMenu>, window: &mut Window, cx: &mut App) {
        // Dismiss any existing menu
        self.dismiss(cx);

        // Focus the menu and auto-select first selectable item
        menu.update(cx, |menu_inner, _cx| {
            for (i, item) in menu_inner.items.iter().enumerate() {
                if item.is_selectable() {
                    menu_inner.highlighted_index = Some(i);
                    break;
                }
            }
        });

        // Focus the menu
        let focus_handle = menu.read(cx).focus_handle.clone();
        window.focus(&focus_handle, cx);

        // Subscribe to menu events
        let subscription =
            cx.subscribe(&menu, |_menu: Entity<ContextMenu>, event: &ContextMenuEvent, cx| {
                if matches!(event, ContextMenuEvent::Close) {
                    cx.update_global::<ContextMenuLayer, _>(|layer: &mut ContextMenuLayer, cx| {
                        layer.dismiss(cx);
                    });
                }
            });

        self.active_menu = Some(MenuEntry {
            menu,
            _subscription: subscription,
        });

        cx.refresh_windows();
    }

    /// Show a context menu without requiring a window.
    ///
    /// This is useful when showing a menu from an event handler that doesn't have window access.
    /// The menu will be focused when the user first interacts with it.
    pub fn show_deferred(&mut self, menu: Entity<ContextMenu>, cx: &mut App) {
        // Dismiss any existing menu
        self.dismiss(cx);

        // Auto-select first selectable item
        menu.update(cx, |menu_inner, _cx| {
            for (i, item) in menu_inner.items.iter().enumerate() {
                if item.is_selectable() {
                    menu_inner.highlighted_index = Some(i);
                    break;
                }
            }
        });

        // Subscribe to menu events
        let subscription =
            cx.subscribe(&menu, |_menu: Entity<ContextMenu>, event: &ContextMenuEvent, cx| {
                if matches!(event, ContextMenuEvent::Close) {
                    cx.update_global::<ContextMenuLayer, _>(|layer: &mut ContextMenuLayer, cx| {
                        layer.dismiss(cx);
                    });
                }
            });

        self.active_menu = Some(MenuEntry {
            menu,
            _subscription: subscription,
        });

        cx.refresh_windows();
    }

    /// Dismiss the active context menu.
    pub fn dismiss(&mut self, cx: &mut App) {
        if self.active_menu.take().is_some() {
            cx.refresh_windows();
        }
    }

    /// Check if any context menu is currently open.
    pub fn is_open(&self) -> bool {
        self.active_menu.is_some()
    }

    /// Render the context menu layer.
    ///
    /// Returns a backdrop with the menu if a menu is open.
    /// The backdrop captures clicks outside the menu to dismiss it (T102).
    pub fn render(&self) -> Option<gpui::AnyElement> {
        self.active_menu.as_ref().map(|entry| {
            let menu = entry.menu.clone();

            // Create a full-screen invisible backdrop that captures clicks
            // When clicked, it dismisses the context menu
            div()
                .id("context-menu-backdrop")
                .absolute()
                .inset_0()
                .size_full()
                // Capture any click on the backdrop to close the menu
                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                    cx.update_global::<ContextMenuLayer, _>(|layer: &mut ContextMenuLayer, cx| {
                        layer.dismiss(cx);
                    });
                })
                .on_mouse_down(MouseButton::Right, |_, _, cx| {
                    // Also close on right-click outside
                    cx.update_global::<ContextMenuLayer, _>(|layer: &mut ContextMenuLayer, cx| {
                        layer.dismiss(cx);
                    });
                })
                // Render the menu on top of the backdrop
                .child(menu)
                .into_any_element()
        })
    }
}

impl Default for ContextMenuLayer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_menu_item_builders() {
        // Test separator
        let sep = ContextMenuItem::separator();
        assert!(sep.is_separator());
        assert!(!sep.is_selectable());

        // Test action
        let action = ContextMenuItem::action("Copy", |_| {})
            .icon(IconName::Copy)
            .shortcut("Cmd+C");
        assert!(!action.is_separator());
        assert!(action.is_selectable());
        assert_eq!(action.label().map(|l| l.as_ref()), Some("Copy"));

        // Test disabled action
        let disabled = ContextMenuItem::action("Delete", |_| {}).disabled(true);
        assert!(!disabled.is_selectable());

        // Test submenu
        let submenu =
            ContextMenuItem::submenu("Export", vec![ContextMenuItem::action("As CSV", |_| {})]);
        assert!(submenu.is_submenu());
        assert!(submenu.is_selectable());
        assert_eq!(submenu.submenu_items().map(|i| i.len()), Some(1));
    }

    #[test]
    fn test_context_menu_layer() {
        let layer = ContextMenuLayer::new();
        assert!(!layer.is_open());
    }
}
