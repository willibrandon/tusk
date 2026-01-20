# Feature 27: Platform Integration

## Overview

This feature implements platform-specific integrations for macOS, Windows, and Linux. Each platform has unique requirements for native menu bars, keyboard shortcuts, credential storage (keychain), window management, file associations, auto-update, and distribution packaging. Tauri v2 provides the foundation, and this feature configures platform-specific behaviors to deliver a native experience on each OS.

## Goals

1. Native menu bar integration on each platform
2. Platform-appropriate keyboard shortcuts (Cmd vs Ctrl)
3. Secure credential storage using OS keychains
4. Native file dialogs and associations
5. Auto-update mechanism via Tauri updater
6. Platform-specific distribution packages
7. Respect system themes and accessibility settings
8. XDG compliance on Linux

## Dependencies

- Feature 01: Project Setup (Tauri configuration)
- Feature 02: Local Storage (config/data paths)
- Feature 06: Settings System (theme preferences)
- Feature 07: Connection Management (credential storage)

## Technical Specification

### 27.1 Platform Detection

**File: `src-tauri/src/platform.rs`**

```rust
use std::env;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    MacOS,
    Windows,
    Linux,
}

impl Platform {
    pub fn current() -> Self {
        #[cfg(target_os = "macos")]
        { Platform::MacOS }

        #[cfg(target_os = "windows")]
        { Platform::Windows }

        #[cfg(target_os = "linux")]
        { Platform::Linux }
    }

    pub fn is_macos(&self) -> bool {
        matches!(self, Platform::MacOS)
    }

    pub fn is_windows(&self) -> bool {
        matches!(self, Platform::Windows)
    }

    pub fn is_linux(&self) -> bool {
        matches!(self, Platform::Linux)
    }

    /// Get the primary modifier key name
    pub fn primary_modifier(&self) -> &'static str {
        match self {
            Platform::MacOS => "Cmd",
            Platform::Windows | Platform::Linux => "Ctrl",
        }
    }

    /// Get config directory path
    pub fn config_dir(&self) -> std::path::PathBuf {
        match self {
            Platform::MacOS => {
                dirs::home_dir()
                    .unwrap_or_default()
                    .join("Library")
                    .join("Application Support")
                    .join("Tusk")
            }
            Platform::Windows => {
                dirs::config_dir()
                    .unwrap_or_default()
                    .join("Tusk")
            }
            Platform::Linux => {
                // XDG_CONFIG_HOME or ~/.config
                env::var("XDG_CONFIG_HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| {
                        dirs::home_dir()
                            .unwrap_or_default()
                            .join(".config")
                    })
                    .join("tusk")
            }
        }
    }

    /// Get data directory path
    pub fn data_dir(&self) -> std::path::PathBuf {
        match self {
            Platform::MacOS => {
                dirs::home_dir()
                    .unwrap_or_default()
                    .join("Library")
                    .join("Application Support")
                    .join("Tusk")
            }
            Platform::Windows => {
                dirs::data_local_dir()
                    .unwrap_or_default()
                    .join("Tusk")
            }
            Platform::Linux => {
                // XDG_DATA_HOME or ~/.local/share
                env::var("XDG_DATA_HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| {
                        dirs::home_dir()
                            .unwrap_or_default()
                            .join(".local")
                            .join("share")
                    })
                    .join("tusk")
            }
        }
    }

    /// Get cache directory path
    pub fn cache_dir(&self) -> std::path::PathBuf {
        match self {
            Platform::MacOS => {
                dirs::home_dir()
                    .unwrap_or_default()
                    .join("Library")
                    .join("Caches")
                    .join("Tusk")
            }
            Platform::Windows => {
                dirs::cache_dir()
                    .unwrap_or_default()
                    .join("Tusk")
            }
            Platform::Linux => {
                // XDG_CACHE_HOME or ~/.cache
                env::var("XDG_CACHE_HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| {
                        dirs::home_dir()
                            .unwrap_or_default()
                            .join(".cache")
                    })
                    .join("tusk")
            }
        }
    }
}

/// Platform info for frontend
#[derive(Debug, Clone, Serialize)]
pub struct PlatformInfo {
    pub platform: Platform,
    pub os_version: String,
    pub arch: &'static str,
    pub primary_modifier: &'static str,
    pub secondary_modifier: &'static str,
    pub config_dir: String,
    pub data_dir: String,
}

impl PlatformInfo {
    pub fn current() -> Self {
        let platform = Platform::current();

        Self {
            platform,
            os_version: os_info::get().version().to_string(),
            arch: std::env::consts::ARCH,
            primary_modifier: platform.primary_modifier(),
            secondary_modifier: match platform {
                Platform::MacOS => "Option",
                _ => "Alt",
            },
            config_dir: platform.config_dir().to_string_lossy().to_string(),
            data_dir: platform.data_dir().to_string_lossy().to_string(),
        }
    }
}
```

### 27.2 Native Menu Bar

**File: `src-tauri/src/menu.rs`**

```rust
use tauri::{
    menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder, PredefinedMenuItem},
    AppHandle, Manager, Wry,
};
use crate::platform::Platform;
use crate::error::Result;

/// Build the native application menu
pub fn build_menu(app: &AppHandle) -> Result<Menu<Wry>> {
    let platform = Platform::current();

    let menu = MenuBuilder::new(app);

    // App menu (macOS only)
    #[cfg(target_os = "macos")]
    let menu = menu.item(&build_app_submenu(app)?);

    // File menu
    let menu = menu.item(&build_file_submenu(app, platform)?);

    // Edit menu
    let menu = menu.item(&build_edit_submenu(app)?);

    // Query menu
    let menu = menu.item(&build_query_submenu(app, platform)?);

    // View menu
    let menu = menu.item(&build_view_submenu(app, platform)?);

    // Tools menu
    let menu = menu.item(&build_tools_submenu(app)?);

    // Window menu
    let menu = menu.item(&build_window_submenu(app)?);

    // Help menu
    let menu = menu.item(&build_help_submenu(app)?);

    menu.build().map_err(Into::into)
}

#[cfg(target_os = "macos")]
fn build_app_submenu(app: &AppHandle) -> Result<tauri::menu::Submenu<Wry>> {
    SubmenuBuilder::new(app, "Tusk")
        .item(&PredefinedMenuItem::about(app, Some("About Tusk"), None)?)
        .separator()
        .item(&MenuItemBuilder::with_id("preferences", "Preferences...")
            .accelerator("Cmd+,")
            .build(app)?)
        .separator()
        .item(&PredefinedMenuItem::services(app, None)?)
        .separator()
        .item(&PredefinedMenuItem::hide(app, None)?)
        .item(&PredefinedMenuItem::hide_others(app, None)?)
        .item(&PredefinedMenuItem::show_all(app, None)?)
        .separator()
        .item(&PredefinedMenuItem::quit(app, None)?)
        .build()
        .map_err(Into::into)
}

fn build_file_submenu(app: &AppHandle, platform: Platform) -> Result<tauri::menu::Submenu<Wry>> {
    let modifier = if platform.is_macos() { "Cmd" } else { "Ctrl" };

    let mut builder = SubmenuBuilder::new(app, "File")
        .item(&MenuItemBuilder::with_id("new_query", "New Query Tab")
            .accelerator(&format!("{}+N", modifier))
            .build(app)?)
        .item(&MenuItemBuilder::with_id("new_connection", "New Connection...")
            .accelerator(&format!("{}+Shift+N", modifier))
            .build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("open_file", "Open SQL File...")
            .accelerator(&format!("{}+O", modifier))
            .build(app)?)
        .item(&MenuItemBuilder::with_id("save", "Save")
            .accelerator(&format!("{}+S", modifier))
            .build(app)?)
        .item(&MenuItemBuilder::with_id("save_as", "Save As...")
            .accelerator(&format!("{}+Shift+S", modifier))
            .build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("close_tab", "Close Tab")
            .accelerator(&format!("{}+W", modifier))
            .build(app)?);

    // Add preferences on Windows/Linux (macOS has it in app menu)
    if !platform.is_macos() {
        builder = builder
            .separator()
            .item(&MenuItemBuilder::with_id("preferences", "Preferences...")
                .accelerator(&format!("{}+,", modifier))
                .build(app)?);
    }

    // Add exit on Windows/Linux
    if !platform.is_macos() {
        builder = builder
            .separator()
            .item(&MenuItemBuilder::with_id("exit", "Exit")
                .accelerator("Alt+F4")
                .build(app)?);
    }

    builder.build().map_err(Into::into)
}

fn build_edit_submenu(app: &AppHandle) -> Result<tauri::menu::Submenu<Wry>> {
    SubmenuBuilder::new(app, "Edit")
        .item(&PredefinedMenuItem::undo(app, None)?)
        .item(&PredefinedMenuItem::redo(app, None)?)
        .separator()
        .item(&PredefinedMenuItem::cut(app, None)?)
        .item(&PredefinedMenuItem::copy(app, None)?)
        .item(&PredefinedMenuItem::paste(app, None)?)
        .item(&PredefinedMenuItem::select_all(app, None)?)
        .separator()
        .item(&MenuItemBuilder::with_id("find", "Find...")
            .accelerator(if Platform::current().is_macos() { "Cmd+F" } else { "Ctrl+F" })
            .build(app)?)
        .item(&MenuItemBuilder::with_id("replace", "Replace...")
            .accelerator(if Platform::current().is_macos() { "Cmd+Option+F" } else { "Ctrl+H" })
            .build(app)?)
        .build()
        .map_err(Into::into)
}

fn build_query_submenu(app: &AppHandle, platform: Platform) -> Result<tauri::menu::Submenu<Wry>> {
    let modifier = if platform.is_macos() { "Cmd" } else { "Ctrl" };

    SubmenuBuilder::new(app, "Query")
        .item(&MenuItemBuilder::with_id("execute", "Execute Statement")
            .accelerator(&format!("{}+Enter", modifier))
            .build(app)?)
        .item(&MenuItemBuilder::with_id("execute_all", "Execute All")
            .accelerator(&format!("{}+Shift+Enter", modifier))
            .build(app)?)
        .item(&MenuItemBuilder::with_id("cancel", "Cancel Query")
            .accelerator(&format!("{}+.", modifier))
            .build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("format", "Format SQL")
            .accelerator(&format!("{}+Shift+F", modifier))
            .build(app)?)
        .item(&MenuItemBuilder::with_id("explain", "Explain Plan")
            .accelerator(&format!("{}+E", modifier))
            .build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("comment", "Toggle Comment")
            .accelerator(&format!("{}+/", modifier))
            .build(app)?)
        .build()
        .map_err(Into::into)
}

fn build_view_submenu(app: &AppHandle, platform: Platform) -> Result<tauri::menu::Submenu<Wry>> {
    let modifier = if platform.is_macos() { "Cmd" } else { "Ctrl" };

    SubmenuBuilder::new(app, "View")
        .item(&MenuItemBuilder::with_id("toggle_sidebar", "Toggle Sidebar")
            .accelerator(&format!("{}+B", modifier))
            .build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("focus_editor", "Focus Editor")
            .accelerator(&format!("{}+1", modifier))
            .build(app)?)
        .item(&MenuItemBuilder::with_id("focus_results", "Focus Results")
            .accelerator(&format!("{}+2", modifier))
            .build(app)?)
        .item(&MenuItemBuilder::with_id("focus_sidebar", "Focus Sidebar")
            .accelerator(&format!("{}+0", modifier))
            .build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("zoom_in", "Zoom In")
            .accelerator(&format!("{}+=", modifier))
            .build(app)?)
        .item(&MenuItemBuilder::with_id("zoom_out", "Zoom Out")
            .accelerator(&format!("{}+-", modifier))
            .build(app)?)
        .item(&MenuItemBuilder::with_id("zoom_reset", "Reset Zoom")
            .accelerator(&format!("{}+0", modifier))
            .build(app)?)
        .separator()
        .item(&PredefinedMenuItem::fullscreen(app, None)?)
        .build()
        .map_err(Into::into)
}

fn build_tools_submenu(app: &AppHandle) -> Result<tauri::menu::Submenu<Wry>> {
    SubmenuBuilder::new(app, "Tools")
        .item(&MenuItemBuilder::with_id("backup_db", "Backup Database...")
            .build(app)?)
        .item(&MenuItemBuilder::with_id("restore_db", "Restore Database...")
            .build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("import_wizard", "Import Wizard...")
            .build(app)?)
        .item(&MenuItemBuilder::with_id("export_data", "Export Data...")
            .build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("er_diagram", "ER Diagram...")
            .build(app)?)
        .build()
        .map_err(Into::into)
}

fn build_window_submenu(app: &AppHandle) -> Result<tauri::menu::Submenu<Wry>> {
    let platform = Platform::current();

    let mut builder = SubmenuBuilder::new(app, "Window")
        .item(&PredefinedMenuItem::minimize(app, None)?);

    #[cfg(target_os = "macos")]
    {
        builder = builder.item(&PredefinedMenuItem::zoom(app, None)?);
    }

    builder = builder
        .separator()
        .item(&MenuItemBuilder::with_id("next_tab", "Next Tab")
            .accelerator(if platform.is_macos() { "Cmd+Shift+]" } else { "Ctrl+Tab" })
            .build(app)?)
        .item(&MenuItemBuilder::with_id("prev_tab", "Previous Tab")
            .accelerator(if platform.is_macos() { "Cmd+Shift+[" } else { "Ctrl+Shift+Tab" })
            .build(app)?);

    builder.build().map_err(Into::into)
}

fn build_help_submenu(app: &AppHandle) -> Result<tauri::menu::Submenu<Wry>> {
    SubmenuBuilder::new(app, "Help")
        .item(&MenuItemBuilder::with_id("documentation", "Documentation")
            .build(app)?)
        .item(&MenuItemBuilder::with_id("keyboard_shortcuts", "Keyboard Shortcuts")
            .build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("check_updates", "Check for Updates...")
            .build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("about", "About Tusk")
            .build(app)?)
        .build()
        .map_err(Into::into)
}

/// Handle menu item clicks
pub fn handle_menu_event(app: &AppHandle, event_id: &str) {
    match event_id {
        "preferences" => {
            app.emit("menu:preferences", ()).ok();
        }
        "new_query" => {
            app.emit("menu:new-query", ()).ok();
        }
        "new_connection" => {
            app.emit("menu:new-connection", ()).ok();
        }
        "execute" => {
            app.emit("menu:execute", ()).ok();
        }
        "execute_all" => {
            app.emit("menu:execute-all", ()).ok();
        }
        "cancel" => {
            app.emit("menu:cancel", ()).ok();
        }
        "format" => {
            app.emit("menu:format", ()).ok();
        }
        "toggle_sidebar" => {
            app.emit("menu:toggle-sidebar", ()).ok();
        }
        "backup_db" => {
            app.emit("menu:backup", ()).ok();
        }
        "restore_db" => {
            app.emit("menu:restore", ()).ok();
        }
        "import_wizard" => {
            app.emit("menu:import", ()).ok();
        }
        "er_diagram" => {
            app.emit("menu:er-diagram", ()).ok();
        }
        "check_updates" => {
            app.emit("menu:check-updates", ()).ok();
        }
        "about" => {
            app.emit("menu:about", ()).ok();
        }
        _ => {}
    }
}
```

### 27.3 Credential Storage (Keychain)

**File: `src-tauri/src/services/keychain.rs`**

```rust
use keyring::Entry;
use crate::error::{Result, TuskError};
use crate::platform::Platform;

const SERVICE_NAME: &str = "tusk-postgres-client";

pub struct KeychainService {
    platform: Platform,
}

impl KeychainService {
    pub fn new() -> Self {
        Self {
            platform: Platform::current(),
        }
    }

    /// Store a password in the system keychain
    pub fn store_password(&self, connection_id: &str, password: &str) -> Result<()> {
        let key = self.build_key(connection_id);
        let entry = Entry::new(SERVICE_NAME, &key)
            .map_err(|e| TuskError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        entry.set_password(password)
            .map_err(|e| TuskError::Keychain(format!("Failed to store password: {}", e)))?;

        Ok(())
    }

    /// Retrieve a password from the system keychain
    pub fn get_password(&self, connection_id: &str) -> Result<Option<String>> {
        let key = self.build_key(connection_id);
        let entry = Entry::new(SERVICE_NAME, &key)
            .map_err(|e| TuskError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(TuskError::Keychain(format!("Failed to retrieve password: {}", e))),
        }
    }

    /// Delete a password from the system keychain
    pub fn delete_password(&self, connection_id: &str) -> Result<()> {
        let key = self.build_key(connection_id);
        let entry = Entry::new(SERVICE_NAME, &key)
            .map_err(|e| TuskError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
            Err(e) => Err(TuskError::Keychain(format!("Failed to delete password: {}", e))),
        }
    }

    /// Store SSH key passphrase
    pub fn store_ssh_passphrase(&self, key_path: &str, passphrase: &str) -> Result<()> {
        let key = format!("ssh:{}", key_path);
        let entry = Entry::new(SERVICE_NAME, &key)
            .map_err(|e| TuskError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        entry.set_password(passphrase)
            .map_err(|e| TuskError::Keychain(format!("Failed to store SSH passphrase: {}", e)))?;

        Ok(())
    }

    /// Get SSH key passphrase
    pub fn get_ssh_passphrase(&self, key_path: &str) -> Result<Option<String>> {
        let key = format!("ssh:{}", key_path);
        let entry = Entry::new(SERVICE_NAME, &key)
            .map_err(|e| TuskError::Keychain(format!("Failed to create keychain entry: {}", e)))?;

        match entry.get_password() {
            Ok(passphrase) => Ok(Some(passphrase)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(TuskError::Keychain(format!("Failed to retrieve SSH passphrase: {}", e))),
        }
    }

    /// Check if keychain is available
    pub fn is_available(&self) -> bool {
        // Try to create a test entry
        let test_entry = Entry::new(SERVICE_NAME, "test-availability");
        test_entry.is_ok()
    }

    /// Get keychain backend name for display
    pub fn backend_name(&self) -> &'static str {
        match self.platform {
            Platform::MacOS => "macOS Keychain",
            Platform::Windows => "Windows Credential Manager",
            Platform::Linux => "Secret Service (GNOME Keyring / KWallet)",
        }
    }

    fn build_key(&self, connection_id: &str) -> String {
        format!("connection:{}", connection_id)
    }
}

impl Default for KeychainService {
    fn default() -> Self {
        Self::new()
    }
}
```

**File: `src-tauri/src/commands/keychain.rs`**

```rust
use crate::services::keychain::KeychainService;
use crate::error::Result;
use tauri::State;

/// Store password in keychain
#[tauri::command]
pub async fn keychain_store(
    connection_id: String,
    password: String,
) -> Result<()> {
    let service = KeychainService::new();
    service.store_password(&connection_id, &password)
}

/// Get password from keychain
#[tauri::command]
pub async fn keychain_get(
    connection_id: String,
) -> Result<Option<String>> {
    let service = KeychainService::new();
    service.get_password(&connection_id)
}

/// Delete password from keychain
#[tauri::command]
pub async fn keychain_delete(
    connection_id: String,
) -> Result<()> {
    let service = KeychainService::new();
    service.delete_password(&connection_id)
}

/// Check if keychain is available
#[tauri::command]
pub async fn keychain_available() -> bool {
    let service = KeychainService::new();
    service.is_available()
}

/// Get keychain backend name
#[tauri::command]
pub async fn keychain_backend() -> String {
    let service = KeychainService::new();
    service.backend_name().to_string()
}
```

### 27.4 Auto-Update Configuration

**File: `src-tauri/tauri.conf.json`** (updater section)

```json
{
	"plugins": {
		"updater": {
			"active": true,
			"endpoints": ["https://releases.tusk-app.dev/{{target}}/{{arch}}/{{current_version}}"],
			"dialog": true,
			"pubkey": "YOUR_PUBLIC_KEY_HERE",
			"windows": {
				"installMode": "passive"
			}
		}
	}
}
```

**File: `src-tauri/src/services/updater.rs`**

```rust
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;
use crate::error::{Result, TuskError};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
    pub pub_date: String,
    pub download_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResult {
    pub available: bool,
    pub current_version: String,
    pub update: Option<UpdateInfo>,
}

pub struct UpdateService {
    app: AppHandle,
}

impl UpdateService {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    /// Check for available updates
    pub async fn check_for_updates(&self) -> Result<UpdateCheckResult> {
        let current_version = self.app.package_info().version.to_string();

        let updater = self.app.updater()
            .map_err(|e| TuskError::Update(format!("Failed to get updater: {}", e)))?;

        match updater.check().await {
            Ok(Some(update)) => {
                Ok(UpdateCheckResult {
                    available: true,
                    current_version,
                    update: Some(UpdateInfo {
                        version: update.version.clone(),
                        notes: update.body.clone().unwrap_or_default(),
                        pub_date: update.date.map(|d| d.to_string()).unwrap_or_default(),
                        download_url: String::new(), // Not exposed by tauri-plugin-updater
                    }),
                })
            }
            Ok(None) => {
                Ok(UpdateCheckResult {
                    available: false,
                    current_version,
                    update: None,
                })
            }
            Err(e) => {
                Err(TuskError::Update(format!("Update check failed: {}", e)))
            }
        }
    }

    /// Download and install update
    pub async fn install_update(&self) -> Result<()> {
        let updater = self.app.updater()
            .map_err(|e| TuskError::Update(format!("Failed to get updater: {}", e)))?;

        let update = updater.check().await
            .map_err(|e| TuskError::Update(format!("Update check failed: {}", e)))?
            .ok_or_else(|| TuskError::Update("No update available".into()))?;

        // Emit progress events
        let app = self.app.clone();
        update.download_and_install(
            move |chunk_length, content_length| {
                let progress = content_length.map(|total| {
                    (chunk_length as f64 / total as f64 * 100.0) as u32
                }).unwrap_or(0);

                app.emit("update:progress", progress).ok();
            },
            || {
                // Download complete
            }
        ).await.map_err(|e| TuskError::Update(format!("Update install failed: {}", e)))?;

        Ok(())
    }
}
```

**File: `src-tauri/src/commands/updater.rs`**

```rust
use crate::services::updater::{UpdateService, UpdateCheckResult};
use crate::error::Result;
use tauri::{AppHandle, State};

/// Check for updates
#[tauri::command]
pub async fn check_updates(app: AppHandle) -> Result<UpdateCheckResult> {
    let service = UpdateService::new(app);
    service.check_for_updates().await
}

/// Install available update
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<()> {
    let service = UpdateService::new(app);
    service.install_update().await
}
```

### 27.5 Tauri Configuration

**File: `src-tauri/tauri.conf.json`**

```json
{
	"$schema": "https://schema.tauri.app/config/2",
	"productName": "Tusk",
	"version": "0.1.0",
	"identifier": "dev.tusk.postgres",
	"build": {
		"beforeDevCommand": "npm run dev",
		"devUrl": "http://localhost:1420",
		"beforeBuildCommand": "npm run build",
		"frontendDist": "../dist"
	},
	"app": {
		"withGlobalTauri": false,
		"windows": [
			{
				"title": "Tusk",
				"width": 1280,
				"height": 800,
				"minWidth": 800,
				"minHeight": 600,
				"center": true,
				"resizable": true,
				"fullscreen": false,
				"decorations": true,
				"transparent": false,
				"titleBarStyle": "Visible"
			}
		],
		"security": {
			"csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:"
		}
	},
	"bundle": {
		"active": true,
		"targets": "all",
		"icon": [
			"icons/32x32.png",
			"icons/128x128.png",
			"icons/128x128@2x.png",
			"icons/icon.icns",
			"icons/icon.ico"
		],
		"resources": [],
		"copyright": "© 2024 Tusk",
		"category": "DeveloperTool",
		"shortDescription": "Fast, native Postgres client",
		"longDescription": "Tusk is a fast, free, native Postgres client built with Tauri. It aims to be a complete replacement for pgAdmin and DBeaver for Postgres-only workflows.",
		"macOS": {
			"entitlements": null,
			"exceptionDomain": "",
			"frameworks": [],
			"providerShortName": null,
			"signingIdentity": null,
			"minimumSystemVersion": "10.15"
		},
		"windows": {
			"certificateThumbprint": null,
			"digestAlgorithm": "sha256",
			"timestampUrl": "",
			"wix": {
				"language": "en-US"
			},
			"nsis": {
				"installerIcon": "icons/icon.ico",
				"headerImage": "icons/header.bmp",
				"sidebarImage": "icons/sidebar.bmp",
				"license": "LICENSE",
				"installMode": "currentUser",
				"languages": ["English"],
				"displayLanguageSelector": false
			}
		},
		"linux": {
			"appimage": {
				"bundleMediaFramework": false
			},
			"deb": {
				"depends": ["libssl3", "libwebkit2gtk-4.1-0"]
			},
			"rpm": {
				"depends": ["openssl", "webkit2gtk4.1"]
			}
		}
	}
}
```

### 27.6 Frontend Platform Integration

**File: `src/lib/stores/platform.ts`**

```typescript
import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type Platform = 'macos' | 'windows' | 'linux';

export interface PlatformInfo {
	platform: Platform;
	os_version: string;
	arch: string;
	primary_modifier: string;
	secondary_modifier: string;
	config_dir: string;
	data_dir: string;
}

function createPlatformStore() {
	const { subscribe, set } = writable<PlatformInfo | null>(null);

	return {
		subscribe,

		async init() {
			const info = await invoke<PlatformInfo>('get_platform_info');
			set(info);
			return info;
		}
	};
}

export const platformStore = createPlatformStore();

// Derived helpers
export const platform = derived(platformStore, ($p) => $p?.platform ?? 'linux');
export const isMac = derived(platform, ($p) => $p === 'macos');
export const isWindows = derived(platform, ($p) => $p === 'windows');
export const isLinux = derived(platform, ($p) => $p === 'linux');
export const primaryMod = derived(platformStore, ($p) => $p?.primary_modifier ?? 'Ctrl');
export const secondaryMod = derived(platformStore, ($p) => $p?.secondary_modifier ?? 'Alt');

// Format shortcut for display
export function formatShortcut(shortcut: string): string {
	let info: PlatformInfo | null = null;
	platformStore.subscribe((p) => (info = p))();

	if (!info) return shortcut;

	// Replace generic modifiers with platform-specific ones
	return shortcut
		.replace(/Mod\+/g, `${info.primary_modifier}+`)
		.replace(/Alt\+/g, `${info.secondary_modifier}+`);
}
```

**File: `src/lib/services/menu.ts`**

```typescript
import { listen } from '@tauri-apps/api/event';
import { goto } from '$app/navigation';
import { queryStore } from '$lib/stores/query';
import { uiStore } from '$lib/stores/ui';
import { dialogStore } from '$lib/stores/dialog';

export async function setupMenuListeners() {
	// File menu
	listen('menu:new-query', () => {
		queryStore.createTab();
	});

	listen('menu:new-connection', () => {
		dialogStore.open('connection');
	});

	listen('menu:preferences', () => {
		goto('/settings');
	});

	// Query menu
	listen('menu:execute', () => {
		queryStore.executeCurrent();
	});

	listen('menu:execute-all', () => {
		queryStore.executeAll();
	});

	listen('menu:cancel', () => {
		queryStore.cancelCurrent();
	});

	listen('menu:format', () => {
		queryStore.formatCurrent();
	});

	// View menu
	listen('menu:toggle-sidebar', () => {
		uiStore.toggleSidebar();
	});

	// Tools menu
	listen('menu:backup', () => {
		dialogStore.open('backup');
	});

	listen('menu:restore', () => {
		dialogStore.open('restore');
	});

	listen('menu:import', () => {
		dialogStore.open('import');
	});

	listen('menu:er-diagram', () => {
		dialogStore.open('er-diagram');
	});

	// Help menu
	listen('menu:check-updates', () => {
		dialogStore.open('update-check');
	});

	listen('menu:about', () => {
		dialogStore.open('about');
	});
}
```

**File: `src/lib/components/dialogs/UpdateDialog.svelte`**

```svelte
<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { onMount, onDestroy } from 'svelte';
	import Dialog from '$lib/components/common/Dialog.svelte';
	import Button from '$lib/components/common/Button.svelte';
	import { Download, CheckCircle, XCircle, Loader } from 'lucide-svelte';

	export let open = false;

	interface UpdateInfo {
		version: string;
		notes: string;
		pub_date: string;
	}

	interface UpdateCheckResult {
		available: boolean;
		current_version: string;
		update: UpdateInfo | null;
	}

	let checking = false;
	let installing = false;
	let result: UpdateCheckResult | null = null;
	let error: string | null = null;
	let progress = 0;

	let unlistenProgress: (() => void) | null = null;

	onMount(async () => {
		unlistenProgress = await listen<number>('update:progress', (event) => {
			progress = event.payload;
		});
	});

	onDestroy(() => {
		unlistenProgress?.();
	});

	async function checkForUpdates() {
		checking = true;
		error = null;

		try {
			result = await invoke<UpdateCheckResult>('check_updates');
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			checking = false;
		}
	}

	async function installUpdate() {
		installing = true;
		progress = 0;
		error = null;

		try {
			await invoke('install_update');
			// App will restart automatically
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			installing = false;
		}
	}

	$: if (open && !result && !checking) {
		checkForUpdates();
	}

	function close() {
		open = false;
		result = null;
		error = null;
		progress = 0;
	}
</script>

<Dialog bind:open title="Software Update" on:close={close}>
	<div class="update-content">
		{#if checking}
			<div class="status">
				<Loader class="spin" size={32} />
				<p>Checking for updates...</p>
			</div>
		{:else if error}
			<div class="status error">
				<XCircle size={32} />
				<p>Failed to check for updates</p>
				<span class="error-text">{error}</span>
			</div>
		{:else if result}
			{#if result.available && result.update}
				<div class="update-available">
					<h3>Update Available</h3>
					<p class="version-info">
						Version {result.update.version} is available (you have {result.current_version})
					</p>

					{#if result.update.notes}
						<div class="release-notes">
							<h4>Release Notes:</h4>
							<div class="notes-content">{result.update.notes}</div>
						</div>
					{/if}

					{#if installing}
						<div class="progress-container">
							<div class="progress-bar" style="width: {progress}%"></div>
							<span class="progress-text">{progress}%</span>
						</div>
						<p class="installing-text">Downloading and installing update...</p>
					{/if}
				</div>
			{:else}
				<div class="status success">
					<CheckCircle size={32} />
					<p>You're up to date!</p>
					<span class="version">Version {result.current_version}</span>
				</div>
			{/if}
		{/if}
	</div>

	<svelte:fragment slot="footer">
		{#if result?.available && !installing}
			<Button variant="ghost" on:click={close}>Later</Button>
			<Button variant="primary" on:click={installUpdate}>
				<Download size={16} />
				Install Update
			</Button>
		{:else if !checking && !installing}
			<Button variant="ghost" on:click={close}>Close</Button>
			<Button variant="primary" on:click={checkForUpdates}>Check Again</Button>
		{/if}
	</svelte:fragment>
</Dialog>

<style>
	.update-content {
		min-height: 150px;
		display: flex;
		flex-direction: column;
	}

	.status {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 12px;
		padding: 32px;
		text-align: center;
	}

	.status.error {
		color: var(--error-color);
	}

	.status.success {
		color: var(--success-color);
	}

	.error-text {
		color: var(--text-secondary);
		font-size: 13px;
	}

	.version {
		color: var(--text-secondary);
	}

	.update-available h3 {
		margin-bottom: 8px;
	}

	.version-info {
		color: var(--text-secondary);
		margin-bottom: 16px;
	}

	.release-notes {
		background: var(--bg-secondary);
		border-radius: 6px;
		padding: 12px;
		margin-bottom: 16px;
	}

	.release-notes h4 {
		font-size: 13px;
		margin-bottom: 8px;
	}

	.notes-content {
		font-size: 13px;
		color: var(--text-secondary);
		max-height: 150px;
		overflow-y: auto;
		white-space: pre-wrap;
	}

	.progress-container {
		height: 8px;
		background: var(--bg-tertiary);
		border-radius: 4px;
		overflow: hidden;
		position: relative;
	}

	.progress-bar {
		height: 100%;
		background: var(--primary-color);
		transition: width 0.3s ease;
	}

	.progress-text {
		position: absolute;
		right: 8px;
		top: 50%;
		transform: translateY(-50%);
		font-size: 11px;
		color: var(--text-secondary);
	}

	.installing-text {
		text-align: center;
		margin-top: 8px;
		color: var(--text-secondary);
		font-size: 13px;
	}

	:global(.spin) {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}
</style>
```

### 27.7 File Associations

**File: `src-tauri/tauri.conf.json`** (additional bundle config)

```json
{
	"bundle": {
		"fileAssociations": [
			{
				"ext": ["sql"],
				"name": "SQL File",
				"description": "SQL Script File",
				"role": "Editor",
				"mimeType": "application/sql"
			},
			{
				"ext": ["pgsql"],
				"name": "PostgreSQL File",
				"description": "PostgreSQL Script File",
				"role": "Editor",
				"mimeType": "application/x-postgresql"
			}
		]
	}
}
```

**File: `src-tauri/src/main.rs`** (file open handler)

```rust
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Handle file open on startup (macOS double-click)
            #[cfg(target_os = "macos")]
            {
                let handle = app.handle().clone();
                app.listen_global("tauri://file-drop", move |event| {
                    if let Some(paths) = event.payload() {
                        // Emit to frontend
                        handle.emit("file:open", paths).ok();
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 27.8 Platform Command

**File: `src-tauri/src/commands/platform.rs`**

```rust
use crate::platform::PlatformInfo;

/// Get platform information
#[tauri::command]
pub fn get_platform_info() -> PlatformInfo {
    PlatformInfo::current()
}
```

## Acceptance Criteria

1. **Menu Bar**
   - [ ] Native menu bar on macOS with app menu
   - [ ] File/Edit/Query/View/Tools/Window/Help menus
   - [ ] Platform-appropriate keyboard shortcuts displayed
   - [ ] Menu actions emit events to frontend

2. **Keyboard Shortcuts**
   - [ ] Cmd modifier on macOS
   - [ ] Ctrl modifier on Windows/Linux
   - [ ] Option/Alt secondary modifier
   - [ ] Shortcuts work when app focused

3. **Credential Storage**
   - [ ] macOS Keychain integration
   - [ ] Windows Credential Manager integration
   - [ ] Linux Secret Service (GNOME Keyring/KWallet)
   - [ ] SSH passphrase storage
   - [ ] Keychain availability check

4. **Auto-Update**
   - [ ] Check for updates on demand
   - [ ] Display release notes
   - [ ] Download with progress indicator
   - [ ] Install and restart

5. **File Associations**
   - [ ] .sql files open in Tusk
   - [ ] .pgsql files open in Tusk
   - [ ] Double-click opens existing instance

6. **XDG Compliance (Linux)**
   - [ ] Config in XDG_CONFIG_HOME
   - [ ] Data in XDG_DATA_HOME
   - [ ] Cache in XDG_CACHE_HOME

7. **Distribution**
   - [ ] macOS: .dmg with notarization
   - [ ] Windows: MSI/NSIS installer
   - [ ] Linux: AppImage, .deb, .rpm

## MCP Testing Instructions

### Using Tauri MCP

```typescript
// Test platform-specific functionality
await driver_session({ action: 'start', port: 9223 });

// Get platform info
const state = await ipc_get_backend_state({});
console.log('Platform:', state.platform);

// Test keychain
await ipc_execute_command({
	command: 'keychain_store',
	args: { connectionId: 'test', password: 'secret123' }
});

const password = await ipc_execute_command({
	command: 'keychain_get',
	args: { connectionId: 'test' }
});
console.log('Retrieved password:', password ? 'yes' : 'no');

// Check for updates
const updateResult = await ipc_execute_command({
	command: 'check_updates'
});
console.log('Update available:', updateResult.available);

// Test menu events
await ipc_emit_event({ eventName: 'menu:new-query' });
await webview_wait_for({ type: 'selector', value: '.query-tab' });

await driver_session({ action: 'stop' });
```

### Using Playwright MCP

```typescript
// Test keyboard shortcuts
await browser_navigate({ url: 'http://localhost:1420' });

// Execute query with keyboard shortcut
await browser_type({
	element: 'Query editor',
	ref: '.monaco-editor textarea',
	text: 'SELECT 1'
});

// Platform-specific execute shortcut
await browser_press_key({ key: 'Meta+Enter' }); // macOS
// await browser_press_key({ key: 'Control+Enter' }); // Windows/Linux

// Verify execution
await browser_wait_for({ text: '1 row' });

// Test settings shortcut
await browser_press_key({ key: 'Meta+,' });
await browser_wait_for({ text: 'Settings' });
```
