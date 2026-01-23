//! Application menu bar for non-macOS platforms.
//!
//! On macOS, menus are handled natively by the OS menu bar via `cx.set_menus()`.
//! On Windows and Linux, this component renders an in-window menu bar similar
//! to how Zed handles cross-platform menus.
//!
//! This module provides:
//! - ApplicationMenu component that renders menu bar entries
//! - Integration with GPUI's menu system via `cx.get_menus()`
//! - Popover-based dropdown menus for each menu entry

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{
    actions, div, prelude::*, px, Action, App, Context, Corner, Entity, FocusHandle, OwnedMenu,
    OwnedMenuItem, Point, Render, SharedString, Window,
};
use parking_lot::RwLock;
use smallvec::SmallVec;

use crate::button::{Button, ButtonStyle};
use crate::context_menu::{ContextMenu, ContextMenuItem};
use crate::popover_menu::{PopoverMenu, PopoverMenuHandle};
use crate::TuskTheme;

/// Stored action for dispatch.
struct StoredAction {
    action: Box<dyn Action>,
}

// SAFETY: We only access StoredAction from the main thread via App context
unsafe impl Send for StoredAction {}
unsafe impl Sync for StoredAction {}

/// Global storage for menu actions that can be dispatched by ID.
#[derive(Default)]
pub struct MenuActionRegistry {
    actions: RwLock<HashMap<SharedString, StoredAction>>,
}

impl MenuActionRegistry {
    /// Register an action with the given ID.
    pub fn register(&self, id: SharedString, action: Box<dyn Action>) {
        self.actions.write().insert(id, StoredAction { action });
    }

    /// Dispatch an action by ID.
    pub fn dispatch(&self, id: &SharedString, cx: &mut App) {
        if let Some(stored) = self.actions.read().get(id) {
            cx.dispatch_action(stored.action.as_ref());
        }
    }

    /// Clear all registered actions.
    pub fn clear(&self) {
        self.actions.write().clear();
    }
}

// ============================================================================
// Actions
// ============================================================================

actions!(
    app_menu,
    [
        /// Activates the menu on the right in the application menu.
        ActivateMenuRight,
        /// Activates the menu on the left in the application menu.
        ActivateMenuLeft
    ]
);

// ============================================================================
// Menu Entry
// ============================================================================

#[derive(Clone)]
struct MenuEntry {
    menu: OwnedMenu,
    handle: PopoverMenuHandle,
}

// ============================================================================
// ApplicationMenu
// ============================================================================

/// Application menu bar component for non-macOS platforms.
///
/// This component renders menu entries horizontally. When a menu entry is
/// clicked, a dropdown menu appears with the menu items.
pub struct ApplicationMenu {
    entries: SmallVec<[MenuEntry; 8]>,
    focus_handle: FocusHandle,
    /// Registry for menu actions that can be dispatched by ID.
    action_registry: Arc<MenuActionRegistry>,
}

impl ApplicationMenu {
    /// Create a new application menu from registered menus.
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let menus = cx.get_menus().unwrap_or_default();
        let action_registry = Arc::new(MenuActionRegistry::default());

        // Register all actions from menus
        for menu in &menus {
            Self::register_actions_from_menu(&action_registry, menu);
        }

        Self {
            entries: menus
                .into_iter()
                .map(|menu| MenuEntry { menu, handle: PopoverMenuHandle::default() })
                .collect(),
            focus_handle: cx.focus_handle(),
            action_registry,
        }
    }

    /// Register actions from a menu recursively.
    fn register_actions_from_menu(registry: &MenuActionRegistry, menu: &OwnedMenu) {
        for item in &menu.items {
            Self::register_actions_from_item(registry, item);
        }
    }

    /// Register actions from a menu item recursively.
    fn register_actions_from_item(registry: &MenuActionRegistry, item: &OwnedMenuItem) {
        match item {
            OwnedMenuItem::Action { name, action, .. } => {
                registry.register(SharedString::from(name.clone()), action.boxed_clone());
            }
            OwnedMenuItem::Submenu(submenu) => {
                for sub_item in &submenu.items {
                    Self::register_actions_from_item(registry, sub_item);
                }
            }
            _ => {}
        }
    }

    /// Refresh menus from the registered menu definitions.
    pub fn refresh_menus(&mut self, cx: &mut Context<Self>) {
        let menus = cx.get_menus().unwrap_or_default();

        // Clear and re-register actions
        self.action_registry.clear();
        for menu in &menus {
            Self::register_actions_from_menu(&self.action_registry, menu);
        }

        self.entries = menus
            .into_iter()
            .map(|menu| MenuEntry { menu, handle: PopoverMenuHandle::default() })
            .collect();
        cx.notify();
    }

    /// Sanitize menu items by removing empty submenus and consecutive separators.
    fn sanitize_menu_items(items: Vec<OwnedMenuItem>) -> Vec<OwnedMenuItem> {
        let mut cleaned = Vec::new();
        let mut last_was_separator = true; // Start true to skip leading separators

        for item in items {
            match item {
                OwnedMenuItem::Separator => {
                    if !last_was_separator {
                        cleaned.push(item);
                        last_was_separator = true;
                    }
                }
                OwnedMenuItem::Submenu(submenu) => {
                    // Skip empty submenus
                    if !submenu.items.is_empty() {
                        cleaned.push(OwnedMenuItem::Submenu(submenu));
                        last_was_separator = false;
                    }
                }
                item => {
                    cleaned.push(item);
                    last_was_separator = false;
                }
            }
        }

        // Remove trailing separator
        if let Some(OwnedMenuItem::Separator) = cleaned.last() {
            cleaned.pop();
        }

        cleaned
    }

    /// Convert OwnedMenuItem to ContextMenuItem for rendering.
    fn convert_menu_items(
        items: Vec<OwnedMenuItem>,
        registry: &Arc<MenuActionRegistry>,
    ) -> Vec<ContextMenuItem> {
        let sanitized = Self::sanitize_menu_items(items);
        sanitized.into_iter().filter_map(|item| Self::convert_menu_item(item, registry)).collect()
    }

    /// Convert a single OwnedMenuItem to ContextMenuItem.
    fn convert_menu_item(
        item: OwnedMenuItem,
        registry: &Arc<MenuActionRegistry>,
    ) -> Option<ContextMenuItem> {
        match item {
            OwnedMenuItem::Separator => Some(ContextMenuItem::Separator),
            OwnedMenuItem::Action { name, .. } => {
                // Create an action item that dispatches via the registry
                let action_id = SharedString::from(name.clone());
                let registry_clone = registry.clone();
                Some(ContextMenuItem::action(name.clone(), move |cx| {
                    // Dispatch the action via the registry
                    registry_clone.dispatch(&action_id, cx);
                }))
            }
            OwnedMenuItem::Submenu(submenu) => {
                let sub_items = Self::convert_menu_items(submenu.items, registry);
                if sub_items.is_empty() {
                    None
                } else {
                    Some(ContextMenuItem::submenu(submenu.name.clone(), sub_items))
                }
            }
            OwnedMenuItem::SystemMenu(_) => {
                // System menus don't make sense in the context menu
                None
            }
        }
    }

    /// Build a context menu from menu items.
    fn build_menu_from_entry(
        entry: &MenuEntry,
        registry: &Arc<MenuActionRegistry>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<Entity<ContextMenu>> {
        let items = Self::convert_menu_items(entry.menu.items.clone(), registry);
        if items.is_empty() {
            return None;
        }

        // Calculate position at origin - PopoverMenu will position it properly
        let position = Point { x: px(0.0), y: px(0.0) };

        Some(cx.new(|cx| ContextMenu::new(position, cx).items(items)))
    }

    /// Check if any menu is currently deployed.
    pub fn any_menu_deployed(&self) -> bool {
        self.entries.iter().any(|entry| entry.handle.is_deployed())
    }

    /// Navigate to the menu on the left.
    #[cfg(not(target_os = "macos"))]
    pub fn navigate_left(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current_index = self.entries.iter().position(|entry| entry.handle.is_deployed());
        let Some(current_index) = current_index else {
            return;
        };

        let next_index =
            if current_index == 0 { self.entries.len() - 1 } else { current_index - 1 };

        self.entries[current_index].handle.hide(cx);

        // Defer showing the next menu to allow the hide to complete
        let next_handle = self.entries[next_index].handle.clone();
        cx.defer_in(window, move |_, window, cx| next_handle.show(window, cx));
    }

    /// Navigate to the menu on the right.
    #[cfg(not(target_os = "macos"))]
    pub fn navigate_right(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current_index = self.entries.iter().position(|entry| entry.handle.is_deployed());
        let Some(current_index) = current_index else {
            return;
        };

        let next_index =
            if current_index == self.entries.len() - 1 { 0 } else { current_index + 1 };

        self.entries[current_index].handle.hide(cx);

        // Defer showing the next menu to allow the hide to complete
        let next_handle = self.entries[next_index].handle.clone();
        cx.defer_in(window, move |_, window, cx| next_handle.show(window, cx));
    }

    /// Render a single menu entry as a button with popover.
    fn render_menu_entry(&self, entry: &MenuEntry) -> impl IntoElement {
        let menu_name = entry.menu.name.clone();
        let entry_clone = entry.clone();
        let registry = self.action_registry.clone();
        let current_handle = entry.handle.clone();
        let all_handles: Vec<_> = self.entries.iter().map(|entry| entry.handle.clone()).collect();

        div()
            .id(SharedString::from(format!("{}-menu-item", menu_name)))
            .occlude()
            .child(
                PopoverMenu::new(SharedString::from(format!("{}-menu-popover", menu_name)))
                    .menu(move |window, cx| {
                        Self::build_menu_from_entry(&entry_clone, &registry, window, cx)
                    })
                    .trigger(
                        Button::new(SharedString::from(format!(
                            "{}-menu-trigger",
                            menu_name.clone()
                        )))
                        .label(menu_name)
                        .style(ButtonStyle::Ghost)
                        .small(),
                    )
                    .anchor(Corner::TopLeft)
                    .attach(Corner::BottomLeft)
                    .with_handle(current_handle.clone()),
            )
            // On hover, if another menu is deployed, switch to this one
            .on_hover(move |hover_enter, window, cx| {
                if *hover_enter && !current_handle.is_deployed() {
                    // Check if any other menu is deployed
                    let any_deployed = all_handles.iter().any(|h| h.is_deployed());
                    if any_deployed {
                        // Hide all other menus
                        all_handles.iter().for_each(|h| h.hide(cx));

                        // Show this menu after a brief delay to allow hide to complete
                        let handle = current_handle.clone();
                        window.defer(cx, move |window, cx| handle.show(window, cx));
                    }
                }
            })
    }
}

impl Render for ApplicationMenu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        div()
            .id("application-menu")
            .key_context("ApplicationMenu")
            .track_focus(&self.focus_handle)
            .flex()
            .flex_row()
            .items_center()
            .h(px(28.0))
            .px(px(4.0))
            .gap(px(2.0))
            .bg(theme.colors.tab_bar_background)
            .border_b_1()
            .border_color(theme.colors.border)
            // Register navigation actions
            .on_action(cx.listener(|_this, _: &ActivateMenuLeft, _window, _cx| {
                #[cfg(not(target_os = "macos"))]
                _this.navigate_left(_window, _cx);
            }))
            .on_action(cx.listener(|_this, _: &ActivateMenuRight, _window, _cx| {
                #[cfg(not(target_os = "macos"))]
                _this.navigate_right(_window, _cx);
            }))
            // Render menu entries
            .children(self.entries.iter().map(|entry| self.render_menu_entry(entry)))
    }
}
