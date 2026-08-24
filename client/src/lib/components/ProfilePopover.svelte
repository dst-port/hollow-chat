<script lang="ts">
	import { fly } from "svelte/transition";
	import MessageSquare from "@lucide/svelte/icons/message-square";
	import MoreHorizontal from "@lucide/svelte/icons/more-horizontal";
	import CalendarDays from "@lucide/svelte/icons/calendar-days";
	import Users from "@lucide/svelte/icons/users";
	import SendHorizontal from "@lucide/svelte/icons/send-horizontal";
	import Badges from "$lib/components/Badges.svelte";
	import { clickOutside } from "$lib/actions/clickOutside";
	import { toast } from "$lib/stores/toast.svelte";
	import type { Member } from "$lib/data/mock";

	let { member, serverName, anchor, onClose }: {
		member: Member;
		serverName: string;
		anchor: HTMLElement;
		onClose: () => void;
	} = $props();

	const POPOVER_WIDTH = 280;
	const NOTE_KEY = `hollowchat_note_${member.id}`;
	const accent = member.roles?.[0]?.color ?? member.color;

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
	let note = $state(localStorage.getItem(NOTE_KEY) ?? "");
	let draft = $state("");

	function saveNote() {
		if (note.trim()) {
			localStorage.setItem(NOTE_KEY, note);
		} else {
			localStorage.removeItem(NOTE_KEY);
		}
	}

	function message() {
		toast.push("Direct messages aren't wired up yet");
		onClose();
	}

	function moreOptions() {
		toast.push("More options aren't wired up yet");
	}

	function sendDraft(event: SubmitEvent) {
		event.preventDefault();
		if (!draft.trim()) return;
		message();
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
	<div class="banner" style:background={`linear-gradient(135deg, ${accent}, color-mix(in srgb, ${accent} 40%, black))`}></div>
	<div class="header-row">
		<div class="status-avatar">
			<div class="avatar" style:background={member.color} style="box-shadow: 0 0 0 2px {accent};">
				{member.name.slice(0, 2).toUpperCase()}
			</div>
			{#if member.status}<span class="status-dot on-panel {member.status}"></span>{/if}
		</div>
		<div class="header-actions">
			<button class="icon-action" title="More" onclick={moreOptions}>
				<MoreHorizontal size={16} strokeWidth={2} />
			</button>
			<button class="icon-action primary" title="Message" onclick={message}>
				<MessageSquare size={16} strokeWidth={2} />
			</button>
		</div>
	</div>

	<div class="body">
		<p class="name-row">
			<span class="name" style:color={accent}>{member.name}</span>
			{#if member.badges}<Badges badges={member.badges} />{/if}
		</p>
		{#if member.activity ?? member.status}<p class="status">{member.activity ?? member.status}</p>{/if}

		<p class="mutual">
			<Users size={12} strokeWidth={2} />
			1 Mutual Server — {serverName}
		</p>

		{#if member.bio}
			<div class="section">
				<p class="section-label">About Me</p>
				<p class="bio">{member.bio}</p>
			</div>
		{/if}

		{#if member.memberSince}
			<div class="section">
				<p class="section-label">Member Since</p>
				<p class="member-since">
					<CalendarDays size={13} strokeWidth={2} />
					{member.memberSince}
				</p>
			</div>
		{/if}

		{#if member.roles && member.roles.length > 0}
			<div class="section">
				<p class="section-label">Roles</p>
				<div class="roles">
					{#each member.roles as role (role.label)}
						<span class="role-chip">
							<span class="role-dot" style:background={role.color}></span>
							{role.label}
						</span>
					{/each}
				</div>
			</div>
		{/if}

		<div class="section">
			<p class="section-label">Note</p>
			<textarea
				class="note"
				placeholder="Click to add a note"
				bind:value={note}
				onblur={saveNote}
			></textarea>
		</div>
	</div>

	<form class="composer" onsubmit={sendDraft}>
		<input type="text" placeholder={`Message @${member.name}`} bind:value={draft} />
		<button type="submit" class="send" disabled={!draft.trim()} title="Send">
			<SendHorizontal size={15} strokeWidth={2} />
		</button>
	</form>
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
		height: 52px;
	}

	.header-row {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		padding: 0 16px;
		margin-top: -26px;
	}

	.avatar {
		width: 56px;
		height: 56px;
		border: 3px solid var(--panel);
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 16px;
		color: var(--void);
	}

	.status-dot {
		width: 13px;
		height: 13px;
		right: 1px;
		bottom: 1px;
	}

	.header-actions {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-bottom: 8px;
	}

	.icon-action {
		flex-shrink: 0;
		padding: 8px;
		border-radius: 50%;
		background: var(--sidebar);
		color: var(--ink-dim);
		display: flex;
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.icon-action:hover {
		background: var(--active);
		color: var(--ink);
	}

	.icon-action.primary {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.body {
		padding: 8px 16px 16px;
	}

	.name-row {
		margin: 0;
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 6px;
	}

	.name {
		font-family: var(--font-mono);
		font-weight: 700;
		font-size: 16px;
		color: var(--ink);
	}

	.status {
		margin: 2px 0 8px;
		font-size: 12px;
		color: var(--ink-faint);
	}

	.status:first-letter {
		text-transform: uppercase;
	}

	.mutual {
		margin: 0 0 10px;
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: 11px;
		color: var(--ink-faint);
	}

	.section {
		padding: 10px 0;
		border-top: 1px solid var(--hairline);
	}

	.section:first-of-type {
		border-top: none;
		padding-top: 0;
	}

	.section-label {
		margin: 0 0 4px;
		font-family: var(--font-mono);
		font-size: 10px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--ink-faint);
	}

	.bio {
		margin: 0;
		font-size: 12px;
		line-height: 1.5;
		color: var(--ink-dim);
	}

	.member-since {
		margin: 0;
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 12px;
		color: var(--ink-dim);
	}

	.roles {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.role-chip {
		display: flex;
		align-items: center;
		gap: 5px;
		background: var(--sidebar);
		border-radius: 5px;
		padding: 3px 8px 3px 6px;
		font-size: 11px;
		font-weight: 500;
		color: var(--ink-dim);
	}

	.role-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.note {
		width: 100%;
		min-height: 44px;
		resize: none;
		background: var(--sidebar);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 8px 10px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 12px;
		line-height: 1.4;
	}

	.note::placeholder {
		color: var(--ink-faint);
	}

	.note:focus {
		outline: none;
		border-color: var(--ink-dim);
	}

	.composer {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 10px 12px;
		border-top: 1px solid var(--hairline);
		background: var(--sidebar);
	}

	.composer input {
		flex: 1;
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 999px;
		padding: 8px 12px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 12px;
	}

	.composer input::placeholder {
		color: var(--ink-faint);
	}

	.composer input:focus {
		outline: none;
		border-color: var(--ink-dim);
	}

	.send {
		flex-shrink: 0;
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.send:disabled {
		background: var(--active);
		color: var(--ink-faint);
	}
</style>
