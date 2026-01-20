/**
 * Store exports for Tusk frontend.
 *
 * @module stores
 */

// UI store - sidebar width, collapsed state, results panel height
export { uiStore } from './ui.svelte';

// Tab store - tab management, active tab, unsaved changes
export { tabStore } from './tabs.svelte';

// Connection store - database connections and status
export { connectionStore } from './connections.svelte';

// Theme store - light/dark/system theme
export { themeStore, theme } from './theme.svelte';
