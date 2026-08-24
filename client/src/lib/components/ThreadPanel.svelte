<script lang="ts">
	import { fly } from "svelte/transition";
	import X from "@lucide/svelte/icons/x";
	import ChevronLeft from "@lucide/svelte/icons/chevron-left";
	import MessagesSquare from "@lucide/svelte/icons/messages-square";
	import SendHorizontal from "@lucide/svelte/icons/send-horizontal";
	import Archive from "@lucide/svelte/icons/archive";
	import ArchiveRestore from "@lucide/svelte/icons/archive-restore";
	import { emojify } from "$lib/actions/emojify";
	import { toast } from "$lib/stores/toast.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { colorForName } from "$lib/utils/color";
	import { renderMarkdown } from "$lib/utils/markdown";
	import {
		listThreads,
		listThreadMessages,
		sendThreadMessage,
		setThreadArchived,
		type ApiThread,
		type ApiMessage
	} from "$lib/api/client";
	import { encryptForChannel, decryptFromChannel, absorbSenderKeyFor } from "$lib/crypto/group";
	import { rememberSent, recallSent } from "$lib/crypto/sent-cache";

	let { channelId, initialThreadId, onClose }: {
		channelId: string;
		initialThreadId?: string;
		onClose: () => void;
	} = $props();

	let threads = $state<ApiThread[]>([]);
	let activeThread = $state<ApiThread | null>(null);
	let threadMessages = $state<{ id: string; author: string; content: string; timestamp: string }[]>([]);
	let draft = $state("");
	let loading = $state(false);

	async function decryptThreadMessage(msg: ApiMessage): Promise<string> {
		if (!msg.content) return "";
		const myUsername = session.username;
		if (!myUsername) return msg.content;
		if (msg.author === myUsername) {
			return recallSent(msg.id) ?? "[sent from another device]";
		}
		try {
			return await decryptFromChannel(myUsername, channelId, msg.author, msg.content);
		} catch {
			const token = session.token;
			if (token) {
				const absorbed = await absorbSenderKeyFor(token, myUsername, channelId, msg.author);
				if (absorbed) {
					try {
						return await decryptFromChannel(myUsername, channelId, msg.author, msg.content);
					} catch {
						return "[unable to decrypt message]";
					}
				}
			}
			return "[unable to decrypt message]";
		}
	}

	async function decryptThreadMessages(rows: ApiMessage[]) {
		const out = [];
		for (const row of rows) {
			out.push({
				id: row.id,
				author: row.author,
				content: await decryptThreadMessage(row),
				timestamp: row.timestamp
			});
		}
		return out;
	}

	function relativeTime(iso: string | null) {
		if (!iso) return "No replies yet";
		const diffMs = Date.now() - new Date(iso).getTime();
		const mins = Math.floor(diffMs / 60000);
		if (mins < 1) return "just now";
		if (mins < 60) return `${mins}m ago`;
		const hours = Math.floor(mins / 60);
		if (hours < 24) return `${hours}h ago`;
		return `${Math.floor(hours / 24)}d ago`;
	}

	async function refreshThreads() {
		const token = session.token;
		if (!token) return;
		try {
			threads = await listThreads(token, channelId);
		} catch {
			toast.push("Couldn't load threads");
		}
	}

	async function openThread(thread: ApiThread) {
		activeThread = thread;
		const token = session.token;
		if (!token) return;
		loading = true;
		try {
			const rows = await listThreadMessages(token, channelId, thread.id);
			threadMessages = await decryptThreadMessages(rows);
		} catch {
			toast.push("Couldn't load thread");
		} finally {
			loading = false;
		}
	}

	function backToList() {
		activeThread = null;
		threadMessages = [];
		refreshThreads();
	}

	async function send(event: SubmitEvent) {
		event.preventDefault();
		const content = draft.trim();
		const token = session.token;
		const myUsername = session.username;
		if (!content || !token || !myUsername || !activeThread) return;
		draft = "";
		try {
			const payload = await encryptForChannel(myUsername, channelId, content);
			const msg = await sendThreadMessage(token, channelId, activeThread.id, payload);
			rememberSent(msg.id, content);
			threadMessages.push({ id: msg.id, author: msg.author, content, timestamp: msg.timestamp });
		} catch {
			toast.push("Reply failed to send");
		}
	}

	async function toggleArchive() {
		const token = session.token;
		if (!token || !activeThread) return;
		try {
			activeThread = await setThreadArchived(token, channelId, activeThread.id, !activeThread.archived);
		} catch {
			toast.push("Couldn't update thread");
		}
	}

	$effect(() => {
		refreshThreads();
		if (initialThreadId) {
			const token = session.token;
			if (!token) return;
			listThreads(token, channelId).then((rows) => {
				const found = rows.find((t) => t.id === initialThreadId);
				if (found) openThread(found);
			});
		}
	});
</script>

<aside class="panel" transition:fly={{ x: 24, duration: 160 }}>
	<header class="header">
		{#if activeThread}
			<button class="icon-button" title="Back to threads" onclick={backToList}>
				<ChevronLeft size={17} strokeWidth={2} />
			</button>
			<span class="title">{activeThread.name}</span>
			<div class="spacer"></div>
			<button class="icon-button" title={activeThread.archived ? "Unarchive" : "Archive"} onclick={toggleArchive}>
				{#if activeThread.archived}
					<ArchiveRestore size={16} strokeWidth={2} />
				{:else}
					<Archive size={16} strokeWidth={2} />
				{/if}
			</button>
		{:else}
			<MessagesSquare size={17} strokeWidth={2} />
			<span class="title">Threads</span>
			<div class="spacer"></div>
		{/if}
		<button class="icon-button" title="Close" onclick={onClose}>
			<X size={17} strokeWidth={2} />
		</button>
	</header>

	{#if !activeThread}
		<div class="list">
			{#if threads.length === 0}
				<p class="empty">No threads yet. Start one from a message's menu.</p>
			{/if}
			{#each threads as thread (thread.id)}
				<button class="thread-row" onclick={() => openThread(thread)}>
					<div class="thread-icon" style:background={colorForName(thread.name)}>
						<MessagesSquare size={15} strokeWidth={2} />
					</div>
					<div class="thread-info">
						<span class="thread-name">
							{thread.name}
							{#if thread.archived}<span class="archived-badge">Archived</span>{/if}
						</span>
						<span class="thread-meta">
							{thread.message_count} {thread.message_count === 1 ? "reply" : "replies"} · {relativeTime(thread.last_message_at)}
						</span>
					</div>
				</button>
			{/each}
		</div>
	{:else}
		<div class="messages">
			{#if loading}
				<p class="empty">Loading…</p>
			{:else if threadMessages.length === 0}
				<p class="empty">No replies yet. Be the first to reply.</p>
			{/if}
			{#each threadMessages as message (message.id)}
				<div class="message">
					<div class="avatar" style:background={colorForName(message.author)}>
						{message.author.slice(0, 2).toUpperCase()}
					</div>
					<div class="body">
						<p class="meta">
							<span class="author" style:color={colorForName(message.author)}>{message.author}</span>
							<span class="time">{new Date(message.timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</span>
						</p>
						{#if message.content}
							<p class="content" use:emojify>{@html renderMarkdown(message.content)}</p>
						{/if}
					</div>
				</div>
			{/each}
		</div>

		<form class="composer" onsubmit={send}>
			<input type="text" placeholder="Reply in thread" bind:value={draft} />
			<button type="submit" disabled={!draft.trim()}>
				<SendHorizontal size={15} strokeWidth={2.25} />
			</button>
		</form>
	{/if}
</aside>

<style>
	.panel {
		width: 320px;
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--panel);
		border-left: 1px solid var(--hairline);
	}

	.header {
		height: 48px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 12px;
		border-bottom: 1px solid var(--hairline);
	}

	.title {
		font-family: var(--font-mono);
		font-weight: 600;
		font-size: 14px;
	}

	.spacer {
		flex: 1;
	}

	.icon-button {
		display: flex;
		color: var(--ink-dim);
		padding: 6px;
		border-radius: 6px;
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.icon-button:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.list {
		flex: 1;
		overflow-y: auto;
		padding: 8px;
	}

	.empty {
		padding: 20px 8px;
		font-size: 13px;
		color: var(--ink-faint);
		text-align: center;
	}

	.thread-row {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px;
		border-radius: 8px;
		text-align: left;
		transition: background-color 0.15s ease;
	}

	.thread-row:hover {
		background: var(--hover);
	}

	.thread-icon {
		width: 32px;
		height: 32px;
		border-radius: 8px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--void);
	}

	.thread-info {
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.thread-name {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 13px;
		font-weight: 600;
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.archived-badge {
		flex-shrink: 0;
		padding: 1px 6px;
		border-radius: 999px;
		background: var(--active);
		color: var(--ink-faint);
		font-size: 10px;
		font-weight: 600;
		text-transform: uppercase;
	}

	.thread-meta {
		font-size: 11px;
		color: var(--ink-faint);
	}

	.messages {
		flex: 1;
		overflow-y: auto;
		padding: 12px;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.message {
		display: flex;
		gap: 10px;
	}

	.avatar {
		width: 28px;
		height: 28px;
		border-radius: 50%;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-mono);
		font-size: 10px;
		font-weight: 600;
		color: var(--void);
	}

	.body {
		min-width: 0;
		flex: 1;
	}

	.meta {
		margin: 0 0 2px;
		display: flex;
		align-items: baseline;
		gap: 6px;
	}

	.author {
		font-family: var(--font-mono);
		font-weight: 600;
		font-size: 12px;
	}

	.time {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--ink-faint);
	}

	.content {
		margin: 0;
		font-size: 13px;
		line-height: 1.4;
		color: var(--ink);
		word-break: break-word;
	}

	.content :global(strong) {
		font-weight: 700;
	}

	.content :global(em) {
		font-style: italic;
	}

	.content :global(u) {
		text-decoration: underline;
	}

	.content :global(del) {
		text-decoration: line-through;
		opacity: 0.7;
	}

	.content :global(code.md-inline) {
		background: var(--sidebar);
		border-radius: 4px;
		padding: 1px 5px;
		font-family: var(--font-mono);
		font-size: 0.9em;
	}

	.content :global(pre.md-block) {
		background: var(--sidebar);
		border-radius: 6px;
		padding: 8px 10px;
		margin: 4px 0;
		overflow-x: auto;
	}

	.content :global(.md-spoiler) {
		background: var(--ink-faint);
		color: transparent;
		border-radius: 3px;
		cursor: pointer;
	}

	.content :global(.md-spoiler.revealed) {
		background: var(--active);
		color: var(--ink);
	}

	.composer {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 12px;
		border-top: 1px solid var(--hairline);
	}

	.composer input {
		flex: 1;
		background: var(--active);
		border-radius: 8px;
		padding: 9px 12px;
		color: var(--ink);
		border: none;
		font-family: var(--font-body);
		font-size: 13px;
	}

	.composer input::placeholder {
		color: var(--ink-faint);
	}

	.composer button[type="submit"] {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 34px;
		height: 34px;
		border-radius: 8px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.composer button[type="submit"]:disabled {
		background: var(--active);
		color: var(--ink-faint);
		cursor: default;
	}
</style>
