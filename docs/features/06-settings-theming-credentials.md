# Feature 06: Settings, Theming & Credentials

## Overview

Implement the settings system, theme management (light/dark/system), and secure credential storage using OS keychain. This is a pure GPUI implementation with themes as Globals, settings stored in SQLite via StorageService, and passwords secured in the OS keychain.

## Goals

- Complete settings panel with all categories from design doc
- Theme system with light/dark/system modes and color palette
- OS keychain integration for passwords and SSH passphrases
- Settings persistence via StorageService
- Keyboard shortcut customization integrated with GPUI's key bindings
- Platform-native appearance following OS conventions

## Technical Specification

### 1. Theme System

```rust
// src/theme/mod.rs

use gpui::{Global, Hsla, Rgba, WindowAppearance};
use serde::{Deserialize, Serialize};

/// Theme mode preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    Light,
    Dark,
    #[default]
    System,
}

/// Active theme colors.
pub struct Theme {
    pub mode: ThemeMode,
    pub appearance: Appearance,
    pub colors: ThemeColors,
    pub syntax: SyntaxColors,
}

impl Global for Theme {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
}

impl Appearance {
    pub fn is_dark(&self) -> bool {
        matches!(self, Self::Dark)
    }
}

/// Color palette for UI elements.
#[derive(Debug, Clone)]
pub struct ThemeColors {
    // Backgrounds
    pub background: Hsla,
    pub surface: Hsla,
    pub surface_elevated: Hsla,
    pub panel: Hsla,

    // Text
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_placeholder: Hsla,
    pub text_accent: Hsla,

    // Interactive states
    pub hover: Hsla,
    pub selection: Hsla,
    pub focus_ring: Hsla,

    // Borders
    pub border: Hsla,
    pub border_focused: Hsla,
    pub border_selected: Hsla,

    // Semantic colors
    pub error: Hsla,
    pub warning: Hsla,
    pub success: Hsla,
    pub info: Hsla,

    // Editor specific
    pub editor_background: Hsla,
    pub editor_gutter: Hsla,
    pub editor_line_highlight: Hsla,
    pub editor_selection: Hsla,

    // Results grid
    pub grid_header: Hsla,
    pub grid_row_even: Hsla,
    pub grid_row_odd: Hsla,
    pub grid_row_hover: Hsla,
    pub grid_cell_null: Hsla,

    // Connection status
    pub connected: Hsla,
    pub disconnected: Hsla,
    pub connecting: Hsla,
}

/// SQL syntax highlighting colors.
#[derive(Debug, Clone)]
pub struct SyntaxColors {
    pub keyword: Hsla,
    pub string: Hsla,
    pub number: Hsla,
    pub comment: Hsla,
    pub operator: Hsla,
    pub function: Hsla,
    pub type_name: Hsla,
    pub identifier: Hsla,
    pub table: Hsla,
    pub column: Hsla,
}

impl Theme {
    /// Create theme from mode, resolving system preference.
    pub fn from_mode(mode: ThemeMode, system_appearance: WindowAppearance) -> Self {
        let appearance = match mode {
            ThemeMode::Light => Appearance::Light,
            ThemeMode::Dark => Appearance::Dark,
            ThemeMode::System => match system_appearance {
                WindowAppearance::Dark | WindowAppearance::VibrantDark => Appearance::Dark,
                _ => Appearance::Light,
            },
        };

        let (colors, syntax) = match appearance {
            Appearance::Light => (Self::light_colors(), Self::light_syntax()),
            Appearance::Dark => (Self::dark_colors(), Self::dark_syntax()),
        };

        Self {
            mode,
            appearance,
            colors,
            syntax,
        }
    }

    fn light_colors() -> ThemeColors {
        ThemeColors {
            // Backgrounds
            background: hsla(0.0, 0.0, 0.98, 1.0),
            surface: hsla(0.0, 0.0, 1.0, 1.0),
            surface_elevated: hsla(0.0, 0.0, 1.0, 1.0),
            panel: hsla(0.0, 0.0, 0.96, 1.0),

            // Text
            text: hsla(0.0, 0.0, 0.1, 1.0),
            text_muted: hsla(0.0, 0.0, 0.4, 1.0),
            text_placeholder: hsla(0.0, 0.0, 0.6, 1.0),
            text_accent: hsla(211.0, 0.8, 0.45, 1.0),

            // Interactive
            hover: hsla(0.0, 0.0, 0.0, 0.04),
            selection: hsla(211.0, 0.8, 0.45, 0.15),
            focus_ring: hsla(211.0, 0.8, 0.55, 1.0),

            // Borders
            border: hsla(0.0, 0.0, 0.0, 0.1),
            border_focused: hsla(211.0, 0.8, 0.55, 1.0),
            border_selected: hsla(211.0, 0.8, 0.45, 1.0),

            // Semantic
            error: hsla(0.0, 0.7, 0.5, 1.0),
            warning: hsla(35.0, 0.9, 0.5, 1.0),
            success: hsla(120.0, 0.5, 0.4, 1.0),
            info: hsla(211.0, 0.8, 0.55, 1.0),

            // Editor
            editor_background: hsla(0.0, 0.0, 1.0, 1.0),
            editor_gutter: hsla(0.0, 0.0, 0.96, 1.0),
            editor_line_highlight: hsla(0.0, 0.0, 0.0, 0.03),
            editor_selection: hsla(211.0, 0.8, 0.45, 0.2),

            // Grid
            grid_header: hsla(0.0, 0.0, 0.96, 1.0),
            grid_row_even: hsla(0.0, 0.0, 1.0, 1.0),
            grid_row_odd: hsla(0.0, 0.0, 0.98, 1.0),
            grid_row_hover: hsla(211.0, 0.8, 0.45, 0.08),
            grid_cell_null: hsla(0.0, 0.0, 0.6, 1.0),

            // Connection
            connected: hsla(120.0, 0.5, 0.4, 1.0),
            disconnected: hsla(0.0, 0.0, 0.5, 1.0),
            connecting: hsla(35.0, 0.9, 0.5, 1.0),
        }
    }

    fn dark_colors() -> ThemeColors {
        ThemeColors {
            // Backgrounds
            background: hsla(220.0, 0.15, 0.1, 1.0),
            surface: hsla(220.0, 0.15, 0.12, 1.0),
            surface_elevated: hsla(220.0, 0.15, 0.14, 1.0),
            panel: hsla(220.0, 0.15, 0.08, 1.0),

            // Text
            text: hsla(0.0, 0.0, 0.9, 1.0),
            text_muted: hsla(0.0, 0.0, 0.6, 1.0),
            text_placeholder: hsla(0.0, 0.0, 0.4, 1.0),
            text_accent: hsla(211.0, 0.8, 0.65, 1.0),

            // Interactive
            hover: hsla(0.0, 0.0, 1.0, 0.06),
            selection: hsla(211.0, 0.8, 0.55, 0.25),
            focus_ring: hsla(211.0, 0.8, 0.65, 1.0),

            // Borders
            border: hsla(0.0, 0.0, 1.0, 0.1),
            border_focused: hsla(211.0, 0.8, 0.65, 1.0),
            border_selected: hsla(211.0, 0.8, 0.55, 1.0),

            // Semantic
            error: hsla(0.0, 0.7, 0.6, 1.0),
            warning: hsla(35.0, 0.9, 0.55, 1.0),
            success: hsla(120.0, 0.5, 0.5, 1.0),
            info: hsla(211.0, 0.8, 0.65, 1.0),

            // Editor
            editor_background: hsla(220.0, 0.15, 0.1, 1.0),
            editor_gutter: hsla(220.0, 0.15, 0.12, 1.0),
            editor_line_highlight: hsla(0.0, 0.0, 1.0, 0.04),
            editor_selection: hsla(211.0, 0.8, 0.55, 0.3),

            // Grid
            grid_header: hsla(220.0, 0.15, 0.14, 1.0),
            grid_row_even: hsla(220.0, 0.15, 0.12, 1.0),
            grid_row_odd: hsla(220.0, 0.15, 0.1, 1.0),
            grid_row_hover: hsla(211.0, 0.8, 0.55, 0.12),
            grid_cell_null: hsla(0.0, 0.0, 0.4, 1.0),

            // Connection
            connected: hsla(120.0, 0.5, 0.5, 1.0),
            disconnected: hsla(0.0, 0.0, 0.5, 1.0),
            connecting: hsla(35.0, 0.9, 0.55, 1.0),
        }
    }

    fn light_syntax() -> SyntaxColors {
        SyntaxColors {
            keyword: hsla(280.0, 0.6, 0.45, 1.0),   // Purple
            string: hsla(30.0, 0.7, 0.45, 1.0),    // Orange
            number: hsla(200.0, 0.8, 0.4, 1.0),    // Cyan
            comment: hsla(0.0, 0.0, 0.5, 1.0),     // Gray
            operator: hsla(0.0, 0.0, 0.3, 1.0),    // Dark gray
            function: hsla(200.0, 0.7, 0.4, 1.0),  // Blue
            type_name: hsla(180.0, 0.6, 0.4, 1.0), // Teal
            identifier: hsla(0.0, 0.0, 0.2, 1.0), // Near black
            table: hsla(280.0, 0.5, 0.5, 1.0),    // Light purple
            column: hsla(220.0, 0.6, 0.45, 1.0),  // Blue
        }
    }

    fn dark_syntax() -> SyntaxColors {
        SyntaxColors {
            keyword: hsla(280.0, 0.6, 0.7, 1.0),   // Light purple
            string: hsla(30.0, 0.7, 0.65, 1.0),    // Light orange
            number: hsla(200.0, 0.8, 0.65, 1.0),   // Light cyan
            comment: hsla(0.0, 0.0, 0.5, 1.0),     // Gray
            operator: hsla(0.0, 0.0, 0.7, 1.0),    // Light gray
            function: hsla(200.0, 0.7, 0.65, 1.0), // Light blue
            type_name: hsla(180.0, 0.6, 0.65, 1.0),// Light teal
            identifier: hsla(0.0, 0.0, 0.85, 1.0),// Near white
            table: hsla(280.0, 0.5, 0.7, 1.0),    // Light purple
            column: hsla(220.0, 0.6, 0.7, 1.0),   // Light blue
        }
    }
}

fn hsla(h: f32, s: f32, l: f32, a: f32) -> Hsla {
    Hsla { h: h / 360.0, s, l, a }
}
```

### 2. Theme Manager

```rust
// src/theme/manager.rs

use gpui::*;
use crate::state::TuskState;
use crate::theme::{Theme, ThemeMode};

/// Manages theme state and switching.
pub struct ThemeManager;

impl ThemeManager {
    /// Initialize theme from saved settings.
    pub fn initialize(cx: &mut AppContext) {
        let settings = cx.global::<TuskState>().storage.get_all_settings()
            .unwrap_or_default();

        let mode = settings.theme.mode;
        let appearance = cx.window_appearance();
        let theme = Theme::from_mode(mode, appearance);

        cx.set_global(theme);

        // Listen for system appearance changes
        cx.observe_window_appearance(|cx| {
            let theme = cx.global::<Theme>();
            if theme.mode == ThemeMode::System {
                ThemeManager::update_theme(theme.mode, cx);
            }
        })
        .detach();
    }

    /// Update theme to a new mode.
    pub fn set_mode(mode: ThemeMode, cx: &mut AppContext) {
        Self::update_theme(mode, cx);

        // Persist to storage
        let state = cx.global::<TuskState>();
        if let Ok(mut settings) = state.storage.get_all_settings() {
            settings.theme.mode = mode;
            let _ = state.storage.save_all_settings(&settings);
        }
    }

    fn update_theme(mode: ThemeMode, cx: &mut AppContext) {
        let appearance = cx.window_appearance();
        let theme = Theme::from_mode(mode, appearance);
        cx.set_global(theme);
    }

    /// Get the current resolved appearance.
    pub fn appearance(cx: &AppContext) -> crate::theme::Appearance {
        cx.global::<Theme>().appearance
    }
}
```

### 3. Credential Management (Keyring)

```rust
// src/services/keyring.rs

use keyring::Entry;
use crate::error::{Result, TuskError};

const SERVICE_NAME: &str = "dev.tusk.Tusk";

/// Secure credential storage using OS keychain.
///
/// - macOS: Keychain
/// - Windows: Credential Manager
/// - Linux: Secret Service (libsecret)
pub struct KeyringService;

impl KeyringService {
    /// Store a database password for a connection.
    pub fn store_password(connection_id: &str, password: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, &format!("db:{}", connection_id))
            .map_err(|e| TuskError::Keyring(format!("Failed to create entry: {}", e)))?;

        entry.set_password(password)
            .map_err(|e| TuskError::Keyring(format!("Failed to store password: {}", e)))?;

        tracing::debug!("Stored password for connection: {}", connection_id);
        Ok(())
    }

    /// Retrieve a database password for a connection.
    pub fn get_password(connection_id: &str) -> Result<Option<String>> {
        let entry = Entry::new(SERVICE_NAME, &format!("db:{}", connection_id))
            .map_err(|e| TuskError::Keyring(format!("Failed to create entry: {}", e)))?;

        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(TuskError::Keyring(format!("Failed to get password: {}", e))),
        }
    }

    /// Delete a database password for a connection.
    pub fn delete_password(connection_id: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, &format!("db:{}", connection_id))
            .map_err(|e| TuskError::Keyring(format!("Failed to create entry: {}", e)))?;

        match entry.delete_credential() {
            Ok(()) => {
                tracing::debug!("Deleted password for connection: {}", connection_id);
                Ok(())
            }
            Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
            Err(e) => Err(TuskError::Keyring(format!("Failed to delete password: {}", e))),
        }
    }

    /// Store SSH key passphrase for a connection.
    pub fn store_ssh_passphrase(connection_id: &str, passphrase: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, &format!("ssh:{}", connection_id))
            .map_err(|e| TuskError::Keyring(format!("Failed to create entry: {}", e)))?;

        entry.set_password(passphrase)
            .map_err(|e| TuskError::Keyring(format!("Failed to store passphrase: {}", e)))?;

        tracing::debug!("Stored SSH passphrase for connection: {}", connection_id);
        Ok(())
    }

    /// Retrieve SSH key passphrase for a connection.
    pub fn get_ssh_passphrase(connection_id: &str) -> Result<Option<String>> {
        let entry = Entry::new(SERVICE_NAME, &format!("ssh:{}", connection_id))
            .map_err(|e| TuskError::Keyring(format!("Failed to create entry: {}", e)))?;

        match entry.get_password() {
            Ok(passphrase) => Ok(Some(passphrase)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(TuskError::Keyring(format!("Failed to get passphrase: {}", e))),
        }
    }

    /// Delete SSH key passphrase for a connection.
    pub fn delete_ssh_passphrase(connection_id: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, &format!("ssh:{}", connection_id))
            .map_err(|e| TuskError::Keyring(format!("Failed to create entry: {}", e)))?;

        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(TuskError::Keyring(format!("Failed to delete passphrase: {}", e))),
        }
    }

    /// Delete all credentials for a connection.
    pub fn delete_all(connection_id: &str) -> Result<()> {
        Self::delete_password(connection_id)?;
        Self::delete_ssh_passphrase(connection_id)?;
        Ok(())
    }

    /// Check if keyring is available on this system.
    pub fn is_available() -> bool {
        // Try to create an entry to check availability
        Entry::new(SERVICE_NAME, "test")
            .map(|_| true)
            .unwrap_or(false)
    }
}
```

### 4. Settings Panel Component

```rust
// src/components/settings_panel.rs

use gpui::*;
use crate::state::TuskState;
use crate::theme::{Theme, ThemeMode, ThemeManager};
use crate::models::settings::*;

/// Settings category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsCategory {
    General,
    Editor,
    Results,
    Query,
    Connections,
    Shortcuts,
}

impl SettingsCategory {
    fn label(&self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Editor => "Editor",
            Self::Results => "Results",
            Self::Query => "Query Execution",
            Self::Connections => "Connections",
            Self::Shortcuts => "Keyboard Shortcuts",
        }
    }

    fn icon(&self) -> IconName {
        match self {
            Self::General => IconName::Settings,
            Self::Editor => IconName::Code,
            Self::Results => IconName::Table,
            Self::Query => IconName::Play,
            Self::Connections => IconName::Database,
            Self::Shortcuts => IconName::Keyboard,
        }
    }

    fn all() -> &'static [Self] {
        &[
            Self::General,
            Self::Editor,
            Self::Results,
            Self::Query,
            Self::Connections,
            Self::Shortcuts,
        ]
    }
}

pub struct SettingsPanel {
    active_category: SettingsCategory,
    settings: Settings,
    original_settings: Settings,
    is_dirty: bool,
    focus_handle: FocusHandle,
}

impl SettingsPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = cx.global::<TuskState>();
        let settings = state.storage.get_all_settings().unwrap_or_default();

        Self {
            active_category: SettingsCategory::General,
            settings: settings.clone(),
            original_settings: settings,
            is_dirty: false,
            focus_handle: cx.focus_handle(),
        }
    }

    fn select_category(&mut self, category: SettingsCategory, cx: &mut Context<Self>) {
        self.active_category = category;
        cx.notify();
    }

    fn mark_dirty(&mut self, cx: &mut Context<Self>) {
        self.is_dirty = true;
        cx.notify();
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let state = cx.global::<TuskState>();

        if let Err(e) = state.storage.save_all_settings(&self.settings) {
            tracing::error!("Failed to save settings: {}", e);
            return;
        }

        // Apply theme change immediately
        ThemeManager::set_mode(self.settings.theme.mode, cx);

        self.original_settings = self.settings.clone();
        self.is_dirty = false;
        cx.notify();
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.settings = self.original_settings.clone();
        self.is_dirty = false;
        cx.notify();
    }

    fn reset_category(&mut self, cx: &mut Context<Self>) {
        match self.active_category {
            SettingsCategory::General => {
                self.settings.theme = ThemeSettings::default();
            }
            SettingsCategory::Editor => {
                self.settings.editor = EditorSettings::default();
            }
            SettingsCategory::Results => {
                self.settings.results = ResultsSettings::default();
            }
            SettingsCategory::Query => {
                self.settings.query_execution = QueryExecutionSettings::default();
            }
            SettingsCategory::Connections => {
                self.settings.connections = ConnectionsSettings::default();
            }
            SettingsCategory::Shortcuts => {
                // Reset shortcuts to defaults
            }
        }
        self.mark_dirty(cx);
    }

    fn render_sidebar(&self, theme: &Theme, cx: &Context<Self>) -> impl IntoElement {
        div()
            .w_48()
            .border_r_1()
            .border_color(theme.colors.border)
            .p_2()
            .flex()
            .flex_col()
            .gap_1()
            .children(SettingsCategory::all().iter().map(|&category| {
                let is_active = self.active_category == category;

                div()
                    .id(ElementId::Name(format!("settings-{:?}", category).into()))
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap_2()
                    .when(is_active, |this| this.bg(theme.colors.selection))
                    .hover(|this| this.bg(theme.colors.hover))
                    .on_click(cx.listener(move |this, _, cx| {
                        this.select_category(category, cx);
                    }))
                    .child(Icon::new(category.icon()).size(IconSize::Small))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.colors.text)
                            .child(category.label())
                    )
            }))
    }

    fn render_content(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .p_4()
            .overflow_y_auto()
            .child(match self.active_category {
                SettingsCategory::General => self.render_general_settings(theme, cx).into_any_element(),
                SettingsCategory::Editor => self.render_editor_settings(theme, cx).into_any_element(),
                SettingsCategory::Results => self.render_results_settings(theme, cx).into_any_element(),
                SettingsCategory::Query => self.render_query_settings(theme, cx).into_any_element(),
                SettingsCategory::Connections => self.render_connections_settings(theme, cx).into_any_element(),
                SettingsCategory::Shortcuts => self.render_shortcuts_settings(theme, cx).into_any_element(),
            })
    }

    fn render_general_settings(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.colors.text)
                    .child("General")
            )
            .child(
                FormField::new("Theme")
                    .child(
                        Select::new("theme-mode")
                            .options(vec![
                                SelectOption::new("light", "Light"),
                                SelectOption::new("dark", "Dark"),
                                SelectOption::new("system", "System"),
                            ])
                            .selected(match self.settings.theme.mode {
                                ThemeMode::Light => "light",
                                ThemeMode::Dark => "dark",
                                ThemeMode::System => "system",
                            })
                            .on_change(cx.listener(|this, value: &str, cx| {
                                this.settings.theme.mode = match value {
                                    "light" => ThemeMode::Light,
                                    "dark" => ThemeMode::Dark,
                                    _ => ThemeMode::System,
                                };
                                this.mark_dirty(cx);
                            }))
                    )
            )
    }

    fn render_editor_settings(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.colors.text)
                    .child("Editor")
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(
                        FormField::new("Font Family")
                            .flex_1()
                            .child(
                                Select::new("font-family")
                                    .options(vec![
                                        SelectOption::new("Zed Plex Mono", "Zed Plex Mono"),
                                        SelectOption::new("JetBrains Mono", "JetBrains Mono"),
                                        SelectOption::new("Fira Code", "Fira Code"),
                                        SelectOption::new("Monaco", "Monaco"),
                                        SelectOption::new("Consolas", "Consolas"),
                                    ])
                                    .selected(&self.settings.editor.font_family)
                                    .on_change(cx.listener(|this, value: &str, cx| {
                                        this.settings.editor.font_family = value.to_string();
                                        this.mark_dirty(cx);
                                    }))
                            )
                    )
                    .child(
                        FormField::new("Font Size")
                            .w_24()
                            .child(
                                NumberInput::new("font-size")
                                    .value(self.settings.editor.font_size)
                                    .min(8.0)
                                    .max(24.0)
                                    .step(1.0)
                                    .on_change(cx.listener(|this, value: f32, cx| {
                                        this.settings.editor.font_size = value;
                                        this.mark_dirty(cx);
                                    }))
                            )
                    )
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(
                        FormField::new("Tab Size")
                            .child(
                                Select::new("tab-size")
                                    .options(vec![
                                        SelectOption::new("2", "2 spaces"),
                                        SelectOption::new("4", "4 spaces"),
                                        SelectOption::new("8", "8 spaces"),
                                    ])
                                    .selected(&self.settings.editor.tab_size.to_string())
                                    .on_change(cx.listener(|this, value: &str, cx| {
                                        this.settings.editor.tab_size = value.parse().unwrap_or(2);
                                        this.mark_dirty(cx);
                                    }))
                            )
                    )
                    .child(
                        FormField::new("Indentation")
                            .child(
                                Select::new("use-spaces")
                                    .options(vec![
                                        SelectOption::new("true", "Spaces"),
                                        SelectOption::new("false", "Tabs"),
                                    ])
                                    .selected(if self.settings.editor.use_spaces { "true" } else { "false" })
                                    .on_change(cx.listener(|this, value: &str, cx| {
                                        this.settings.editor.use_spaces = value == "true";
                                        this.mark_dirty(cx);
                                    }))
                            )
                    )
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        Checkbox::new("line-numbers")
                            .label("Show line numbers")
                            .checked(self.settings.editor.line_numbers)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.settings.editor.line_numbers = checked;
                                this.mark_dirty(cx);
                            }))
                    )
                    .child(
                        Checkbox::new("highlight-line")
                            .label("Highlight current line")
                            .checked(self.settings.editor.highlight_current_line)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.settings.editor.highlight_current_line = checked;
                                this.mark_dirty(cx);
                            }))
                    )
                    .child(
                        Checkbox::new("bracket-matching")
                            .label("Bracket matching")
                            .checked(self.settings.editor.bracket_matching)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.settings.editor.bracket_matching = checked;
                                this.mark_dirty(cx);
                            }))
                    )
                    .child(
                        Checkbox::new("word-wrap")
                            .label("Word wrap")
                            .checked(self.settings.editor.word_wrap)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.settings.editor.word_wrap = checked;
                                this.mark_dirty(cx);
                            }))
                    )
            )
            .child(
                FormField::new("Autocomplete delay (ms)")
                    .child(
                        NumberInput::new("autocomplete-delay")
                            .value(self.settings.editor.autocomplete_delay_ms as f32)
                            .min(0.0)
                            .max(1000.0)
                            .step(50.0)
                            .on_change(cx.listener(|this, value: f32, cx| {
                                this.settings.editor.autocomplete_delay_ms = value as u32;
                                this.mark_dirty(cx);
                            }))
                    )
            )
    }

    fn render_results_settings(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.colors.text)
                    .child("Results")
            )
            .child(
                FormField::new("Default row limit")
                    .description("Maximum rows to fetch for SELECT queries without LIMIT")
                    .child(
                        NumberInput::new("row-limit")
                            .value(self.settings.results.default_row_limit as f32)
                            .min(100.0)
                            .max(100000.0)
                            .step(100.0)
                            .on_change(cx.listener(|this, value: f32, cx| {
                                this.settings.results.default_row_limit = value as u32;
                                this.mark_dirty(cx);
                            }))
                    )
            )
            .child(
                FormField::new("Timestamp format")
                    .child(
                        Select::new("timestamp-format")
                            .options(vec![
                                SelectOption::new("YYYY-MM-DD HH:mm:ss", "ISO (2024-03-15 10:30:00)"),
                                SelectOption::new("MM/DD/YYYY h:mm A", "US (03/15/2024 10:30 AM)"),
                                SelectOption::new("DD/MM/YYYY HH:mm", "EU (15/03/2024 10:30)"),
                            ])
                            .selected(&self.settings.results.timestamp_format)
                            .on_change(cx.listener(|this, value: &str, cx| {
                                this.settings.results.timestamp_format = value.to_string();
                                this.mark_dirty(cx);
                            }))
                    )
            )
            .child(
                FormField::new("NULL display")
                    .child(
                        TextInput::new("null-display")
                            .value(self.settings.results.null_display.clone())
                            .placeholder("NULL")
                            .on_change(cx.listener(|this, value: String, cx| {
                                this.settings.results.null_display = value;
                                this.mark_dirty(cx);
                            }))
                    )
            )
            .child(
                FormField::new("Truncate text at (characters)")
                    .child(
                        NumberInput::new("truncate-at")
                            .value(self.settings.results.truncate_text_at as f32)
                            .min(50.0)
                            .max(10000.0)
                            .step(50.0)
                            .on_change(cx.listener(|this, value: f32, cx| {
                                this.settings.results.truncate_text_at = value as u32;
                                this.mark_dirty(cx);
                            }))
                    )
            )
            .child(
                FormField::new("Copy format")
                    .child(
                        Select::new("copy-format")
                            .options(vec![
                                SelectOption::new("tsv", "Tab-separated (TSV)"),
                                SelectOption::new("csv", "Comma-separated (CSV)"),
                                SelectOption::new("json", "JSON"),
                                SelectOption::new("markdown", "Markdown"),
                            ])
                            .selected(match self.settings.results.copy_format {
                                CopyFormat::Tsv => "tsv",
                                CopyFormat::Csv => "csv",
                                CopyFormat::Json => "json",
                                CopyFormat::Markdown => "markdown",
                            })
                            .on_change(cx.listener(|this, value: &str, cx| {
                                this.settings.results.copy_format = match value {
                                    "csv" => CopyFormat::Csv,
                                    "json" => CopyFormat::Json,
                                    "markdown" => CopyFormat::Markdown,
                                    _ => CopyFormat::Tsv,
                                };
                                this.mark_dirty(cx);
                            }))
                    )
            )
    }

    fn render_query_settings(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.colors.text)
                    .child("Query Execution")
            )
            .child(
                FormField::new("Default statement timeout")
                    .description("Leave empty for no timeout")
                    .child(
                        NumberInput::new("timeout")
                            .value(self.settings.query_execution.default_timeout_ms.unwrap_or(0) as f32)
                            .min(0.0)
                            .max(300000.0)
                            .step(1000.0)
                            .suffix("ms")
                            .on_change(cx.listener(|this, value: f32, cx| {
                                this.settings.query_execution.default_timeout_ms =
                                    if value > 0.0 { Some(value as u64) } else { None };
                                this.mark_dirty(cx);
                            }))
                    )
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        Checkbox::new("confirm-ddl")
                            .label("Confirm DDL statements (CREATE, ALTER, DROP)")
                            .checked(self.settings.query_execution.confirm_ddl)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.settings.query_execution.confirm_ddl = checked;
                                this.mark_dirty(cx);
                            }))
                    )
                    .child(
                        Checkbox::new("confirm-destructive")
                            .label("Confirm destructive statements (DELETE without WHERE, TRUNCATE)")
                            .checked(self.settings.query_execution.confirm_destructive)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.settings.query_execution.confirm_destructive = checked;
                                this.mark_dirty(cx);
                            }))
                    )
                    .child(
                        Checkbox::new("auto-uppercase")
                            .label("Auto-uppercase SQL keywords")
                            .checked(self.settings.query_execution.auto_uppercase_keywords)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.settings.query_execution.auto_uppercase_keywords = checked;
                                this.mark_dirty(cx);
                            }))
                    )
                    .child(
                        Checkbox::new("auto-limit")
                            .label("Auto-add LIMIT to SELECT queries without LIMIT")
                            .checked(self.settings.query_execution.auto_limit_select)
                            .on_change(cx.listener(|this, checked: bool, cx| {
                                this.settings.query_execution.auto_limit_select = checked;
                                this.mark_dirty(cx);
                            }))
                    )
            )
            .child(
                FormField::new("EXPLAIN output format")
                    .child(
                        Select::new("explain-format")
                            .options(vec![
                                SelectOption::new("text", "Text"),
                                SelectOption::new("json", "JSON"),
                                SelectOption::new("yaml", "YAML"),
                                SelectOption::new("xml", "XML"),
                            ])
                            .selected(match self.settings.query_execution.explain_format {
                                ExplainFormat::Text => "text",
                                ExplainFormat::Json => "json",
                                ExplainFormat::Yaml => "yaml",
                                ExplainFormat::Xml => "xml",
                            })
                            .on_change(cx.listener(|this, value: &str, cx| {
                                this.settings.query_execution.explain_format = match value {
                                    "json" => ExplainFormat::Json,
                                    "yaml" => ExplainFormat::Yaml,
                                    "xml" => ExplainFormat::Xml,
                                    _ => ExplainFormat::Text,
                                };
                                this.mark_dirty(cx);
                            }))
                    )
            )
    }

    fn render_connections_settings(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.colors.text)
                    .child("Connections")
            )
            .child(
                FormField::new("Default SSL mode")
                    .child(
                        Select::new("ssl-mode")
                            .options(vec![
                                SelectOption::new("disable", "Disable"),
                                SelectOption::new("prefer", "Prefer"),
                                SelectOption::new("require", "Require"),
                                SelectOption::new("verify-ca", "Verify CA"),
                                SelectOption::new("verify-full", "Verify Full"),
                            ])
                            .selected(match self.settings.connections.default_ssl_mode {
                                SslMode::Disable => "disable",
                                SslMode::Prefer => "prefer",
                                SslMode::Require => "require",
                                SslMode::VerifyCa => "verify-ca",
                                SslMode::VerifyFull => "verify-full",
                            })
                            .on_change(cx.listener(|this, value: &str, cx| {
                                this.settings.connections.default_ssl_mode = match value {
                                    "disable" => SslMode::Disable,
                                    "require" => SslMode::Require,
                                    "verify-ca" => SslMode::VerifyCa,
                                    "verify-full" => SslMode::VerifyFull,
                                    _ => SslMode::Prefer,
                                };
                                this.mark_dirty(cx);
                            }))
                    )
            )
            .child(
                FormField::new("Connection timeout")
                    .child(
                        NumberInput::new("connect-timeout")
                            .value(self.settings.connections.default_connect_timeout_sec as f32)
                            .min(1.0)
                            .max(120.0)
                            .step(1.0)
                            .suffix("seconds")
                            .on_change(cx.listener(|this, value: f32, cx| {
                                this.settings.connections.default_connect_timeout_sec = value as u64;
                                this.mark_dirty(cx);
                            }))
                    )
            )
            .child(
                FormField::new("Auto-reconnect attempts")
                    .child(
                        NumberInput::new("reconnect-attempts")
                            .value(self.settings.connections.auto_reconnect_attempts as f32)
                            .min(0.0)
                            .max(10.0)
                            .step(1.0)
                            .on_change(cx.listener(|this, value: f32, cx| {
                                this.settings.connections.auto_reconnect_attempts = value as u32;
                                this.mark_dirty(cx);
                            }))
                    )
            )
            .child(
                FormField::new("Keepalive interval")
                    .child(
                        NumberInput::new("keepalive")
                            .value(self.settings.connections.keepalive_interval_sec as f32)
                            .min(0.0)
                            .max(300.0)
                            .step(10.0)
                            .suffix("seconds")
                            .on_change(cx.listener(|this, value: f32, cx| {
                                this.settings.connections.keepalive_interval_sec = value as u64;
                                this.mark_dirty(cx);
                            }))
                    )
            )
            .child(
                Checkbox::new("auto-reconnect")
                    .label("Automatically reconnect on connection loss")
                    .checked(self.settings.connections.auto_reconnect)
                    .on_change(cx.listener(|this, checked: bool, cx| {
                        this.settings.connections.auto_reconnect = checked;
                        this.mark_dirty(cx);
                    }))
            )
    }

    fn render_shortcuts_settings(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        // Keyboard shortcuts configuration
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.colors.text)
                    .child("Keyboard Shortcuts")
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.colors.text_muted)
                    .child("Keyboard shortcuts are configured through key bindings. See documentation for customization.")
            )
            // Shortcut categories would be rendered here
            // This integrates with GPUI's key binding system
    }

    fn render_footer(&self, theme: &Theme, cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_3()
            .border_t_1()
            .border_color(theme.colors.border)
            .child(
                Button::new("reset")
                    .label("Reset to Defaults")
                    .style(ButtonStyle::Ghost)
                    .on_click(cx.listener(|this, _, cx| {
                        this.reset_category(cx);
                    }))
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("cancel")
                            .label("Cancel")
                            .style(ButtonStyle::Secondary)
                            .on_click(cx.listener(|this, _, cx| {
                                this.cancel(cx);
                            }))
                    )
                    .child(
                        Button::new("save")
                            .label("Save")
                            .style(ButtonStyle::Primary)
                            .disabled(!self.is_dirty)
                            .on_click(cx.listener(|this, _, cx| {
                                this.save(cx);
                            }))
                    )
            )
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id("settings-panel")
            .key_context("SettingsPanel")
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.surface)
            .child(
                // Header
                div()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text)
                            .child("Settings")
                    )
            )
            .child(
                // Content
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    .child(self.render_sidebar(theme, cx))
                    .child(self.render_content(theme, cx))
            )
            .child(self.render_footer(theme, cx))
    }
}
```

### 5. Form Components

```rust
// src/components/forms/mod.rs

use gpui::*;
use crate::theme::Theme;

/// Form field wrapper with label and optional description.
pub struct FormField {
    label: SharedString,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    width: Option<DefiniteLength>,
}

impl FormField {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            description: None,
            children: Vec::new(),
            width: None,
        }
    }

    pub fn description(mut self, desc: impl Into<SharedString>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn w_24(mut self) -> Self {
        self.width = Some(px(96.0).into());
        self
    }

    pub fn flex_1(mut self) -> Self {
        self.width = None; // Will use flex-1 instead
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl IntoElement for FormField {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = // get from context;

        div()
            .flex()
            .flex_col()
            .gap_1()
            .when_some(self.width, |this, w| this.w(w))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.colors.text)
                    .child(self.label)
            )
            .when_some(self.description, |this, desc| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(theme.colors.text_muted)
                        .child(desc)
                )
            })
            .children(self.children)
    }
}

/// Checkbox input.
pub struct Checkbox {
    id: ElementId,
    label: Option<SharedString>,
    checked: bool,
    on_change: Option<Box<dyn Fn(bool, &mut WindowContext) + 'static>>,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            checked: false,
            on_change: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(bool, &mut WindowContext) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
}

impl IntoElement for Checkbox {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = // get from context;
        let checked = self.checked;

        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .child(
                div()
                    .size_4()
                    .rounded_sm()
                    .border_1()
                    .border_color(if checked { theme.colors.border_focused } else { theme.colors.border })
                    .bg(if checked { theme.colors.text_accent } else { theme.colors.surface })
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(checked, |this| {
                        this.child(Icon::new(IconName::Check).size(IconSize::XSmall).color(theme.colors.surface))
                    })
            )
            .when_some(self.label, |this, label| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(theme.colors.text)
                        .child(label)
                )
            })
    }
}

/// Number input with optional min/max/step.
pub struct NumberInput {
    id: ElementId,
    value: f32,
    min: Option<f32>,
    max: Option<f32>,
    step: f32,
    suffix: Option<SharedString>,
    on_change: Option<Box<dyn Fn(f32, &mut WindowContext) + 'static>>,
}

impl NumberInput {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: 0.0,
            min: None,
            max: None,
            step: 1.0,
            suffix: None,
            on_change: None,
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }

    pub fn min(mut self, min: f32) -> Self {
        self.min = Some(min);
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn suffix(mut self, suffix: impl Into<SharedString>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f32, &mut WindowContext) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
}
```

### 6. Key Bindings Integration

```rust
// src/keybindings.rs

use gpui::*;

/// Define application-wide key bindings.
pub fn register_bindings(cx: &mut AppContext) {
    // Global bindings
    cx.bind_keys([
        // Settings
        KeyBinding::new("cmd-,", OpenSettings, None),

        // Tabs
        KeyBinding::new("cmd-n", NewQueryTab, None),
        KeyBinding::new("cmd-w", CloseTab, None),
        KeyBinding::new("cmd-shift-]", NextTab, None),
        KeyBinding::new("cmd-shift-[", PreviousTab, None),

        // Sidebar
        KeyBinding::new("cmd-b", ToggleSidebar, None),

        // Navigation
        KeyBinding::new("cmd-1", FocusEditor, None),
        KeyBinding::new("cmd-2", FocusResults, None),
        KeyBinding::new("cmd-0", FocusSidebar, None),
    ]);

    // Editor context bindings
    cx.bind_keys([
        KeyBinding::new("cmd-enter", ExecuteQuery, Some("Editor")),
        KeyBinding::new("cmd-shift-enter", ExecuteAll, Some("Editor")),
        KeyBinding::new("cmd-.", CancelQuery, Some("Editor")),
        KeyBinding::new("cmd-shift-f", FormatSql, Some("Editor")),
        KeyBinding::new("cmd-s", Save, Some("Editor")),
        KeyBinding::new("cmd-/", ToggleComment, Some("Editor")),
        KeyBinding::new("cmd-f", Find, Some("Editor")),
        KeyBinding::new("cmd-h", Replace, Some("Editor")),
        KeyBinding::new("cmd-g", GoToLine, Some("Editor")),
    ]);

    // Results context bindings
    cx.bind_keys([
        KeyBinding::new("cmd-c", Copy, Some("ResultsGrid")),
        KeyBinding::new("cmd-a", SelectAll, Some("ResultsGrid")),
        KeyBinding::new("cmd-e", Export, Some("ResultsGrid")),
        KeyBinding::new("cmd-shift-e", ToggleEditMode, Some("ResultsGrid")),
    ]);
}

// Action definitions
actions!(
    tusk,
    [
        OpenSettings,
        NewQueryTab,
        CloseTab,
        NextTab,
        PreviousTab,
        ToggleSidebar,
        FocusEditor,
        FocusResults,
        FocusSidebar,
        ExecuteQuery,
        ExecuteAll,
        CancelQuery,
        FormatSql,
        Save,
        ToggleComment,
        Find,
        Replace,
        GoToLine,
        Copy,
        SelectAll,
        Export,
        ToggleEditMode,
    ]
);
```

## Acceptance Criteria

1. [ ] Theme system with light/dark/system modes
2. [ ] System theme detection and automatic switching
3. [ ] Theme persists across restarts
4. [ ] Settings panel with all categories
5. [ ] Settings persist to SQLite via StorageService
6. [ ] Passwords stored in OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
7. [ ] SSH passphrases stored in OS keychain
8. [ ] Credentials deleted when connection deleted
9. [ ] Key bindings work in appropriate contexts
10. [ ] Settings changes apply immediately (theme)
11. [ ] Cancel reverts unsaved changes
12. [ ] Reset defaults works per category

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_resolution() {
        // Light mode
        let theme = Theme::from_mode(ThemeMode::Light, WindowAppearance::Light);
        assert_eq!(theme.appearance, Appearance::Light);

        // Dark mode
        let theme = Theme::from_mode(ThemeMode::Dark, WindowAppearance::Light);
        assert_eq!(theme.appearance, Appearance::Dark);

        // System mode follows window
        let theme = Theme::from_mode(ThemeMode::System, WindowAppearance::Dark);
        assert_eq!(theme.appearance, Appearance::Dark);

        let theme = Theme::from_mode(ThemeMode::System, WindowAppearance::Light);
        assert_eq!(theme.appearance, Appearance::Light);
    }

    #[test]
    fn test_keyring_roundtrip() {
        // Skip if keyring not available (CI)
        if !KeyringService::is_available() {
            return;
        }

        let conn_id = "test-conn-123";
        let password = "secret-password";

        // Store
        KeyringService::store_password(conn_id, password).unwrap();

        // Retrieve
        let retrieved = KeyringService::get_password(conn_id).unwrap();
        assert_eq!(retrieved, Some(password.to_string()));

        // Delete
        KeyringService::delete_password(conn_id).unwrap();

        // Verify deleted
        let retrieved = KeyringService::get_password(conn_id).unwrap();
        assert_eq!(retrieved, None);
    }
}
```

## Dependencies on Other Features

- **03-frontend-architecture.md**: GPUI component patterns
- **04-ipc-layer.md**: TuskState global
- **05-local-storage.md**: Settings persistence

## Dependent Features

- **07-connection-management.md**: Connection settings, credential storage
- **12-sql-editor.md**: Editor settings
- **14-results-grid.md**: Results settings
- All features that use theme colors
