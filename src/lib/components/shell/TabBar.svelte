<script lang="ts">
  import { tabStore } from '$lib/stores';
  import type { Tab } from '$lib/types';
  import TabComponent from './Tab.svelte';
  import Button from '$lib/components/common/Button.svelte';
  import Icon from '$lib/components/common/Icon.svelte';

  interface Props {
    class?: string;
  }

  let { class: className = '' }: Props = $props();

  // Drag and drop state
  let draggedTab: Tab | null = $state(null);

  function handleNewTab() {
    tabStore.createTab('query');
  }

  function handleActivate(id: string) {
    tabStore.setActiveTab(id);
  }

  async function handleClose(id: string) {
    await tabStore.closeTab(id);
  }

  function handleDragStart(e: DragEvent, tab: Tab) {
    draggedTab = tab;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', tab.id);
    }
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
  }

  function handleDrop(e: DragEvent, targetTab: Tab) {
    if (!draggedTab || draggedTab.id === targetTab.id) {
      resetDragState();
      return;
    }

    // Calculate new order
    const tabs = [...tabStore.tabs];
    const draggedIndex = tabs.findIndex((t) => t.id === draggedTab!.id);
    const targetIndex = tabs.findIndex((t) => t.id === targetTab.id);

    if (draggedIndex !== -1 && targetIndex !== -1) {
      // Remove dragged tab and insert at target position
      const [removed] = tabs.splice(draggedIndex, 1);
      tabs.splice(targetIndex, 0, removed);
      tabStore.reorderTabs(tabs);
    }

    resetDragState();
  }

  function handleDragEnd() {
    resetDragState();
  }

  function resetDragState() {
    draggedTab = null;
  }
</script>

<svelte:window ondragend={handleDragEnd} />

<div
  class="tab-bar flex h-10 items-center border-b border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-800 {className}"
  role="tablist"
  aria-label="Open tabs"
>
  <!-- Tab container -->
  <div class="flex flex-1 items-center overflow-x-auto">
    {#if tabStore.tabs.length === 0}
      <span class="px-3 text-sm text-gray-500 dark:text-gray-400">
        No tabs open
      </span>
    {:else}
      {#each tabStore.tabs as tab (tab.id)}
        <TabComponent
          {tab}
          isActive={tab.id === tabStore.activeTabId}
          onActivate={handleActivate}
          onClose={handleClose}
          onDragStart={handleDragStart}
          onDragOver={handleDragOver}
          onDrop={handleDrop}
        />
      {/each}
    {/if}
  </div>

  <!-- New Tab button -->
  <div class="flex items-center px-2">
    <Button
      variant="ghost"
      size="sm"
      onclick={handleNewTab}
      aria-label="New tab"
    >
      <Icon name="plus" size={16} />
    </Button>
  </div>
</div>

<style>
  .tab-bar {
    user-select: none;
    -webkit-user-select: none;
  }
</style>
