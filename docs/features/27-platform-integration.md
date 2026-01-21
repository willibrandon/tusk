# Feature 27: Platform Integration

## Overview

This feature implements platform-specific integrations for macOS, Windows, and Linux using GPUI's native capabilities. Each platform has unique requirements for native menu bars, keyboard shortcuts, credential storage (keychain), window management, file associations, auto-update, and distribution packaging. GPUI provides direct platform API access, and this feature configures platform-specific behaviors to deliver a native experience on each OS.

## Goals

1. Native menu bar integration on each platform
2. Platform-appropriate keyboard shortcuts (Cmd vs Ctrl)
3. Secure credential storage using OS keychains
4. Native file dialogs and associations
5. Auto-update mechanism with self-updating binary
6. Platform-specific distribution packages
7. Respect system themes and accessibility settings
8. XDG compliance on Linux

## Dependencies

- Feature 02: Backend Architecture (Rust services)
- Feature 05: Local Storage (config/data paths)
- Feature 06: Settings System (theme preferences)
- Feature 07: Connection Management (credential storage)

## Technical Specification

### 27.1 Platform Detection

**File: `src/platform/mod.rs`**

```rust
use std::env;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Get the secondary modifier key name
    pub fn secondary_modifier(&self) -> &'static str {
        match self {
            Platform::MacOS => "Option",
            Platform::Windows | Platform::Linux => "Alt",
        }
    }

    /// Get GPUI modifier for keybindings
    pub fn primary_modifier_gpui(&self) -> gpui::Modifiers {
        match self {
            Platform::MacOS => gpui::Modifiers {
                command: true,
                ..Default::default()
            },
            Platform::Windows | Platform::Linux => gpui::Modifiers {
                control: true,
                ..Default::default()
            },
        }
    }

    /// Get config directory path
    pub fn config_dir(&self) -> PathBuf {
        match self {
            Platform::MacOS => dirs::home_dir()
                .unwrap_or_default()
                .join("Library")
                .join("Application Support")
                .join("Tusk"),
            Platform::Windows => dirs::config_dir()
                .unwrap_or_default()
                .join("Tusk"),
            Platform::Linux => {
                // XDG_CONFIG_HOME or ~/.config
                env::var("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
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
    pub fn data_dir(&self) -> PathBuf {
        match self {
            Platform::MacOS => dirs::home_dir()
                .unwrap_or_default()
                .join("Library")
                .join("Application Support")
                .join("Tusk"),
            Platform::Windows => dirs::data_local_dir()
                .unwrap_or_default()
                .join("Tusk"),
            Platform::Linux => {
                // XDG_DATA_HOME or ~/.local/share
                env::var("XDG_DATA_HOME")
                    .map(PathBuf::from)
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
    pub fn cache_dir(&self) -> PathBuf {
        match self {
            Platform::MacOS => dirs::home_dir()
                .unwrap_or_default()
                .join("Library")
                .join("Caches")
                .join("Tusk"),
            Platform::Windows => dirs::cache_dir()
                .unwrap_or_default()
                .join("Tusk"),
            Platform::Linux => {
                // XDG_CACHE_HOME or ~/.cache
                env::var("XDG_CACHE_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| {
                        dirs::home_dir()
                            .unwrap_or_default()
                            .join(".cache")
                    })
                    .join("tusk")
            }
        }
    }

    /// Get log directory path
    pub fn log_dir(&self) -> PathBuf {
        match self {
            Platform::MacOS => dirs::home_dir()
                .unwrap_or_default()
                .join("Library")
                .join("Logs")
                .join("Tusk"),
            Platform::Windows => dirs::data_local_dir()
                .unwrap_or_default()
                .join("Tusk")
                .join("logs"),
            Platform::Linux => self.cache_dir().join("logs"),
        }
    }
}

/// Platform info for UI display
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub platform: Platform,
    pub os_version: String,
    pub arch: &'static str,
    pub primary_modifier: &'static str,
    pub secondary_modifier: &'static str,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl PlatformInfo {
    pub fn current() -> Self {
        let platform = Platform::current();

        Self {
            platform,
            os_version: Self::get_os_version(),
            arch: std::env::consts::ARCH,
            primary_modifier: platform.primary_modifier(),
            secondary_modifier: platform.secondary_modifier(),
            config_dir: platform.config_dir(),
            data_dir: platform.data_dir(),
            cache_dir: platform.cache_dir(),
        }
    }

    fn get_os_version() -> String {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            Command::new("sw_vers")
                .arg("-productVersion")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        }

        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            Command::new("cmd")
                .args(["/C", "ver"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        }

        #[cfg(target_os = "linux")]
        {
            std::fs::read_to_string("/etc/os-release")
                .ok()
                .and_then(|content| {
                    content
                        .lines()
                        .find(|l| l.starts_with("PRETTY_NAME="))
                        .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
                })
                .unwrap_or_else(|| "Linux".to_string())
        }
    }
}
```

### 27.2 Global Platform State

**File: `src/state/platform_state.rs`**

```rust
use crate::platform::{Platform, PlatformInfo};
use gpui::Global;
use parking_lot::RwLock;
use std::sync::Arc;

/// Application-wide platform state
pub struct PlatformState {
    inner: Arc<RwLock<PlatformStateInner>>,
}

struct PlatformStateInner {
    info: PlatformInfo,
    dark_mode: bool,
    high_contrast: bool,
    reduce_motion: bool,
}

impl Global for PlatformState {}

impl PlatformState {
    pub fn new() -> Self {
        let info = PlatformInfo::current();

        Self {
            inner: Arc::new(RwLock::new(PlatformStateInner {
                info,
                dark_mode: Self::detect_dark_mode(),
                high_contrast: Self::detect_high_contrast(),
                reduce_motion: Self::detect_reduce_motion(),
            })),
        }
    }

    pub fn info(&self) -> PlatformInfo {
        self.inner.read().info.clone()
    }

    pub fn platform(&self) -> Platform {
        self.inner.read().info.platform
    }

    pub fn is_dark_mode(&self) -> bool {
        self.inner.read().dark_mode
    }

    pub fn set_dark_mode(&self, dark: bool) {
        self.inner.write().dark_mode = dark;
    }

    pub fn is_high_contrast(&self) -> bool {
        self.inner.read().high_contrast
    }

    pub fn reduce_motion(&self) -> bool {
        self.inner.read().reduce_motion
    }

    pub fn primary_modifier(&self) -> &'static str {
        self.inner.read().info.primary_modifier
    }

    pub fn config_dir(&self) -> std::path::PathBuf {
        self.inner.read().info.config_dir.clone()
    }

    pub fn data_dir(&self) -> std::path::PathBuf {
        self.inner.read().info.data_dir.clone()
    }

    fn detect_dark_mode() -> bool {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            Command::new("defaults")
                .args(["read", "-g", "AppleInterfaceStyle"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).contains("Dark"))
                .unwrap_or(false)
        }

        #[cfg(target_os = "windows")]
        {
            // Read from Windows registry
            use winreg::enums::*;
            use winreg::RegKey;

            RegKey::predef(HKEY_CURRENT_USER)
                .open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")
                .and_then(|key| key.get_value::<u32, _>("AppsUseLightTheme"))
                .map(|v| v == 0) // 0 means dark mode
                .unwrap_or(false)
        }

        #[cfg(target_os = "linux")]
        {
            // Check GTK/Qt settings or environment
            std::env::var("GTK_THEME")
                .map(|t| t.to_lowercase().contains("dark"))
                .unwrap_or(false)
        }
    }

    fn detect_high_contrast() -> bool {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            Command::new("defaults")
                .args(["read", "-g", "AppleHighContrastEnabled"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
                .unwrap_or(false)
        }

        #[cfg(target_os = "windows")]
        {
            use winreg::enums::*;
            use winreg::RegKey;

            RegKey::predef(HKEY_CURRENT_USER)
                .open_subkey("Control Panel\\Accessibility\\HighContrast")
                .and_then(|key| key.get_value::<String, _>("Flags"))
                .map(|f| f.parse::<u32>().unwrap_or(0) & 1 != 0)
                .unwrap_or(false)
        }

        #[cfg(target_os = "linux")]
        {
            std::env::var("GTK_THEME")
                .map(|t| t.to_lowercase().contains("contrast"))
                .unwrap_or(false)
        }
    }

    fn detect_reduce_motion() -> bool {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            Command::new("defaults")
                .args(["read", "-g", "ReduceMotion"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
                .unwrap_or(false)
        }

        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
}

impl Default for PlatformState {
    fn default() -> Self {
        Self::new()
    }
}
```

### 27.3 Native Menu Bar

**File: `src/platform/menu.rs`**

```rust
use crate::platform::Platform;
use gpui::*;

/// Menu action identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    // File menu
    NewQuery,
    NewConnection,
    OpenFile,
    Save,
    SaveAs,
    CloseTab,
    Preferences,
    Quit,

    // Edit menu
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Find,
    Replace,

    // Query menu
    Execute,
    ExecuteAll,
    Cancel,
    Format,
    Explain,
    Comment,

    // View menu
    ToggleSidebar,
    FocusEditor,
    FocusResults,
    FocusSidebar,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Fullscreen,

    // Tools menu
    BackupDatabase,
    RestoreDatabase,
    ImportWizard,
    ExportData,
    ErDiagram,

    // Window menu
    Minimize,
    Zoom,
    NextTab,
    PreviousTab,

    // Help menu
    Documentation,
    KeyboardShortcuts,
    CheckUpdates,
    About,
}

/// Build application menus for GPUI
pub fn build_application_menus(cx: &mut App) -> Menu {
    let platform = Platform::current();
    let cmd = platform.is_macos();

    Menu::new("Tusk")
        // App menu (macOS)
        .when(platform.is_macos(), |menu| {
            menu.submenu(
                Submenu::new("Tusk")
                    .item(MenuItem::action("About Tusk", MenuAction::About))
                    .separator()
                    .item(MenuItem::action("Preferences...", MenuAction::Preferences)
                        .shortcut(if cmd { "cmd-," } else { "ctrl-," }))
                    .separator()
                    .item(MenuItem::os_action("Hide Tusk", OsAction::Hide))
                    .item(MenuItem::os_action("Hide Others", OsAction::HideOthers))
                    .item(MenuItem::os_action("Show All", OsAction::ShowAll))
                    .separator()
                    .item(MenuItem::action("Quit Tusk", MenuAction::Quit)
                        .shortcut(if cmd { "cmd-q" } else { "alt-f4" }))
            )
        })
        // File menu
        .submenu(
            Submenu::new("File")
                .item(MenuItem::action("New Query Tab", MenuAction::NewQuery)
                    .shortcut(if cmd { "cmd-n" } else { "ctrl-n" }))
                .item(MenuItem::action("New Connection...", MenuAction::NewConnection)
                    .shortcut(if cmd { "cmd-shift-n" } else { "ctrl-shift-n" }))
                .separator()
                .item(MenuItem::action("Open SQL File...", MenuAction::OpenFile)
                    .shortcut(if cmd { "cmd-o" } else { "ctrl-o" }))
                .item(MenuItem::action("Save", MenuAction::Save)
                    .shortcut(if cmd { "cmd-s" } else { "ctrl-s" }))
                .item(MenuItem::action("Save As...", MenuAction::SaveAs)
                    .shortcut(if cmd { "cmd-shift-s" } else { "ctrl-shift-s" }))
                .separator()
                .item(MenuItem::action("Close Tab", MenuAction::CloseTab)
                    .shortcut(if cmd { "cmd-w" } else { "ctrl-w" }))
                .when(!platform.is_macos(), |submenu| {
                    submenu
                        .separator()
                        .item(MenuItem::action("Preferences...", MenuAction::Preferences)
                            .shortcut("ctrl-,"))
                        .separator()
                        .item(MenuItem::action("Exit", MenuAction::Quit)
                            .shortcut("alt-f4"))
                })
        )
        // Edit menu
        .submenu(
            Submenu::new("Edit")
                .item(MenuItem::action("Undo", MenuAction::Undo)
                    .shortcut(if cmd { "cmd-z" } else { "ctrl-z" }))
                .item(MenuItem::action("Redo", MenuAction::Redo)
                    .shortcut(if cmd { "cmd-shift-z" } else { "ctrl-y" }))
                .separator()
                .item(MenuItem::action("Cut", MenuAction::Cut)
                    .shortcut(if cmd { "cmd-x" } else { "ctrl-x" }))
                .item(MenuItem::action("Copy", MenuAction::Copy)
                    .shortcut(if cmd { "cmd-c" } else { "ctrl-c" }))
                .item(MenuItem::action("Paste", MenuAction::Paste)
                    .shortcut(if cmd { "cmd-v" } else { "ctrl-v" }))
                .item(MenuItem::action("Select All", MenuAction::SelectAll)
                    .shortcut(if cmd { "cmd-a" } else { "ctrl-a" }))
                .separator()
                .item(MenuItem::action("Find...", MenuAction::Find)
                    .shortcut(if cmd { "cmd-f" } else { "ctrl-f" }))
                .item(MenuItem::action("Replace...", MenuAction::Replace)
                    .shortcut(if cmd { "cmd-alt-f" } else { "ctrl-h" }))
        )
        // Query menu
        .submenu(
            Submenu::new("Query")
                .item(MenuItem::action("Execute Statement", MenuAction::Execute)
                    .shortcut(if cmd { "cmd-enter" } else { "ctrl-enter" }))
                .item(MenuItem::action("Execute All", MenuAction::ExecuteAll)
                    .shortcut(if cmd { "cmd-shift-enter" } else { "ctrl-shift-enter" }))
                .item(MenuItem::action("Cancel Query", MenuAction::Cancel)
                    .shortcut(if cmd { "cmd-." } else { "ctrl-." }))
                .separator()
                .item(MenuItem::action("Format SQL", MenuAction::Format)
                    .shortcut(if cmd { "cmd-shift-f" } else { "ctrl-shift-f" }))
                .item(MenuItem::action("Explain Plan", MenuAction::Explain)
                    .shortcut(if cmd { "cmd-e" } else { "ctrl-e" }))
                .separator()
                .item(MenuItem::action("Toggle Comment", MenuAction::Comment)
                    .shortcut(if cmd { "cmd-/" } else { "ctrl-/" }))
        )
        // View menu
        .submenu(
            Submenu::new("View")
                .item(MenuItem::action("Toggle Sidebar", MenuAction::ToggleSidebar)
                    .shortcut(if cmd { "cmd-b" } else { "ctrl-b" }))
                .separator()
                .item(MenuItem::action("Focus Editor", MenuAction::FocusEditor)
                    .shortcut(if cmd { "cmd-1" } else { "ctrl-1" }))
                .item(MenuItem::action("Focus Results", MenuAction::FocusResults)
                    .shortcut(if cmd { "cmd-2" } else { "ctrl-2" }))
                .item(MenuItem::action("Focus Sidebar", MenuAction::FocusSidebar)
                    .shortcut(if cmd { "cmd-0" } else { "ctrl-0" }))
                .separator()
                .item(MenuItem::action("Zoom In", MenuAction::ZoomIn)
                    .shortcut(if cmd { "cmd-=" } else { "ctrl-=" }))
                .item(MenuItem::action("Zoom Out", MenuAction::ZoomOut)
                    .shortcut(if cmd { "cmd--" } else { "ctrl--" }))
                .item(MenuItem::action("Reset Zoom", MenuAction::ZoomReset)
                    .shortcut(if cmd { "cmd-0" } else { "ctrl-0" }))
                .separator()
                .item(MenuItem::action("Toggle Fullscreen", MenuAction::Fullscreen)
                    .shortcut(if cmd { "cmd-ctrl-f" } else { "f11" }))
        )
        // Tools menu
        .submenu(
            Submenu::new("Tools")
                .item(MenuItem::action("Backup Database...", MenuAction::BackupDatabase))
                .item(MenuItem::action("Restore Database...", MenuAction::RestoreDatabase))
                .separator()
                .item(MenuItem::action("Import Wizard...", MenuAction::ImportWizard))
                .item(MenuItem::action("Export Data...", MenuAction::ExportData))
                .separator()
                .item(MenuItem::action("ER Diagram...", MenuAction::ErDiagram))
        )
        // Window menu
        .submenu(
            Submenu::new("Window")
                .item(MenuItem::action("Minimize", MenuAction::Minimize)
                    .shortcut(if cmd { "cmd-m" } else { "" }))
                .when(platform.is_macos(), |submenu| {
                    submenu.item(MenuItem::action("Zoom", MenuAction::Zoom))
                })
                .separator()
                .item(MenuItem::action("Next Tab", MenuAction::NextTab)
                    .shortcut(if cmd { "cmd-shift-]" } else { "ctrl-tab" }))
                .item(MenuItem::action("Previous Tab", MenuAction::PreviousTab)
                    .shortcut(if cmd { "cmd-shift-[" } else { "ctrl-shift-tab" }))
        )
        // Help menu
        .submenu(
            Submenu::new("Help")
                .item(MenuItem::action("Documentation", MenuAction::Documentation))
                .item(MenuItem::action("Keyboard Shortcuts", MenuAction::KeyboardShortcuts))
                .separator()
                .item(MenuItem::action("Check for Updates...", MenuAction::CheckUpdates))
                .when(!platform.is_macos(), |submenu| {
                    submenu
                        .separator()
                        .item(MenuItem::action("About Tusk", MenuAction::About))
                })
        )
}

/// Handle menu actions
impl MenuAction {
    pub fn handle(self, cx: &mut App) {
        match self {
            // These are handled by the global action system
            MenuAction::Quit => cx.quit(),
            MenuAction::Minimize => {
                if let Some(window) = cx.active_window() {
                    window.minimize();
                }
            }
            MenuAction::Fullscreen => {
                if let Some(window) = cx.active_window() {
                    window.toggle_fullscreen();
                }
            }
            // Other actions are dispatched as GPUI actions
            _ => {
                cx.dispatch_action(Box::new(self));
            }
        }
    }
}

impl gpui::Action for MenuAction {
    fn name(&self) -> &'static str {
        match self {
            MenuAction::NewQuery => "tusk::new_query",
            MenuAction::NewConnection => "tusk::new_connection",
            MenuAction::OpenFile => "tusk::open_file",
            MenuAction::Save => "tusk::save",
            MenuAction::SaveAs => "tusk::save_as",
            MenuAction::CloseTab => "tusk::close_tab",
            MenuAction::Preferences => "tusk::preferences",
            MenuAction::Execute => "tusk::execute",
            MenuAction::ExecuteAll => "tusk::execute_all",
            MenuAction::Cancel => "tusk::cancel",
            MenuAction::Format => "tusk::format",
            MenuAction::Explain => "tusk::explain",
            MenuAction::Comment => "tusk::comment",
            MenuAction::ToggleSidebar => "tusk::toggle_sidebar",
            MenuAction::FocusEditor => "tusk::focus_editor",
            MenuAction::FocusResults => "tusk::focus_results",
            MenuAction::FocusSidebar => "tusk::focus_sidebar",
            MenuAction::ZoomIn => "tusk::zoom_in",
            MenuAction::ZoomOut => "tusk::zoom_out",
            MenuAction::ZoomReset => "tusk::zoom_reset",
            MenuAction::BackupDatabase => "tusk::backup",
            MenuAction::RestoreDatabase => "tusk::restore",
            MenuAction::ImportWizard => "tusk::import",
            MenuAction::ExportData => "tusk::export",
            MenuAction::ErDiagram => "tusk::er_diagram",
            MenuAction::NextTab => "tusk::next_tab",
            MenuAction::PreviousTab => "tusk::previous_tab",
            MenuAction::Documentation => "tusk::documentation",
            MenuAction::KeyboardShortcuts => "tusk::keyboard_shortcuts",
            MenuAction::CheckUpdates => "tusk::check_updates",
            MenuAction::About => "tusk::about",
            _ => "tusk::unknown",
        }
    }

    fn debug_name() -> &'static str
    where
        Self: Sized,
    {
        "MenuAction"
    }
}
```

### 27.4 Credential Storage (Keychain)

**File: `src/services/keychain.rs`**

```rust
use crate::error::{Result, TuskError};
use crate::platform::Platform;
use keyring::Entry;

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
            Err(e) => Err(TuskError::Keychain(format!(
                "Failed to retrieve SSH passphrase: {}",
                e
            ))),
        }
    }

    /// Check if keychain is available
    pub fn is_available(&self) -> bool {
        Entry::new(SERVICE_NAME, "test-availability").is_ok()
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

### 27.5 Auto-Update Service

**File: `src/services/updater.rs`**

```rust
use crate::error::{Result, TuskError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const UPDATE_URL: &str = "https://api.github.com/repos/willibrandon/tusk/releases/latest";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
    pub pub_date: String,
    pub download_url: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub available: bool,
    pub current_version: String,
    pub update: Option<UpdateInfo>,
}

#[derive(Debug, Clone)]
pub struct UpdateProgress {
    pub downloaded: u64,
    pub total: u64,
    pub percentage: f32,
}

pub struct UpdateService;

impl UpdateService {
    /// Check for available updates
    pub async fn check_for_updates() -> Result<UpdateCheckResult> {
        let current_version = env!("CARGO_PKG_VERSION").to_string();
        let target = Self::get_target();
        let arch = std::env::consts::ARCH;

        let url = format!(
            "{}/{}/{}/{}",
            UPDATE_URL, target, arch, current_version
        );

        let response = reqwest::get(&url).await
            .map_err(|e| TuskError::Update(format!("Failed to check for updates: {}", e)))?;

        if response.status() == 204 {
            // No update available
            return Ok(UpdateCheckResult {
                available: false,
                current_version,
                update: None,
            });
        }

        if !response.status().is_success() {
            return Err(TuskError::Update(format!(
                "Update server returned status: {}",
                response.status()
            )));
        }

        let update: UpdateInfo = response.json().await
            .map_err(|e| TuskError::Update(format!("Failed to parse update info: {}", e)))?;

        Ok(UpdateCheckResult {
            available: true,
            current_version,
            update: Some(update),
        })
    }

    /// Download update with progress callback
    pub async fn download_update<F>(
        update: &UpdateInfo,
        progress_callback: F,
    ) -> Result<PathBuf>
    where
        F: Fn(UpdateProgress) + Send + 'static,
    {
        let response = reqwest::get(&update.download_url).await
            .map_err(|e| TuskError::Update(format!("Failed to start download: {}", e)))?;

        let total = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;

        // Download to temp file
        let temp_dir = std::env::temp_dir();
        let file_name = format!("tusk-update-{}.bin", update.version);
        let download_path = temp_dir.join(&file_name);

        let mut file = tokio::fs::File::create(&download_path).await
            .map_err(|e| TuskError::Update(format!("Failed to create temp file: {}", e)))?;

        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk
                .map_err(|e| TuskError::Update(format!("Download error: {}", e)))?;

            file.write_all(&chunk).await
                .map_err(|e| TuskError::Update(format!("Write error: {}", e)))?;

            downloaded += chunk.len() as u64;

            progress_callback(UpdateProgress {
                downloaded,
                total,
                percentage: if total > 0 {
                    (downloaded as f32 / total as f32) * 100.0
                } else {
                    0.0
                },
            });
        }

        file.flush().await
            .map_err(|e| TuskError::Update(format!("Failed to flush file: {}", e)))?;

        // Verify signature
        Self::verify_signature(&download_path, &update.signature)?;

        Ok(download_path)
    }

    /// Install the downloaded update
    pub fn install_update(download_path: &PathBuf) -> Result<()> {
        let current_exe = std::env::current_exe()
            .map_err(|e| TuskError::Update(format!("Failed to get current exe: {}", e)))?;

        #[cfg(target_os = "macos")]
        {
            // macOS: Replace app bundle
            Self::install_macos_update(download_path, &current_exe)?;
        }

        #[cfg(target_os = "windows")]
        {
            // Windows: Run installer or replace exe
            Self::install_windows_update(download_path)?;
        }

        #[cfg(target_os = "linux")]
        {
            // Linux: Replace binary
            Self::install_linux_update(download_path, &current_exe)?;
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn install_macos_update(download_path: &PathBuf, current_exe: &PathBuf) -> Result<()> {
        use std::process::Command;

        // Get the app bundle path
        let app_path = current_exe
            .ancestors()
            .find(|p| p.extension().map_or(false, |e| e == "app"))
            .ok_or_else(|| TuskError::Update("Could not find app bundle".into()))?;

        // Create a script to replace the app after exit
        let script = format!(
            r#"#!/bin/bash
sleep 2
rm -rf "{}"
unzip -o "{}" -d "$(dirname "{}")"
open "{}"
rm "{}"
"#,
            app_path.display(),
            download_path.display(),
            app_path.display(),
            app_path.display(),
            download_path.display()
        );

        let script_path = std::env::temp_dir().join("tusk-update.sh");
        std::fs::write(&script_path, script)
            .map_err(|e| TuskError::Update(format!("Failed to write update script: {}", e)))?;

        Command::new("chmod")
            .args(["+x", &script_path.to_string_lossy()])
            .status()
            .map_err(|e| TuskError::Update(format!("Failed to chmod: {}", e)))?;

        Command::new("bash")
            .arg(&script_path)
            .spawn()
            .map_err(|e| TuskError::Update(format!("Failed to run update script: {}", e)))?;

        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn install_windows_update(download_path: &PathBuf) -> Result<()> {
        use std::process::Command;

        // Run the NSIS/MSI installer
        Command::new(download_path)
            .arg("/S") // Silent install
            .spawn()
            .map_err(|e| TuskError::Update(format!("Failed to run installer: {}", e)))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn install_linux_update(download_path: &PathBuf, current_exe: &PathBuf) -> Result<()> {
        use std::process::Command;

        // Create update script
        let script = format!(
            r#"#!/bin/bash
sleep 2
cp "{}" "{}"
chmod +x "{}"
"{}"
rm "{}"
"#,
            download_path.display(),
            current_exe.display(),
            current_exe.display(),
            current_exe.display(),
            download_path.display()
        );

        let script_path = std::env::temp_dir().join("tusk-update.sh");
        std::fs::write(&script_path, script)
            .map_err(|e| TuskError::Update(format!("Failed to write update script: {}", e)))?;

        Command::new("chmod")
            .args(["+x", &script_path.to_string_lossy()])
            .status()
            .map_err(|e| TuskError::Update(format!("Failed to chmod: {}", e)))?;

        Command::new("bash")
            .arg(&script_path)
            .spawn()
            .map_err(|e| TuskError::Update(format!("Failed to run update script: {}", e)))?;

        Ok(())
    }

    fn verify_signature(file_path: &PathBuf, _signature: &str) -> Result<()> {
        // TODO: Implement signature verification using ed25519
        // For now, just check file exists and has content
        let metadata = std::fs::metadata(file_path)
            .map_err(|e| TuskError::Update(format!("Failed to read download: {}", e)))?;

        if metadata.len() == 0 {
            return Err(TuskError::Update("Downloaded file is empty".into()));
        }

        Ok(())
    }

    fn get_target() -> &'static str {
        #[cfg(target_os = "macos")]
        { "darwin" }

        #[cfg(target_os = "windows")]
        { "windows" }

        #[cfg(target_os = "linux")]
        { "linux" }
    }
}
```

### 27.6 Update Dialog Component

**File: `src/ui/dialogs/update_dialog.rs`**

```rust
use crate::services::updater::{UpdateCheckResult, UpdateInfo, UpdateProgress, UpdateService};
use crate::ui::components::{Button, Icon, IconName, Modal, ProgressBar};
use gpui::*;

pub struct UpdateDialog {
    state: UpdateState,
    result: Option<UpdateCheckResult>,
    progress: Option<UpdateProgress>,
    error: Option<String>,
    focus_handle: FocusHandle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum UpdateState {
    Checking,
    NoUpdate,
    UpdateAvailable,
    Downloading,
    ReadyToInstall,
    Error,
}

pub enum UpdateDialogEvent {
    Close,
    InstallAndRestart,
}

impl EventEmitter<UpdateDialogEvent> for UpdateDialog {}

impl UpdateDialog {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let dialog = Self {
            state: UpdateState::Checking,
            result: None,
            progress: None,
            error: None,
            focus_handle: cx.focus_handle(),
        };

        // Start checking for updates
        cx.spawn(|this, mut cx| async move {
            match UpdateService::check_for_updates().await {
                Ok(result) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.result = Some(result.clone());
                        this.state = if result.available {
                            UpdateState::UpdateAvailable
                        } else {
                            UpdateState::NoUpdate
                        };
                        cx.notify();
                    });
                }
                Err(e) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.error = Some(e.to_string());
                        this.state = UpdateState::Error;
                        cx.notify();
                    });
                }
            }
        }).detach();

        dialog
    }

    fn download_update(&mut self, cx: &mut Context<Self>) {
        let Some(ref result) = self.result else { return };
        let Some(ref update) = result.update else { return };

        self.state = UpdateState::Downloading;
        self.progress = Some(UpdateProgress {
            downloaded: 0,
            total: 0,
            percentage: 0.0,
        });

        let update = update.clone();

        cx.spawn(|this, mut cx| async move {
            let progress_callback = {
                let this = this.clone();
                let cx = cx.clone();
                move |progress: UpdateProgress| {
                    let _ = this.update(&mut cx.clone(), |this, cx| {
                        this.progress = Some(progress);
                        cx.notify();
                    });
                }
            };

            match UpdateService::download_update(&update, progress_callback).await {
                Ok(path) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.state = UpdateState::ReadyToInstall;
                        cx.notify();
                    });

                    // Store path for installation
                    // In real impl, store this somewhere accessible
                }
                Err(e) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.error = Some(e.to_string());
                        this.state = UpdateState::Error;
                        cx.notify();
                    });
                }
            }
        }).detach();

        cx.notify();
    }

    fn install_and_restart(&mut self, cx: &mut Context<Self>) {
        // In real impl, get the download path and install
        // For now, just emit the event
        cx.emit(UpdateDialogEvent::InstallAndRestart);
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        cx.emit(UpdateDialogEvent::Close);
    }

    fn check_again(&mut self, cx: &mut Context<Self>) {
        self.state = UpdateState::Checking;
        self.error = None;
        self.result = None;

        cx.spawn(|this, mut cx| async move {
            match UpdateService::check_for_updates().await {
                Ok(result) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.result = Some(result.clone());
                        this.state = if result.available {
                            UpdateState::UpdateAvailable
                        } else {
                            UpdateState::NoUpdate
                        };
                        cx.notify();
                    });
                }
                Err(e) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.error = Some(e.to_string());
                        this.state = UpdateState::Error;
                        cx.notify();
                    });
                }
            }
        }).detach();

        cx.notify();
    }
}

impl Render for UpdateDialog {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.state {
            UpdateState::Checking => self.render_checking(cx),
            UpdateState::NoUpdate => self.render_no_update(cx),
            UpdateState::UpdateAvailable => self.render_update_available(cx),
            UpdateState::Downloading => self.render_downloading(cx),
            UpdateState::ReadyToInstall => self.render_ready_to_install(cx),
            UpdateState::Error => self.render_error(cx),
        };

        Modal::new("update-dialog")
            .title("Software Update")
            .width(px(450.0))
            .child(content)
            .footer(self.render_footer(cx))
    }
}

impl UpdateDialog {
    fn render_checking(&self, _cx: &Context<Self>) -> impl IntoElement {
        div()
            .py(px(32.0))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(12.0))
            .child(
                Icon::new(IconName::Loader)
                    .size(px(32.0))
                    .class("animate-spin")
            )
            .child(
                div()
                    .text_color(rgb(0x6b7280))
                    .child("Checking for updates...")
            )
    }

    fn render_no_update(&self, _cx: &Context<Self>) -> impl IntoElement {
        let version = self.result
            .as_ref()
            .map(|r| r.current_version.clone())
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

        div()
            .py(px(32.0))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(12.0))
            .child(
                Icon::new(IconName::CheckCircle)
                    .size(px(32.0))
                    .color(rgb(0x10b981))
            )
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .child("You're up to date!")
            )
            .child(
                div()
                    .text_color(rgb(0x6b7280))
                    .child(format!("Version {}", version))
            )
    }

    fn render_update_available(&self, _cx: &Context<Self>) -> impl IntoElement {
        let result = self.result.as_ref().unwrap();
        let update = result.update.as_ref().unwrap();

        div()
            .p(px(16.0))
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Update Available")
            )
            .child(
                div()
                    .text_color(rgb(0x6b7280))
                    .child(format!(
                        "Version {} is available (you have {})",
                        update.version, result.current_version
                    ))
            )
            .when(!update.notes.is_empty(), |this| {
                this.child(
                    div()
                        .bg(rgb(0xf9fafb))
                        .rounded_md()
                        .p(px(12.0))
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .mb(px(8.0))
                                .child("Release Notes:")
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x6b7280))
                                .max_h(px(150.0))
                                .overflow_y_auto()
                                .whitespace_pre_wrap()
                                .child(update.notes.clone())
                        )
                )
            })
    }

    fn render_downloading(&self, _cx: &Context<Self>) -> impl IntoElement {
        let progress = self.progress.as_ref().map(|p| p.percentage).unwrap_or(0.0);

        div()
            .p(px(16.0))
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Downloading Update...")
            )
            .child(ProgressBar::new(progress))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x6b7280))
                    .text_center()
                    .child(format!("{:.0}%", progress))
            )
    }

    fn render_ready_to_install(&self, _cx: &Context<Self>) -> impl IntoElement {
        div()
            .py(px(32.0))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(12.0))
            .child(
                Icon::new(IconName::Download)
                    .size(px(32.0))
                    .color(rgb(0x3b82f6))
            )
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .child("Ready to Install")
            )
            .child(
                div()
                    .text_color(rgb(0x6b7280))
                    .text_center()
                    .child("The update has been downloaded. Click Install to apply the update and restart Tusk.")
            )
    }

    fn render_error(&self, _cx: &Context<Self>) -> impl IntoElement {
        let error = self.error.as_deref().unwrap_or("Unknown error");

        div()
            .py(px(32.0))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(12.0))
            .child(
                Icon::new(IconName::XCircle)
                    .size(px(32.0))
                    .color(rgb(0xef4444))
            )
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .child("Update Failed")
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x6b7280))
                    .text_center()
                    .child(error.to_string())
            )
    }

    fn render_footer(&self, cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .justify_end()
            .gap(px(8.0))
            .children(match self.state {
                UpdateState::Checking => vec![],
                UpdateState::NoUpdate => vec![
                    Button::ghost()
                        .label("Close")
                        .on_click(cx.listener(|this, _, cx| this.close(cx)))
                        .into_any_element(),
                    Button::primary()
                        .label("Check Again")
                        .on_click(cx.listener(|this, _, cx| this.check_again(cx)))
                        .into_any_element(),
                ],
                UpdateState::UpdateAvailable => vec![
                    Button::ghost()
                        .label("Later")
                        .on_click(cx.listener(|this, _, cx| this.close(cx)))
                        .into_any_element(),
                    Button::primary()
                        .icon(IconName::Download)
                        .label("Download Update")
                        .on_click(cx.listener(|this, _, cx| this.download_update(cx)))
                        .into_any_element(),
                ],
                UpdateState::Downloading => vec![],
                UpdateState::ReadyToInstall => vec![
                    Button::ghost()
                        .label("Later")
                        .on_click(cx.listener(|this, _, cx| this.close(cx)))
                        .into_any_element(),
                    Button::primary()
                        .label("Install and Restart")
                        .on_click(cx.listener(|this, _, cx| this.install_and_restart(cx)))
                        .into_any_element(),
                ],
                UpdateState::Error => vec![
                    Button::ghost()
                        .label("Close")
                        .on_click(cx.listener(|this, _, cx| this.close(cx)))
                        .into_any_element(),
                    Button::primary()
                        .label("Try Again")
                        .on_click(cx.listener(|this, _, cx| this.check_again(cx)))
                        .into_any_element(),
                ],
            })
    }
}

impl FocusableView for UpdateDialog {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
```

### 27.7 About Dialog Component

**File: `src/ui/dialogs/about_dialog.rs`**

```rust
use crate::platform::PlatformInfo;
use crate::state::PlatformState;
use crate::ui::components::{Button, Modal};
use gpui::*;

pub struct AboutDialog {
    focus_handle: FocusHandle,
}

pub enum AboutDialogEvent {
    Close,
}

impl EventEmitter<AboutDialogEvent> for AboutDialog {}

impl AboutDialog {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        cx.emit(AboutDialogEvent::Close);
    }
}

impl Render for AboutDialog {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let platform_state = cx.global::<PlatformState>();
        let info = platform_state.info();

        let version = env!("CARGO_PKG_VERSION");
        let authors = env!("CARGO_PKG_AUTHORS");

        Modal::new("about-dialog")
            .title("")
            .width(px(400.0))
            .child(
                div()
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(16.0))
                    // App icon
                    .child(
                        div()
                            .w(px(80.0))
                            .h(px(80.0))
                            .rounded_xl()
                            .bg(rgb(0x3b82f6))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_4xl()
                                    .text_color(rgb(0xffffff))
                                    .child("🐘")
                            )
                    )
                    // App name
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .child("Tusk")
                    )
                    // Version
                    .child(
                        div()
                            .text_color(rgb(0x6b7280))
                            .child(format!("Version {}", version))
                    )
                    // Description
                    .child(
                        div()
                            .text_center()
                            .text_color(rgb(0x6b7280))
                            .child("A fast, free, native PostgreSQL client")
                    )
                    // System info
                    .child(
                        div()
                            .w_full()
                            .mt(px(8.0))
                            .p(px(12.0))
                            .bg(rgb(0xf9fafb))
                            .rounded_md()
                            .text_xs()
                            .text_color(rgb(0x6b7280))
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(format!("Platform: {:?}", info.platform))
                            .child(format!("OS: {}", info.os_version))
                            .child(format!("Architecture: {}", info.arch))
                    )
                    // Links
                    .child(
                        div()
                            .flex()
                            .gap(px(16.0))
                            .mt(px(8.0))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x3b82f6))
                                    .cursor_pointer()
                                    .child("GitHub")
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x3b82f6))
                                    .cursor_pointer()
                                    .child("Website")
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x3b82f6))
                                    .cursor_pointer()
                                    .child("License")
                            )
                    )
                    // Copyright
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x9ca3af))
                            .mt(px(8.0))
                            .child("© 2024 Tusk. All rights reserved.")
                    )
            )
            .footer(
                div()
                    .flex()
                    .justify_center()
                    .child(
                        Button::ghost()
                            .label("Close")
                            .on_click(cx.listener(|this, _, cx| this.close(cx)))
                    )
            )
    }
}

impl FocusableView for AboutDialog {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
```

### 27.8 Keyboard Shortcut Display

**File: `src/ui/components/shortcut.rs`**

```rust
use crate::platform::Platform;
use gpui::*;

/// Component to display keyboard shortcuts with platform-appropriate modifiers
pub struct Shortcut {
    keys: Vec<String>,
}

impl Shortcut {
    pub fn new(shortcut: &str) -> Self {
        let platform = Platform::current();
        let keys = Self::parse_shortcut(shortcut, &platform);

        Self { keys }
    }

    fn parse_shortcut(shortcut: &str, platform: &Platform) -> Vec<String> {
        shortcut
            .split('+')
            .map(|key| {
                match key.to_lowercase().as_str() {
                    "cmd" | "meta" | "command" => {
                        if platform.is_macos() {
                            "⌘".to_string()
                        } else {
                            "Ctrl".to_string()
                        }
                    }
                    "ctrl" | "control" => {
                        if platform.is_macos() {
                            "⌃".to_string()
                        } else {
                            "Ctrl".to_string()
                        }
                    }
                    "alt" | "option" => {
                        if platform.is_macos() {
                            "⌥".to_string()
                        } else {
                            "Alt".to_string()
                        }
                    }
                    "shift" => {
                        if platform.is_macos() {
                            "⇧".to_string()
                        } else {
                            "Shift".to_string()
                        }
                    }
                    "enter" | "return" => "↵".to_string(),
                    "backspace" => "⌫".to_string(),
                    "delete" => "⌦".to_string(),
                    "escape" | "esc" => "Esc".to_string(),
                    "tab" => "⇥".to_string(),
                    "space" => "Space".to_string(),
                    "up" => "↑".to_string(),
                    "down" => "↓".to_string(),
                    "left" => "←".to_string(),
                    "right" => "→".to_string(),
                    other => other.to_uppercase(),
                }
            })
            .collect()
    }
}

impl IntoElement for Shortcut {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let platform = Platform::current();

        div()
            .flex()
            .items_center()
            .gap(px(if platform.is_macos() { 2.0 } else { 4.0 }))
            .children(
                self.keys.iter().enumerate().map(|(i, key)| {
                    if platform.is_macos() {
                        // macOS style: symbols without separators
                        div()
                            .text_xs()
                            .text_color(rgb(0x9ca3af))
                            .child(key.clone())
                    } else {
                        // Windows/Linux style: keys with + separators
                        div()
                            .flex()
                            .items_center()
                            .when(i > 0, |this| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0xd1d5db))
                                        .mx(px(2.0))
                                        .child("+")
                                )
                            })
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x9ca3af))
                                    .px(px(4.0))
                                    .py(px(1.0))
                                    .bg(rgb(0xf3f4f6))
                                    .rounded(px(3.0))
                                    .border_1()
                                    .border_color(rgb(0xe5e7eb))
                                    .child(key.clone())
                            )
                    }
                })
            )
    }
}
```

### 27.9 File Associations Handler

**File: `src/platform/file_handler.rs`**

```rust
use crate::error::Result;
use std::path::PathBuf;

/// Handle file open requests
pub struct FileHandler;

impl FileHandler {
    /// Handle opening a file
    pub fn open_file(path: PathBuf) -> Result<String> {
        // Read the SQL file
        let content = std::fs::read_to_string(&path)?;
        Ok(content)
    }

    /// Check if a file is a supported SQL file
    pub fn is_sql_file(path: &PathBuf) -> bool {
        path.extension()
            .map(|ext| {
                let ext = ext.to_string_lossy().to_lowercase();
                ext == "sql" || ext == "pgsql"
            })
            .unwrap_or(false)
    }

    /// Register file associations (platform-specific)
    #[cfg(target_os = "macos")]
    pub fn register_file_associations() -> Result<()> {
        // macOS handles this via Info.plist CFBundleDocumentTypes
        // Nothing to do at runtime
        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub fn register_file_associations() -> Result<()> {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // Register .sql extension
        let (sql_key, _) = hkcu.create_subkey("Software\\Classes\\.sql")?;
        sql_key.set_value("", &"Tusk.SQLFile")?;

        // Register .pgsql extension
        let (pgsql_key, _) = hkcu.create_subkey("Software\\Classes\\.pgsql")?;
        pgsql_key.set_value("", &"Tusk.SQLFile")?;

        // Register the file type
        let (type_key, _) = hkcu.create_subkey("Software\\Classes\\Tusk.SQLFile")?;
        type_key.set_value("", &"SQL File")?;

        let exe_path = std::env::current_exe()?.to_string_lossy().to_string();

        let (command_key, _) = hkcu.create_subkey("Software\\Classes\\Tusk.SQLFile\\shell\\open\\command")?;
        command_key.set_value("", &format!("\"{}\" \"%1\"", exe_path))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn register_file_associations() -> Result<()> {
        // Linux uses .desktop files and xdg-mime
        // This is typically handled during package installation
        // Runtime registration is not typical
        Ok(())
    }
}
```

### 27.10 Module Organization

**File: `src/platform/mod.rs`**

```rust
mod menu;
mod file_handler;

pub use menu::{build_application_menus, MenuAction};
pub use file_handler::FileHandler;

use std::env;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

// Platform enum and PlatformInfo from section 27.1
```

## Acceptance Criteria

1. **Menu Bar**
   - [ ] Native menu bar on macOS with app menu
   - [ ] File/Edit/Query/View/Tools/Window/Help menus
   - [ ] Platform-appropriate keyboard shortcuts displayed
   - [ ] Menu actions dispatch GPUI actions

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

## Testing Instructions

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = Platform::current();

        #[cfg(target_os = "macos")]
        assert!(platform.is_macos());

        #[cfg(target_os = "windows")]
        assert!(platform.is_windows());

        #[cfg(target_os = "linux")]
        assert!(platform.is_linux());
    }

    #[test]
    fn test_modifier_keys() {
        let platform = Platform::current();

        if platform.is_macos() {
            assert_eq!(platform.primary_modifier(), "Cmd");
            assert_eq!(platform.secondary_modifier(), "Option");
        } else {
            assert_eq!(platform.primary_modifier(), "Ctrl");
            assert_eq!(platform.secondary_modifier(), "Alt");
        }
    }

    #[test]
    fn test_config_dir() {
        let platform = Platform::current();
        let config_dir = platform.config_dir();

        assert!(config_dir.to_string_lossy().contains("tusk") ||
                config_dir.to_string_lossy().contains("Tusk"));
    }

    #[test]
    fn test_keychain_service() {
        let service = KeychainService::new();

        // Test availability
        let available = service.is_available();
        println!("Keychain available: {}", available);

        if available {
            // Store and retrieve
            service.store_password("test-conn", "test-pass").unwrap();
            let retrieved = service.get_password("test-conn").unwrap();
            assert_eq!(retrieved, Some("test-pass".to_string()));

            // Delete
            service.delete_password("test-conn").unwrap();
            let after_delete = service.get_password("test-conn").unwrap();
            assert_eq!(after_delete, None);
        }
    }

    #[test]
    fn test_shortcut_parsing() {
        let shortcut = Shortcut::new("Cmd+Shift+N");

        // On macOS: ⌘⇧N
        // On Windows/Linux: Ctrl+Shift+N
        assert!(!shortcut.keys.is_empty());
    }
}
```

### Integration Tests with Tauri MCP

```typescript
// Test platform-specific functionality
await driver_session({ action: 'start', port: 9223 });

// Get platform info (via GPUI state)
const snapshot = await webview_dom_snapshot({ type: 'accessibility' });
console.log('Platform UI rendered');

// Test keyboard shortcuts (execute query)
await webview_keyboard({
  action: 'press',
  key: 'Enter',
  modifiers: ['Meta'] // or 'Control' on Windows/Linux
});

// Verify query execution
await webview_wait_for({ type: 'text', value: 'rows' });

// Test menu-triggered actions
await webview_keyboard({
  action: 'press',
  key: ',',
  modifiers: ['Meta'] // Opens preferences
});
await webview_wait_for({ type: 'selector', value: '#settings-dialog' });

// Screenshot the about dialog
await webview_click({ selector: '[data-testid="menu-about"]' });
await webview_screenshot({ filePath: 'about-dialog.png' });

await driver_session({ action: 'stop' });
```
