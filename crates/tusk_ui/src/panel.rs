//! Panel trait and types for dock content.
//!
//! Panels are the content areas that live inside docks. Each panel implements
//! the Panel trait and can be registered with a dock.
//!
//! This module uses the handle pattern from Zed: instead of `Entity<dyn Panel>`,
//! we use `Arc<dyn PanelHandle>` where PanelHandle is implemented for all `Entity<T: Panel>`.

use std::sync::Arc;

use gpui::{
    AnyView, App, Context, Entity, EntityId, EventEmitter, FocusHandle, Render, SharedString,
    Window,
};

use crate::icon::IconName;

/// Position of a dock in the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum DockPosition {
    /// Left side of the workspace.
    #[default]
    Left,
    /// Right side of the workspace.
    Right,
    /// Bottom of the workspace.
    Bottom,
}

/// Events emitted by panels.
#[derive(Debug, Clone)]
pub enum PanelEvent {
    /// Panel requests focus.
    Focus,
    /// Panel requests to be closed.
    Close,
    /// Panel wants to activate a specific tab.
    ActivateTab(usize),
}

/// Trait for focusable components.
pub trait Focusable: 'static {
    /// Get the focus handle for this component.
    fn focus_handle(&self, cx: &App) -> FocusHandle;
}

/// Trait for dock panel content.
///
/// Panels are the main content areas that live inside docks.
/// Each panel type (schema browser, results, messages, etc.) implements this trait.
pub trait Panel: Render + Focusable + EventEmitter<PanelEvent> + Sized + 'static {
    /// Unique identifier for this panel type.
    fn panel_id(&self) -> &'static str;

    /// Display title for the panel tab.
    fn title(&self, cx: &App) -> SharedString;

    /// Icon for the panel tab.
    fn icon(&self, cx: &App) -> IconName;

    /// Focus the primary interactive element in this panel.
    fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>);

    /// Whether the panel can be closed (default: true).
    fn closable(&self, _cx: &App) -> bool {
        true
    }

    /// Whether the panel has unsaved changes (default: false).
    fn is_dirty(&self, _cx: &App) -> bool {
        false
    }

    /// Preferred position for this panel (default: Left).
    fn position(&self, _cx: &App) -> DockPosition {
        DockPosition::Left
    }
}

/// Type-erased handle for panels.
///
/// This trait allows storing heterogeneous panels in a collection.
/// It is automatically implemented for all `Entity<T>` where `T: Panel`.
pub trait PanelHandle: Send + Sync {
    /// Get the entity ID for this panel.
    fn entity_id(&self) -> EntityId;

    /// Get the panel type ID.
    fn panel_id(&self, cx: &App) -> &'static str;

    /// Get the panel title.
    fn title(&self, cx: &App) -> SharedString;

    /// Get the panel icon.
    fn icon(&self, cx: &App) -> IconName;

    /// Focus the panel.
    fn focus(&self, window: &mut Window, cx: &mut App);

    /// Check if the panel is closable.
    fn closable(&self, cx: &App) -> bool;

    /// Check if the panel has unsaved changes.
    fn is_dirty(&self, cx: &App) -> bool;

    /// Get the preferred position.
    fn position(&self, cx: &App) -> DockPosition;

    /// Get the focus handle.
    fn focus_handle(&self, cx: &App) -> FocusHandle;

    /// Convert to an AnyView for rendering.
    fn to_any(&self) -> AnyView;

    /// Clone as a boxed trait object.
    fn boxed_clone(&self) -> Arc<dyn PanelHandle>;
}

/// Blanket implementation of PanelHandle for all Entity<T: Panel>.
impl<T: Panel> PanelHandle for Entity<T> {
    fn entity_id(&self) -> EntityId {
        Entity::entity_id(self)
    }

    fn panel_id(&self, cx: &App) -> &'static str {
        self.read(cx).panel_id()
    }

    fn title(&self, cx: &App) -> SharedString {
        self.read(cx).title(cx)
    }

    fn icon(&self, cx: &App) -> IconName {
        self.read(cx).icon(cx)
    }

    fn focus(&self, window: &mut Window, cx: &mut App) {
        self.update(cx, |panel, cx| panel.focus(window, cx));
    }

    fn closable(&self, cx: &App) -> bool {
        self.read(cx).closable(cx)
    }

    fn is_dirty(&self, cx: &App) -> bool {
        self.read(cx).is_dirty(cx)
    }

    fn position(&self, cx: &App) -> DockPosition {
        self.read(cx).position(cx)
    }

    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.read(cx).focus_handle(cx)
    }

    fn to_any(&self) -> AnyView {
        self.clone().into()
    }

    fn boxed_clone(&self) -> Arc<dyn PanelHandle> {
        Arc::new(self.clone())
    }
}

/// A stored panel entry in a dock.
pub struct PanelEntry {
    /// The panel handle (type-erased).
    pub panel: Arc<dyn PanelHandle>,
}

impl PanelEntry {
    /// Create a new panel entry from a panel entity.
    pub fn new<P: Panel>(entity: Entity<P>) -> Self {
        Self {
            panel: Arc::new(entity),
        }
    }

    /// Get the panel as a specific type if it matches.
    pub fn downcast<P: Panel>(&self) -> Option<Entity<P>> {
        self.panel.to_any().downcast().ok()
    }
}

impl Clone for PanelEntry {
    fn clone(&self) -> Self {
        Self {
            panel: self.panel.boxed_clone(),
        }
    }
}
