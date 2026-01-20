<script lang="ts">
	import { uiStore } from '$lib/stores';
	import SidebarHeader from './SidebarHeader.svelte';
	import SidebarSearch from './SidebarSearch.svelte';

	interface Props {
		class?: string;
	}

	let { class: className = '' }: Props = $props();

	let searchFilter = $state('');

	function handleNewConnection() {
		// Will be implemented when connection feature is added
		console.log('New connection clicked');
	}

	function handleSearchInput(value: string) {
		searchFilter = value;
	}

	// Get sidebar width from store
	const sidebarWidth = $derived(uiStore.sidebarWidth);
	const isCollapsed = $derived(uiStore.sidebarCollapsed);
</script>

{#if !isCollapsed}
	<aside
		class="sidebar flex flex-col border-r border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-800 {className}"
		style="width: {sidebarWidth}px; flex: 0 0 {sidebarWidth}px;"
	>
		<SidebarHeader onNewConnection={handleNewConnection} />
		<SidebarSearch value={searchFilter} onInput={handleSearchInput} />

		<!-- Connection tree placeholder -->
		<div class="flex-1 overflow-y-auto px-3 py-2">
			<p class="text-xs text-gray-500 dark:text-gray-400">
				{#if searchFilter}
					Filtering by: "{searchFilter}"
				{:else}
					Connection tree will appear here
				{/if}
			</p>
		</div>
	</aside>
{/if}

<style>
	.sidebar {
		user-select: none;
		-webkit-user-select: none;
		overflow: hidden;
	}
</style>
