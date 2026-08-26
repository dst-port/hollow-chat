<script lang="ts">
	import ProfilePopover from "$lib/components/ProfilePopover.svelte";
	import FullProfileModal from "$lib/components/FullProfileModal.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import * as api from "$lib/api/client";
	import type { Member } from "$lib/data/mock";

	let { members, serverName, onMessage }: {
		members: Member[];
		serverName: string;
		onMessage: (username: string) => void;
	} = $props();

	let openMember = $state<{ id: string; anchor: HTMLElement } | null>(null);
	let fullProfileUsername = $state<string | null>(null);

	$effect(() => {
		const token = session.token;
		if (!token) return;
		for (const member of members) {
			profileStore.load(token, member.name);
		}
	});

	function presenceOf(member: Member) {
		return profileStore.forUser(member.name)?.presence ?? "online";
	}

	function fallbackGroupFor(member: Member): string {
		return presenceOf(member) === "invisible" ? "Offline" : "Online";
	}

	const groups = $derived.by(() => {
		const map = new Map<string, Member[]>();
		for (const member of members) {
			const key = member.roles?.[0]?.label ?? fallbackGroupFor(member);
			if (!map.has(key)) map.set(key, []);
			map.get(key)!.push(member);
		}
		return Array.from(map.entries()).map(([name, list]) => ({ name, members: list }));
	});

	function toggle(id: string, event: MouseEvent) {
		const target = event.currentTarget as HTMLElement;
		openMember = openMember?.id === id ? null : { id, anchor: target };
	}
</script>

<aside class="members">
	{#each groups as group (group.name)}
		<p class="label">{group.name} — {group.members.length}</p>
		{#each group.members as member (member.id)}
			{@const profile = profileStore.forUser(member.name)}
			{@const presence = presenceOf(member)}
			<div class="anchor">
				<button class="member" class:offline={presence === "invisible"} onclick={(e) => toggle(member.id, e)}>
					<div class="status-avatar">
						<div
							class="avatar avatar-ring on-panel {presence}"
							style:background={profile?.avatar_url ? undefined : member.color}
							style:background-image={profile?.avatar_url ? `url(${api.resolveUrl(profile.avatar_url, session.token)})` : undefined}
						>
							{#if !profile?.avatar_url}{member.name.slice(0, 2).toUpperCase()}{/if}
						</div>
					</div>
					<div class="identity">
						<p
							class="name"
							style:color={presence !== "invisible" ? (profile?.accent_color || member.roles?.[0]?.color) : undefined}
						>
							{profile?.display_name || member.name}
						</p>
						{#if profile?.status_text}<p class="activity">{profile.status_text}</p>{:else if member.activity}<p class="activity">{member.activity}</p>{/if}
					</div>
				</button>
			</div>
		{/each}
	{/each}
</aside>

{#if openMember}
	{@const member = members.find((m) => m.id === openMember!.id)!}
	<ProfilePopover
		{member}
		{serverName}
		anchor={openMember.anchor}
		onClose={() => (openMember = null)}
		{onMessage}
		onViewFullProfile={(username) => (fullProfileUsername = username)}
	/>
{/if}

{#if fullProfileUsername}
	{@const fullMember = members.find((m) => m.name === fullProfileUsername) ?? null}
	<FullProfileModal
		username={fullProfileUsername}
		member={fullMember}
		{serverName}
		onClose={() => (fullProfileUsername = null)}
		{onMessage}
	/>
{/if}

<style>
	.members {
		width: 240px;
		flex-shrink: 0;
		background: var(--sidebar);
		padding: 16px 8px;
		overflow-y: auto;
	}

	.anchor {
		position: relative;
	}

	.label {
		margin: 8px 8px 4px;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--ink-faint);
	}

	.label:first-child {
		margin-top: 0;
	}

	.member {
		width: 100%;
		transition: background-color 0.15s ease;
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 6px 8px;
		border-radius: 6px;
	}

	.member:hover {
		background: var(--hover);
	}

	.member.offline {
		opacity: 0.5;
	}

	.avatar {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background-position: center;
		background-size: cover;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-mono);
		font-size: 10px;
		font-weight: 600;
		color: var(--void);
	}

	.identity {
		flex: 1;
		min-width: 0;
		text-align: left;
	}

	.name {
		margin: 0;
		font-size: 13px;
		font-weight: 500;
		color: var(--ink-dim);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.activity {
		margin: 1px 0 0;
		font-size: 11px;
		color: var(--ink-faint);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
