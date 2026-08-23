<script lang="ts">
	import type { Member } from "$lib/data/mock";

	let { members }: { members: Member[] } = $props();

	const online = $derived(members.filter((m) => m.status === "online" || m.status === "idle"));
	const offline = $derived(members.filter((m) => m.status === "offline"));
</script>

<aside class="members">
	{#if online.length > 0}
		<p class="label">Online — {online.length}</p>
		{#each online as member (member.id)}
			<div class="member">
				<div class="avatar" style:background={member.color}>
					{member.name.slice(0, 2).toUpperCase()}
					<span class="dot" class:idle={member.status === "idle"}></span>
				</div>
				<span class="name">{member.name}</span>
			</div>
		{/each}
	{/if}

	{#if offline.length > 0}
		<p class="label">Offline — {offline.length}</p>
		{#each offline as member (member.id)}
			<div class="member offline">
				<div class="avatar" style:background={member.color}>
					{member.name.slice(0, 2).toUpperCase()}
				</div>
				<span class="name">{member.name}</span>
			</div>
		{/each}
	{/if}
</aside>

<style>
	.members {
		width: 240px;
		flex-shrink: 0;
		background: var(--bg-sidebar);
		padding: 16px 8px;
		overflow-y: auto;
	}

	.label {
		margin: 8px 8px 4px;
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--text-faint);
	}

	.member {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 6px 8px;
		border-radius: 6px;
	}

	.member:hover {
		background: var(--bg-hover);
	}

	.member.offline {
		opacity: 0.5;
	}

	.avatar {
		position: relative;
		width: 28px;
		height: 28px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 10px;
		font-weight: 700;
		color: white;
		flex-shrink: 0;
	}

	.dot {
		position: absolute;
		bottom: -2px;
		right: -2px;
		width: 10px;
		height: 10px;
		border-radius: 50%;
		background: var(--online);
		border: 2px solid var(--bg-sidebar);
	}

	.dot.idle {
		background: var(--idle);
	}

	.name {
		font-size: 13px;
		font-weight: 500;
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
