<script lang="ts">
	import Heart from "@lucide/svelte/icons/heart";
	import Hammer from "@lucide/svelte/icons/hammer";
	import Code2 from "@lucide/svelte/icons/code-2";
	import Crown from "@lucide/svelte/icons/crown";
	import ShieldCheck from "@lucide/svelte/icons/shield-check";
	import Award from "@lucide/svelte/icons/award";
	import { badgeStore } from "$lib/stores/badges.svelte";

	let { badges }: { badges: string[] } = $props();

	const ICONS: Record<string, typeof Heart> = {
		supporter: Heart,
		"dev-contributor": Hammer,
		developer: Code2,
		owner: Crown,
		staff: ShieldCheck
	};

	const COLORS: Record<string, string> = {
		supporter: "#d9718a",
		"dev-contributor": "#5b96c9",
		developer: "#7fa88a",
		owner: "#c9a227",
		staff: "#3ba55d"
	};

	const FALLBACK_COLOR = "#8f97a8";
</script>

{#if badges.length > 0}
	<div class="badges">
		{#each badges as badge (badge)}
			{@const Icon = ICONS[badge] ?? Award}
			{@const meta = badgeStore.catalog[badge]}
			<span
				class="badge"
				style:color={COLORS[badge] ?? FALLBACK_COLOR}
				title={meta ? `${meta.label} — ${meta.description}` : badge}
			>
				<Icon size={13} strokeWidth={2.25} />
			</span>
		{/each}
	</div>
{/if}

<style>
	.badges {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.badge {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 20px;
		height: 20px;
		border-radius: 50%;
		background: var(--sidebar);
	}
</style>
