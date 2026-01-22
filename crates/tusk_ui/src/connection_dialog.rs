//! Connection dialog for database connections.
//!
//! Provides a modal dialog for:
//! - Entering connection credentials (T039, T040)
//! - SSL mode selection (T041)
//! - Connect and Test Connection buttons (T042, T043)
//! - Connection progress indicator (T044)
//! - Error display with actionable hints (T045)

use gpui::{
    div, prelude::*, px, App, Context, Entity, FocusHandle, Focusable, Render, Task, Window,
};

use crate::key_bindings::form::{Tab, TabPrev};
use uuid::Uuid;

use crate::select::{Select, SelectOption};
use crate::spinner::{Spinner, SpinnerSize};
use crate::text_input::TextInput;
use crate::TuskTheme;

#[cfg(feature = "persistence")]
use tusk_core::{ConnectionConfig, ConnectionOptions, SslMode, TuskState};

/// SSL mode value for the select component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SslModeValue(pub String);

/// State of the connection dialog.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ConnectionDialogState {
    /// Ready for input.
    #[default]
    Idle,
    /// Connection attempt in progress.
    Connecting,
    /// Test connection in progress.
    Testing,
    /// Test connection succeeded.
    TestSuccess,
    /// Connection successful.
    Connected { connection_id: Uuid },
    /// Connection failed with error.
    Error { message: String, hint: Option<String> },
}

impl ConnectionDialogState {
    /// Check if the dialog is in a loading state.
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Connecting | Self::Testing)
    }

    /// Check if the dialog has an error.
    pub fn has_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    /// Check if test connection succeeded.
    pub fn is_test_success(&self) -> bool {
        matches!(self, Self::TestSuccess)
    }

    /// Get the error message if present.
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Error { message, .. } => Some(message),
            _ => None,
        }
    }

    /// Get the error hint if present.
    pub fn error_hint(&self) -> Option<&str> {
        match self {
            Self::Error { hint, .. } => hint.as_deref(),
            _ => None,
        }
    }
}

/// Events emitted by the connection dialog.
#[derive(Debug, Clone)]
pub enum ConnectionDialogEvent {
    /// Connection was successful.
    Connected { connection_id: Uuid },
    /// Dialog was cancelled/closed.
    Cancelled,
}

/// Connection dialog component (T039-T045).
pub struct ConnectionDialog {
    /// Focus handle for the dialog.
    focus_handle: FocusHandle,
    /// Current dialog state.
    state: ConnectionDialogState,
    /// Host input field.
    host_input: Entity<TextInput>,
    /// Port input field.
    port_input: Entity<TextInput>,
    /// Database name input field.
    database_input: Entity<TextInput>,
    /// Username input field.
    username_input: Entity<TextInput>,
    /// Password input field.
    password_input: Entity<TextInput>,
    /// SSL mode selector.
    ssl_mode_select: Entity<Select<SslModeValue>>,
    /// Connection name (optional).
    connection_name: String,
    /// Background task for connection attempts.
    _connection_task: Option<Task<()>>,
}

impl ConnectionDialog {
    /// Create a new connection dialog.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let host_input = cx.new(|cx| {
            let mut input = TextInput::new("localhost", cx);
            input.set_text("localhost", cx);
            input.set_tab_index(1);
            input
        });

        let port_input = cx.new(|cx| {
            let mut input = TextInput::new("5432", cx);
            input.set_text("5432", cx);
            input.set_tab_index(2);
            input
        });

        let database_input = cx.new(|cx| {
            let mut input = TextInput::new("postgres", cx);
            input.set_text("postgres", cx);
            input.set_tab_index(3);
            input
        });

        let username_input = cx.new(|cx| {
            let mut input = TextInput::new("postgres", cx);
            input.set_text("postgres", cx);
            input.set_tab_index(4);
            input
        });

        let password_input = cx.new(|cx| {
            let mut input = TextInput::new("Enter password", cx);
            input.set_password(true);
            input.set_tab_index(5);
            input
        });

        // SSL mode options (T041)
        let ssl_options = vec![
            SelectOption::new(SslModeValue("prefer".to_string()), "Prefer"),
            SelectOption::new(SslModeValue("disable".to_string()), "Disable"),
            SelectOption::new(SslModeValue("require".to_string()), "Require"),
            SelectOption::new(SslModeValue("verify-ca".to_string()), "Verify CA"),
            SelectOption::new(SslModeValue("verify-full".to_string()), "Verify Full"),
        ];

        let ssl_mode_select = cx.new(|cx| {
            Select::new("ssl-mode-select", ssl_options, cx)
                .selected(Some(SslModeValue("prefer".to_string())))
        });

        Self {
            focus_handle: cx.focus_handle(),
            state: ConnectionDialogState::Idle,
            host_input,
            port_input,
            database_input,
            username_input,
            password_input,
            ssl_mode_select,
            connection_name: String::new(),
            _connection_task: None,
        }
    }

    /// Get the current state.
    pub fn state(&self) -> &ConnectionDialogState {
        &self.state
    }

    /// Set connection name.
    pub fn set_connection_name(&mut self, name: impl Into<String>) {
        self.connection_name = name.into();
    }

    /// Pre-fill the form with existing connection config.
    #[cfg(feature = "persistence")]
    pub fn set_config(&mut self, config: &ConnectionConfig, cx: &mut Context<Self>) {
        self.connection_name = config.name.clone();

        self.host_input.update(cx, |input, cx| {
            input.set_text(&config.host, cx);
        });

        self.port_input.update(cx, |input, cx| {
            input.set_text(&config.port.to_string(), cx);
        });

        self.database_input.update(cx, |input, cx| {
            input.set_text(&config.database, cx);
        });

        self.username_input.update(cx, |input, cx| {
            input.set_text(&config.username, cx);
        });

        let ssl_value = match config.ssl_mode {
            SslMode::Disable => "disable",
            SslMode::Prefer => "prefer",
            SslMode::Require => "require",
            SslMode::VerifyCa => "verify-ca",
            SslMode::VerifyFull => "verify-full",
        };

        self.ssl_mode_select.update(cx, |select, cx| {
            select.set_selected(Some(SslModeValue(ssl_value.to_string())), cx);
        });

        cx.notify();
    }

    /// Get the current form values as a ConnectionConfig.
    #[cfg(feature = "persistence")]
    fn get_config(&self, cx: &App) -> Option<ConnectionConfig> {
        let host = self.host_input.read(cx).text().to_string();
        let port_str = self.port_input.read(cx).text().to_string();
        let database = self.database_input.read(cx).text().to_string();
        let username = self.username_input.read(cx).text().to_string();

        let port: u16 = port_str.parse().ok()?;

        let ssl_mode = match self
            .ssl_mode_select
            .read(cx)
            .selected_value()
            .map(|v| v.0.as_str())
        {
            Some("disable") => SslMode::Disable,
            Some("require") => SslMode::Require,
            Some("verify-ca") => SslMode::VerifyCa,
            Some("verify-full") => SslMode::VerifyFull,
            _ => SslMode::Prefer,
        };

        let name = if self.connection_name.is_empty() {
            format!("{}@{}:{}/{}", username, host, port, database)
        } else {
            self.connection_name.clone()
        };

        Some(ConnectionConfig {
            id: Uuid::new_v4(),
            name,
            host,
            port,
            database,
            username,
            ssl_mode,
            ssh_tunnel: None,
            options: ConnectionOptions::default(),
            color: None,
        })
    }

    /// Get the password from the form.
    fn get_password(&self, cx: &App) -> String {
        self.password_input.read(cx).text().to_string()
    }

    /// Attempt to connect (T042).
    #[cfg(feature = "persistence")]
    pub fn connect(&mut self, cx: &mut Context<Self>) {
        use std::sync::Arc;
        use tusk_core::services::ConnectionPool;

        if self.state.is_loading() {
            return;
        }

        let Some(config) = self.get_config(cx) else {
            self.state = ConnectionDialogState::Error {
                message: "Invalid port number".to_string(),
                hint: Some("Port must be a number between 1 and 65535".to_string()),
            };
            cx.notify();
            return;
        };

        let password = self.get_password(cx);

        if password.is_empty() {
            self.state = ConnectionDialogState::Error {
                message: "Password is required".to_string(),
                hint: Some("Enter the database password".to_string()),
            };
            cx.notify();
            return;
        }

        // Update state to connecting (T044)
        self.state = ConnectionDialogState::Connecting;
        cx.notify();

        // Get runtime handle from TuskState
        let Some(tusk_state) = cx.try_global::<TuskState>() else {
            self.state = ConnectionDialogState::Error {
                message: "Application not initialized".to_string(),
                hint: Some("Please restart the application".to_string()),
            };
            cx.notify();
            return;
        };
        let runtime_handle = tusk_state.runtime().handle().clone();

        // Clone config for async block
        let config_clone = config.clone();
        let password_clone = password.clone();

        self._connection_task = Some(cx.spawn(async move |this, cx| {
            // Create connection pool on tokio runtime
            let pool_result = runtime_handle
                .spawn(async move { ConnectionPool::new(config_clone.clone(), &password_clone).await })
                .await;

            let result = match pool_result {
                Ok(Ok(pool)) => Ok((config.clone(), Arc::new(pool))),
                Ok(Err(e)) => Err(e),
                Err(e) => Err(tusk_core::TuskError::internal(format!(
                    "Connection task panicked: {e}"
                ))),
            };

            let _ = this.update(cx, |dialog, cx| {
                match result {
                    Ok((config, pool)) => {
                        // Register connection with TuskState
                        if let Some(tusk_state) = cx.try_global::<TuskState>() {
                            tusk_state.add_connection_arc(config.clone(), pool);

                            // Store password in credential service (T050)
                            if let Err(e) = tusk_state.store_password(config.id, &password) {
                                tracing::warn!(
                                    connection_id = %config.id,
                                    error = %e,
                                    "Failed to store password in keychain"
                                );
                            }
                        }

                        dialog.state = ConnectionDialogState::Connected {
                            connection_id: config.id,
                        };
                        cx.emit(ConnectionDialogEvent::Connected {
                            connection_id: config.id,
                        });
                    }
                    Err(e) => {
                        // Extract error info for display (T045)
                        let error_info = e.to_error_info();
                        dialog.state = ConnectionDialogState::Error {
                            message: error_info.message,
                            hint: error_info.hint,
                        };
                    }
                }
                cx.notify();
            });
        }));
    }

    /// Connect placeholder for non-persistence builds.
    #[cfg(not(feature = "persistence"))]
    pub fn connect(&mut self, cx: &mut Context<Self>) {
        self.state = ConnectionDialogState::Error {
            message: "Connection requires persistence feature".to_string(),
            hint: None,
        };
        cx.notify();
    }

    /// Test connection without storing credentials (T043).
    #[cfg(feature = "persistence")]
    pub fn test_connection(&mut self, cx: &mut Context<Self>) {
        use tusk_core::services::ConnectionPool;

        if self.state.is_loading() {
            return;
        }

        let Some(config) = self.get_config(cx) else {
            self.state = ConnectionDialogState::Error {
                message: "Invalid port number".to_string(),
                hint: Some("Port must be a number between 1 and 65535".to_string()),
            };
            cx.notify();
            return;
        };

        let password = self.get_password(cx);

        if password.is_empty() {
            self.state = ConnectionDialogState::Error {
                message: "Password is required".to_string(),
                hint: Some("Enter the database password".to_string()),
            };
            cx.notify();
            return;
        }

        // Update state to testing (T044)
        self.state = ConnectionDialogState::Testing;
        cx.notify();

        // Get runtime handle from TuskState
        let Some(tusk_state) = cx.try_global::<TuskState>() else {
            self.state = ConnectionDialogState::Error {
                message: "Application not initialized".to_string(),
                hint: Some("Please restart the application".to_string()),
            };
            cx.notify();
            return;
        };
        let runtime_handle = tusk_state.runtime().handle().clone();

        self._connection_task = Some(cx.spawn(async move |this, cx| {
            // Test connection by creating a pool and immediately dropping it
            let result = runtime_handle
                .spawn(async move {
                    let pool = ConnectionPool::new(config, &password).await?;
                    // Immediately close the test pool
                    pool.close();
                    Ok::<(), tusk_core::TuskError>(())
                })
                .await;

            let _ = this.update(cx, |dialog, cx| {
                match result {
                    Ok(Ok(())) => {
                        // Test succeeded - show success feedback
                        dialog.state = ConnectionDialogState::TestSuccess;
                    }
                    Ok(Err(e)) => {
                        // Extract error info for display (T045)
                        let error_info = e.to_error_info();
                        dialog.state = ConnectionDialogState::Error {
                            message: error_info.message,
                            hint: error_info.hint,
                        };
                    }
                    Err(e) => {
                        dialog.state = ConnectionDialogState::Error {
                            message: format!("Test connection task failed: {e}"),
                            hint: None,
                        };
                    }
                }
                cx.notify();
            });
        }));
    }

    /// Test connection placeholder for non-persistence builds.
    #[cfg(not(feature = "persistence"))]
    pub fn test_connection(&mut self, cx: &mut Context<Self>) {
        self.state = ConnectionDialogState::Error {
            message: "Test connection requires persistence feature".to_string(),
            hint: None,
        };
        cx.notify();
    }

    /// Cancel and close the dialog.
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        self._connection_task = None;
        self.state = ConnectionDialogState::Idle;
        cx.emit(ConnectionDialogEvent::Cancelled);
        cx.notify();
    }

    /// Handle Tab action to cycle focus to next field.
    fn on_tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        let handles = [
            self.host_input.focus_handle(cx),
            self.port_input.focus_handle(cx),
            self.database_input.focus_handle(cx),
            self.username_input.focus_handle(cx),
            self.password_input.focus_handle(cx),
        ];

        if let Some(current) = handles.iter().position(|h| h.is_focused(window)) {
            let next = (current + 1) % handles.len();
            handles[next].focus(window, cx);
        }
    }

    /// Handle Shift+Tab action to cycle focus to previous field.
    fn on_tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        let handles = [
            self.host_input.focus_handle(cx),
            self.port_input.focus_handle(cx),
            self.database_input.focus_handle(cx),
            self.username_input.focus_handle(cx),
            self.password_input.focus_handle(cx),
        ];

        if let Some(current) = handles.iter().position(|h| h.is_focused(window)) {
            let next = if current == 0 {
                handles.len() - 1
            } else {
                current - 1
            };
            handles[next].focus(window, cx);
        }
    }

    /// Clear any error or success state.
    pub fn clear_error(&mut self, cx: &mut Context<Self>) {
        if self.state.has_error() || self.state.is_test_success() {
            self.state = ConnectionDialogState::Idle;
            cx.notify();
        }
    }

    /// Render a form field with label.
    fn render_field(
        &self,
        label: &str,
        input: Entity<TextInput>,
        theme: &TuskTheme,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.colors.text_muted)
                    .child(label.to_string()),
            )
            .child(input.clone())
    }

    /// Render the error section (T045).
    fn render_error(&self, theme: &TuskTheme) -> impl IntoElement {
        if let ConnectionDialogState::Error { message, hint } = &self.state {
            div()
                .p(px(12.0))
                .rounded(px(4.0))
                .bg(theme.colors.error.opacity(0.1))
                .border_1()
                .border_color(theme.colors.error.opacity(0.3))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(theme.colors.error)
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child(message.clone()),
                        )
                        .when_some(hint.clone(), |el, hint| {
                            el.child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_muted)
                                    .child(hint),
                            )
                        }),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }

    /// Render the success section for test connection.
    fn render_success(&self, theme: &TuskTheme) -> impl IntoElement {
        if self.state.is_test_success() {
            div()
                .p(px(12.0))
                .rounded(px(4.0))
                .bg(theme.colors.success.opacity(0.1))
                .border_1()
                .border_color(theme.colors.success.opacity(0.3))
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.colors.success)
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child("Connection successful!"),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }

    /// Render the button section.
    fn render_buttons(&self, theme: &TuskTheme, cx: &mut Context<Self>) -> impl IntoElement {
        let is_loading = self.state.is_loading();
        let is_connecting = matches!(self.state, ConnectionDialogState::Connecting);
        let is_testing = matches!(self.state, ConnectionDialogState::Testing);

        div()
            .flex()
            .justify_between()
            .gap(px(12.0))
            .child(
                // Test Connection button (T043)
                div()
                    .id("test-connection-button")
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(16.0))
                    .py(px(8.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(theme.colors.border)
                    .when(!is_loading, |el| {
                        el.hover(|s| s.bg(theme.colors.element_hover))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.test_connection(cx);
                            }))
                    })
                    .when(is_loading, |el| el.opacity(0.5).cursor_not_allowed())
                    .when(is_testing, |el| el.child(Spinner::new().size(SpinnerSize::Small)))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(theme.colors.text)
                            .child(if is_testing {
                                "Testing..."
                            } else {
                                "Test Connection"
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .child(
                        // Cancel button
                        div()
                            .id("cancel-button")
                            .px(px(16.0))
                            .py(px(8.0))
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(theme.colors.border)
                            .hover(|s| s.bg(theme.colors.element_hover))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cancel(cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(theme.colors.text)
                                    .child("Cancel"),
                            ),
                    )
                    .child(
                        // Connect button (T042)
                        div()
                            .id("connect-button")
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .px(px(16.0))
                            .py(px(8.0))
                            .rounded(px(4.0))
                            .bg(theme.colors.accent)
                            .when(!is_loading, |el| {
                                el.hover(|s| s.bg(theme.colors.accent_hover))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.connect(cx);
                                    }))
                            })
                            .when(is_loading, |el| el.opacity(0.7).cursor_not_allowed())
                            .when(is_connecting, |el| {
                                el.child(Spinner::new().size(SpinnerSize::Small))
                            })
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(theme.colors.on_accent)
                                    .child(if is_connecting {
                                        "Connecting..."
                                    } else {
                                        "Connect"
                                    }),
                            ),
                    ),
            )
    }
}

impl Focusable for ConnectionDialog {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl gpui::EventEmitter<ConnectionDialogEvent> for ConnectionDialog {}

impl Render for ConnectionDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>().clone();
        let has_error = self.state.has_error();
        let has_success = self.state.is_test_success();
        let error_element = self.render_error(&theme);
        let success_element = self.render_success(&theme);
        let buttons_element = self.render_buttons(&theme, cx);

        div()
            .id("connection-dialog")
            .key_context("ConnectionDialog")
            .track_focus(&self.focus_handle)
            .capture_action(cx.listener(Self::on_tab))
            .capture_action(cx.listener(Self::on_tab_prev))
            .w(px(420.0))
            .flex()
            .flex_col()
            .bg(theme.colors.panel_background)
            .rounded(px(8.0))
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            // Header
            .child(
                div()
                    .px(px(20.0))
                    .py(px(16.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .text_size(px(16.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text)
                            .child("Connect to Database"),
                    ),
            )
            // Form fields (T040)
            .child(
                div()
                    .px(px(20.0))
                    .py(px(16.0))
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    // Host and Port row
                    .child(
                        div()
                            .flex()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .flex_1()
                                    .child(self.render_field("Host", self.host_input.clone(), &theme)),
                            )
                            .child(
                                div()
                                    .w(px(100.0))
                                    .child(self.render_field("Port", self.port_input.clone(), &theme)),
                            ),
                    )
                    // Database
                    .child(self.render_field("Database", self.database_input.clone(), &theme))
                    // Username
                    .child(self.render_field("Username", self.username_input.clone(), &theme))
                    // Password
                    .child(self.render_field("Password", self.password_input.clone(), &theme))
                    // SSL Mode (T041)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("SSL Mode"),
                            )
                            .child(self.ssl_mode_select.clone()),
                    )
                    // Error display (T045)
                    .when(has_error, |el| el.child(error_element))
                    // Success display for test connection
                    .when(has_success, |el| el.child(success_element)),
            )
            // Footer with buttons
            .child(
                div()
                    .px(px(20.0))
                    .py(px(16.0))
                    .border_t_1()
                    .border_color(theme.colors.border)
                    .child(buttons_element),
            )
    }
}
