<script lang="ts">
	import Heart from "@lucide/svelte/icons/heart";
	import Hammer from "@lucide/svelte/icons/hammer";
	import Code2 from "@lucide/svelte/icons/code-2";
	import Crown from "@lucide/svelte/icons/crown";
	import ShieldCheck from "@lucide/svelte/icons/shield-check";
	import { BADGE_META, type BadgeId } from "$lib/data/mock";

	let { badges }: { badges: BadgeId[] } = $props();

	const ICONS: Record<BadgeId, typeof Heart> = {
		supporter: Heart,
		"dev-contributor": Hammer,
		developer: Code2,
		owner: Crown,
		staff: ShieldCheck
	};

	const COLORS: Record<BadgeId, string> = {
		supporter: "#d9718a",
		"dev-contributor": "#5b96c9",
		developer: "#7fa88a",
		owner: "#c9a227",
		staff: "#3ba55d"
	};
</script>

{#if badges.length > 0}
	<div class="badges">
		{#each badges as badge (badge)}
			{@const Icon = ICONS[badge]}
			<span class="badge" style:color={COLORS[badge]} title={`${BADGE_META[badge].label} — ${BADGE_META[badge].description}`}>
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
