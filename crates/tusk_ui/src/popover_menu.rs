//! Popover menu component for dropdown menus.
//!
//! This module provides:
//! - PopoverMenu component for button-triggered dropdown menus
//! - PopoverMenuHandle for programmatic control of menu state
//! - Integration with ContextMenu for menu rendering

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    anchored, deferred, div, point, prelude::*, px, AnyElement, App, Bounds, Corner, Element,
    ElementId, Entity, GlobalElementId, HitboxBehavior, HitboxId, IntoElement, LayoutId,
    MouseDownEvent, ParentElement, Pixels, Point, Window,
};

use crate::context_menu::{ContextMenu, ContextMenuEvent};

/// Trait for elements that can be used as popover menu triggers.
pub trait PopoverTrigger: IntoElement + 'static {}

impl<T: IntoElement + 'static> PopoverTrigger for T {}

// ============================================================================
// PopoverMenuHandle
// ============================================================================

/// Handle for programmatic control of a popover menu.
///
/// This allows showing/hiding the menu from outside the component,
/// which is useful for keyboard navigation between menus.
#[derive(Clone, Default)]
pub struct PopoverMenuHandle {
    state: Rc<RefCell<Option<PopoverMenuHandleState>>>,
}

struct PopoverMenuHandleState {
    menu_builder: Rc<dyn Fn(&mut Window, &mut App) -> Option<Entity<ContextMenu>>>,
    menu: Rc<RefCell<Option<Entity<ContextMenu>>>>,
}

impl PopoverMenuHandle {
    /// Show the menu.
    pub fn show(&self, window: &mut Window, cx: &mut App) {
        if let Some(state) = self.state.borrow().as_ref() {
            show_menu(&state.menu_builder, &state.menu, window, cx);
        }
    }

    /// Hide the menu.
    pub fn hide(&self, cx: &mut App) {
        if let Some(state) = self.state.borrow().as_ref() {
            if let Some(menu) = state.menu.borrow().as_ref() {
                menu.update(cx, |menu, cx| {
                    menu.dismiss(cx);
                });
            }
        }
    }

    /// Check if the menu is currently deployed (visible).
    pub fn is_deployed(&self) -> bool {
        self.state.borrow().as_ref().is_some_and(|state| state.menu.borrow().is_some())
    }
}

// ============================================================================
// PopoverMenu
// ============================================================================

/// A popover menu that appears when a trigger element is clicked.
pub struct PopoverMenu {
    id: ElementId,
    child_builder: Option<
        Box<
            dyn FnOnce(
                    Rc<RefCell<Option<Entity<ContextMenu>>>>,
                    Option<
                        Rc<dyn Fn(&mut Window, &mut App) -> Option<Entity<ContextMenu>> + 'static>,
                    >,
                ) -> AnyElement
                + 'static,
        >,
    >,
    menu_builder:
        Option<Rc<dyn Fn(&mut Window, &mut App) -> Option<Entity<ContextMenu>> + 'static>>,
    anchor: Corner,
    attach: Option<Corner>,
    offset: Option<Point<Pixels>>,
    trigger_handle: Option<PopoverMenuHandle>,
}

impl PopoverMenu {
    /// Create a new popover menu with the given ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            child_builder: None,
            menu_builder: None,
            anchor: Corner::TopLeft,
            attach: None,
            offset: None,
            trigger_handle: None,
        }
    }

    /// Set the menu builder function.
    pub fn menu(
        mut self,
        f: impl Fn(&mut Window, &mut App) -> Option<Entity<ContextMenu>> + 'static,
    ) -> Self {
        self.menu_builder = Some(Rc::new(f));
        self
    }

    /// Set an external handle for programmatic menu control.
    pub fn with_handle(mut self, handle: PopoverMenuHandle) -> Self {
        self.trigger_handle = Some(handle);
        self
    }

    /// Set the trigger element.
    pub fn trigger<T: PopoverTrigger>(mut self, t: T) -> Self {
        self.child_builder = Some(Box::new(move |menu, builder| {
            let is_open = menu.borrow().is_some();
            div()
                .id("popover-trigger")
                .cursor_pointer()
                .when(is_open, |d| d.bg(gpui::black().opacity(0.1)))
                .when_some(builder, |el, builder| {
                    el.on_click(move |_, window, cx| {
                        show_menu(&builder, &menu, window, cx);
                    })
                })
                .child(t)
                .into_any_element()
        }));
        self
    }

    /// Define which corner of the menu to anchor to the attachment point.
    pub fn anchor(mut self, anchor: Corner) -> Self {
        self.anchor = anchor;
        self
    }

    /// Define which corner of the trigger to attach the menu's anchor to.
    pub fn attach(mut self, attach: Corner) -> Self {
        self.attach = Some(attach);
        self
    }

    /// Offset the position of the menu by the given amount.
    pub fn offset(mut self, offset: Point<Pixels>) -> Self {
        self.offset = Some(offset);
        self
    }

    fn resolved_attach(&self) -> Corner {
        self.attach.unwrap_or(match self.anchor {
            Corner::TopLeft => Corner::BottomLeft,
            Corner::TopRight => Corner::BottomRight,
            Corner::BottomLeft => Corner::TopLeft,
            Corner::BottomRight => Corner::TopRight,
        })
    }

    fn resolved_offset(&self) -> Point<Pixels> {
        self.offset.unwrap_or_else(|| point(px(0.), px(4.)))
    }
}

fn show_menu(
    builder: &Rc<dyn Fn(&mut Window, &mut App) -> Option<Entity<ContextMenu>>>,
    menu: &Rc<RefCell<Option<Entity<ContextMenu>>>>,
    window: &mut Window,
    cx: &mut App,
) {
    // Build the new menu
    let Some(new_menu) = (builder)(window, cx) else {
        return;
    };

    let menu_ref = menu.clone();

    // Subscribe to dismiss events to clean up
    cx.subscribe(&new_menu, move |_menu_entity, event: &ContextMenuEvent, cx| {
        if matches!(event, ContextMenuEvent::Close) {
            *menu_ref.borrow_mut() = None;
            cx.refresh_windows();
        }
    })
    .detach();

    // Focus the menu
    let focus_handle = new_menu.read(cx).focus_handle.clone();
    window.focus(&focus_handle, cx);

    *menu.borrow_mut() = Some(new_menu);
    cx.refresh_windows();
}

// ============================================================================
// Element State
// ============================================================================

#[derive(Clone, Default)]
struct PopoverMenuElementState {
    menu: Rc<RefCell<Option<Entity<ContextMenu>>>>,
    child_bounds: Option<Bounds<Pixels>>,
}

pub struct PopoverMenuFrameState {
    child_layout_id: Option<LayoutId>,
    child_element: Option<AnyElement>,
    menu_element: Option<AnyElement>,
    menu_handle: Rc<RefCell<Option<Entity<ContextMenu>>>>,
}

// ============================================================================
// Element Implementation
// ============================================================================

impl Element for PopoverMenu {
    type RequestLayoutState = PopoverMenuFrameState;
    type PrepaintState = Option<HitboxId>;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        window.with_element_state(
            global_id.unwrap(),
            |element_state: Option<PopoverMenuElementState>, window| {
                let element_state = element_state.unwrap_or_default();
                let mut menu_layout_id = None;

                let menu_element = element_state.menu.borrow_mut().as_mut().map(|menu| {
                    let offset = self.resolved_offset();
                    let mut anch = anchored()
                        .snap_to_window_with_margin(px(8.))
                        .anchor(self.anchor)
                        .offset(offset);
                    if let Some(child_bounds) = element_state.child_bounds {
                        anch = anch.position(child_bounds.corner(self.resolved_attach()) + offset);
                    }
                    let mut element = deferred(anch.child(div().occlude().child(menu.clone())))
                        .with_priority(1)
                        .into_any();

                    menu_layout_id = Some(element.request_layout(window, cx));
                    element
                });

                let mut child_element = self.child_builder.take().map(|child_builder| {
                    (child_builder)(element_state.menu.clone(), self.menu_builder.clone())
                });

                // Set up the handle state if provided
                if let Some(trigger_handle) = self.trigger_handle.take() {
                    if let Some(menu_builder) = self.menu_builder.clone() {
                        *trigger_handle.state.borrow_mut() = Some(PopoverMenuHandleState {
                            menu_builder,
                            menu: element_state.menu.clone(),
                        });
                    }
                }

                let child_layout_id = child_element
                    .as_mut()
                    .map(|child_element| child_element.request_layout(window, cx));

                let layout_id = window.request_layout(
                    gpui::Style::default(),
                    menu_layout_id.into_iter().chain(child_layout_id),
                    cx,
                );

                (
                    (
                        layout_id,
                        PopoverMenuFrameState {
                            child_element,
                            child_layout_id,
                            menu_element,
                            menu_handle: element_state.menu.clone(),
                        },
                    ),
                    element_state,
                )
            },
        )
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<HitboxId> {
        if let Some(child) = request_layout.child_element.as_mut() {
            child.prepaint(window, cx);
        }

        if let Some(menu) = request_layout.menu_element.as_mut() {
            menu.prepaint(window, cx);
        }

        request_layout.child_layout_id.map(|layout_id| {
            let bounds = window.layout_bounds(layout_id);
            window.with_element_state(global_id.unwrap(), |element_state, _cx| {
                let mut element_state: PopoverMenuElementState = element_state.unwrap();
                element_state.child_bounds = Some(bounds);
                ((), element_state)
            });

            window.insert_hitbox(bounds, HitboxBehavior::Normal).id
        })
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        child_hitbox: &mut Option<HitboxId>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(mut child) = request_layout.child_element.take() {
            child.paint(window, cx);
        }

        if let Some(mut menu) = request_layout.menu_element.take() {
            menu.paint(window, cx);

            if let Some(child_hitbox) = *child_hitbox {
                let menu_handle = request_layout.menu_handle.clone();
                // Mouse-downing outside the menu dismisses it, so we don't
                // want a click on the toggle to re-open it.
                window.on_mouse_event(move |_: &MouseDownEvent, phase, window, cx| {
                    if phase == gpui::DispatchPhase::Bubble && child_hitbox.is_hovered(window) {
                        if let Some(menu) = menu_handle.borrow().as_ref() {
                            menu.update(cx, |menu, cx| {
                                menu.dismiss(cx);
                            });
                        }
                        cx.stop_propagation();
                    }
                });
            }
        }
    }
}

impl IntoElement for PopoverMenu {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_popover_menu_handle_default() {
        let handle = PopoverMenuHandle::default();
        assert!(!handle.is_deployed());
    }
}
