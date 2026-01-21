# Feature 01: Project Initialization

## Overview

Initialize the Tusk project as a pure Rust application using GPUI, establishing the foundational build system, directory structure, and development tooling for a cross-platform native PostgreSQL client.

## Goals

- Create a working Rust workspace with GPUI integration
- Configure build tooling for all target platforms (macOS, Windows, Linux)
- Establish development workflow with cargo watch for hot reload
- Set up linting, formatting, and code quality tools
- Configure platform-specific rendering backends (Metal, DirectX, Vulkan)

## Technical Specification

### 1. Project Creation

```bash
# Create new Rust workspace
mkdir tusk && cd tusk
cargo init --name tusk

# Or create workspace with multiple crates
mkdir tusk && cd tusk
mkdir -p crates/tusk crates/tusk_ui crates/tusk_db crates/tusk_core
```

### 2. Directory Structure

```
tusk/
├── .github/
│   └── workflows/
│       ├── ci.yml                    # CI pipeline
│       └── release.yml               # Release builds
├── assets/
│   ├── fonts/
│   │   ├── JetBrainsMono-Regular.ttf
│   │   ├── JetBrainsMono-Bold.ttf
│   │   ├── JetBrainsMono-Italic.ttf
│   │   └── JetBrainsMono-BoldItalic.ttf
│   ├── icons/
│   │   ├── app_icon.png              # App icon source
│   │   ├── app_icon.icns             # macOS icon
│   │   ├── app_icon.ico              # Windows icon
│   │   ├── database.svg              # UI icons
│   │   ├── table.svg
│   │   ├── column.svg
│   │   ├── key.svg
│   │   ├── index.svg
│   │   ├── function.svg
│   │   ├── view.svg
│   │   ├── schema.svg
│   │   └── ...
│   └── themes/
│       ├── default_dark.json
│       └── default_light.json
├── crates/
│   ├── tusk/                         # Main application binary
│   │   ├── src/
│   │   │   └── main.rs
│   │   ├── Cargo.toml
│   │   └── build.rs
│   ├── tusk_app/                     # Application logic
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── app.rs                # TuskApp main struct
│   │   │   ├── workspace.rs          # Workspace management
│   │   │   └── actions.rs            # Global actions
│   │   └── Cargo.toml
│   ├── tusk_ui/                      # UI components
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── components/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── button.rs
│   │   │   │   ├── input.rs
│   │   │   │   ├── modal.rs
│   │   │   │   ├── context_menu.rs
│   │   │   │   ├── tab_bar.rs
│   │   │   │   ├── tree.rs
│   │   │   │   ├── table.rs
│   │   │   │   ├── toolbar.rs
│   │   │   │   ├── status_bar.rs
│   │   │   │   ├── split_pane.rs
│   │   │   │   ├── tooltip.rs
│   │   │   │   ├── dropdown.rs
│   │   │   │   └── ...
│   │   │   ├── theme.rs              # Theme types
│   │   │   ├── colors.rs             # Color palette
│   │   │   └── icons.rs              # Icon rendering
│   │   └── Cargo.toml
│   ├── tusk_editor/                  # SQL editor component
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── editor.rs             # Main editor view
│   │   │   ├── buffer.rs             # Text buffer (rope)
│   │   │   ├── display_map.rs        # Display mapping
│   │   │   ├── syntax.rs             # SQL syntax highlighting
│   │   │   ├── autocomplete.rs       # Autocomplete popup
│   │   │   ├── element.rs            # Editor element rendering
│   │   │   └── selections.rs         # Cursor/selection handling
│   │   └── Cargo.toml
│   ├── tusk_grid/                    # Results grid component
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── grid.rs               # Main grid view
│   │   │   ├── column.rs             # Column definitions
│   │   │   ├── cell.rs               # Cell rendering
│   │   │   ├── selection.rs          # Cell/row selection
│   │   │   ├── sorting.rs            # Sort handling
│   │   │   └── export.rs             # Export functionality
│   │   └── Cargo.toml
│   ├── tusk_db/                      # Database connectivity
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── connection.rs         # Connection management
│   │   │   ├── pool.rs               # Connection pooling
│   │   │   ├── query.rs              # Query execution
│   │   │   ├── schema.rs             # Schema introspection
│   │   │   ├── types.rs              # PostgreSQL type mapping
│   │   │   ├── ssh.rs                # SSH tunnel support
│   │   │   └── ssl.rs                # SSL/TLS support
│   │   └── Cargo.toml
│   ├── tusk_storage/                 # Local storage (SQLite)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── database.rs           # SQLite database
│   │   │   ├── migrations.rs         # Schema migrations
│   │   │   ├── connections.rs        # Saved connections
│   │   │   ├── history.rs            # Query history
│   │   │   └── settings.rs           # User settings
│   │   └── Cargo.toml
│   └── tusk_core/                    # Shared types and utilities
│       ├── src/
│       │   ├── lib.rs
│       │   ├── models/
│       │   │   ├── mod.rs
│       │   │   ├── connection.rs     # Connection model
│       │   │   ├── schema.rs         # Schema model
│       │   │   ├── query.rs          # Query model
│       │   │   └── settings.rs       # Settings model
│       │   ├── error.rs              # Error types
│       │   └── util.rs               # Utilities
│       └── Cargo.toml
├── docs/
│   ├── design.md                     # Design specification
│   └── features/                     # Feature documents
├── tests/
│   ├── integration/                  # Integration tests
│   └── fixtures/                     # Test fixtures
├── .gitignore
├── .rustfmt.toml
├── clippy.toml
├── Cargo.toml                        # Workspace root
├── Cargo.lock
├── CLAUDE.md
├── LICENSE
└── README.md
```

### 3. Root Cargo.toml (Workspace)

```toml
[workspace]
resolver = "2"
members = [
    "crates/tusk",
    "crates/tusk_app",
    "crates/tusk_ui",
    "crates/tusk_editor",
    "crates/tusk_grid",
    "crates/tusk_db",
    "crates/tusk_storage",
    "crates/tusk_core",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.80"
authors = ["Tusk Contributors"]
license = "MIT"
repository = "https://github.com/willibrandon/tusk"

[workspace.dependencies]
# GPUI from Zed (use git dependency or path)
gpui = { git = "https://github.com/zed-industries/zed", rev = "main" }

# Async runtime
tokio = { version = "1", features = ["full"] }
futures = "0.3"
async-trait = "0.1"

# PostgreSQL
tokio-postgres = { version = "0.7", features = ["with-uuid-1", "with-chrono-0_4", "with-serde_json-1"] }
deadpool-postgres = "0.14"
postgres-types = { version = "0.2", features = ["derive"] }
postgres-native-tls = "0.5"
native-tls = "0.2"

# SSH tunneling
russh = "0.45"
russh-keys = "0.45"

# Local storage
rusqlite = { version = "0.32", features = ["bundled"] }

# Credential storage
keyring = "3"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Text handling
ropey = "1.6"                        # Rope data structure for text buffer
tree-sitter = "0.24"                 # Parsing for syntax highlighting
tree-sitter-sql = "0.3"              # SQL grammar

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
directories = "5"
parking_lot = "0.12"
smallvec = "1.13"
indexmap = { version = "2", features = ["serde"] }
regex = "1"
once_cell = "1"

# Platform-specific
[target.'cfg(target_os = "macos")'.workspace.dependencies]
cocoa = "0.26"
objc = "0.2"
core-foundation = "0.10"
core-graphics = "0.24"

[target.'cfg(target_os = "windows")'.workspace.dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_Graphics_Direct3D",
    "Win32_Graphics_Dxgi",
] }

[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
opt-level = 3

[profile.dev]
opt-level = 0
debug = true

[profile.dev.package."*"]
opt-level = 3
```

### 4. Main Binary Crate (crates/tusk/Cargo.toml)

```toml
[package]
name = "tusk"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
description = "A fast, free, native PostgreSQL client"
default-run = "tusk"

[[bin]]
name = "tusk"
path = "src/main.rs"

[dependencies]
tusk_app.path = "../tusk_app"
tusk_core.path = "../tusk_core"
gpui.workspace = true
tokio.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
anyhow.workspace = true
directories.workspace = true

[target.'cfg(target_os = "macos")'.dependencies]
cocoa.workspace = true
objc.workspace = true

[target.'cfg(target_os = "windows")'.dependencies]
windows.workspace = true

[build-dependencies]
# For embedding app icon and resources
embed-resource = "2"

[package.metadata.bundle]
name = "Tusk"
identifier = "com.tusk.app"
icon = ["../../assets/icons/app_icon.icns", "../../assets/icons/app_icon.ico"]
version = "0.1.0"
copyright = "Copyright (c) 2024 Tusk Contributors"
category = "Developer Tool"
short_description = "A fast PostgreSQL client"
```

### 5. Main Entry Point (crates/tusk/src/main.rs)

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use gpui::*;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tusk_app::TuskApp;

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("tusk=debug".parse()?))
        .init();

    tracing::info!("Starting Tusk");

    // Initialize GPUI application
    App::new().run(|cx: &mut AppContext| {
        // Load assets
        load_assets(cx);

        // Initialize global state
        TuskApp::init(cx);

        // Create main window
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::default(),
                size: Size {
                    width: px(1400.0),
                    height: px(900.0),
                },
            })),
            titlebar: Some(TitlebarOptions {
                title: Some("Tusk".into()),
                appears_transparent: false,
                ..Default::default()
            }),
            window_min_size: Some(Size {
                width: px(800.0),
                height: px(600.0),
            }),
            kind: WindowKind::Normal,
            is_movable: true,
            center: true,
            ..Default::default()
        };

        cx.open_window(window_options, |cx| {
            cx.new_view(|cx| TuskApp::new(cx))
        })
        .expect("Failed to open window");
    });

    Ok(())
}

fn load_assets(cx: &mut AppContext) {
    // Register fonts
    let font_paths = [
        "assets/fonts/JetBrainsMono-Regular.ttf",
        "assets/fonts/JetBrainsMono-Bold.ttf",
        "assets/fonts/JetBrainsMono-Italic.ttf",
        "assets/fonts/JetBrainsMono-BoldItalic.ttf",
    ];

    for path in font_paths {
        if let Ok(font_data) = std::fs::read(path) {
            cx.text_system()
                .add_fonts(vec![font_data.into()])
                .expect("Failed to load font");
        }
    }
}
```

### 6. Application Crate (crates/tusk_app/src/lib.rs)

```rust
mod actions;
mod app;
mod workspace;

pub use app::TuskApp;
pub use workspace::TuskWorkspace;
```

### 7. TuskApp Implementation (crates/tusk_app/src/app.rs)

```rust
use gpui::*;
use tusk_core::models::settings::Settings;
use tusk_storage::Database;

use crate::workspace::TuskWorkspace;
use crate::actions;

/// Global application state
pub struct TuskAppState {
    pub database: Database,
    pub settings: Settings,
}

impl Global for TuskAppState {}

pub struct TuskApp {
    workspace: Entity<TuskWorkspace>,
}

impl TuskApp {
    /// Initialize global application state
    pub fn init(cx: &mut AppContext) {
        // Register global actions
        actions::register_actions(cx);

        // Initialize database
        let database = Database::new().expect("Failed to initialize database");

        // Load settings
        let settings = database.load_settings().unwrap_or_default();

        // Set global state
        cx.set_global(TuskAppState { database, settings });
    }

    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let workspace = cx.new_model(|cx| TuskWorkspace::new(cx));

        Self { workspace }
    }
}

impl Render for TuskApp {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let workspace = self.workspace.clone();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().colors().background)
            .text_color(cx.theme().colors().text)
            .font_family("JetBrains Mono")
            .child(workspace)
    }
}
```

### 8. Core Types Crate (crates/tusk_core/src/lib.rs)

```rust
pub mod error;
pub mod models;
pub mod util;

pub use error::{Error, Result};
```

### 9. Error Types (crates/tusk_core/src/error.rs)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Connection error: {message}")]
    Connection {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Query error: {message}")]
    Query {
        message: String,
        code: Option<String>,
        position: Option<u32>,
        hint: Option<String>,
    },

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("SSH tunnel error: {0}")]
    SshTunnel(String),

    #[error("SSL/TLS error: {0}")]
    Ssl(String),

    #[error("Storage error: {0}")]
    Storage(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Timeout: operation did not complete within {0}ms")]
    Timeout(u64),

    #[error("Cancelled")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection {
            message: message.into(),
            source: None,
        }
    }

    pub fn query(message: impl Into<String>) -> Self {
        Self::Query {
            message: message.into(),
            code: None,
            position: None,
            hint: None,
        }
    }

    pub fn with_pg_error(err: &tokio_postgres::Error) -> Self {
        if let Some(db_err) = err.as_db_error() {
            Self::Query {
                message: db_err.message().to_string(),
                code: Some(db_err.code().code().to_string()),
                position: db_err.position().map(|p| match p {
                    tokio_postgres::error::ErrorPosition::Original(pos) => *pos,
                    tokio_postgres::error::ErrorPosition::Internal { position, .. } => *position,
                }),
                hint: db_err.hint().map(String::from),
            }
        } else {
            Self::connection(err.to_string())
        }
    }
}
```

### 10. Build Script (crates/tusk/build.rs)

```rust
fn main() {
    // Embed Windows resources (icon, manifest)
    #[cfg(target_os = "windows")]
    {
        let mut res = embed_resource::WindowsResource::new();
        res.set_icon("../../assets/icons/app_icon.ico");
        res.set_manifest_file("app.manifest");
        res.compile().expect("Failed to compile Windows resources");
    }

    // macOS: No special build steps needed, icon handled by bundle
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rerun-if-changed=../../assets/icons/app_icon.icns");
    }

    // Link platform-specific frameworks
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=Metal");
        println!("cargo:rustc-link-lib=framework=QuartzCore");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=AppKit");
    }
}
```

### 11. Windows Manifest (crates/tusk/app.manifest)

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    version="0.1.0.0"
    processorArchitecture="*"
    name="com.tusk.app"
    type="win32"
  />
  <description>Tusk PostgreSQL Client</description>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 10/11 -->
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>
```

### 12. Rust Formatting (.rustfmt.toml)

```toml
edition = "2021"
max_width = 100
tab_spaces = 4
newline_style = "Auto"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
remove_nested_parens = true
format_strings = false
wrap_comments = true
comment_width = 80
normalize_comments = true
format_code_in_doc_comments = true
format_macro_matchers = true
format_macro_bodies = true
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
```

### 13. Clippy Configuration (clippy.toml)

```toml
avoid-breaking-exported-api = false
msrv = "1.80"
```

### 14. Git Configuration (.gitignore)

```gitignore
# Rust build artifacts
/target/
**/*.rs.bk
Cargo.lock

# IDE
.idea/
.vscode/
*.swp
*.swo
.vim/

# OS
.DS_Store
Thumbs.db
Desktop.ini

# Environment
.env
.env.local
.env.*.local

# Test artifacts
coverage/
*.profraw

# Logs
*.log

# Editor backup files
*~
\#*\#
.\#*

# Build artifacts
*.dll
*.so
*.dylib
```

### 15. CI Configuration (.github/workflows/ci.yml)

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install Linux dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libxkbcommon-dev libwayland-dev
      - run: cargo check --all-features

  test:
    name: Test
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install Linux dependencies
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libxkbcommon-dev libwayland-dev
      - run: cargo test --all-features

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - name: Install Linux dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libxkbcommon-dev libwayland-dev
      - run: cargo clippy --all-features -- -D warnings

  fmt:
    name: Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all -- --check

  build:
    name: Build
    needs: [check, test, clippy, fmt]
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      - name: Install Linux dependencies
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libxkbcommon-dev libwayland-dev
      - run: cargo build --release --target ${{ matrix.target }}
      - uses: actions/upload-artifact@v4
        with:
          name: tusk-${{ matrix.target }}
          path: |
            target/${{ matrix.target }}/release/tusk
            target/${{ matrix.target }}/release/tusk.exe
```

### 16. Release Configuration (.github/workflows/release.yml)

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

env:
  CARGO_TERM_COLOR: always

jobs:
  build-release:
    name: Build Release
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact_name: tusk
            asset_name: tusk-macos-x64
          - os: macos-latest
            target: aarch64-apple-darwin
            artifact_name: tusk
            asset_name: tusk-macos-arm64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact_name: tusk.exe
            asset_name: tusk-windows-x64.exe
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact_name: tusk
            asset_name: tusk-linux-x64
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      - name: Install Linux dependencies
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libxkbcommon-dev libwayland-dev
      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
      - name: Package macOS app
        if: runner.os == 'macOS'
        run: |
          mkdir -p Tusk.app/Contents/MacOS
          mkdir -p Tusk.app/Contents/Resources
          cp target/${{ matrix.target }}/release/tusk Tusk.app/Contents/MacOS/
          cp assets/icons/app_icon.icns Tusk.app/Contents/Resources/
          # Create Info.plist
          cat > Tusk.app/Contents/Info.plist << EOF
          <?xml version="1.0" encoding="UTF-8"?>
          <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
          <plist version="1.0">
          <dict>
              <key>CFBundleName</key>
              <string>Tusk</string>
              <key>CFBundleIdentifier</key>
              <string>com.tusk.app</string>
              <key>CFBundleVersion</key>
              <string>${GITHUB_REF#refs/tags/v}</string>
              <key>CFBundleExecutable</key>
              <string>tusk</string>
              <key>CFBundleIconFile</key>
              <string>app_icon</string>
              <key>LSMinimumSystemVersion</key>
              <string>10.15</string>
              <key>NSHighResolutionCapable</key>
              <true/>
          </dict>
          </plist>
          EOF
          zip -r ${{ matrix.asset_name }}.zip Tusk.app
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.asset_name }}
          path: |
            target/${{ matrix.target }}/release/${{ matrix.artifact_name }}
            ${{ matrix.asset_name }}.zip

  create-release:
    name: Create Release
    needs: build-release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with:
          path: artifacts
      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: artifacts/**/*
          draft: false
          prerelease: false
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### 17. Development Commands

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install targets for cross-compilation
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
rustup target add x86_64-pc-windows-msvc
rustup target add x86_64-unknown-linux-gnu

# Install development tools
cargo install cargo-watch
cargo install cargo-audit
cargo install cargo-outdated

# Development with auto-reload
cargo watch -x run

# Run tests
cargo test --workspace

# Run with debug logging
RUST_LOG=tusk=debug cargo run

# Build for release
cargo build --release

# Build for specific target
cargo build --release --target aarch64-apple-darwin

# Run clippy
cargo clippy --workspace -- -D warnings

# Format code
cargo fmt --all

# Check for security vulnerabilities
cargo audit

# Check for outdated dependencies
cargo outdated
```

### 18. GPUI Asset Loading Pattern

```rust
// crates/tusk_ui/src/icons.rs
use gpui::*;

#[derive(Clone)]
pub struct Icons {
    pub database: SharedString,
    pub table: SharedString,
    pub column: SharedString,
    pub key: SharedString,
    pub index: SharedString,
    pub function: SharedString,
    pub view: SharedString,
    pub schema: SharedString,
    pub play: SharedString,
    pub stop: SharedString,
    pub save: SharedString,
    pub folder: SharedString,
    pub file: SharedString,
    pub settings: SharedString,
    pub refresh: SharedString,
    pub search: SharedString,
    pub close: SharedString,
    pub add: SharedString,
    pub remove: SharedString,
    pub edit: SharedString,
    pub copy: SharedString,
    pub paste: SharedString,
    pub undo: SharedString,
    pub redo: SharedString,
}

impl Icons {
    pub fn load() -> Self {
        Self {
            database: include_str!("../../assets/icons/database.svg").into(),
            table: include_str!("../../assets/icons/table.svg").into(),
            column: include_str!("../../assets/icons/column.svg").into(),
            key: include_str!("../../assets/icons/key.svg").into(),
            index: include_str!("../../assets/icons/index.svg").into(),
            function: include_str!("../../assets/icons/function.svg").into(),
            view: include_str!("../../assets/icons/view.svg").into(),
            schema: include_str!("../../assets/icons/schema.svg").into(),
            play: include_str!("../../assets/icons/play.svg").into(),
            stop: include_str!("../../assets/icons/stop.svg").into(),
            save: include_str!("../../assets/icons/save.svg").into(),
            folder: include_str!("../../assets/icons/folder.svg").into(),
            file: include_str!("../../assets/icons/file.svg").into(),
            settings: include_str!("../../assets/icons/settings.svg").into(),
            refresh: include_str!("../../assets/icons/refresh.svg").into(),
            search: include_str!("../../assets/icons/search.svg").into(),
            close: include_str!("../../assets/icons/close.svg").into(),
            add: include_str!("../../assets/icons/add.svg").into(),
            remove: include_str!("../../assets/icons/remove.svg").into(),
            edit: include_str!("../../assets/icons/edit.svg").into(),
            copy: include_str!("../../assets/icons/copy.svg").into(),
            paste: include_str!("../../assets/icons/paste.svg").into(),
            undo: include_str!("../../assets/icons/undo.svg").into(),
            redo: include_str!("../../assets/icons/redo.svg").into(),
        }
    }
}

impl Global for Icons {}

// Register icons at startup
pub fn init_icons(cx: &mut AppContext) {
    cx.set_global(Icons::load());
}

// Usage in components
pub fn icon(name: &str, cx: &AppContext) -> impl IntoElement {
    let icons = cx.global::<Icons>();
    let svg_content = match name {
        "database" => &icons.database,
        "table" => &icons.table,
        // ... etc
        _ => &icons.database,
    };

    svg()
        .path(svg_content.clone())
        .size_4()
        .text_color(cx.theme().colors().text)
}
```

### 19. Theme System Bootstrap

```rust
// crates/tusk_ui/src/theme.rs
use gpui::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeColors {
    pub background: Hsla,
    pub background_elevated: Hsla,
    pub surface: Hsla,
    pub surface_hover: Hsla,
    pub border: Hsla,
    pub border_focused: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_accent: Hsla,
    pub primary: Hsla,
    pub primary_hover: Hsla,
    pub secondary: Hsla,
    pub success: Hsla,
    pub warning: Hsla,
    pub error: Hsla,
    pub selection: Hsla,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyntaxColors {
    pub keyword: Hsla,
    pub string: Hsla,
    pub number: Hsla,
    pub comment: Hsla,
    pub function: Hsla,
    pub type_name: Hsla,
    pub variable: Hsla,
    pub operator: Hsla,
    pub punctuation: Hsla,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub is_dark: bool,
    pub colors: ThemeColors,
    pub syntax: SyntaxColors,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            name: "Tusk Dark".into(),
            is_dark: true,
            colors: ThemeColors {
                background: hsla(220.0 / 360.0, 0.13, 0.10, 1.0),
                background_elevated: hsla(220.0 / 360.0, 0.13, 0.12, 1.0),
                surface: hsla(220.0 / 360.0, 0.13, 0.14, 1.0),
                surface_hover: hsla(220.0 / 360.0, 0.13, 0.18, 1.0),
                border: hsla(220.0 / 360.0, 0.13, 0.20, 1.0),
                border_focused: hsla(210.0 / 360.0, 0.80, 0.55, 1.0),
                text: hsla(220.0 / 360.0, 0.09, 0.93, 1.0),
                text_muted: hsla(220.0 / 360.0, 0.09, 0.60, 1.0),
                text_accent: hsla(210.0 / 360.0, 0.80, 0.65, 1.0),
                primary: hsla(210.0 / 360.0, 0.80, 0.55, 1.0),
                primary_hover: hsla(210.0 / 360.0, 0.80, 0.60, 1.0),
                secondary: hsla(220.0 / 360.0, 0.13, 0.25, 1.0),
                success: hsla(142.0 / 360.0, 0.71, 0.45, 1.0),
                warning: hsla(38.0 / 360.0, 0.92, 0.50, 1.0),
                error: hsla(0.0, 0.84, 0.60, 1.0),
                selection: hsla(210.0 / 360.0, 0.80, 0.55, 0.3),
            },
            syntax: SyntaxColors {
                keyword: hsla(280.0 / 360.0, 0.68, 0.70, 1.0),
                string: hsla(95.0 / 360.0, 0.50, 0.60, 1.0),
                number: hsla(30.0 / 360.0, 0.90, 0.65, 1.0),
                comment: hsla(220.0 / 360.0, 0.10, 0.50, 1.0),
                function: hsla(210.0 / 360.0, 0.80, 0.70, 1.0),
                type_name: hsla(180.0 / 360.0, 0.60, 0.60, 1.0),
                variable: hsla(220.0 / 360.0, 0.09, 0.93, 1.0),
                operator: hsla(220.0 / 360.0, 0.09, 0.70, 1.0),
                punctuation: hsla(220.0 / 360.0, 0.09, 0.60, 1.0),
            },
        }
    }

    pub fn light() -> Self {
        Self {
            name: "Tusk Light".into(),
            is_dark: false,
            colors: ThemeColors {
                background: hsla(0.0, 0.0, 1.0, 1.0),
                background_elevated: hsla(220.0 / 360.0, 0.13, 0.97, 1.0),
                surface: hsla(220.0 / 360.0, 0.13, 0.95, 1.0),
                surface_hover: hsla(220.0 / 360.0, 0.13, 0.90, 1.0),
                border: hsla(220.0 / 360.0, 0.13, 0.85, 1.0),
                border_focused: hsla(210.0 / 360.0, 0.80, 0.50, 1.0),
                text: hsla(220.0 / 360.0, 0.13, 0.15, 1.0),
                text_muted: hsla(220.0 / 360.0, 0.09, 0.45, 1.0),
                text_accent: hsla(210.0 / 360.0, 0.80, 0.45, 1.0),
                primary: hsla(210.0 / 360.0, 0.80, 0.50, 1.0),
                primary_hover: hsla(210.0 / 360.0, 0.80, 0.45, 1.0),
                secondary: hsla(220.0 / 360.0, 0.13, 0.90, 1.0),
                success: hsla(142.0 / 360.0, 0.71, 0.35, 1.0),
                warning: hsla(38.0 / 360.0, 0.92, 0.45, 1.0),
                error: hsla(0.0, 0.84, 0.50, 1.0),
                selection: hsla(210.0 / 360.0, 0.80, 0.50, 0.2),
            },
            syntax: SyntaxColors {
                keyword: hsla(280.0 / 360.0, 0.68, 0.45, 1.0),
                string: hsla(95.0 / 360.0, 0.60, 0.35, 1.0),
                number: hsla(30.0 / 360.0, 0.90, 0.40, 1.0),
                comment: hsla(220.0 / 360.0, 0.10, 0.55, 1.0),
                function: hsla(210.0 / 360.0, 0.80, 0.45, 1.0),
                type_name: hsla(180.0 / 360.0, 0.70, 0.40, 1.0),
                variable: hsla(220.0 / 360.0, 0.13, 0.15, 1.0),
                operator: hsla(220.0 / 360.0, 0.09, 0.40, 1.0),
                punctuation: hsla(220.0 / 360.0, 0.09, 0.50, 1.0),
            },
        }
    }
}

impl Global for Theme {}

pub fn init_theme(cx: &mut AppContext, dark: bool) {
    let theme = if dark { Theme::dark() } else { Theme::light() };
    cx.set_global(theme);
}

// Extension trait for easy theme access
pub trait ThemeExt {
    fn theme(&self) -> &Theme;
}

impl ThemeExt for AppContext {
    fn theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}

impl<V> ThemeExt for ViewContext<'_, V> {
    fn theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}
```

## Acceptance Criteria

1. [ ] `cargo build` compiles successfully with no errors
2. [ ] `cargo run` launches the application window at 1400x900
3. [ ] Window respects minimum size constraints (800x600)
4. [ ] Application displays basic UI shell with correct theming
5. [ ] All linting passes with `cargo clippy -- -D warnings`
6. [ ] Code formatting is correct with `cargo fmt --check`
7. [ ] Cross-platform build succeeds for macOS (x64, ARM), Windows, Linux
8. [ ] CI pipeline runs and passes all checks
9. [ ] Custom font (JetBrains Mono) loads and renders correctly
10. [ ] Debug logging works with `RUST_LOG=tusk=debug`

## Platform-Specific Notes

### macOS
- Uses Metal for GPU rendering
- App bundling requires Info.plist and .icns icon
- Minimum macOS version: 10.15 (Catalina)
- Universal binary support (x64 + ARM)

### Windows
- Uses DirectX 12 for GPU rendering
- Requires app.manifest for DPI awareness
- .ico icon embedded via build.rs
- Windows 10/11 support

### Linux
- Uses Vulkan via Blade for GPU rendering
- Requires libxkbcommon and libwayland development headers
- AppImage/Flatpak packaging (future)

## Dependencies on Other Features

None - this is the first feature.

## Dependent Features

- 02-backend-architecture.md
- 03-frontend-architecture.md
- All other features depend on this project structure
