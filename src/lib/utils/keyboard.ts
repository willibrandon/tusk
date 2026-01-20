/**
 * Keyboard utility module with platform detection and modifier key helpers.
 *
 * @module utils/keyboard
 */

import { browser } from '$app/environment';

/**
 * Detect if the current platform is macOS.
 */
export const isMac: boolean =
	browser && typeof navigator !== 'undefined'
		? navigator.platform.toUpperCase().includes('MAC')
		: false;

/**
 * Check if the platform-appropriate modifier key is pressed.
 * - macOS: Meta (Cmd) key
 * - Windows/Linux: Ctrl key
 *
 * @param event - The keyboard event
 * @returns true if the modifier key is pressed
 */
export function isModifierPressed(event: KeyboardEvent): boolean {
	return isMac ? event.metaKey : event.ctrlKey;
}

/**
 * Check if Shift modifier is pressed.
 *
 * @param event - The keyboard event
 * @returns true if Shift is pressed
 */
export function isShiftPressed(event: KeyboardEvent): boolean {
	return event.shiftKey;
}

/**
 * Check if Alt/Option modifier is pressed.
 *
 * @param event - The keyboard event
 * @returns true if Alt/Option is pressed
 */
export function isAltPressed(event: KeyboardEvent): boolean {
	return event.altKey;
}

/**
 * Check if the event target is an input element where shortcuts should be ignored.
 *
 * @param event - The keyboard event
 * @returns true if the event originates from an input-like element
 */
export function isInputElement(event: KeyboardEvent): boolean {
	const target = event.target;
	if (!(target instanceof HTMLElement)) {
		return false;
	}

	const tagName = target.tagName.toLowerCase();
	if (tagName === 'input' || tagName === 'textarea' || tagName === 'select') {
		return true;
	}

	// Check for contenteditable
	if (target.isContentEditable) {
		return true;
	}

	return false;
}

/**
 * Format a keyboard shortcut for display.
 *
 * @param key - The key (e.g., 'b', 'Enter')
 * @param modifier - Whether Cmd/Ctrl is required
 * @param shift - Whether Shift is required
 * @param alt - Whether Alt/Option is required
 * @returns Formatted shortcut string (e.g., '⌘B' on Mac, 'Ctrl+B' on Windows)
 */
export function formatShortcutKey(
	key: string,
	modifier = false,
	shift = false,
	alt = false
): string {
	const parts: string[] = [];

	if (modifier) {
		parts.push(isMac ? '⌘' : 'Ctrl');
	}

	if (shift) {
		parts.push(isMac ? '⇧' : 'Shift');
	}

	if (alt) {
		parts.push(isMac ? '⌥' : 'Alt');
	}

	// Format special keys
	const keyMap: Record<string, string> = {
		Enter: isMac ? '↵' : 'Enter',
		Tab: isMac ? '⇥' : 'Tab',
		Escape: 'Esc',
		ArrowUp: '↑',
		ArrowDown: '↓',
		ArrowLeft: '←',
		ArrowRight: '→',
		Backspace: isMac ? '⌫' : 'Backspace',
		Delete: isMac ? '⌦' : 'Del',
		' ': 'Space'
	};

	const displayKey = keyMap[key] ?? key.toUpperCase();
	parts.push(displayKey);

	return isMac ? parts.join('') : parts.join('+');
}

/**
 * Normalize a key from KeyboardEvent to a consistent format.
 *
 * @param key - The key from KeyboardEvent.key
 * @returns Normalized key string
 */
export function normalizeKey(key: string): string {
	// Normalize common key variations
	return key.toLowerCase();
}

/**
 * Check if a keyboard event matches a shortcut definition.
 *
 * @param event - The keyboard event
 * @param key - The expected key (case-insensitive)
 * @param modifier - Whether Cmd/Ctrl should be pressed
 * @param shift - Whether Shift should be pressed
 * @param alt - Whether Alt should be pressed
 * @returns true if the event matches the shortcut
 */
export function matchesShortcut(
	event: KeyboardEvent,
	key: string,
	modifier = false,
	shift = false,
	alt = false
): boolean {
	// Check modifiers
	if (modifier !== isModifierPressed(event)) return false;
	if (shift !== event.shiftKey) return false;
	if (alt !== event.altKey) return false;

	// Check key (case-insensitive)
	return normalizeKey(event.key) === normalizeKey(key);
}
