<script lang="ts">
	import { fly } from "svelte/transition";
	import UserPlus from "@lucide/svelte/icons/user-plus";
	import PlusCircle from "@lucide/svelte/icons/plus-circle";
	import FolderPlus from "@lucide/svelte/icons/folder-plus";
	import Settings from "@lucide/svelte/icons/settings";
	import Bell from "@lucide/svelte/icons/bell";
	import ShieldCheck from "@lucide/svelte/icons/shield-check";
	import BellOff from "@lucide/svelte/icons/bell-off";
	import IdCard from "@lucide/svelte/icons/id-card";
	import LogOut from "@lucide/svelte/icons/log-out";
	import { clickOutside } from "$lib/actions/clickOutside";
	import { toast } from "$lib/stores/toast.svelte";
	import { t } from "$lib/i18n/index.svelte";

	let { serverId, onClose, onInvite, onCreateChannel, onSettings, onLeave }: {
		serverId: string;
		onClose: () => void;
		onInvite: () => void;
		onCreateChannel: () => void;
		onSettings: () => void;
		onLeave: () => void;
	} = $props();

	let hideMuted = $state(false);

	function stub(label: string) {
		toast.push(t("serverMenu.notWired", { label }));
		onClose();
	}

	function copyServerId() {
		navigator.clipboard.writeText(serverId);
		toast.push(t("toast.serverIdCopied"));
		onClose();
	}
</script>

<div class="menu" use:clickOutside={onClose} transition:fly={{ y: -6, duration: 140 }}>
	<button class="item" onclick={onInvite}>
		{t("serverMenu.invitePeople")}
		<UserPlus size={15} strokeWidth={2} />
	</button>
	<button class="item" onclick={onSettings}>
		{t("serverMenu.serverSettings")}
		<Settings size={15} strokeWidth={2} />
	</button>
	<div class="divider"></div>
	<button class="item" onclick={onCreateChannel}>
		{t("serverMenu.createChannel")}
		<PlusCircle size={15} strokeWidth={2} />
	</button>
	<button class="item" onclick={() => stub(t("serverMenu.createCategory"))}>
		{t("serverMenu.createCategory")}
		<FolderPlus size={15} strokeWidth={2} />
	</button>
	<div class="divider"></div>
	<button class="item" onclick={() => stub(t("serverMenu.notificationSettings"))}>
		{t("serverMenu.notificationSettings")}
		<Bell size={15} strokeWidth={2} />
	</button>
	<button class="item" onclick={() => stub(t("serverMenu.privacySettings"))}>
		{t("serverMenu.privacySettings")}
		<ShieldCheck size={15} strokeWidth={2} />
	</button>
	<button class="item" onclick={() => (hideMuted = !hideMuted)}>
		{t("serverMenu.hideMutedChannels")}
		<span class="checkbox" class:checked={hideMuted}>
			{#if hideMuted}<BellOff size={11} strokeWidth={3} />{/if}
		</span>
	</button>
	<div class="divider"></div>
	<button class="item" onclick={copyServerId}>
		{t("serverMenu.copyServerId")}
		<IdCard size={15} strokeWidth={2} />
	</button>
	<div class="divider"></div>
	<button class="item danger" onclick={onLeave}>
		{t("serverMenu.leaveServer")}
		<LogOut size={15} strokeWidth={2} />
	</button>
</div>

<style>
	.menu {
		position: absolute;
		top: calc(100% + 4px);
		left: 8px;
		right: 8px;
		background: var(--panel);
		border-radius: 8px;
		padding: 6px;
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
		z-index: 50;
	}

	.item {
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 8px 10px;
		border-radius: 6px;
		font-size: 13px;
		font-weight: 500;
		color: var(--ink-dim);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.item:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.item.danger {
		color: var(--danger);
	}

	.item.danger:hover {
		background: rgba(216, 60, 62, 0.12);
		color: var(--danger);
	}

	.checkbox {
		width: 15px;
		height: 15px;
		flex-shrink: 0;
		border-radius: 4px;
		border: 1.5px solid var(--ink-faint);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--void);
		transition: background-color 0.15s ease, border-color 0.15s ease;
	}

	.checkbox.checked {
		background: var(--accent-fill);
		border-color: var(--accent-fill);
	}

	.divider {
		height: 1px;
		background: var(--hairline);
		margin: 6px 4px;
	}
</style>
