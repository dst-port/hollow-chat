<script lang="ts">
	import { fly } from "svelte/transition";
	import MessageSquare from "@lucide/svelte/icons/message-square";
	import { clickOutside } from "$lib/actions/clickOutside";
	import { toast } from "$lib/stores/toast.svelte";
	import type { Member } from "$lib/data/mock";

	let { member, anchor, onClose }: {
		member: Member;
		anchor: HTMLElement;
		onClose: () => void;
	} = $props();

	const POPOVER_WIDTH = 260;

	function computePosition() {
		const frame = document.querySelector(".window-frame");
		const frameRect = frame ? frame.getBoundingClientRect() : { top: 0, left: 0 };
		const anchorRect = anchor.getBoundingClientRect();

		return {
			top: anchorRect.top - frameRect.top,
			left: anchorRect.left - frameRect.left - POPOVER_WIDTH - 12
		};
	}

	const position = computePosition();

	function message() {
		toast.push("Direct messages aren't wired up yet");
		onClose();
	}
</script>

<div
	class="popover"
	use:clickOutside={onClose}
	style:top={`${position.top}px`}
	style:left={`${position.left}px`}
	style:width={`${POPOVER_WIDTH}px`}
	transition:fly={{ x: 6, duration: 140 }}
>
	<div class="banner" style:background={member.color}></div>
	<div class="ring">
		<div class="avatar" style:background={member.color}>
			{member.name.slice(0, 2).toUpperCase()}
		</div>
	</div>
	<div class="body">
		<p class="name">{member.name}</p>
		<p class="status">{member.status}</p>
		<button class="message-button" onclick={message}>
			<MessageSquare size={15} strokeWidth={2} />
			Message
		</button>
	</div>
</div>

<style>
	.popover {
		position: fixed;
		background: var(--panel);
		border-radius: 10px;
		overflow: hidden;
		box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
		z-index: 100;
	}

	.banner {
		height: 60px;
	}

	.ring {
		width: 68px;
		height: 68px;
		border-radius: 50%;
		padding: 3px;
		background: var(--panel);
		margin: -34px 0 0 16px;
	}

	.avatar {
		width: 100%;
		height: 100%;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-display);
		font-weight: 700;
		font-size: 18px;
		color: var(--void);
	}

	.body {
		padding: 12px 16px 16px;
	}

	.name {
		margin: 0;
		font-family: var(--font-mono);
		font-weight: 700;
		font-size: 15px;
		color: var(--ink);
	}

	.status {
		margin: 2px 0 12px;
		font-size: 12px;
		color: var(--ink-faint);
		text-transform: capitalize;
	}

	.message-button {
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 8px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-weight: 600;
		font-size: 13px;
	}
</style>
