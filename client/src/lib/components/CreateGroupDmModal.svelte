<script lang="ts">
	import Check from "@lucide/svelte/icons/check";
	import Modal from "$lib/components/Modal.svelte";
	import { toast } from "$lib/stores/toast.svelte";
	import { colorForName } from "$lib/utils/color";
	import { t } from "$lib/i18n/index.svelte";
	import * as api from "$lib/api/client";

	let { token, friends, onClose, onCreated }: {
		token: string;
		friends: api.ApiFriend[];
		onClose: () => void;
		onCreated: (dm: api.ApiDmChannel) => void;
	} = $props();

	let selected = $state<Set<string>>(new Set());
	let nameDraft = $state("");
	let creating = $state(false);

	function toggle(username: string) {
		const next = new Set(selected);
		if (next.has(username)) next.delete(username);
		else next.add(username);
		selected = next;
	}

	async function create() {
		if (selected.size < 2 || creating) return;
		creating = true;
		try {
			const dm = await api.createGroupDm(token, [...selected], nameDraft.trim() || undefined);
			onCreated(dm);
			onClose();
		} catch (err) {
			toast.push(err instanceof api.ApiError ? err.message : t("toast.groupCreateFailed"));
		} finally {
			creating = false;
		}
	}
</script>

<Modal title={t("groupDm.create.title")} {onClose} width={420}>
	<p class="hint">{t("groupDm.create.body")}</p>

	<input type="text" class="name-input" placeholder={t("groupDm.create.namePlaceholder")} bind:value={nameDraft} maxlength="100" />

	{#if friends.length === 0}
		<p class="empty">{t("groupDm.create.noFriends")}</p>
	{:else}
		<div class="friend-list">
			{#each friends as friend (friend.id)}
				<button type="button" class="friend-row" onclick={() => toggle(friend.username)}>
					<div class="avatar" style:background={colorForName(friend.username)}>
						{friend.username.slice(0, 2).toUpperCase()}
					</div>
					<span class="name">{friend.display_name || friend.username}</span>
					<span class="check" class:active={selected.has(friend.username)}>
						{#if selected.has(friend.username)}<Check size={12} strokeWidth={3} />{/if}
					</span>
				</button>
			{/each}
		</div>
	{/if}

	<button type="button" class="create-btn" disabled={selected.size < 2 || creating} onclick={create}>
		{creating ? t("groupDm.create.submitting") : t("groupDm.create.submit", { count: selected.size })}
	</button>
</Modal>

<style>
	.hint {
		font-size: 12px;
		color: var(--ink-faint);
		margin: 0 0 12px;
		line-height: 1.4;
	}

	.name-input {
		width: 100%;
		background: var(--sidebar);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 9px 10px;
		font-size: 13px;
		color: var(--ink);
		margin-bottom: 12px;
	}

	.empty {
		font-size: 12px;
		color: var(--ink-faint);
	}

	.friend-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
		max-height: 280px;
		overflow-y: auto;
		margin-bottom: 14px;
	}

	.friend-row {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		padding: 7px 8px;
		border-radius: 6px;
	}

	.friend-row:hover {
		background: var(--hover);
	}

	.avatar {
		flex-shrink: 0;
		width: 28px;
		height: 28px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-mono);
		font-size: 10px;
		font-weight: 600;
		color: var(--void);
	}

	.name {
		flex: 1;
		text-align: left;
		font-size: 13px;
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.check {
		flex-shrink: 0;
		width: 18px;
		height: 18px;
		border-radius: 5px;
		border: 1px solid var(--hairline);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--accent-fill-ink);
	}

	.check.active {
		background: var(--accent-fill);
		border-color: var(--accent-fill);
	}

	.create-btn {
		width: 100%;
		padding: 10px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-weight: 600;
		font-size: 13px;
	}

	.create-btn:disabled {
		opacity: 0.5;
	}
</style>
