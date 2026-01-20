<script lang="ts">
	import { tabStore } from '$lib/stores';
	import type { QueryTabContent } from '$lib/types';
	import Icon from '$lib/components/common/Icon.svelte';
	import Button from '$lib/components/common/Button.svelte';

	const activeTab = $derived(tabStore.activeTab);
	const hasNoTabs = $derived(tabStore.tabs.length === 0);

	// Local SQL value to avoid controlled input issues with key repeat
	let localSql = $state('');
	let lastActiveTabId = $state<string | null>(null);

	// Sync local SQL with active tab when tab changes
	$effect(() => {
		if (activeTab && activeTab.id !== lastActiveTabId) {
			lastActiveTabId = activeTab.id;
			if (activeTab.content.type === 'query') {
				localSql = activeTab.content.sql;
			}
		}
	});

	function handleNewTab() {
		tabStore.createTab('query');
	}

	function handleSqlInput(e: Event) {
		const target = e.target as HTMLTextAreaElement;
		localSql = target.value;
		if (activeTab && activeTab.content.type === 'query') {
			const updatedContent: QueryTabContent = {
				...activeTab.content,
				sql: target.value
			};
			tabStore.updateTab(activeTab.id, {
				content: updatedContent,
				isModified: true
			});
		}
	}
</script>

<div class="flex h-full items-center justify-center bg-gray-50 dark:bg-gray-900">
	{#if hasNoTabs}
		<!-- Empty state when no tabs are open -->
		<div class="text-center">
			<div class="mb-4 flex justify-center">
				<div class="rounded-full bg-gray-200 p-4 dark:bg-gray-700">
					<Icon name="code" size={32} class="text-gray-400 dark:text-gray-500" />
				</div>
			</div>
			<h2 class="mb-2 text-lg font-semibold text-gray-900 dark:text-white">No tabs open</h2>
			<p class="mb-4 text-sm text-gray-500 dark:text-gray-400">
				Create a new query tab to get started
			</p>
			<Button variant="primary" onclick={handleNewTab}>
				<Icon name="plus" size={16} />
				New Query
			</Button>
		</div>
	{:else if activeTab}
		<!-- Active tab content -->
		<div class="flex h-full w-full flex-col p-4">
			<div class="mb-2 text-sm text-gray-500 dark:text-gray-400">
				{activeTab.type === 'query' ? 'Query Editor' : activeTab.type}
			</div>
			<div
				class="flex-1 rounded-md border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800"
			>
				{#if activeTab.type === 'query' && activeTab.content.type === 'query'}
					<!-- Query editor -->
					<textarea
						class="h-full w-full resize-none border-0 bg-transparent font-mono text-sm focus:outline-none dark:text-gray-100"
						placeholder="-- Enter your SQL query here..."
						bind:value={localSql}
						oninput={handleSqlInput}
					></textarea>
				{:else if activeTab.type === 'table'}
					<!-- Table viewer. See docs/features/17-table-data-viewer.md -->
					<p class="text-gray-500 dark:text-gray-400">
						Select a table from the sidebar to view its data.
					</p>
				{:else if activeTab.type === 'view'}
					<!-- View viewer. See docs/features/17-table-data-viewer.md -->
					<p class="text-gray-500 dark:text-gray-400">
						Select a view from the sidebar to view its data.
					</p>
				{:else if activeTab.type === 'function'}
					<!-- Function editor. See docs/features/16-schema-browser.md -->
					<p class="text-gray-500 dark:text-gray-400">
						Select a function from the sidebar to view or edit it.
					</p>
				{:else if activeTab.type === 'schema'}
					<!-- Schema browser. See docs/features/16-schema-browser.md -->
					<p class="text-gray-500 dark:text-gray-400">
						Select a schema from the sidebar to browse its objects.
					</p>
				{:else}
					<p class="text-gray-500 dark:text-gray-400">
						Unknown tab type: {activeTab.type}
					</p>
				{/if}
			</div>
		</div>
	{/if}
</div>
