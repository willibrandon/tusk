/**
 * UI store for persistent layout preferences.
 *
 * @module stores/ui
 */

import { browser } from '$app/environment';
import type { UIState, UIStoreInterface } from '$lib/types';
import {
	SIDEBAR_DEFAULT_WIDTH,
	RESULTS_DEFAULT_HEIGHT,
	clampSidebarWidth,
	clampResultsHeight
} from '$lib/types';
import { getStorageItem, setStorageItem, STORAGE_KEYS } from '$lib/utils';

/**
 * Default UI state values.
 */
const DEFAULT_STATE: UIState = {
	sidebarWidth: SIDEBAR_DEFAULT_WIDTH,
	sidebarCollapsed: false,
	resultsPanelHeight: RESULTS_DEFAULT_HEIGHT
};

/**
 * Create the UI store with Svelte 5 runes pattern.
 */
function createUIStore(): UIStoreInterface {
	// Load initial state from localStorage
	const stored = browser
		? getStorageItem<UIState>(STORAGE_KEYS.UI_STATE, DEFAULT_STATE)
		: DEFAULT_STATE;

	// Initialize state with validation
	let sidebarWidth = $state(clampSidebarWidth(stored.sidebarWidth ?? DEFAULT_STATE.sidebarWidth));
	let sidebarCollapsed = $state(stored.sidebarCollapsed ?? DEFAULT_STATE.sidebarCollapsed);
	let resultsPanelHeight = $state(
		clampResultsHeight(stored.resultsPanelHeight ?? DEFAULT_STATE.resultsPanelHeight)
	);

	// Track first run to avoid persisting initial load
	let isFirstRun = true;

	// Persist state changes to localStorage
	if (browser) {
		$effect.root(() => {
			$effect(() => {
				// Access all state to track dependencies
				const state: UIState = {
					sidebarWidth,
					sidebarCollapsed,
					resultsPanelHeight
				};

				if (!isFirstRun) {
					setStorageItem(STORAGE_KEYS.UI_STATE, state);
				}
				isFirstRun = false;
			});
		});
	}

	return {
		get sidebarWidth() {
			return sidebarWidth;
		},

		get sidebarCollapsed() {
			return sidebarCollapsed;
		},

		get resultsPanelHeight() {
			return resultsPanelHeight;
		},

		setSidebarWidth(width: number) {
			sidebarWidth = clampSidebarWidth(width);
		},

		toggleSidebar() {
			sidebarCollapsed = !sidebarCollapsed;
		},

		setSidebarCollapsed(collapsed: boolean) {
			sidebarCollapsed = collapsed;
		},

		setResultsPanelHeight(height: number) {
			resultsPanelHeight = clampResultsHeight(height);
		}
	};
}

export const uiStore = createUIStore();
