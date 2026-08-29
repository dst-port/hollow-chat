<script lang="ts">
	import { fade, scale } from "svelte/transition";
	import { cubicOut } from "svelte/easing";
	import X from "@lucide/svelte/icons/x";
	import LayoutGrid from "@lucide/svelte/icons/layout-grid";
	import UserPlus from "@lucide/svelte/icons/user-plus";
	import ShieldAlert from "@lucide/svelte/icons/shield-alert";
	import Hash from "@lucide/svelte/icons/hash";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import ShieldCheck from "@lucide/svelte/icons/shield-check";
	import Users from "@lucide/svelte/icons/users";
	import UserX from "@lucide/svelte/icons/user-x";
	import Plus from "@lucide/svelte/icons/plus";
	import Check from "@lucide/svelte/icons/check";
	import Smile from "@lucide/svelte/icons/smile";
	import Sparkles from "@lucide/svelte/icons/sparkles";
	import InviteModal from "$lib/components/InviteModal.svelte";
	import ColorPicker from "$lib/components/ColorPicker.svelte";
	import { toast } from "$lib/stores/toast.svelte";
	import { t, tp } from "$lib/i18n/index.svelte";
	import { session } from "$lib/stores/session.svelte";
	import Dropdown from "$lib/components/Dropdown.svelte";
	import {
		renameServer,
		setServerIcon,
		clearServerIcon,
		uploadFile,
		resolveUrl,
		setSlowmode,
		listRoles,
		createRole,
		updateRole,
		deleteRole,
		assignRole,
		unassignRole,
		listMembers,
		kickMember,
		listBans,
		banMember,
		unbanMember,
		getServerInvite,
		setVanityInvite,
		billingStatus,
		getBoosts,
		listCustomEmoji,
		addCustomEmoji,
		removeCustomEmoji,
		ApiError,
		PERMISSION_LABELS,
		PERMISSIONS,
		type ApiRole,
		type ApiMember,
		type ApiBan,
		type BoostStatus,
		type CustomEmoji
	} from "$lib/api/client";
	import type { ServerEntry } from "$lib/data/mock";

	let { server, onClose, onLeave }: {
		server: ServerEntry;
		onClose: () => void;
		onLeave: () => void;
	} = $props();

	type Section = "overview" | "channels" | "invites" | "emoji" | "roles" | "members" | "bans" | "moderation";
	const initialName = server.name;
	const isOwner = server.ownerId === session.userId;

	let roles = $state<ApiRole[]>([]);
	let members = $state<ApiMember[]>([]);
	let bans = $state<ApiBan[]>([]);
	let newRoleName = $state("");

	let inviteCode = $state<string | null>(null);
	let vanityCode = $state("");
	let vanitySaving = $state(false);
	let isPremiumOwner = $state(false);

	let boostStatus = $state<BoostStatus | null>(null);
	let customEmoji = $state<CustomEmoji[]>([]);
	let newEmojiName = $state("");
	let emojiUploading = $state(false);
	let emojiInput: HTMLInputElement | undefined;

	async function loadInvite() {
		const token = session.token;
		if (!token) return;
		try {
			const invite = await getServerInvite(token, server.id);
			inviteCode = invite.code;
			vanityCode = invite.code;
		} catch {
			toast.push(t("serverSettings.invites.loadFailed"));
		}
		if (isOwner) {
			try {
				const status = await billingStatus(token);
				isPremiumOwner = status.tier === "premium";
			} catch {
				isPremiumOwner = false;
			}
		}
	}

	async function saveVanityCode() {
		const token = session.token;
		const code = vanityCode.trim();
		if (!token || !code || code === inviteCode) return;
		vanitySaving = true;
		try {
			const invite = await setVanityInvite(token, server.id, code);
			inviteCode = invite.code;
			vanityCode = invite.code;
			toast.push(t("serverSettings.invites.updated"));
		} catch (err) {
			if (err instanceof ApiError && err.status === 403) {
				toast.push(t("serverSettings.vanity.premiumOnly"));
			} else if (err instanceof ApiError && err.status === 400) {
				toast.push(t("serverSettings.vanity.rules"));
			} else {
				toast.push(t("serverSettings.vanity.taken"));
			}
		} finally {
			vanitySaving = false;
		}
	}

	async function loadEmoji() {
		const token = session.token;
		if (!token) return;
		try {
			const [status, emoji] = await Promise.all([
				getBoosts(token, server.id),
				listCustomEmoji(token, server.id)
			]);
			boostStatus = status;
			customEmoji = emoji;
		} catch {
			toast.push(t("serverSettings.emoji.loadFailed"));
		}
	}

	async function onEmojiFileChosen(event: Event) {
		const token = session.token;
		const file = (event.target as HTMLInputElement).files?.[0];
		const name = newEmojiName.trim();
		if (!token || !file || !name) return;
		emojiUploading = true;
		try {
			const attachment = await uploadFile(token, file);
			const emoji = await addCustomEmoji(token, server.id, name, attachment.id);
			customEmoji = [...customEmoji, emoji];
			newEmojiName = "";
		} catch (err) {
			if (err instanceof ApiError && err.status === 409) {
				if (err.message.includes("slot")) toast.push(t("serverSettings.emoji.noSlots"));
				else toast.push(t("serverSettings.emoji.nameUsed"));
			} else {
				toast.push(t("serverSettings.emoji.addFailed"));
			}
		} finally {
			emojiUploading = false;
			if (emojiInput) emojiInput.value = "";
		}
	}

	async function removeEmoji(emoji: CustomEmoji) {
		const token = session.token;
		if (!token) return;
		try {
			await removeCustomEmoji(token, server.id, emoji.id);
			customEmoji = customEmoji.filter((e) => e.id !== emoji.id);
		} catch {
			toast.push(t("serverSettings.emoji.removeFailed"));
		}
	}

	let iconInput: HTMLInputElement | undefined;
	let iconUploading = $state(false);

	async function onIconChosen(event: Event) {
		const token = session.token;
		const file = (event.target as HTMLInputElement).files?.[0];
		if (!token || !file) return;
		iconUploading = true;
		try {
			const attachment = await uploadFile(token, file);
			const updated = await setServerIcon(token, server.id, attachment.id);
			server.iconUrl = updated.icon_url;
		} catch {
			toast.push(t("serverSettings.overview.iconUpdateFailed"));
		} finally {
			iconUploading = false;
			if (iconInput) iconInput.value = "";
		}
	}

	async function removeIcon() {
		const token = session.token;
		if (!token) return;
		try {
			const updated = await clearServerIcon(token, server.id);
			server.iconUrl = updated.icon_url;
		} catch {
			toast.push(t("serverSettings.overview.iconRemoveFailed"));
		}
	}

	async function loadRoles() {
		const token = session.token;
		if (!token) return;
		try {
			roles = await listRoles(token, server.id);
		} catch {
			toast.push(t("serverSettings.roles.loadFailed"));
		}
	}

	async function loadMembers() {
		const token = session.token;
		if (!token) return;
		try {
			members = await listMembers(token, server.id);
		} catch {
			toast.push(t("serverSettings.members.loadFailed"));
		}
	}

	async function loadBans() {
		const token = session.token;
		if (!token) return;
		try {
			bans = await listBans(token, server.id);
		} catch {
			toast.push(t("serverSettings.bans.loadFailed"));
		}
	}

	$effect(() => {
		if (section === "roles") loadRoles();
		if (section === "members") {
			loadRoles();
			loadMembers();
		}
		if (section === "bans") loadBans();
		if (section === "invites") loadInvite();
		if (section === "emoji") loadEmoji();
	});

	async function addRole() {
		const token = session.token;
		const name = newRoleName.trim();
		if (!token || !name) return;
		try {
			const role = await createRole(token, server.id, name, "#8a8f98", 0);
			roles.push(role);
			newRoleName = "";
		} catch {
			toast.push(t("serverSettings.roles.createFailed"));
		}
	}

	async function togglePermission(role: ApiRole, bit: number) {
		const token = session.token;
		if (!token) return;
		const next = role.permissions ^ bit;
		role.permissions = next;
		try {
			await updateRole(token, server.id, role.id, { permissions: next });
		} catch {
			role.permissions = next ^ bit;
			toast.push(t("serverSettings.roles.updateFailed"));
		}
	}

	async function renameRole(role: ApiRole, name: string) {
		const token = session.token;
		const trimmed = name.trim();
		if (!token || !trimmed || trimmed === role.name) return;
		try {
			await updateRole(token, server.id, role.id, { name: trimmed });
			role.name = trimmed;
		} catch {
			toast.push(t("serverSettings.roles.renameFailed"));
		}
	}

	async function recolorRole(role: ApiRole, color: string) {
		const token = session.token;
		if (!token) return;
		role.color = color;
		try {
			await updateRole(token, server.id, role.id, { color });
		} catch {
			toast.push(t("serverSettings.roles.recolorFailed"));
		}
	}

	async function removeRole(role: ApiRole) {
		const token = session.token;
		if (!token) return;
		try {
			await deleteRole(token, server.id, role.id);
			roles = roles.filter((r) => r.id !== role.id);
			for (const member of members) member.roles = member.roles.filter((r) => r.id !== role.id);
		} catch {
			toast.push(t("serverSettings.roles.deleteFailed"));
		}
	}

	async function toggleMemberRole(member: ApiMember, role: ApiRole) {
		const token = session.token;
		if (!token) return;
		const has = member.roles.some((r) => r.id === role.id);
		try {
			if (has) {
				await unassignRole(token, server.id, member.id, role.id);
				member.roles = member.roles.filter((r) => r.id !== role.id);
			} else {
				await assignRole(token, server.id, member.id, role.id);
				member.roles = [...member.roles, role];
			}
		} catch {
			toast.push(t("serverSettings.members.rolesUpdateFailed"));
		}
	}

	async function kick(member: ApiMember) {
		const token = session.token;
		if (!token) return;
		try {
			await kickMember(token, server.id, member.id);
			members = members.filter((m) => m.id !== member.id);
			toast.push(t("serverSettings.members.kickedToast", { name: member.username }));
		} catch {
			toast.push(t("serverSettings.members.kickFailed"));
		}
	}

	async function ban(member: ApiMember) {
		const token = session.token;
		if (!token) return;
		try {
			await banMember(token, server.id, member.id);
			members = members.filter((m) => m.id !== member.id);
			toast.push(t("serverSettings.members.bannedToast", { name: member.username }));
		} catch {
			toast.push(t("serverSettings.members.banFailed"));
		}
	}

	async function unban(ban: ApiBan) {
		const token = session.token;
		if (!token) return;
		try {
			await unbanMember(token, server.id, ban.user_id);
			bans = bans.filter((b) => b.user_id !== ban.user_id);
		} catch {
			toast.push(t("serverSettings.bans.unbanFailed"));
		}
	}

	const SLOWMODE_OPTIONS = [
		{ label: t("serverSettings.channels.slowmodeOff"), seconds: 0 },
		{ label: "5s", seconds: 5 },
		{ label: "10s", seconds: 10 },
		{ label: "30s", seconds: 30 },
		{ label: "1m", seconds: 60 },
		{ label: "5m", seconds: 300 },
		{ label: "15m", seconds: 900 },
		{ label: "1h", seconds: 3600 }
	];

	let section = $state<Section>("overview");
	let name = $state(initialName);
	let inviteOpen = $state(false);
	let confirmDelete = $state(false);

	async function changeSlowmode(channelId: string, seconds: number) {
		const token = session.token;
		if (!token) return;
		try {
			await setSlowmode(token, server.id, channelId, seconds);
			const channel = server.channels.find((c) => c.id === channelId);
			if (channel) channel.slowmodeSeconds = seconds;
		} catch {
			toast.push(t("serverSettings.channels.slowmodeFailed"));
		}
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") onClose();
	}

	async function saveName() {
		const token = session.token;
		const trimmed = name.trim();
		if (!token || !trimmed) return;
		try {
			await renameServer(token, server.id, trimmed);
			server.name = trimmed;
			server.initials = trimmed.slice(0, 2).toUpperCase();
			toast.push(t("serverSettings.overview.updatedToast"));
		} catch {
			toast.push(t("serverSettings.overview.renameFailed"));
		}
	}

	function deleteServer() {
		onClose();
		onLeave();
		toast.push(t("serverSettings.deletedToast", { name: server.name }));
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="overlay" role="presentation" onclick={onClose} transition:fade={{ duration: 150 }}>
	<div
		class="modal"
		role="dialog"
		aria-modal="true"
		aria-label={t("serverSettings.title")}
		tabindex="-1"
		onclick={(e) => e.stopPropagation()}
		transition:scale={{ duration: 180, start: 0.97, easing: cubicOut }}
	>
		<nav class="nav">
			<p class="nav-label">{server.name}</p>
			<button class="nav-item" class:active={section === "overview"} onclick={() => (section = "overview")}>
				<LayoutGrid size={16} strokeWidth={2} />
				{t("serverSettings.nav.overview")}
			</button>
			<button class="nav-item" class:active={section === "channels"} onclick={() => (section = "channels")}>
				<Hash size={16} strokeWidth={2} />
				{t("serverSettings.nav.channels")}
			</button>
			<button class="nav-item" class:active={section === "invites"} onclick={() => (section = "invites")}>
				<UserPlus size={16} strokeWidth={2} />
				{t("serverSettings.nav.invites")}
			</button>
			<button class="nav-item" class:active={section === "emoji"} onclick={() => (section = "emoji")}>
				<Smile size={16} strokeWidth={2} />
				{t("serverSettings.nav.emoji")}
			</button>
			<button class="nav-item" class:active={section === "roles"} onclick={() => (section = "roles")}>
				<ShieldCheck size={16} strokeWidth={2} />
				{t("serverSettings.nav.roles")}
			</button>
			<button class="nav-item" class:active={section === "members"} onclick={() => (section = "members")}>
				<Users size={16} strokeWidth={2} />
				{t("serverSettings.nav.members")}
			</button>
			<button class="nav-item" class:active={section === "bans"} onclick={() => (section = "bans")}>
				<UserX size={16} strokeWidth={2} />
				{t("serverSettings.nav.bans")}
			</button>
			<button class="nav-item" class:active={section === "moderation"} onclick={() => (section = "moderation")}>
				<ShieldAlert size={16} strokeWidth={2} />
				{t("serverSettings.nav.moderation")}
			</button>

			<div class="nav-spacer"></div>

			<button class="nav-item danger" onclick={() => (confirmDelete = true)}>
				<Trash2 size={16} strokeWidth={2} />
				{t("serverSettings.nav.deleteServer")}
			</button>
		</nav>

		<div class="content">
			<button class="close" onclick={onClose} title={t("common.close")}>
				<X size={20} strokeWidth={2} />
			</button>

			{#if section === "overview"}
				<h2>{t("serverSettings.overview.title")}</h2>

				<div class="card">
					<div class="identity">
						<button
							class="server-icon"
							style:background-image={server.iconUrl ? `url(${resolveUrl(server.iconUrl, session.token)})` : undefined}
							onclick={() => iconInput?.click()}
							disabled={iconUploading}
							title={t("serverSettings.overview.changeIcon")}
						>
							{#if !server.iconUrl}{server.initials}{/if}
						</button>
						<input bind:this={iconInput} type="file" accept="image/*" hidden onchange={onIconChosen} />
						<div>
							<p class="hint">{t("serverSettings.overview.serverIcon")}</p>
							<p class="hint muted">
								{#if server.iconUrl}
									<button class="link" onclick={() => iconInput?.click()}>{t("common.change")}</button> ·
									<button class="link" onclick={removeIcon}>{t("common.remove")}</button>
								{:else}
									{t("serverSettings.overview.iconHint")}
								{/if}
							</p>
						</div>
					</div>

					<label class="field">
						{t("serverSettings.overview.serverName")}
						<div class="row-input">
							<input type="text" bind:value={name} maxlength="48" />
							<button class="save" disabled={!name.trim() || name === server.name} onclick={saveName}>
								{t("common.save")}
							</button>
						</div>
					</label>

					<div class="row">
						<div>
							<p class="row-label">{t("serverSettings.overview.serverId")}</p>
							<p class="row-value muted">{server.id}</p>
						</div>
					</div>

					<div class="row">
						<div>
							<p class="row-label">{t("serverSettings.overview.channelsLabel")}</p>
							<p class="row-value muted">{tp("serverSettings.overview.channelsTotal", server.channels.length)}</p>
						</div>
					</div>
				</div>
			{:else if section === "channels"}
				<h2>{t("serverSettings.channels.title")}</h2>
				<div class="card">
					<p class="row-label">{t("serverSettings.channels.slowmode")}</p>
					<p class="row-value muted" style="margin-bottom: 12px;">
						{t("serverSettings.channels.slowmodeHint")}
					</p>
					{#each server.channels.filter((c) => c.type === "text") as channel (channel.id)}
						<div class="row slowmode-row">
							<span class="channel-name">
								<Hash size={14} strokeWidth={2} />
								{channel.name}
							</span>
							{#if isOwner}
								<span class="select-slot">
									<Dropdown
										value={String(channel.slowmodeSeconds ?? 0)}
										options={SLOWMODE_OPTIONS.map((opt) => ({
											value: String(opt.seconds),
											label: opt.label
										}))}
										onChange={(v) => changeSlowmode(channel.id, Number(v))}
									/>
								</span>
							{:else}
								<span class="row-value muted">
									{SLOWMODE_OPTIONS.find((o) => o.seconds === (channel.slowmodeSeconds ?? 0))?.label ?? t("serverSettings.channels.slowmodeOff")}
								</span>
							{/if}
						</div>
					{/each}
				</div>
			{:else if section === "invites"}
				<h2>{t("serverSettings.invites.title")}</h2>
				<div class="card">
					<p class="row-label">{t("serverSettings.invites.activeLink")}</p>
					<p class="row-value muted" style="margin-bottom: 12px;">
						{t("serverSettings.invites.activeLinkHint")}
					</p>
					<button class="save" onclick={() => (inviteOpen = true)}>{t("serverSettings.invites.showLink")}</button>
				</div>
				{#if isOwner}
					<div class="card">
						<p class="row-label">{t("serverSettings.invites.vanityCode")}</p>
						{#if isPremiumOwner}
							<p class="row-value muted" style="margin-bottom: 12px;">
								{t("serverSettings.invites.vanityHint")}
							</p>
							<div class="row-input">
								<input type="text" bind:value={vanityCode} maxlength="32" placeholder="your-server" />
								<button
									class="save"
									disabled={vanitySaving || !vanityCode.trim() || vanityCode === inviteCode}
									onclick={saveVanityCode}
								>
									{vanitySaving ? t("common.saving") : t("common.save")}
								</button>
							</div>
						{:else}
							<p class="row-value muted">
								{t("serverSettings.invites.vanityUpsell")}
							</p>
						{/if}
					</div>
				{/if}
			{:else if section === "emoji"}
				<h2>{t("serverSettings.emoji.title")}</h2>
				<div class="card">
					<p class="row-label">{t("serverSettings.emoji.slots")}</p>
					<p class="row-value muted" style="margin-bottom: 12px;">
						{#if boostStatus}
							{t("serverSettings.emoji.slotsUsed", { used: customEmoji.length, total: boostStatus.emoji_slots })}
							{#if boostStatus.emoji_slots < 30}
								{t("serverSettings.emoji.slotsMore")}
							{/if}
						{:else}
							{t("common.loading")}
						{/if}
					</p>
					{#if isOwner}
						<div class="row-input">
							<input type="text" placeholder="emoji_name" bind:value={newEmojiName} maxlength="32" />
							<button
								class="save"
								disabled={emojiUploading || !newEmojiName.trim() || (boostStatus ? customEmoji.length >= boostStatus.emoji_slots : true)}
								onclick={() => emojiInput?.click()}
							>
								{emojiUploading ? t("common.uploading") : t("serverSettings.emoji.upload")}
							</button>
							<input bind:this={emojiInput} type="file" accept="image/*" hidden onchange={onEmojiFileChosen} />
						</div>
					{/if}
				</div>
				{#if customEmoji.length > 0}
					<div class="card emoji-grid">
						{#each customEmoji as emoji (emoji.id)}
							<div class="emoji-item">
								<img src={resolveUrl(emoji.image_url, session.token)} alt={emoji.name} />
								<span class="emoji-name">:{emoji.name}:</span>
								{#if isOwner}
									<button class="emoji-remove" title={t("common.remove")} onclick={() => removeEmoji(emoji)}>
										<Trash2 size={13} strokeWidth={2} />
									</button>
								{/if}
							</div>
						{/each}
					</div>
				{/if}
			{:else if section === "roles"}
				<h2>{t("serverSettings.roles.title")}</h2>
				<div class="card">
					<div class="row-input" style="margin-bottom: 4px;">
						<input type="text" placeholder={t("serverSettings.roles.newRoleName")} bind:value={newRoleName} maxlength="32" />
						<button class="save" disabled={!newRoleName.trim()} onclick={addRole}>
							<Plus size={14} strokeWidth={2.5} />
							{t("common.create")}
						</button>
					</div>
				</div>
				{#each roles as role (role.id)}
					<div class="card role-card">
						<div class="role-header">
							<span class="role-swatch" style:background={role.color}></span>
							<input
								class="role-name-input"
								type="text"
								value={role.name}
								maxlength="32"
								onblur={(e) => renameRole(role, e.currentTarget.value)}
							/>
							<ColorPicker bind:value={role.color} onCommit={(hex) => recolorRole(role, hex)} />
							<button class="icon-danger" title={t("serverSettings.roles.deleteRole")} onclick={() => removeRole(role)}>
								<Trash2 size={14} strokeWidth={2} />
							</button>
						</div>
						<div class="permission-grid">
							{#each PERMISSION_LABELS as perm (perm.key)}
								<label class="permission-row">
									<input
										type="checkbox"
										checked={(role.permissions & PERMISSIONS[perm.key]) !== 0}
										onchange={() => togglePermission(role, PERMISSIONS[perm.key])}
									/>
									<span class="permission-check">
										{#if (role.permissions & PERMISSIONS[perm.key]) !== 0}
											<Check size={12} strokeWidth={3} />
										{/if}
									</span>
									<span>
										<span class="permission-label">{perm.label}</span>
										<span class="permission-desc">{perm.description}</span>
									</span>
								</label>
							{/each}
						</div>
					</div>
				{/each}
				{#if roles.length === 0}
					<p class="row-value muted">{t("serverSettings.roles.empty")}</p>
				{/if}
			{:else if section === "members"}
				<h2>{t("serverSettings.members.title")}</h2>
				{#each members as member (member.id)}
					<div class="card member-card">
						<div class="member-header">
							<span class="member-name">
								{member.username}
								{#if member.is_owner}<span class="owner-badge">{t("serverSettings.members.owner")}</span>{/if}
							</span>
							{#if !member.is_owner}
								<div class="member-actions">
									<button class="ghost-small" onclick={() => kick(member)}>{t("serverSettings.members.kick")}</button>
									<button class="ghost-small danger-text" onclick={() => ban(member)}>{t("serverSettings.members.ban")}</button>
								</div>
							{/if}
						</div>
						{#if !member.is_owner && roles.length > 0}
							<div class="role-chips">
								{#each roles as role (role.id)}
									<button
										class="role-chip-toggle"
										class:active={member.roles.some((r) => r.id === role.id)}
										style:border-color={role.color}
										style:color={member.roles.some((r) => r.id === role.id) ? role.color : undefined}
										onclick={() => toggleMemberRole(member, role)}
									>
										{role.name}
									</button>
								{/each}
							</div>
						{/if}
					</div>
				{/each}
			{:else if section === "bans"}
				<h2>{t("serverSettings.bans.title")}</h2>
				{#if bans.length === 0}
					<p class="row-value muted">{t("serverSettings.bans.empty")}</p>
				{/if}
				{#each bans as b (b.user_id)}
					<div class="card member-card">
						<div class="member-header">
							<span class="member-name">{b.username}</span>
							<button class="ghost-small" onclick={() => unban(b)}>{t("serverSettings.bans.unban")}</button>
						</div>
						{#if b.reason}<p class="row-value muted">{b.reason}</p>{/if}
					</div>
				{/each}
			{:else}
				<h2>{t("serverSettings.moderation.title")}</h2>
				<div class="card">
					<div class="row">
						<div>
							<p class="row-label">{t("serverSettings.moderation.verificationLevel")}</p>
							<p class="row-value muted">{t("serverSettings.moderation.verificationLevelHint")}</p>
						</div>
					</div>
					<div class="row">
						<div>
							<p class="row-label">{t("serverSettings.moderation.messageLogging")}</p>
							<p class="row-value muted">{t("serverSettings.moderation.messageLoggingHint")}</p>
						</div>
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>

{#if inviteOpen}
	<InviteModal serverName={server.name} serverId={server.id} onClose={() => (inviteOpen = false)} />
{/if}

{#if confirmDelete}
	<div class="confirm-overlay" role="presentation" onclick={() => (confirmDelete = false)} transition:fade={{ duration: 120 }}>
		<div class="confirm" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()}>
			<h3>{t("serverSettings.deleteConfirm.title", { name: server.name })}</h3>
			<p>{t("serverSettings.deleteConfirm.body")}</p>
			<div class="confirm-actions">
				<button class="cancel" onclick={() => (confirmDelete = false)}>{t("common.cancel")}</button>
				<button class="delete" onclick={deleteServer}>{t("serverSettings.nav.deleteServer")}</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.overlay {
		position: absolute;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		z-index: 100;
	}

	.modal {
		display: flex;
		width: 100%;
		height: 100%;
		background: var(--panel);
	}

	.nav {
		width: 220px;
		flex-shrink: 0;
		background: var(--sidebar);
		padding: 24px 12px;
		display: flex;
		flex-direction: column;
	}

	.nav-label {
		margin: 0 8px 8px;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--ink-faint);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.nav-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px;
		border-radius: 6px;
		font-size: 14px;
		font-weight: 500;
		color: var(--ink-dim);
		text-align: left;
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.nav-item:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.nav-item.active {
		background: var(--active);
		color: var(--ink);
	}

	.nav-spacer {
		flex: 1;
	}

	.nav-item.danger {
		color: var(--danger);
	}

	.nav-item.danger:hover {
		background: rgba(216, 60, 62, 0.12);
		color: var(--danger);
	}

	.content {
		flex: 1;
		position: relative;
		padding: 48px 40px;
		max-width: 660px;
		overflow-y: auto;
	}

	.close {
		position: absolute;
		top: 24px;
		right: 24px;
		padding: 8px;
		border-radius: 50%;
		border: 1px solid var(--hairline);
		color: var(--ink-dim);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.close:hover {
		background: var(--hover);
		color: var(--ink);
	}

	h2 {
		margin: 0 0 20px;
		font-family: var(--font-display);
		font-weight: 700;
		font-size: 20px;
	}

	.card {
		background: var(--sidebar);
		border-radius: 8px;
		padding: 20px;
		margin-bottom: 16px;
	}

	.emoji-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(90px, 1fr));
		gap: 10px;
	}

	.emoji-item {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 4px;
		padding: 10px 6px;
		border-radius: 8px;
		background: var(--active);
	}

	.emoji-item img {
		width: 36px;
		height: 36px;
		object-fit: contain;
	}

	.emoji-name {
		font-size: 11px;
		color: var(--ink-faint);
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.emoji-remove {
		position: absolute;
		top: 4px;
		right: 4px;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background: var(--danger);
		color: white;
		opacity: 0;
		transition: opacity 0.15s ease;
	}

	.emoji-item:hover .emoji-remove {
		opacity: 1;
	}

	.identity {
		display: flex;
		align-items: center;
		gap: 14px;
		margin-bottom: 20px;
	}

	.server-icon {
		width: 48px;
		height: 48px;
		border-radius: 16px;
		background: var(--accent-fill);
		background-position: center;
		background-size: 100% 100%;
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 15px;
		flex-shrink: 0;
		overflow: hidden;
		transition: opacity 0.15s ease;
	}

	.server-icon:hover:not(:disabled) {
		opacity: 0.85;
	}

	.server-icon:disabled {
		opacity: 0.6;
	}

	.link {
		color: var(--ink-dim);
		text-decoration: underline;
		font-size: inherit;
	}

	.link:hover {
		color: var(--ink);
	}

	.hint {
		margin: 0;
		font-size: 13px;
		color: var(--ink);
	}

	.hint.muted {
		margin-top: 2px;
		font-size: 12px;
		color: var(--ink-faint);
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--ink-dim);
		margin-bottom: 4px;
	}

	.row-input {
		display: flex;
		gap: 8px;
	}

	.row-input input {
		flex: 1;
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 10px 12px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 14px;
	}

	.row-input input:focus {
		outline: none;
		border-color: var(--ink-dim);
	}

	.save {
		flex-shrink: 0;
		padding: 10px 16px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-weight: 700;
		font-size: 13px;
	}

	.save:disabled {
		background: var(--active);
		color: var(--ink-faint);
	}

	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 0;
		border-top: 1px solid var(--hairline);
	}

	.row-label {
		margin: 0 0 4px;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--ink-faint);
	}

	.row-value {
		margin: 0;
		font-size: 14px;
		color: var(--ink);
	}

	.row-value.muted {
		color: var(--ink-dim);
		line-height: 1.5;
	}

	.slowmode-row {
		padding: 10px 0;
	}

	.channel-name {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 14px;
		color: var(--ink);
	}

	.channel-name :global(svg) {
		color: var(--ink-faint);
	}

	.select-slot {
		display: block;
		width: 150px;
		flex-shrink: 0;
	}

	.role-card {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.role-header {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.role-swatch {
		width: 14px;
		height: 14px;
		border-radius: 4px;
		flex-shrink: 0;
	}

	.role-name-input {
		flex: 1;
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 8px 10px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 13px;
		font-weight: 600;
	}

	.role-name-input:focus {
		outline: none;
		border-color: var(--ink-dim);
	}

	.icon-danger {
		flex-shrink: 0;
		display: flex;
		padding: 8px;
		border-radius: 6px;
		color: var(--ink-faint);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.icon-danger:hover {
		background: rgba(216, 60, 62, 0.12);
		color: var(--danger);
	}

	.permission-grid {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.permission-row {
		display: flex;
		align-items: flex-start;
		gap: 10px;
		font-size: 13px;
		cursor: pointer;
	}

	.permission-row input {
		position: absolute;
		width: 1px;
		height: 1px;
		opacity: 0;
	}

	.permission-check {
		flex-shrink: 0;
		margin-top: 2px;
		width: 18px;
		height: 18px;
		border-radius: 5px;
		border: 1px solid var(--ink-faint);
		background: var(--panel);
		color: var(--void);
		display: flex;
		align-items: center;
		justify-content: center;
		transition: background-color 0.15s ease, border-color 0.15s ease;
	}

	.permission-row input:checked + .permission-check {
		background: var(--accent-fill);
		border-color: var(--accent-fill);
	}

	.permission-row input:focus-visible + .permission-check {
		box-shadow: 0 0 0 3px var(--accent-soft);
	}

	.permission-label {
		display: block;
		font-weight: 600;
		color: var(--ink);
	}

	.permission-desc {
		display: block;
		font-size: 12px;
		color: var(--ink-faint);
	}

	.member-card {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.member-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
	}

	.member-name {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 14px;
		font-weight: 600;
		color: var(--ink);
	}

	.owner-badge {
		padding: 2px 8px;
		border-radius: 999px;
		background: var(--active);
		color: var(--ink-faint);
		font-size: 10px;
		font-weight: 700;
		text-transform: uppercase;
	}

	.member-actions {
		display: flex;
		gap: 6px;
	}

	.ghost-small {
		padding: 6px 10px;
		border-radius: 6px;
		font-size: 12px;
		font-weight: 600;
		color: var(--ink-dim);
	}

	.ghost-small:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.ghost-small.danger-text {
		color: var(--danger);
	}

	.ghost-small.danger-text:hover {
		background: rgba(216, 60, 62, 0.12);
		color: var(--danger);
	}

	.role-chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.role-chip-toggle {
		padding: 4px 10px;
		border-radius: 999px;
		border: 1px solid var(--hairline);
		background: none;
		font-size: 12px;
		font-weight: 600;
		color: var(--ink-faint);
		transition: background-color 0.15s ease, color 0.15s ease, border-color 0.15s ease;
	}

	.role-chip-toggle.active {
		background: var(--active);
	}

	.confirm-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 300;
	}

	.confirm {
		width: 420px;
		background: var(--sidebar);
		border-radius: 10px;
		padding: 24px;
		box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
	}

	.confirm h3 {
		margin: 0 0 10px;
		font-family: var(--font-body);
		font-size: 17px;
	}

	.confirm p {
		margin: 0 0 20px;
		font-size: 13px;
		color: var(--ink-dim);
		line-height: 1.5;
	}

	.confirm-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}

	.cancel {
		padding: 9px 16px;
		border-radius: 6px;
		color: var(--ink-dim);
	}

	.cancel:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.delete {
		padding: 9px 16px;
		border-radius: 6px;
		background: var(--danger);
		color: white;
		font-weight: 600;
	}
</style>
