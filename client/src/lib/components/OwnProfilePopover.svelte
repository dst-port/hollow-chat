<script lang="ts">
	import { fly } from "svelte/transition";
	import Pencil from "@lucide/svelte/icons/pencil";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import IdCard from "@lucide/svelte/icons/id-card";
	import SmilePlus from "@lucide/svelte/icons/smile-plus";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import { clickOutside } from "$lib/actions/clickOutside";
	import { toast } from "$lib/stores/toast.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { badgeStore } from "$lib/stores/badges.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import Badges from "$lib/components/Badges.svelte";
	import ActivityCard from "$lib/components/ActivityCard.svelte";
	import StatusModal from "$lib/components/StatusModal.svelte";
	import { t } from "$lib/i18n/index.svelte";
	import * as api from "$lib/api/client";

	let { username, anchor, onEditProfile, onClose }: {
		username: string;
		anchor: HTMLElement;
		onEditProfile: () => void;
		onClose: () => void;
	} = $props();

	$effect(() => {
		const token = session.token;
		if (!token) return;
		badgeStore.loadForUser(token, username);
		profileStore.load(token, username);
	});

	const profile = $derived(profileStore.forUser(username));

	let statusModalOpen = $state(false);
	let presenceMenuOpen = $state(false);
	let presenceButtonEl = $state<HTMLElement | undefined>();
	let presencePosition = $state({ top: 0, left: 0 });

	const PRESENCE_FLYOUT_HEIGHT = 250;

	function togglePresenceMenu() {
		if (!presenceMenuOpen && presenceButtonEl) {
			const rect = presenceButtonEl.getBoundingClientRect();
			const frame = document.querySelector(".window-frame");
			const frameRect = frame
				? frame.getBoundingClientRect()
				: { top: 0, bottom: window.innerHeight };
			const maxTop = frameRect.bottom - PRESENCE_FLYOUT_HEIGHT - 8;
			const minTop = frameRect.top + 8;
			presencePosition = {
				top: Math.max(minTop, Math.min(rect.top, maxTop)),
				left: rect.right + 8
			};
		}
		presenceMenuOpen = !presenceMenuOpen;
	}

	const PRESENCE_OPTIONS = $derived<{ value: api.PresenceState; label: string; description?: string }[]>([
		{ value: "online", label: t("presence.online") },
		{ value: "idle", label: t("profile.own.idle") },
		{ value: "dnd", label: t("presence.dnd"), description: t("presence.dndHint") },
		{ value: "invisible", label: t("presence.invisible"), description: t("presence.invisibleHint") }
	]);

	const PRESENCE_LABELS = $derived<Record<api.PresenceState, string>>({
		online: t("presence.online"),
		idle: t("profile.own.idle"),
		dnd: t("presence.dnd"),
		invisible: t("presence.invisible")
	});

	async function pickPresence(value: api.PresenceState) {
		const token = session.token;
		presenceMenuOpen = false;
		if (!token) return;
		try {
			profileStore.set(await api.setPresence(token, value));
		} catch {
			toast.push(t("profile.own.toast.presenceFailed"));
		}
	}

	async function quickClearStatus() {
		const token = session.token;
		if (!token) return;
		try {
			const updated = await api.updateProfile(token, { status_text: "", status_clear_minutes: 0 });
			profileStore.set(updated);
		} catch {
			toast.push(t("status.toast.clearFailed"));
		}
	}

	const POPOVER_WIDTH = 280;

	function computePosition() {
		const frame = document.querySelector(".window-frame");
		const frameRect = frame ? frame.getBoundingClientRect() : { top: 0, left: 0, bottom: window.innerHeight };
		const anchorRect = anchor.getBoundingClientRect();

		return {
			bottom: frameRect.bottom - anchorRect.top + 8,
			left: anchorRect.left - frameRect.left
		};
	}

	const position = computePosition();

	function copyId() {
		navigator.clipboard.writeText(username);
		toast.push(t("profile.actions.toast.userIdCopied"));
	}

	function editProfile() {
		onEditProfile();
		onClose();
	}
</script>

<div
	class="popover"
	use:clickOutside={onClose}
	style:bottom={`${position.bottom}px`}
	style:left={`${position.left}px`}
	style:width={`${POPOVER_WIDTH}px`}
	transition:fly={{ y: 6, duration: 140 }}
>
	<div
		class="banner"
		style:background={api.bannerBackground(profile, session.token)}
	></div>
	<div class="avatar-row status-avatar">
		<div
			class="avatar {profile?.presence ?? 'online'}"
			style:background-image={profile?.avatar_url ? `url(${api.resolveUrl(profile.avatar_url, session.token)})` : undefined}
		>
			{#if !profile?.avatar_url}{username.slice(0, 2).toUpperCase()}{/if}
		</div>
		{#if profile?.status_text}
			<div class="status-bubble" transition:fly={{ y: 4, duration: 140 }}>
				{profile.status_text}
				<div class="status-bubble-actions">
					<button class="bubble-action" title={t("profile.own.editStatus")} onclick={() => (statusModalOpen = true)}>
						<Pencil size={11} strokeWidth={2.5} />
					</button>
					<button class="bubble-action" title={t("profile.own.clearStatus")} onclick={quickClearStatus}>
						<Trash2 size={11} strokeWidth={2.5} />
					</button>
				</div>
			</div>
		{/if}
	</div>

	<div class="body">
		<p class="name" style:color={profile?.accent_color || "var(--default-accent)"}>{profile?.display_name || username}</p>
		<p class="role-line">
			<span>{username}</span>
			{#if profile?.pronouns}<span class="role-dot">•</span><span>{profile.pronouns}</span>{/if}
			<Badges badges={badgeStore.forUser(username)} />
		</p>

		<div class="card">
			<p class="card-label">{t("profile.own.aboutMe")}</p>
			<p class="card-text">{profile?.bio || t("profile.own.noBio")}</p>
		</div>

		{#if profile?.activity_application}
			<ActivityCard
				label={t("profile.full.playing")}
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

		<button class="cta" onclick={editProfile}>
			<Pencil size={16} strokeWidth={2.25} />
			{t("settings.editProfile")}
		</button>

		<div class="list">
			<button class="list-row" onclick={() => (statusModalOpen = true)}>
				<SmilePlus size={17} strokeWidth={2} class="list-row-icon" />
				<span class="list-row-label">{profile?.status_text ? t("profile.own.editStatus") : t("profile.own.setStatus")}</span>
			</button>

			<div class="list-row-wrapper">
				<button class="list-row" bind:this={presenceButtonEl} onclick={togglePresenceMenu}>
					<span class="status-dot static list-row-icon {profile?.presence ?? 'online'}"></span>
					<span class="list-row-label">{PRESENCE_LABELS[profile?.presence ?? "online"]}</span>
					<ChevronRight size={14} strokeWidth={2} class="list-row-chevron" />
				</button>
				{#if presenceMenuOpen}
					<div
						class="presence-flyout"
						role="menu"
						tabindex="-1"
						use:clickOutside={() => (presenceMenuOpen = false)}
						style:top={`${presencePosition.top}px`}
						style:left={`${presencePosition.left}px`}
						transition:fly={{ x: -4, duration: 120 }}
					>
						{#each PRESENCE_OPTIONS as option (option.value)}
							<button class="presence-option {option.value}" onclick={() => pickPresence(option.value)}>
								<span class="status-dot static presence-option-dot {option.value}"></span>
								<span class="presence-option-text">
									<span class="presence-option-label">{option.label}</span>
									{#if option.description}
										<span class="presence-option-desc">{option.description}</span>
									{/if}
								</span>
							</button>
						{/each}
					</div>
				{/if}
			</div>

			<button class="list-row last" onclick={copyId}>
				<IdCard size={17} strokeWidth={2} class="list-row-icon" />
				<span class="list-row-label">{t("profile.actions.copyUserId")}</span>
			</button>
		</div>
	</div>

	{#if statusModalOpen}
		<StatusModal {username} onClose={() => (statusModalOpen = false)} />
	{/if}
</div>

<style>
	.popover {
		--radius-sm: 8px;
		--radius-lg: 16px;
		--gap-xs: 4px;
		--gap-sm: 8px;
		--gap-md: 16px;
		--gap-lg: 24px;
		--container-padding: 16px;
		--default-accent: #8ea1ff;

		position: fixed;
		background: var(--panel);
		border-radius: var(--radius-lg);
		overflow: hidden;
		box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5), 0 2px 6px rgba(0, 0, 0, 0.3);
		z-index: 100;
	}

	@media (max-width: 640px) {
		.popover {
			left: 12px !important;
			right: 12px;
			bottom: 12px !important;
			width: auto !important;
			max-height: calc(100% - 24px);
			overflow-y: auto;
		}
	}

	.banner {
		height: 100px;
		background: var(--active);
	}

	.avatar-row {
		position: relative;
		margin: -38px 0 0 var(--container-padding);
		width: fit-content;
	}

	.status-bubble {
		position: absolute;
		left: 64px;
		bottom: 28px;
		max-width: 170px;
		background: var(--active);
		color: var(--ink);
		border-radius: var(--radius-sm);
		border-bottom-left-radius: var(--gap-xs);
		padding: var(--gap-sm) 26px var(--gap-sm) var(--gap-sm);
		font-size: 12px;
		line-height: 1.35;
		box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35);
	}

	.status-bubble-actions {
		position: absolute;
		top: var(--gap-xs);
		right: var(--gap-xs);
		display: flex;
		gap: 2px;
		opacity: 0;
		transition: opacity 0.12s ease;
	}

	.status-bubble:hover .status-bubble-actions {
		opacity: 1;
	}

	.bubble-action {
		width: 18px;
		height: 18px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--sidebar);
		color: var(--ink-dim);
	}

	.bubble-action:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.avatar {
		width: 80px;
		height: 80px;
		border: 3px solid var(--ink-faint);
		border-radius: 50%;
		background: var(--accent-fill) center/cover;
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 24px;
		box-shadow: 0 4px 10px rgba(0, 0, 0, 0.4);
	}

	.avatar.online {
		border-color: var(--online);
	}

	.avatar.idle {
		border-color: var(--idle);
	}

	.avatar.dnd {
		border-color: var(--danger);
	}

	.avatar.invisible {
		border-color: var(--ink-faint);
	}

	.status-dot {
		width: 13px;
		height: 13px;
	}

	.status-dot.static {
		position: static;
		width: 9px;
		height: 9px;
		border: none;
		flex-shrink: 0;
	}

	.body {
		padding: var(--gap-md) var(--container-padding) var(--container-padding);
		display: flex;
		flex-direction: column;
	}

	.name {
		margin: var(--gap-sm) 0 0;
		font-family: var(--font-mono);
		font-weight: 700;
		font-size: 17px;
		line-height: 1.2;
	}

	.role-line {
		margin: var(--gap-xs) 0 0;
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 5px;
		font-size: 12px;
		font-weight: 500;
		color: var(--ink-faint);
	}

	.role-dot {
		font-size: 10px;
	}

	.card {
		margin-top: var(--gap-md);
		background: rgba(255, 255, 255, 0.03);
		border-radius: var(--radius-sm);
		padding: var(--gap-sm) var(--gap-sm);
	}

	.card-label {
		margin: 0 0 var(--gap-xs);
		font-family: var(--font-mono);
		font-size: 10px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--ink-faint);
	}

	.card-text {
		margin: 0;
		font-size: 12px;
		line-height: 1.4;
		color: var(--ink-dim);
	}

	.cta {
		margin-top: var(--gap-md);
		width: 100%;
		height: 36px;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--gap-sm);
		border-radius: var(--radius-sm);
		background: var(--active);
		color: var(--ink);
		font-weight: 700;
		font-size: 13px;
	}

	.cta:hover {
		filter: brightness(1.1);
	}

	.list {
		margin-top: var(--gap-md);
		background: rgba(255, 255, 255, 0.03);
		border-radius: var(--radius-sm);
		overflow: hidden;
	}

	.list-row-wrapper {
		position: relative;
	}

	.list-row {
		width: 100%;
		height: 40px;
		display: flex;
		align-items: center;
		gap: var(--gap-sm);
		padding: 0 var(--gap-sm);
		font-size: 13px;
		font-weight: 500;
		color: var(--ink-dim);
		text-align: left;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		transition: background-color 0.12s ease, color 0.12s ease;
	}

	.list-row.last {
		border-bottom: none;
	}

	.list-row:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.list-row :global(svg.list-row-icon) {
		flex-shrink: 0;
		width: 17px;
	}

	.list-row .status-dot.list-row-icon {
		flex-shrink: 0;
	}

	.list-row-label {
		flex: 1;
	}

	.list-row :global(.list-row-chevron) {
		flex-shrink: 0;
		color: var(--ink-faint);
	}

	.presence-flyout {
		position: fixed;
		width: 220px;
		display: flex;
		flex-direction: column;
		gap: 2px;
		background: var(--panel);
		border-radius: var(--radius-sm);
		padding: var(--gap-xs);
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
		z-index: 120;
	}

	.presence-option {
		width: 100%;
		display: flex;
		align-items: flex-start;
		gap: var(--gap-sm);
		padding: 10px var(--gap-sm);
		border-radius: var(--gap-xs);
		text-align: left;
	}

	.presence-option:first-child {
		margin-bottom: 4px;
		padding-bottom: 12px;
		border-bottom: 1px solid var(--hairline);
		border-radius: var(--gap-xs) var(--gap-xs) 0 0;
	}

	.presence-option:hover {
		background: var(--hover);
	}

	.presence-option.online:hover {
		background: var(--online);
	}

	.presence-option.idle:hover {
		background: var(--idle);
	}

	.presence-option.dnd:hover {
		background: var(--danger);
	}

	.presence-option.invisible:hover {
		background: var(--ink-faint);
	}

	.presence-option.online:hover .presence-option-label,
	.presence-option.idle:hover .presence-option-label,
	.presence-option.dnd:hover .presence-option-label,
	.presence-option.invisible:hover .presence-option-label {
		color: #fff;
	}

	.presence-option.online:hover .presence-option-desc,
	.presence-option.idle:hover .presence-option-desc,
	.presence-option.dnd:hover .presence-option-desc,
	.presence-option.invisible:hover .presence-option-desc {
		color: rgba(255, 255, 255, 0.85);
	}

	.presence-option-dot {
		margin-top: 4px;
	}

	.presence-option-text {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.presence-option-label {
		font-size: 13px;
		font-weight: 500;
		color: var(--ink);
	}

	.presence-option-desc {
		font-size: 11px;
		color: var(--ink-faint);
		line-height: 1.3;
	}
</style>
