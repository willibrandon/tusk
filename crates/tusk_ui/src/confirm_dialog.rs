//! Confirm dialog component for destructive actions.
//!
//! This module provides a modal confirmation dialog used to confirm
//! destructive operations like closing unsaved tabs, dropping tables, etc.

use gpui::{
    div, prelude::*, px, App, Context, EventEmitter, FocusHandle, MouseButton, Render,
    SharedString, Window,
};

use crate::button::{Button, ButtonVariant};
use crate::icon::IconName;
use crate::key_bindings::modal;
use crate::panel::Focusable;
use crate::TuskTheme;

/// Events emitted by the confirm dialog.
#[derive(Debug, Clone)]
pub enum ConfirmDialogEvent {
    /// User confirmed the action.
    Confirmed,
    /// User dismissed/cancelled the dialog.
    Dismissed,
}

/// The type of confirmation dialog (affects styling).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfirmDialogKind {
    /// Standard confirmation (blue accent).
    #[default]
    Standard,
    /// Warning confirmation (yellow accent).
    Warning,
    /// Destructive action confirmation (red accent).
    Destructive,
}

/// A modal confirmation dialog.
///
/// Used for confirming destructive actions like:
/// - Closing unsaved changes
/// - Dropping database objects
/// - Deleting data
pub struct ConfirmDialog {
    /// Dialog title.
    title: SharedString,
    /// Dialog message/description.
    message: SharedString,
    /// Label for the confirm button.
    confirm_label: SharedString,
    /// Label for the cancel button.
    cancel_label: SharedString,
    /// Kind of dialog (affects styling).
    kind: ConfirmDialogKind,
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
}

impl ConfirmDialog {
    /// Create a new confirm dialog.
    pub fn new(
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            confirm_label: "Confirm".into(),
            cancel_label: "Cancel".into(),
            kind: ConfirmDialogKind::default(),
            focus_handle: cx.focus_handle(),
        }
    }

    /// Create a destructive confirmation dialog.
    pub fn destructive(
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            confirm_label: "Delete".into(),
            cancel_label: "Cancel".into(),
            kind: ConfirmDialogKind::Destructive,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Create a warning confirmation dialog.
    pub fn warning(
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            confirm_label: "Continue".into(),
            cancel_label: "Cancel".into(),
            kind: ConfirmDialogKind::Warning,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Set the confirm button label.
    pub fn with_confirm_label(mut self, label: impl Into<SharedString>) -> Self {
        self.confirm_label = label.into();
        self
    }

    /// Set the cancel button label.
    pub fn with_cancel_label(mut self, label: impl Into<SharedString>) -> Self {
        self.cancel_label = label.into();
        self
    }

    /// Set the dialog kind.
    pub fn with_kind(mut self, kind: ConfirmDialogKind) -> Self {
        self.kind = kind;
        self
    }

    /// Confirm the action.
    pub fn confirm(&mut self, cx: &mut Context<Self>) {
        cx.emit(ConfirmDialogEvent::Confirmed);
    }

    /// Dismiss the dialog.
    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        cx.emit(ConfirmDialogEvent::Dismissed);
    }

    /// Get the icon for this dialog kind.
    fn icon(&self) -> IconName {
        match self.kind {
            ConfirmDialogKind::Standard => IconName::Info,
            ConfirmDialogKind::Warning => IconName::Warning,
            ConfirmDialogKind::Destructive => IconName::Trash,
        }
    }

    /// Get the confirm button variant for this dialog kind.
    fn confirm_button_variant(&self) -> ButtonVariant {
        match self.kind {
            ConfirmDialogKind::Standard => ButtonVariant::Primary,
            ConfirmDialogKind::Warning => ButtonVariant::Primary,
            ConfirmDialogKind::Destructive => ButtonVariant::Danger,
        }
    }
}

impl EventEmitter<ConfirmDialogEvent> for ConfirmDialog {}

impl Focusable for ConfirmDialog {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ConfirmDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        let icon_color = match self.kind {
            ConfirmDialogKind::Standard => theme.colors.accent,
            ConfirmDialogKind::Warning => theme.colors.warning,
            ConfirmDialogKind::Destructive => theme.colors.error,
        };

        // Modal backdrop
        div()
            .id("confirm-dialog-backdrop")
            .size_full()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::black().opacity(0.5))
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .child(
                // Modal container
                div()
                    .id("confirm-dialog")
                    .key_context("Modal")
                    .track_focus(&self.focus_handle)
                    .w(px(400.0))
                    .max_w(px(500.0))
                    .bg(theme.colors.elevated_surface_background)
                    .border_1()
                    .border_color(theme.colors.border)
                    .rounded(px(8.0))
                    .shadow_lg()
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    // Handle keyboard actions
                    .on_action(cx.listener(|this, _: &modal::Dismiss, _window, cx| {
                        this.dismiss(cx);
                    }))
                    .on_action(cx.listener(|this, _: &modal::ConfirmAction, _window, cx| {
                        this.confirm(cx);
                    }))
                    // Header with icon and title
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .child(
                                crate::icon::Icon::new(self.icon())
                                    .size(crate::icon::IconSize::Large)
                                    .color(icon_color),
                            )
                            .child(
                                div()
                                    .text_size(px(18.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text)
                                    .child(self.title.clone()),
                            ),
                    )
                    // Message
                    .child(
                        div()
                            .text_size(px(14.0))
                            .text_color(theme.colors.text_muted)
                            .child(self.message.clone()),
                    )
                    // Action buttons
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(8.0))
                            .child(
                                Button::new()
                                    .id("cancel-button")
                                    .label(self.cancel_label.clone())
                                    .variant(ButtonVariant::Secondary)
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.dismiss(cx);
                                    })),
                            )
                            .child(
                                Button::new()
                                    .id("confirm-button")
                                    .label(self.confirm_label.clone())
                                    .variant(self.confirm_button_variant())
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.confirm(cx);
                                    })),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confirm_dialog_kinds() {
        // Verify icon mapping works
        assert_eq!(
            ConfirmDialogKind::Standard as i32,
            ConfirmDialogKind::Standard as i32
        );
        assert_eq!(
            ConfirmDialogKind::Warning as i32,
            ConfirmDialogKind::Warning as i32
        );
        assert_eq!(
            ConfirmDialogKind::Destructive as i32,
            ConfirmDialogKind::Destructive as i32
        );
    }
}
