//! Messages panel for displaying query messages and notices.
//!
//! The messages panel lives in the bottom dock and displays:
//! - Query execution messages (info, warning, error)
//! - PostgreSQL notices and warnings
//! - System messages

use gpui::{
    div, prelude::*, px, App, Context, EventEmitter, FocusHandle, Render, SharedString, Window,
};

use crate::icon::{Icon, IconName, IconSize};
use crate::panel::{DockPosition, Focusable, Panel, PanelEvent};
use crate::TuskTheme;

/// Severity level of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSeverity {
    /// Informational message.
    Info,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
    /// Success message.
    Success,
}

impl MessageSeverity {
    /// Get the icon for this severity.
    fn icon(&self) -> IconName {
        match self {
            MessageSeverity::Info => IconName::Info,
            MessageSeverity::Warning => IconName::Warning,
            MessageSeverity::Error => IconName::Error,
            MessageSeverity::Success => IconName::Check,
        }
    }
}

/// A message entry in the messages panel.
#[derive(Debug, Clone)]
pub struct Message {
    /// The message severity.
    pub severity: MessageSeverity,
    /// The message text.
    pub text: String,
    /// Optional timestamp (as formatted string).
    pub timestamp: Option<String>,
}

impl Message {
    /// Create a new info message.
    pub fn info(text: impl Into<String>) -> Self {
        Self { severity: MessageSeverity::Info, text: text.into(), timestamp: None }
    }

    /// Create a new warning message.
    pub fn warning(text: impl Into<String>) -> Self {
        Self { severity: MessageSeverity::Warning, text: text.into(), timestamp: None }
    }

    /// Create a new error message.
    pub fn error(text: impl Into<String>) -> Self {
        Self { severity: MessageSeverity::Error, text: text.into(), timestamp: None }
    }

    /// Create a new success message.
    pub fn success(text: impl Into<String>) -> Self {
        Self { severity: MessageSeverity::Success, text: text.into(), timestamp: None }
    }

    /// Add a timestamp to the message.
    pub fn with_timestamp(mut self, timestamp: impl Into<String>) -> Self {
        self.timestamp = Some(timestamp.into());
        self
    }
}

/// Messages panel for displaying query messages and notices.
///
/// This panel shows informational messages, warnings, and errors from
/// query execution and the database server.
pub struct MessagesPanel {
    /// Focus handle for keyboard navigation.
    focus_handle: FocusHandle,
    /// List of messages to display.
    messages: Vec<Message>,
}

impl MessagesPanel {
    /// Create a new messages panel.
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle(), messages: Vec::new() }
    }

    /// Get the messages.
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Add a message to the panel.
    pub fn add_message(&mut self, message: Message, cx: &mut Context<Self>) {
        self.messages.push(message);
        cx.notify();
    }

    /// Add multiple messages to the panel.
    pub fn add_messages(&mut self, messages: Vec<Message>, cx: &mut Context<Self>) {
        self.messages.extend(messages);
        cx.notify();
    }

    /// Clear all messages.
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.messages.clear();
        cx.notify();
    }

    /// Render the empty state.
    fn render_empty_state(&self, theme: &TuskTheme) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap(px(12.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(48.0))
                    .rounded(px(8.0))
                    .bg(theme.colors.element_background)
                    .child(
                        Icon::new(IconName::Info)
                            .size(IconSize::XLarge)
                            .color(theme.colors.text_muted),
                    ),
            )
            .child(
                div().text_color(theme.colors.text_muted).text_size(px(13.0)).child("No messages"),
            )
            .child(
                div()
                    .text_color(theme.colors.text_muted)
                    .text_size(px(12.0))
                    .child("Messages and notices will appear here"),
            )
    }

    /// Render a single message.
    fn render_message(&self, message: &Message, theme: &TuskTheme) -> impl IntoElement {
        let icon_color = match message.severity {
            MessageSeverity::Info => theme.colors.accent,
            MessageSeverity::Warning => theme.colors.warning,
            MessageSeverity::Error => theme.colors.error,
            MessageSeverity::Success => theme.colors.success,
        };

        div()
            .w_full()
            .px(px(12.0))
            .py(px(6.0))
            .flex()
            .items_start()
            .gap(px(8.0))
            .border_b_1()
            .border_color(theme.colors.border.opacity(0.5))
            .child(Icon::new(message.severity.icon()).size(IconSize::Small).color(icon_color))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.colors.text)
                            .child(message.text.clone()),
                    )
                    .when_some(message.timestamp.as_ref(), |d, timestamp| {
                        d.child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted)
                                .child(timestamp.clone()),
                        )
                    }),
            )
    }

    /// Render the messages list.
    fn render_messages_list(&self, theme: &TuskTheme) -> impl IntoElement {
        div()
            .id("messages-list")
            .size_full()
            .overflow_y_scroll()
            .children(self.messages.iter().map(|msg| self.render_message(msg, theme)))
    }
}

impl EventEmitter<PanelEvent> for MessagesPanel {}

impl Focusable for MessagesPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for MessagesPanel {
    fn panel_id(&self) -> &'static str {
        "messages"
    }

    fn title(&self, _cx: &App) -> SharedString {
        "Messages".into()
    }

    fn icon(&self, _cx: &App) -> IconName {
        IconName::Info
    }

    fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
    }

    fn closable(&self, _cx: &App) -> bool {
        false // Messages panel is always visible when bottom dock is open
    }

    fn position(&self, _cx: &App) -> DockPosition {
        DockPosition::Bottom
    }
}

impl Render for MessagesPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();

        let content = if self.messages.is_empty() {
            self.render_empty_state(theme).into_any_element()
        } else {
            self.render_messages_list(theme).into_any_element()
        };

        // Badge showing message count if there are messages
        let badge = if !self.messages.is_empty() {
            Some(
                div()
                    .text_size(px(10.0))
                    .text_color(theme.colors.text_muted)
                    .child(format!("({})", self.messages.len())),
            )
        } else {
            None
        };

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.panel_background)
            .child(
                // Panel header
                div()
                    .h(px(32.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(Icon::new(IconName::Info).size(IconSize::Small))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text)
                                    .child("Messages"),
                            )
                            .children(badge),
                    ),
            )
            .child(
                // Panel content
                div().flex_1().overflow_hidden().child(content),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let info = Message::info("Test info");
        assert_eq!(info.severity, MessageSeverity::Info);
        assert_eq!(info.text, "Test info");
        assert!(info.timestamp.is_none());

        let error = Message::error("Test error").with_timestamp("12:34:56");
        assert_eq!(error.severity, MessageSeverity::Error);
        assert_eq!(error.timestamp, Some("12:34:56".to_string()));
    }

    #[test]
    fn test_message_severity_icon() {
        assert_eq!(MessageSeverity::Info.icon(), IconName::Info);
        assert_eq!(MessageSeverity::Warning.icon(), IconName::Warning);
        assert_eq!(MessageSeverity::Error.icon(), IconName::Error);
        assert_eq!(MessageSeverity::Success.icon(), IconName::Check);
    }
}
