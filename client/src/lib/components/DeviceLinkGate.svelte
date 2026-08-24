<script lang="ts">
	import { fly } from "svelte/transition";
	import { cubicOut } from "svelte/easing";
	import { session } from "$lib/stores/session.svelte";
	import { ensureIdentity, syncIdentityToServer } from "$lib/crypto/identity";
	import { deviceLink } from "$lib/devicelink/link.svelte";

	type Step = "choice" | "linking" | "fresh";
	let step = $state<Step>("choice");
	let freshBusy = $state(false);

	function startLinking() {
		const token = session.token;
		const username = session.username;
		if (!token || !username) return;
		step = "linking";
		deviceLink.start(token, username);
	}

	async function startFresh() {
		const token = session.token;
		const username = session.username;
		if (!token || !username) return;
		step = "fresh";
		freshBusy = true;
		await ensureIdentity(token, username);
		session.completeDeviceSetup();
	}

	function backToChoice() {
		deviceLink.reset();
		step = "choice";
	}

	$effect(() => {
		if (deviceLink.phase !== "done" || step !== "linking") return;
		const token = session.token;
		const username = session.username;
		if (!token || !username) return;
		syncIdentityToServer(token, username)
			.catch(() => {})
			.finally(() => session.completeDeviceSetup());
	});
</script>

<div class="window-frame screen">
	<div class="card">
		<div class="brand">
			<img class="mark" src="/logo/hollowchat-mark.png" alt="" />
			<span class="name">HollowChat</span>
		</div>

		{#key step}
			<div class="pane" in:fly={{ y: 8, duration: 260, easing: cubicOut }}>
				{#if step === "choice"}
					<h1>Set up encryption</h1>
					<p class="subtitle">
						This device doesn't have your encryption keys yet. Link it to a device you're already
						signed in on to keep reading your conversations, or start fresh.
					</p>
					<div class="actions">
						<button type="button" class="primary" onclick={startLinking}>Link with another device</button>
						<button type="button" class="ghost" onclick={startFresh} disabled={freshBusy}>
							This is my only device
						</button>
					</div>
				{:else if step === "linking"}
					{#if deviceLink.phase === "connecting" || deviceLink.phase === "waiting-for-peer"}
						<h1>Waiting for your other device</h1>
						<p class="subtitle">
							On your other device, open Settings → Account and choose "Link a Device".
						</p>
						<button type="button" class="link" onclick={backToChoice}>Cancel</button>
					{:else if deviceLink.phase === "confirm"}
						<h1>Confirm this code matches</h1>
						<p class="subtitle">Check that this code is shown on both devices before continuing.</p>
						<p class="fingerprint">{deviceLink.fingerprint}</p>
						<p class="subtitle">Waiting for the other device to confirm and send your keys…</p>
						<button type="button" class="link" onclick={backToChoice}>Cancel</button>
					{:else if deviceLink.phase === "receiving"}
						<h1>Receiving your keys…</h1>
					{:else if deviceLink.phase === "done"}
						<h1>Linked</h1>
						<p class="subtitle">Your encryption keys are synced. Loading your conversations…</p>
					{:else if deviceLink.phase === "error"}
						<h1>Couldn't link this device</h1>
						<p class="subtitle error">{deviceLink.error}</p>
						<button type="button" class="link" onclick={backToChoice}>Try again</button>
					{/if}
				{:else if step === "fresh"}
					<h1>Setting up encryption…</h1>
				{/if}
			</div>
		{/key}
	</div>
</div>

<style>
	.screen {
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.card {
		width: 380px;
		background: var(--sidebar);
		border: 1px solid var(--hairline);
		border-radius: 12px;
		padding: 32px;
		overflow: hidden;
	}

	.pane {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-bottom: 24px;
	}

	.mark {
		width: 32px;
		height: 32px;
		border-radius: 10px;
		background: var(--accent-soft);
		object-fit: contain;
		padding: 5px;
	}

	.brand .name {
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 15px;
		letter-spacing: 0.01em;
	}

	h1 {
		margin: 0 0 4px;
		font-family: var(--font-display);
		font-weight: 700;
		font-size: 19px;
	}

	.subtitle {
		margin: 0 0 20px;
		color: var(--ink-dim);
		font-size: 13px;
		line-height: 1.5;
	}

	.subtitle.error {
		color: var(--danger);
	}

	.fingerprint {
		margin: 0 0 20px;
		font-family: var(--font-mono);
		font-size: 28px;
		font-weight: 700;
		letter-spacing: 0.1em;
		text-align: center;
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 14px;
	}

	.actions {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.primary {
		padding: 10px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-weight: 700;
		font-size: 14px;
	}

	.primary:hover:not(:disabled) {
		box-shadow: 0 0 0 3px var(--accent-soft);
	}

	.ghost {
		padding: 10px;
		border-radius: 6px;
		background: transparent;
		border: 1px solid var(--hairline);
		color: var(--ink-dim);
		font-weight: 600;
		font-size: 13px;
	}

	.ghost:hover:not(:disabled) {
		color: var(--ink);
		border-color: var(--ink-dim);
	}

	.link {
		color: var(--ink-dim);
		font-weight: 600;
		font-size: 13px;
		text-decoration: underline;
	}

	.link:hover {
		color: var(--ink);
	}
</style>
