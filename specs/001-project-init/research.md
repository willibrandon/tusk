# Research: Project Initialization

**Feature**: 001-project-init
**Date**: 2026-01-20
**Status**: Complete

## Research Tasks

### 1. GPUI Application Entry Point

**Decision**: Use `Application::new().run(|cx| { ... })` pattern from Zed's GPUI.

**Rationale**: This is the canonical GPUI entry point pattern found in all GPUI examples (`/Users/brandon/src/zed/crates/gpui/examples/hello_world.rs`, lines 89-106). The closure receives `&mut App` context for window creation and global setup.

**Alternatives Considered**:
- Custom application loop: Rejected - GPUI handles platform event loops internally
- Separate init function: Rejected - all initialization must happen in the run closure

**Pattern**:
```rust
fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1400.0), px(900.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(Size { width: px(800.0), height: px(600.0) }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| TuskApp::new()),
        ).unwrap();
        cx.activate(true);
    });
}
```

### 2. Window Configuration

**Decision**: Use `WindowOptions` with explicit bounds, min size, and default titlebar.

**Rationale**: Zed's `build_window_options` (`/Users/brandon/src/zed/crates/zed/src/zed.rs`, lines 299-343) demonstrates the complete WindowOptions configuration. Key fields:
- `window_bounds`: Initial size and position
- `window_min_size`: Enforced minimum dimensions
- `kind`: WindowKind::Normal for main windows

**Alternatives Considered**:
- Lazy window showing (`show: false`): Considered but not needed for simple init
- Custom titlebar (`appears_transparent: true`): Deferred to future features

**Key Settings**:
| Setting | Value | Source |
|---------|-------|--------|
| Default size | 1400x900 px | Spec FR-003 |
| Min size | 800x600 px | Spec FR-004 |
| Positioning | Centered on primary display | GPUI `Bounds::centered()` |
| Window kind | Normal | Standard main window |

### 3. Component Rendering (Render Trait)

**Decision**: Implement `Render` trait on `TuskApp` struct for root component.

**Rationale**: All GPUI components implement the `Render` trait (`/Users/brandon/src/zed/crates/gpui/src/element.rs`). The trait requires:
```rust
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement;
```

**Alternatives Considered**:
- Functional components: GPUI doesn't support this pattern
- Separate view/model: Unnecessary complexity for init feature

**Pattern**:
```rust
struct TuskApp {
    // State fields (none for init)
}

impl Render for TuskApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)  // Dark theme background
            .child("Tusk - PostgreSQL Client")  // Placeholder content
    }
}
```

### 4. Theme System

**Decision**: Define `TuskTheme` struct with dark mode colors as default, using GPUI's `Hsla` color type.

**Rationale**: Zed's theme system (`/Users/brandon/src/zed/crates/theme/src/`) uses `Hsla` for all colors. The `ThemeColors` struct contains 80+ fields for comprehensive UI theming. For init, we need a minimal subset.

**Alternatives Considered**:
- JSON theme files: Deferred - hardcoded colors sufficient for init
- System appearance detection: Deferred - dark mode default per spec

**Minimal Theme Colors**:
| Color | Purpose | Hex Value |
|-------|---------|-----------|
| background | Window background | #1e1e2e |
| surface | Panel backgrounds | #313244 |
| text | Primary text | #cdd6f4 |
| text_muted | Secondary text | #a6adc8 |
| border | Element borders | #45475a |
| accent | Highlights | #89b4fa |

### 5. Font Loading

**Decision**: Bundle JetBrains Mono font files in `assets/fonts/` and rely on GPUI's automatic font discovery.

**Rationale**: GPUI's text system (`/Users/brandon/src/zed/crates/gpui/src/text_system/`) uses `font-kit` for font discovery. Bundled fonts in the assets directory are discovered automatically on app launch.

**Alternatives Considered**:
- System fonts only: Rejected - spec FR-007 requires custom fonts
- Runtime font download: Rejected - violates offline-first principle

**Files Required**:
- JetBrainsMono-Regular.ttf
- JetBrainsMono-Bold.ttf
- JetBrainsMono-Italic.ttf
- JetBrainsMono-BoldItalic.ttf

### 6. Cargo Workspace Configuration

**Decision**: Create workspace with three crates: `tusk` (bin), `tusk_core` (lib), `tusk_ui` (lib).

**Rationale**: Zed uses a workspace with many crates for modularity (`/Users/brandon/src/zed/Cargo.toml`). Three crates provides separation without over-engineering:
- `tusk`: Application entry point and main binary
- `tusk_core`: Shared error types and utilities
- `tusk_ui`: Theme and UI component definitions

**Alternatives Considered**:
- Single crate: Rejected - violates spec FR-011 workspace requirement
- More crates (tusk_editor, tusk_grid, etc.): Deferred - added in later features

**Workspace Dependency Pattern**:
```toml
[workspace.dependencies]
gpui = { git = "https://github.com/zed-industries/zed", rev = "..." }
tracing = "0.1"
tracing-subscriber = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
parking_lot = "0.12"
thiserror = "2.0"
```

### 7. Cross-Platform Build Configuration

**Decision**: Use Cargo workspace with platform-specific dependencies and CI matrix builds.

**Rationale**: Spec FR-006 requires macOS (x64/ARM64), Windows (x64), and Linux (x64) support. GPUI has platform-specific backends:
- macOS: Metal rendering, cocoa windowing
- Windows: DirectX 12 rendering, windows crate
- Linux: Vulkan rendering, wayland-client/xkbcommon

**Alternatives Considered**:
- Cross-compilation from single platform: Not feasible for native GUI
- Docker builds: Useful for Linux CI only

**CI Matrix**:
| Platform | Runner | Target |
|----------|--------|--------|
| macOS ARM | macos-14 | aarch64-apple-darwin |
| macOS x64 | macos-13 | x86_64-apple-darwin |
| Windows | windows-latest | x86_64-pc-windows-msvc |
| Linux | ubuntu-latest | x86_64-unknown-linux-gnu |

### 8. Debug Logging

**Decision**: Use `tracing` with `tracing-subscriber` for structured logging, controlled by `RUST_LOG` env var.

**Rationale**: Constitution specifies `tracing` for logging. The `tracing-subscriber` crate provides `EnvFilter` for `RUST_LOG` support per spec FR-010.

**Alternatives Considered**:
- `log` + `env_logger`: Rejected - constitution mandates `tracing`
- Custom logging: Rejected - unnecessary

**Initialization Pattern**:
```rust
use tracing_subscriber::EnvFilter;

tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .init();

tracing::info!("Starting Tusk");
```

### 9. Code Quality Tooling

**Decision**: Configure `rustfmt.toml` and `clippy.toml` with strict settings, fail CI on warnings.

**Rationale**: Spec FR-008 requires clippy `-D warnings`, FR-009 requires rustfmt configuration.

**rustfmt.toml**:
```toml
edition = "2021"
max_width = 100
use_small_heuristics = "Max"
```

**clippy.toml**:
```toml
# Default lint groups are sufficient
# CI runs: cargo clippy -- -D warnings
```

**CI Check Commands**:
```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
```

### 10. Application Icons

**Decision**: Create platform-specific icons (icns for macOS, ico for Windows) in `assets/icons/`.

**Rationale**: Spec FR-012 requires embedded application icons. macOS requires `.icns` format, Windows requires `.ico` format.

**Alternatives Considered**:
- SVG source with build-time conversion: Adds complexity
- Single PNG: Not supported by all platforms

**Icon Sizes Required**:
- macOS icns: 16, 32, 128, 256, 512, 1024 px
- Windows ico: 16, 32, 48, 256 px

## Summary

All NEEDS CLARIFICATION items resolved. Technical context is complete:

| Item | Resolution |
|------|------------|
| GPUI entry point | `Application::new().run()` |
| Window options | WindowOptions with bounds + min_size |
| Component pattern | Render trait implementation |
| Theme approach | TuskTheme struct with Hsla colors |
| Font loading | Bundled assets, automatic discovery |
| Workspace structure | 3 crates: tusk, tusk_core, tusk_ui |
| Cross-platform | CI matrix with native runners |
| Logging | tracing + RUST_LOG env var |
| Code quality | rustfmt + clippy -D warnings |
| Icons | Platform-specific icns/ico files |

**Ready for Phase 1: Design & Contracts**
