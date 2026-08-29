<script lang="ts">
	import Modal from "$lib/components/Modal.svelte";
	import Paperclip from "@lucide/svelte/icons/paperclip";
	import { toast } from "$lib/stores/toast.svelte";
	import { sealReport, type ReportedMessage } from "$lib/crypto/moderation";
	import { submitReport } from "$lib/api/client";
	import { t } from "$lib/i18n/index.svelte";
	import type { Message } from "$lib/data/mock";

	const MAX_SCREENSHOT_BYTES = 5 * 1024 * 1024;
	const CATEGORIES = ["Spam", "Harassment", "NSFW", "Scam / Fraud", "Other"];
	const CATEGORY_LABEL_KEYS: Record<string, string> = {
		Spam: "report.catSpam",
		Harassment: "report.catHarassment",
		NSFW: "report.catNsfw",
		"Scam / Fraud": "report.catScam",
		Other: "report.catOther"
	};
	const categoryLabel = (value: string) => t(CATEGORY_LABEL_KEYS[value] ?? value);

	let {
		token,
		reportedUsername,
		reportedUserId,
		contextKind,
		contextId,
		serverId,
		candidates,
		initialMessageId,
		onClose
	}: {
		token: string;
		reportedUsername: string;
		reportedUserId: string;
		contextKind: "dm" | "channel";
		contextId: string;
		serverId?: string;
		candidates: Message[];
		initialMessageId: string;
		onClose: () => void;
	} = $props();

	let selected = $state(new Set<string>([initialMessageId]));
	let category = $state(CATEGORIES[0]);
	let reason = $state("");
	let screenshot = $state<{ mimeType: string; dataBase64: string; name: string } | null>(null);
	let submitting = $state(false);
	let fileInput: HTMLInputElement | undefined;

	function toggle(id: string) {
		const next = new Set(selected);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		selected = next;
	}

	function onPickScreenshot(event: Event) {
		const file = (event.target as HTMLInputElement).files?.[0];
		if (!file) return;
		if (file.size > MAX_SCREENSHOT_BYTES) {
			toast.push(t("report.screenshotTooLarge"));
			return;
		}
		const reader = new FileReader();
		reader.onload = () => {
			const dataUrl = reader.result as string;
			const base64 = dataUrl.slice(dataUrl.indexOf(",") + 1);
			screenshot = { mimeType: file.type || "image/png", dataBase64: base64, name: file.name };
		};
		reader.readAsDataURL(file);
	}

	async function submit() {
		if (selected.size === 0 && !screenshot) {
			toast.push(t("report.selectSomething"));
			return;
		}
		if (!reason.trim()) {
			toast.push(t("report.describePrompt"));
			return;
		}

		submitting = true;
		try {
			const messages: ReportedMessage[] = candidates
				.filter((m) => selected.has(m.id))
				.map((m) => ({
					id: m.id,
					senderUsername: m.author,
					timestamp: new Date(m.timestampMs).toISOString(),
					text: m.content,
					attachmentFilename: m.attachment?.filename
				}));

			const sealed = await sealReport({
				version: 1,
				category,
				reason: reason.trim(),
				reportedUsername,
				messages,
				screenshot: screenshot ? { mimeType: screenshot.mimeType, dataBase64: screenshot.dataBase64 } : undefined
			});

			await submitReport(token, {
				reported_user_id: reportedUserId,
				context_kind: contextKind,
				context_id: contextId,
				server_id: serverId,
				sealed_key: {
					ephemeral_public: sealed.sealedKey.ephemeralPublicKey,
					nonce: sealed.sealedKey.nonce,
					ciphertext: sealed.sealedKey.ciphertext
				},
				payload_nonce: sealed.payloadNonce,
				payload_ciphertext: sealed.payloadCiphertext
			});

			toast.push(t("report.sent"));
			onClose();
		} catch {
			toast.push(t("report.failed"));
		} finally {
			submitting = false;
		}
	}
</script>

<Modal title={t("report.title", { name: reportedUsername })} {onClose} width={420}>
	<div class="form">
		<label class="field">
			<span>{t("report.category")}</span>
			<select bind:value={category}>
				{#each CATEGORIES as c (c)}
					<option value={c}>{categoryLabel(c)}</option>
				{/each}
			</select>
		</label>

		<label class="field">
			<span>{t("report.whatHappened")}</span>
			<textarea bind:value={reason} rows="3" placeholder={t("report.reasonPlaceholder")}></textarea>
		</label>

		<div class="field">
			<span>{t("report.messagesToInclude")}</span>
			<div class="messages">
				{#each candidates as message (message.id)}
					<label class="message-row">
						<input type="checkbox" checked={selected.has(message.id)} onchange={() => toggle(message.id)} />
						<span class="message-check"></span>
						<span class="message-text">{message.content || t("report.attachment")}</span>
					</label>
				{/each}
			</div>
		</div>

		<div class="field">
			<span>{t("report.screenshotOptional")}</span>
			{#if screenshot}
				<div class="screenshot-preview">
					<span>{screenshot.name}</span>
					<button type="button" onclick={() => (screenshot = null)}>{t("common.remove")}</button>
				</div>
			{:else}
				<button type="button" class="attach-btn" onclick={() => fileInput?.click()}>
					<Paperclip size={14} strokeWidth={2} />
					{t("report.attachScreenshot")}
				</button>
				<input bind:this={fileInput} type="file" accept="image/*" class="hidden-input" onchange={onPickScreenshot} />
			{/if}
		</div>

		<p class="notice">
			{t("report.notice")}
		</p>

		<button class="submit-btn" disabled={submitting} onclick={submit}>
			{submitting ? t("report.submitting") : t("report.submit")}
		</button>
	</div>
</Modal>

<style>
	.form {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
		font-size: 13px;
		font-weight: 600;
		color: var(--ink-dim);
	}

	select,
	textarea {
		background: var(--sidebar);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 8px 10px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 13px;
		resize: vertical;
	}

	select:focus,
	textarea:focus {
		outline: none;
		border-color: var(--ink-dim);
	}

	.messages {
		display: flex;
		flex-direction: column;
		gap: 2px;
		max-height: 160px;
		overflow-y: auto;
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 6px;
	}

	.message-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px;
		border-radius: 4px;
		cursor: pointer;
		font-weight: 400;
	}

	.message-row:hover {
		background: var(--hover);
	}

	.message-row input {
		position: absolute;
		width: 1px;
		height: 1px;
		opacity: 0;
	}

	.message-check {
		width: 16px;
		height: 16px;
		flex-shrink: 0;
		border-radius: 4px;
		border: 1px solid var(--hairline);
		background: var(--sidebar);
		transition: background-color 0.15s ease, border-color 0.15s ease;
	}

	.message-row input:checked + .message-check {
		background: var(--accent, #5865f2);
		border-color: var(--accent, #5865f2);
	}

	.message-row input:focus-visible + .message-check {
		outline: 2px solid var(--ink-dim);
		outline-offset: 1px;
	}

	.message-text {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--ink);
		font-size: 13px;
	}

	.attach-btn {
		display: flex;
		align-items: center;
		gap: 6px;
		align-self: flex-start;
		padding: 7px 12px;
		border-radius: 6px;
		border: 1px solid var(--hairline);
		font-size: 12px;
		font-weight: 600;
		color: var(--ink-dim);
	}

	.attach-btn:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.hidden-input {
		display: none;
	}

	.screenshot-preview {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 7px 10px;
		border-radius: 6px;
		border: 1px solid var(--hairline);
		font-size: 12px;
		color: var(--ink);
	}

	.screenshot-preview button {
		color: var(--danger);
		font-weight: 600;
	}

	.notice {
		margin: 0;
		font-size: 11px;
		font-weight: 400;
		line-height: 1.4;
		color: var(--ink-dim);
	}

	.submit-btn {
		padding: 10px;
		border-radius: 6px;
		background: var(--danger);
		color: white;
		font-weight: 700;
		font-size: 13px;
	}

	.submit-btn:disabled {
		opacity: 0.6;
	}

	.submit-btn:not(:disabled):hover {
		filter: brightness(1.08);
	}
</style>
