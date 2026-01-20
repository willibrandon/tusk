/**
 * Tab store for managing open tabs and active tab selection.
 *
 * @module stores/tabs
 */

import { browser } from '$app/environment';
import type {
	Tab,
	TabType,
	TabContent,
	QueryTabContent,
	TabStoreInterface,
	CreateTabOptions,
	CloseResult,
	UnsavedChangesResult
} from '$lib/types';
import { getStorageItem, setStorageItem, STORAGE_KEYS } from '$lib/utils';

/**
 * Generate a UUID v4.
 */
function generateId(): string {
	return crypto.randomUUID();
}

/**
 * Generate a counter for untitled tabs.
 */
let untitledCounter = 1;

/**
 * Generate a default title for a new tab.
 */
function generateTitle(type: TabType): string {
	switch (type) {
		case 'query':
			return `Query ${untitledCounter++}`;
		case 'table':
			return 'Table';
		case 'view':
			return 'View';
		case 'function':
			return 'Function';
		case 'schema':
			return 'Schema';
	}
}

/**
 * Create default content for a tab type.
 */
function createDefaultContent(type: TabType): TabContent {
	switch (type) {
		case 'query':
			return {
				type: 'query',
				sql: '',
				cursorPosition: { line: 1, column: 1 },
				selectionRange: null,
				results: null
			} satisfies QueryTabContent;
		case 'table':
			return {
				type: 'table',
				schema: 'public',
				table: '',
				filters: [],
				sortColumn: null,
				sortDirection: 'asc',
				offset: 0,
				limit: 100
			};
		case 'view':
			return {
				type: 'view',
				schema: 'public',
				view: '',
				filters: [],
				sortColumn: null,
				sortDirection: 'asc'
			};
		case 'function':
			return {
				type: 'function',
				schema: 'public',
				name: '',
				source: '',
				cursorPosition: { line: 1, column: 1 }
			};
		case 'schema':
			return {
				type: 'schema',
				schema: 'public',
				expandedNodes: []
			};
	}
}

/**
 * Stored tab state (what goes to localStorage).
 */
interface StoredTabState {
	tabs: Tab[];
	activeTabId: string | null;
}

/**
 * Dialog resolver for unsaved changes dialog.
 * This will be set by the ConfirmDialog component.
 */
let unsavedChangesResolver: ((result: UnsavedChangesResult) => void) | null = null;
let pendingUnsavedTab: Tab | null = null;

/**
 * Create the tab store with Svelte 5 runes pattern.
 */
function createTabStore(): TabStoreInterface & {
	// Additional methods for dialog integration
	readonly pendingUnsavedTab: Tab | null;
	readonly showUnsavedDialog: boolean;
	setDialogResolver(resolver: ((result: UnsavedChangesResult) => void) | null): void;
	resolvePendingClose(result: UnsavedChangesResult): void;
} {
	// Load initial state from localStorage
	const stored = browser
		? getStorageItem<StoredTabState>(STORAGE_KEYS.TABS, { tabs: [], activeTabId: null })
		: { tabs: [], activeTabId: null };

	// Validate stored tabs
	const validTabs = Array.isArray(stored.tabs)
		? stored.tabs.filter(
				(t): t is Tab => t && typeof t.id === 'string' && typeof t.title === 'string'
			)
		: [];

	// Initialize state
	let tabs = $state<Tab[]>(validTabs);
	let activeTabId = $state<string | null>(
		stored.activeTabId && validTabs.some((t) => t.id === stored.activeTabId)
			? stored.activeTabId
			: validTabs.length > 0
				? validTabs[0].id
				: null
	);
	let showUnsavedDialog = $state(false);

	// Update untitled counter based on existing tabs
	if (validTabs.length > 0) {
		const queryTitles = validTabs
			.filter((t) => t.type === 'query')
			.map((t) => t.title)
			.filter((title) => title.startsWith('Query '));

		const numbers = queryTitles
			.map((title) => parseInt(title.replace('Query ', ''), 10))
			.filter((n) => !isNaN(n));

		if (numbers.length > 0) {
			untitledCounter = Math.max(...numbers) + 1;
		}
	}

	// Track first run to avoid persisting initial load
	let isFirstRun = true;

	// Persist state changes to localStorage
	if (browser) {
		$effect.root(() => {
			$effect(() => {
				const state: StoredTabState = {
					tabs: tabs,
					activeTabId: activeTabId
				};

				if (!isFirstRun) {
					setStorageItem(STORAGE_KEYS.TABS, state);
				}
				isFirstRun = false;
			});
		});
	}

	// Derived values
	const activeTab = $derived(tabs.find((t) => t.id === activeTabId) ?? null);
	const hasUnsavedChanges = $derived(tabs.some((t) => t.isModified));

	return {
		get tabs() {
			return tabs;
		},

		get activeTabId() {
			return activeTabId;
		},

		get activeTab() {
			return activeTab;
		},

		get hasUnsavedChanges() {
			return hasUnsavedChanges;
		},

		get pendingUnsavedTab() {
			return pendingUnsavedTab;
		},

		get showUnsavedDialog() {
			return showUnsavedDialog;
		},

		createTab(type: TabType, options?: CreateTabOptions): Tab {
			const defaultContent = createDefaultContent(type);
			const newTab: Tab = {
				id: generateId(),
				type,
				title: options?.title ?? generateTitle(type),
				connectionId: options?.connectionId ?? null,
				isModified: false,
				content: defaultContent,
				createdAt: Date.now()
			};

			tabs = [...tabs, newTab];
			activeTabId = newTab.id;

			return newTab;
		},

		async closeTab(id: string): Promise<CloseResult> {
			const tabIndex = tabs.findIndex((t) => t.id === id);
			if (tabIndex === -1) {
				return 'closed';
			}

			const tab = tabs[tabIndex];

			// Check for unsaved changes
			if (tab.isModified) {
				// Store pending tab and show dialog
				pendingUnsavedTab = tab;
				showUnsavedDialog = true;

				// Wait for dialog result
				const result = await new Promise<UnsavedChangesResult>((resolve) => {
					unsavedChangesResolver = resolve;
				});

				// Clean up
				pendingUnsavedTab = null;
				showUnsavedDialog = false;
				unsavedChangesResolver = null;

				if (result === 'cancel') {
					return 'cancelled';
				}

				if (result === 'save') {
					// In a real implementation, this would save the tab content
					// For now, we just mark it as saved and close
					return 'saved';
				}

				// result === 'discard' - fall through to close
			}

			// Remove the tab
			const newTabs = tabs.filter((t) => t.id !== id);

			// Update active tab if needed
			if (activeTabId === id) {
				if (newTabs.length === 0) {
					activeTabId = null;
				} else {
					// Activate adjacent tab (prefer next, fall back to previous)
					const newIndex = Math.min(tabIndex, newTabs.length - 1);
					activeTabId = newTabs[newIndex].id;
				}
			}

			tabs = newTabs;
			return 'closed';
		},

		setActiveTab(id: string) {
			if (tabs.some((t) => t.id === id)) {
				activeTabId = id;
			}
		},

		updateTab(id: string, updates: Partial<Tab>) {
			const index = tabs.findIndex((t) => t.id === id);
			if (index !== -1) {
				tabs = tabs.map((t, i) => (i === index ? { ...t, ...updates } : t));
			}
		},

		reorderTabs(newOrder: Tab[]) {
			// Validate that the new order contains the same tabs
			if (newOrder.length !== tabs.length) return;

			const currentIds = new Set(tabs.map((t) => t.id));
			const newIds = new Set(newOrder.map((t) => t.id));

			if (currentIds.size !== newIds.size) return;
			for (const id of currentIds) {
				if (!newIds.has(id)) return;
			}

			tabs = newOrder;
		},

		markModified(id: string, modified: boolean) {
			const index = tabs.findIndex((t) => t.id === id);
			if (index !== -1 && tabs[index].isModified !== modified) {
				tabs = tabs.map((t, i) => (i === index ? { ...t, isModified: modified } : t));
			}
		},

		getTab(id: string): Tab | undefined {
			return tabs.find((t) => t.id === id);
		},

		setDialogResolver(resolver: ((result: UnsavedChangesResult) => void) | null) {
			unsavedChangesResolver = resolver;
		},

		resolvePendingClose(result: UnsavedChangesResult) {
			if (unsavedChangesResolver) {
				unsavedChangesResolver(result);
			}
		}
	};
}

export const tabStore = createTabStore();
