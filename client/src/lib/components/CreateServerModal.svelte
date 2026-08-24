<script lang="ts">
	import ChevronLeft from "@lucide/svelte/icons/chevron-left";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import Compass from "@lucide/svelte/icons/compass";
	import Sparkles from "@lucide/svelte/icons/sparkles";
	import DoorOpen from "@lucide/svelte/icons/door-open";
	import Modal from "$lib/components/Modal.svelte";
	import { toast } from "$lib/stores/toast.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { joinServer, ApiError, type ApiServer } from "$lib/api/client";

	let { onClose, onCreate, onJoin }: {
		onClose: () => void;
		onCreate: (name: string) => void;
		onJoin: (server: ApiServer) => void;
	} = $props();

	type Step = "choice" | "create" | "join";
	let step = $state<Step>("choice");
	let name = $state("");
	let inviteLink = $state("");
	let joining = $state(false);

	function submitCreate(event: SubmitEvent) {
		event.preventDefault();
		const trimmed = name.trim();
		if (!trimmed) return;
		onCreate(trimmed);
	}

	function extractCode(input: string): string {
		const trimmed = input.trim();
		const parts = trimmed.split("/").filter(Boolean);
		return parts[parts.length - 1] ?? trimmed;
	}

	async function submitJoin(event: SubmitEvent) {
		event.preventDefault();
		const code = extractCode(inviteLink);
		const token = session.token;
		if (!code || !token) return;
		joining = true;
		try {
			const server = await joinServer(token, code);
			onJoin(server);
		} catch (err) {
			if (err instanceof ApiError && err.status === 404) {
				toast.push("That invite doesn't exist or has expired");
			} else {
				toast.push("Couldn't join server");
			}
		} finally {
			joining = false;
		}
	}

	const title = $derived(
		step === "choice" ? "Add a Server" : step === "create" ? "Create Your Server" : "Join a Server"
	);
</script>

<Modal {title} {onClose}>
	{#if step === "choice"}
		<p class="hint">Your server is where you and your friends hang out. Make yours and start talking.</p>
		<div class="choices">
			<button class="choice" onclick={() => (step = "create")}>
				<span class="choice-icon create"><Sparkles size={16} strokeWidth={2} /></span>
				<span class="choice-label">Create My Own</span>
				<ChevronRight size={16} strokeWidth={2} class="choice-chevron" />
			</button>
			<button class="choice" onclick={() => (step = "join")}>
				<span class="choice-icon join"><DoorOpen size={16} strokeWidth={2} /></span>
				<span class="choice-label">Join a Server</span>
				<ChevronRight size={16} strokeWidth={2} class="choice-chevron" />
			</button>
		</div>
	{:else if step === "create"}
		<button class="back" onclick={() => (step = "choice")}>
			<ChevronLeft size={14} strokeWidth={2.5} />
			Back
		</button>
		<form onsubmit={submitCreate}>
			<p class="hint">Give your server a name. You can invite people once it exists.</p>
			<label>
				Server name
				<input type="text" bind:value={name} required maxlength="48" placeholder="Void Raiders" />
			</label>
			<button type="submit" disabled={!name.trim()}>Create</button>
		</form>
	{:else}
		<button class="back" onclick={() => (step = "choice")}>
			<ChevronLeft size={14} strokeWidth={2.5} />
			Back
		</button>
		<form onsubmit={submitJoin}>
			<p class="hint">Enter an invite below to join an existing server.</p>
			<label>
				Invite link
				<input type="text" bind:value={inviteLink} required placeholder="hollowchat.app/invite/a1b2c3d4" />
			</label>
			<button type="submit" disabled={!inviteLink.trim() || joining}>
				{joining ? "Joining…" : "Join Server"}
			</button>
		</form>
		<div class="discover">
			<Compass size={18} strokeWidth={2} />
			<div>
				<p class="discover-title">Don't have an invite?</p>
				<p class="discover-hint">Ask whoever runs the server you want to join for a link.</p>
			</div>
		</div>
	{/if}
</Modal>

<style>
	form {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.hint {
		margin: 0;
		font-size: 13px;
		color: var(--ink-dim);
		line-height: 1.5;
	}

	.choices {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin-top: 16px;
	}

	.choice {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px 14px;
		border-radius: 8px;
		background: var(--sidebar);
		border: 1px solid transparent;
		color: var(--ink);
		font-weight: 600;
		font-size: 14px;
		transition: background-color 0.15s ease, border-color 0.15s ease, transform 0.15s ease;
	}

	.choice-icon {
		flex-shrink: 0;
		width: 32px;
		height: 32px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.choice-icon.create {
		background: color-mix(in srgb, var(--accent-fill) 22%, transparent);
		color: var(--accent-fill);
	}

	.choice-icon.join {
		background: color-mix(in srgb, var(--online) 20%, transparent);
		color: var(--online);
	}

	.choice-label {
		flex: 1;
	}

	.choice :global(.choice-chevron) {
		flex-shrink: 0;
		color: var(--ink-faint);
		transition: transform 0.15s ease;
	}

	.choice:hover {
		background: var(--active);
		border-color: var(--hairline);
	}

	.choice:hover :global(.choice-chevron) {
		transform: translateX(2px);
		color: var(--ink-dim);
	}

	.choice:active {
		transform: scale(0.985);
	}

	.back {
		display: flex;
		align-items: center;
		gap: 4px;
		margin-bottom: 12px;
		font-size: 12px;
		font-weight: 600;
		color: var(--ink-faint);
	}

	.back:hover {
		color: var(--ink-dim);
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 6px;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--ink-dim);
	}

	input {
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 10px 12px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 14px;
	}

	input:focus {
		outline: none;
		border-color: var(--ink-dim);
	}

	button[type="submit"] {
		padding: 10px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-weight: 700;
		font-size: 14px;
	}

	button[type="submit"]:disabled {
		background: var(--active);
		color: var(--ink-faint);
	}

	.discover {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-top: 16px;
		padding: 14px;
		border-radius: 8px;
		background: var(--sidebar);
		color: var(--ink-faint);
	}

	.discover-title {
		margin: 0;
		font-size: 13px;
		font-weight: 600;
		color: var(--ink);
	}

	.discover-hint {
		margin: 2px 0 0;
		font-size: 12px;
		color: var(--ink-faint);
	}
</style>
