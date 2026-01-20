<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';

	interface AppInfo {
		name: string;
		version: string;
		tauriVersion: string;
		platform: string;
	}

	let appInfo: AppInfo | null = $state(null);
	let error: string | null = $state(null);

	onMount(async () => {
		try {
			appInfo = await invoke<AppInfo>('get_app_info');
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	});
</script>

<div class="flex h-screen bg-gray-100 text-gray-900 dark:bg-gray-900 dark:text-gray-100">
	<!-- Sidebar -->
	<aside class="w-64 border-r border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-800">
		<div class="flex h-14 items-center border-b border-gray-200 px-4 dark:border-gray-700">
			<h1 class="text-lg font-semibold">Tusk</h1>
		</div>
		<nav class="p-4">
			<p class="text-sm text-gray-500 dark:text-gray-400">Connection browser will appear here</p>
		</nav>
	</aside>

	<!-- Main Content -->
	<main class="flex flex-1 flex-col">
		<!-- Header -->
		<header class="flex h-14 items-center border-b border-gray-200 bg-white px-4 dark:border-gray-700 dark:bg-gray-800">
			<span class="text-sm text-gray-600 dark:text-gray-300">Query Editor</span>
		</header>

		<!-- Content Area -->
		<div class="flex flex-1 items-center justify-center p-8">
			<div class="text-center">
				<h2 class="mb-4 text-2xl font-bold text-gray-900 dark:text-white">Welcome to Tusk</h2>
				<p class="mb-6 text-gray-600 dark:text-gray-400">A fast, free, native Postgres client</p>

				{#if error}
					<div class="rounded-lg border border-red-200 bg-red-50 p-4 text-red-800 dark:border-red-800 dark:bg-red-900/20 dark:text-red-200">
						<p class="font-medium">Error loading app info</p>
						<p class="text-sm">{error}</p>
					</div>
				{:else if appInfo}
					<div class="rounded-lg border border-gray-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-800">
						<dl class="grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
							<dt class="text-gray-500 dark:text-gray-400">App Name:</dt>
							<dd class="font-medium text-gray-900 dark:text-white">{appInfo.name}</dd>
							<dt class="text-gray-500 dark:text-gray-400">Version:</dt>
							<dd class="font-medium text-gray-900 dark:text-white">{appInfo.version}</dd>
							<dt class="text-gray-500 dark:text-gray-400">Tauri:</dt>
							<dd class="font-medium text-gray-900 dark:text-white">{appInfo.tauriVersion}</dd>
							<dt class="text-gray-500 dark:text-gray-400">Platform:</dt>
							<dd class="font-medium text-gray-900 dark:text-white">{appInfo.platform}</dd>
						</dl>
					</div>
				{:else}
					<div class="flex items-center justify-center">
						<div class="h-6 w-6 animate-spin rounded-full border-2 border-gray-300 border-t-blue-500"></div>
						<span class="ml-2 text-gray-500 dark:text-gray-400">Loading...</span>
					</div>
				{/if}
			</div>
		</div>

		<!-- Status Bar -->
		<footer class="flex h-6 items-center border-t border-gray-200 bg-gray-50 px-4 text-xs text-gray-500 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-400">
			<span>Ready</span>
			<span class="ml-auto">{appInfo?.platform ?? 'Loading...'}</span>
		</footer>
	</main>
</div>
