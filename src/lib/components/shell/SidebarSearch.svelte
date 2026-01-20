<script lang="ts">
	import Icon from '$lib/components/common/Icon.svelte';

	interface Props {
		value?: string;
		placeholder?: string;
		onInput?: (value: string) => void;
		class?: string;
	}

	let {
		value = $bindable(''),
		placeholder = 'Filter connections...',
		onInput,
		class: className = ''
	}: Props = $props();

	function handleInput(e: Event) {
		const target = e.target as HTMLInputElement;
		value = target.value;
		onInput?.(value);
	}

	function handleClear() {
		value = '';
		onInput?.('');
	}
</script>

<div class="sidebar-search relative px-3 py-2 {className}">
	<div class="relative">
		<Icon
			name="search"
			size={14}
			class="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-gray-400"
		/>
		<input
			type="text"
			{value}
			{placeholder}
			oninput={handleInput}
			class="w-full rounded-md border border-gray-200 bg-white py-1.5 pl-8 pr-8 text-sm placeholder:text-gray-400 focus:border-tusk-500 focus:outline-none focus:ring-1 focus:ring-tusk-500 dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100 dark:placeholder:text-gray-500 dark:focus:border-tusk-400 dark:focus:ring-tusk-400"
		/>
		{#if value}
			<button
				type="button"
				onclick={handleClear}
				class="absolute right-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-gray-400 hover:bg-gray-100 hover:text-gray-600 dark:hover:bg-gray-600 dark:hover:text-gray-300"
				aria-label="Clear search"
			>
				<Icon name="x" size={14} />
			</button>
		{/if}
	</div>
</div>
