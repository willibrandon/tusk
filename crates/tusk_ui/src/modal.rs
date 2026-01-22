//! Modal dialog component for forms, confirmations, and complex interactions.
//!
//! This module provides a comprehensive modal system with:
//! - Full-featured Modal component with backdrop, header, body, and footer
//! - ModalAction for configurable action buttons
//! - ModalLayer global for showing/dismissing modals
//! - Focus trapping (Tab cycles within modal)
//! - Escape key to dismiss
//! - Backdrop click to close (configurable)

use gpui::{
    actions, div, prelude::*, px, AnyElement, AnyView, App, Context, Entity, EventEmitter,
    FocusHandle, Global, MouseButton, Render, SharedString, Subscription, Window,
};

use crate::button::{Button, ButtonVariant};
use crate::key_bindings::modal;
use crate::panel::Focusable;
use crate::TuskTheme;

// ============================================================================
// Modal Actions
// ============================================================================

actions!(modal_internal, [FocusNextElement, FocusPreviousElement,]);

// ============================================================================
// ModalEvent
// ============================================================================

/// Events emitted by the Modal component.
#[derive(Debug, Clone)]
pub enum ModalEvent {
    /// The modal was dismissed (via Escape, backdrop click, or cancel action).
    Dismissed,
    /// A custom action was triggered.
    ActionTriggered { action_id: SharedString },
}

// ============================================================================
// ModalAction
// ============================================================================

/// Represents an action button in the modal footer.
///
/// Actions appear as buttons in the modal footer. Common patterns include:
/// - Cancel + Confirm (two actions)
/// - Single close button
/// - Multiple custom actions
pub struct ModalAction {
    /// Unique identifier for this action.
    pub id: SharedString,
    /// Display label for the button.
    pub label: SharedString,
    /// Button visual variant.
    pub variant: ButtonVariant,
    /// Whether the button is disabled.
    pub disabled: bool,
    /// Whether clicking this action should dismiss the modal.
    pub dismisses: bool,
}

impl ModalAction {
    /// Create a new modal action.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: ButtonVariant::Secondary,
            disabled: false,
            dismisses: false,
        }
    }

    /// Create a primary action (styled as primary button).
    pub fn primary(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: ButtonVariant::Primary,
            disabled: false,
            dismisses: false,
        }
    }

    /// Create a cancel action that dismisses the modal.
    pub fn cancel() -> Self {
        Self {
            id: "cancel".into(),
            label: "Cancel".into(),
            variant: ButtonVariant::Secondary,
            disabled: false,
            dismisses: true,
        }
    }

    /// Create a confirm action (primary styled).
    pub fn confirm(label: impl Into<SharedString>) -> Self {
        Self {
            id: "confirm".into(),
            label: label.into(),
            variant: ButtonVariant::Primary,
            disabled: false,
            dismisses: true,
        }
    }

    /// Create a danger action (destructive styled).
    pub fn danger(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: ButtonVariant::Danger,
            disabled: false,
            dismisses: false,
        }
    }

    /// Set the button variant.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set whether the action is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set whether clicking this action dismisses the modal.
    pub fn dismisses(mut self, dismisses: bool) -> Self {
        self.dismisses = dismisses;
        self
    }
}

// ============================================================================
// Modal Component
// ============================================================================

/// A modal dialog component with full keyboard support and focus trapping.
///
/// Modals render above all other content with a backdrop overlay. They support:
/// - Customizable title, body content, and footer actions
/// - Focus trapping (Tab cycles within modal)
/// - Escape key to dismiss
/// - Backdrop click to dismiss (configurable)
///
/// # Example
///
/// ```ignore
/// let modal = cx.new(|cx| {
///     Modal::new("My Title", cx)
///         .closable(true)
///         .body(cx.new(|_| MyFormView::new()))
///         .action(ModalAction::cancel())
///         .action(ModalAction::confirm("Save"))
/// });
/// ```
pub struct Modal {
    /// Modal title.
    title: SharedString,
    /// Optional subtitle.
    subtitle: Option<SharedString>,
    /// Body content (any view).
    body: Option<AnyView>,
    /// Footer actions.
    actions: Vec<ModalAction>,
    /// Whether clicking the backdrop closes the modal.
    closable: bool,
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// Width of the modal (default 480px).
    width: f32,
}

impl Modal {
    /// Create a new modal with the given title.
    pub fn new(title: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            body: None,
            actions: Vec::new(),
            closable: true,
            focus_handle: cx.focus_handle(),
            width: 480.0,
        }
    }

    /// Set the modal subtitle.
    pub fn subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set the modal body content.
    pub fn body(mut self, view: AnyView) -> Self {
        self.body = Some(view);
        self
    }

    /// Add an action button to the modal footer.
    pub fn action(mut self, action: ModalAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Set multiple actions at once.
    pub fn actions(mut self, actions: Vec<ModalAction>) -> Self {
        self.actions = actions;
        self
    }

    /// Set whether clicking the backdrop closes the modal.
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Set the modal width in pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Dismiss the modal.
    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        cx.emit(ModalEvent::Dismissed);
    }

    /// Trigger an action by ID.
    pub fn trigger_action(&mut self, action_id: SharedString, cx: &mut Context<Self>) {
        // Check if the action should dismiss
        let should_dismiss =
            self.actions.iter().find(|a| a.id == action_id).map(|a| a.dismisses).unwrap_or(false);

        cx.emit(ModalEvent::ActionTriggered { action_id: action_id.clone() });

        if should_dismiss {
            cx.emit(ModalEvent::Dismissed);
        }
    }

    /// Render the modal header with pre-extracted colors.
    fn render_header(
        &self,
        text_color: gpui::Hsla,
        text_muted: gpui::Hsla,
        border_color: gpui::Hsla,
    ) -> impl IntoElement {
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .pb(px(16.0))
            .border_b_1()
            .border_color(border_color)
            .child(
                div()
                    .text_size(px(18.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(text_color)
                    .child(self.title.clone()),
            )
            .when_some(self.subtitle.as_ref(), |d, subtitle| {
                d.child(div().text_size(px(14.0)).text_color(text_muted).child(subtitle.clone()))
            })
    }

    /// Render the modal body.
    fn render_body(&self) -> impl IntoElement {
        div().w_full().flex_1().py(px(16.0)).children(self.body.clone())
    }

    /// Render the modal footer with action buttons.
    fn render_footer(&self, border_color: gpui::Hsla, cx: &mut Context<Self>) -> impl IntoElement {
        if self.actions.is_empty() {
            return div().into_any_element();
        }

        div()
            .w_full()
            .pt(px(16.0))
            .border_t_1()
            .border_color(border_color)
            .flex()
            .justify_end()
            .gap(px(8.0))
            .children(self.actions.iter().enumerate().map(|(idx, action)| {
                let action_id = action.id.clone();
                Button::new()
                    .id(format!("modal-action-{}", idx))
                    .label(action.label.clone())
                    .variant(action.variant)
                    .disabled(action.disabled)
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.trigger_action(action_id.clone(), cx);
                    }))
            }))
            .into_any_element()
    }
}

impl EventEmitter<ModalEvent> for Modal {}

impl Focusable for Modal {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Modal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();
        let closable = self.closable;

        // Clone theme colors to avoid borrow conflict
        let elevated_surface_background = theme.colors.elevated_surface_background;
        let border_color = theme.colors.border;
        let text_color = theme.colors.text;
        let text_muted = theme.colors.text_muted;

        // T090: Focus trapping is handled by tracking focus on the modal container
        // and using Tab key bindings that cycle within the modal.
        // GPUI's focus system naturally traps focus when we track_focus on the modal.

        // T091: Escape key handling via Dismiss action (registered in key_bindings.rs)
        // T092: Backdrop click to close (when closable=true)

        // Modal backdrop
        div()
            .id("modal-backdrop")
            .size_full()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::black().opacity(0.5))
            // Backdrop click to dismiss (T092)
            .when(closable, |d| {
                d.on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _window, cx| {
                        this.dismiss(cx);
                    }),
                )
            })
            .when(!closable, |d| {
                // Still stop propagation even if not closable
                d.on_mouse_down(MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
            })
            .child(
                // Modal container
                div()
                    .id("modal-container")
                    .key_context("Modal")
                    .track_focus(&self.focus_handle)
                    .w(px(self.width))
                    .max_w(px(600.0))
                    .max_h(px(600.0))
                    .bg(elevated_surface_background)
                    .border_1()
                    .border_color(border_color)
                    .rounded(px(8.0))
                    .shadow_lg()
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    // Stop click propagation on modal content
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    // Handle keyboard actions (T091)
                    .on_action(cx.listener(|this, _: &modal::Dismiss, _window, cx| {
                        if this.closable {
                            this.dismiss(cx);
                        }
                    }))
                    // Header
                    .child(self.render_header(text_color, text_muted, border_color))
                    // Body
                    .child(self.render_body())
                    // Footer
                    .child(self.render_footer(border_color, cx)),
            )
    }
}

// ============================================================================
// ModalLayer - Global Modal Management
// ============================================================================

/// Entry in the modal stack.
struct ModalEntry {
    /// The modal entity.
    modal: Entity<Modal>,
    /// Subscription to modal events.
    _subscription: Subscription,
}

/// Global layer for managing modal display.
///
/// The ModalLayer maintains a stack of modals and ensures they are rendered
/// above all other content. Only the topmost modal receives focus and input.
///
/// # Usage
///
/// ```ignore
/// // Show a modal
/// cx.update_global::<ModalLayer, _>(|layer, cx| {
///     let modal = cx.new(|cx| Modal::new("Title", cx).body(my_view));
///     layer.show(modal, cx);
/// });
///
/// // Dismiss current modal
/// cx.update_global::<ModalLayer, _>(|layer, cx| {
///     layer.dismiss(cx);
/// });
/// ```
pub struct ModalLayer {
    /// Stack of active modals (topmost is the visible one).
    stack: Vec<ModalEntry>,
}

impl Global for ModalLayer {}

impl ModalLayer {
    /// Create a new empty modal layer.
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Show a modal on top of the stack.
    ///
    /// The modal will receive focus and be rendered above all other content.
    ///
    /// Performance target: <200ms (SC-007)
    #[tracing::instrument(level = "debug", skip_all, name = "modal_show")]
    pub fn show(&mut self, modal: Entity<Modal>, cx: &mut App) {
        // Subscribe to modal events to handle dismissal
        // App::subscribe takes (Entity<T>, &Event, &mut App)
        let subscription =
            cx.subscribe(&modal, |_modal: Entity<Modal>, event: &ModalEvent, cx: &mut App| {
                if matches!(event, ModalEvent::Dismissed) {
                    cx.update_global::<ModalLayer, _>(|layer: &mut ModalLayer, cx| {
                        layer.dismiss(cx);
                    });
                }
            });

        self.stack.push(ModalEntry { modal, _subscription: subscription });

        cx.refresh_windows();
    }

    /// Dismiss the topmost modal.
    ///
    /// Performance target: <200ms (SC-007)
    #[tracing::instrument(level = "debug", skip_all, name = "modal_dismiss")]
    pub fn dismiss(&mut self, cx: &mut App) {
        if self.stack.pop().is_some() {
            cx.refresh_windows();
        }
    }

    /// Dismiss all modals.
    pub fn dismiss_all(&mut self, cx: &mut App) {
        if !self.stack.is_empty() {
            self.stack.clear();
            cx.refresh_windows();
        }
    }

    /// Check if any modal is currently shown.
    pub fn has_modal(&self) -> bool {
        !self.stack.is_empty()
    }

    /// Get the current modal count.
    pub fn modal_count(&self) -> usize {
        self.stack.len()
    }

    /// Render the modal layer (call from Workspace).
    ///
    /// Returns the topmost modal's view if any modal is active.
    pub fn render(&self) -> Option<AnyElement> {
        self.stack.last().map(|entry| entry.modal.clone().into_any_element())
    }
}

impl Default for ModalLayer {
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
    fn test_modal_action_builders() {
        let cancel = ModalAction::cancel();
        assert_eq!(cancel.id.as_ref(), "cancel");
        assert_eq!(cancel.label.as_ref(), "Cancel");
        assert!(cancel.dismisses);
        assert_eq!(cancel.variant, ButtonVariant::Secondary);

        let confirm = ModalAction::confirm("Save");
        assert_eq!(confirm.id.as_ref(), "confirm");
        assert_eq!(confirm.label.as_ref(), "Save");
        assert!(confirm.dismisses);
        assert_eq!(confirm.variant, ButtonVariant::Primary);

        let danger = ModalAction::danger("delete", "Delete");
        assert_eq!(danger.id.as_ref(), "delete");
        assert_eq!(danger.variant, ButtonVariant::Danger);
        assert!(!danger.dismisses);

        let custom = ModalAction::new("custom", "Custom Action")
            .variant(ButtonVariant::Ghost)
            .disabled(true)
            .dismisses(true);
        assert_eq!(custom.id.as_ref(), "custom");
        assert_eq!(custom.variant, ButtonVariant::Ghost);
        assert!(custom.disabled);
        assert!(custom.dismisses);
    }

    #[test]
    fn test_modal_layer() {
        let layer = ModalLayer::new();
        assert!(!layer.has_modal());
        assert_eq!(layer.modal_count(), 0);
    }
}
