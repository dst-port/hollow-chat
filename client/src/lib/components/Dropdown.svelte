<script lang="ts">
	import { fly } from "svelte/transition";
	import { cubicOut } from "svelte/easing";
	import Check from "@lucide/svelte/icons/check";
	import ChevronDown from "@lucide/svelte/icons/chevron-down";
	import { clickOutside } from "$lib/actions/clickOutside";

	let { value, options, onChange, placeholder = "" }: {
		value: string;
		options: { value: string; label: string }[];
		onChange: (value: string) => void;
		placeholder?: string;
	} = $props();

	let open = $state(false);

	const selectedLabel = $derived(options.find((o) => o.value === value)?.label ?? placeholder);

	function pick(next: string) {
		onChange(next);
		open = false;
	}
</script>

<div class="dropdown" use:clickOutside={() => (open = false)}>
	<button type="button" class="trigger" onclick={() => (open = !open)}>
		<span>{selectedLabel}</span>
		<ChevronDown size={14} strokeWidth={2.25} class={open ? "flipped" : ""} />
	</button>
	{#if open}
		<div class="menu" transition:fly={{ y: -4, duration: 120, easing: cubicOut }}>
			{#each options as option (option.value)}
				<button type="button" class="option" class:active={option.value === value} onclick={() => pick(option.value)}>
					<span>{option.label}</span>
					{#if option.value === value}<Check size={13} strokeWidth={2.5} />{/if}
				</button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.dropdown {
		position: relative;
	}

	.trigger {
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 7px 9px;
		border-radius: 6px;
		background: var(--active);
		color: var(--ink);
		font-size: 13px;
		text-align: left;
		transition: background-color 0.15s ease;
	}

	.trigger:hover {
		background: var(--hover);
	}

	.trigger span {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.trigger :global(svg) {
		flex-shrink: 0;
		color: var(--ink-faint);
		transition: transform 0.15s ease;
	}

	.trigger :global(svg.flipped) {
		transform: rotate(180deg);
	}

	.menu {
		position: absolute;
		top: calc(100% + 4px);
		left: 0;
		right: 0;
		max-height: 220px;
		overflow-y: auto;
		padding: 4px;
		border-radius: 8px;
		background: var(--panel-raised, var(--panel));
		box-shadow: 0 12px 28px rgba(0, 0, 0, 0.45);
		z-index: 80;
	}

	.option {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		width: 100%;
		padding: 7px 9px;
		border-radius: 6px;
		color: var(--ink-dim);
		font-size: 13px;
		text-align: left;
	}

	.option:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.option.active {
		color: var(--ink);
		font-weight: 600;
	}

	.option :global(svg) {
		flex-shrink: 0;
		color: var(--online);
	}
</style>
