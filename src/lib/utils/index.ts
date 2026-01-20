/**
 * Utility exports for Tusk frontend.
 *
 * @module utils
 */

// Storage utilities
export {
  getStorageItem,
  setStorageItem,
  removeStorageItem,
  isStorageAvailable,
  STORAGE_KEYS,
  type StorageResult,
  type StorageKey,
} from './storage';

// Keyboard utilities
export {
  isMac,
  isModifierPressed,
  isShiftPressed,
  isAltPressed,
  isInputElement,
  formatShortcutKey,
  normalizeKey,
  matchesShortcut,
} from './keyboard';
