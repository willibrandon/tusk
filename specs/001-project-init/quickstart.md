# Quickstart: Project Initialization

**Feature**: 001-project-init
**Date**: 2026-01-20

## Prerequisites

### Required Software

| Software | Version | Verification |
|----------|---------|--------------|
| Rust | 1.80+ | `rustc --version` |
| Cargo | latest | `cargo --version` |
| Git | any | `git --version` |

### Platform-Specific Requirements

**macOS**:
```bash
# Xcode Command Line Tools (for Metal framework linking)
xcode-select --install
```

**Linux (Ubuntu/Debian)**:
```bash
# Required system packages
sudo apt install -y \
    libxkbcommon-dev \
    libwayland-dev \
    libvulkan-dev \
    pkg-config
```

**Windows**:
- Visual Studio Build Tools 2022 with "Desktop development with C++"
- Or: Full Visual Studio 2022 with C++ workload

## Quick Start

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/willibrandon/tusk.git
cd tusk

# Build the project
cargo build

# Run the application
cargo run
```

### Expected Outcome

After running `cargo run`, you should see:
1. A window opens at 1400x900 pixels
2. The window has dark theme styling (dark background)
3. The window title shows "Tusk"
4. Attempting to resize below 800x600 is prevented

### Debug Logging

Enable debug output with:

```bash
RUST_LOG=tusk=debug cargo run
```

You should see:
```
2026-01-20T12:00:00.000000Z  INFO tusk: Starting Tusk
```

## Development Workflow

### Hot Reload (requires cargo-watch)

```bash
# Install cargo-watch if not present
cargo install cargo-watch

# Run with auto-reload on file changes
cargo watch -x run
```

### Code Quality Checks

```bash
# Format code
cargo fmt

# Check formatting without modifying
cargo fmt --check

# Run lints (warnings as errors)
cargo clippy -- -D warnings

# Run tests
cargo test
```

### Build for Release

```bash
cargo build --release
```

The release binary will be at `target/release/tusk` (or `tusk.exe` on Windows).

## Project Structure Overview

```
tusk/
├── Cargo.toml           # Workspace manifest
├── crates/
│   ├── tusk/            # Main application binary
│   │   └── src/
│   │       ├── main.rs  # Entry point
│   │       └── app.rs   # Root component
│   ├── tusk_core/       # Shared types and errors
│   └── tusk_ui/         # UI components and theming
├── assets/
│   ├── fonts/           # JetBrains Mono font files
│   └── icons/           # Application icons
└── .github/workflows/   # CI configuration
```

## Common Issues

### Build Fails on Linux

**Error**: `pkg-config: package 'xkbcommon' not found`

**Solution**:
```bash
sudo apt install libxkbcommon-dev
```

### Build Fails on macOS

**Error**: `ld: framework not found Metal`

**Solution**:
```bash
xcode-select --install
```

### Window Doesn't Appear

**Possible Causes**:
1. Running over SSH without X11 forwarding (Linux)
2. Missing Vulkan drivers (Linux)
3. Running in headless environment

**Solution**: Ensure you're running on a display-capable system with GPU drivers installed.

### Rust Version Too Old

**Error**: `error: package 'gpui' requires rustc 1.80.0 or newer`

**Solution**:
```bash
rustup update stable
```

## Performance

### Cold Start Target

The application targets a cold start time under 500ms. To measure:

```bash
time cargo run --release
```

### Release Build

For production performance:

```bash
cargo build --release
./target/release/tusk
```

## CI Integration

The project uses GitHub Actions for continuous integration. CI runs on:
- macOS ARM64 (Apple Silicon)
- macOS x64 (Intel)
- Windows x64
- Linux x64

Each platform runs: format check, clippy lints, build, and tests.

## Verification Checklist

After successful build and run:
- [ ] Window appears at 1400x900 with dark theme
- [ ] Minimum size constraint works (cannot resize below 800x600)
- [ ] Debug logging works with `RUST_LOG=tusk=debug cargo run`
- [ ] JetBrains Mono fonts render correctly (when text is displayed)
- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] Release build succeeds: `cargo build --release`

The project is ready for feature development when all verification steps pass.
