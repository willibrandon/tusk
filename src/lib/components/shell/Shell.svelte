<script lang="ts">
  import type { Snippet } from 'svelte';
  import { uiStore } from '$lib/stores';
  import Sidebar from './Sidebar.svelte';
  import Resizer from './Resizer.svelte';
  import TabBar from './TabBar.svelte';
  import StatusBar from './StatusBar.svelte';

  interface Props {
    children?: Snippet;
    class?: string;
  }

  let { children, class: className = '' }: Props = $props();

  // Get sidebar state
  const isCollapsed = $derived(uiStore.sidebarCollapsed);

  function handleResize(delta: number) {
    const newWidth = uiStore.sidebarWidth + delta;
    uiStore.setSidebarWidth(newWidth);
  }

  function handleResizeStart() {
    document.body.classList.add('resizing');
  }

  function handleResizeEnd() {
    document.body.classList.remove('resizing');
  }
</script>

<div class="shell flex h-screen flex-col bg-gray-100 text-gray-900 dark:bg-gray-900 dark:text-gray-100 {className}">
  <!-- Main content area (sidebar + main) -->
  <div class="flex flex-1 overflow-hidden">
    <!-- Sidebar -->
    <Sidebar />

    <!-- Resizer (hidden when sidebar collapsed) -->
    {#if !isCollapsed}
      <Resizer
        direction="horizontal"
        onResize={handleResize}
        onResizeStart={handleResizeStart}
        onResizeEnd={handleResizeEnd}
      />
    {/if}

    <!-- Main area -->
    <main class="flex flex-1 flex-col overflow-hidden">
      <!-- Tab bar -->
      <TabBar />

      <!-- Content area -->
      <div class="flex-1 overflow-auto">
        {#if children}
          {@render children()}
        {/if}
      </div>
    </main>
  </div>

  <!-- Status bar -->
  <StatusBar />
</div>

<style>
  .shell {
    /* Prevent overflow and ensure full viewport coverage */
    overflow: hidden;
  }

  /* Global styles for resize state */
  :global(body.resizing) {
    cursor: col-resize !important;
    user-select: none !important;
    -webkit-user-select: none !important;
  }

  :global(body.resizing *) {
    cursor: col-resize !important;
  }
</style>
