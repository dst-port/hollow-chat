<script lang="ts">
	import ShieldCheck from "@lucide/svelte/icons/shield-check";
	import ShieldAlert from "@lucide/svelte/icons/shield-alert";
	import Modal from "$lib/components/Modal.svelte";
	import * as api from "$lib/api/client";
	import { session } from "$lib/stores/session.svelte";
	import { toast } from "$lib/stores/toast.svelte";
	import { t } from "$lib/i18n/index.svelte";
	import { getIdentityX25519 } from "$lib/crypto/identity";
	import { toBase64 } from "$lib/crypto/encoding";
	import { clearSession } from "$lib/crypto/session-store";
	import {
		clearPeerIdentity,
		loadPeerIdentity,
		safetyNumber,
		savePeerIdentity
	} from "$lib/crypto/peer-identity";

	let { peerUsername, onClose }: { peerUsername: string; onClose: () => void } = $props();

	type State =
		| { kind: "loading" }
		| { kind: "error" }
		// Nothing pinned yet: what the server hands us today, unverified.
		| { kind: "unpinned"; number: string }
		// Pinned and the server still agrees - the normal case.
		| { kind: "pinned"; number: string }
		// Pinned but the server now offers a different key. Either they
		// reinstalled, or someone is standing in the middle.
		| { kind: "changed"; pinnedNumber: string; offeredNumber: string; offeredKey: string };

	let view = $state<State>({ kind: "loading" });
	let accepting = $state(false);

	$effect(() => {
		void load(peerUsername);
	});

	async function load(peer: string) {
		const token = session.token;
		const me = session.username;
		if (!token || !me) return;
		view = { kind: "loading" };
		try {
			const mine = toBase64(getIdentityX25519(me).publicKey);
			const pinned = loadPeerIdentity(me, peer);
			const bundle = await api.fetchKeyBundle(token, peer);
			const offered = bundle.x25519_public;

			if (pinned === null) {
				view = { kind: "unpinned", number: safetyNumber(mine, offered) };
			} else if (pinned === offered) {
				view = { kind: "pinned", number: safetyNumber(mine, pinned) };
			} else {
				view = {
					kind: "changed",
					pinnedNumber: safetyNumber(mine, pinned),
					offeredNumber: safetyNumber(mine, offered),
					offeredKey: offered
				};
			}
		} catch {
			view = { kind: "error" };
		}
	}

	// Only ever reached by an explicit click, after the warning: accepting a new
	// key means abandoning the old conversation's ratchet, so the next message
	// starts a fresh session against the key being trusted now.
	function acceptNewKey() {
		const me = session.username;
		if (!me || view.kind !== "changed" || accepting) return;
		accepting = true;
		try {
			clearPeerIdentity(me, peerUsername);
			clearSession(me, peerUsername);
			savePeerIdentity(me, peerUsername, view.offeredKey);
			toast.push(t("safety.accepted"));
			onClose();
		} finally {
			accepting = false;
		}
	}
</script>

<Modal title={t("safety.title")} {onClose} width={480}>
	{#if view.kind === "loading"}
		<p class="body">{t("common.loading")}</p>
	{:else if view.kind === "error"}
		<p class="body">{t("safety.loadFailed")}</p>
	{:else if view.kind === "changed"}
		<div class="banner warn">
			<ShieldAlert size={16} strokeWidth={2} />
			<span>{t("safety.changedTitle", { name: peerUsername })}</span>
		</div>
		<p class="body">{t("safety.changedBody", { name: peerUsername })}</p>

		<p class="label">{t("safety.previouslyVerified")}</p>
		<p class="number stale">{view.pinnedNumber}</p>

		<p class="label">{t("safety.offeredNow")}</p>
		<p class="number">{view.offeredNumber}</p>

		<div class="actions">
			<button class="ghost" onclick={onClose}>{t("common.cancel")}</button>
			<button class="danger" onclick={acceptNewKey} disabled={accepting}>
				{t("safety.acceptNewKey")}
			</button>
		</div>
	{:else}
		<div class="banner" class:ok={view.kind === "pinned"}>
			<ShieldCheck size={16} strokeWidth={2} />
			<span>
				{view.kind === "pinned" ? t("safety.pinnedTitle", { name: peerUsername }) : t("safety.unpinnedTitle")}
			</span>
		</div>
		<p class="body">{t("safety.compareBody", { name: peerUsername })}</p>
		<p class="number">{view.number}</p>
		<div class="actions">
			<button class="ghost" onclick={onClose}>{t("common.close")}</button>
		</div>
	{/if}
</Modal>

<style>
	.body {
		margin: 0 0 14px;
		font-size: 13px;
		line-height: 1.5;
		color: var(--text-muted);
	}

	.banner {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 12px;
		padding: 10px 12px;
		border-radius: 8px;
		font-size: 13px;
		font-weight: 600;
		background: color-mix(in srgb, var(--accent) 12%, transparent);
		color: var(--text);
	}

	.banner.ok {
		background: color-mix(in srgb, #3ba55d 16%, transparent);
	}

	.banner.warn {
		background: color-mix(in srgb, #d83c3e 16%, transparent);
	}

	.label {
		margin: 0 0 4px;
		font-size: 11px;
		font-weight: 700;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		color: var(--text-muted);
	}

	.number {
		margin: 0 0 16px;
		padding: 12px;
		border: 1px solid var(--hairline);
		border-radius: 8px;
		font-family: var(--font-mono, ui-monospace, monospace);
		font-size: 16px;
		line-height: 1.7;
		letter-spacing: 0.08em;
		text-align: center;
		word-spacing: 0.5em;
		color: var(--text);
		user-select: text;
	}

	.number.stale {
		opacity: 0.6;
		text-decoration: line-through;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}

	button {
		padding: 8px 14px;
		border-radius: 6px;
		border: 1px solid transparent;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
	}

	.ghost {
		background: transparent;
		border-color: var(--hairline);
		color: var(--text);
	}

	.danger {
		background: #d83c3e;
		color: #fff;
	}

	.danger:disabled {
		opacity: 0.6;
		cursor: default;
	}
</style>
