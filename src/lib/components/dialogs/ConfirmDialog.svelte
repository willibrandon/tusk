<script lang="ts">
	import Button from '$lib/components/common/Button.svelte';

	interface Props {
		title: string;
		message: string;
		confirmLabel?: string;
		discardLabel?: string;
		cancelLabel?: string;
		onConfirm?: () => void;
		onDiscard?: () => void;
		onCancel?: () => void;
	}

	let {
		title,
		message,
		confirmLabel = 'Confirm',
		discardLabel = 'Discard',
		cancelLabel = 'Cancel',
		onConfirm,
		onDiscard,
		onCancel
	}: Props = $props();

	let dialogRef: HTMLDivElement | undefined = $state();

	// Focus trap and initial focus
	$effect(() => {
		if (dialogRef) {
			const firstButton = dialogRef.querySelector('button');
			firstButton?.focus();
		}
	});

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			onCancel?.();
			return;
		}

		// Focus trap
		if (e.key === 'Tab' && dialogRef) {
			const focusable = dialogRef.querySelectorAll<HTMLElement>(
				'button, [tabindex]:not([tabindex="-1"])'
			);
			const first = focusable[0];
			const last = focusable[focusable.length - 1];

			if (e.shiftKey && document.activeElement === first) {
				e.preventDefault();
				last?.focus();
			} else if (!e.shiftKey && document.activeElement === last) {
				e.preventDefault();
				first?.focus();
			}
		}
	}

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			onCancel?.();
		}
	}
</script>

<svelte:window onkeydown={handleKeyDown} />

<div
	class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
	role="presentation"
	onclick={handleBackdropClick}
>
	<div
		bind:this={dialogRef}
		class="mx-4 w-full max-w-md rounded-lg bg-white p-6 shadow-xl dark:bg-gray-800"
		role="dialog"
		aria-modal="true"
		aria-labelledby="dialog-title"
		aria-describedby="dialog-message"
	>
		<h2 id="dialog-title" class="mb-2 text-lg font-semibold text-gray-900 dark:text-white">
			{title}
		</h2>
		<p id="dialog-message" class="mb-6 text-sm text-gray-600 dark:text-gray-300">
			{message}
		</p>
		<div class="flex justify-end gap-3">
			<Button variant="ghost" onclick={onCancel}>
				{cancelLabel}
			</Button>
			{#if onDiscard}
				<Button variant="secondary" onclick={onDiscard}>
					{discardLabel}
				</Button>
			{/if}
			<Button variant="primary" onclick={onConfirm}>
				{confirmLabel}
			</Button>
		</div>
	</div>
</div>
