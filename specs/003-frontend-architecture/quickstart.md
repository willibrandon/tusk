# Quickstart: Frontend Architecture

**Feature**: 003-frontend-architecture
**Date**: 2026-01-19

This guide provides step-by-step instructions for implementing the Tusk frontend architecture.

---

## Prerequisites

- Node.js 18+ installed
- Rust 1.75+ installed (for Tauri)
- Project initialized (`npm install` completed)
- Existing files:
  - `src/app.css` — Tailwind configuration with dark mode
  - `src/lib/stores/theme.svelte.ts` — Theme store (needs enhancement)
  - `src/routes/+layout.svelte` — Root layout
  - `src/routes/+page.svelte` — Demo page (to be replaced)

---

## Implementation Order

Follow this order to ensure dependencies are satisfied:

### Phase 1: Type Foundation
1. Copy type contracts to `src/lib/types/`
2. Update `src/lib/types/index.ts` with exports

### Phase 2: Utility Modules
3. Create `src/lib/utils/storage.ts` — localStorage helpers
4. Create `src/lib/utils/keyboard.ts` — Platform detection, shortcut utilities

### Phase 3: State Management
5. Enhance `src/lib/stores/theme.svelte.ts` — Add three-way preference support
6. Create `src/lib/stores/ui.svelte.ts` — Sidebar width, collapsed state
7. Create `src/lib/stores/tabs.svelte.ts` — Tab management
8. Create `src/lib/stores/connections.svelte.ts` — Connection state (placeholder)
9. Update `src/lib/stores/index.ts` — Export all stores

### Phase 4: Base Components
10. Create `src/lib/components/common/Button.svelte`
11. Create `src/lib/components/common/Icon.svelte`

### Phase 5: Shell Components
12. Create `src/lib/components/shell/Resizer.svelte` — Drag handle
13. Create `src/lib/components/shell/StatusBar.svelte` — Bottom status bar
14. Create `src/lib/components/shell/Tab.svelte` — Individual tab
15. Create `src/lib/components/shell/TabBar.svelte` — Tab container with drag-drop
16. Create `src/lib/components/shell/SidebarSearch.svelte` — Filter input
17. Create `src/lib/components/shell/SidebarHeader.svelte` — Header with actions
18. Create `src/lib/components/shell/Sidebar.svelte` — Collapsible sidebar
19. Create `src/lib/components/shell/Shell.svelte` — Main layout container

### Phase 6: Dialogs
20. Create `src/lib/components/dialogs/ConfirmDialog.svelte` — Unsaved changes

### Phase 7: Integration
21. Update `src/routes/+layout.svelte` — Add Shell component
22. Update `src/routes/+page.svelte` — Tab content area
23. Update `src/app.html` — Add FOUC prevention script

### Phase 8: Testing
24. Create unit tests for stores
25. Create component tests
26. Create E2E tests for shell interactions

---

## Key Implementation Patterns

### Store Pattern (Svelte 5 Runes)

```typescript
// src/lib/stores/example.svelte.ts
import { browser } from '$app/environment';

const STORAGE_KEY = 'tusk-example';

function createExampleStore() {
  let value = $state<string>('default');
  let isFirstRun = true;

  // Load from localStorage
  if (browser) {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      try {
        value = JSON.parse(stored);
      } catch { /* use default */ }
    }

    // Persist changes
    $effect(() => {
      const current = value;
      if (!isFirstRun) {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(current));
      }
      isFirstRun = false;
    });
  }

  return {
    get value() { return value; },
    setValue(newValue: string) { value = newValue; }
  };
}

export const exampleStore = createExampleStore();
```

### Component Pattern

```svelte
<!-- src/lib/components/shell/Example.svelte -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    class?: string;
    children?: Snippet;
  }

  let { title, class: className = '', children }: Props = $props();
</script>

<div class="example {className}">
  <h2>{title}</h2>
  {#if children}
    {@render children()}
  {/if}
</div>

<style>
  .example {
    @apply p-4 bg-white dark:bg-gray-900;
  }
</style>
```

### Keyboard Shortcut Pattern

```typescript
// In a component or +layout.svelte
import { isModifierPressed } from '$lib/utils/keyboard';
import { uiStore } from '$lib/stores';

function handleKeyDown(e: KeyboardEvent) {
  if (isModifierPressed(e) && e.key === 'b') {
    e.preventDefault();
    uiStore.toggleSidebar();
  }
}
```

---

## Testing Commands

```bash
# Run all tests
npm test

# Run unit tests only
npm run test:unit

# Run E2E tests only
npm run test:e2e

# Run with coverage
npm run test:coverage

# Run in watch mode
npm run test:unit -- --watch
```

---

## Development Workflow

```bash
# Start development server
npm run tauri:dev

# Type checking
npm run check

# Lint
npm run lint

# Format
npm run format
```

---

## Verification Checklist

After implementation, verify:

- [ ] Shell renders with sidebar, main area, and status bar
- [ ] Sidebar resizes between 200px and 500px
- [ ] Sidebar collapses/expands with Cmd/Ctrl+B
- [ ] Sidebar state persists across page reloads
- [ ] New Tab button creates query tabs
- [ ] Tabs can be selected by clicking
- [ ] Tabs can be closed via close button and middle-click
- [ ] Tabs can be reordered via drag-and-drop
- [ ] Closing modified tab shows confirmation dialog
- [ ] Status bar shows "No connection" initially
- [ ] Theme toggle works (light/dark/system)
- [ ] Theme persists across page reloads
- [ ] No flash of wrong theme on page load
- [ ] All interactive elements are keyboard accessible
- [ ] Tab components have proper ARIA attributes

---

## Common Issues

### Theme flash on page load
Add the inline script to `app.html` `<head>` to apply theme before render.

### Store not reactive
Ensure you're using `$state()` and returning getters, not direct values.

### Drag-and-drop not working
Check that `e.preventDefault()` is called in `dragover` handler.

### Sidebar not persisting
Verify localStorage is available (`browser` check from `$app/environment`).

### Type errors in stores
Ensure `.svelte.ts` extension is used for files with runes.
