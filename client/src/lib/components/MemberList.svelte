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
				<div class="ring" class:idle={member.status === "idle"}>
					<div class="avatar" style:background={member.color}>
						{member.name.slice(0, 2).toUpperCase()}
					</div>
				</div>
				<span class="name">{member.name}</span>
			</div>
		{/each}
	{/if}

	{#if offline.length > 0}
		<p class="label">Offline — {offline.length}</p>
		{#each offline as member (member.id)}
			<div class="member offline">
				<div class="ring offline">
					<div class="avatar" style:background={member.color}>
						{member.name.slice(0, 2).toUpperCase()}
					</div>
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
		background: var(--sidebar);
		padding: 16px 8px;
		overflow-y: auto;
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

	.member {
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

	.ring {
		flex-shrink: 0;
		width: 32px;
		height: 32px;
		border-radius: 50%;
		padding: 2px;
		background: conic-gradient(var(--online) 0deg, var(--online) 360deg);
		display: flex;
		align-items: center;
		justify-content: center;
		animation: ring-pulse 2.4s ease-in-out infinite;
	}

	.ring.idle {
		background: conic-gradient(var(--idle) 0deg, var(--idle) 360deg);
		animation: none;
	}

	.ring.offline {
		background: var(--ink-faint);
		animation: none;
	}

	@keyframes ring-pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.5;
		}
	}

	.avatar {
		width: 100%;
		height: 100%;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-mono);
		font-size: 10px;
		font-weight: 600;
		color: var(--void);
		border: 2px solid var(--sidebar);
	}

	.name {
		font-size: 13px;
		font-weight: 500;
		color: var(--ink-dim);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
