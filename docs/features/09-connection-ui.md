# Feature 09: Connection UI

## Overview

Implement the connection user interface using GPUI including the connection dialog for creating/editing connections, the connection tree in the sidebar, visual status indicators, connection groups/folders, drag-drop reordering, and context menus.

## Goals

- Full connection dialog with all fields from design doc
- Connection tree with groups and status indicators
- Drag-drop for reordering connections and groups
- Context menu for connection actions
- Visual feedback for connection status
- Keyboard navigation throughout

## Dependencies

- 06-settings-theming-credentials.md (Theme system, KeyringService)
- 07-connection-management.md (ConnectionService, ConnectionConfig)
- 08-ssl-ssh-security.md (SSL/SSH configuration panels)

## Technical Specification

### 1. Connection Dialog Data Model

```rust
// src/ui/dialogs/connection_dialog.rs

use gpui::*;
use uuid::Uuid;
use crate::services::connection::{ConnectionConfig, ConnectionOptions, SslMode};
use crate::services::ssh::{SshTunnelConfig, SshAuthMethod};
use crate::state::TuskState;

/// Active tab in the connection dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionDialogTab {
    #[default]
    General,
    Ssl,
    Ssh,
    Options,
}

impl ConnectionDialogTab {
    pub fn label(&self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Ssl => "SSL",
            Self::Ssh => "SSH Tunnel",
            Self::Options => "Options",
        }
    }

    pub fn all() -> &'static [ConnectionDialogTab] {
        &[
            ConnectionDialogTab::General,
            ConnectionDialogTab::Ssl,
            ConnectionDialogTab::Ssh,
            ConnectionDialogTab::Options,
        ]
    }
}

/// Test connection result
#[derive(Debug, Clone)]
pub struct TestConnectionResult {
    pub success: bool,
    pub message: String,
    pub server_version: Option<String>,
    pub latency_ms: Option<u64>,
}

/// State for the connection dialog
pub struct ConnectionDialog {
    /// Whether this is editing an existing connection (vs creating new)
    editing_id: Option<Uuid>,

    /// Current active tab
    active_tab: ConnectionDialogTab,

    /// Form state - General tab
    name: String,
    color: ConnectionColor,
    group_id: Option<Uuid>,
    host: String,
    port: String, // String for text input, parsed on save
    database: String,
    username: String,
    password: String, // Not persisted in config, stored in keyring

    /// Form state - SSL tab
    ssl_mode: SslMode,
    ssl_ca_cert: String,
    ssl_client_cert: String,
    ssl_client_key: String,

    /// Form state - SSH tab
    ssh_enabled: bool,
    ssh_host: String,
    ssh_port: String,
    ssh_username: String,
    ssh_auth_method: SshAuthMethod,
    ssh_key_path: String,
    ssh_password: String,
    ssh_passphrase: String,

    /// Form state - Options tab
    connect_timeout_sec: String,
    statement_timeout_ms: String,
    application_name: String,
    readonly: bool,

    /// UI state
    is_testing: bool,
    is_saving: bool,
    test_result: Option<TestConnectionResult>,
    validation_errors: Vec<ValidationError>,

    /// Callbacks
    on_save: Option<Box<dyn Fn(ConnectionConfig) + Send + Sync>>,
    on_close: Option<Box<dyn Fn() + Send + Sync>>,
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// Connection color (preset or custom)
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionColor(pub String);

impl Default for ConnectionColor {
    fn default() -> Self {
        Self("#3b82f6".to_string()) // Blue
    }
}

impl ConnectionColor {
    pub fn presets() -> &'static [(&'static str, &'static str)] {
        &[
            ("#ef4444", "Red"),
            ("#f97316", "Orange"),
            ("#eab308", "Yellow"),
            ("#22c55e", "Green"),
            ("#06b6d4", "Cyan"),
            ("#3b82f6", "Blue"),
            ("#8b5cf6", "Purple"),
            ("#ec4899", "Pink"),
            ("#6b7280", "Gray"),
        ]
    }

    pub fn to_hsla(&self) -> Hsla {
        // Parse hex color to HSLA
        parse_hex_color(&self.0).unwrap_or(hsla(0.6, 0.8, 0.5, 1.0))
    }
}

fn parse_hex_color(hex: &str) -> Option<Hsla> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;

    // Convert RGB to HSL
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if max == min {
        return Some(hsla(0.0, 0.0, l, 1.0));
    }

    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };

    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } / 6.0;

    Some(hsla(h, s, l, 1.0))
}

impl ConnectionDialog {
    /// Create a new dialog for creating a connection
    pub fn new() -> Self {
        Self {
            editing_id: None,
            active_tab: ConnectionDialogTab::General,

            // General defaults
            name: String::new(),
            color: ConnectionColor::default(),
            group_id: None,
            host: "localhost".to_string(),
            port: "5432".to_string(),
            database: "postgres".to_string(),
            username: "postgres".to_string(),
            password: String::new(),

            // SSL defaults
            ssl_mode: SslMode::Prefer,
            ssl_ca_cert: String::new(),
            ssl_client_cert: String::new(),
            ssl_client_key: String::new(),

            // SSH defaults
            ssh_enabled: false,
            ssh_host: String::new(),
            ssh_port: "22".to_string(),
            ssh_username: String::new(),
            ssh_auth_method: SshAuthMethod::Key,
            ssh_key_path: "~/.ssh/id_rsa".to_string(),
            ssh_password: String::new(),
            ssh_passphrase: String::new(),

            // Options defaults
            connect_timeout_sec: "10".to_string(),
            statement_timeout_ms: String::new(),
            application_name: "Tusk".to_string(),
            readonly: false,

            // UI state
            is_testing: false,
            is_saving: false,
            test_result: None,
            validation_errors: Vec::new(),

            on_save: None,
            on_close: None,
        }
    }

    /// Create dialog for editing an existing connection
    pub fn edit(config: &ConnectionConfig) -> Self {
        let mut dialog = Self::new();

        dialog.editing_id = Some(config.id);
        dialog.name = config.name.clone();
        dialog.color = ConnectionColor(config.color.clone().unwrap_or_default());
        dialog.group_id = config.group_id;
        dialog.host = config.host.clone();
        dialog.port = config.port.to_string();
        dialog.database = config.database.clone();
        dialog.username = config.username.clone();
        // Password not loaded - user must re-enter or leave blank to keep existing

        dialog.ssl_mode = config.ssl_mode;
        dialog.ssl_ca_cert = config.ssl_ca_cert.clone().unwrap_or_default();
        dialog.ssl_client_cert = config.ssl_client_cert.clone().unwrap_or_default();
        dialog.ssl_client_key = config.ssl_client_key.clone().unwrap_or_default();

        if let Some(ref ssh) = config.ssh_tunnel {
            dialog.ssh_enabled = true;
            dialog.ssh_host = ssh.host.clone();
            dialog.ssh_port = ssh.port.to_string();
            dialog.ssh_username = ssh.username.clone();
            dialog.ssh_auth_method = ssh.auth_method;
            dialog.ssh_key_path = ssh.key_path.clone().unwrap_or_default();
        }

        dialog.connect_timeout_sec = config.options.connect_timeout_sec.to_string();
        dialog.statement_timeout_ms = config.options.statement_timeout_ms
            .map(|ms| ms.to_string())
            .unwrap_or_default();
        dialog.application_name = config.options.application_name.clone();
        dialog.readonly = config.options.readonly;

        dialog
    }

    pub fn on_save(mut self, callback: impl Fn(ConnectionConfig) + Send + Sync + 'static) -> Self {
        self.on_save = Some(Box::new(callback));
        self
    }

    pub fn on_close(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_close = Some(Box::new(callback));
        self
    }

    /// Validate all fields and return errors
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        if self.name.trim().is_empty() {
            errors.push(ValidationError {
                field: "name".to_string(),
                message: "Connection name is required".to_string(),
            });
        }

        if self.host.trim().is_empty() {
            errors.push(ValidationError {
                field: "host".to_string(),
                message: "Host is required".to_string(),
            });
        }

        if self.port.parse::<u16>().is_err() {
            errors.push(ValidationError {
                field: "port".to_string(),
                message: "Port must be a valid number (1-65535)".to_string(),
            });
        }

        if self.database.trim().is_empty() {
            errors.push(ValidationError {
                field: "database".to_string(),
                message: "Database name is required".to_string(),
            });
        }

        if self.username.trim().is_empty() {
            errors.push(ValidationError {
                field: "username".to_string(),
                message: "Username is required".to_string(),
            });
        }

        // SSL validation
        if matches!(self.ssl_mode, SslMode::VerifyCa | SslMode::VerifyFull) {
            if self.ssl_ca_cert.trim().is_empty() {
                errors.push(ValidationError {
                    field: "ssl_ca_cert".to_string(),
                    message: "CA certificate is required for this SSL mode".to_string(),
                });
            }
        }

        // SSH validation
        if self.ssh_enabled {
            if self.ssh_host.trim().is_empty() {
                errors.push(ValidationError {
                    field: "ssh_host".to_string(),
                    message: "SSH host is required".to_string(),
                });
            }
            if self.ssh_username.trim().is_empty() {
                errors.push(ValidationError {
                    field: "ssh_username".to_string(),
                    message: "SSH username is required".to_string(),
                });
            }
            if self.ssh_auth_method == SshAuthMethod::Key && self.ssh_key_path.trim().is_empty() {
                errors.push(ValidationError {
                    field: "ssh_key_path".to_string(),
                    message: "SSH key path is required".to_string(),
                });
            }
        }

        errors
    }

    /// Build ConnectionConfig from form state
    fn build_config(&self) -> Option<ConnectionConfig> {
        let port = self.port.parse().ok()?;
        let ssh_port = self.ssh_port.parse().unwrap_or(22);
        let connect_timeout = self.connect_timeout_sec.parse().unwrap_or(10);
        let statement_timeout = self.statement_timeout_ms.parse().ok();

        let ssh_tunnel = if self.ssh_enabled {
            Some(SshTunnelConfig {
                host: self.ssh_host.clone(),
                port: ssh_port,
                username: self.ssh_username.clone(),
                auth_method: self.ssh_auth_method,
                key_path: if self.ssh_key_path.is_empty() {
                    None
                } else {
                    Some(self.ssh_key_path.clone())
                },
            })
        } else {
            None
        };

        Some(ConnectionConfig {
            id: self.editing_id.unwrap_or_else(Uuid::new_v4),
            name: self.name.clone(),
            color: Some(self.color.0.clone()),
            group_id: self.group_id,
            host: self.host.clone(),
            port,
            database: self.database.clone(),
            username: self.username.clone(),
            password_in_keyring: true,
            ssl_mode: self.ssl_mode,
            ssl_ca_cert: if self.ssl_ca_cert.is_empty() {
                None
            } else {
                Some(self.ssl_ca_cert.clone())
            },
            ssl_client_cert: if self.ssl_client_cert.is_empty() {
                None
            } else {
                Some(self.ssl_client_cert.clone())
            },
            ssl_client_key: if self.ssl_client_key.is_empty() {
                None
            } else {
                Some(self.ssl_client_key.clone())
            },
            ssh_tunnel,
            options: ConnectionOptions {
                connect_timeout_sec: connect_timeout,
                statement_timeout_ms: statement_timeout,
                application_name: self.application_name.clone(),
                readonly: self.readonly,
            },
        })
    }
}
```

### 2. Connection Dialog UI Component

```rust
// src/ui/dialogs/connection_dialog.rs (continued)

impl Render for ConnectionDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();

        // Modal overlay
        div()
            .id("connection-dialog-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(hsla(0.0, 0.0, 0.0, 0.5))
            .on_click(cx.listener(|this, _, cx| {
                if let Some(on_close) = &this.on_close {
                    on_close();
                }
            }))
            .child(self.render_dialog(cx))
    }
}

impl ConnectionDialog {
    fn render_dialog(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();
        let title = if self.editing_id.is_some() {
            "Edit Connection"
        } else {
            "New Connection"
        };

        div()
            .id("connection-dialog")
            .w(px(640.0))
            .max_h(px(600.0))
            .bg(theme.colors.surface)
            .border_1()
            .border_color(theme.colors.border)
            .rounded_lg()
            .shadow_xl()
            .flex()
            .flex_col()
            .on_click(|_, _| {}) // Prevent click through to overlay
            .child(
                // Header
                div()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text)
                            .child(title)
                    )
                    .child(
                        self.render_close_button(cx)
                    )
            )
            .child(
                // Content area with tabs
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    .child(self.render_tab_sidebar(cx))
                    .child(self.render_tab_content(cx))
            )
            .child(self.render_test_result(cx))
            .child(self.render_footer(cx))
    }

    fn render_close_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();

        div()
            .id("close-btn")
            .w_6()
            .h_6()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.0))
            .cursor_pointer()
            .hover(|s| s.bg(theme.colors.hover))
            .text_color(theme.colors.text_muted)
            .child("×")
            .on_click(cx.listener(|this, _, _| {
                if let Some(on_close) = &this.on_close {
                    on_close();
                }
            }))
    }

    fn render_tab_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();

        div()
            .w(px(140.0))
            .border_r_1()
            .border_color(theme.colors.border)
            .p_2()
            .flex()
            .flex_col()
            .gap_1()
            .children(
                ConnectionDialogTab::all().iter().map(|&tab| {
                    self.render_tab_button(tab, cx)
                })
            )
    }

    fn render_tab_button(&self, tab: ConnectionDialogTab, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();
        let is_active = self.active_tab == tab;

        div()
            .id(SharedString::from(format!("tab-{:?}", tab)))
            .px_3()
            .py_2()
            .text_sm()
            .rounded(px(4.0))
            .cursor_pointer()
            .when(is_active, |s| {
                s.bg(theme.colors.accent.opacity(0.15))
                    .text_color(theme.colors.accent)
            })
            .when(!is_active, |s| {
                s.text_color(theme.colors.text)
                    .hover(|s| s.bg(theme.colors.hover))
            })
            .child(tab.label())
            .on_click(cx.listener(move |this, _, cx| {
                this.active_tab = tab;
                cx.notify();
            }))
    }

    fn render_tab_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();

        div()
            .flex_1()
            .p_4()
            .overflow_y_auto()
            .child(match self.active_tab {
                ConnectionDialogTab::General => self.render_general_tab(cx).into_any_element(),
                ConnectionDialogTab::Ssl => self.render_ssl_tab(cx).into_any_element(),
                ConnectionDialogTab::Ssh => self.render_ssh_tab(cx).into_any_element(),
                ConnectionDialogTab::Options => self.render_options_tab(cx).into_any_element(),
            })
    }

    fn render_general_tab(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();

        div()
            .flex()
            .flex_col()
            .gap_4()
            // Row 1: Name and Color
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(
                        self.render_form_field("Connection Name", true, cx)
                            .flex_1()
                            .child(self.render_text_input("name", &self.name, "My Database", cx))
                    )
                    .child(
                        self.render_form_field("Color", false, cx)
                            .child(self.render_color_picker(cx))
                    )
            )
            // Row 2: Host and Port
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(
                        self.render_form_field("Host", true, cx)
                            .flex_1()
                            .child(self.render_text_input("host", &self.host, "localhost", cx))
                    )
                    .child(
                        self.render_form_field("Port", true, cx)
                            .w(px(100.0))
                            .child(self.render_text_input("port", &self.port, "5432", cx))
                    )
            )
            // Row 3: Database
            .child(
                self.render_form_field("Database", true, cx)
                    .child(self.render_text_input("database", &self.database, "postgres", cx))
            )
            // Row 4: Username
            .child(
                self.render_form_field("Username", true, cx)
                    .child(self.render_text_input("username", &self.username, "postgres", cx))
            )
            // Row 5: Password
            .child(
                self.render_form_field("Password", false, cx)
                    .child(self.render_password_input("password", &self.password, cx))
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.colors.text_muted)
                            .mt_1()
                            .child("Stored securely in your system keychain")
                    )
            )
    }

    fn render_ssl_tab(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();
        let needs_ca = matches!(self.ssl_mode, SslMode::VerifyCa | SslMode::VerifyFull);

        div()
            .flex()
            .flex_col()
            .gap_4()
            // SSL Mode selector
            .child(
                self.render_form_field("SSL Mode", false, cx)
                    .child(self.render_ssl_mode_select(cx))
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.colors.text_muted)
                            .mt_1()
                            .child(self.ssl_mode_description())
                    )
            )
            // CA Certificate (required for verify modes)
            .when(needs_ca, |s| {
                s.child(
                    self.render_form_field("CA Certificate", true, cx)
                        .child(self.render_file_input("ssl_ca_cert", &self.ssl_ca_cert, "/path/to/ca.crt", cx))
                )
            })
            // Client Certificate (optional)
            .child(
                self.render_form_field("Client Certificate", false, cx)
                    .child(self.render_file_input("ssl_client_cert", &self.ssl_client_cert, "/path/to/client.crt", cx))
            )
            // Client Key (optional)
            .child(
                self.render_form_field("Client Key", false, cx)
                    .child(self.render_file_input("ssl_client_key", &self.ssl_client_key, "/path/to/client.key", cx))
            )
    }

    fn ssl_mode_description(&self) -> &'static str {
        match self.ssl_mode {
            SslMode::Disable => "No SSL encryption",
            SslMode::Prefer => "Use SSL if server supports it",
            SslMode::Require => "Require SSL, skip certificate verification",
            SslMode::VerifyCa => "Verify server certificate against CA",
            SslMode::VerifyFull => "Verify certificate and hostname",
        }
    }

    fn render_ssh_tab(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();

        div()
            .flex()
            .flex_col()
            .gap_4()
            // Enable SSH toggle
            .child(self.render_checkbox("ssh_enabled", "Enable SSH Tunnel", self.ssh_enabled, cx))
            // SSH fields (only when enabled)
            .when(self.ssh_enabled, |s| {
                s.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_4()
                        // Host and Port
                        .child(
                            div()
                                .flex()
                                .gap_4()
                                .child(
                                    self.render_form_field("SSH Host", true, cx)
                                        .flex_1()
                                        .child(self.render_text_input("ssh_host", &self.ssh_host, "ssh.example.com", cx))
                                )
                                .child(
                                    self.render_form_field("SSH Port", false, cx)
                                        .w(px(100.0))
                                        .child(self.render_text_input("ssh_port", &self.ssh_port, "22", cx))
                                )
                        )
                        // Username
                        .child(
                            self.render_form_field("SSH Username", true, cx)
                                .child(self.render_text_input("ssh_username", &self.ssh_username, "", cx))
                        )
                        // Auth method
                        .child(
                            self.render_form_field("Authentication", false, cx)
                                .child(self.render_ssh_auth_select(cx))
                        )
                        // Auth-specific fields
                        .child(self.render_ssh_auth_fields(cx))
                )
            })
    }

    fn render_ssh_auth_fields(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.ssh_auth_method {
            SshAuthMethod::Password => {
                self.render_form_field("SSH Password", false, cx)
                    .child(self.render_password_input("ssh_password", &self.ssh_password, cx))
                    .into_any_element()
            }
            SshAuthMethod::Key | SshAuthMethod::Agent => {
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .when(self.ssh_auth_method == SshAuthMethod::Key, |s| {
                        s.child(
                            self.render_form_field("SSH Key File", true, cx)
                                .child(self.render_file_input("ssh_key_path", &self.ssh_key_path, "~/.ssh/id_rsa", cx))
                        )
                    })
                    .child(
                        self.render_form_field("Key Passphrase", false, cx)
                            .child(self.render_password_input("ssh_passphrase", &self.ssh_passphrase, cx))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.global::<ThemeSettings>().colors.text_muted)
                                    .mt_1()
                                    .child("Leave empty if key is not encrypted")
                            )
                    )
                    .into_any_element()
            }
        }
    }

    fn render_options_tab(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();

        div()
            .flex()
            .flex_col()
            .gap_4()
            // Connection timeout
            .child(
                self.render_form_field("Connection Timeout (seconds)", false, cx)
                    .child(self.render_text_input("connect_timeout_sec", &self.connect_timeout_sec, "10", cx))
            )
            // Statement timeout
            .child(
                self.render_form_field("Statement Timeout (milliseconds)", false, cx)
                    .child(self.render_text_input("statement_timeout_ms", &self.statement_timeout_ms, "No timeout", cx))
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.colors.text_muted)
                            .mt_1()
                            .child("Maximum time for queries to run. Leave empty for no timeout.")
                    )
            )
            // Application name
            .child(
                self.render_form_field("Application Name", false, cx)
                    .child(self.render_text_input("application_name", &self.application_name, "Tusk", cx))
            )
            // Read-only mode
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(self.render_checkbox("readonly", "Read-only mode", self.readonly, cx))
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.colors.text_muted)
                            .ml_6()
                            .child("Prevents INSERT, UPDATE, DELETE, and DDL statements")
                    )
            )
    }

    fn render_test_result(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();

        if let Some(ref result) = self.test_result {
            let (bg_color, text_color) = if result.success {
                (hsla(0.35, 0.8, 0.4, 0.15), hsla(0.35, 0.8, 0.3, 1.0))
            } else {
                (hsla(0.0, 0.8, 0.5, 0.15), hsla(0.0, 0.8, 0.4, 1.0))
            };

            div()
                .mx_4()
                .mb_4()
                .p_3()
                .rounded(px(4.0))
                .bg(bg_color)
                .child(
                    div()
                        .text_sm()
                        .font_family("monospace")
                        .text_color(text_color)
                        .child(&result.message)
                )
        } else {
            div()
        }
    }

    fn render_footer(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();
        let is_valid = self.validate().is_empty();

        div()
            .px_4()
            .py_3()
            .border_t_1()
            .border_color(theme.colors.border)
            .flex()
            .items_center()
            .justify_between()
            // Test button on left
            .child(
                self.render_button(
                    if self.is_testing { "Testing..." } else { "Test Connection" },
                    ButtonVariant::Secondary,
                    !is_valid || self.is_testing,
                    cx.listener(|this, _, cx| {
                        this.handle_test(cx);
                    }),
                    cx,
                )
            )
            // Cancel and Save buttons on right
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        self.render_button(
                            "Cancel",
                            ButtonVariant::Secondary,
                            false,
                            cx.listener(|this, _, _| {
                                if let Some(on_close) = &this.on_close {
                                    on_close();
                                }
                            }),
                            cx,
                        )
                    )
                    .child(
                        self.render_button(
                            if self.is_saving { "Saving..." } else { "Save" },
                            ButtonVariant::Primary,
                            !is_valid || self.is_saving,
                            cx.listener(|this, _, cx| {
                                this.handle_save(cx);
                            }),
                            cx,
                        )
                    )
            )
    }
}
```

### 3. Form Input Components

```rust
// src/ui/dialogs/connection_dialog.rs (continued)

#[derive(Debug, Clone, Copy, PartialEq)]
enum ButtonVariant {
    Primary,
    Secondary,
}

impl ConnectionDialog {
    fn render_form_field(
        &self,
        label: &str,
        required: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let theme = cx.global::<ThemeSettings>();

        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.colors.text)
                            .child(label)
                    )
                    .when(required, |s| {
                        s.child(
                            div()
                                .text_sm()
                                .text_color(hsla(0.0, 0.8, 0.5, 1.0))
                                .child("*")
                        )
                    })
            )
    }

    fn render_text_input(
        &self,
        field: &str,
        value: &str,
        placeholder: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();
        let field_owned = field.to_string();
        let value_owned = value.to_string();

        // Check for validation error on this field
        let has_error = self.validation_errors.iter().any(|e| e.field == field);

        div()
            .w_full()
            .h_8()
            .px_2()
            .bg(theme.colors.input_background)
            .border_1()
            .when(has_error, |s| s.border_color(hsla(0.0, 0.8, 0.5, 1.0)))
            .when(!has_error, |s| s.border_color(theme.colors.border))
            .rounded(px(4.0))
            .flex()
            .items_center()
            .child(
                // Using GPUI's TextInput would go here
                // For now, showing the structure
                div()
                    .flex_1()
                    .text_sm()
                    .when(value.is_empty(), |s| s.text_color(theme.colors.text_muted))
                    .when(!value.is_empty(), |s| s.text_color(theme.colors.text))
                    .child(if value.is_empty() { placeholder } else { value })
            )
    }

    fn render_password_input(
        &self,
        field: &str,
        value: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();
        let placeholder = if self.editing_id.is_some() && field == "password" {
            "(unchanged)"
        } else {
            ""
        };
        let display_value = if value.is_empty() {
            placeholder.to_string()
        } else {
            "•".repeat(value.len())
        };

        div()
            .w_full()
            .h_8()
            .px_2()
            .bg(theme.colors.input_background)
            .border_1()
            .border_color(theme.colors.border)
            .rounded(px(4.0))
            .flex()
            .items_center()
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .when(value.is_empty(), |s| s.text_color(theme.colors.text_muted))
                    .when(!value.is_empty(), |s| s.text_color(theme.colors.text))
                    .child(display_value)
            )
    }

    fn render_file_input(
        &self,
        field: &str,
        value: &str,
        placeholder: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();
        let field_owned = field.to_string();

        div()
            .flex()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .h_8()
                    .px_2()
                    .bg(theme.colors.input_background)
                    .border_1()
                    .border_color(theme.colors.border)
                    .rounded(px(4.0))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .truncate()
                            .when(value.is_empty(), |s| s.text_color(theme.colors.text_muted))
                            .when(!value.is_empty(), |s| s.text_color(theme.colors.text))
                            .child(if value.is_empty() { placeholder } else { value })
                    )
            )
            .child(
                self.render_button(
                    "Browse",
                    ButtonVariant::Secondary,
                    false,
                    cx.listener(move |this, _, cx| {
                        this.handle_browse_file(&field_owned, cx);
                    }),
                    cx,
                )
            )
    }

    fn render_checkbox(
        &self,
        field: &str,
        label: &str,
        checked: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();
        let field_owned = field.to_string();

        div()
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, cx| {
                this.handle_checkbox_toggle(&field_owned, cx);
            }))
            .child(
                div()
                    .w_4()
                    .h_4()
                    .border_1()
                    .border_color(theme.colors.border)
                    .rounded(px(2.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(checked, |s| s.bg(theme.colors.accent))
                    .when(checked, |s| s.child(
                        div()
                            .text_xs()
                            .text_color(white())
                            .child("✓")
                    ))
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.colors.text)
                    .child(label)
            )
    }

    fn render_ssl_mode_select(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();
        let modes = [
            (SslMode::Disable, "Disable"),
            (SslMode::Prefer, "Prefer"),
            (SslMode::Require, "Require"),
            (SslMode::VerifyCa, "Verify CA"),
            (SslMode::VerifyFull, "Verify Full"),
        ];

        div()
            .flex()
            .gap_1()
            .children(modes.iter().map(|(mode, label)| {
                let is_selected = self.ssl_mode == *mode;
                let mode_copy = *mode;

                div()
                    .px_3()
                    .py_1()
                    .text_sm()
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .when(is_selected, |s| {
                        s.bg(theme.colors.accent)
                            .text_color(white())
                    })
                    .when(!is_selected, |s| {
                        s.bg(theme.colors.input_background)
                            .text_color(theme.colors.text)
                            .hover(|s| s.bg(theme.colors.hover))
                    })
                    .child(*label)
                    .on_click(cx.listener(move |this, _, cx| {
                        this.ssl_mode = mode_copy;
                        cx.notify();
                    }))
            }))
    }

    fn render_ssh_auth_select(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();
        let methods = [
            (SshAuthMethod::Key, "SSH Key"),
            (SshAuthMethod::Password, "Password"),
            (SshAuthMethod::Agent, "SSH Agent"),
        ];

        div()
            .flex()
            .gap_1()
            .children(methods.iter().map(|(method, label)| {
                let is_selected = self.ssh_auth_method == *method;
                let method_copy = *method;

                div()
                    .px_3()
                    .py_1()
                    .text_sm()
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .when(is_selected, |s| {
                        s.bg(theme.colors.accent)
                            .text_color(white())
                    })
                    .when(!is_selected, |s| {
                        s.bg(theme.colors.input_background)
                            .text_color(theme.colors.text)
                            .hover(|s| s.bg(theme.colors.hover))
                    })
                    .child(*label)
                    .on_click(cx.listener(move |this, _, cx| {
                        this.ssh_auth_method = method_copy;
                        cx.notify();
                    }))
            }))
    }

    fn render_color_picker(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();

        div()
            .relative()
            .child(
                // Color button that shows current selection
                div()
                    .id("color-picker-btn")
                    .w_8()
                    .h_8()
                    .rounded(px(4.0))
                    .border_2()
                    .border_color(theme.colors.border)
                    .cursor_pointer()
                    .bg(self.color.to_hsla())
            )
            // Dropdown would be rendered here when clicked
    }

    fn render_button<F>(
        &self,
        label: &str,
        variant: ButtonVariant,
        disabled: bool,
        on_click: F,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        F: Fn(&ClickEvent, &mut Window, &mut Context<Self>) + 'static,
    {
        let theme = cx.global::<ThemeSettings>();

        let (bg, text_color, hover_bg) = match variant {
            ButtonVariant::Primary => (
                theme.colors.accent,
                white(),
                theme.colors.accent.opacity(0.9),
            ),
            ButtonVariant::Secondary => (
                theme.colors.input_background,
                theme.colors.text,
                theme.colors.hover,
            ),
        };

        div()
            .px_4()
            .py_2()
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .rounded(px(4.0))
            .when(disabled, |s| {
                s.opacity(0.5)
                    .cursor_not_allowed()
            })
            .when(!disabled, |s| {
                s.cursor_pointer()
                    .bg(bg)
                    .text_color(text_color)
                    .hover(|s| s.bg(hover_bg))
            })
            .child(label)
            .when(!disabled, |s| s.on_click(on_click))
    }
}
```

### 4. Dialog Event Handlers

```rust
// src/ui/dialogs/connection_dialog.rs (continued)

impl ConnectionDialog {
    fn handle_test(&mut self, cx: &mut Context<Self>) {
        self.is_testing = true;
        self.test_result = None;
        cx.notify();

        // Build config and test
        if let Some(config) = self.build_config() {
            let tusk_state = cx.global::<TuskState>();
            let connection_service = tusk_state.connection_service.clone();
            let password = self.password.clone();
            let ssh_password = self.ssh_password.clone();
            let ssh_passphrase = self.ssh_passphrase.clone();

            cx.spawn(|this, mut cx| async move {
                // Store temporary credentials for test
                let keyring = connection_service.keyring();

                if !password.is_empty() {
                    let _ = keyring.store_password(&config.id, &password);
                }
                if !ssh_password.is_empty() {
                    let _ = keyring.store_ssh_password(&config.id, &ssh_password);
                }
                if !ssh_passphrase.is_empty() {
                    let _ = keyring.store_ssh_passphrase(&config.id, &ssh_passphrase);
                }

                // Test connection
                let result = connection_service.test_connection(&config);

                // Clean up temporary credentials if this is a new connection
                // (If editing, leave them for potential save)

                this.update(&mut cx, |this, cx| {
                    this.is_testing = false;
                    this.test_result = Some(match result {
                        Ok(info) => TestConnectionResult {
                            success: true,
                            message: format!(
                                "Connected to {}\nLatency: {}ms",
                                info.server_version,
                                info.latency_ms
                            ),
                            server_version: Some(info.server_version),
                            latency_ms: Some(info.latency_ms),
                        },
                        Err(e) => TestConnectionResult {
                            success: false,
                            message: e.to_string(),
                            server_version: None,
                            latency_ms: None,
                        },
                    });
                    cx.notify();
                }).ok();
            }).detach();
        }
    }

    fn handle_save(&mut self, cx: &mut Context<Self>) {
        // Validate
        self.validation_errors = self.validate();
        if !self.validation_errors.is_empty() {
            cx.notify();
            return;
        }

        self.is_saving = true;
        cx.notify();

        if let Some(config) = self.build_config() {
            let tusk_state = cx.global::<TuskState>();
            let connection_service = tusk_state.connection_service.clone();
            let storage_service = tusk_state.storage_service.clone();
            let password = self.password.clone();
            let ssh_password = self.ssh_password.clone();
            let ssh_passphrase = self.ssh_passphrase.clone();
            let is_editing = self.editing_id.is_some();
            let on_save = self.on_save.take();
            let on_close = self.on_close.take();

            cx.spawn(|this, mut cx| async move {
                let keyring = connection_service.keyring();

                // Store credentials in keyring
                if !password.is_empty() {
                    keyring.store_password(&config.id, &password)?;
                }
                if !ssh_password.is_empty() {
                    keyring.store_ssh_password(&config.id, &ssh_password)?;
                }
                if !ssh_passphrase.is_empty() {
                    keyring.store_ssh_passphrase(&config.id, &ssh_passphrase)?;
                }

                // Save to storage
                if is_editing {
                    storage_service.update_connection(&config)?;
                } else {
                    storage_service.insert_connection(&config)?;
                }

                this.update(&mut cx, |this, cx| {
                    this.is_saving = false;

                    // Trigger callbacks
                    if let Some(on_save) = &on_save {
                        on_save(config.clone());
                    }
                    if let Some(on_close) = &on_close {
                        on_close();
                    }

                    cx.notify();
                }).ok();

                Ok::<_, anyhow::Error>(())
            }).detach();
        }
    }

    fn handle_browse_file(&mut self, field: &str, cx: &mut Context<Self>) {
        use rfd::FileDialog;

        let (title, filters) = match field {
            "ssl_ca_cert" | "ssl_client_cert" => (
                "Select Certificate",
                &[("Certificates", &["crt", "pem", "cer"][..])][..],
            ),
            "ssl_client_key" => (
                "Select Private Key",
                &[("Keys", &["key", "pem"][..])][..],
            ),
            "ssh_key_path" => (
                "Select SSH Key",
                &[("SSH Keys", &["", "pem", "pub"][..])][..],
            ),
            _ => return,
        };

        let mut dialog = FileDialog::new().set_title(title);
        for (name, extensions) in filters {
            dialog = dialog.add_filter(*name, extensions);
        }

        // Default to ~/.ssh for SSH keys
        if field == "ssh_key_path" {
            if let Some(home) = dirs::home_dir() {
                dialog = dialog.set_directory(home.join(".ssh"));
            }
        }

        if let Some(path) = dialog.pick_file() {
            let path_str = path.to_string_lossy().to_string();
            match field {
                "ssl_ca_cert" => self.ssl_ca_cert = path_str,
                "ssl_client_cert" => self.ssl_client_cert = path_str,
                "ssl_client_key" => self.ssl_client_key = path_str,
                "ssh_key_path" => self.ssh_key_path = path_str,
                _ => {}
            }
            cx.notify();
        }
    }

    fn handle_checkbox_toggle(&mut self, field: &str, cx: &mut Context<Self>) {
        match field {
            "ssh_enabled" => self.ssh_enabled = !self.ssh_enabled,
            "readonly" => self.readonly = !self.readonly,
            _ => {}
        }
        cx.notify();
    }
}
```

### 5. Connection Tree Component

```rust
// src/ui/sidebar/connection_tree.rs

use gpui::*;
use uuid::Uuid;
use crate::services::connection::{ConnectionConfig, ConnectionStatus};
use crate::state::TuskState;

/// A connection with its current status
#[derive(Debug, Clone)]
pub struct ConnectionWithStatus {
    pub config: ConnectionConfig,
    pub status: ConnectionStatus,
}

/// A group/folder of connections
#[derive(Debug, Clone)]
pub struct ConnectionGroup {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub expanded: bool,
    pub order: i32,
}

/// Tree node representing either a connection or a group
#[derive(Debug, Clone)]
pub enum TreeNode {
    Connection(ConnectionWithStatus),
    Group {
        group: ConnectionGroup,
        children: Vec<TreeNode>,
    },
}

/// State for the connection tree
pub struct ConnectionTree {
    /// All connections
    connections: Vec<ConnectionWithStatus>,

    /// All groups
    groups: Vec<ConnectionGroup>,

    /// Currently active connection
    active_connection_id: Option<Uuid>,

    /// Filter text for searching
    filter: String,

    /// Context menu state
    context_menu: Option<ContextMenuState>,

    /// Drag state for reordering
    drag_state: Option<DragState>,

    /// Expanded group IDs
    expanded_groups: HashSet<Uuid>,
}

#[derive(Debug, Clone)]
struct ContextMenuState {
    position: Point<Pixels>,
    target: ContextMenuTarget,
}

#[derive(Debug, Clone)]
enum ContextMenuTarget {
    Connection(Uuid),
    Group(Uuid),
    Empty,
}

#[derive(Debug, Clone)]
struct DragState {
    dragging: DragItem,
    over: Option<DropTarget>,
}

#[derive(Debug, Clone)]
enum DragItem {
    Connection(Uuid),
    Group(Uuid),
}

#[derive(Debug, Clone)]
enum DropTarget {
    Connection(Uuid),
    Group(Uuid),
    Root,
}

use std::collections::HashSet;

impl ConnectionTree {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            groups: Vec::new(),
            active_connection_id: None,
            filter: String::new(),
            context_menu: None,
            drag_state: None,
            expanded_groups: HashSet::new(),
        }
    }

    pub fn set_connections(&mut self, connections: Vec<ConnectionWithStatus>) {
        self.connections = connections;
    }

    pub fn set_groups(&mut self, groups: Vec<ConnectionGroup>) {
        // Initialize expanded state from groups
        for group in &groups {
            if group.expanded {
                self.expanded_groups.insert(group.id);
            }
        }
        self.groups = groups;
    }

    pub fn set_active_connection(&mut self, id: Option<Uuid>) {
        self.active_connection_id = id;
    }

    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
    }

    /// Build the tree structure from flat data
    fn build_tree(&self) -> Vec<TreeNode> {
        let filter_lower = self.filter.to_lowercase();

        // Filter connections
        let filtered_connections: Vec<_> = if self.filter.is_empty() {
            self.connections.clone()
        } else {
            self.connections.iter()
                .filter(|c| {
                    c.config.name.to_lowercase().contains(&filter_lower) ||
                    c.config.host.to_lowercase().contains(&filter_lower)
                })
                .cloned()
                .collect()
        };

        // Build group hierarchy
        fn build_group_children(
            parent_id: Option<Uuid>,
            groups: &[ConnectionGroup],
            connections: &[ConnectionWithStatus],
            expanded: &HashSet<Uuid>,
        ) -> Vec<TreeNode> {
            let mut nodes = Vec::new();

            // Add child groups
            for group in groups.iter().filter(|g| g.parent_id == parent_id) {
                let children = build_group_children(
                    Some(group.id),
                    groups,
                    connections,
                    expanded,
                );
                nodes.push(TreeNode::Group {
                    group: ConnectionGroup {
                        expanded: expanded.contains(&group.id),
                        ..group.clone()
                    },
                    children,
                });
            }

            // Add connections in this group
            for conn in connections.iter().filter(|c| c.config.group_id == parent_id) {
                nodes.push(TreeNode::Connection(conn.clone()));
            }

            nodes
        }

        build_group_children(None, &self.groups, &filtered_connections, &self.expanded_groups)
    }

    fn toggle_group(&mut self, group_id: Uuid, cx: &mut Context<Self>) {
        if self.expanded_groups.contains(&group_id) {
            self.expanded_groups.remove(&group_id);
        } else {
            self.expanded_groups.insert(group_id);
        }

        // Persist expanded state
        let tusk_state = cx.global::<TuskState>();
        if let Err(e) = tusk_state.storage_service.update_group_expanded(group_id, self.expanded_groups.contains(&group_id)) {
            log::error!("Failed to save group expanded state: {}", e);
        }

        cx.notify();
    }

    fn handle_connection_click(&mut self, conn_id: Uuid, cx: &mut Context<Self>) {
        let conn = self.connections.iter().find(|c| c.config.id == conn_id);

        if let Some(conn) = conn {
            let tusk_state = cx.global::<TuskState>();

            match conn.status {
                ConnectionStatus::Connected => {
                    // Already connected, just set as active
                    self.active_connection_id = Some(conn_id);
                }
                ConnectionStatus::Disconnected | ConnectionStatus::Error(_) => {
                    // Connect
                    if let Err(e) = tusk_state.connection_service.connect(conn.config.clone()) {
                        log::error!("Failed to connect: {}", e);
                    } else {
                        self.active_connection_id = Some(conn_id);
                    }
                }
                ConnectionStatus::Connecting | ConnectionStatus::Reconnecting => {
                    // Already connecting, do nothing
                }
            }
        }

        cx.notify();
    }

    fn handle_connection_double_click(&mut self, conn_id: Uuid, cx: &mut Context<Self>) {
        // Open a new query tab for this connection
        cx.emit(ConnectionTreeEvent::OpenQueryTab(conn_id));
    }

    fn show_context_menu(&mut self, position: Point<Pixels>, target: ContextMenuTarget, cx: &mut Context<Self>) {
        self.context_menu = Some(ContextMenuState { position, target });
        cx.notify();
    }

    fn hide_context_menu(&mut self, cx: &mut Context<Self>) {
        self.context_menu = None;
        cx.notify();
    }
}

/// Events emitted by the connection tree
#[derive(Debug, Clone)]
pub enum ConnectionTreeEvent {
    OpenQueryTab(Uuid),
    EditConnection(ConnectionConfig),
    DeleteConnection(Uuid),
    CreateGroup,
    EditGroup(Uuid),
    DeleteGroup(Uuid),
}

impl EventEmitter<ConnectionTreeEvent> for ConnectionTree {}
```

### 6. Connection Tree Rendering

```rust
// src/ui/sidebar/connection_tree.rs (continued)

impl Render for ConnectionTree {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();
        let tree = self.build_tree();

        div()
            .id("connection-tree")
            .flex_1()
            .flex()
            .flex_col()
            .overflow_y_auto()
            // Click outside to close context menu
            .on_click(cx.listener(|this, _, cx| {
                this.hide_context_menu(cx);
            }))
            // Tree content
            .children(tree.iter().map(|node| {
                self.render_tree_node(node, 0, cx)
            }))
            // Empty state
            .when(tree.is_empty(), |s| {
                s.child(
                    div()
                        .p_4()
                        .text_center()
                        .text_sm()
                        .text_color(theme.colors.text_muted)
                        .child(
                            if self.filter.is_empty() {
                                "No connections yet"
                            } else {
                                "No connections match filter"
                            }
                        )
                )
            })
            // Context menu overlay
            .children(self.context_menu.as_ref().map(|menu| {
                self.render_context_menu(menu, cx)
            }))
    }
}

impl ConnectionTree {
    fn render_tree_node(
        &self,
        node: &TreeNode,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        match node {
            TreeNode::Connection(conn) => {
                self.render_connection_node(conn, depth, cx).into_any_element()
            }
            TreeNode::Group { group, children } => {
                self.render_group_node(group, children, depth, cx).into_any_element()
            }
        }
    }

    fn render_connection_node(
        &self,
        conn: &ConnectionWithStatus,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();
        let conn_id = conn.config.id;
        let is_active = self.active_connection_id == Some(conn_id);

        let status_color = match conn.status {
            ConnectionStatus::Disconnected => theme.colors.text_muted,
            ConnectionStatus::Connecting | ConnectionStatus::Reconnecting => {
                hsla(0.14, 0.9, 0.5, 1.0) // Yellow
            }
            ConnectionStatus::Connected => hsla(0.35, 0.8, 0.4, 1.0), // Green
            ConnectionStatus::Error(_) => hsla(0.0, 0.8, 0.5, 1.0), // Red
        };

        let conn_color = conn.config.color.as_ref()
            .and_then(|c| parse_hex_color(c))
            .unwrap_or(theme.colors.accent);

        div()
            .id(SharedString::from(format!("conn-{}", conn_id)))
            .pl(px((depth * 16 + 8) as f32))
            .pr_2()
            .py(px(6.0))
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .rounded(px(4.0))
            .when(is_active, |s| s.bg(theme.colors.accent.opacity(0.15)))
            .when(!is_active, |s| s.hover(|s| s.bg(theme.colors.hover)))
            // Color indicator
            .child(
                div()
                    .w(px(6.0))
                    .h(px(6.0))
                    .rounded_full()
                    .bg(conn_color)
                    .flex_shrink_0()
            )
            // Status indicator
            .child(
                div()
                    .w(px(8.0))
                    .h(px(8.0))
                    .rounded_full()
                    .bg(status_color)
                    .flex_shrink_0()
                    .when(
                        matches!(conn.status, ConnectionStatus::Connecting | ConnectionStatus::Reconnecting),
                        |s| s.with_animation(
                            "pulse",
                            Animation::new(Duration::from_millis(1000))
                                .repeat()
                                .with_easing(Easing::EaseInOut),
                            |s, progress| s.opacity(0.5 + 0.5 * (progress * std::f32::consts::PI * 2.0).sin())
                        )
                    )
            )
            // Database icon
            .child(
                Icon::new(IconName::Database)
                    .size(IconSize::Small)
                    .color(theme.colors.text_muted)
            )
            // Connection name
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(theme.colors.text)
                    .truncate()
                    .child(&conn.config.name)
            )
            // Event handlers
            .on_click(cx.listener(move |this, _, cx| {
                this.handle_connection_click(conn_id, cx);
            }))
            .on_double_click(cx.listener(move |this, _, cx| {
                this.handle_connection_double_click(conn_id, cx);
            }))
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &MouseDownEvent, cx| {
                this.show_context_menu(
                    event.position,
                    ContextMenuTarget::Connection(conn_id),
                    cx,
                );
            }))
    }

    fn render_group_node(
        &self,
        group: &ConnectionGroup,
        children: &[TreeNode],
        depth: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();
        let group_id = group.id;
        let is_expanded = self.expanded_groups.contains(&group_id);
        let child_count = count_connections(children);

        div()
            .flex()
            .flex_col()
            // Group header
            .child(
                div()
                    .id(SharedString::from(format!("group-{}", group_id)))
                    .pl(px((depth * 16 + 8) as f32))
                    .pr_2()
                    .py(px(6.0))
                    .flex()
                    .items_center()
                    .gap_1()
                    .cursor_pointer()
                    .rounded(px(4.0))
                    .hover(|s| s.bg(theme.colors.hover))
                    // Chevron
                    .child(
                        Icon::new(if is_expanded { IconName::ChevronDown } else { IconName::ChevronRight })
                            .size(IconSize::Small)
                            .color(theme.colors.text_muted)
                    )
                    // Folder icon
                    .child(
                        Icon::new(IconName::Folder)
                            .size(IconSize::Small)
                            .color(theme.colors.text_muted)
                    )
                    // Group name
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.colors.text)
                            .truncate()
                            .child(&group.name)
                    )
                    // Count badge
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.colors.text_muted)
                            .child(format!("({})", child_count))
                    )
                    .on_click(cx.listener(move |this, _, cx| {
                        this.toggle_group(group_id, cx);
                    }))
                    .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &MouseDownEvent, cx| {
                        this.show_context_menu(
                            event.position,
                            ContextMenuTarget::Group(group_id),
                            cx,
                        );
                    }))
            )
            // Children (when expanded)
            .when(is_expanded, |s| {
                s.children(children.iter().map(|child| {
                    self.render_tree_node(child, depth + 1, cx)
                }))
            })
    }

    fn render_context_menu(
        &self,
        menu: &ContextMenuState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();

        div()
            .absolute()
            .left(menu.position.x)
            .top(menu.position.y)
            .min_w(px(160.0))
            .py_1()
            .bg(theme.colors.surface)
            .border_1()
            .border_color(theme.colors.border)
            .rounded(px(4.0))
            .shadow_lg()
            .z_index(100)
            .on_click(|_, _| {}) // Prevent click through
            .children(match &menu.target {
                ContextMenuTarget::Connection(conn_id) => {
                    self.render_connection_context_menu(*conn_id, cx)
                }
                ContextMenuTarget::Group(group_id) => {
                    self.render_group_context_menu(*group_id, cx)
                }
                ContextMenuTarget::Empty => {
                    self.render_empty_context_menu(cx)
                }
            })
    }

    fn render_connection_context_menu(
        &self,
        conn_id: Uuid,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let conn = self.connections.iter().find(|c| c.config.id == conn_id);
        let is_connected = conn.map(|c| c.status == ConnectionStatus::Connected).unwrap_or(false);
        let config = conn.map(|c| c.config.clone());

        let mut items = Vec::new();

        // Connect/Disconnect
        if is_connected {
            items.push(self.render_menu_item(
                "Disconnect",
                IconName::PlugOff,
                cx.listener(move |this, _, cx| {
                    let tusk_state = cx.global::<TuskState>();
                    let _ = tusk_state.connection_service.disconnect(&conn_id);
                    this.hide_context_menu(cx);
                }),
                cx,
            ).into_any_element());
        } else {
            items.push(self.render_menu_item(
                "Connect",
                IconName::Plug,
                cx.listener(move |this, _, cx| {
                    this.handle_connection_click(conn_id, cx);
                    this.hide_context_menu(cx);
                }),
                cx,
            ).into_any_element());
        }

        items.push(self.render_menu_separator(cx).into_any_element());

        // New Query
        items.push(self.render_menu_item(
            "New Query",
            IconName::Code,
            cx.listener(move |this, _, cx| {
                cx.emit(ConnectionTreeEvent::OpenQueryTab(conn_id));
                this.hide_context_menu(cx);
            }),
            cx,
        ).into_any_element());

        items.push(self.render_menu_separator(cx).into_any_element());

        // Edit
        if let Some(config) = config.clone() {
            items.push(self.render_menu_item(
                "Edit",
                IconName::Edit,
                cx.listener(move |this, _, cx| {
                    cx.emit(ConnectionTreeEvent::EditConnection(config.clone()));
                    this.hide_context_menu(cx);
                }),
                cx,
            ).into_any_element());
        }

        // Duplicate
        if let Some(config) = config {
            items.push(self.render_menu_item(
                "Duplicate",
                IconName::Copy,
                cx.listener(move |this, _, cx| {
                    let mut new_config = config.clone();
                    new_config.id = Uuid::new_v4();
                    new_config.name = format!("{} (copy)", new_config.name);
                    cx.emit(ConnectionTreeEvent::EditConnection(new_config));
                    this.hide_context_menu(cx);
                }),
                cx,
            ).into_any_element());
        }

        items.push(self.render_menu_separator(cx).into_any_element());

        // Delete
        items.push(self.render_menu_item_danger(
            "Delete",
            IconName::Trash,
            cx.listener(move |this, _, cx| {
                cx.emit(ConnectionTreeEvent::DeleteConnection(conn_id));
                this.hide_context_menu(cx);
            }),
            cx,
        ).into_any_element());

        items
    }

    fn render_group_context_menu(
        &self,
        group_id: Uuid,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        vec![
            self.render_menu_item(
                "Rename",
                IconName::Edit,
                cx.listener(move |this, _, cx| {
                    cx.emit(ConnectionTreeEvent::EditGroup(group_id));
                    this.hide_context_menu(cx);
                }),
                cx,
            ).into_any_element(),
            self.render_menu_separator(cx).into_any_element(),
            self.render_menu_item_danger(
                "Delete Group",
                IconName::Trash,
                cx.listener(move |this, _, cx| {
                    cx.emit(ConnectionTreeEvent::DeleteGroup(group_id));
                    this.hide_context_menu(cx);
                }),
                cx,
            ).into_any_element(),
        ]
    }

    fn render_empty_context_menu(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        vec![
            self.render_menu_item(
                "New Connection",
                IconName::Plus,
                cx.listener(|this, _, cx| {
                    cx.emit(ConnectionTreeEvent::EditConnection(ConnectionConfig::default()));
                    this.hide_context_menu(cx);
                }),
                cx,
            ).into_any_element(),
            self.render_menu_item(
                "New Group",
                IconName::FolderPlus,
                cx.listener(|this, _, cx| {
                    cx.emit(ConnectionTreeEvent::CreateGroup);
                    this.hide_context_menu(cx);
                }),
                cx,
            ).into_any_element(),
        ]
    }

    fn render_menu_item<F>(
        &self,
        label: &str,
        icon: IconName,
        on_click: F,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        F: Fn(&ClickEvent, &mut Window, &mut Context<Self>) + 'static,
    {
        let theme = cx.global::<ThemeSettings>();

        div()
            .px_3()
            .py(px(6.0))
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .hover(|s| s.bg(theme.colors.hover))
            .child(
                Icon::new(icon)
                    .size(IconSize::Small)
                    .color(theme.colors.text_muted)
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.colors.text)
                    .child(label)
            )
            .on_click(on_click)
    }

    fn render_menu_item_danger<F>(
        &self,
        label: &str,
        icon: IconName,
        on_click: F,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        F: Fn(&ClickEvent, &mut Window, &mut Context<Self>) + 'static,
    {
        let danger_color = hsla(0.0, 0.8, 0.5, 1.0);
        let theme = cx.global::<ThemeSettings>();

        div()
            .px_3()
            .py(px(6.0))
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .hover(|s| s.bg(danger_color.opacity(0.1)))
            .child(
                Icon::new(icon)
                    .size(IconSize::Small)
                    .color(danger_color)
            )
            .child(
                div()
                    .text_sm()
                    .text_color(danger_color)
                    .child(label)
            )
            .on_click(on_click)
    }

    fn render_menu_separator(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();

        div()
            .my_1()
            .h(px(1.0))
            .bg(theme.colors.border)
    }
}

fn count_connections(nodes: &[TreeNode]) -> usize {
    nodes.iter().map(|node| {
        match node {
            TreeNode::Connection(_) => 1,
            TreeNode::Group { children, .. } => count_connections(children),
        }
    }).sum()
}
```

### 7. Connection Sidebar Panel

```rust
// src/ui/sidebar/connections_panel.rs

use gpui::*;
use uuid::Uuid;
use crate::ui::sidebar::connection_tree::{ConnectionTree, ConnectionTreeEvent};
use crate::ui::dialogs::connection_dialog::ConnectionDialog;
use crate::state::TuskState;

/// The connections panel in the sidebar
pub struct ConnectionsPanel {
    tree: Entity<ConnectionTree>,
    search_text: String,
    show_dialog: bool,
    dialog: Option<Entity<ConnectionDialog>>,
}

impl ConnectionsPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let tree = cx.new(|_| ConnectionTree::new());

        // Subscribe to tree events
        cx.subscribe(&tree, Self::handle_tree_event).detach();

        // Load initial data
        Self::load_connections(&tree, cx);

        Self {
            tree,
            search_text: String::new(),
            show_dialog: false,
            dialog: None,
        }
    }

    fn load_connections(tree: &Entity<ConnectionTree>, cx: &mut Context<Self>) {
        let tusk_state = cx.global::<TuskState>();

        // Load connections from storage
        let connections = tusk_state.storage_service
            .list_connections()
            .unwrap_or_default();

        // Get status for each connection
        let connections_with_status: Vec<_> = connections.into_iter()
            .map(|config| {
                let status = tusk_state.connection_service
                    .get_status(&config.id)
                    .unwrap_or_default();
                ConnectionWithStatus { config, status }
            })
            .collect();

        // Load groups
        let groups = tusk_state.storage_service
            .list_groups()
            .unwrap_or_default();

        tree.update(cx, |tree, _| {
            tree.set_connections(connections_with_status);
            tree.set_groups(groups);
        });
    }

    fn handle_tree_event(
        &mut self,
        tree: &Entity<ConnectionTree>,
        event: &ConnectionTreeEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            ConnectionTreeEvent::OpenQueryTab(conn_id) => {
                // Emit to parent to open a query tab
                cx.emit(ConnectionsPanelEvent::OpenQueryTab(*conn_id));
            }
            ConnectionTreeEvent::EditConnection(config) => {
                self.open_edit_dialog(config.clone(), cx);
            }
            ConnectionTreeEvent::DeleteConnection(conn_id) => {
                self.delete_connection(*conn_id, cx);
            }
            ConnectionTreeEvent::CreateGroup => {
                self.create_group(cx);
            }
            ConnectionTreeEvent::EditGroup(group_id) => {
                self.edit_group(*group_id, cx);
            }
            ConnectionTreeEvent::DeleteGroup(group_id) => {
                self.delete_group(*group_id, cx);
            }
        }
    }

    fn open_new_dialog(&mut self, cx: &mut Context<Self>) {
        let dialog = cx.new(|_| {
            ConnectionDialog::new()
                .on_save(|config| {
                    // Handled via close callback
                })
                .on_close(|| {
                    // Handled in parent
                })
        });

        self.dialog = Some(dialog);
        self.show_dialog = true;
        cx.notify();
    }

    fn open_edit_dialog(&mut self, config: ConnectionConfig, cx: &mut Context<Self>) {
        let dialog = cx.new(|_| {
            ConnectionDialog::edit(&config)
                .on_save(|_| {})
                .on_close(|| {})
        });

        self.dialog = Some(dialog);
        self.show_dialog = true;
        cx.notify();
    }

    fn close_dialog(&mut self, cx: &mut Context<Self>) {
        self.show_dialog = false;
        self.dialog = None;

        // Refresh the tree
        Self::load_connections(&self.tree, cx);
        cx.notify();
    }

    fn delete_connection(&mut self, conn_id: Uuid, cx: &mut Context<Self>) {
        let tusk_state = cx.global::<TuskState>();

        // Disconnect if connected
        let _ = tusk_state.connection_service.disconnect(&conn_id);

        // Delete from storage
        if let Err(e) = tusk_state.storage_service.delete_connection(&conn_id) {
            log::error!("Failed to delete connection: {}", e);
            return;
        }

        // Delete credentials
        let _ = tusk_state.keyring_service.delete_password(&conn_id);
        let _ = tusk_state.keyring_service.delete_ssh_password(&conn_id);
        let _ = tusk_state.keyring_service.delete_ssh_passphrase(&conn_id);

        // Refresh tree
        Self::load_connections(&self.tree, cx);
    }

    fn create_group(&mut self, cx: &mut Context<Self>) {
        let tusk_state = cx.global::<TuskState>();

        let group = ConnectionGroup {
            id: Uuid::new_v4(),
            name: "New Group".to_string(),
            parent_id: None,
            expanded: true,
            order: 0,
        };

        if let Err(e) = tusk_state.storage_service.insert_group(&group) {
            log::error!("Failed to create group: {}", e);
            return;
        }

        Self::load_connections(&self.tree, cx);
    }

    fn edit_group(&mut self, group_id: Uuid, cx: &mut Context<Self>) {
        // Would open a rename dialog
        // For simplicity, emit event to parent
        cx.emit(ConnectionsPanelEvent::EditGroup(group_id));
    }

    fn delete_group(&mut self, group_id: Uuid, cx: &mut Context<Self>) {
        let tusk_state = cx.global::<TuskState>();

        // Move connections out of group first
        if let Ok(connections) = tusk_state.storage_service.list_connections() {
            for mut conn in connections {
                if conn.group_id == Some(group_id) {
                    conn.group_id = None;
                    let _ = tusk_state.storage_service.update_connection(&conn);
                }
            }
        }

        // Delete group
        if let Err(e) = tusk_state.storage_service.delete_group(&group_id) {
            log::error!("Failed to delete group: {}", e);
            return;
        }

        Self::load_connections(&self.tree, cx);
    }
}

#[derive(Debug, Clone)]
pub enum ConnectionsPanelEvent {
    OpenQueryTab(Uuid),
    EditGroup(Uuid),
}

impl EventEmitter<ConnectionsPanelEvent> for ConnectionsPanel {}

impl Render for ConnectionsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();

        div()
            .id("connections-panel")
            .flex_1()
            .flex()
            .flex_col()
            .bg(theme.colors.sidebar)
            // Header
            .child(
                div()
                    .px_3()
                    .py_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text)
                            .child("Connections")
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            // New group button
                            .child(
                                self.render_icon_button(
                                    IconName::FolderPlus,
                                    "New Group",
                                    cx.listener(|this, _, cx| {
                                        this.create_group(cx);
                                    }),
                                    cx,
                                )
                            )
                            // New connection button
                            .child(
                                self.render_icon_button(
                                    IconName::Plus,
                                    "New Connection",
                                    cx.listener(|this, _, cx| {
                                        this.open_new_dialog(cx);
                                    }),
                                    cx,
                                )
                            )
                    )
            )
            // Search box
            .child(
                div()
                    .px_3()
                    .pb_2()
                    .child(self.render_search_input(cx))
            )
            // Connection tree
            .child(self.tree.clone())
            // Dialog overlay
            .when(self.show_dialog, |s| {
                if let Some(ref dialog) = self.dialog {
                    s.child(dialog.clone())
                } else {
                    s
                }
            })
    }
}

impl ConnectionsPanel {
    fn render_icon_button<F>(
        &self,
        icon: IconName,
        tooltip: &str,
        on_click: F,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        F: Fn(&ClickEvent, &mut Window, &mut Context<Self>) + 'static,
    {
        let theme = cx.global::<ThemeSettings>();

        div()
            .w_6()
            .h_6()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.0))
            .cursor_pointer()
            .hover(|s| s.bg(theme.colors.hover))
            .child(
                Icon::new(icon)
                    .size(IconSize::Small)
                    .color(theme.colors.text_muted)
            )
            .on_click(on_click)
    }

    fn render_search_input(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();

        div()
            .h_7()
            .px_2()
            .bg(theme.colors.input_background)
            .border_1()
            .border_color(theme.colors.border)
            .rounded(px(4.0))
            .flex()
            .items_center()
            .gap_2()
            .child(
                Icon::new(IconName::Search)
                    .size(IconSize::Small)
                    .color(theme.colors.text_muted)
            )
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .when(self.search_text.is_empty(), |s| {
                        s.text_color(theme.colors.text_muted)
                            .child("Search connections...")
                    })
                    .when(!self.search_text.is_empty(), |s| {
                        s.text_color(theme.colors.text)
                            .child(&self.search_text)
                    })
            )
    }
}
```

### 8. Icon Definitions

```rust
// src/ui/icons.rs

use gpui::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconName {
    Database,
    Folder,
    FolderPlus,
    ChevronDown,
    ChevronRight,
    Plus,
    Plug,
    PlugOff,
    Code,
    Edit,
    Copy,
    Trash,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconSize {
    Small,  // 14px
    Medium, // 16px
    Large,  // 20px
}

impl IconSize {
    fn pixels(&self) -> Pixels {
        match self {
            Self::Small => px(14.0),
            Self::Medium => px(16.0),
            Self::Large => px(20.0),
        }
    }
}

pub struct Icon {
    name: IconName,
    size: IconSize,
    color: Hsla,
}

impl Icon {
    pub fn new(name: IconName) -> Self {
        Self {
            name,
            size: IconSize::Medium,
            color: hsla(0.0, 0.0, 0.5, 1.0),
        }
    }

    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = color;
        self
    }

    fn svg_path(&self) -> &'static str {
        match self.name {
            IconName::Database => "M12 2C6.48 2 2 4.24 2 7v10c0 2.76 4.48 5 10 5s10-2.24 10-5V7c0-2.76-4.48-5-10-5zm0 2c4.41 0 8 1.79 8 3s-3.59 3-8 3-8-1.79-8-3 3.59-3 8-3zM4 17v-3.27c1.62 1.2 4.51 2.27 8 2.27s6.38-1.07 8-2.27V17c0 1.21-3.59 3-8 3s-8-1.79-8-3z",
            IconName::Folder => "M10 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z",
            IconName::FolderPlus => "M20 6h-8l-2-2H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm-1 8h-3v3h-2v-3h-3v-2h3V9h2v3h3v2z",
            IconName::ChevronDown => "M7 10l5 5 5-5z",
            IconName::ChevronRight => "M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z",
            IconName::Plus => "M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z",
            IconName::Plug => "M12 2a2 2 0 0 1 2 2c0 .74-.4 1.39-1 1.73V7h1a7 7 0 0 1 7 7h1a1 1 0 0 1 1 1v3a1 1 0 0 1-1 1h-1v1a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-1H3a1 1 0 0 1-1-1v-3a1 1 0 0 1 1-1h1a7 7 0 0 1 7-7h1V5.73c-.6-.34-1-.99-1-1.73a2 2 0 0 1 2-2z",
            IconName::PlugOff => "M2 5.27L3.28 4 20 20.72 18.73 22l-3.08-3.08c-.39.05-.78.08-1.18.08H6a2 2 0 0 1-2-2v-1H3a1 1 0 0 1-1-1v-3a1 1 0 0 1 1-1h1a7 7 0 0 1 .68-2.97L2 5.27zm8 1.73h1V5.73c-.6-.34-1-.99-1-1.73a2 2 0 0 1 4 0c0 .74-.4 1.39-1 1.73V7h1a7 7 0 0 1 7 7h1a1 1 0 0 1 1 1v3a1 1 0 0 1-1 1h-.18L10 8.82V7z",
            IconName::Code => "M9.4 16.6L4.8 12l4.6-4.6L8 6l-6 6 6 6 1.4-1.4zm5.2 0l4.6-4.6-4.6-4.6L16 6l6 6-6 6-1.4-1.4z",
            IconName::Edit => "M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z",
            IconName::Copy => "M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z",
            IconName::Trash => "M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z",
            IconName::Search => "M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z",
        }
    }
}

impl IntoElement for Icon {
    type Element = Svg;

    fn into_element(self) -> Self::Element {
        let size = self.size.pixels();

        svg()
            .path(self.svg_path())
            .size(size)
            .text_color(self.color)
    }
}
```

### 9. Quick Connect Widget

```rust
// src/ui/widgets/quick_connect.rs

use gpui::*;
use crate::state::TuskState;
use crate::services::connection::{ConnectionConfig, ConnectionOptions, SslMode};

/// A compact widget for quick connection without opening the full dialog
pub struct QuickConnect {
    host: String,
    port: String,
    database: String,
    username: String,
    password: String,
    is_connecting: bool,
    error: Option<String>,
    on_connected: Option<Box<dyn Fn(uuid::Uuid) + Send + Sync>>,
}

impl QuickConnect {
    pub fn new() -> Self {
        Self {
            host: "localhost".to_string(),
            port: "5432".to_string(),
            database: "postgres".to_string(),
            username: "postgres".to_string(),
            password: String::new(),
            is_connecting: false,
            error: None,
            on_connected: None,
        }
    }

    pub fn on_connected(mut self, callback: impl Fn(uuid::Uuid) + Send + Sync + 'static) -> Self {
        self.on_connected = Some(Box::new(callback));
        self
    }

    fn connect(&mut self, cx: &mut Context<Self>) {
        let port: u16 = match self.port.parse() {
            Ok(p) => p,
            Err(_) => {
                self.error = Some("Invalid port number".to_string());
                cx.notify();
                return;
            }
        };

        let config = ConnectionConfig {
            id: uuid::Uuid::new_v4(),
            name: format!("{}@{}", self.database, self.host),
            color: None,
            group_id: None,
            host: self.host.clone(),
            port,
            database: self.database.clone(),
            username: self.username.clone(),
            password_in_keyring: false, // Quick connect doesn't save to keyring
            ssl_mode: SslMode::Prefer,
            ssl_ca_cert: None,
            ssl_client_cert: None,
            ssl_client_key: None,
            ssh_tunnel: None,
            options: ConnectionOptions::default(),
        };

        self.is_connecting = true;
        self.error = None;
        cx.notify();

        let tusk_state = cx.global::<TuskState>();
        let connection_service = tusk_state.connection_service.clone();
        let password = self.password.clone();
        let on_connected = self.on_connected.take();

        cx.spawn(|this, mut cx| async move {
            // Store password temporarily
            let keyring = connection_service.keyring();
            let _ = keyring.store_password(&config.id, &password);

            // Connect
            let result = connection_service.connect(config.clone());

            this.update(&mut cx, |this, cx| {
                this.is_connecting = false;

                match result {
                    Ok(_) => {
                        if let Some(on_connected) = &on_connected {
                            on_connected(config.id);
                        }
                        // Clear form
                        this.password.clear();
                    }
                    Err(e) => {
                        this.error = Some(e.to_string());
                        // Clean up temporary password
                        let _ = keyring.delete_password(&config.id);
                    }
                }

                cx.notify();
            }).ok();
        }).detach();
    }
}

impl Render for QuickConnect {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();

        div()
            .id("quick-connect")
            .p_4()
            .bg(theme.colors.surface)
            .border_1()
            .border_color(theme.colors.border)
            .rounded_lg()
            .flex()
            .flex_col()
            .gap_3()
            // Title
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.colors.text)
                    .child("Quick Connect")
            )
            // Host:Port row
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        self.render_input("Host", &self.host, "localhost", false, cx)
                            .flex_1()
                    )
                    .child(
                        self.render_input("Port", &self.port, "5432", false, cx)
                            .w(px(80.0))
                    )
            )
            // Database row
            .child(self.render_input("Database", &self.database, "postgres", false, cx))
            // Username row
            .child(self.render_input("Username", &self.username, "postgres", false, cx))
            // Password row
            .child(self.render_input("Password", &self.password, "", true, cx))
            // Error message
            .when(self.error.is_some(), |s| {
                s.child(
                    div()
                        .text_xs()
                        .text_color(hsla(0.0, 0.8, 0.5, 1.0))
                        .child(self.error.as_ref().unwrap().as_str())
                )
            })
            // Connect button
            .child(
                div()
                    .w_full()
                    .h_8()
                    .bg(theme.colors.accent)
                    .rounded(px(4.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .when(self.is_connecting, |s| s.opacity(0.7))
                    .hover(|s| s.opacity(0.9))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(white())
                            .child(if self.is_connecting { "Connecting..." } else { "Connect" })
                    )
                    .when(!self.is_connecting, |s| {
                        s.on_click(cx.listener(|this, _, cx| {
                            this.connect(cx);
                        }))
                    })
            )
    }
}

impl QuickConnect {
    fn render_input(
        &self,
        label: &str,
        value: &str,
        placeholder: &str,
        is_password: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.global::<ThemeSettings>();
        let display = if is_password && !value.is_empty() {
            "•".repeat(value.len())
        } else if value.is_empty() {
            placeholder.to_string()
        } else {
            value.to_string()
        };

        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.colors.text_muted)
                    .child(label)
            )
            .child(
                div()
                    .h_7()
                    .px_2()
                    .bg(theme.colors.input_background)
                    .border_1()
                    .border_color(theme.colors.border)
                    .rounded(px(4.0))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .when(value.is_empty(), |s| s.text_color(theme.colors.text_muted))
                            .when(!value.is_empty(), |s| s.text_color(theme.colors.text))
                            .child(display)
                    )
            )
    }
}
```

### 10. Drag and Drop for Reordering

```rust
// src/ui/sidebar/connection_tree.rs (drag-drop additions)

impl ConnectionTree {
    fn start_drag(&mut self, item: DragItem, cx: &mut Context<Self>) {
        self.drag_state = Some(DragState {
            dragging: item,
            over: None,
        });
        cx.notify();
    }

    fn update_drag_over(&mut self, target: Option<DropTarget>, cx: &mut Context<Self>) {
        if let Some(ref mut state) = self.drag_state {
            state.over = target;
            cx.notify();
        }
    }

    fn end_drag(&mut self, cx: &mut Context<Self>) {
        if let Some(state) = self.drag_state.take() {
            if let Some(target) = state.over {
                self.apply_drop(state.dragging, target, cx);
            }
        }
        cx.notify();
    }

    fn apply_drop(&mut self, item: DragItem, target: DropTarget, cx: &mut Context<Self>) {
        let tusk_state = cx.global::<TuskState>();

        match (item, target) {
            (DragItem::Connection(conn_id), DropTarget::Group(group_id)) => {
                // Move connection to group
                if let Some(conn) = self.connections.iter_mut().find(|c| c.config.id == conn_id) {
                    conn.config.group_id = Some(group_id);
                    let _ = tusk_state.storage_service.update_connection(&conn.config);
                }
            }
            (DragItem::Connection(conn_id), DropTarget::Root) => {
                // Move connection to root
                if let Some(conn) = self.connections.iter_mut().find(|c| c.config.id == conn_id) {
                    conn.config.group_id = None;
                    let _ = tusk_state.storage_service.update_connection(&conn.config);
                }
            }
            (DragItem::Group(source_id), DropTarget::Group(target_id)) => {
                // Move group into another group
                if source_id != target_id {
                    if let Some(group) = self.groups.iter_mut().find(|g| g.id == source_id) {
                        group.parent_id = Some(target_id);
                        let _ = tusk_state.storage_service.update_group(group);
                    }
                }
            }
            (DragItem::Group(source_id), DropTarget::Root) => {
                // Move group to root
                if let Some(group) = self.groups.iter_mut().find(|g| g.id == source_id) {
                    group.parent_id = None;
                    let _ = tusk_state.storage_service.update_group(group);
                }
            }
            _ => {}
        }
    }

    fn render_connection_node_draggable(
        &self,
        conn: &ConnectionWithStatus,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let conn_id = conn.config.id;
        let is_dragging = self.drag_state.as_ref()
            .map(|s| matches!(&s.dragging, DragItem::Connection(id) if *id == conn_id))
            .unwrap_or(false);
        let is_drop_target = self.drag_state.as_ref()
            .map(|s| matches!(&s.over, Some(DropTarget::Connection(id)) if *id == conn_id))
            .unwrap_or(false);

        self.render_connection_node(conn, depth, cx)
            .when(is_dragging, |s| s.opacity(0.5))
            .when(is_drop_target, |s| {
                s.border_t_2()
                    .border_color(cx.global::<ThemeSettings>().colors.accent)
            })
            .on_drag(cx.listener(move |this, _, cx| {
                this.start_drag(DragItem::Connection(conn_id), cx);
            }))
            .on_drag_over(cx.listener(move |this, _, cx| {
                this.update_drag_over(Some(DropTarget::Connection(conn_id)), cx);
            }))
            .on_drop(cx.listener(move |this, _, cx| {
                this.end_drag(cx);
            }))
    }
}
```

## Acceptance Criteria

1. [ ] Connection dialog opens for new/edit connections
2. [ ] All connection fields populate correctly
3. [ ] Test connection shows server version and latency
4. [ ] Validation prevents saving invalid connections
5. [ ] Password stored in keyring on save
6. [ ] Connection tree displays all connections
7. [ ] Groups expand/collapse correctly
8. [ ] Status indicators show correct colors with animation for connecting state
9. [ ] Double-click opens new query tab
10. [ ] Context menu shows correct actions based on connection state
11. [ ] Disconnect/connect works from context menu
12. [ ] Edit opens dialog with connection data
13. [ ] Delete removes connection with confirmation
14. [ ] Search filters connections correctly
15. [ ] Drag-drop reorders connections between groups
16. [ ] Keyboard navigation works throughout

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_dialog_validation() {
        let mut dialog = ConnectionDialog::new();

        // Empty name should fail
        let errors = dialog.validate();
        assert!(errors.iter().any(|e| e.field == "name"));

        // Valid form should pass
        dialog.name = "Test".to_string();
        dialog.host = "localhost".to_string();
        dialog.port = "5432".to_string();
        dialog.database = "postgres".to_string();
        dialog.username = "postgres".to_string();

        let errors = dialog.validate();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_tree_building() {
        let mut tree = ConnectionTree::new();

        let group = ConnectionGroup {
            id: Uuid::new_v4(),
            name: "Production".to_string(),
            parent_id: None,
            expanded: true,
            order: 0,
        };

        let conn = ConnectionWithStatus {
            config: ConnectionConfig {
                id: Uuid::new_v4(),
                name: "Prod DB".to_string(),
                group_id: Some(group.id),
                ..Default::default()
            },
            status: ConnectionStatus::Disconnected,
        };

        tree.set_groups(vec![group.clone()]);
        tree.set_connections(vec![conn]);

        let nodes = tree.build_tree();
        assert_eq!(nodes.len(), 1);

        if let TreeNode::Group { group: g, children } = &nodes[0] {
            assert_eq!(g.name, "Production");
            assert_eq!(children.len(), 1);
        } else {
            panic!("Expected group node");
        }
    }

    #[test]
    fn test_connection_filtering() {
        let mut tree = ConnectionTree::new();

        tree.set_connections(vec![
            ConnectionWithStatus {
                config: ConnectionConfig {
                    name: "Production".to_string(),
                    host: "prod.example.com".to_string(),
                    ..Default::default()
                },
                status: ConnectionStatus::Disconnected,
            },
            ConnectionWithStatus {
                config: ConnectionConfig {
                    name: "Development".to_string(),
                    host: "localhost".to_string(),
                    ..Default::default()
                },
                status: ConnectionStatus::Disconnected,
            },
        ]);

        // No filter - both visible
        let nodes = tree.build_tree();
        assert_eq!(nodes.len(), 2);

        // Filter by name
        tree.set_filter("prod".to_string());
        let nodes = tree.build_tree();
        assert_eq!(nodes.len(), 1);

        // Filter by host
        tree.set_filter("localhost".to_string());
        let nodes = tree.build_tree();
        assert_eq!(nodes.len(), 1);
    }

    #[test]
    fn test_color_parsing() {
        let blue = parse_hex_color("#3b82f6").unwrap();
        assert!(blue.h > 0.5 && blue.h < 0.7); // Blue hue range

        let red = parse_hex_color("#ef4444").unwrap();
        assert!(red.h < 0.1 || red.h > 0.9); // Red hue range

        assert!(parse_hex_color("invalid").is_none());
        assert!(parse_hex_color("#fff").is_none()); // Too short
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use gpui::TestAppContext;

    #[gpui::test]
    async fn test_connection_dialog_flow(cx: &mut TestAppContext) {
        // Initialize app state
        let app = cx.new(|cx| {
            let state = TuskState::new(cx).unwrap();
            cx.set_global(state);
            TestApp::new(cx)
        });

        // Open connection dialog
        app.update(cx, |app, cx| {
            app.open_connection_dialog(cx);
        });

        // Verify dialog is visible
        cx.run_until_parked();

        // Fill in form and save
        app.update(cx, |app, cx| {
            if let Some(dialog) = &mut app.connection_dialog {
                dialog.name = "Test Connection".to_string();
                dialog.host = "localhost".to_string();
                dialog.handle_save(cx);
            }
        });

        cx.run_until_parked();

        // Verify connection was saved
        let state = cx.global::<TuskState>();
        let connections = state.storage_service.list_connections().unwrap();
        assert!(connections.iter().any(|c| c.name == "Test Connection"));
    }
}
```

## Dependencies on Other Features

- 06-settings-theming-credentials.md (Theme system, KeyringService)
- 07-connection-management.md (ConnectionService, ConnectionConfig, ConnectionStatus)
- 08-ssl-ssh-security.md (SSL mode enums, SSH tunnel config, file browse integration)

## Dependent Features

- 10-schema-introspection.md (Uses connected connections)
- 11-query-execution.md (Uses active connection)
- 12-sql-editor.md (Uses connection for autocomplete context)
- 16-schema-browser.md (Extends connection tree with schema nodes)
