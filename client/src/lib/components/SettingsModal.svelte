<script lang="ts">
	import { fade, scale, fly } from "svelte/transition";
	import { cubicOut } from "svelte/easing";
	import X from "@lucide/svelte/icons/x";
	import Search from "@lucide/svelte/icons/search";
	import UserRound from "@lucide/svelte/icons/user-round";
	import UserPen from "@lucide/svelte/icons/user-pen";
	import ShieldCheck from "@lucide/svelte/icons/shield-check";
	import Bell from "@lucide/svelte/icons/bell";
	import Palette from "@lucide/svelte/icons/palette";
	import Accessibility from "@lucide/svelte/icons/accessibility";
	import Monitor from "@lucide/svelte/icons/monitor";
	import LogOut from "@lucide/svelte/icons/log-out";
	import CreditCard from "@lucide/svelte/icons/credit-card";
	import Sparkles from "@lucide/svelte/icons/sparkles";
	import Check from "@lucide/svelte/icons/check";
	import Copy from "@lucide/svelte/icons/copy";
	import ShieldPlus from "@lucide/svelte/icons/shield-plus";
	import Smartphone from "@lucide/svelte/icons/smartphone";
	import ImagePlus from "@lucide/svelte/icons/image-plus";
	import Pencil from "@lucide/svelte/icons/pencil";
	import ShieldAlert from "@lucide/svelte/icons/shield-alert";
	import MessageSquare from "@lucide/svelte/icons/message-square";
	import { openUrl } from "@tauri-apps/plugin-opener";
	import QRCode from "qrcode";
	import { renameLocalIdentity } from "$lib/crypto/identity";
	import { renameAllSessions } from "$lib/crypto/session-store";
	import { renameAllGroupKeys } from "$lib/crypto/group-key-store";
	import { toast } from "$lib/stores/toast.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { deviceLink } from "$lib/devicelink/link.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import { badgeStore } from "$lib/stores/badges.svelte";
	import { themeStore, COLOR_GROUPS, COLOR_LABELS } from "$lib/stores/theme.svelte";
	import { notificationSettings } from "$lib/stores/notifications.svelte";
	import { pendingDm } from "$lib/stores/pendingDm.svelte";
	import Badges from "$lib/components/Badges.svelte";
	import ColorPicker from "$lib/components/ColorPicker.svelte";
	import * as api from "$lib/api/client";

	let { username, onClose, onLogout, initialSection = "account" }: {
		username: string;
		onClose: () => void;
		onLogout: () => void;
		initialSection?: Section;
	} = $props();

	type Section =
		| "profile"
		| "account"
		| "privacy"
		| "notifications"
		| "appearance"
		| "accessibility"
		| "sessions"
		| "billing"
		| "moderation";
	let section = $state<Section>(initialSection);
	let navSearch = $state("");

	const NAV_ITEMS: { section: Section; label: string }[] = [
		{ section: "profile", label: "Profile" },
		{ section: "account", label: "My Account" },
		{ section: "privacy", label: "Privacy & Safety" },
		{ section: "notifications", label: "Notifications" },
		{ section: "sessions", label: "Devices" },
		{ section: "appearance", label: "Appearance" },
		{ section: "accessibility", label: "Accessibility" },
		{ section: "billing", label: "Billing" }
	];

	function matchesSearch(label: string) {
		return !navSearch.trim() || label.toLowerCase().includes(navSearch.trim().toLowerCase());
	}

	function goToAccountField(anchorId: string) {
		section = "account";
		requestAnimationFrame(() => {
			document.getElementById(anchorId)?.scrollIntoView({ behavior: "smooth", block: "start" });
		});
	}

	let billing = $state<api.BillingStatus | null>(null);
	let checkoutLoading = $state(false);
	let myServers = $state<api.ApiServer[]>([]);
	let boostedServerIds = $state<Set<string>>(new Set());
	let boostBusyId = $state<string | null>(null);

	$effect(() => {
		const token = session.token;
		if (!token) return;
		api
			.billingStatus(token)
			.then((status) => (billing = status))
			.catch(() => {});
	});

	$effect(() => {
		const token = session.token;
		if (!token || section !== "billing") return;
		api
			.listServers(token)
			.then((servers) => {
				myServers = servers;
				return Promise.all(servers.map((s) => api.getBoosts(token, s.id).then((b) => [s.id, b.boosted_by_me] as const)));
			})
			.then((pairs) => {
				boostedServerIds = new Set(pairs.filter(([, boosted]) => boosted).map(([id]) => id));
			})
			.catch(() => {});
	});

	let reports = $state<api.ReportSummary[]>([]);
	let reportsLoading = $state(false);
	let reportStatusFilter = $state<api.ReportStatus | "all">("open");

	$effect(() => {
		const token = session.token;
		if (!token || section !== "moderation" || !isStaff) return;
		reportsLoading = true;
		api
			.listReports(token)
			.then((rows) => (reports = rows))
			.catch(() => toast.push("Couldn't load reports"))
			.finally(() => (reportsLoading = false));
	});

	const visibleReports = $derived(
		reportStatusFilter === "all" ? reports : reports.filter((r) => r.status === reportStatusFilter)
	);

	async function setReportStatus(report: api.ReportSummary, status: api.ReportStatus) {
		const token = session.token;
		if (!token) return;
		const previous = report.status;
		report.status = status;
		try {
			await api.updateReportStatus(token, report.id, status);
		} catch {
			report.status = previous;
			toast.push("Couldn't update report");
		}
	}

	function messageReporter(username: string) {
		pendingDm.request(username);
		onClose();
	}

	const REPORT_FILTERS: (api.ReportStatus | "all")[] = ["open", "reviewing", "resolved", "dismissed", "all"];

	async function toggleBoost(serverId: string) {
		const token = session.token;
		if (!token || boostBusyId) return;
		boostBusyId = serverId;
		const wasBoosted = boostedServerIds.has(serverId);
		try {
			const status = wasBoosted
				? await api.removeBoost(token, serverId)
				: await api.addBoost(token, serverId);
			const next = new Set(boostedServerIds);
			if (status.boosted_by_me) next.add(serverId);
			else next.delete(serverId);
			boostedServerIds = next;
			if (billing) {
				billing = { ...billing, boost_slots_used: billing.boost_slots_used + (wasBoosted ? -1 : 1) };
			}
		} catch (err) {
			if (err instanceof api.ApiError && err.status === 409) {
				toast.push("All your Void Shards are already assigned");
			} else if (err instanceof api.ApiError && err.status === 403) {
				toast.push("Void Shards are a Premium perk");
			} else {
				toast.push("Couldn't update boost");
			}
		} finally {
			boostBusyId = null;
		}
	}

	async function upgrade() {
		const token = session.token;
		if (!token) return;
		checkoutLoading = true;
		try {
			const { url } = await api.createCheckout(token);
			await openUrl(url);
		} catch (err) {
			if (err instanceof api.ApiError && err.status === 503) {
				toast.push("Billing isn't configured on this server yet");
			} else {
				toast.push("Couldn't start checkout");
			}
		} finally {
			checkoutLoading = false;
		}
	}

	const initialUsername = username;

	let editingUsername = $state(false);
	let usernameDraft = $state(initialUsername);
	let editingPassword = $state(false);
	let regeneratedPassword = $state<string | null>(null);
	let regenerating = $state(false);
	let passwordCopied = $state(false);

	let notifyMessages = $state(true);
	let notifySounds = $state(true);

	let reducedMotion = $state(false);
	let compactMode = $state(false);

	type TotpStage = "idle" | "enabled" | "setting-up" | "backup-codes" | "disabling" | "regenerating";
	let totpStage = $state<TotpStage>("idle");
	let totpSecret = $state("");
	let totpQrDataUrl = $state("");
	let totpCodeInput = $state("");
	let totpBackupCodes = $state<string[]>([]);
	let totpBusy = $state(false);
	let totpError = $state("");

	function loadTotpStatus() {
		const token = session.token;
		if (!token) return;
		api
			.fetchTotpStatus(token)
			.then((res) => {
				totpStage = res.enabled ? "enabled" : "idle";
			})
			.catch(() => {});
	}

	async function beginTotpSetup() {
		const token = session.token;
		if (!token) return;
		totpBusy = true;
		totpError = "";
		try {
			const setup = await api.setupTotp(token);
			totpSecret = setup.secret;
			totpQrDataUrl = await QRCode.toDataURL(setup.otpauth_url, { margin: 1, width: 200 });
			totpCodeInput = "";
			totpStage = "setting-up";
		} catch {
			toast.push("Couldn't start 2FA setup");
		} finally {
			totpBusy = false;
		}
	}

	async function confirmTotpSetup() {
		const token = session.token;
		if (!token) return;
		totpBusy = true;
		totpError = "";
		try {
			const result = await api.verifyTotp(token, totpCodeInput.trim());
			totpBackupCodes = result.backup_codes;
			totpCodeInput = "";
			totpStage = "backup-codes";
			toast.push("Two-factor authentication enabled");
		} catch {
			totpError = "That code didn't work — try again";
		} finally {
			totpBusy = false;
		}
	}

	function finishBackupCodesReview() {
		totpBackupCodes = [];
		totpStage = "enabled";
	}

	async function confirmTotpDisable() {
		const token = session.token;
		if (!token) return;
		totpBusy = true;
		totpError = "";
		try {
			await api.disableTotp(token, totpCodeInput.trim());
			totpCodeInput = "";
			totpStage = "idle";
			toast.push("Two-factor authentication disabled");
		} catch {
			totpError = "That code didn't work — try again";
		} finally {
			totpBusy = false;
		}
	}

	async function confirmRegenerateBackupCodes() {
		const token = session.token;
		if (!token) return;
		totpBusy = true;
		totpError = "";
		try {
			const result = await api.regenerateBackupCodes(token, totpCodeInput.trim());
			totpBackupCodes = result.backup_codes;
			totpCodeInput = "";
			totpStage = "backup-codes";
		} catch {
			totpError = "That code didn't work — try again";
		} finally {
			totpBusy = false;
		}
	}

	let sessions = $state<api.ApiSession[]>([]);

	function loadSessions() {
		const token = session.token;
		if (!token) return;
		api
			.listSessions(token)
			.then((rows) => (sessions = rows))
			.catch(() => {});
	}

	let blocked = $state<api.ApiBlockedUser[]>([]);

	function loadBlocked() {
		const token = session.token;
		if (!token) return;
		api
			.listBlocked(token)
			.then((rows) => (blocked = rows))
			.catch(() => {});
	}

	function unblock(id: string) {
		const token = session.token;
		if (!token) return;
		api
			.unblockUser(token, id)
			.then(() => {
				blocked = blocked.filter((b) => b.id !== id);
				toast.push("Unblocked");
			})
			.catch(() => toast.push("Couldn't unblock"));
	}

	let displayNameDraft = $state("");
	let bioDraft = $state("");
	let pronounsDraft = $state("");
	let statusTextDraft = $state("");
	let accentColorDraft = $state("#5b96c9");
	let bannerColorDraft = $state("#2b2d31");
	let profileSaving = $state(false);
	let avatarUploading = $state(false);
	let bannerUploading = $state(false);
	let avatarInput: HTMLInputElement | undefined;
	let bannerInput: HTMLInputElement | undefined;

	function syncProfileDrafts() {
		const profile = profileStore.forUser(username);
		if (!profile) return;
		displayNameDraft = profile.display_name ?? "";
		bioDraft = profile.bio ?? "";
		pronounsDraft = profile.pronouns ?? "";
		statusTextDraft = profile.status_text ?? "";
		accentColorDraft = profile.accent_color ?? "#5b96c9";
		bannerColorDraft = profile.banner_color ?? "#2b2d31";
	}

	async function loadOwnProfile() {
		const token = session.token;
		if (!token) return;
		await profileStore.load(token, username);
		syncProfileDrafts();
	}

	async function saveProfile() {
		const token = session.token;
		if (!token) return;
		profileSaving = true;
		try {
			const updated = await api.updateProfile(token, {
				display_name: displayNameDraft,
				bio: bioDraft,
				pronouns: pronounsDraft,
				status_text: statusTextDraft,
				accent_color: accentColorDraft,
				banner_color: bannerColorDraft
			});
			profileStore.set(updated);
			toast.push("Profile saved");
		} catch {
			toast.push("Couldn't save profile");
		} finally {
			profileSaving = false;
		}
	}

	async function toggleShareActivity(value: boolean) {
		const token = session.token;
		if (!token) return;
		try {
			const updated = await api.updateProfile(token, { share_activity: value });
			profileStore.set(updated);
		} catch {
			toast.push("Couldn't update activity sharing");
		}
	}

	async function onAvatarChosen(event: Event) {
		const token = session.token;
		const file = (event.target as HTMLInputElement).files?.[0];
		if (!token || !file) return;
		avatarUploading = true;
		try {
			const attachment = await api.uploadFile(token, file);
			const updated = await api.setAvatar(token, attachment.id);
			profileStore.set(updated);
		} catch {
			toast.push("Couldn't update avatar");
		} finally {
			avatarUploading = false;
			if (avatarInput) avatarInput.value = "";
		}
	}

	async function removeAvatar() {
		const token = session.token;
		if (!token) return;
		try {
			profileStore.set(await api.clearAvatar(token));
		} catch {
			toast.push("Couldn't remove avatar");
		}
	}

	async function onBannerChosen(event: Event) {
		const token = session.token;
		const file = (event.target as HTMLInputElement).files?.[0];
		if (!token || !file) return;
		bannerUploading = true;
		try {
			const attachment = await api.uploadFile(token, file);
			const updated = await api.setBanner(token, attachment.id);
			profileStore.set(updated);
		} catch {
			toast.push("Couldn't update banner");
		} finally {
			bannerUploading = false;
			if (bannerInput) bannerInput.value = "";
		}
	}

	async function removeBanner() {
		const token = session.token;
		if (!token) return;
		try {
			profileStore.set(await api.clearBanner(token));
		} catch {
			toast.push("Couldn't remove banner");
		}
	}

	$effect(() => {
		if (session.token) loadOwnProfile();
	});

	$effect(() => {
		if (session.token) badgeStore.loadForUser(session.token, username);
	});

	const isStaff = $derived(
		badgeStore.forUser(username).includes("owner") || badgeStore.forUser(username).includes("staff")
	);

	$effect(() => {
		if (section === "sessions" && session.token) loadSessions();
		if (section === "privacy" && session.token) loadBlocked();
		if (section === "account" && session.token) loadTotpStatus();
		if (section === "profile" && session.token) {
			badgeStore.loadForUser(session.token, username);
		}
	});

	$effect(() => {
		return () => deviceLink.reset();
	});

	function beginDeviceLink() {
		const token = session.token;
		if (!token) return;
		deviceLink.start(token, username);
	}

	function confirmDeviceLink() {
		deviceLink.confirmAndSend(username);
	}

	function cancelDeviceLink() {
		deviceLink.reset();
	}

	function describeSession(s: api.ApiSession): string {
		const when = new Date(s.created_at).toLocaleDateString([], { month: "short", day: "numeric" });
		return `Signed in ${when}`;
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") onClose();
	}

	async function saveUsername() {
		const token = session.token;
		const newUsername = usernameDraft.trim();
		if (!token || !newUsername || newUsername === username) {
			editingUsername = false;
			return;
		}
		try {
			await api.changeUsername(token, newUsername);
			renameLocalIdentity(username, newUsername);
			renameAllSessions(username, newUsername);
			renameAllGroupKeys(username, newUsername);
			session.set(token, newUsername);
			editingUsername = false;
			toast.push("Username updated");
		} catch (err) {
			if (err instanceof api.ApiError && err.status === 409) {
				toast.push("That username is already taken");
			} else {
				toast.push("Couldn't change username");
			}
		}
	}

	async function regeneratePassword() {
		const token = session.token;
		if (!token) return;
		regenerating = true;
		try {
			const res = await api.regeneratePassword(token);
			regeneratedPassword = res.password;
		} catch {
			toast.push("Couldn't generate a new password");
			editingPassword = false;
		} finally {
			regenerating = false;
		}
	}

	async function copyRegeneratedPassword() {
		if (!regeneratedPassword) return;
		await navigator.clipboard.writeText(regeneratedPassword);
		passwordCopied = true;
		setTimeout(() => (passwordCopied = false), 1500);
	}

	function closePasswordChange() {
		editingPassword = false;
		regeneratedPassword = null;
	}

	function revoke(id: string) {
		const token = session.token;
		if (!token) return;
		api
			.revokeSession(token, id)
			.then(() => {
				sessions = sessions.filter((s) => s.id !== id);
				toast.push("Session revoked");
			})
			.catch(() => toast.push("Couldn't revoke session"));
	}

	function toggle(setter: (v: boolean) => void, current: boolean, label: string) {
		setter(!current);
		toast.push(`${label} ${!current ? "enabled" : "disabled"}`);
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="overlay" role="presentation" onclick={onClose} transition:fade={{ duration: 150 }}>
	<div
		class="modal"
		role="dialog"
		aria-modal="true"
		aria-label="User settings"
		tabindex="-1"
		onclick={(e) => e.stopPropagation()}
		transition:scale={{ duration: 180, start: 0.97, easing: cubicOut }}
	>
		<nav class="nav">
			<button class="nav-identity" onclick={() => (section = "profile")}>
				<div class="nav-avatar" style:background-image={profileStore.forUser(username)?.avatar_url ? `url(${api.resolveUrl(profileStore.forUser(username)!.avatar_url!, session.token)})` : undefined}>
					{#if !profileStore.forUser(username)?.avatar_url}{username.slice(0, 2).toUpperCase()}{/if}
				</div>
				<div class="nav-identity-text">
					<p class="nav-identity-name">{profileStore.forUser(username)?.display_name || username}</p>
					<p class="nav-identity-edit"><Pencil size={10} strokeWidth={2.5} />Edit Profile</p>
				</div>
			</button>

			<label class="nav-search">
				<Search size={13} strokeWidth={2} />
				<input type="text" placeholder="Search" bind:value={navSearch} />
			</label>

			{#if matchesSearch("Profile")}
				<button class="nav-item" class:active={section === "profile"} onclick={() => (section = "profile")}>
					<UserPen size={16} strokeWidth={2} />
					Profile
				</button>
			{/if}
			{#if matchesSearch("My Account") || matchesSearch("Account Info") || matchesSearch("Password & Security")}
				<button class="nav-item" class:active={section === "account"} onclick={() => (section = "account")}>
					<UserRound size={16} strokeWidth={2} />
					My Account
				</button>
				{#if section === "account"}
					<button class="nav-subitem" onclick={() => goToAccountField("account-info")}>Account Info</button>
					<button class="nav-subitem" onclick={() => goToAccountField("account-security")}>Password &amp; Security</button>
				{/if}
			{/if}
			{#if matchesSearch("Privacy & Safety")}
				<button class="nav-item" class:active={section === "privacy"} onclick={() => (section = "privacy")}>
					<ShieldCheck size={16} strokeWidth={2} />
					Privacy &amp; Safety
				</button>
			{/if}
			{#if matchesSearch("Notifications")}
				<button class="nav-item" class:active={section === "notifications"} onclick={() => (section = "notifications")}>
					<Bell size={16} strokeWidth={2} />
					Notifications
				</button>
			{/if}
			{#if matchesSearch("Devices")}
				<button class="nav-item" class:active={section === "sessions"} onclick={() => (section = "sessions")}>
					<Monitor size={16} strokeWidth={2} />
					Devices
				</button>
			{/if}

			{#if matchesSearch("Appearance") || matchesSearch("Accessibility")}
				<p class="nav-label">Experience</p>
				{#if matchesSearch("Appearance")}
					<button class="nav-item" class:active={section === "appearance"} onclick={() => (section = "appearance")}>
						<Palette size={16} strokeWidth={2} />
						Appearance
					</button>
				{/if}
				{#if matchesSearch("Accessibility")}
					<button class="nav-item" class:active={section === "accessibility"} onclick={() => (section = "accessibility")}>
						<Accessibility size={16} strokeWidth={2} />
						Accessibility
					</button>
				{/if}
			{/if}

			{#if matchesSearch("Billing")}
				<p class="nav-label">Billing</p>
				<button class="nav-item" class:active={section === "billing"} onclick={() => (section = "billing")}>
					<CreditCard size={16} strokeWidth={2} />
					Billing
				</button>
			{/if}

			{#if isStaff && matchesSearch("Moderation")}
				<p class="nav-label">Staff</p>
				<button class="nav-item" class:active={section === "moderation"} onclick={() => (section = "moderation")}>
					<ShieldAlert size={16} strokeWidth={2} />
					Moderation
				</button>
			{/if}

			<div class="nav-spacer"></div>

			<button class="nav-item danger" onclick={onLogout}>
				<LogOut size={16} strokeWidth={2} />
				Log Out
			</button>
		</nav>

		<button class="close" onclick={onClose} title="Close">
			<X size={20} strokeWidth={2} />
		</button>

		<div class="content">
			{#if section === "profile"}
				{@const ownBadges = badgeStore.forUser(username)}
				<h2>Profile</h2>

				<div class="card no-pad" in:fade={{ duration: 140 }}>
					<div
						class="preview-banner"
						style:background={profileStore.forUser(username)?.banner_url
							? `url(${api.resolveUrl(profileStore.forUser(username)!.banner_url!, session.token)}) center/cover`
							: bannerColorDraft}
					></div>
					<div class="preview-body">
						<div class="preview-avatar-row">
							<div
								class="preview-avatar"
								style:background-image={profileStore.forUser(username)?.avatar_url ? `url(${api.resolveUrl(profileStore.forUser(username)!.avatar_url!, session.token)})` : undefined}
							>
								{#if !profileStore.forUser(username)?.avatar_url}{username.slice(0, 2).toUpperCase()}{/if}
							</div>
							<div class="preview-image-actions">
								<input bind:this={avatarInput} type="file" accept="image/*" hidden onchange={onAvatarChosen} />
								<button class="ghost small" onclick={() => avatarInput?.click()} disabled={avatarUploading}>
									<ImagePlus size={13} strokeWidth={2} />
									{avatarUploading ? "Uploading…" : "Change Avatar"}
								</button>
								{#if profileStore.forUser(username)?.avatar_url}
									<button class="ghost small danger-text" onclick={removeAvatar}>Remove</button>
								{/if}
							</div>
						</div>

						<p class="preview-name" style:color={accentColorDraft || undefined}>
							{displayNameDraft || username}
							{#if pronounsDraft}<span class="preview-pronouns">{pronounsDraft}</span>{/if}
							{#if ownBadges.length > 0}<Badges badges={ownBadges} />{/if}
						</p>
						{#if displayNameDraft}<p class="preview-handle">@{username}</p>{/if}
						{#if statusTextDraft}<p class="preview-status">{statusTextDraft}</p>{/if}

						<div class="preview-image-actions" style="margin-top: 10px;">
							<input bind:this={bannerInput} type="file" accept="image/*" hidden onchange={onBannerChosen} />
							<button class="ghost small" onclick={() => bannerInput?.click()} disabled={bannerUploading}>
								<ImagePlus size={13} strokeWidth={2} />
								{bannerUploading ? "Uploading…" : "Change Banner"}
							</button>
							{#if profileStore.forUser(username)?.banner_url}
								<button class="ghost small danger-text" onclick={removeBanner}>Remove</button>
							{/if}
						</div>
					</div>
				</div>

				<div class="card">
					<label class="field">
						Display Name
						<input class="inline-input" type="text" bind:value={displayNameDraft} maxlength="32" placeholder={username} />
					</label>
					<label class="field" style="margin-top: 14px;">
						About Me
						<textarea class="inline-textarea" bind:value={bioDraft} maxlength="190" rows="3" placeholder="Tell people a bit about yourself"></textarea>
					</label>
					<label class="field" style="margin-top: 14px;">
						Pronouns
						<input class="inline-input" type="text" bind:value={pronounsDraft} maxlength="40" placeholder="they/them" />
					</label>
					<label class="field" style="margin-top: 14px;">
						Custom Status
						<input class="inline-input" type="text" bind:value={statusTextDraft} maxlength="128" placeholder="What's happening?" />
					</label>
				</div>

				<div class="card">
					<div class="row">
						<div>
							<p class="row-label">Accent Color</p>
							<p class="row-value muted">Colors your name in profiles and popovers.</p>
						</div>
						<ColorPicker bind:value={accentColorDraft} />
					</div>
					<div class="row">
						<div>
							<p class="row-label">Banner Color</p>
							<p class="row-value muted">Used when you don't have a banner image.</p>
						</div>
						<ColorPicker bind:value={bannerColorDraft} />
					</div>
				</div>

				<div class="row-actions" style="justify-content: flex-end;">
					<button class="primary" onclick={saveProfile} disabled={profileSaving}>
						{profileSaving ? "Saving…" : "Save Changes"}
					</button>
				</div>
			{:else if section === "account"}
				<h2>My Account</h2>

				<div class="card" id="account-info">
					<div class="identity">
						<div
							class="avatar"
							style:background-image={profileStore.forUser(username)?.avatar_url ? `url(${api.resolveUrl(profileStore.forUser(username)!.avatar_url!, session.token)})` : undefined}
						>
							{#if !profileStore.forUser(username)?.avatar_url}{username.slice(0, 2).toUpperCase()}{/if}
						</div>
						<div>
							<p class="username">{username}</p>
							<p class="hint">HollowChat account</p>
						</div>
					</div>

					<div class="row">
						<div>
							<p class="row-label">Username</p>
							{#if editingUsername}
								<input class="inline-input" type="text" bind:value={usernameDraft} maxlength="32" />
							{:else}
								<p class="row-value">{username}</p>
							{/if}
						</div>
						{#if editingUsername}
							<div class="row-actions">
								<button class="ghost" onclick={() => ((editingUsername = false), (usernameDraft = username))}>Cancel</button>
								<button class="primary" onclick={saveUsername} disabled={!usernameDraft.trim()}>Save</button>
							</div>
						{:else}
							<button class="edit" onclick={() => (editingUsername = true)}>Edit</button>
						{/if}
					</div>

					<div class="row">
						<div>
							<p class="row-label">Email</p>
							<p class="row-value muted">Not collected — HollowChat never asks for one.</p>
						</div>
					</div>

					<div class="row">
						<div>
							<p class="row-label">Phone number</p>
							<p class="row-value muted">Not collected — HollowChat never asks for one.</p>
						</div>
					</div>
				</div>

				<div class="card" id="account-security">
					{#if !editingPassword}
						<div class="row">
							<div>
								<p class="row-label">Password</p>
								<p class="row-value">••••••••••••</p>
							</div>
							<button class="edit" onclick={() => (editingPassword = true)}>Change</button>
						</div>
					{:else if regeneratedPassword}
						<p class="row-label">Save your new password now</p>
						<p class="hint" style="margin-bottom: 12px;">
							This is the only time we'll show it. Your old password no longer works.
						</p>
						<div class="password-box">
							<code>{regeneratedPassword}</code>
							<button type="button" class="copy" onclick={copyRegeneratedPassword} title="Copy password">
								{#if passwordCopied}
									<Check size={15} strokeWidth={2.5} />
								{:else}
									<Copy size={15} strokeWidth={2} />
								{/if}
							</button>
						</div>
						<div class="row-actions" style="margin-top: 12px;">
							<button class="primary" onclick={closePasswordChange}>Done</button>
						</div>
					{:else}
						<p class="row-label">Generate a new password?</p>
						<p class="hint" style="margin-bottom: 12px;">
							HollowChat has no user-chosen passwords — we'll generate a new random one and show it once. Your current password stops working immediately.
						</p>
						<div class="row-actions">
							<button class="ghost" onclick={() => (editingPassword = false)}>Cancel</button>
							<button class="primary" onclick={regeneratePassword} disabled={regenerating}>
								{regenerating ? "Generating…" : "Generate New Password"}
							</button>
						</div>
					{/if}

					<div class="row">
						<div>
							<p class="row-label">Password recovery</p>
							<p class="row-value muted">
								There is no email or phone number on file, so there is no password reset. Losing
								your password means losing the account.
							</p>
						</div>
					</div>
				</div>

				<div class="card">
					{#if totpStage === "idle"}
						<div class="row">
							<div>
								<p class="row-label">Two-factor authentication</p>
								<p class="row-value muted">Not enabled. Add an authenticator app for extra login security.</p>
							</div>
							<button class="edit" onclick={beginTotpSetup} disabled={totpBusy}>
								<ShieldPlus size={14} strokeWidth={2} />
								Enable
							</button>
						</div>
					{:else if totpStage === "setting-up"}
						<p class="row-label">Scan this with your authenticator app</p>
						<p class="hint" style="margin-bottom: 12px;">
							Google Authenticator, Aegis, 1Password — anything that supports TOTP.
						</p>
						{#if totpQrDataUrl}
							<img class="totp-qr" src={totpQrDataUrl} alt="Two-factor authentication QR code" />
						{/if}
						<p class="hint" style="margin: 8px 0 4px;">Or enter this code manually:</p>
						<p class="row-value totp-secret">{totpSecret}</p>
						<label class="field" style="margin-top: 12px;">
							6-digit code
							<input class="inline-input" type="text" bind:value={totpCodeInput} placeholder="123456" maxlength="6" />
						</label>
						{#if totpError}<p class="error-text">{totpError}</p>{/if}
						<div class="row-actions" style="margin-top: 12px;">
							<button class="ghost" onclick={() => (totpStage = "idle")}>Cancel</button>
							<button class="primary" onclick={confirmTotpSetup} disabled={totpBusy || totpCodeInput.trim().length !== 6}>
								{totpBusy ? "Verifying…" : "Verify & Enable"}
							</button>
						</div>
					{:else if totpStage === "backup-codes"}
						<p class="row-label">Save your backup codes</p>
						<p class="hint" style="margin-bottom: 12px;">
							Each code works once, if you lose access to your authenticator app. There's no other
							way back into the account — store these somewhere safe.
						</p>
						<div class="backup-codes">
							{#each totpBackupCodes as code (code)}
								<code>{code}</code>
							{/each}
						</div>
						<div class="row-actions" style="margin-top: 12px;">
							<button class="primary" onclick={finishBackupCodesReview}>I saved these codes</button>
						</div>
					{:else if totpStage === "disabling"}
						<p class="row-label">Disable two-factor authentication</p>
						<p class="hint" style="margin-bottom: 12px;">Enter a current code from your app, or a backup code, to confirm.</p>
						<label class="field">
							Code
							<input class="inline-input" type="text" bind:value={totpCodeInput} placeholder="123456" />
						</label>
						{#if totpError}<p class="error-text">{totpError}</p>{/if}
						<div class="row-actions" style="margin-top: 12px;">
							<button class="ghost" onclick={() => (totpStage = "enabled")}>Cancel</button>
							<button class="primary danger-fill" onclick={confirmTotpDisable} disabled={totpBusy || !totpCodeInput.trim()}>
								{totpBusy ? "Disabling…" : "Disable"}
							</button>
						</div>
					{:else if totpStage === "regenerating"}
						<p class="row-label">Regenerate backup codes</p>
						<p class="hint" style="margin-bottom: 12px;">This invalidates your old backup codes. Confirm with a current code.</p>
						<label class="field">
							Code
							<input class="inline-input" type="text" bind:value={totpCodeInput} placeholder="123456" />
						</label>
						{#if totpError}<p class="error-text">{totpError}</p>{/if}
						<div class="row-actions" style="margin-top: 12px;">
							<button class="ghost" onclick={() => (totpStage = "enabled")}>Cancel</button>
							<button class="primary" onclick={confirmRegenerateBackupCodes} disabled={totpBusy || !totpCodeInput.trim()}>
								{totpBusy ? "Generating…" : "Regenerate"}
							</button>
						</div>
					{:else}
						<div class="row">
							<div>
								<p class="row-label">Two-factor authentication</p>
								<p class="row-value muted">Enabled — your login also asks for a code from your authenticator app.</p>
							</div>
						</div>
						<div class="row-actions">
							<button class="ghost" onclick={() => { totpCodeInput = ""; totpError = ""; totpStage = "regenerating"; }}>
								New Backup Codes
							</button>
							<button class="edit danger-text" onclick={() => { totpCodeInput = ""; totpError = ""; totpStage = "disabling"; }}>
								Disable
							</button>
						</div>
					{/if}
				</div>

				<div class="card">
					{#if deviceLink.phase === "idle" || deviceLink.phase === "error"}
						<div class="row">
							<div>
								<p class="row-label">Linked devices</p>
								<p class="row-value muted">Move your encryption keys to a new device without losing your conversations.</p>
							</div>
							<button class="edit" onclick={beginDeviceLink}>
								<Smartphone size={14} strokeWidth={2} />
								Link a Device
							</button>
						</div>
						{#if deviceLink.phase === "error" && deviceLink.error}
							<p class="error-text" style="margin-top: 8px;">{deviceLink.error}</p>
						{/if}
					{:else if deviceLink.phase === "connecting" || deviceLink.phase === "waiting-for-peer"}
						<p class="row-label">Waiting for the new device…</p>
						<p class="hint" style="margin-bottom: 12px;">
							On the new device, sign in with this account, then choose "Link with another device" when asked.
						</p>
						<div class="row-actions">
							<button class="ghost" onclick={cancelDeviceLink}>Cancel</button>
						</div>
					{:else if deviceLink.phase === "confirm"}
						<p class="row-label">Confirm this code matches on both devices</p>
						<p class="row-value totp-secret">{deviceLink.fingerprint}</p>
						<div class="row-actions" style="margin-top: 12px;">
							<button class="ghost" onclick={cancelDeviceLink}>Cancel</button>
							<button class="primary" onclick={confirmDeviceLink}>Codes Match — Send Keys</button>
						</div>
					{:else if deviceLink.phase === "sending"}
						<p class="row-label">Sending your encryption keys…</p>
					{:else if deviceLink.phase === "done"}
						<div class="row">
							<div>
								<p class="row-label">Linked devices</p>
								<p class="row-value muted">Keys sent. The new device can now read your conversations.</p>
							</div>
							<button class="edit" onclick={() => deviceLink.reset()}>
								<Smartphone size={14} strokeWidth={2} />
								Link Another
							</button>
						</div>
					{/if}
				</div>
			{:else if section === "privacy"}
				<h2>Privacy &amp; Safety</h2>

				<div class="card">
					<div class="switch-row">
						<div>
							<p class="row-label">Show activity status</p>
							<p class="row-value muted">
								Let friends see what you're playing, watching, or listening to via Rich
								Presence. Off clears it for everyone but you.
							</p>
						</div>
						<label class="switch">
							<input
								type="checkbox"
								checked={profileStore.forUser(username)?.share_activity ?? true}
								onchange={(e) => toggleShareActivity((e.target as HTMLInputElement).checked)}
							/>
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
				</div>

				<div class="card">
					<div class="row">
						<div>
							<p class="row-label">Data collected</p>
							<p class="row-value muted">Username and a password hash. Nothing else.</p>
						</div>
					</div>
					<div class="row">
						<div>
							<p class="row-label">Direct message storage</p>
							<p class="row-value muted">
								End-to-end encrypted (X3DH + Double Ratchet). The server only ever sees ciphertext.
							</p>
						</div>
					</div>
					<div class="row">
						<div>
							<p class="row-label">Server channel storage</p>
							<p class="row-value muted">
								Message text and file attachments are end-to-end encrypted with a per-channel
								sender key, shared directly between members — the server only ever sees
								ciphertext. A member removed from the server can still read messages sent with a
								key they already received until the channel is next re-keyed.
							</p>
						</div>
					</div>
					<div class="row">
						<div>
							<p class="row-label">IP logging</p>
							<p class="row-value muted">
								None. HollowChat doesn't record your IP address or device info against your
								account, even for active sessions — a server compromise can't link your
								account to a network or a device.
							</p>
						</div>
					</div>
				</div>

				<div class="card">
					<p class="row-label" style="margin-bottom: 12px;">Blocked users</p>
					{#if blocked.length === 0}
						<p class="row-value muted">You haven't blocked anyone.</p>
					{:else}
						{#each blocked as b (b.id)}
							<div class="row">
								<p class="row-value">{b.username}</p>
								<button class="edit" onclick={() => unblock(b.id)}>Unblock</button>
							</div>
						{/each}
					{/if}
				</div>
			{:else if section === "notifications"}
				<h2>Notifications</h2>

				<div class="card">
					<div class="switch-row">
						<div>
							<p class="row-label">Messages</p>
							<p class="row-value muted">Notify when someone sends you a message.</p>
						</div>
						<label class="switch">
							<input type="checkbox" bind:checked={notifyMessages} onchange={() => toggle((v) => (notifyMessages = v), !notifyMessages, "Message notifications")} />
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
					<div class="switch-row">
						<div>
							<p class="row-label">Mentions</p>
							<p class="row-value muted">Notify when someone @mentions you.</p>
						</div>
						<label class="switch">
							<input
							type="checkbox"
							checked={notificationSettings.mentionsEnabled}
							onchange={(e) => {
								const value = (e.target as HTMLInputElement).checked;
								notificationSettings.setMentionsEnabled(value);
								toast.push(`Mention notifications ${value ? "enabled" : "disabled"}`);
							}}
						/>
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
					<div class="switch-row">
						<div>
							<p class="row-label">Notification sounds</p>
							<p class="row-value muted">Play a sound for incoming notifications.</p>
						</div>
						<label class="switch">
							<input type="checkbox" bind:checked={notifySounds} onchange={() => toggle((v) => (notifySounds = v), !notifySounds, "Notification sounds")} />
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
				</div>
			{:else if section === "appearance"}
				<h2>Appearance</h2>

				<div class="card">
					<div class="switch-row">
						<div>
							<p class="row-label">Compact mode</p>
							<p class="row-value muted">Reduce spacing between messages.</p>
						</div>
						<label class="switch">
							<input type="checkbox" bind:checked={compactMode} onchange={() => toggle((v) => (compactMode = v), !compactMode, "Compact mode")} />
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
				</div>

				<div class="card">
					<div class="row">
						<div>
							<p class="row-label">Theme</p>
							<p class="row-value muted">Hollow Theme lets you recolor every surface of the app.</p>
						</div>
					</div>
					<div class="theme-options">
						<button
							class="theme-option"
							class:active={themeStore.settings.mode === "default"}
							onclick={() => themeStore.setMode("default")}
						>
							Default
						</button>
						<button
							class="theme-option"
							class:active={themeStore.settings.mode === "custom"}
							onclick={() => themeStore.setMode("custom")}
						>
							Hollow Theme
						</button>
					</div>
				</div>

				{#if themeStore.settings.mode === "custom"}
					{#each COLOR_GROUPS as group (group.label)}
						<div class="card">
							<p class="row-label" style="margin-bottom: 8px;">{group.label}</p>
							{#each group.keys as key (key)}
								<div class="row">
									<p class="row-value">{COLOR_LABELS[key]}</p>
									<ColorPicker
										value={themeStore.settings.colors[key]}
										onCommit={(hex) => themeStore.setColor(key, hex)}
									/>
								</div>
							{/each}
						</div>
					{/each}
					<button class="theme-option" onclick={() => themeStore.resetColors()}>
						Reset colors to default
					</button>
				{/if}
			{:else if section === "accessibility"}
				<h2>Accessibility</h2>

				<div class="card">
					<div class="switch-row">
						<div>
							<p class="row-label">Reduce motion</p>
							<p class="row-value muted">Minimize animations and transitions across the app.</p>
						</div>
						<label class="switch">
							<input type="checkbox" bind:checked={reducedMotion} onchange={() => toggle((v) => (reducedMotion = v), !reducedMotion, "Reduced motion")} />
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
				</div>
			{:else if section === "sessions"}
				<h2>Devices</h2>
				<p class="hint" style="margin-bottom: 16px;">Sessions currently signed in to your account.</p>

				<div class="card">
					{#if sessions.length === 0}
						<p class="row-value muted">No active sessions.</p>
					{/if}
					{#each sessions as s (s.id)}
						<div class="row">
							<div>
								<p class="row-label">{s.current ? "This device" : "Other device"}</p>
								<p class="row-value muted">{describeSession(s)}</p>
							</div>
							{#if !s.current}
								<button class="edit danger-text" onclick={() => revoke(s.id)}>Revoke</button>
							{/if}
						</div>
					{/each}
				</div>
			{:else if section === "billing"}
				<h2>Billing</h2>

				<div class="card plan-card" class:premium={billing?.tier === "premium"}>
					<div class="plan-header">
						{#if billing?.tier === "premium"}
							<Sparkles size={18} strokeWidth={2} />
						{:else}
							<CreditCard size={18} strokeWidth={2} />
						{/if}
						<p class="row-label">{billing?.tier === "premium" ? "Premium" : "Free"} plan</p>
					</div>
					<p class="row-value muted">
						{#if billing?.tier === "premium"}
							File uploads up to 2GB. Thanks for supporting HollowChat.
						{:else}
							File uploads up to 50MB. Upgrade for 2GB uploads.
						{/if}
					</p>
					{#if billing?.subscription_status && billing.subscription_status !== "active"}
						<p class="row-value muted">Subscription status: {billing.subscription_status}</p>
					{/if}
				</div>

				{#if billing?.tier !== "premium"}
					<div class="card">
						<p class="row-label">Upgrade to Premium</p>
						<p class="row-value muted" style="margin-bottom: 12px;">
							2GB uploads, up to 8 linked devices, a Supporter badge, and 2 Void Shards to boost
							servers with.
						</p>
						<button class="edit" onclick={upgrade} disabled={checkoutLoading}>
							{checkoutLoading ? "Opening checkout…" : "Upgrade"}
						</button>
					</div>
				{:else}
					<div class="card">
						<p class="row-label">Void Shards</p>
						<p class="row-value muted" style="margin-bottom: 12px;">
							{billing.boost_slots_used} of {billing.boost_slots_total} assigned. Boosting a server
							raises its custom emoji slots for everyone in it.
						</p>
						{#if myServers.length === 0}
							<p class="row-value muted">You're not in any servers yet.</p>
						{:else}
							<div class="boost-list">
								{#each myServers as server (server.id)}
									{@const boosted = boostedServerIds.has(server.id)}
									{@const outOfSlots = !boosted && billing.boost_slots_used >= billing.boost_slots_total}
									<div class="boost-row">
										<span class="boost-name">{server.name}</span>
										{#if server.boost_count > 0}
											<span class="boost-count">
												<Sparkles size={12} strokeWidth={2.25} />
												{server.boost_count}
											</span>
										{/if}
										<button
											type="button"
											class="boost-toggle"
											class:active={boosted}
											disabled={boostBusyId === server.id || outOfSlots}
											onclick={() => toggleBoost(server.id)}
										>
											{boosted ? "Boosted" : "Boost"}
										</button>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{/if}
			{:else if section === "moderation" && isStaff}
				<h2>Moderation</h2>
				<div class="card">
					<p class="row-value muted" style="margin-bottom: 12px;">
						Report contents are sealed to a key this app never holds — decrypt them offline with
						<code>tools/decrypt-report.mjs</code>. This just tracks who reported whom and lets you
						open a reply DM as Hollow Support.
					</p>
					<div class="report-filter">
						{#each REPORT_FILTERS as filter (filter)}
							<button
								type="button"
								class="filter-chip"
								class:active={reportStatusFilter === filter}
								onclick={() => (reportStatusFilter = filter)}
							>
								{filter}
							</button>
						{/each}
					</div>
				</div>
				{#if reportsLoading}
					<p class="row-value muted">Loading…</p>
				{:else if visibleReports.length === 0}
					<p class="row-value muted">No reports here.</p>
				{:else}
					<div class="report-list">
						{#each visibleReports as report (report.id)}
							<div class="report-row">
								<div class="report-main">
									<p class="report-line">
										<strong>{report.reporter_username}</strong> reported
										<strong>{report.reported_username}</strong>
									</p>
									<p class="report-meta">
										{report.context_kind} · {new Date(report.created_at).toLocaleString()}
									</p>
								</div>
								<select
									value={report.status}
									onchange={(e) => setReportStatus(report, e.currentTarget.value as api.ReportStatus)}
								>
									<option value="open">Open</option>
									<option value="reviewing">Reviewing</option>
									<option value="resolved">Resolved</option>
									<option value="dismissed">Dismissed</option>
								</select>
								<button
									type="button"
									class="save"
									onclick={() => messageReporter(report.reporter_username)}
								>
									<MessageSquare size={14} strokeWidth={2} />
									Message
								</button>
							</div>
						{/each}
					</div>
				{/if}
			{/if}
		</div>
	</div>
</div>

<style>
	.overlay {
		position: absolute;
		inset: 0;
		background: rgba(0, 0, 0, 0.55);
		backdrop-filter: blur(3px);
		-webkit-backdrop-filter: blur(3px);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 40px;
		z-index: 100;
	}

	.modal {
		position: relative;
		display: flex;
		width: min(960px, 100%);
		height: min(640px, 100%);
		background: var(--panel);
		border-radius: 12px;
		overflow: hidden;
		box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
	}

	.nav {
		width: 220px;
		flex-shrink: 0;
		background: var(--sidebar);
		padding: 16px 12px;
		display: flex;
		flex-direction: column;
		overflow-y: auto;
	}

	.nav-search {
		position: relative;
		margin: 4px 4px 12px;
	}

	.nav-search :global(svg) {
		position: absolute;
		left: 9px;
		top: 50%;
		translate: 0 -50%;
		color: var(--ink-faint);
		pointer-events: none;
	}

	.nav-search input {
		width: 100%;
		padding: 7px 8px 7px 28px;
		border-radius: 6px;
		background: var(--panel);
		border: 1px solid var(--hairline);
		color: var(--ink);
		font-size: 12px;
	}

	.nav-search input:focus {
		outline: none;
		border-color: var(--ink-faint);
	}

	.nav-subitem {
		display: block;
		width: 100%;
		text-align: left;
		padding: 6px 8px 6px 24px;
		margin-left: 8px;
		border-left: 1px solid var(--hairline);
		font-size: 13px;
		font-weight: 500;
		color: var(--ink-faint);
	}

	.nav-subitem:hover {
		color: var(--ink-dim);
	}

	.nav-subitem.active {
		color: var(--ink);
		border-left: 1px solid var(--ink);
	}

	.nav-label {
		margin: 0 8px 8px;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--ink-faint);
	}

	.nav-item {
		position: relative;
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 8px 8px 12px;
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

	.nav-item.active::before {
		content: "";
		position: absolute;
		left: 0;
		top: 50%;
		translate: 0 -50%;
		width: 3px;
		height: 18px;
		border-radius: 0 3px 3px 0;
		background: var(--accent-fill);
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

	.password-box {
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 14px 12px 14px 16px;
	}

	.password-box code {
		flex: 1;
		font-family: var(--font-mono);
		font-size: 15px;
		font-weight: 500;
		letter-spacing: 0.02em;
		word-break: break-all;
		color: var(--ink);
	}

	.password-box .copy {
		flex-shrink: 0;
		display: flex;
		padding: 8px;
		border-radius: 6px;
		color: var(--ink-dim);
	}

	.password-box .copy:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.identity {
		display: flex;
		align-items: center;
		gap: 14px;
		margin-bottom: 20px;
	}

	.avatar {
		width: 48px;
		height: 48px;
		border-radius: 50%;
		background: var(--accent-fill) center/cover;
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 15px;
		flex-shrink: 0;
	}

	.username {
		margin: 0;
		font-family: var(--font-mono);
		font-weight: 600;
		font-size: 16px;
		color: var(--ink);
	}

	.hint {
		margin: 2px 0 0;
		font-size: 12px;
		color: var(--ink-faint);
	}

	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		padding: 12px 0;
		border-top: 1px solid var(--hairline);
	}

	.row:first-of-type {
		border-top: none;
		padding-top: 0;
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

	.row-actions {
		display: flex;
		gap: 8px;
		flex-shrink: 0;
	}

	.theme-options {
		display: flex;
		gap: 8px;
		padding-top: 4px;
	}

	.theme-option {
		padding: 8px 14px;
		border-radius: 6px;
		border: 1px solid var(--hairline);
		font-size: 13px;
		font-weight: 600;
		color: var(--ink-dim);
		transition: background-color 0.15s ease, color 0.15s ease, border-color 0.15s ease;
	}

	.theme-option:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.theme-option.active {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		border-color: var(--accent-fill);
	}

	.inline-input {
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 8px 10px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 13px;
		min-width: 220px;
	}

	.inline-input:focus {
		outline: none;
		border-color: var(--ink-dim);
	}

	.edit {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 8px 14px;
		border-radius: 6px;
		background: var(--active);
		color: var(--ink);
		font-weight: 600;
		font-size: 12px;
	}

	.edit:hover {
		background: var(--hover);
	}

	.edit:disabled {
		color: var(--ink-faint);
		cursor: default;
	}

	.edit.danger-text {
		color: var(--danger);
	}

	.plan-card {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.plan-card.premium {
		background: var(--accent-soft);
	}

	.plan-header {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.plan-header :global(svg) {
		color: var(--ink-dim);
	}

	.plan-card.premium .plan-header :global(svg) {
		color: var(--online);
	}

	.plan-header .row-label {
		margin: 0;
	}

	.boost-list {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.boost-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 10px;
		border-radius: 8px;
		background: var(--active);
	}

	.boost-name {
		flex: 1;
		min-width: 0;
		font-size: 13px;
		font-weight: 600;
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.boost-count {
		display: flex;
		align-items: center;
		gap: 3px;
		font-size: 12px;
		font-weight: 700;
		color: var(--online);
	}

	.boost-toggle {
		flex-shrink: 0;
		padding: 5px 12px;
		border-radius: 6px;
		background: var(--hover);
		color: var(--ink-dim);
		font-size: 12px;
		font-weight: 700;
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.boost-toggle:hover:not(:disabled) {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.boost-toggle.active {
		background: var(--online);
		color: var(--void);
	}

	.boost-toggle:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.report-filter {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.filter-chip {
		padding: 5px 12px;
		border-radius: 999px;
		background: var(--active);
		color: var(--ink-dim);
		font-size: 12px;
		font-weight: 600;
		text-transform: capitalize;
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.filter-chip:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.filter-chip.active {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.report-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.report-row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 12px;
		border-radius: 8px;
		background: var(--sidebar);
	}

	.report-main {
		flex: 1;
		min-width: 0;
	}

	.report-line {
		margin: 0;
		font-size: 13px;
		color: var(--ink);
	}

	.report-meta {
		margin: 2px 0 0;
		font-size: 11px;
		color: var(--ink-faint);
		text-transform: capitalize;
	}

	.report-row select {
		flex-shrink: 0;
	}

	.report-row .save {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 5px;
	}

	.ghost {
		padding: 8px 14px;
		border-radius: 6px;
		color: var(--ink-dim);
		font-weight: 600;
		font-size: 12px;
	}

	.ghost:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.primary {
		padding: 8px 14px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-weight: 700;
		font-size: 12px;
	}

	.primary:disabled {
		background: var(--active);
		color: var(--ink-faint);
	}

	.primary.danger-fill {
		background: var(--danger);
		color: white;
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
	}

	.error-text {
		margin: 8px 0 0;
		font-size: 12px;
		color: var(--danger);
	}

	.totp-qr {
		display: block;
		width: 160px;
		height: 160px;
		border-radius: 8px;
		background: white;
		padding: 8px;
	}

	.totp-secret {
		font-family: var(--font-mono);
		letter-spacing: 0.06em;
		word-break: break-all;
	}

	.backup-codes {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}

	.backup-codes code {
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 8px 10px;
		text-align: center;
		font-family: var(--font-mono);
		font-size: 13px;
		color: var(--ink);
	}

	.switch-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		padding: 12px 0;
		border-top: 1px solid var(--hairline);
	}

	.switch-row:first-of-type {
		border-top: none;
		padding-top: 0;
	}

	.switch {
		position: relative;
		flex-shrink: 0;
		width: 40px;
		height: 22px;
	}

	.switch input {
		position: absolute;
		opacity: 0;
		width: 100%;
		height: 100%;
		margin: 0;
		cursor: pointer;
	}

	.track {
		display: block;
		width: 100%;
		height: 100%;
		border-radius: 999px;
		background: var(--active);
		transition: background-color 0.15s ease;
	}

	.thumb {
		position: absolute;
		top: 3px;
		left: 3px;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: var(--ink-faint);
		transition: transform 0.15s ease, background-color 0.15s ease;
	}

	.switch input:checked + .track {
		background: var(--accent-soft);
	}

	.switch input:checked + .track .thumb {
		transform: translateX(18px);
		background: var(--ink);
	}

	.nav-identity {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 6px;
		margin-bottom: 16px;
		border-radius: 8px;
		text-align: left;
		transition: background-color 0.15s ease;
	}

	.nav-identity:hover {
		background: var(--hover);
	}

	.nav-avatar {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: var(--accent-fill) center/cover;
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 12px;
		flex-shrink: 0;
	}

	.nav-identity-text {
		min-width: 0;
	}

	.nav-identity-name {
		margin: 0;
		font-size: 13px;
		font-weight: 700;
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.nav-identity-edit {
		margin: 1px 0 0;
		display: flex;
		align-items: center;
		gap: 4px;
		font-size: 11px;
		color: var(--ink-faint);
	}

	.card.no-pad {
		padding: 0;
		overflow: hidden;
	}

	.preview-banner {
		height: 80px;
		background: var(--accent-soft);
	}

	.preview-body {
		padding: 0 20px 20px;
	}

	.preview-avatar-row {
		display: flex;
		align-items: flex-end;
		gap: 12px;
		margin-top: -28px;
	}

	.preview-avatar {
		width: 68px;
		height: 68px;
		border-radius: 50%;
		border: 4px solid var(--sidebar);
		background: var(--accent-fill) center/cover;
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 20px;
		flex-shrink: 0;
	}

	.preview-image-actions {
		display: flex;
		gap: 6px;
		margin-bottom: 6px;
		flex-wrap: wrap;
	}

	.ghost.small {
		padding: 6px 10px;
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: 11px;
		background: var(--panel);
	}

	.ghost.danger-text {
		color: var(--danger);
	}

	.preview-name {
		margin: 12px 0 0;
		display: flex;
		align-items: center;
		gap: 6px;
		font-family: var(--font-mono);
		font-weight: 700;
		font-size: 16px;
		color: var(--ink);
	}

	.preview-pronouns {
		font-size: 12px;
		font-weight: 500;
		color: var(--ink-faint);
	}

	.preview-handle {
		margin: 1px 0 0;
		font-size: 12px;
		color: var(--ink-faint);
	}

	.preview-status {
		margin: 4px 0 0;
		font-size: 13px;
		color: var(--ink-dim);
	}

	.inline-textarea {
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 10px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 13px;
		resize: vertical;
		min-height: 60px;
	}

	.inline-textarea:focus {
		outline: none;
		border-color: var(--ink-dim);
	}
</style>
