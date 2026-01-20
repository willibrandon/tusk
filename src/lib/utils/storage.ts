/**
 * localStorage helper utilities with error handling and type-safe get/set.
 *
 * @module utils/storage
 */

import { browser } from '$app/environment';

/**
 * Storage result indicating success or failure with error message.
 */
export type StorageResult<T> =
  | { success: true; data: T }
  | { success: false; error: string };

/**
 * Get a value from localStorage with type-safe parsing.
 *
 * @param key - The localStorage key
 * @param defaultValue - Default value if key doesn't exist or parsing fails
 * @returns The parsed value or default value
 */
export function getStorageItem<T>(key: string, defaultValue: T): T {
  if (!browser) {
    return defaultValue;
  }

  try {
    const item = localStorage.getItem(key);
    if (item === null) {
      return defaultValue;
    }
    return JSON.parse(item) as T;
  } catch {
    // Return default if JSON parsing fails
    return defaultValue;
  }
}

/**
 * Set a value in localStorage with JSON serialization.
 *
 * @param key - The localStorage key
 * @param value - The value to store
 * @returns Result indicating success or failure
 */
export function setStorageItem<T>(key: string, value: T): StorageResult<void> {
  if (!browser) {
    return { success: false, error: 'localStorage not available (not in browser)' };
  }

  try {
    localStorage.setItem(key, JSON.stringify(value));
    return { success: true, data: undefined };
  } catch (e) {
    // Handle QuotaExceededError or other storage errors
    const message = e instanceof Error ? e.message : 'Unknown storage error';
    return { success: false, error: message };
  }
}

/**
 * Remove a value from localStorage.
 *
 * @param key - The localStorage key to remove
 * @returns Result indicating success or failure
 */
export function removeStorageItem(key: string): StorageResult<void> {
  if (!browser) {
    return { success: false, error: 'localStorage not available (not in browser)' };
  }

  try {
    localStorage.removeItem(key);
    return { success: true, data: undefined };
  } catch (e) {
    const message = e instanceof Error ? e.message : 'Unknown storage error';
    return { success: false, error: message };
  }
}

/**
 * Check if localStorage is available and working.
 *
 * @returns true if localStorage is available and functional
 */
export function isStorageAvailable(): boolean {
  if (!browser) {
    return false;
  }

  try {
    const testKey = '__tusk_storage_test__';
    localStorage.setItem(testKey, 'test');
    localStorage.removeItem(testKey);
    return true;
  } catch {
    return false;
  }
}

/**
 * Storage keys used by Tusk application.
 */
export const STORAGE_KEYS = {
  TABS: 'tusk-tabs',
  ACTIVE_TAB: 'tusk-active-tab',
  UI_STATE: 'tusk-ui-state',
  THEME: 'tusk-theme',
} as const;

export type StorageKey = (typeof STORAGE_KEYS)[keyof typeof STORAGE_KEYS];
