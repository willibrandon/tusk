<script lang="ts">
	import '../app.css';
	import { theme } from '$lib/stores/theme.svelte';

	let { children } = $props();

	// Theme store initializes and applies dark class automatically on import
	// Access theme.mode to ensure the store is initialized
	$effect(() => {
		// Re-run when theme mode changes to ensure class is applied
		void theme.mode;
	});
</script>

{@render children()}
