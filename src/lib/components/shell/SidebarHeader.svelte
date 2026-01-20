<script lang="ts">
	import Button from '$lib/components/common/Button.svelte';
	import Icon from '$lib/components/common/Icon.svelte';

	interface Props {
		title?: string;
		onNewConnection?: () => void;
		class?: string;
	}

	let { title = 'Connections', onNewConnection, class: className = '' }: Props = $props();
</script>

<div
	class="sidebar-header flex h-12 items-center justify-between border-b border-gray-200 px-3 dark:border-gray-700 {className}"
>
	<h2 class="text-sm font-semibold text-gray-900 dark:text-gray-100">
		{title}
	</h2>
	<Button
		variant="ghost"
		size="sm"
		onclick={onNewConnection}
		aria-label="New connection"
		title="New connection"
	>
		<Icon name="plus" size={16} />
	</Button>
</div>

<style>
	.sidebar-header {
		user-select: none;
		-webkit-user-select: none;
	}
</style>
