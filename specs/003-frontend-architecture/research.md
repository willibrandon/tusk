# Research: Frontend Architecture

**Feature**: 003-frontend-architecture
**Date**: 2026-01-19

## Overview

This research covers the technical patterns and best practices required to implement the Tusk frontend architecture using Svelte 5 runes, native browser APIs, and the existing Tailwind CSS v4 configuration.

---

## 1. Svelte 5 Store Pattern with Runes

### Decision: Function-based stores with object return pattern

**Rationale**: Svelte 5 runes (`$state`, `$derived`, `$effect`) cannot be exported directly due to compiler limitations. The function pattern encapsulates state while exposing reactive getters and mutation methods.

**Alternatives Considered**:

- Class-based stores — Better V8 optimization but less idiomatic for simple stores
- Direct export of `$state` — Not supported by Svelte 5 compiler
- Legacy `writable`/`readable` stores — Deprecated pattern, not recommended for new Svelte 5 code

### Pattern

```typescript
// src/lib/stores/example.svelte.ts
import { browser } from '$app/environment';

function createExampleStore() {
	let items = $state<Item[]>([]);
	let activeId = $state<string | null>(null);

	return {
		get items() {
			return items;
		},
		get activeId() {
			return activeId;
		},
		get activeItem() {
			return items.find((i) => i.id === activeId) ?? null;
		},

		add(item: Item) {
			items.push(item);
		},
		remove(id: string) {
			/* mutation logic */
		},
		setActive(id: string) {
			activeId = id;
		}
	};
}

export const exampleStore = createExampleStore();
```

### localStorage Persistence Pattern

Use `$effect` with a first-run guard to avoid writing back initial values:

```typescript
let isFirstRun = true;

if (browser) {
	$effect(() => {
		const current = { ...state }; // Access to track dependency
		if (!isFirstRun) {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(current));
		}
		isFirstRun = false;
	});
}
```

---

## 2. Tab Drag-and-Drop Reordering

### Decision: Native HTML5 Drag and Drop API

**Rationale**: No external libraries needed. Modern browsers have full support. Works with touch devices via `pointer-events`.

**Alternatives Considered**:

- `@neodrag/svelte` — External dependency, violates minimal dependency goal
- Custom pointer-based implementation — More complex, no browser drag feedback

### Implementation Approach

| Event       | Handler Purpose                                             |
| ----------- | ----------------------------------------------------------- |
| `dragstart` | Set `dataTransfer`, mark dragged tab                        |
| `dragover`  | `preventDefault()` to allow drop, calculate insert position |
| `dragleave` | Clear drop indicator                                        |
| `drop`      | Reorder tabs array, clear state                             |
| `dragend`   | Cleanup regardless of drop success                          |

### Visual Feedback

- Dragged tab: `opacity: 0.5` via `.dragging` class
- Drop indicator: 2px vertical line at insert position (`::before`/`::after` pseudo-elements)
- Position detection: Compare `e.clientX` to element midpoint for before/after

### Accessibility

- Keyboard reordering: Arrow keys with modifier (e.g., Alt+Arrow) to move tabs
- ARIA: `role="tablist"` on container, `role="tab"` and `aria-selected` on tabs
- Announce reorder to screen readers via live region

---

## 3. Resizable Sidebar Panel

### Decision: CSS Flexbox + Pointer Events

**Rationale**: Pure CSS layout with JavaScript resize handle. No layout thrashing during drag.

**Alternatives Considered**:

- CSS `resize` property — Limited styling, no programmatic control
- `react-split-pane` style library — External dependency

### Implementation Approach

```
[Sidebar (flex: 0 0 ${width}px)] [Resizer (4px)] [Main (flex: 1)]
```

- Sidebar width stored in `$state`, persisted to localStorage
- Resizer captures pointer events during drag
- Use `requestAnimationFrame` to throttle updates
- Clamp width between 200px and 500px

### Collapse Behavior

- Collapsed state: Set `display: none` on sidebar, resizer invisible
- Toggle with Cmd/Ctrl+B
- Persist collapsed state to localStorage

### Accessibility

- Resizer: `role="separator"`, `aria-orientation="vertical"`
- Keyboard: Arrow keys to resize (±10px increments)
- Focus styles: 2px solid outline

---

## 4. Keyboard Shortcuts

### Decision: `svelte:window` event handler with platform detection

**Rationale**: Global keyboard handling at window level catches all shortcuts. Platform detection enables proper Cmd/Ctrl mapping.

**Alternatives Considered**:

- `@svelte-put/shortcut` — External dependency
- Tauri global shortcuts — Only for app-global shortcuts (works even when unfocused)
- Per-component handlers — Scattered logic, hard to maintain

### Platform Detection

```typescript
export const isMac =
	typeof navigator !== 'undefined' && navigator.platform.toUpperCase().includes('MAC');

export function isModifierPressed(e: KeyboardEvent): boolean {
	return isMac ? e.metaKey : e.ctrlKey;
}
```

### Shortcuts for This Feature

| Shortcut           | Action            |
| ------------------ | ----------------- |
| Cmd/Ctrl+B         | Toggle sidebar    |
| Cmd/Ctrl+W         | Close current tab |
| Cmd/Ctrl+T         | New tab           |
| Cmd/Ctrl+Tab       | Next tab          |
| Cmd/Ctrl+Shift+Tab | Previous tab      |

### Input Field Handling

Skip shortcut processing when focus is in text inputs, except for explicitly allowed shortcuts (e.g., Cmd+Enter to execute query).

---

## 5. Theme Support

### Decision: Tailwind v4 class strategy with localStorage persistence

**Rationale**: Existing `app.css` already configures `@custom-variant dark`. Theme store already exists and follows correct pattern.

**Alternatives Considered**:

- CSS `prefers-color-scheme` only — No manual override possible
- CSS custom properties only — Tailwind already abstracts this

### Three-Way Theme Mode

1. **Light** — Force light theme
2. **Dark** — Force dark theme
3. **System** — Follow OS preference, listen for changes

### FOUC Prevention

Add inline script in `app.html` `<head>` to apply `dark` class before Svelte hydration:

```html
<script>
	(function () {
		const stored = localStorage.getItem('theme');
		// Parse and apply before any rendering
		if (shouldBeDark) {
			document.documentElement.classList.add('dark');
		}
	})();
</script>
```

### System Preference Tracking

```typescript
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
	if (preferSystem) {
		mode = e.matches ? 'dark' : 'light';
		applyTheme();
	}
});
```

---

## 6. Connection Status Display

### Decision: Dedicated status bar component with reactive connection state

**Rationale**: Status bar is a separate concern from main content. Connection state will come from backend via Tauri events in future features.

### Connection States

| State        | Color  | Label Example                     |
| ------------ | ------ | --------------------------------- |
| Disconnected | Gray   | "No connection"                   |
| Connecting   | Yellow | "Connecting to localhost:5432..." |
| Connected    | Green  | "postgres@localhost:5432"         |
| Error        | Red    | "Connection failed: timeout"      |

### Status Bar Layout

```
[Connection indicator + info] [Spacer] [Cursor position] [Query stats]
```

For this feature, only connection status is implemented. Cursor position and query stats are placeholders for future editor/query features.

---

## 7. Unsaved Changes Dialog

### Decision: Custom modal dialog component

**Rationale**: Native `confirm()` is synchronous and blocks. Custom dialog allows styling and proper Tauri integration.

### Dialog Behavior

- **Trigger**: Attempting to close a tab with `isModified: true`
- **Options**: Save (primary), Discard, Cancel
- **Focus trap**: Tab key cycles through buttons
- **Escape**: Closes dialog (same as Cancel)
- **Backdrop click**: Closes dialog (same as Cancel)

### Implementation Pattern

```svelte
{#if showDialog}
	<div class="backdrop" onclick={cancel}>
		<div class="dialog" onclick|stopPropagation role="dialog" aria-modal="true">
			<h2>Unsaved Changes</h2>
			<p>Do you want to save changes to "{tabTitle}"?</p>
			<div class="actions">
				<Button onclick={save}>Save</Button>
				<Button onclick={discard} variant="secondary">Discard</Button>
				<Button onclick={cancel} variant="ghost">Cancel</Button>
			</div>
		</div>
	</div>
{/if}
```

---

## Dependencies

No new dependencies required. All implementations use:

- Svelte 5 runes (`$state`, `$derived`, `$effect`)
- Native HTML5 APIs (Drag and Drop, Pointer Events, matchMedia)
- Tailwind CSS v4 (already configured)
- SvelteKit utilities (`$app/environment`)
- Tauri API (`@tauri-apps/api` for future IPC)

---

## Performance Considerations

| Operation      | Target | Approach                                        |
| -------------- | ------ | ----------------------------------------------- |
| Sidebar resize | 60fps  | `requestAnimationFrame` throttle, CSS transform |
| Tab drag       | 60fps  | Native drag API, minimal DOM updates            |
| Theme switch   | <100ms | Class toggle on `<html>`, CSS transitions       |
| Store updates  | <16ms  | Fine-grained Svelte 5 reactivity                |
| Initial render | <1s    | Minimal component tree, code splitting ready    |

---

## References

- [Svelte 5 Runes Documentation](https://svelte.dev/docs/svelte/$state)
- [HTML Drag and Drop API | MDN](https://developer.mozilla.org/en-US/docs/Web/API/HTML_Drag_and_Drop_API)
- [Pointer Events | MDN](https://developer.mozilla.org/en-US/docs/Web/API/Pointer_events)
- [Tailwind CSS v4 Dark Mode](https://tailwindcss.com/docs/dark-mode)
- [Window: matchMedia() | MDN](https://developer.mozilla.org/en-US/docs/Web/API/Window/matchMedia)
