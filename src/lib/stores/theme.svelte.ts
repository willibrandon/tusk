/**
 * Theme store for managing light/dark/system theme preferences.
 *
 * @module stores/theme
 */

import { browser } from '$app/environment';
import type { ThemeMode, ThemePreference, ThemeStoreInterface } from '$lib/types';
import { getStorageItem, setStorageItem, STORAGE_KEYS } from '$lib/utils';

/**
 * Stored theme state.
 */
interface ThemeState {
  preference: ThemePreference;
}

/**
 * Default theme preference.
 */
const DEFAULT_PREFERENCE: ThemePreference = 'system';

/**
 * Create the theme store with Svelte 5 runes pattern.
 */
function createThemeStore(): ThemeStoreInterface {
  // Load stored preference
  const stored = browser
    ? getStorageItem<ThemeState>(STORAGE_KEYS.THEME, { preference: DEFAULT_PREFERENCE })
    : { preference: DEFAULT_PREFERENCE };

  // Initialize state
  let preference = $state<ThemePreference>(stored.preference ?? DEFAULT_PREFERENCE);
  let systemPrefersDark = $state(
    browser ? window.matchMedia('(prefers-color-scheme: dark)').matches : false
  );

  // Helper to resolve mode
  function resolveMode(): ThemeMode {
    if (preference === 'system') {
      return systemPrefersDark ? 'dark' : 'light';
    }
    return preference;
  }

  // Apply theme to document
  function applyTheme() {
    if (!browser) return;

    const resolvedMode = resolveMode();

    const root = document.documentElement;
    if (resolvedMode === 'dark') {
      root.classList.add('dark');
    } else {
      root.classList.remove('dark');
    }
  }

  // Track first run to avoid persisting initial load
  let isFirstRun = true;

  // Initialize and persist
  if (browser) {
    // Apply initial theme
    applyTheme();

    // Listen for system preference changes
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQuery.addEventListener('change', (e) => {
      systemPrefersDark = e.matches;
      applyTheme();
    });

    // Persist preference changes
    $effect.root(() => {
      $effect(() => {
        const state: ThemeState = { preference };

        if (!isFirstRun) {
          setStorageItem(STORAGE_KEYS.THEME, state);
        }
        isFirstRun = false;

        // Apply theme whenever preference changes
        applyTheme();
      });
    });
  }

  return {
    get mode() {
      return resolveMode();
    },

    get preference() {
      return preference;
    },

    get isDark() {
      return resolveMode() === 'dark';
    },

    setPreference(newPreference: ThemePreference) {
      preference = newPreference;
      applyTheme();
    },

    toggle() {
      // Toggle directly to light/dark, ignoring system
      const currentMode = resolveMode();
      preference = currentMode === 'light' ? 'dark' : 'light';
      applyTheme();
    },
  };
}

export const themeStore = createThemeStore();

// Re-export for backward compatibility with existing code
export const theme = themeStore;
