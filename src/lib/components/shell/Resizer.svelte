<script lang="ts">
  import type { ResizeDirection } from '$lib/types';

  interface Props {
    direction?: ResizeDirection;
    onResize?: (delta: number) => void;
    onResizeStart?: () => void;
    onResizeEnd?: () => void;
    class?: string;
  }

  let {
    direction = 'horizontal',
    onResize,
    onResizeStart,
    onResizeEnd,
    class: className = '',
  }: Props = $props();

  let isResizing = $state(false);
  let startPosition = $state(0);

  function handlePointerDown(e: PointerEvent) {
    if (e.button !== 0) return; // Only left mouse button

    isResizing = true;
    startPosition = direction === 'horizontal' ? e.clientX : e.clientY;

    // Capture pointer for reliable tracking
    (e.target as HTMLElement).setPointerCapture(e.pointerId);

    onResizeStart?.();
    e.preventDefault();
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isResizing) return;

    const currentPosition = direction === 'horizontal' ? e.clientX : e.clientY;
    const delta = currentPosition - startPosition;

    if (delta !== 0) {
      // Use requestAnimationFrame for smooth 60fps updates
      requestAnimationFrame(() => {
        onResize?.(delta);
      });
      startPosition = currentPosition;
    }
  }

  function handlePointerUp(e: PointerEvent) {
    if (!isResizing) return;

    isResizing = false;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
    onResizeEnd?.();
  }

  function handleKeyDown(e: KeyboardEvent) {
    const step = 10; // pixels per key press
    let delta = 0;

    if (direction === 'horizontal') {
      if (e.key === 'ArrowLeft') delta = -step;
      else if (e.key === 'ArrowRight') delta = step;
    } else {
      if (e.key === 'ArrowUp') delta = -step;
      else if (e.key === 'ArrowDown') delta = step;
    }

    if (delta !== 0) {
      e.preventDefault();
      onResize?.(delta);
    }
  }

  const isHorizontal = $derived(direction === 'horizontal');
</script>

<!-- role="separator" makes this element interactive per ARIA spec -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="resizer {className}"
  class:resizer-horizontal={isHorizontal}
  class:resizer-vertical={!isHorizontal}
  class:resizing={isResizing}
  role="separator"
  aria-orientation={isHorizontal ? 'vertical' : 'horizontal'}
  aria-valuenow={0}
  tabindex="0"
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerUp}
  onkeydown={handleKeyDown}
>
  <div class="resizer-handle"></div>
</div>

<style>
  .resizer {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    touch-action: none;
    user-select: none;
    -webkit-user-select: none;
  }

  .resizer-horizontal {
    width: 4px;
    cursor: col-resize;
  }

  .resizer-vertical {
    height: 4px;
    cursor: row-resize;
  }

  .resizer:hover,
  .resizer:focus-visible,
  .resizer.resizing {
    background: var(--color-tusk-500);
  }

  .resizer:focus-visible {
    outline: 2px solid var(--color-tusk-500);
    outline-offset: 2px;
  }

  .resizer-handle {
    width: 100%;
    height: 100%;
  }

  .resizer-horizontal .resizer-handle {
    width: 4px;
    height: 24px;
  }

  .resizer-vertical .resizer-handle {
    width: 24px;
    height: 4px;
  }

  /* Visual feedback during resize */
  .resizing {
    background: var(--color-tusk-500);
  }

  /* Prevent body scroll during resize */
  :global(body.resizing) {
    cursor: col-resize !important;
    user-select: none !important;
    -webkit-user-select: none !important;
  }

  :global(body.resizing-vertical) {
    cursor: row-resize !important;
  }
</style>
