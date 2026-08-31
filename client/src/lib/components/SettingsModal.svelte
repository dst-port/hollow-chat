<script lang="ts">
	import { fade, scale, fly } from "svelte/transition";
	import { cubicOut } from "svelte/easing";
	import X from "@lucide/svelte/icons/x";
	import Search from "@lucide/svelte/icons/search";
	import UserRound from "@lucide/svelte/icons/user-round";
	import UserPen from "@lucide/svelte/icons/user-pen";
	import ShieldCheck from "@lucide/svelte/icons/shield-check";
	import Bell from "@lucide/svelte/icons/bell";
	import Mic from "@lucide/svelte/icons/mic";
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
	import ArrowLeft from "@lucide/svelte/icons/arrow-left";
	import { openUrl } from "@tauri-apps/plugin-opener";
	import QRCode from "qrcode";
	import { renameLocalIdentity } from "$lib/crypto/identity";
	import { renameAllSessions } from "$lib/crypto/session-store";
	import { renameAllGroupKeys } from "$lib/crypto/group-key-store";
	import { toast } from "$lib/stores/toast.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { deviceLink } from "$lib/devicelink/link.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import { isVideoMedia } from "$lib/utils/media";
	import { badgeStore } from "$lib/stores/badges.svelte";
	import { themeStore, COLOR_GROUPS, COLOR_LABELS, THEME_PRESETS } from "$lib/stores/theme.svelte";
	import { fontStore, FONT_STACKS, FONT_LABELS, PRESET_FONT_IDS, type FontId } from "$lib/stores/font.svelte";
	import { notificationSettings } from "$lib/stores/notifications.svelte";
	import { disablePush, enablePush, pushEnabledLocally, pushSupported } from "$lib/push/push";
	import { pendingDm } from "$lib/stores/pendingDm.svelte";
	import { t, i18n, LOCALES, type LocaleCode } from "$lib/i18n/index.svelte";
	import Badges from "$lib/components/Badges.svelte";
	import ColorPicker from "$lib/components/ColorPicker.svelte";
	import Dropdown from "$lib/components/Dropdown.svelte";
	import VoiceSettingsPanel from "$lib/components/VoiceSettingsPanel.svelte";
	import * as api from "$lib/api/client";
	import { openReport, type ReportPayload } from "$lib/crypto/moderation";
	import { moderationKey } from "$lib/stores/moderationKey.svelte";

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
		| "voice"
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
	let myBoostCounts = $state<Record<string, number>>({});
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
				return Promise.all(
					servers.map((s) => api.getBoosts(token, s.id).then((b) => [s.id, b.my_boost_count] as const))
				);
			})
			.then((pairs) => {
				myBoostCounts = Object.fromEntries(pairs);
			})
			.catch(() => {});
	});

	let reports = $state<api.ReportSummary[]>([]);
	let reportsLoading = $state(false);
	let reportStatusFilter = $state<api.ReportStatus | "all">("open");
	let openReportId = $state<string | null>(null);

	$effect(() => {
		const token = session.token;
		if (!token || section !== "moderation" || !isStaff) return;
		reportsLoading = true;
		api
			.listReports(token)
			.then((rows) => (reports = rows))
			.catch(() => toast.push(t("settings.moderation.loadFailed")))
			.finally(() => (reportsLoading = false));
	});

	const visibleReports = $derived(
		reportStatusFilter === "all" ? reports : reports.filter((r) => r.status === reportStatusFilter)
	);

	let decryptedReport = $state<ReportPayload | null>(null);
	let decrypting = $state(false);
	let decryptError = $state<string | null>(null);
	let showKeyInput = $state(false);

	function openReportDetail(id: string) {
		openReportId = id;
		decryptedReport = null;
		decryptError = null;
		showKeyInput = false;
	}

	async function decryptReport(id: string) {
		const token = session.token;
		if (!token || !moderationKey.present) {
			showKeyInput = true;
			return;
		}
		decrypting = true;
		decryptError = null;
		try {
			const sealed = await api.getReportSealed(token, id);
			decryptedReport = await openReport(sealed, moderationKey.value);
			showKeyInput = false;
		} catch {
			decryptError = t("settings.moderation.decryptFailed");
		} finally {
			decrypting = false;
		}
	}

	async function setReportStatus(report: api.ReportSummary, status: api.ReportStatus) {
		const token = session.token;
		if (!token) return;
		const previous = report.status;
		report.status = status;
		try {
			await api.updateReportStatus(token, report.id, status);
		} catch {
			report.status = previous;
			toast.push(t("settings.moderation.updateFailed"));
		}
	}

	function messageReporter(username: string) {
		pendingDm.request(username);
		onClose();
	}

	function copyReportId(id: string) {
		navigator.clipboard.writeText(id);
		toast.push(t("settings.moderation.reportIdCopied"));
	}

	function copyFetchCommand(id: string) {
		const cmd =
			`psql "$DATABASE_URL" -c "SELECT ` +
			`encode(sealed_key_ephemeral_public, 'base64') AS sealed_key_ephemeral_public, ` +
			`encode(sealed_key_nonce, 'base64') AS sealed_key_nonce, ` +
			`encode(sealed_key_ciphertext, 'base64') AS sealed_key_ciphertext, ` +
			`encode(payload_nonce, 'base64') AS payload_nonce, ` +
			`encode(payload_ciphertext, 'base64') AS payload_ciphertext ` +
			`FROM reports WHERE id = '${id}'\\gx"`;
		navigator.clipboard.writeText(cmd);
		toast.push(t("settings.moderation.psqlCopied"));
	}

	const REPORT_FILTERS: (api.ReportStatus | "all")[] = ["open", "reviewing", "resolved", "dismissed", "all"];
	const REPORT_STATUSES_LIST: api.ReportStatus[] = ["open", "reviewing", "resolved", "dismissed"];

	async function changeBoost(serverId: string, delta: 1 | -1) {
		const token = session.token;
		if (!token || boostBusyId) return;
		boostBusyId = serverId;
		try {
			const status =
				delta === 1
					? await api.addBoost(token, serverId)
					: await api.removeBoost(token, serverId);
			myBoostCounts = { ...myBoostCounts, [serverId]: status.my_boost_count };
			if (billing) {
				billing = { ...billing, boost_slots_used: billing.boost_slots_used + delta };
			}
		} catch (err) {
			if (err instanceof api.ApiError && err.status === 409) {
				toast.push(t("settings.billing.allShardsAssigned"));
			} else if (err instanceof api.ApiError && err.status === 403) {
				toast.push(t("settings.billing.shardsPremiumOnly"));
			} else {
				toast.push(t("settings.billing.boostFailed"));
			}
		} finally {
			boostBusyId = null;
		}
	}

	async function revealExtensionFolder() {
		try {
			const { resolveResource } = await import("@tauri-apps/api/path");
			const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
			const dir = await resolveResource("extension");
			await revealItemInDir(dir);
		} catch {
			toast.push(t("settings.privacy.extensionFolderFailed"));
		}
	}

	// Free vs HollowChatter comparison. `free`/`pro` are either a short value
	// string (i18n key) or true for a plain checkmark / false for a dash.
	const PLAN_ROWS: { label: string; free: string | boolean; pro: string | boolean }[] = [
		{ label: "settings.billing.cmp.uploads", free: "settings.billing.cmp.uploads.free", pro: "settings.billing.cmp.uploads.pro" },
		{ label: "settings.billing.cmp.devices", free: "3", pro: "8" },
		{ label: "settings.billing.cmp.boosts", free: false, pro: "settings.billing.cmp.boosts.pro" },
		{ label: "settings.billing.cmp.vanity", free: false, pro: true },
		{ label: "settings.billing.cmp.nameFont", free: false, pro: true },
		{ label: "settings.billing.cmp.badge", free: false, pro: true },
		{ label: "settings.billing.cmp.themeColors", free: true, pro: true },
		{ label: "settings.billing.cmp.core", free: true, pro: true }
	];

	async function upgrade() {
		const token = session.token;
		if (!token) return;
		checkoutLoading = true;
		try {
			const { url } = await api.createCheckout(token);
			await openUrl(url);
		} catch (err) {
			if (err instanceof api.ApiError && err.status === 503) {
				toast.push(t("settings.billing.notConfigured"));
			} else {
				toast.push(t("settings.billing.checkoutFailed"));
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

	const pushIsSupported = pushSupported();
	let pushBusy = $state(false);
	let pushOn = $state(pushEnabledLocally());

	async function togglePush(next: boolean) {
		const token = session.token;
		if (!token || pushBusy) return;
		pushBusy = true;
		try {
			if (next) {
				const result = await enablePush(token);
				pushOn = result === "enabled";
				if (result === "enabled") {
					toast.push(t("toast.toggledOn", { label: t("settings.notifications.push") }));
				} else if (result === "denied") {
					toast.push(t("settings.notifications.pushDenied"));
				} else {
					toast.push(t("settings.notifications.pushFailed"));
				}
			} else {
				await disablePush(token);
				pushOn = false;
				toast.push(t("toast.toggledOff", { label: t("settings.notifications.push") }));
			}
		} finally {
			pushBusy = false;
		}
	}

	let reducedMotion = $state(false);
	let compactMode = $state(false);

	let customFontFamilyDraft = $state(fontStore.settings.customFamily);
	let customFontUrlDraft = $state(fontStore.settings.customUrl);

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
			toast.push(t("settings.account.setupFailed"));
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
			toast.push(t("settings.account.twoFactorEnabledToast"));
		} catch {
			totpError = t("settings.account.badCode");
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
			toast.push(t("settings.account.twoFactorDisabledToast"));
		} catch {
			totpError = t("settings.account.badCode");
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
			totpError = t("settings.account.badCode");
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
				toast.push(t("settings.privacy.unblockedToast"));
			})
			.catch(() => toast.push(t("settings.privacy.unblockFailed")));
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
			toast.push(t("settings.privacy.activityUpdateFailed"));
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
		return t("settings.devices.signedIn", { date: when });
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
			toast.push(t("settings.account.usernameUpdated"));
		} catch (err) {
			if (err instanceof api.ApiError && err.status === 409) {
				toast.push(t("settings.account.usernameTaken"));
			} else {
				toast.push(t("settings.account.usernameChangeFailed"));
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
			toast.push(t("settings.account.passwordGenFailed"));
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
				toast.push(t("settings.devices.revokedToast"));
			})
			.catch(() => toast.push(t("settings.devices.revokeFailed")));
	}

	function toggle(setter: (v: boolean) => void, current: boolean, label: string) {
		setter(!current);
		toast.push(t(!current ? "toast.toggledOn" : "toast.toggledOff", { label }));
	}

	function changeLanguage(code: LocaleCode) {
		i18n.set(code);
		const label = LOCALES.find((l) => l.code === code)?.label ?? code;
		toast.push(t("settings.language.changed", { lang: label }));
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="overlay" role="presentation" onclick={onClose} transition:fade={{ duration: 150 }}>
	<div
		class="modal"
		role="dialog"
		aria-modal="true"
		aria-label={t("settings.title")}
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
					<p class="nav-identity-edit"><Pencil size={10} strokeWidth={2.5} />{t("settings.editProfile")}</p>
				</div>
			</button>

			<label class="nav-search">
				<Search size={13} strokeWidth={2} />
				<input type="text" placeholder={t("common.search")} bind:value={navSearch} />
			</label>

			{#if matchesSearch(t("settings.nav.profile"))}
				<button class="nav-item" class:active={section === "profile"} onclick={() => (section = "profile")}>
					<UserPen size={16} strokeWidth={2} />
					{t("settings.nav.profile")}
				</button>
			{/if}
			{#if matchesSearch(t("settings.nav.account")) || matchesSearch(t("settings.nav.accountInfo")) || matchesSearch(t("settings.nav.passwordSecurity"))}
				<button class="nav-item" class:active={section === "account"} onclick={() => (section = "account")}>
					<UserRound size={16} strokeWidth={2} />
					{t("settings.nav.account")}
				</button>
				{#if section === "account"}
					<button class="nav-subitem" onclick={() => goToAccountField("account-info")}>{t("settings.nav.accountInfo")}</button>
					<button class="nav-subitem" onclick={() => goToAccountField("account-security")}>{t("settings.nav.passwordSecurity")}</button>
				{/if}
			{/if}
			{#if matchesSearch(t("settings.nav.privacy"))}
				<button class="nav-item" class:active={section === "privacy"} onclick={() => (section = "privacy")}>
					<ShieldCheck size={16} strokeWidth={2} />
					{t("settings.nav.privacy")}
				</button>
			{/if}
			{#if matchesSearch(t("settings.nav.notifications"))}
				<button class="nav-item" class:active={section === "notifications"} onclick={() => (section = "notifications")}>
					<Bell size={16} strokeWidth={2} />
					{t("settings.nav.notifications")}
				</button>
			{/if}
			{#if matchesSearch(t("settings.nav.voice"))}
				<button class="nav-item" class:active={section === "voice"} onclick={() => (section = "voice")}>
					<Mic size={16} strokeWidth={2} />
					{t("settings.nav.voice")}
				</button>
			{/if}
			{#if matchesSearch(t("settings.nav.devices"))}
				<button class="nav-item" class:active={section === "sessions"} onclick={() => (section = "sessions")}>
					<Monitor size={16} strokeWidth={2} />
					{t("settings.nav.devices")}
				</button>
			{/if}

			{#if matchesSearch(t("settings.nav.appearance")) || matchesSearch(t("settings.nav.accessibility"))}
				<p class="nav-label">{t("settings.group.experience")}</p>
				{#if matchesSearch(t("settings.nav.appearance"))}
					<button class="nav-item" class:active={section === "appearance"} onclick={() => (section = "appearance")}>
						<Palette size={16} strokeWidth={2} />
						{t("settings.nav.appearance")}
					</button>
				{/if}
				{#if matchesSearch(t("settings.nav.accessibility"))}
					<button class="nav-item" class:active={section === "accessibility"} onclick={() => (section = "accessibility")}>
						<Accessibility size={16} strokeWidth={2} />
						{t("settings.nav.accessibility")}
					</button>
				{/if}
			{/if}

			{#if matchesSearch(t("settings.nav.billing"))}
				<p class="nav-label">{t("settings.group.billing")}</p>
				<button class="nav-item" class:active={section === "billing"} onclick={() => (section = "billing")}>
					<CreditCard size={16} strokeWidth={2} />
					{t("settings.nav.billing")}
				</button>
			{/if}

			{#if isStaff && matchesSearch(t("settings.nav.moderation"))}
				<p class="nav-label">{t("settings.group.staff")}</p>
				<button class="nav-item" class:active={section === "moderation"} onclick={() => (section = "moderation")}>
					<ShieldAlert size={16} strokeWidth={2} />
					{t("settings.nav.moderation")}
				</button>
			{/if}

			<div class="nav-spacer"></div>

			<button class="nav-item danger" onclick={onLogout}>
				<LogOut size={16} strokeWidth={2} />
				{t("settings.logOut")}
			</button>
		</nav>

		<button class="close" onclick={onClose} title={t("common.close")}>
			<X size={20} strokeWidth={2} />
		</button>

		<div class="content">
			{#if section === "profile"}
				{@const ownBadges = badgeStore.forUser(username)}
				{@const ownProf = profileStore.forUser(username)}
				{@const pAvatarSrc = ownProf?.avatar_url ? api.resolveUrl(ownProf.avatar_url, session.token) : ""}
				{@const pBannerSrc = ownProf?.banner_url ? api.resolveUrl(ownProf.banner_url, session.token) : ""}
				{@const pAvatarVideo = isVideoMedia(ownProf?.avatar_url)}
				{@const pBannerVideo = isVideoMedia(ownProf?.banner_url)}
				<h2>{t("settings.nav.profile")}</h2>

				<div class="card no-pad" in:fade={{ duration: 140 }}>
					<div
						class="preview-banner"
						style:background={pBannerVideo ? "#000" : pBannerSrc ? `url(${pBannerSrc}) center/cover` : bannerColorDraft}
					>
						{#if pBannerVideo}<video class="banner-media" src={pBannerSrc} autoplay loop muted playsinline></video>{/if}
					</div>
					<div class="preview-body">
						<div class="preview-avatar-row">
							<div
								class="preview-avatar"
								style:background-image={pAvatarSrc && !pAvatarVideo ? `url(${pAvatarSrc})` : undefined}
							>
								{#if pAvatarSrc && pAvatarVideo}
									<video class="avatar-media" src={pAvatarSrc} autoplay loop muted playsinline></video>
								{:else if !pAvatarSrc}{username.slice(0, 2).toUpperCase()}{/if}
							</div>
							<div class="preview-image-actions">
								<input bind:this={avatarInput} type="file" accept="image/*,video/mp4,video/webm" hidden onchange={onAvatarChosen} />
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
							<input bind:this={bannerInput} type="file" accept="image/*,video/mp4,video/webm" hidden onchange={onBannerChosen} />
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
				<h2>{t("settings.nav.account")}</h2>

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
							<p class="hint">{t("settings.account.accountType")}</p>
						</div>
					</div>

					<div class="row">
						<div>
							<p class="row-label">{t("settings.account.username")}</p>
							{#if editingUsername}
								<input class="inline-input" type="text" bind:value={usernameDraft} maxlength="32" />
							{:else}
								<p class="row-value">{username}</p>
							{/if}
						</div>
						{#if editingUsername}
							<div class="row-actions">
								<button class="ghost" onclick={() => ((editingUsername = false), (usernameDraft = username))}>{t("common.cancel")}</button>
								<button class="primary" onclick={saveUsername} disabled={!usernameDraft.trim()}>{t("common.save")}</button>
							</div>
						{:else}
							<button class="edit" onclick={() => (editingUsername = true)}>{t("common.edit")}</button>
						{/if}
					</div>

					<div class="row">
						<div>
							<p class="row-label">{t("settings.account.email")}</p>
							<p class="row-value muted">{t("settings.account.notCollected")}</p>
						</div>
					</div>

					<div class="row">
						<div>
							<p class="row-label">{t("settings.account.phone")}</p>
							<p class="row-value muted">{t("settings.account.notCollected")}</p>
						</div>
					</div>
				</div>

				<div class="card" id="account-security">
					{#if !editingPassword}
						<div class="row">
							<div>
								<p class="row-label">{t("settings.account.password")}</p>
								<p class="row-value">••••••••••••</p>
							</div>
							<button class="edit" onclick={() => (editingPassword = true)}>{t("common.change")}</button>
						</div>
					{:else if regeneratedPassword}
						<p class="row-label">{t("settings.account.savePasswordNow")}</p>
						<p class="hint" style="margin-bottom: 12px;">
							{t("settings.account.savePasswordHint")}
						</p>
						<div class="password-box">
							<code>{regeneratedPassword}</code>
							<button type="button" class="copy" onclick={copyRegeneratedPassword} title={t("settings.account.copyPassword")}>
								{#if passwordCopied}
									<Check size={15} strokeWidth={2.5} />
								{:else}
									<Copy size={15} strokeWidth={2} />
								{/if}
							</button>
						</div>
						<div class="row-actions" style="margin-top: 12px;">
							<button class="primary" onclick={closePasswordChange}>{t("common.done")}</button>
						</div>
					{:else}
						<p class="row-label">{t("settings.account.generateNewQuestion")}</p>
						<p class="hint" style="margin-bottom: 12px;">
							{t("settings.account.generateNewHint")}
						</p>
						<div class="row-actions">
							<button class="ghost" onclick={() => (editingPassword = false)}>{t("common.cancel")}</button>
							<button class="primary" onclick={regeneratePassword} disabled={regenerating}>
								{regenerating ? t("settings.account.generating") : t("settings.account.generateNew")}
							</button>
						</div>
					{/if}

					<div class="row">
						<div>
							<p class="row-label">{t("settings.account.recovery")}</p>
							<p class="row-value muted">
								{t("settings.account.recoveryHint")}
							</p>
						</div>
					</div>
				</div>

				<div class="card">
					{#if totpStage === "idle"}
						<div class="row">
							<div>
								<p class="row-label">{t("settings.account.twoFactor")}</p>
								<p class="row-value muted">{t("settings.account.twoFactorOff")}</p>
							</div>
							<button class="edit" onclick={beginTotpSetup} disabled={totpBusy}>
								<ShieldPlus size={14} strokeWidth={2} />
								{t("settings.account.enable")}
							</button>
						</div>
					{:else if totpStage === "setting-up"}
						<p class="row-label">{t("settings.account.scanTitle")}</p>
						<p class="hint" style="margin-bottom: 12px;">
							{t("settings.account.scanHint")}
						</p>
						{#if totpQrDataUrl}
							<img class="totp-qr" src={totpQrDataUrl} alt={t("settings.account.qrAlt")} />
						{/if}
						<p class="hint" style="margin: 8px 0 4px;">{t("settings.account.manualEntry")}</p>
						<p class="row-value totp-secret">{totpSecret}</p>
						<label class="field" style="margin-top: 12px;">
							{t("settings.account.sixDigitCode")}
							<input class="inline-input" type="text" bind:value={totpCodeInput} placeholder="123456" maxlength="6" />
						</label>
						{#if totpError}<p class="error-text">{totpError}</p>{/if}
						<div class="row-actions" style="margin-top: 12px;">
							<button class="ghost" onclick={() => (totpStage = "idle")}>{t("common.cancel")}</button>
							<button class="primary" onclick={confirmTotpSetup} disabled={totpBusy || totpCodeInput.trim().length !== 6}>
								{totpBusy ? t("settings.account.verifying") : t("settings.account.verifyEnable")}
							</button>
						</div>
					{:else if totpStage === "backup-codes"}
						<p class="row-label">{t("settings.account.saveBackupCodes")}</p>
						<p class="hint" style="margin-bottom: 12px;">
							{t("settings.account.backupCodesHint")}
						</p>
						<div class="backup-codes">
							{#each totpBackupCodes as code (code)}
								<code>{code}</code>
							{/each}
						</div>
						<div class="row-actions" style="margin-top: 12px;">
							<button class="primary" onclick={finishBackupCodesReview}>{t("settings.account.savedCodes")}</button>
						</div>
					{:else if totpStage === "disabling"}
						<p class="row-label">{t("settings.account.disableTwoFactor")}</p>
						<p class="hint" style="margin-bottom: 12px;">{t("settings.account.disableHint")}</p>
						<label class="field">
							{t("settings.account.code")}
							<input class="inline-input" type="text" bind:value={totpCodeInput} placeholder="123456" />
						</label>
						{#if totpError}<p class="error-text">{totpError}</p>{/if}
						<div class="row-actions" style="margin-top: 12px;">
							<button class="ghost" onclick={() => (totpStage = "enabled")}>{t("common.cancel")}</button>
							<button class="primary danger-fill" onclick={confirmTotpDisable} disabled={totpBusy || !totpCodeInput.trim()}>
								{totpBusy ? t("settings.account.disabling") : t("settings.account.disable")}
							</button>
						</div>
					{:else if totpStage === "regenerating"}
						<p class="row-label">{t("settings.account.regenerateTitle")}</p>
						<p class="hint" style="margin-bottom: 12px;">{t("settings.account.regenerateHint")}</p>
						<label class="field">
							{t("settings.account.code")}
							<input class="inline-input" type="text" bind:value={totpCodeInput} placeholder="123456" />
						</label>
						{#if totpError}<p class="error-text">{totpError}</p>{/if}
						<div class="row-actions" style="margin-top: 12px;">
							<button class="ghost" onclick={() => (totpStage = "enabled")}>{t("common.cancel")}</button>
							<button class="primary" onclick={confirmRegenerateBackupCodes} disabled={totpBusy || !totpCodeInput.trim()}>
								{totpBusy ? t("settings.account.generating") : t("settings.account.regenerate")}
							</button>
						</div>
					{:else}
						<div class="row">
							<div>
								<p class="row-label">{t("settings.account.twoFactor")}</p>
								<p class="row-value muted">{t("settings.account.twoFactorOn")}</p>
							</div>
						</div>
						<div class="row-actions">
							<button class="ghost" onclick={() => { totpCodeInput = ""; totpError = ""; totpStage = "regenerating"; }}>
								{t("settings.account.newBackupCodes")}
							</button>
							<button class="edit danger-text" onclick={() => { totpCodeInput = ""; totpError = ""; totpStage = "disabling"; }}>
								{t("settings.account.disable")}
							</button>
						</div>
					{/if}
				</div>

				<div class="card">
					{#if deviceLink.phase === "idle" || deviceLink.phase === "error"}
						<div class="row">
							<div>
								<p class="row-label">{t("settings.account.linkedDevices")}</p>
								<p class="row-value muted">{t("settings.account.linkedDevicesHint")}</p>
							</div>
							<button class="edit" onclick={beginDeviceLink}>
								<Smartphone size={14} strokeWidth={2} />
								{t("settings.account.linkDevice")}
							</button>
						</div>
						{#if deviceLink.phase === "error" && deviceLink.error}
							<p class="error-text" style="margin-top: 8px;">{deviceLink.error}</p>
						{/if}
					{:else if deviceLink.phase === "connecting" || deviceLink.phase === "waiting-for-peer"}
						<p class="row-label">{t("settings.account.waitingForDevice")}</p>
						<p class="hint" style="margin-bottom: 12px;">
							{t("settings.account.waitingForDeviceHint")}
						</p>
						<div class="row-actions">
							<button class="ghost" onclick={cancelDeviceLink}>{t("common.cancel")}</button>
						</div>
					{:else if deviceLink.phase === "confirm"}
						<p class="row-label">{t("settings.account.confirmCodeMatch")}</p>
						<p class="row-value totp-secret">{deviceLink.fingerprint}</p>
						<div class="row-actions" style="margin-top: 12px;">
							<button class="ghost" onclick={cancelDeviceLink}>{t("common.cancel")}</button>
							<button class="primary" onclick={confirmDeviceLink}>{t("settings.account.codesMatchSend")}</button>
						</div>
					{:else if deviceLink.phase === "sending"}
						<p class="row-label">{t("settings.account.sendingKeys")}</p>
					{:else if deviceLink.phase === "done"}
						<div class="row">
							<div>
								<p class="row-label">{t("settings.account.linkedDevices")}</p>
								<p class="row-value muted">{t("settings.account.keysSent")}</p>
							</div>
							<button class="edit" onclick={() => deviceLink.reset()}>
								<Smartphone size={14} strokeWidth={2} />
								{t("settings.account.linkAnother")}
							</button>
						</div>
					{/if}
				</div>
			{:else if section === "privacy"}
				<h2>{t("settings.nav.privacy")}</h2>

				<div class="card">
					<div class="switch-row">
						<div>
							<p class="row-label">{t("settings.privacy.activityStatus")}</p>
							<p class="row-value muted">
								{t("settings.privacy.activityStatusHint")}
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
					<p class="row-label">{t("settings.privacy.extension")}</p>
					<p class="row-value muted" style="margin-bottom: 12px;">
						{t("settings.privacy.extensionHint")}
					</p>
					<ol class="extension-steps">
						<li>{t("settings.privacy.extensionStep1")}</li>
						<li>
							{t("settings.privacy.extensionStep2a")} <code>chrome://extensions</code>
							{t("settings.privacy.extensionStep2b")}
							<strong>{t("settings.privacy.extensionStep2DevMode")}</strong>
						</li>
						<li>{t("settings.privacy.extensionStep3a")} <strong>{t("settings.privacy.extensionStep3Load")}</strong> {t("settings.privacy.extensionStep3b")}</li>
					</ol>
					<button class="edit" onclick={revealExtensionFolder}>{t("settings.privacy.openExtensionFolder")}</button>
				</div>

				<div class="card">
					<div class="row">
						<div>
							<p class="row-label">{t("settings.privacy.dataCollected")}</p>
							<p class="row-value muted">{t("settings.privacy.dataCollectedHint")}</p>
						</div>
					</div>
					<div class="row">
						<div>
							<p class="row-label">{t("settings.privacy.dmStorage")}</p>
							<p class="row-value muted">
								{t("settings.privacy.dmStorageHint")}
							</p>
						</div>
					</div>
					<div class="row">
						<div>
							<p class="row-label">{t("settings.privacy.channelStorage")}</p>
							<p class="row-value muted">
								{t("settings.privacy.channelStorageHint")}
							</p>
						</div>
					</div>
					<div class="row">
						<div>
							<p class="row-label">{t("settings.privacy.ipLogging")}</p>
							<p class="row-value muted">
								{t("settings.privacy.ipLoggingHint")}
							</p>
						</div>
					</div>
				</div>

				<div class="card">
					<p class="row-label" style="margin-bottom: 12px;">{t("settings.privacy.blockedUsers")}</p>
					{#if blocked.length === 0}
						<p class="row-value muted">{t("settings.privacy.noBlocked")}</p>
					{:else}
						{#each blocked as b (b.id)}
							<div class="row">
								<p class="row-value">{b.username}</p>
								<button class="edit" onclick={() => unblock(b.id)}>{t("settings.privacy.unblock")}</button>
							</div>
						{/each}
					{/if}
				</div>
			{:else if section === "notifications"}
				<h2>{t("settings.nav.notifications")}</h2>

				<div class="card">
					{#if pushIsSupported}
						<div class="switch-row">
							<div>
								<p class="row-label">{t("settings.notifications.push")}</p>
								<p class="row-value muted">{t("settings.notifications.pushHint")}</p>
							</div>
							<label class="switch">
								<input
									type="checkbox"
									checked={pushOn}
									disabled={pushBusy}
									onchange={(e) => togglePush((e.target as HTMLInputElement).checked)}
								/>
								<span class="track"><span class="thumb"></span></span>
							</label>
						</div>
					{/if}
					<div class="switch-row">
						<div>
							<p class="row-label">{t("settings.notifications.messages")}</p>
							<p class="row-value muted">{t("settings.notifications.messagesHint")}</p>
						</div>
						<label class="switch">
							<input type="checkbox" bind:checked={notifyMessages} onchange={() => toggle((v) => (notifyMessages = v), !notifyMessages, t("settings.notifications.messagesLabel"))} />
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
					<div class="switch-row">
						<div>
							<p class="row-label">{t("settings.notifications.mentions")}</p>
							<p class="row-value muted">{t("settings.notifications.mentionsHint")}</p>
						</div>
						<label class="switch">
							<input
							type="checkbox"
							checked={notificationSettings.mentionsEnabled}
							onchange={(e) => {
								const value = (e.target as HTMLInputElement).checked;
								notificationSettings.setMentionsEnabled(value);
								toast.push(t(value ? "toast.toggledOn" : "toast.toggledOff", { label: t("settings.notifications.mentionsLabel") }));
							}}
						/>
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
					<div class="switch-row">
						<div>
							<p class="row-label">{t("settings.notifications.sounds")}</p>
							<p class="row-value muted">{t("settings.notifications.soundsHint")}</p>
						</div>
						<label class="switch">
							<input type="checkbox" bind:checked={notifySounds} onchange={() => toggle((v) => (notifySounds = v), !notifySounds, t("settings.notifications.soundsLabel"))} />
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
				</div>
			{:else if section === "voice"}
				<h2>{t("settings.nav.voice")}</h2>
				<div class="card">
					<VoiceSettingsPanel focus="all" />
				</div>
			{:else if section === "appearance"}
				<h2>{t("settings.appearance.title")}</h2>

				<div class="card">
					<div class="row">
						<div>
							<p class="row-label">{t("settings.language.label")}</p>
							<p class="row-value muted">{t("settings.language.help")}</p>
						</div>
						<div class="select-slot">
							<Dropdown
								value={i18n.lang}
								options={LOCALES.map((l) => ({ value: l.code, label: l.label }))}
								onChange={(v) => changeLanguage(v as LocaleCode)}
							/>
						</div>
					</div>
				</div>

				<div class="card">
					<div class="switch-row">
						<div>
							<p class="row-label">{t("settings.appearance.compact")}</p>
							<p class="row-value muted">{t("settings.appearance.compactHelp")}</p>
						</div>
						<label class="switch">
							<input type="checkbox" bind:checked={compactMode} onchange={() => toggle((v) => (compactMode = v), !compactMode, t("settings.appearance.compact"))} />
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
				</div>

				<div class="card">
					<div class="row">
						<div>
							<p class="row-label">{t("settings.appearance.font")}</p>
							<p class="row-value muted">{t("settings.appearance.fontHelp")}</p>
						</div>
					</div>
					<div class="theme-options">
						<button
							class="theme-option"
							class:active={fontStore.current === "default"}
							onclick={() => fontStore.setMode("default")}
						>
							{t("settings.appearance.fontDefault")}
						</button>
						<button
							class="theme-option"
							class:active={fontStore.current === "preset"}
							onclick={() => fontStore.setMode("preset")}
						>
							{t("settings.appearance.fontPreset")}
						</button>
						<button
							class="theme-option"
							class:active={fontStore.current === "link"}
							onclick={() => fontStore.setMode("link")}
						>
							{t("settings.appearance.fontLink")}
						</button>
					</div>

					{#if fontStore.current === "preset"}
						<div class="select-slot wide">
							<Dropdown
								value={fontStore.settings.presetId}
								options={PRESET_FONT_IDS.map((id) => ({ value: id, label: FONT_LABELS[id] }))}
								onChange={(v) => fontStore.setPreset(v as FontId)}
							/>
						</div>
					{:else if fontStore.current === "link"}
						<div class="font-link-form">
							<input
								type="text"
								class="font-link-input"
								placeholder={t("settings.appearance.fontFamilyPlaceholder")}
								bind:value={customFontFamilyDraft}
							/>
							<input
								type="text"
								class="font-link-input"
								placeholder={t("settings.appearance.fontUrlPlaceholder")}
								bind:value={customFontUrlDraft}
							/>
							<button
								class="theme-option"
								disabled={!customFontFamilyDraft.trim() || !customFontUrlDraft.trim()}
								onclick={() => fontStore.setCustom(customFontFamilyDraft.trim(), customFontUrlDraft.trim())}
							>
								{t("common.apply")}
							</button>
						</div>
					{/if}
				</div>

				<div class="card">
					<div class="row">
						<div>
							<p class="row-label">{t("settings.appearance.theme")}</p>
							<p class="row-value muted">{t("settings.appearance.themeHelp")}</p>
						</div>
					</div>
					<div class="theme-options">
						<button
							class="theme-option"
							class:active={themeStore.settings.mode === "default"}
							onclick={() => themeStore.setMode("default")}
						>
							{t("settings.appearance.themeDefault")}
						</button>
						<button
							class="theme-option"
							class:active={themeStore.settings.mode === "custom"}
							onclick={() => themeStore.setMode("custom")}
						>
							{t("settings.appearance.themeCustom")}
						</button>
					</div>
				</div>

				{#if themeStore.settings.mode === "custom"}
					<div class="card">
						<p class="row-label" style="margin-bottom: 8px;">{t("settings.appearance.presets")}</p>
						<div class="preset-grid">
							{#each THEME_PRESETS as preset (preset.id)}
								<button
									class="preset-swatch"
									class:active={themeStore.settings.presetId === preset.id}
									onclick={() => themeStore.applyPreset(preset.id)}
									title={preset.label}
								>
									<span class="preset-preview">
										<span class="preset-dot" style:background={preset.colors["accent-fill"]}></span>
										<span class="preset-dot" style:background={preset.colors.panel}></span>
										<span class="preset-dot" style:background={preset.colors.void}></span>
									</span>
									<span class="preset-name">{preset.label}</span>
								</button>
							{/each}
						</div>
					</div>
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
						{t("settings.appearance.resetColors")}
					</button>
				{/if}
			{:else if section === "accessibility"}
				<h2>{t("settings.nav.accessibility")}</h2>

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
				<h2>{t("settings.nav.devices")}</h2>
				<p class="hint" style="margin-bottom: 16px;">{t("settings.devices.subtitle")}</p>

				<div class="card">
					{#if sessions.length === 0}
						<p class="row-value muted">{t("settings.devices.none")}</p>
					{/if}
					{#each sessions as s (s.id)}
						<div class="row">
							<div>
								<p class="row-label">{s.current ? t("settings.devices.thisDevice") : t("settings.devices.otherDevice")}</p>
								<p class="row-value muted">{describeSession(s)}</p>
							</div>
							{#if !s.current}
								<button class="edit danger-text" onclick={() => revoke(s.id)}>{t("settings.devices.revoke")}</button>
							{/if}
						</div>
					{/each}
				</div>
			{:else if section === "billing"}
				<h2>{t("settings.nav.billing")}</h2>

				<div class="card plan-card" class:premium={billing?.tier === "premium"}>
					<div class="plan-header">
						{#if billing?.tier === "premium"}
							<Sparkles size={18} strokeWidth={2} />
						{:else}
							<CreditCard size={18} strokeWidth={2} />
						{/if}
						<p class="row-label">{billing?.tier === "premium" ? t("settings.billing.premiumPlan") : t("settings.billing.freePlan")}</p>
					</div>
					<p class="row-value muted">
						{#if billing?.tier === "premium"}
							{t("settings.billing.premiumBlurb")}
						{:else}
							{t("settings.billing.freeBlurb")}
						{/if}
					</p>
					{#if billing?.subscription_status && billing.subscription_status !== "active"}
						<p class="row-value muted">{t("settings.billing.subscriptionStatus", { status: billing.subscription_status })}</p>
					{/if}
				</div>

				<div class="card compare-card">
					<div class="compare-head">
						<span></span>
						<span>{t("settings.billing.freePlan")}</span>
						<span class="pro-col">{t("settings.billing.planName")}</span>
					</div>
					{#each PLAN_ROWS as row (row.label)}
						<div class="compare-row">
							<span class="compare-label">{t(row.label)}</span>
							<span class="compare-cell">
								{#if typeof row.free === "string"}{t(row.free)}
								{:else if row.free}<Check size={14} strokeWidth={2.5} class="cmp-yes" />
								{:else}<span class="cmp-no">—</span>{/if}
							</span>
							<span class="compare-cell pro-col">
								{#if typeof row.pro === "string"}{t(row.pro)}
								{:else if row.pro}<Check size={14} strokeWidth={2.5} class="cmp-yes" />
								{:else}<span class="cmp-no">—</span>{/if}
							</span>
						</div>
					{/each}
				</div>

				{#if billing?.tier !== "premium"}
					<div class="card">
						<p class="row-label">{t("settings.billing.upgradeTitle")}</p>
						<p class="row-value muted" style="margin-bottom: 12px;">
							{t("settings.billing.upgradeBlurb")}
						</p>
						<button class="edit" onclick={upgrade} disabled={checkoutLoading}>
							{checkoutLoading ? t("settings.billing.openingCheckout") : t("settings.billing.upgrade")}
						</button>
					</div>
				{:else}
					<div class="card">
						<p class="row-label">{t("settings.billing.voidShards")}</p>
						<p class="row-value muted" style="margin-bottom: 12px;">
							{t("settings.billing.voidShardsHint", { used: billing.boost_slots_used, total: billing.boost_slots_total })}
						</p>
						{#if myServers.length === 0}
							<p class="row-value muted">{t("settings.billing.noServers")}</p>
						{:else}
							<div class="boost-list">
								{#each myServers as server (server.id)}
									{@const mine = myBoostCounts[server.id] ?? 0}
									{@const outOfSlots = billing.boost_slots_used >= billing.boost_slots_total}
									<div class="boost-row">
										<span class="boost-name">{server.name}</span>
										{#if server.boost_count > 0}
											<span class="boost-count">
												<Sparkles size={12} strokeWidth={2.25} />
												{server.boost_count}
											</span>
										{/if}
										<div class="boost-stepper" class:active={mine > 0}>
											<button
												type="button"
												class="boost-step"
												aria-label={t("settings.billing.boostRemove")}
												disabled={boostBusyId === server.id || mine === 0}
												onclick={() => changeBoost(server.id, -1)}
											>−</button>
											<span class="boost-mine">{mine}</span>
											<button
												type="button"
												class="boost-step"
												aria-label={t("settings.billing.boostAdd")}
												disabled={boostBusyId === server.id || outOfSlots}
												onclick={() => changeBoost(server.id, 1)}
											>+</button>
										</div>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{/if}
			{:else if section === "moderation" && isStaff}
				{@const openReport = reports.find((r) => r.id === openReportId) ?? null}
				{#if openReport}
					<button type="button" class="back-link" onclick={() => (openReportId = null)}>
						<ArrowLeft size={14} strokeWidth={2.25} />
						{t("settings.moderation.backToReports")}
					</button>

					<div class="card ticket-card">
						<p class="ticket-field">
							<span class="ticket-label">{t("settings.moderation.reportFrom")}</span>
							<span class="ticket-value">{openReport.reporter_username}</span>
						</p>
						<p class="ticket-field">
							<span class="ticket-label">{t("settings.moderation.reported")}</span>
							<span class="ticket-value">{openReport.reported_username}</span>
						</p>
						<p class="ticket-field">
							<span class="ticket-label">{t("settings.moderation.context")}</span>
							<span class="ticket-value">{openReport.context_kind}</span>
						</p>
						<p class="ticket-field">
							<span class="ticket-label">{t("settings.moderation.filed")}</span>
							<span class="ticket-value">{new Date(openReport.created_at).toLocaleString()}</span>
						</p>
						<p class="ticket-field">
							<span class="ticket-label">{t("settings.moderation.status")}</span>
							<span class="select-slot">
								<Dropdown
									value={openReport.status}
									options={REPORT_STATUSES_LIST.map((s) => ({
										value: s,
										label: t(`settings.moderation.status.${s}`)
									}))}
									onChange={(v) => setReportStatus(openReport, v as api.ReportStatus)}
								/>
							</span>
						</p>
						<button type="button" class="save" onclick={() => messageReporter(openReport.reporter_username)}>
							<MessageSquare size={14} strokeWidth={2} />
							{t("settings.moderation.messageReporter")}
						</button>
					</div>

					<div class="card">
						<p class="row-label" style="margin-bottom: 8px;">{t("settings.moderation.reasonMessages")}</p>

						{#if decryptedReport}
							<div class="ticket-field">
								<span class="ticket-label">{t("settings.moderation.category")}</span>
								<span class="ticket-value">{decryptedReport.category}</span>
							</div>
							<p class="row-label" style="margin: 12px 0 4px;">{t("settings.moderation.reasonLabel")}</p>
							<p class="row-value" style="white-space: pre-wrap;">{decryptedReport.reason}</p>

							{#if decryptedReport.messages.length}
								<p class="row-label" style="margin: 16px 0 8px;">{t("settings.moderation.reportedMessages")}</p>
								<div class="report-list">
									{#each decryptedReport.messages as m (m.id)}
										<div class="mod-message">
											<p class="report-meta">
												<strong>{m.senderUsername}</strong> · {new Date(m.timestamp).toLocaleString()}
											</p>
											<p class="row-value" style="white-space: pre-wrap;">{m.text}</p>
											{#if m.attachmentFilename}
												<p class="report-meta">📎 {m.attachmentFilename}</p>
											{/if}
										</div>
									{/each}
								</div>
							{/if}

							{#if decryptedReport.screenshot}
								<p class="row-label" style="margin: 16px 0 8px;">{t("settings.moderation.screenshot")}</p>
								<img
									class="mod-screenshot"
									src={`data:${decryptedReport.screenshot.mimeType};base64,${decryptedReport.screenshot.dataBase64}`}
									alt={t("settings.moderation.screenshot")}
								/>
							{/if}
						{:else}
							<p class="row-value muted" style="margin-bottom: 12px;">
								{t("settings.moderation.decryptIntro")}
							</p>

							{#if showKeyInput || !moderationKey.present}
								<input
									class="inline-input"
									type="password"
									autocomplete="off"
									placeholder={t("settings.moderation.keyPlaceholder")}
									value={moderationKey.value}
									oninput={(e) => (moderationKey.value = e.currentTarget.value)}
								/>
								<p class="row-value muted" style="margin: 6px 0 12px; font-size: 12px;">
									{t("settings.moderation.keyHint")}
								</p>
							{/if}

							{#if decryptError}
								<p class="row-value" style="color: var(--danger); margin-bottom: 12px;">{decryptError}</p>
							{/if}

							<div class="ticket-actions">
								<button
									type="button"
									class="save"
									disabled={decrypting}
									onclick={() => decryptReport(openReport.id)}
								>
									{decrypting ? t("common.loading") : t("settings.moderation.decryptButton")}
								</button>
								{#if moderationKey.present}
									<button type="button" class="theme-option" onclick={() => moderationKey.clear()}>
										{t("settings.moderation.forgetKey")}
									</button>
								{/if}
							</div>
						{/if}

						<details class="mod-offline">
							<summary>{t("settings.moderation.offlineFallback")}</summary>
							<p class="row-value muted" style="margin: 8px 0 12px;">
								{t("settings.moderation.reasonMessagesHint")} <code>tools/decrypt-report.mjs</code>.
							</p>
							<div class="ticket-actions">
								<button type="button" class="theme-option" onclick={() => copyReportId(openReport.id)}>
									<Copy size={13} strokeWidth={2} />
									{t("settings.moderation.copyReportId")}
								</button>
								<button type="button" class="theme-option" onclick={() => copyFetchCommand(openReport.id)}>
									<Copy size={13} strokeWidth={2} />
									{t("settings.moderation.copyPsql")}
								</button>
							</div>
						</details>
					</div>
				{:else}
					<h2>{t("settings.nav.moderation")}</h2>
					<div class="card">
						<p class="row-value muted" style="margin-bottom: 12px;">
							{t("settings.moderation.overviewHintA")} <code>tools/decrypt-report.mjs</code>. {t("settings.moderation.overviewHintB")}
						</p>
						<div class="report-filter">
							{#each REPORT_FILTERS as filter (filter)}
								<button
									type="button"
									class="filter-chip"
									class:active={reportStatusFilter === filter}
									onclick={() => (reportStatusFilter = filter)}
								>
									{t(`settings.moderation.status.${filter}`)}
								</button>
							{/each}
						</div>
					</div>
					{#if reportsLoading}
						<p class="row-value muted">{t("common.loading")}</p>
					{:else if visibleReports.length === 0}
						<p class="row-value muted">{t("settings.moderation.noReports")}</p>
					{:else}
						<div class="report-list">
							{#each visibleReports as report (report.id)}
								<div class="report-row">
									<div class="report-main">
										<p class="report-line">
											<strong>{report.reporter_username}</strong> {t("settings.moderation.reportedConnector")}
											<strong>{report.reported_username}</strong>
										</p>
										<p class="report-meta">
											{report.context_kind} · {new Date(report.created_at).toLocaleString()}
										</p>
									</div>
									<span class="status-pill {report.status}">{t(`settings.moderation.status.${report.status}`)}</span>
									<button type="button" class="save" onclick={() => openReportDetail(report.id)}>
										{t("settings.moderation.openButton")}
									</button>
								</div>
							{/each}
						</div>
					{/if}
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

	.extension-steps {
		margin: 0 0 12px;
		padding-left: 18px;
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: 13px;
		color: var(--ink-dim);
		line-height: 1.5;
	}

	.extension-steps code {
		padding: 1px 5px;
		border-radius: 4px;
		background: var(--active);
		font-family: var(--font-mono);
		font-size: 12px;
		color: var(--ink);
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

	.font-select {
		margin-top: 10px;
		width: 100%;
		padding: 10px 12px;
		border-radius: 6px;
		border: 1px solid var(--hairline);
		background: var(--panel);
		color: var(--ink);
		font-size: 13px;
	}

	.select-slot {
		width: 200px;
		flex-shrink: 0;
	}

	.select-slot.wide {
		width: 100%;
		display: block;
		margin-top: 10px;
	}

	.font-link-form {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin-top: 10px;
	}

	.font-link-input {
		padding: 10px 12px;
		border-radius: 6px;
		border: 1px solid var(--hairline);
		background: var(--panel);
		color: var(--ink);
		font-size: 13px;
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

	.preset-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
		gap: 8px;
	}

	.preset-swatch {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 8px;
		padding: 10px 12px;
		border-radius: 8px;
		border: 1px solid var(--hairline);
		background: var(--sidebar);
		transition: background-color 0.15s ease, border-color 0.15s ease;
	}

	.preset-swatch:hover {
		background: var(--hover);
	}

	.preset-swatch.active {
		border-color: var(--accent-fill);
		background: var(--hover);
	}

	.preset-preview {
		display: flex;
		gap: 4px;
	}

	.preset-dot {
		width: 16px;
		height: 16px;
		border-radius: 50%;
		border: 1px solid rgba(255, 255, 255, 0.1);
	}

	.preset-name {
		font-size: 12px;
		font-weight: 600;
		color: var(--ink-dim);
	}

	.preset-swatch.active .preset-name {
		color: var(--ink);
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

	.compare-card {
		display: flex;
		flex-direction: column;
		gap: 0;
		padding: 4px 0;
	}

	.compare-head,
	.compare-row {
		display: grid;
		grid-template-columns: 1fr 88px 110px;
		align-items: center;
		gap: 8px;
		padding: 9px 16px;
	}

	.compare-row {
		border-top: 1px solid var(--hairline);
	}

	.compare-head {
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--ink-faint);
	}

	.compare-head .pro-col,
	.compare-cell.pro-col {
		color: var(--accent, #8ea1ff);
	}

	.compare-head span:not(:first-child),
	.compare-cell {
		text-align: center;
	}

	.compare-label {
		font-size: 13px;
		color: var(--ink-dim);
	}

	.compare-cell {
		font-size: 12px;
		font-weight: 600;
		color: var(--ink);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.compare-cell :global(.cmp-yes) {
		color: var(--online);
	}

	.compare-cell .cmp-no {
		color: var(--ink-faint);
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

	.boost-stepper {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 2px;
		border-radius: 6px;
		background: var(--hover);
	}

	.boost-stepper.active {
		background: color-mix(in srgb, var(--online) 22%, transparent);
	}

	.boost-step {
		width: 22px;
		height: 22px;
		border-radius: 4px;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 15px;
		font-weight: 700;
		line-height: 1;
		color: var(--ink-dim);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.boost-step:hover:not(:disabled) {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.boost-step:disabled {
		opacity: 0.35;
		cursor: default;
	}

	.boost-mine {
		min-width: 16px;
		text-align: center;
		font-size: 12px;
		font-weight: 700;
		color: var(--ink);
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

	.report-row .save {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 5px;
	}

	.status-pill {
		flex-shrink: 0;
		padding: 3px 9px;
		border-radius: 999px;
		font-size: 11px;
		font-weight: 600;
		text-transform: capitalize;
		background: var(--active);
		color: var(--ink-dim);
	}

	.status-pill.open {
		background: var(--danger);
		color: #fff;
	}

	.status-pill.reviewing {
		background: var(--idle);
		color: #1c1815;
	}

	.status-pill.resolved {
		background: var(--online);
		color: #06210f;
	}

	.back-link {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-bottom: 12px;
		font-size: 13px;
		font-weight: 600;
		color: var(--ink-dim);
	}

	.back-link:hover {
		color: var(--ink);
	}

	.ticket-card {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.ticket-field {
		display: flex;
		align-items: center;
		gap: 10px;
		margin: 0;
	}

	.mod-message {
		padding: 10px 12px;
		border-radius: var(--radius-sm, 8px);
		background: var(--active);
	}

	.mod-message .report-meta {
		margin-bottom: 4px;
	}

	.mod-screenshot {
		max-width: 100%;
		border-radius: var(--radius-sm, 8px);
		border: 1px solid var(--border, var(--ink-faint));
	}

	.mod-offline {
		margin-top: 16px;
		border-top: 1px solid var(--border, var(--ink-faint));
		padding-top: 12px;
	}

	.mod-offline summary {
		cursor: pointer;
		font-size: 12px;
		color: var(--ink-dim);
		user-select: none;
	}

	.ticket-label {
		width: 100px;
		flex-shrink: 0;
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--ink-faint);
	}

	.ticket-value {
		font-size: 13px;
		color: var(--ink);
	}

	.ticket-card .save {
		align-self: flex-start;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.ticket-actions {
		display: flex;
		gap: 8px;
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
		position: relative;
		overflow: hidden;
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
		position: relative;
		overflow: hidden;
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
