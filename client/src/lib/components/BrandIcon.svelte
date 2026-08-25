<script lang="ts">
	import { BRAND_ICONS } from "$lib/data/brandIcons";

	let { service, size = 14, mono = false, chip = false }: {
		service: string;
		size?: number;
		mono?: boolean;
		chip?: boolean;
	} = $props();

	const icon = $derived(BRAND_ICONS[service]);
</script>

{#if icon}
	{#if chip}
		<span class="chip" style:width={`${size + 8}px`} style:height={`${size + 8}px`}>
			<svg
				width={size}
				height={size}
				viewBox="0 0 24 24"
				fill={mono ? "#1a1a1a" : `#${icon.hex}`}
				xmlns="http://www.w3.org/2000/svg"
				aria-hidden="true"
			>
				<path d={icon.path} />
			</svg>
		</span>
	{:else}
		<svg
			width={size}
			height={size}
			viewBox="0 0 24 24"
			fill={mono ? "currentColor" : `#${icon.hex}`}
			xmlns="http://www.w3.org/2000/svg"
			aria-hidden="true"
		>
			<path d={icon.path} />
		</svg>
	{/if}
{/if}

<style>
	.chip {
		flex-shrink: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border-radius: 50%;
		background: #f2f2f2;
	}
</style>
