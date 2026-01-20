<script lang="ts">
	import { connectionStore, tabStore } from '$lib/stores';
	import type { ConnectionState } from '$lib/types';
	import { getConnectionDisplayInfo, isQueryContent } from '$lib/types';

	interface Props {
		class?: string;
	}

	let { class: className = '' }: Props = $props();

	// Get active connection and status
	const connection = $derived(connectionStore.activeConnection);
	const status = $derived(connection ? connectionStore.getStatus(connection.id) : null);
	const displayInfo = $derived(getConnectionDisplayInfo(connection, status ?? null));

	// Get active tab for cursor position and query results
	const activeTab = $derived(tabStore.activeTab);
	const isQueryTab = $derived(activeTab?.type === 'query');
	const queryContent = $derived.by(() => {
		if (activeTab && isQueryContent(activeTab.content)) {
			return activeTab.content;
		}
		return null;
	});

	// Cursor position (from query tab content)
	const cursorPosition = $derived(queryContent?.cursorPosition ?? null);

	// Query results (from query tab content)
	const queryResults = $derived(queryContent?.results ?? null);

	// Color classes for status indicator
	const statusColorClasses: Record<ConnectionState | 'default', string> = {
		connected: 'bg-green-500',
		connecting: 'bg-yellow-500',
		error: 'bg-red-500',
		disconnected: 'bg-gray-400 dark:bg-gray-600',
		default: 'bg-gray-400 dark:bg-gray-600'
	};

	const statusColor = $derived(statusColorClasses[status?.state ?? 'default']);

	/**
	 * Format execution time for display.
	 */
	function formatExecutionTime(ms: number): string {
		if (ms < 1000) {
			return `${ms}ms`;
		}
		return `${(ms / 1000).toFixed(2)}s`;
	}

	/**
	 * Format row count for display.
	 */
	function formatRowCount(count: number, hasMore: boolean): string {
		const formattedCount = count.toLocaleString();
		return hasMore ? `${formattedCount}+ rows` : `${formattedCount} rows`;
	}
</script>

<footer
	class="status-bar flex h-6 items-center border-t border-gray-200 bg-gray-50 px-3 text-xs dark:border-gray-700 dark:bg-gray-800 {className}"
>
	<!-- Connection status (left side) -->
	<div class="flex items-center gap-2">
		<span
			class="status-indicator h-2 w-2 rounded-full {statusColor}"
			aria-label={displayInfo.statusText}
		></span>
		{#if connection}
			<span class="text-gray-700 dark:text-gray-300">
				{connection.name}
			</span>
			<span class="text-gray-500 dark:text-gray-500">
				{connection.host}:{connection.port}
			</span>
		{:else}
			<span class="text-gray-500 dark:text-gray-400">No connection</span>
		{/if}
	</div>

	<!-- Spacer -->
	<div class="flex-1"></div>

	<!-- Query result info (shown after query execution) -->
	{#if queryResults}
		<div class="flex items-center gap-3 text-gray-500 dark:text-gray-400">
			<span>{formatRowCount(queryResults.rowCount, queryResults.hasMore)}</span>
			<span>{formatExecutionTime(queryResults.executionTimeMs)}</span>
		</div>
	{/if}

	<!-- Cursor position (shown when editor tab is active) -->
	{#if isQueryTab && cursorPosition}
		<div class="ml-4 flex items-center gap-2 text-gray-500 dark:text-gray-400">
			<span>Ln {cursorPosition.line}, Col {cursorPosition.column}</span>
		</div>
	{/if}
</footer>

<style>
	.status-bar {
		user-select: none;
		-webkit-user-select: none;
	}

	.status-indicator {
		flex-shrink: 0;
	}
</style>
