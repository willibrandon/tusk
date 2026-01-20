<script lang="ts">
	import type { Tab, TabType } from '$lib/types';
	import Icon from '$lib/components/common/Icon.svelte';
	import { connectionStore } from '$lib/stores';

	interface Props {
		tab: Tab;
		isActive?: boolean;
		isDragOver?: boolean;
		onActivate?: (id: string) => void;
		onClose?: (id: string) => void;
		onDragStart?: (e: DragEvent, tab: Tab) => void;
		onDragEnter?: (e: DragEvent, tab: Tab) => void;
		onDragLeave?: (e: DragEvent, tab: Tab) => void;
		onDragOver?: (e: DragEvent) => void;
		onDrop?: (e: DragEvent, tab: Tab) => void;
	}

	let {
		tab,
		isActive = false,
		isDragOver = false,
		onActivate,
		onClose,
		onDragStart,
		onDragEnter,
		onDragLeave,
		onDragOver,
		onDrop
	}: Props = $props();

	/**
	 * Map tab type to icon name.
	 */
	function getTabIcon(type: TabType): 'code' | 'table' | 'eye' | 'function' | 'schema' {
		switch (type) {
			case 'query':
				return 'code';
			case 'table':
				return 'table';
			case 'view':
				return 'eye';
			case 'function':
				return 'function';
			case 'schema':
				return 'schema';
		}
	}

	/**
	 * Get connection color if tab has a connection.
	 */
	const connectionColor = $derived.by(() => {
		if (!tab.connectionId) return null;
		const connection = connectionStore.getConnection(tab.connectionId);
		return connection?.color ?? null;
	});

	/**
	 * Handle tab click to activate.
	 */
	function handleClick(e: MouseEvent) {
		// Middle click to close
		if (e.button === 1) {
			e.preventDefault();
			onClose?.(tab.id);
			return;
		}
		// Left click to activate
		if (e.button === 0) {
			onActivate?.(tab.id);
		}
	}

	/**
	 * Handle close button click.
	 */
	function handleCloseClick(e: MouseEvent) {
		e.stopPropagation();
		onClose?.(tab.id);
	}

	/**
	 * Handle keyboard activation.
	 */
	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			onActivate?.(tab.id);
		}
	}

	/**
	 * Handle drag start.
	 */
	function handleDragStart(e: DragEvent) {
		onDragStart?.(e, tab);
	}

	/**
	 * Handle drag enter.
	 */
	function handleDragEnter(e: DragEvent) {
		e.preventDefault();
		onDragEnter?.(e, tab);
	}

	/**
	 * Handle drag leave.
	 */
	function handleDragLeave(e: DragEvent) {
		onDragLeave?.(e, tab);
	}

	/**
	 * Handle drag over.
	 */
	function handleDragOver(e: DragEvent) {
		e.preventDefault();
		if (e.dataTransfer) {
			e.dataTransfer.dropEffect = 'move';
		}
		onDragOver?.(e);
	}

	/**
	 * Handle drop.
	 */
	function handleDrop(e: DragEvent) {
		e.preventDefault();
		onDrop?.(e, tab);
	}

	const iconName = $derived(getTabIcon(tab.type));
</script>

<div
	class="tab group relative flex h-9 min-w-[120px] max-w-[200px] cursor-pointer items-center gap-2 border-r border-gray-200 px-3 transition-colors dark:border-gray-700"
	class:tab-active={isActive}
	class:tab-drag-over={isDragOver}
	class:bg-white={isActive}
	class:dark:bg-gray-800={isActive}
	class:bg-gray-100={!isActive}
	class:dark:bg-gray-900={!isActive}
	class:hover:bg-gray-50={!isActive}
	class:dark:hover:bg-gray-800={!isActive}
	role="tab"
	tabindex={isActive ? 0 : -1}
	aria-selected={isActive}
	draggable="true"
	onmousedown={handleClick}
	onkeydown={handleKeyDown}
	ondragstart={handleDragStart}
	ondragenter={handleDragEnter}
	ondragleave={handleDragLeave}
	ondragover={handleDragOver}
	ondrop={handleDrop}
>
	<!-- Connection color indicator -->
	{#if connectionColor}
		<div
			class="absolute bottom-0 left-0 right-0 h-0.5"
			style="background-color: {connectionColor};"
		></div>
	{/if}

	<!-- Tab type icon -->
	<Icon name={iconName} size={14} class="flex-shrink-0 text-gray-500 dark:text-gray-400" />

	<!-- Tab title -->
	<span class="flex-1 truncate text-sm text-gray-700 dark:text-gray-200" title={tab.title}>
		{tab.title}
	</span>

	<!-- Modification indicator (blue dot) -->
	{#if tab.isModified}
		<div class="h-2 w-2 flex-shrink-0 rounded-full bg-blue-500" title="Unsaved changes"></div>
	{/if}

	<!-- Close button -->
	<button
		type="button"
		class="flex-shrink-0 rounded p-0.5 opacity-0 transition-opacity hover:bg-gray-200 group-hover:opacity-100 dark:hover:bg-gray-600"
		class:opacity-100={isActive}
		onclick={handleCloseClick}
		title="Close tab"
		aria-label="Close {tab.title}"
	>
		<Icon name="x" size={14} class="text-gray-500 dark:text-gray-400" />
	</button>
</div>

<style>
	.tab-active {
		border-bottom: 2px solid var(--color-tusk-500);
		margin-bottom: -1px;
	}

	/* Drag over indicator */
	.tab-drag-over {
		border-left: 2px solid var(--color-tusk-500);
	}

	/* Focus styles for keyboard navigation */
	.tab:focus-visible {
		outline: 2px solid var(--color-tusk-500);
		outline-offset: -2px;
	}
</style>
