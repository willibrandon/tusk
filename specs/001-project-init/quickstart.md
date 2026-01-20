# Quickstart: Project Initialization

**Feature**: 001-project-init
**Date**: 2026-01-19

## Prerequisites

Before starting, ensure you have:

- **Node.js** 18+ installed (`node --version`)
- **Rust** 1.75+ installed (`rustc --version`)
- **Cargo** (comes with Rust)
- **Platform-specific build tools**:
  - macOS: Xcode Command Line Tools (`xcode-select --install`)
  - Windows: Visual Studio Build Tools with C++ workload
  - Linux: `build-essential`, `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`

---

## Quick Setup

```bash
# Clone repository
git clone <repo-url> tusk
cd tusk

# Install Node.js dependencies
npm install

# Install Rust dependencies (happens automatically on first build)
cd src-tauri && cargo build && cd ..

# Start development mode
npm run tauri dev
```

---

## Development Commands

| Command               | Description                |
| --------------------- | -------------------------- |
| `npm run tauri dev`   | Start app with hot reload  |
| `npm run tauri build` | Build production binary    |
| `npm run dev`         | Start Vite dev server only |
| `npm run build`       | Build frontend only        |
| `npm run check`       | TypeScript type checking   |
| `npm run lint`        | ESLint code linting        |
| `npm run lint:fix`    | ESLint with auto-fix       |
| `npm run format`      | Prettier formatting        |
| `npm test`            | Run Vitest unit tests      |

---

## Project Structure Overview

```
tusk/
├── src/                    # Svelte frontend
│   ├── lib/               # Shared code
│   │   ├── components/    # UI components
│   │   ├── stores/        # Svelte stores
│   │   ├── services/      # IPC wrappers
│   │   └── utils/         # Utilities
│   ├── routes/            # SvelteKit routes
│   └── app.css            # Global styles
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── main.rs        # Entry point
│   │   ├── lib.rs         # Tauri setup
│   │   ├── commands/      # IPC commands
│   │   ├── services/      # Business logic
│   │   └── models/        # Data types
│   ├── Cargo.toml         # Rust deps
│   └── tauri.conf.json    # Tauri config
└── specs/                 # Feature specs
```

---

## Verification Checklist

After setup, verify everything works:

1. **Dev server starts**: `npm run tauri dev`
   - Window opens at 1400x900
   - "Welcome to Tusk" message visible
   - DevTools accessible (Cmd+Option+I on macOS)

2. **Hot reload works**:
   - Edit `src/routes/+page.svelte`
   - Changes appear without restart

3. **Linting passes**: `npm run lint`
   - Zero errors

4. **Types check**: `npm run check`
   - Zero errors

5. **Rust compiles**: `cd src-tauri && cargo build`
   - Build succeeds

---

## Common Issues

### Port 5173 in use

```bash
# Kill process on port
lsof -ti:5173 | xargs kill -9
```

### Rust dependencies fail to compile

```bash
# Update Rust toolchain
rustup update stable

# Clean and rebuild
cd src-tauri && cargo clean && cargo build
```

### WebKit not found (Linux)

```bash
# Ubuntu/Debian
sudo apt install libwebkit2gtk-4.1-dev

# Fedora
sudo dnf install webkit2gtk4.1-devel
```

### macOS code signing issues

Development builds don't require signing. For distribution, configure signing in `tauri.conf.json`.

---

## Next Steps

After project initialization is complete:

1. Run `/speckit.tasks` to generate implementation tasks
2. Complete tasks in order (no skipping)
3. Verify all acceptance criteria pass
4. Proceed to Feature 002: Backend Architecture

---

## Resources

- [Tauri v2 Documentation](https://v2.tauri.app)
- [Svelte 5 Documentation](https://svelte.dev/docs)
- [SvelteKit Documentation](https://svelte.dev/docs/kit)
- [Tailwind CSS Documentation](https://tailwindcss.com/docs)
