# Feature 01: Project Initialization

## Overview

Initialize the Tusk project with Tauri v2, establishing the foundational build system, directory structure, and development tooling.

## Goals

- Create a working Tauri v2 + Svelte 5 project structure
- Configure build tooling for all target platforms (macOS, Windows, Linux)
- Establish development workflow with hot reload
- Set up linting, formatting, and code quality tools

## Technical Specification

### 1. Project Creation

```bash
# Create Tauri project with Svelte template
npm create tauri-app@latest tusk -- --template svelte-ts

# Or manually initialize
mkdir tusk && cd tusk
npm init -y
npm install -D @sveltejs/vite-plugin-svelte svelte vite typescript
npm install -D @tauri-apps/cli@next
npm run tauri init
```

### 2. Directory Structure

```
tusk/
├── .github/
│   └── workflows/
│       ├── ci.yml              # CI pipeline
│       └── release.yml         # Release builds
├── docs/
│   ├── design.md
│   └── features/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs             # Entry point
│   │   ├── lib.rs              # Library root
│   │   ├── commands/           # Tauri commands (mod.rs + files)
│   │   ├── services/           # Business logic
│   │   ├── models/             # Data structures
│   │   └── error.rs            # Error types
│   ├── icons/                  # App icons
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── tauri.conf.json
│   └── build.rs
├── src/
│   ├── lib/
│   │   ├── components/
│   │   ├── stores/
│   │   ├── services/
│   │   └── utils/
│   ├── routes/
│   ├── app.html
│   ├── app.css
│   └── main.ts
├── static/
├── tests/
│   ├── e2e/
│   └── unit/
├── .gitignore
├── .prettierrc
├── .eslintrc.cjs
├── package.json
├── svelte.config.js
├── tailwind.config.js
├── postcss.config.js
├── tsconfig.json
├── vite.config.ts
├── CLAUDE.md
└── README.md
```

### 3. Cargo.toml Configuration

```toml
[package]
name = "tusk"
version = "0.1.0"
description = "A fast, free, native Postgres client"
authors = ["Tusk Contributors"]
license = "MIT"
repository = "https://github.com/username/tusk"
edition = "2021"
rust-version = "1.75"

[lib]
name = "tusk_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["macos-private-api"] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tokio-postgres = { version = "0.7", features = ["with-uuid-1", "with-chrono-0_4", "with-serde_json-1"] }
deadpool-postgres = "0.14"
rusqlite = { version = "0.32", features = ["bundled"] }
keyring = "3"
russh = "0.45"
russh-keys = "0.45"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
native-tls = "0.2"
postgres-native-tls = "0.5"
directories = "5"

[target.'cfg(target_os = "macos")'.dependencies]
cocoa = "0.26"
objc = "0.2"

[features]
default = ["custom-protocol"]
custom-protocol = ["tauri/custom-protocol"]

[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### 4. tauri.conf.json

```json
{
	"$schema": "https://schema.tauri.app/config/2",
	"productName": "Tusk",
	"version": "0.1.0",
	"identifier": "com.tusk.app",
	"build": {
		"beforeBuildCommand": "npm run build",
		"beforeDevCommand": "npm run dev",
		"devUrl": "http://localhost:5173",
		"frontendDist": "../build"
	},
	"app": {
		"withGlobalTauri": true,
		"windows": [
			{
				"title": "Tusk",
				"width": 1400,
				"height": 900,
				"minWidth": 800,
				"minHeight": 600,
				"resizable": true,
				"fullscreen": false,
				"decorations": true,
				"transparent": false,
				"center": true
			}
		],
		"security": {
			"csp": "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self' ws://localhost:*"
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
		"macOS": {
			"minimumSystemVersion": "10.15",
			"frameworks": [],
			"exceptionDomain": "",
			"signingIdentity": null,
			"providerShortName": null,
			"entitlements": null
		},
		"windows": {
			"certificateThumbprint": null,
			"digestAlgorithm": "sha256",
			"timestampUrl": ""
		},
		"linux": {
			"appimage": {
				"bundleMediaFramework": false
			},
			"deb": {
				"depends": ["libssl3", "libpq5"]
			},
			"rpm": {
				"depends": ["openssl", "postgresql-libs"]
			}
		}
	}
}
```

### 5. package.json

```json
{
	"name": "tusk",
	"version": "0.1.0",
	"private": true,
	"type": "module",
	"scripts": {
		"dev": "vite dev",
		"build": "vite build",
		"preview": "vite preview",
		"check": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json",
		"check:watch": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json --watch",
		"lint": "eslint . --ext .js,.ts,.svelte",
		"lint:fix": "eslint . --ext .js,.ts,.svelte --fix",
		"format": "prettier --write .",
		"test": "vitest",
		"test:e2e": "playwright test",
		"tauri": "tauri"
	},
	"devDependencies": {
		"@sveltejs/adapter-static": "^3.0.0",
		"@sveltejs/kit": "^2.0.0",
		"@sveltejs/vite-plugin-svelte": "^4.0.0",
		"@tauri-apps/api": "^2.0.0",
		"@tauri-apps/cli": "^2.0.0",
		"@types/node": "^22.0.0",
		"@typescript-eslint/eslint-plugin": "^8.0.0",
		"@typescript-eslint/parser": "^8.0.0",
		"autoprefixer": "^10.4.0",
		"eslint": "^9.0.0",
		"eslint-plugin-svelte": "^2.0.0",
		"postcss": "^8.4.0",
		"prettier": "^3.0.0",
		"prettier-plugin-svelte": "^3.0.0",
		"prettier-plugin-tailwindcss": "^0.6.0",
		"svelte": "^5.0.0",
		"svelte-check": "^4.0.0",
		"tailwindcss": "^3.4.0",
		"typescript": "^5.5.0",
		"vite": "^5.0.0",
		"vitest": "^2.0.0"
	},
	"dependencies": {
		"@tauri-apps/plugin-shell": "^2.0.0",
		"monaco-editor": "^0.52.0",
		"@tanstack/svelte-table": "^8.20.0",
		"@xyflow/svelte": "^0.1.0"
	}
}
```

### 6. vite.config.ts

```typescript
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	clearScreen: false,
	server: {
		port: 5173,
		strictPort: true,
		watch: {
			ignored: ['**/src-tauri/**']
		}
	},
	envPrefix: ['VITE_', 'TAURI_'],
	build: {
		target: process.env.TAURI_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
		minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
		sourcemap: !!process.env.TAURI_DEBUG
	}
});
```

### 7. svelte.config.js

```javascript
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({
			pages: 'build',
			assets: 'build',
			fallback: 'index.html',
			precompress: false,
			strict: true
		}),
		alias: {
			$components: 'src/lib/components',
			$stores: 'src/lib/stores',
			$services: 'src/lib/services',
			$utils: 'src/lib/utils'
		}
	}
};

export default config;
```

### 8. tailwind.config.js

```javascript
/** @type {import('tailwindcss').Config} */
export default {
	content: ['./src/**/*.{html,js,svelte,ts}'],
	darkMode: 'class',
	theme: {
		extend: {
			colors: {
				// Custom color palette for Tusk
				tusk: {
					50: '#f0f9ff',
					100: '#e0f2fe',
					200: '#bae6fd',
					300: '#7dd3fc',
					400: '#38bdf8',
					500: '#0ea5e9',
					600: '#0284c7',
					700: '#0369a1',
					800: '#075985',
					900: '#0c4a6e',
					950: '#082f49'
				}
			},
			fontFamily: {
				mono: ['JetBrains Mono', 'Fira Code', 'Monaco', 'Consolas', 'monospace']
			}
		}
	},
	plugins: []
};
```

### 9. TypeScript Configuration

```json
// tsconfig.json
{
	"extends": "./.svelte-kit/tsconfig.json",
	"compilerOptions": {
		"allowJs": true,
		"checkJs": true,
		"esModuleInterop": true,
		"forceConsistentCasingInFileNames": true,
		"resolveJsonModule": true,
		"skipLibCheck": true,
		"sourceMap": true,
		"strict": true,
		"moduleResolution": "bundler"
	}
}
```

### 10. Initial Rust Entry Point

```rust
// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tusk_lib::run();
}
```

```rust
// src-tauri/src/lib.rs
mod commands;
mod error;
mod models;
mod services;

use tauri::Manager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn run() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Commands will be registered here
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 11. Initial Svelte App

```svelte
<!-- src/routes/+page.svelte -->
<script lang="ts">
	// Main application entry
</script>

<main class="h-screen w-screen bg-white dark:bg-gray-900">
	<div class="flex h-full">
		<!-- Sidebar placeholder -->
		<aside class="w-64 border-r border-gray-200 dark:border-gray-700">
			<div class="p-4">
				<h1 class="text-xl font-bold text-gray-900 dark:text-white">Tusk</h1>
			</div>
		</aside>

		<!-- Main content placeholder -->
		<div class="flex-1">
			<p class="p-4 text-gray-600 dark:text-gray-300">Welcome to Tusk</p>
		</div>
	</div>
</main>
```

```css
/* src/app.css */
@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
	font-family: Inter, system-ui, Avenir, Helvetica, Arial, sans-serif;
	font-synthesis: none;
	text-rendering: optimizeLegibility;
	-webkit-font-smoothing: antialiased;
	-moz-osx-font-smoothing: grayscale;
}

/* Prevent text selection on UI elements */
.no-select {
	user-select: none;
	-webkit-user-select: none;
}

/* Scrollbar styling */
::-webkit-scrollbar {
	width: 8px;
	height: 8px;
}

::-webkit-scrollbar-track {
	background: transparent;
}

::-webkit-scrollbar-thumb {
	background: #cbd5e1;
	border-radius: 4px;
}

.dark ::-webkit-scrollbar-thumb {
	background: #475569;
}
```

### 12. Git Configuration

```gitignore
# .gitignore

# Dependencies
node_modules/
.pnpm-store/

# Build outputs
build/
dist/
target/

# IDE
.idea/
.vscode/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db

# Tauri
src-tauri/target/

# Environment
.env
.env.local
.env.*.local

# Test
coverage/
.nyc_output/

# Logs
*.log
npm-debug.log*
yarn-debug.log*
yarn-error.log*

# SvelteKit
.svelte-kit/
```

## Acceptance Criteria

1. [ ] `npm run tauri dev` launches the application with hot reload
2. [ ] `npm run tauri build` produces working binaries for current platform
3. [ ] Application window opens at 1400x900 with minimum size constraints
4. [ ] Dev tools are available in development mode
5. [ ] Tailwind CSS classes work correctly
6. [ ] Dark mode class toggle works
7. [ ] All linting passes with `npm run lint`
8. [ ] TypeScript compilation succeeds with `npm run check`
9. [ ] Rust compilation succeeds with `cargo build`

## Testing with MCP

### Tauri MCP Testing

```
1. Start the app: npm run tauri dev
2. Connect: driver_session action=start
3. Take screenshot: webview_screenshot
4. Get DOM: webview_dom_snapshot type=accessibility
5. Verify window size: manage_window action=info
```

## Dependencies on Other Features

None - this is the first feature.

## Dependent Features

- 02-backend-architecture.md
- 03-frontend-architecture.md
