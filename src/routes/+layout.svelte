<script lang="ts">
	import '../app.css';
	import { theme, uiStore, tabStore } from '$lib/stores';
	import { isModifierPressed, isInputElement, matchesShortcut } from '$lib/utils';
	import Shell from '$lib/components/shell/Shell.svelte';
	import ConfirmDialog from '$lib/components/dialogs/ConfirmDialog.svelte';

	let { children } = $props();

	// Theme store initializes and applies dark class automatically on import
	// Access theme.mode to ensure the store is initialized
	$effect(() => {
		void theme.mode;
	});

	/**
	 * Global keyboard shortcut handler.
	 */
	function handleKeyDown(e: KeyboardEvent) {
		// Skip shortcuts in input elements unless explicitly allowed
		if (isInputElement(e)) {
			return;
		}

		// Cmd/Ctrl+B - Toggle sidebar
		if (matchesShortcut(e, 'b', true)) {
			e.preventDefault();
			uiStore.toggleSidebar();
			return;
		}

		// Cmd/Ctrl+T - New tab
		if (matchesShortcut(e, 't', true)) {
			e.preventDefault();
			tabStore.createTab('query');
			return;
		}

		// Cmd/Ctrl+W - Close current tab
		if (matchesShortcut(e, 'w', true)) {
			e.preventDefault();
			if (tabStore.activeTabId) {
				tabStore.closeTab(tabStore.activeTabId);
			}
			return;
		}

		// Cmd/Ctrl+Tab - Next tab
		if (e.key === 'Tab' && isModifierPressed(e) && !e.shiftKey) {
			e.preventDefault();
			cycleTab(1);
			return;
		}

		// Cmd/Ctrl+Shift+Tab - Previous tab
		if (e.key === 'Tab' && isModifierPressed(e) && e.shiftKey) {
			e.preventDefault();
			cycleTab(-1);
			return;
		}

		// Cmd/Ctrl+Shift+L - Toggle theme (Light/Dark)
		if (matchesShortcut(e, 'l', true, true)) {
			e.preventDefault();
			theme.toggle();
			return;
		}
	}

	/**
	 * Cycle through tabs in the specified direction.
	 */
	function cycleTab(direction: 1 | -1) {
		const tabs = tabStore.tabs;
		if (tabs.length === 0) return;

		const currentIndex = tabs.findIndex((t) => t.id === tabStore.activeTabId);
		if (currentIndex === -1) {
			tabStore.setActiveTab(tabs[0].id);
			return;
		}

		let newIndex = currentIndex + direction;
		if (newIndex < 0) newIndex = tabs.length - 1;
		if (newIndex >= tabs.length) newIndex = 0;

		tabStore.setActiveTab(tabs[newIndex].id);
	}
</script>

<svelte:window onkeydown={handleKeyDown} />

<Shell>
	{@render children()}
</Shell>

<!-- Unsaved changes confirmation dialog -->
{#if tabStore.showUnsavedDialog && tabStore.pendingUnsavedTab}
	<ConfirmDialog
		title="Unsaved Changes"
		message={`Do you want to save changes to "${tabStore.pendingUnsavedTab.title}"?`}
		confirmLabel="Save"
		discardLabel="Discard"
		cancelLabel="Cancel"
		onConfirm={() => tabStore.resolvePendingClose('save')}
		onDiscard={() => tabStore.resolvePendingClose('discard')}
		onCancel={() => tabStore.resolvePendingClose('cancel')}
	/>
{/if}
