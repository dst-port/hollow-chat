<script lang="ts">
	import { fly } from "svelte/transition";
	import UserPlus from "@lucide/svelte/icons/user-plus";
	import PlusCircle from "@lucide/svelte/icons/plus-circle";
	import Settings from "@lucide/svelte/icons/settings";
	import LogOut from "@lucide/svelte/icons/log-out";
	import { clickOutside } from "$lib/actions/clickOutside";

	let { onClose, onInvite, onCreateChannel, onSettings, onLeave }: {
		onClose: () => void;
		onInvite: () => void;
		onCreateChannel: () => void;
		onSettings: () => void;
		onLeave: () => void;
	} = $props();
</script>

<div class="menu" use:clickOutside={onClose} transition:fly={{ y: -6, duration: 140 }}>
	<button class="item" onclick={onInvite}>
		Invite People
		<UserPlus size={15} strokeWidth={2} />
	</button>
	<button class="item" onclick={onCreateChannel}>
		Create Channel
		<PlusCircle size={15} strokeWidth={2} />
	</button>
	<button class="item" onclick={onSettings}>
		Server Settings
		<Settings size={15} strokeWidth={2} />
	</button>
	<div class="divider"></div>
	<button class="item danger" onclick={onLeave}>
		Leave Server
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

	.divider {
		height: 1px;
		background: var(--hairline);
		margin: 6px 4px;
	}
</style>
