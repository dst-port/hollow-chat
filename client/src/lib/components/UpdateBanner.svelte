<script lang="ts">
	import { fly } from "svelte/transition";
	import Download from "@lucide/svelte/icons/download";
	import RefreshCw from "@lucide/svelte/icons/refresh-cw";
	import X from "@lucide/svelte/icons/x";
	import { updater } from "$lib/stores/updater.svelte";
</script>

{#if updater.status === "available" || updater.status === "downloading" || updater.status === "ready"}
	<div class="banner" transition:fly={{ y: 16, duration: 160 }}>
		<div class="icon">
			{#if updater.status === "ready"}
				<RefreshCw size={16} strokeWidth={2.25} />
			{:else}
				<Download size={16} strokeWidth={2.25} />
			{/if}
		</div>
		<div class="body">
			{#if updater.status === "available"}
				<p class="title">Update available — v{updater.version}</p>
				<p class="sub">Download and install the latest version.</p>
			{:else if updater.status === "downloading"}
				<p class="title">Downloading update…</p>
				<div class="progress"><div class="progress-fill" style:width={`${Math.round(updater.progress * 100)}%`}></div></div>
			{:else}
				<p class="title">Update ready</p>
				<p class="sub">Restart HollowChat to finish installing.</p>
			{/if}
		</div>
		{#if updater.status === "available"}
			<button class="action" onclick={() => updater.downloadAndInstall()}>Update</button>
			<button class="dismiss" aria-label="Dismiss" onclick={() => updater.dismiss()}>
				<X size={14} strokeWidth={2.25} />
			</button>
		{:else if updater.status === "ready"}
			<button class="action" onclick={() => updater.restart()}>Restart</button>
		{/if}
	</div>
{/if}

<style>
	.banner {
		position: fixed;
		right: 16px;
		bottom: 16px;
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 12px;
		max-width: 320px;
		border-radius: 10px;
		background: var(--panel-raised, var(--panel));
		border: 1px solid var(--hairline);
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
		z-index: 250;
	}

	.icon {
		flex-shrink: 0;
		width: 30px;
		height: 30px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--accent-soft);
		color: var(--accent-fill);
	}

	.body {
		flex: 1;
		min-width: 0;
	}

	.title {
		margin: 0;
		font-size: 12px;
		font-weight: 700;
		color: var(--ink);
	}

	.sub {
		margin: 2px 0 0;
		font-size: 11px;
		color: var(--ink-faint);
	}

	.progress {
		margin-top: 6px;
		height: 4px;
		border-radius: 2px;
		background: var(--active);
		overflow: hidden;
	}

	.progress-fill {
		height: 100%;
		background: var(--accent-fill);
		transition: width 0.2s ease;
	}

	.action {
		flex-shrink: 0;
		padding: 6px 12px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-size: 12px;
		font-weight: 700;
	}

	.action:hover {
		filter: brightness(1.08);
	}

	.dismiss {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 22px;
		height: 22px;
		border-radius: 50%;
		color: var(--ink-faint);
	}

	.dismiss:hover {
		background: var(--hover);
		color: var(--ink);
	}
</style>
