<script lang="ts">
	import { fly } from "svelte/transition";
	import MessageSquare from "@lucide/svelte/icons/message-square";
	import MoreHorizontal from "@lucide/svelte/icons/more-horizontal";
	import CalendarDays from "@lucide/svelte/icons/calendar-days";
	import Users from "@lucide/svelte/icons/users";
	import SendHorizontal from "@lucide/svelte/icons/send-horizontal";
	import Badges from "$lib/components/Badges.svelte";
	import ActivityCard from "$lib/components/ActivityCard.svelte";
	import ProfileActionsMenu from "$lib/components/ProfileActionsMenu.svelte";
	import { clickOutside } from "$lib/actions/clickOutside";
	import { session } from "$lib/stores/session.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import { badgeStore } from "$lib/stores/badges.svelte";
	import * as api from "$lib/api/client";
	import type { Member } from "$lib/data/mock";

	let { member, serverName, anchor, onClose, onMessage, onViewFullProfile }: {
		member: Member;
		serverName: string;
		anchor: HTMLElement;
		onClose: () => void;
		onMessage: (username: string) => void;
		onViewFullProfile: (username: string) => void;
	} = $props();

	const POPOVER_WIDTH = 280;
	const NOTE_KEY = `hollowchat_note_${member.id}`;

	$effect(() => {
		const token = session.token;
		if (!token) return;
		badgeStore.loadForUser(token, member.name);
		profileStore.load(token, member.name);
	});

	const profile = $derived(profileStore.forUser(member.name));
	const presence = $derived(profile?.presence ?? "online");
	const accent = $derived(profile?.accent_color || member.roles?.[0]?.color || member.color);
	const displayName = $derived(profile?.display_name || member.name);
	const isSelf = $derived(member.name === session.username);

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
		onMessage(member.name);
		onClose();
	}

	let moreButtonEl: HTMLElement | undefined;
	let moreOpen = $state(false);
	let morePosition = $state({ top: 0, left: 0 });
	const MORE_MENU_HEIGHT = 260;

	function toggleMore() {
		if (!moreOpen && moreButtonEl) {
			const frame = document.querySelector(".window-frame");
			const frameRect = frame ? frame.getBoundingClientRect() : ({ top: 0, left: 0, right: window.innerWidth, bottom: window.innerHeight } as DOMRect);
			const rect = moreButtonEl.getBoundingClientRect();
			const maxTop = frameRect.bottom - frameRect.top - MORE_MENU_HEIGHT - 8;
			morePosition = {
				top: Math.max(8, Math.min(rect.bottom - frameRect.top + 4, maxTop)),
				left: Math.min(rect.right - frameRect.left - 210, frameRect.right - frameRect.left - 218)
			};
		}
		moreOpen = !moreOpen;
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
	<div class="banner" style:background={api.bannerBackground(profile, session.token)}></div>
	<div class="header-row">
		<div class="status-avatar">
			<div
				class="avatar avatar-ring on-panel {presence}"
				style:background={profile?.avatar_url ? undefined : member.color}
				style:background-image={profile?.avatar_url ? `url(${api.resolveUrl(profile.avatar_url, session.token)})` : undefined}
			>
				{#if !profile?.avatar_url}{member.name.slice(0, 2).toUpperCase()}{/if}
			</div>
		</div>
		<div class="header-actions">
			<div class="anchor">
				<button bind:this={moreButtonEl} class="icon-action" title="More" onclick={toggleMore}>
					<MoreHorizontal size={16} strokeWidth={2} />
				</button>
			</div>
			{#if !isSelf}
				<button class="icon-action primary" title="Message" onclick={message}>
					<MessageSquare size={16} strokeWidth={2} />
				</button>
			{/if}
		</div>
	</div>

	<div class="body">
		<p class="name-row">
			<span class="name" style:color={accent}>{displayName}</span>
		</p>
		<p class="handle">
			<span>{member.name}</span>
			{#if profile?.pronouns}<span>· {profile.pronouns}</span>{/if}
			<Badges badges={badgeStore.forUser(member.name)} />
		</p>
		{#if profile?.status_text}<p class="status">{profile.status_text}</p>{:else if member.activity}<p class="status">{member.activity}</p>{/if}

		{#if !isSelf && serverName}
			<p class="mutual">
				<Users size={12} strokeWidth={2} />
				1 Mutual Server — {serverName}
			</p>
		{/if}

		{#if profile?.bio || member.bio}
			<div class="section">
				<p class="section-label">About Me</p>
				<p class="bio">{profile?.bio || member.bio}</p>
			</div>
		{/if}

		{#if profile?.activity_application}
			<ActivityCard
				label="Playing"
				application={profile.activity_application}
				details={profile.activity_details}
				activityState={profile.activity_state}
				image={profile.activity_image}
				smallImage={profile.activity_small_image}
				smallText={profile.activity_small_text}
				startedAt={profile.activity_started_at}
				partySize={profile.activity_party_size}
				partyMax={profile.activity_party_max}
			/>
		{/if}
		{#if profile?.media_details}
			<p class="activity-line">
				{profile.media_details}
				{#if profile.media_application}<br /><strong>{profile.media_application}</strong>{/if}
				{#if profile.media_state}<br />{profile.media_state}{/if}
			</p>
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

	{#if moreOpen}
		<ProfileActionsMenu {member} {isSelf} position={morePosition} onClose={() => (moreOpen = false)} {onViewFullProfile} />
	{/if}
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
		height: 90px;
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
		border-radius: 50%;
		background-position: center;
		background-size: cover;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 16px;
		color: var(--void);
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

	.anchor {
		position: relative;
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

	.handle {
		margin: 1px 0 0;
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 5px;
		font-size: 11px;
		color: var(--ink-faint);
	}

	.status {
		margin: 2px 0 8px;
		font-size: 12px;
		color: var(--ink-faint);
	}

	.status:first-letter {
		text-transform: uppercase;
	}

	.activity-line {
		margin: 2px 0 8px;
		font-size: 12px;
		line-height: 1.4;
		color: var(--ink-dim);
	}

	.activity-line strong {
		color: var(--ink);
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
