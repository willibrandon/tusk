/**
 * UI state type definitions for Tusk frontend architecture.
 *
 * @module contracts/ui
 */

// =============================================================================
// Theme Types
// =============================================================================

/**
 * Theme mode values.
 */
export type ThemeMode = 'light' | 'dark';

/**
 * Theme preference values (includes system option).
 */
export type ThemePreference = 'light' | 'dark' | 'system';

/**
 * Theme state stored in localStorage.
 */
export interface ThemeState {
  /** User's theme preference */
  preference: ThemePreference;

  /** Resolved theme mode (computed from preference) */
  mode: ThemeMode;
}

/**
 * Theme store interface.
 */
export interface ThemeStoreInterface {
  /** Current resolved theme mode */
  readonly mode: ThemeMode;

  /** User's theme preference */
  readonly preference: ThemePreference;

  /** Whether dark mode is active */
  readonly isDark: boolean;

  /** Set theme preference */
  setPreference(preference: ThemePreference): void;

  /** Toggle between light and dark */
  toggle(): void;
}

// =============================================================================
// UI Layout Types
// =============================================================================

/**
 * Sidebar width constraints.
 */
export const SIDEBAR_MIN_WIDTH = 200;
export const SIDEBAR_MAX_WIDTH = 500;
export const SIDEBAR_DEFAULT_WIDTH = 264;

/**
 * Results panel height constraints.
 */
export const RESULTS_MIN_HEIGHT = 100;
export const RESULTS_MAX_HEIGHT = 800;
export const RESULTS_DEFAULT_HEIGHT = 300;

/**
 * Persistent UI state for layout preferences.
 */
export interface UIState {
  /** Sidebar width in pixels */
  sidebarWidth: number;

  /** Whether the sidebar is collapsed */
  sidebarCollapsed: boolean;

  /** Results panel height in pixels */
  resultsPanelHeight: number;
}

/**
 * Default UI state values.
 */
export const DEFAULT_UI_STATE: UIState = {
  sidebarWidth: SIDEBAR_DEFAULT_WIDTH,
  sidebarCollapsed: false,
  resultsPanelHeight: RESULTS_DEFAULT_HEIGHT,
};

/**
 * UI store interface.
 */
export interface UIStoreInterface {
  /** Current sidebar width */
  readonly sidebarWidth: number;

  /** Whether sidebar is collapsed */
  readonly sidebarCollapsed: boolean;

  /** Current results panel height */
  readonly resultsPanelHeight: number;

  /** Set sidebar width (clamped to constraints) */
  setSidebarWidth(width: number): void;

  /** Toggle sidebar collapsed state */
  toggleSidebar(): void;

  /** Set sidebar collapsed state explicitly */
  setSidebarCollapsed(collapsed: boolean): void;

  /** Set results panel height (clamped to constraints) */
  setResultsPanelHeight(height: number): void;
}

// =============================================================================
// Dialog Types
// =============================================================================

/**
 * Confirmation dialog result.
 */
export type ConfirmDialogResult = 'confirm' | 'cancel';

/**
 * Unsaved changes dialog result.
 */
export type UnsavedChangesResult = 'save' | 'discard' | 'cancel';

/**
 * Dialog state for managing modal visibility.
 */
export interface DialogState<T> {
  /** Whether the dialog is open */
  isOpen: boolean;

  /** Data passed to the dialog */
  data: T | null;

  /** Resolve function for the dialog promise */
  resolve: ((result: unknown) => void) | null;
}

/**
 * Unsaved changes dialog data.
 */
export interface UnsavedChangesDialogData {
  /** Tab ID with unsaved changes */
  tabId: string;

  /** Tab title for display */
  tabTitle: string;
}

// =============================================================================
// Keyboard Shortcut Types
// =============================================================================

/**
 * Keyboard shortcut definition.
 */
export interface KeyboardShortcut {
  /** Primary key (e.g., 'b', 'w', 'Enter') */
  key: string;

  /** Requires Cmd/Ctrl modifier */
  modifier?: boolean;

  /** Requires Shift modifier */
  shift?: boolean;

  /** Requires Alt/Option modifier */
  alt?: boolean;

  /** Action to perform when shortcut is triggered */
  action: () => void;

  /** Human-readable description */
  description: string;

  /** Whether this shortcut works in text inputs */
  allowInInput?: boolean;
}

/**
 * Format a shortcut for display.
 */
export function formatShortcut(shortcut: KeyboardShortcut, isMac: boolean): string {
  const parts: string[] = [];

  if (shortcut.modifier) {
    parts.push(isMac ? '⌘' : 'Ctrl');
  }

  if (shortcut.shift) {
    parts.push(isMac ? '⇧' : 'Shift');
  }

  if (shortcut.alt) {
    parts.push(isMac ? '⌥' : 'Alt');
  }

  // Format special keys
  const keyMap: Record<string, string> = {
    Enter: isMac ? '↵' : 'Enter',
    Tab: isMac ? '⇥' : 'Tab',
    Escape: isMac ? 'Esc' : 'Esc',
    ArrowUp: '↑',
    ArrowDown: '↓',
    ArrowLeft: '←',
    ArrowRight: '→',
    Backspace: isMac ? '⌫' : 'Backspace',
    Delete: isMac ? '⌦' : 'Del',
  };

  const displayKey = keyMap[shortcut.key] ?? shortcut.key.toUpperCase();
  parts.push(displayKey);

  return isMac ? parts.join('') : parts.join('+');
}

// =============================================================================
// Resize Handle Types
// =============================================================================

/**
 * Resize direction for panel resizers.
 */
export type ResizeDirection = 'horizontal' | 'vertical';

/**
 * Resize handle state.
 */
export interface ResizeState {
  /** Whether a resize is in progress */
  isResizing: boolean;

  /** Starting position of the resize */
  startPosition: number;

  /** Starting size before resize */
  startSize: number;
}

// =============================================================================
// Drag and Drop Types
// =============================================================================

/**
 * Drag state for tab reordering.
 */
export interface TabDragState {
  /** ID of the tab being dragged */
  draggedTabId: string | null;

  /** ID of the tab being hovered over */
  dropTargetTabId: string | null;

  /** Position relative to drop target */
  dropPosition: 'before' | 'after' | null;
}

/**
 * Initial drag state.
 */
export const INITIAL_TAB_DRAG_STATE: TabDragState = {
  draggedTabId: null,
  dropTargetTabId: null,
  dropPosition: null,
};

// =============================================================================
// Status Bar Types
// =============================================================================

/**
 * Information displayed in the status bar.
 */
export interface StatusBarInfo {
  /** Connection display (left side) */
  connection: {
    name: string;
    host: string;
    port: number;
    state: 'connected' | 'connecting' | 'error' | 'disconnected';
    error?: string;
  } | null;

  /** Cursor position (for editor tabs) */
  cursor: {
    line: number;
    column: number;
  } | null;

  /** Query result info (after execution) */
  queryResult: {
    rowCount: number;
    executionTimeMs: number;
  } | null;
}

// =============================================================================
// Type Guards
// =============================================================================

/**
 * Check if a value is a valid ThemePreference.
 */
export function isThemePreference(value: unknown): value is ThemePreference {
  return typeof value === 'string' && ['light', 'dark', 'system'].includes(value);
}

/**
 * Check if a value is a valid ThemeMode.
 */
export function isThemeMode(value: unknown): value is ThemeMode {
  return typeof value === 'string' && ['light', 'dark'].includes(value);
}

/**
 * Check if a value is a valid UIState.
 */
export function isUIState(value: unknown): value is UIState {
  if (typeof value !== 'object' || value === null) return false;

  const state = value as UIState;

  return (
    typeof state.sidebarWidth === 'number' &&
    typeof state.sidebarCollapsed === 'boolean' &&
    typeof state.resultsPanelHeight === 'number'
  );
}

// =============================================================================
// Utility Functions
// =============================================================================

/**
 * Clamp a value between min and max.
 */
export function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

/**
 * Clamp sidebar width to valid range.
 */
export function clampSidebarWidth(width: number): number {
  return clamp(width, SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
}

/**
 * Clamp results panel height to valid range.
 */
export function clampResultsHeight(height: number): number {
  return clamp(height, RESULTS_MIN_HEIGHT, RESULTS_MAX_HEIGHT);
}
