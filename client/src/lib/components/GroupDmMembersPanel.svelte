<script lang="ts">
	import UserPlus from "@lucide/svelte/icons/user-plus";
	import LogOut from "@lucide/svelte/icons/log-out";
	import { colorForName } from "$lib/utils/color";
	import { toast } from "$lib/stores/toast.svelte";
	import { session } from "$lib/stores/session.svelte";
	import * as api from "$lib/api/client";

	let { dm, onChanged, onLeft }: {
		dm: api.ApiDmChannel;
		onChanged: (dm: api.ApiDmChannel) => void;
		onLeft: () => void;
	} = $props();

	let addDraft = $state("");
	let busy = $state(false);

	async function addMember() {
		const token = session.token;
		const username = addDraft.trim();
		if (!token || !username || busy) return;
		busy = true;
		try {
			const updated = await api.addDmMember(token, dm.id, username);
			onChanged(updated);
			addDraft = "";
		} catch (err) {
			toast.push(err instanceof api.ApiError ? err.message : "Couldn't add member");
		} finally {
			busy = false;
		}
	}

	async function leave() {
		const token = session.token;
		if (!token || busy) return;
		busy = true;
		try {
			await api.leaveDm(token, dm.id);
			onLeft();
		} catch {
			toast.push("Couldn't leave group");
		} finally {
			busy = false;
		}
	}
</script>

<aside class="panel">
	<p class="title">{dm.name || "Group DM"}</p>
	<p class="count">{dm.members.length} members</p>

	<div class="members">
		{#each dm.members as member (member.id)}
			<div class="member-row">
				<div class="avatar" style:background={colorForName(member.username)}>
					{member.username.slice(0, 2).toUpperCase()}
				</div>
				<span class="name">{member.username}</span>
			</div>
		{/each}
	</div>

	<form class="add-form" onsubmit={(e) => (e.preventDefault(), addMember())}>
		<input type="text" placeholder="Add friend by username" bind:value={addDraft} maxlength="32" />
		<button type="submit" disabled={!addDraft.trim() || busy}>
			<UserPlus size={14} strokeWidth={2.25} />
		</button>
	</form>

	<button type="button" class="leave-btn" onclick={leave} disabled={busy}>
		<LogOut size={14} strokeWidth={2.25} />
		Leave Group
	</button>
</aside>

<style>
	.panel {
		width: 240px;
		flex-shrink: 0;
		background: var(--sidebar);
		padding: 16px;
		display: flex;
		flex-direction: column;
		min-height: 0;
	}

	.title {
		margin: 0;
		font-size: 15px;
		font-weight: 700;
		color: var(--ink);
	}

	.count {
		margin: 2px 0 12px;
		font-size: 12px;
		color: var(--ink-faint);
	}

	.members {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.member-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 5px 4px;
	}

	.avatar {
		flex-shrink: 0;
		width: 26px;
		height: 26px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-mono);
		font-size: 9px;
		font-weight: 600;
		color: var(--void);
	}

	.name {
		font-size: 13px;
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.add-form {
		display: flex;
		gap: 6px;
		margin-top: 10px;
	}

	.add-form input {
		flex: 1;
		min-width: 0;
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 7px 8px;
		font-size: 12px;
		color: var(--ink);
	}

	.add-form button {
		flex-shrink: 0;
		width: 30px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.add-form button:disabled {
		opacity: 0.5;
	}

	.leave-btn {
		margin-top: 10px;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		padding: 8px;
		border-radius: 6px;
		background: var(--panel);
		color: var(--danger);
		font-size: 12px;
		font-weight: 600;
	}

	.leave-btn:hover {
		background: var(--hover);
	}
</style>
