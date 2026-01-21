//! Generic tree component with virtualized rendering.
//!
//! This module provides a reusable tree component that supports:
//! - Virtualized rendering via GPUI's UniformList (60fps for 1000+ items)
//! - Expand/collapse with keyboard navigation
//! - Single selection with click and keyboard
//! - Filtering with recursive descendant matching
//! - Event emission for selection, activation, and context menus

use std::collections::HashSet;
use std::hash::Hash;
use std::ops::Range;

use gpui::{
    div, prelude::*, px, uniform_list, Context, EventEmitter, FocusHandle, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, UniformListScrollHandle, Window,
};

use crate::icon::{Icon, IconName, IconSize};
use crate::key_bindings::tree::{
    ActivateSelected, CollapseAll, CollapseSelected, ExpandAll, ExpandSelected, SelectNext,
    SelectPrevious,
};
use crate::layout::spacing;
use crate::TuskTheme;

/// Trait for items that can be displayed in a tree.
pub trait TreeItem: Clone + 'static {
    /// The type used to uniquely identify items.
    type Id: Clone + Eq + Hash + std::fmt::Display + 'static;

    /// Returns the unique identifier for this item.
    fn id(&self) -> Self::Id;

    /// Returns the display label for this item.
    fn label(&self) -> SharedString;

    /// Returns an optional icon for this item.
    fn icon(&self) -> Option<IconName>;

    /// Returns the children of this item, if any.
    fn children(&self) -> Option<&[Self]>;

    /// Returns whether this item can be expanded (has children).
    fn is_expandable(&self) -> bool {
        self.children().is_some()
    }
}

/// Events emitted by the Tree component.
#[derive(Clone, Debug)]
pub enum TreeEvent<Id> {
    /// An item was selected (single click).
    Selected { id: Id },
    /// An item was activated (double click or Enter).
    Activated { id: Id },
    /// An item was expanded.
    Expanded { id: Id },
    /// An item was collapsed.
    Collapsed { id: Id },
    /// Context menu was requested for an item.
    ContextMenu {
        id: Id,
        position: gpui::Point<gpui::Pixels>,
    },
}

/// A visible entry in the flattened tree, including depth information.
#[derive(Clone)]
pub struct VisibleEntry<T: TreeItem> {
    /// The tree item.
    pub item: T,
    /// The depth in the tree (0 = root).
    pub depth: usize,
}

/// A generic tree component with virtualized rendering.
pub struct Tree<T: TreeItem> {
    /// All root items in the tree.
    items: Vec<T>,
    /// IDs of currently expanded items.
    expanded: HashSet<T::Id>,
    /// Currently selected item ID.
    selected: Option<T::Id>,
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// Scroll handle for the uniform list.
    scroll_handle: UniformListScrollHandle,
    /// Current filter text (empty = no filter).
    filter_text: String,
    /// Cached visible entries (flattened tree).
    visible_entries: Vec<VisibleEntry<T>>,
}

impl<T: TreeItem> Tree<T> {
    /// Create a new tree with the given root items.
    pub fn new(items: Vec<T>, cx: &mut Context<Self>) -> Self {
        let mut tree = Self {
            items,
            expanded: HashSet::new(),
            selected: None,
            focus_handle: cx.focus_handle(),
            scroll_handle: UniformListScrollHandle::new(),
            filter_text: String::new(),
            visible_entries: Vec::new(),
        };
        tree.rebuild_visible_entries();
        tree
    }

    /// Get the root items.
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Set new root items.
    pub fn set_items(&mut self, items: Vec<T>, cx: &mut Context<Self>) {
        self.items = items;
        self.rebuild_visible_entries();
        cx.notify();
    }

    /// Get the currently selected item ID.
    pub fn selected(&self) -> Option<&T::Id> {
        self.selected.as_ref()
    }

    /// Set the selected item by ID.
    pub fn set_selected(&mut self, id: Option<T::Id>, cx: &mut Context<Self>) {
        self.selected = id;
        cx.notify();
    }

    /// Check if an item is expanded.
    pub fn is_expanded(&self, id: &T::Id) -> bool {
        self.expanded.contains(id)
    }

    /// Expand an item.
    pub fn expand(&mut self, id: T::Id, cx: &mut Context<Self>) {
        if self.expanded.insert(id.clone()) {
            self.rebuild_visible_entries();
            cx.emit(TreeEvent::Expanded { id });
            cx.notify();
        }
    }

    /// Collapse an item.
    pub fn collapse(&mut self, id: T::Id, cx: &mut Context<Self>) {
        if self.expanded.remove(&id) {
            self.rebuild_visible_entries();
            cx.emit(TreeEvent::Collapsed { id });
            cx.notify();
        }
    }

    /// Toggle expand/collapse state of an item.
    pub fn toggle_expanded(&mut self, id: T::Id, cx: &mut Context<Self>) {
        if self.expanded.contains(&id) {
            self.collapse(id, cx);
        } else {
            self.expand(id, cx);
        }
    }

    /// Expand all items.
    pub fn expand_all(&mut self, cx: &mut Context<Self>) {
        self.expand_all_recursive(&self.items.clone());
        self.rebuild_visible_entries();
        cx.notify();
    }

    fn expand_all_recursive(&mut self, items: &[T]) {
        for item in items {
            if item.is_expandable() {
                self.expanded.insert(item.id());
                if let Some(children) = item.children() {
                    self.expand_all_recursive(children);
                }
            }
        }
    }

    /// Collapse all items.
    pub fn collapse_all(&mut self, cx: &mut Context<Self>) {
        self.expanded.clear();
        self.rebuild_visible_entries();
        cx.notify();
    }

    /// Set filter text for filtering items.
    pub fn set_filter(&mut self, filter: String, cx: &mut Context<Self>) {
        self.filter_text = filter;
        self.rebuild_visible_entries();
        cx.notify();
    }

    /// Get the visible entries (flattened tree).
    pub fn visible_entries(&self) -> &[VisibleEntry<T>] {
        &self.visible_entries
    }

    /// Rebuild the cached visible entries.
    fn rebuild_visible_entries(&mut self) {
        self.visible_entries.clear();

        if self.filter_text.is_empty() {
            self.flatten_items(&self.items.clone(), 0);
        } else {
            let filter = self.filter_text.to_lowercase();
            self.flatten_items_filtered(&self.items.clone(), 0, &filter);
        }
    }

    fn flatten_items(&mut self, items: &[T], depth: usize) {
        for item in items {
            self.visible_entries.push(VisibleEntry {
                item: item.clone(),
                depth,
            });

            if item.is_expandable() && self.expanded.contains(&item.id()) {
                if let Some(children) = item.children() {
                    self.flatten_items(children, depth + 1);
                }
            }
        }
    }

    fn flatten_items_filtered(&mut self, items: &[T], depth: usize, filter: &str) {
        for item in items {
            let label_matches = item.label().to_lowercase().contains(filter);
            let descendant_matches = self.has_matching_descendant(item, filter);

            if label_matches || descendant_matches {
                self.visible_entries.push(VisibleEntry {
                    item: item.clone(),
                    depth,
                });

                // When filtering, auto-expand items with matching descendants
                if descendant_matches {
                    if let Some(children) = item.children() {
                        self.flatten_items_filtered(children, depth + 1, filter);
                    }
                } else if item.is_expandable() && self.expanded.contains(&item.id()) {
                    if let Some(children) = item.children() {
                        self.flatten_items_filtered(children, depth + 1, filter);
                    }
                }
            }
        }
    }

    fn has_matching_descendant(&self, item: &T, filter: &str) -> bool {
        if let Some(children) = item.children() {
            for child in children {
                if child.label().to_lowercase().contains(filter) {
                    return true;
                }
                if self.has_matching_descendant(child, filter) {
                    return true;
                }
            }
        }
        false
    }

    /// Find the index of an item in visible entries.
    fn find_visible_index(&self, id: &T::Id) -> Option<usize> {
        self.visible_entries
            .iter()
            .position(|entry| entry.item.id() == *id)
    }

    /// Select the next item in the tree.
    fn select_next(&mut self, cx: &mut Context<Self>) {
        if self.visible_entries.is_empty() {
            return;
        }

        let next_index = if let Some(selected_id) = &self.selected {
            if let Some(current_index) = self.find_visible_index(selected_id) {
                (current_index + 1).min(self.visible_entries.len() - 1)
            } else {
                0
            }
        } else {
            0
        };

        let id = self.visible_entries[next_index].item.id();
        self.selected = Some(id.clone());
        self.scroll_handle
            .scroll_to_item(next_index, gpui::ScrollStrategy::Nearest);
        cx.emit(TreeEvent::Selected { id });
        cx.notify();
    }

    /// Select the previous item in the tree.
    fn select_previous(&mut self, cx: &mut Context<Self>) {
        if self.visible_entries.is_empty() {
            return;
        }

        let prev_index = if let Some(selected_id) = &self.selected {
            if let Some(current_index) = self.find_visible_index(selected_id) {
                current_index.saturating_sub(1)
            } else {
                0
            }
        } else {
            0
        };

        let id = self.visible_entries[prev_index].item.id();
        self.selected = Some(id.clone());
        self.scroll_handle
            .scroll_to_item(prev_index, gpui::ScrollStrategy::Nearest);
        cx.emit(TreeEvent::Selected { id });
        cx.notify();
    }

    /// Expand the currently selected item.
    fn expand_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.selected.clone() {
            self.expand(id, cx);
        }
    }

    /// Collapse the currently selected item.
    fn collapse_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.selected.clone() {
            self.collapse(id, cx);
        }
    }

    /// Activate the currently selected item (emit Activated event).
    fn activate_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.selected.clone() {
            cx.emit(TreeEvent::Activated { id });
        }
    }

    /// Render a single tree item row.
    fn render_item(
        &self,
        entry: &VisibleEntry<T>,
        is_selected: bool,
        theme: &TuskTheme,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let item_id = entry.item.id();
        let is_expandable = entry.item.is_expandable();
        let is_expanded = self.is_expanded(&item_id);
        let indent = px(entry.depth as f32 * 16.0 + 4.0);

        let item_id_for_click = item_id.clone();
        let item_id_for_toggle = item_id.clone();
        let item_id_for_context = item_id.clone();

        div()
            .id(format!("tree-item-{}", item_id))
            .h(px(24.0))
            .w_full()
            .flex()
            .items_center()
            .pl(indent)
            .pr(spacing::SM)
            .cursor_pointer()
            .when(is_selected, |d| {
                d.bg(theme.colors.list_active_selection_background)
            })
            .hover(|d| {
                if !is_selected {
                    d.bg(theme.colors.list_hover_background)
                } else {
                    d
                }
            })
            .on_click(cx.listener(move |this, e: &gpui::ClickEvent, _window, cx| {
                let id = item_id_for_click.clone();
                this.selected = Some(id.clone());
                cx.emit(TreeEvent::Selected { id: id.clone() });

                if e.click_count() == 2 {
                    cx.emit(TreeEvent::Activated { id });
                }

                cx.notify();
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |_this, e: &gpui::MouseDownEvent, _window, cx| {
                    let id = item_id_for_context.clone();
                    cx.emit(TreeEvent::ContextMenu {
                        id,
                        position: e.position,
                    });
                }),
            )
            .child(
                // Chevron for expandable items
                div()
                    .id(format!("tree-chevron-{}", item_id_for_toggle))
                    .w(px(16.0))
                    .h(px(16.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(is_expandable, |d| {
                        let toggle_id = item_id_for_toggle.clone();
                        d.cursor_pointer()
                            .on_click(
                                cx.listener(move |this, _e: &gpui::ClickEvent, _window, cx| {
                                    this.toggle_expanded(toggle_id.clone(), cx);
                                }),
                            )
                            .child(
                                Icon::new(if is_expanded {
                                    IconName::ChevronDown
                                } else {
                                    IconName::ChevronRight
                                })
                                .size(IconSize::Small)
                                .color(theme.colors.text_muted),
                            )
                    }),
            )
            .child(
                // Icon
                div()
                    .w(px(16.0))
                    .h(px(16.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .mr(spacing::XS)
                    .when_some(entry.item.icon(), |d, icon_name| {
                        d.child(Icon::new(icon_name).size(IconSize::Small).color(
                            if is_selected {
                                theme.colors.text
                            } else {
                                theme.colors.text_muted
                            },
                        ))
                    }),
            )
            .child(
                // Label
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(if is_selected {
                        theme.colors.text
                    } else {
                        theme.colors.text_muted
                    })
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(entry.item.label()),
            )
    }
}

impl<T: TreeItem> EventEmitter<TreeEvent<T::Id>> for Tree<T> {}

impl<T: TreeItem> Render for Tree<T> {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>().clone();
        let item_count = self.visible_entries.len();
        let selected_id = self.selected.clone();

        div()
            .id("tree")
            .key_context("Tree")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &SelectNext, _window, cx| {
                this.select_next(cx)
            }))
            .on_action(cx.listener(|this, _: &SelectPrevious, _window, cx| {
                this.select_previous(cx)
            }))
            .on_action(cx.listener(|this, _: &ExpandSelected, _window, cx| {
                this.expand_selected(cx)
            }))
            .on_action(cx.listener(|this, _: &CollapseSelected, _window, cx| {
                this.collapse_selected(cx)
            }))
            .on_action(cx.listener(|this, _: &ActivateSelected, _window, cx| {
                this.activate_selected(cx)
            }))
            .on_action(cx.listener(|this, _: &ExpandAll, _window, cx| {
                this.expand_all(cx)
            }))
            .on_action(cx.listener(|this, _: &CollapseAll, _window, cx| {
                this.collapse_all(cx)
            }))
            .size_full()
            .overflow_hidden()
            .child(
                uniform_list("tree-items", item_count, {
                    let theme = theme.clone();
                    cx.processor(move |this, range: Range<usize>, _window, cx| {
                        let mut items = Vec::with_capacity(range.len());
                        for i in range {
                            if let Some(entry) = this.visible_entries.get(i) {
                                let is_selected = selected_id
                                    .as_ref()
                                    .map(|s| *s == entry.item.id())
                                    .unwrap_or(false);
                                items.push(this.render_item(entry, is_selected, &theme, cx));
                            }
                        }
                        items
                    })
                })
                .size_full()
                .track_scroll(&self.scroll_handle),
            )
    }
}
