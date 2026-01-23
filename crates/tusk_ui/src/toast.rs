//! Toast notification system for recoverable errors (T061, FR-022).
//!
//! Toast notifications are used to display recoverable errors with auto-dismiss
//! behavior (10 seconds by default). They appear at the bottom of the workspace
//! and can include an action button.
//!
//! Display rules per error-handling.md:
//! - Recoverable errors without position: Toast notification (auto-dismiss 10s)
//! - Errors with position (query errors): Error panel instead
//! - Non-recoverable errors: Error modal instead

use gpui::{
    div, prelude::*, px, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Global, Render,
    SharedString, Subscription, Task, Window,
};
use std::time::Duration;

use crate::icon::{Icon, IconName, IconSize};
use crate::TuskTheme;

/// Default toast duration (10 seconds per FR-022).
const DEFAULT_TOAST_DURATION: Duration = Duration::from_secs(10);

/// Toast severity levels for styling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ToastSeverity {
    /// Informational message (blue).
    #[default]
    Info,
    /// Warning message (yellow).
    Warning,
    /// Error message (red).
    Error,
    /// Success message (green).
    Success,
}

impl ToastSeverity {
    /// Get the icon for this severity.
    pub fn icon(&self) -> IconName {
        match self {
            Self::Info => IconName::Info,
            Self::Warning => IconName::Warning,
            Self::Error => IconName::Error,
            Self::Success => IconName::Check,
        }
    }
}

/// A toast notification.
pub struct Toast {
    /// The message to display.
    message: SharedString,
    /// Optional hint or additional context.
    hint: Option<SharedString>,
    /// Severity level for styling.
    severity: ToastSeverity,
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
}

impl Toast {
    /// Create a new toast with a message.
    pub fn new(message: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            message: message.into(),
            hint: None,
            severity: ToastSeverity::Info,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Create an info toast.
    pub fn info(message: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            message: message.into(),
            hint: None,
            severity: ToastSeverity::Info,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Create a warning toast.
    pub fn warning(message: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            message: message.into(),
            hint: None,
            severity: ToastSeverity::Warning,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Create an error toast.
    pub fn error(message: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            message: message.into(),
            hint: None,
            severity: ToastSeverity::Error,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Create a success toast.
    pub fn success(message: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            message: message.into(),
            hint: None,
            severity: ToastSeverity::Success,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Set the hint text.
    pub fn with_hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Dismiss this toast.
    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
    }
}

impl EventEmitter<DismissEvent> for Toast {}

impl Render for Toast {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        // Determine colors based on severity
        let (icon_color, border_color) = match self.severity {
            ToastSeverity::Info => (theme.colors.accent, theme.colors.accent),
            ToastSeverity::Warning => (theme.colors.warning, theme.colors.warning),
            ToastSeverity::Error => (theme.colors.error, theme.colors.error),
            ToastSeverity::Success => (theme.colors.success, theme.colors.success),
        };

        div()
            .track_focus(&self.focus_handle)
            .id("toast")
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(16.0))
            .py(px(12.0))
            .min_w(px(300.0))
            .max_w(px(500.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(border_color.opacity(0.3))
            .bg(theme.colors.panel_background)
            .shadow_md()
            // Icon
            .child(Icon::new(self.severity.icon()).size(IconSize::Medium).color(icon_color))
            // Content
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    // Message
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(theme.colors.text)
                            .child(self.message.clone()),
                    )
                    // Hint (if present)
                    .when_some(self.hint.clone(), |s, hint| {
                        s.child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted)
                                .child(hint),
                        )
                    }),
            )
            // Close button
            .child(
                div()
                    .id("toast-close")
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(20.0))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.element_hover))
                    .on_click(cx.listener(|this, _, _, cx| this.dismiss(cx)))
                    .child(
                        Icon::new(IconName::Close)
                            .size(IconSize::Small)
                            .color(theme.colors.text_muted),
                    ),
            )
    }
}

/// Toast layer for managing active toasts.
///
/// This is registered as a global and renders toast notifications
/// at the bottom of the workspace.
pub struct ToastLayer {
    /// Currently active toast.
    active_toast: Option<ActiveToast>,
}

struct ActiveToast {
    toast: Entity<Toast>,
    _subscription: Subscription,
    _dismiss_task: Task<()>,
}

impl Default for ToastLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl ToastLayer {
    /// Create a new toast layer.
    pub fn new() -> Self {
        Self { active_toast: None }
    }

    /// Show a toast notification.
    ///
    /// This replaces any existing toast with the new one.
    pub fn show_toast(&mut self, toast: Entity<Toast>, cx: &mut Context<Self>) {
        // Subscribe to dismiss event
        let subscription = cx.subscribe(&toast, |this, _, _: &DismissEvent, cx| {
            this.hide_toast(cx);
        });

        // Start auto-dismiss timer (10 seconds per FR-022)
        let dismiss_task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(DEFAULT_TOAST_DURATION).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| this.hide_toast(cx));
            }
        });

        self.active_toast =
            Some(ActiveToast { toast, _subscription: subscription, _dismiss_task: dismiss_task });

        cx.notify();
    }

    /// Hide the current toast.
    pub fn hide_toast(&mut self, cx: &mut Context<Self>) {
        self.active_toast.take();
        cx.notify();
    }

    /// Check if there's an active toast.
    pub fn has_active_toast(&self) -> bool {
        self.active_toast.is_some()
    }

    /// Render the toast layer.
    pub fn render(&self) -> Option<impl IntoElement> {
        let active_toast = self.active_toast.as_ref()?;
        Some(
            div().absolute().size_full().bottom_0().left_0().child(
                div()
                    .absolute()
                    .w_full()
                    .bottom(px(60.0)) // Above status bar
                    .flex()
                    .justify_center()
                    .child(active_toast.toast.clone()),
            ),
        )
    }
}

impl Global for ToastLayer {}

impl Render for ToastLayer {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let Some(active_toast) = &self.active_toast else {
            return div();
        };

        div().absolute().size_full().bottom_0().left_0().child(
            div()
                .absolute()
                .w_full()
                .bottom(px(60.0)) // Above status bar
                .flex()
                .justify_center()
                .child(active_toast.toast.clone()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_severity_icons() {
        assert_eq!(ToastSeverity::Info.icon(), IconName::Info);
        assert_eq!(ToastSeverity::Warning.icon(), IconName::Warning);
        assert_eq!(ToastSeverity::Error.icon(), IconName::Error);
        assert_eq!(ToastSeverity::Success.icon(), IconName::Check);
    }
}
