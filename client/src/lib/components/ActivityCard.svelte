<script module lang="ts">
	const coverCache = new Map<string, string | null>();
</script>

<script lang="ts">
	import Gamepad2 from "@lucide/svelte/icons/gamepad-2";
	import Clock from "@lucide/svelte/icons/clock";
	import BrandIcon from "$lib/components/BrandIcon.svelte";
	import { BRAND_ICONS } from "$lib/data/brandIcons";
	import { session } from "$lib/stores/session.svelte";
	import { gameCover } from "$lib/api/client";

	let { label, application, details, activityState, image, smallImage, smallText, startedAt, partySize, partyMax }: {
		label: string;
		application: string;
		details?: string | null;
		activityState?: string | null;
		image?: string | null;
		smallImage?: string | null;
		smallText?: string | null;
		startedAt?: string | null;
		partySize?: number | null;
		partyMax?: number | null;
	} = $props();

	let smallImageFailed = $state(false);
	$effect(() => {
		smallImage;
		smallImageFailed = false;
	});

	let now = $state(Date.now());

	$effect(() => {
		if (!startedAt) return;
		const ticker = setInterval(() => (now = Date.now()), 1000);
		return () => clearInterval(ticker);
	});

	function formatElapsed(startIso: string, current: number): string {
		const elapsedSec = Math.max(0, Math.floor((current - new Date(startIso).getTime()) / 1000));
		const hours = Math.floor(elapsedSec / 3600);
		const minutes = Math.floor((elapsedSec % 3600) / 60);
		const seconds = elapsedSec % 60;
		const pad = (n: number) => n.toString().padStart(2, "0");
		return hours > 0 ? `${hours}:${pad(minutes)}:${pad(seconds)}` : `${minutes}:${pad(seconds)}`;
	}

	const elapsed = $derived(startedAt ? formatElapsed(startedAt, now) : null);

	let imageFailed = $state(false);
	$effect(() => {
		image;
		imageFailed = false;
	});

	/// Not every game's RPC integration bothers uploading cover art to
	/// Discord's asset CDN - when that's missing (or the fetch just fails),
	/// fall back to our own bundled brand mark for well-known platforms
	/// rather than an empty box. Real logo, not a guess: same Simple Icons
	/// data already used for profile connections.
	const brandKey = $derived(application.toLowerCase().replace(/[^a-z0-9]/g, ""));
	const brandIcon = $derived(BRAND_ICONS[brandKey]);

	// Neither the game's own RPC assets nor our bundled brand marks cover
	// it - ask the server to resolve a real cover via SteamGridDB. Cached
	// module-wide so every open popover/card for the same game shares one
	// lookup instead of re-querying per instance.
	let resolvedCover = $state<string | null>(null);

	$effect(() => {
		const name = application;
		if (image || brandIcon || !name) {
			resolvedCover = null;
			return;
		}
		const cached = coverCache.get(name);
		if (cached !== undefined) {
			resolvedCover = cached;
			return;
		}
		const token = session.token;
		if (!token) return;
		let cancelled = false;
		gameCover(token, name)
			.then((res) => {
				coverCache.set(name, res.url);
				if (!cancelled) resolvedCover = res.url;
			})
			.catch(() => {
				coverCache.set(name, null);
			});
		return () => {
			cancelled = true;
		};
	});
</script>

<div class="activity-card">
	<p class="label">{label}</p>
	<div class="row">
		<div class="cover">
			{#if image && !imageFailed}
				<img src={image} alt="" onerror={() => (imageFailed = true)} />
			{:else if brandIcon}
				<BrandIcon service={brandKey} size={26} />
			{:else if resolvedCover}
				<img src={resolvedCover} alt="" onerror={() => (resolvedCover = null)} />
			{:else}
				<Gamepad2 size={22} strokeWidth={1.5} />
			{/if}
			{#if smallImage && !smallImageFailed}
				<img class="badge" src={smallImage} alt="" title={smallText ?? undefined} onerror={() => (smallImageFailed = true)} />
			{/if}
		</div>
		<div class="info">
			<p class="title">{application}</p>
			{#if details}<p class="line">{details}</p>{/if}
			{#if activityState}<p class="line">{activityState}</p>{/if}
			{#if partySize}
				<p class="line">{partySize}{partyMax ? ` of ${partyMax}` : ""} in party</p>
			{/if}
			{#if elapsed}
				<p class="line elapsed">
					<Clock size={12} strokeWidth={2} />
					{elapsed}
				</p>
			{/if}
		</div>
	</div>
</div>

<style>
	.activity-card {
		margin: 4px 0 8px;
		padding: 10px 12px;
		border-radius: 8px;
		background: var(--panel);
		border: 1px solid var(--hairline);
	}

	.label {
		margin: 0 0 8px;
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--ink-faint);
	}

	.row {
		display: flex;
		gap: 10px;
	}

	.cover {
		position: relative;
		flex-shrink: 0;
		width: 48px;
		height: 48px;
		border-radius: 8px;
		background: var(--void);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--ink-faint);
		overflow: hidden;
	}

	.cover img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.cover .badge {
		position: absolute;
		right: -3px;
		bottom: -3px;
		width: 18px;
		height: 18px;
		border-radius: 50%;
		border: 2px solid var(--panel);
		background: var(--void);
		object-fit: cover;
	}

	.info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
		justify-content: center;
	}

	.title {
		margin: 0;
		font-size: 13px;
		font-weight: 700;
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.line {
		margin: 0;
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: 12px;
		color: var(--ink-dim);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.line.elapsed {
		color: var(--ink-faint);
	}
</style>
