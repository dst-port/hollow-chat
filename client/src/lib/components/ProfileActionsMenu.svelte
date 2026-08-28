<script lang="ts">
	import { fly } from "svelte/transition";
	import Eye from "@lucide/svelte/icons/eye";
	import UserPlus from "@lucide/svelte/icons/user-plus";
	import EyeOff from "@lucide/svelte/icons/eye-off";
	import Ban from "@lucide/svelte/icons/ban";
	import Flag from "@lucide/svelte/icons/flag";
	import IdCard from "@lucide/svelte/icons/id-card";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import { clickOutside } from "$lib/actions/clickOutside";
	import { toast } from "$lib/stores/toast.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { packPayload } from "$lib/crypto/messagePayload";
	import { encryptForPeer } from "$lib/crypto/dm";
	import * as api from "$lib/api/client";

	const MENU_WIDTH = 210;
	const SUBMENU_WIDTH = 220;

	let { member, position, onClose, onViewFullProfile, isSelf = false }: {
		member: { id: string; name: string };
		position: { top: number; left: number };
		onClose: () => void;
		onViewFullProfile: (username: string) => void;
		isSelf?: boolean;
	} = $props();

	let inviteButtonEl: HTMLElement | undefined;
	let inviteOpen = $state(false);
	let invitePosition = $state({ top: 0, left: 0 });
	let inviteServers = $state<{ id: string; name: string }[] | null>(null);
	let inviteLoading = $state(false);
	let sendingInviteTo = $state<string | null>(null);

	function frameBounds() {
		const frame = document.querySelector(".window-frame");
		return frame ? frame.getBoundingClientRect() : ({ top: 0, left: 0, right: window.innerWidth, bottom: window.innerHeight } as DOMRect);
	}

	function copyUserId() {
		navigator.clipboard.writeText(member.id);
		toast.push("User ID copied");
		onClose();
	}

	function viewFullProfile() {
		onViewFullProfile(member.name);
		onClose();
	}

	function ignoreUser() {
		toast.push("Ignore isn't available yet");
		onClose();
	}

	function blockUser() {
		const token = session.token;
		if (!token) return;
		api
			.blockUser(token, member.id)
			.then(() => toast.push(`Blocked ${member.name}`))
			.catch(() => toast.push("Couldn't block user"));
		onClose();
	}

	function reportProfile() {
		toast.push("Profile reports aren't available yet");
		onClose();
	}

	async function loadInviteServers() {
		if (inviteServers !== null || inviteLoading) return;
		const token = session.token;
		if (!token) return;
		inviteLoading = true;
		try {
			const servers = await api.listServers(token);
			const eligible: { id: string; name: string }[] = [];
			for (const server of servers) {
				if (server.owner_id === session.userId) {
					eligible.push({ id: server.id, name: server.name });
					continue;
				}
				try {
					const members = await api.listMembers(token, server.id);
					const self = members.find((m) => m.id === session.userId);
					const canInvite = self?.roles.some((r) => (r.permissions & api.PERMISSIONS.CREATE_INVITE) !== 0);
					if (canInvite) eligible.push({ id: server.id, name: server.name });
				} catch {
					// skip servers we can't inspect
				}
			}
			inviteServers = eligible;
		} catch {
			inviteServers = [];
		} finally {
			inviteLoading = false;
		}
	}

	function toggleInvite() {
		if (inviteOpen) {
			inviteOpen = false;
			return;
		}
		if (inviteButtonEl) {
			const frame = frameBounds();
			const rect = inviteButtonEl.getBoundingClientRect();
			const maxTop = frame.bottom - frame.top - 260;
			invitePosition = {
				top: Math.max(8, Math.min(rect.top - frame.top, maxTop)),
				left: rect.right - frame.left + 4
			};
		}
		loadInviteServers();
		inviteOpen = true;
	}

	async function sendInvite(server: { id: string; name: string }) {
		const token = session.token;
		const myUsername = session.username;
		if (!token || !myUsername) return;
		sendingInviteTo = server.id;
		try {
			const { code } = await api.getServerInvite(token, server.id);
			const link = `hollowchat.org/invite/${code}`;
			const dm = await api.openDm(token, member.name);
			const packed = packPayload(`Join ${server.name}: ${link}`);
			const payload = await encryptForPeer(token, myUsername, member.name, packed);
			await api.sendDmMessage(token, dm.id, payload);
			toast.push(`Invite sent to ${member.name}`);
		} catch {
			toast.push("Couldn't send invite");
		} finally {
			sendingInviteTo = null;
			inviteOpen = false;
			onClose();
		}
	}
</script>

<div class="menu-root" use:clickOutside={onClose}>
<div
	class="menu"
	style:top={`${position.top}px`}
	style:left={`${position.left}px`}
	style:width={`${MENU_WIDTH}px`}
	transition:fly={{ y: -4, duration: 120 }}
>
	<button class="menu-item" onclick={viewFullProfile}>
		<Eye size={14} strokeWidth={2} />
		View Full Profile
	</button>
	{#if !isSelf}
		<button bind:this={inviteButtonEl} class="menu-item" class:active={inviteOpen} onclick={toggleInvite}>
			<UserPlus size={14} strokeWidth={2} />
			Invite to Server
			<ChevronRight size={13} strokeWidth={2} class="chevron" />
		</button>

		<div class="divider"></div>

		<button class="menu-item" onclick={ignoreUser}>
			<EyeOff size={14} strokeWidth={2} />
			Ignore
		</button>

		<div class="divider"></div>

		<button class="menu-item danger" onclick={blockUser}>
			<Ban size={14} strokeWidth={2} />
			Block
		</button>
		<button class="menu-item danger" onclick={reportProfile}>
			<Flag size={14} strokeWidth={2} />
			Report User Profile
		</button>
	{/if}

	<div class="divider"></div>

	<button class="menu-item" onclick={copyUserId}>
		<IdCard size={14} strokeWidth={2} />
		Copy User ID
	</button>
</div>

{#if !isSelf && inviteOpen}
	<div
		class="menu submenu"
		style:top={`${invitePosition.top}px`}
		style:left={`${invitePosition.left}px`}
		style:width={`${SUBMENU_WIDTH}px`}
		transition:fly={{ x: -4, duration: 120 }}
	>
		{#if inviteLoading}
			<p class="submenu-hint">Loading your servers…</p>
		{:else if !inviteServers || inviteServers.length === 0}
			<p class="submenu-hint">No servers you can invite from</p>
		{:else}
			{#each inviteServers as server (server.id)}
				<button class="menu-item" disabled={sendingInviteTo === server.id} onclick={() => sendInvite(server)}>
					{sendingInviteTo === server.id ? "Sending…" : server.name}
				</button>
			{/each}
		{/if}
	</div>
{/if}
</div>

<style>
	.menu-root {
		display: contents;
	}

	.menu {
		position: fixed;
		background: var(--panel);
		border-radius: 8px;
		padding: 6px;
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
		z-index: 130;
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.submenu {
		z-index: 131;
		max-height: 260px;
		overflow-y: auto;
	}

	.menu-item {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 10px;
		border-radius: 6px;
		font-size: 13px;
		font-weight: 500;
		color: var(--ink-dim);
		white-space: nowrap;
		text-align: left;
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.menu-item:hover:not(:disabled),
	.menu-item.active {
		background: var(--hover);
		color: var(--ink);
	}

	.menu-item:disabled {
		opacity: 0.6;
		cursor: default;
	}

	.menu-item :global(.chevron) {
		margin-left: auto;
		flex-shrink: 0;
		color: var(--ink-faint);
	}

	.menu-item.danger {
		color: var(--danger);
	}

	.menu-item.danger:hover {
		background: rgba(216, 60, 62, 0.12);
		color: var(--danger);
	}

	.divider {
		height: 1px;
		margin: 4px 2px;
		background: var(--hairline);
	}

	.submenu-hint {
		margin: 0;
		padding: 8px 10px;
		font-size: 12px;
		color: var(--ink-faint);
	}
</style>
