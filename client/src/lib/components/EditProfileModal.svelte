<script lang="ts">
	import { fade } from "svelte/transition";
	import X from "@lucide/svelte/icons/x";
	import ChevronDown from "@lucide/svelte/icons/chevron-down";
	import Plus from "@lucide/svelte/icons/plus";
	import Check from "@lucide/svelte/icons/check";
	import Settings2 from "@lucide/svelte/icons/settings-2";
	import MessageSquare from "@lucide/svelte/icons/message-square";
	import LayoutGrid from "@lucide/svelte/icons/layout-grid";
	import MoreHorizontal from "@lucide/svelte/icons/more-horizontal";
	import ExternalLink from "@lucide/svelte/icons/external-link";
	import Globe from "@lucide/svelte/icons/globe";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import Search from "@lucide/svelte/icons/search";
	import Pencil from "@lucide/svelte/icons/pencil";
	import Pin from "@lucide/svelte/icons/pin";
	import Gamepad2 from "@lucide/svelte/icons/gamepad-2";
	import ColorPicker from "$lib/components/ColorPicker.svelte";
	import Badges from "$lib/components/Badges.svelte";
	import BrandIcon from "$lib/components/BrandIcon.svelte";
	import { BRAND_ICONS } from "$lib/data/brandIcons";

	// Xbox has no CC0 mark in Simple Icons (pulled after a takedown request),
	// so it gets a neutral glyph instead of a logo.
	const FALLBACK_ICON: Partial<Record<api.ConnectionService, typeof Globe>> = {
		xbox: Gamepad2
	};
	import { session } from "$lib/stores/session.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import { badgeStore } from "$lib/stores/badges.svelte";
	import { toast } from "$lib/stores/toast.svelte";
	import * as api from "$lib/api/client";
	import { GAME_CATALOG, coverUrl, type CatalogGame } from "$lib/data/gameCatalog";

	let { username, onClose }: {
		username: string;
		onClose: () => void;
	} = $props();

	const profile = $derived(profileStore.forUser(username));
	const badges = $derived(badgeStore.forUser(username));

	let avatarInput: HTMLInputElement | undefined;
	let bannerInput: HTMLInputElement | undefined;
	let avatarUploading = $state(false);
	let bannerUploading = $state(false);
	let styleOpen = $state(false);
	let accentDraft = $state("#5b96c9");
	let initialized = false;

	$effect(() => {
		const p = profile;
		if (!p || initialized) return;
		accentDraft = p.accent_color ?? "#5b96c9";
		initialized = true;
	});

	$effect(() => {
		accentDraft;
		if (!initialized) return;
		const token = session.token;
		if (!token) return;
		api
			.updateProfile(token, { accent_color: accentDraft })
			.then((updated) => profileStore.set(updated))
			.catch(() => toast.push("Couldn't update name color"));
	});

	async function onAvatarChosen(event: Event) {
		const token = session.token;
		const file = (event.target as HTMLInputElement).files?.[0];
		if (!token || !file) return;
		avatarUploading = true;
		try {
			const attachment = await api.uploadFile(token, file);
			profileStore.set(await api.setAvatar(token, attachment.id));
		} catch {
			toast.push("Couldn't update avatar");
		} finally {
			avatarUploading = false;
			if (avatarInput) avatarInput.value = "";
		}
	}

	async function onBannerChosen(event: Event) {
		const token = session.token;
		const file = (event.target as HTMLInputElement).files?.[0];
		if (!token || !file) return;
		bannerUploading = true;
		try {
			const attachment = await api.uploadFile(token, file);
			profileStore.set(await api.setBanner(token, attachment.id));
		} catch {
			toast.push("Couldn't update banner");
		} finally {
			bannerUploading = false;
			if (bannerInput) bannerInput.value = "";
		}
	}

	function notAvailable(what: string) {
		toast.push(`${what} isn't available yet`);
	}

	type ServiceMeta = { id: api.ConnectionService; label: string; placeholder: string };

	// Xbox has no CC0 mark available (removed from Simple Icons after a
	// takedown request), so it falls back to a neutral glyph via BrandIcon.
	const CONNECTION_CATALOG: ServiceMeta[] = [
		{ id: "github", label: "GitHub", placeholder: "https://github.com/username" },
		{ id: "youtube", label: "YouTube", placeholder: "https://youtube.com/@channel" },
		{ id: "twitch", label: "Twitch", placeholder: "https://twitch.tv/username" },
		{ id: "x", label: "X", placeholder: "https://x.com/username" },
		{ id: "instagram", label: "Instagram", placeholder: "https://instagram.com/username" },
		{ id: "tiktok", label: "TikTok", placeholder: "https://tiktok.com/@username" },
		{ id: "reddit", label: "Reddit", placeholder: "https://reddit.com/u/username" },
		{ id: "steam", label: "Steam", placeholder: "https://steamcommunity.com/id/username" },
		{ id: "spotify", label: "Spotify", placeholder: "https://open.spotify.com/user/username" },
		{ id: "discord", label: "Discord", placeholder: "https://discord.gg/invite" },
		{ id: "facebook", label: "Facebook", placeholder: "https://facebook.com/username" },
		{ id: "telegram", label: "Telegram", placeholder: "https://t.me/username" },
		{ id: "vk", label: "VK", placeholder: "https://vk.com/username" },
		{ id: "behance", label: "Behance", placeholder: "https://behance.net/username" },
		{ id: "dribbble", label: "Dribbble", placeholder: "https://dribbble.com/username" },
		{ id: "soundcloud", label: "SoundCloud", placeholder: "https://soundcloud.com/username" },
		{ id: "bandcamp", label: "Bandcamp", placeholder: "https://username.bandcamp.com" },
		{ id: "itchio", label: "itch.io", placeholder: "https://username.itch.io" },
		{ id: "xbox", label: "Xbox", placeholder: "https://xbox.com/en-US/play/user/username" },
		{ id: "playstation", label: "PlayStation", placeholder: "https://playstation.com/en-us/psn-profile/username" },
		{ id: "battlenet", label: "Battle.net", placeholder: "https://battle.net/..." },
		{ id: "epicgames", label: "Epic Games", placeholder: "https://store.epicgames.com/u/username" },
		{ id: "roblox", label: "Roblox", placeholder: "https://roblox.com/users/username/profile" }
	];

	let connections = $state<api.ApiConnection[]>([]);
	let addingConnection = $state(false);
	let serviceSearch = $state("");
	let selectedService = $state<ServiceMeta | null>(null);
	let newUrl = $state("");
	let savingConnection = $state(false);

	const filteredServices = $derived(
		CONNECTION_CATALOG.filter((s) => s.label.toLowerCase().includes(serviceSearch.trim().toLowerCase()))
	);

	async function loadConnections() {
		const token = session.token;
		if (!token) return;
		try {
			connections = await api.listConnections(token, username);
		} catch {
			connections = [];
		}
	}

	$effect(() => {
		if (session.token) loadConnections();
	});

	function startAddConnection() {
		serviceSearch = "";
		selectedService = null;
		newUrl = "";
		addingConnection = true;
	}

	function pickService(service: ServiceMeta) {
		selectedService = service;
		newUrl = "";
	}

	function cancelAddConnection() {
		addingConnection = false;
		selectedService = null;
	}

	async function submitConnection() {
		const token = session.token;
		if (!token || !selectedService) return;
		const url = newUrl.trim();
		if (!url) return;
		savingConnection = true;
		try {
			const created = await api.addConnection(token, selectedService.id, url);
			connections = [...connections, created];
			addingConnection = false;
			selectedService = null;
		} catch (err) {
			toast.push(err instanceof api.ApiError ? err.message : "Couldn't add connection");
		} finally {
			savingConnection = false;
		}
	}

	async function deleteConnection(id: string) {
		const token = session.token;
		if (!token) return;
		const previous = connections;
		connections = connections.filter((c) => c.id !== id);
		try {
			await api.removeConnection(token, id);
		} catch {
			connections = previous;
			toast.push("Couldn't remove connection");
		}
	}

	type BoardTab = "board" | "activity" | "wishlist";
	let boardTab = $state<BoardTab>("board");

	const WIDGET_KIND_LABELS: Record<api.WidgetKind, string> = {
		favorite_game: "Favorite Game",
		want_to_play: "Want to Play",
		games_i_like: "Games I Like",
		games_in_rotation: "Games in Rotation"
	};
	const WIDGET_KIND_OPTIONS: { id: api.WidgetKind; label: string }[] = [
		{ id: "favorite_game", label: "Favorite Game" },
		{ id: "want_to_play", label: "Want to Play" },
		{ id: "games_i_like", label: "Games I Like" },
		{ id: "games_in_rotation", label: "Games in Rotation" }
	];

	let widgets = $state<api.ApiWidget[]>([]);
	let pickingGame = $state(false);
	let pendingKind = $state<api.WidgetKind>("favorite_game");
	let gameSearch = $state("");
	let savingWidget = $state(false);

	let editingDescriptionId = $state<string | null>(null);
	let descriptionDraft = $state("");
	let addingTagId = $state<string | null>(null);
	let tagDraft = $state("");

	const filteredGames = $derived.by(() => {
		const q = gameSearch.trim().toLowerCase();
		const pool = q ? GAME_CATALOG.filter((g) => g.name.toLowerCase().includes(q)) : GAME_CATALOG;
		return pool.slice(0, 8);
	});

	const suggestedGames = GAME_CATALOG.slice(0, 6);

	async function loadWidgets() {
		const token = session.token;
		if (!token) return;
		try {
			widgets = await api.listWidgets(token, username);
		} catch {
			widgets = [];
		}
	}

	$effect(() => {
		if (session.token) loadWidgets();
	});

	function chooseKind(kind: api.WidgetKind) {
		pendingKind = kind;
		gameSearch = "";
		pickingGame = true;
	}

	function cancelAddWidget() {
		pickingGame = false;
		gameSearch = "";
	}

	async function selectGame(game: CatalogGame) {
		const token = session.token;
		if (!token) return;
		savingWidget = true;
		try {
			const created = await api.addWidget(token, pendingKind, game.name, {
				externalImageUrl: coverUrl(game.appid)
			});
			widgets = [...widgets, created];
			cancelAddWidget();
		} catch (err) {
			toast.push(err instanceof api.ApiError ? err.message : "Couldn't add widget");
		} finally {
			savingWidget = false;
		}
	}

	async function togglePinned(widget: api.ApiWidget) {
		const token = session.token;
		if (!token) return;
		const previous = widgets;
		try {
			const updated = await api.updateWidget(token, widget.id, { pinned: !widget.pinned });
			widgets = widgets.map((w) => (w.id === widget.id ? updated : w)).sort((a, b) => Number(b.pinned) - Number(a.pinned));
		} catch {
			widgets = previous;
			toast.push("Couldn't update widget");
		}
	}

	async function deleteWidget(id: string) {
		const token = session.token;
		if (!token) return;
		const previous = widgets;
		widgets = widgets.filter((w) => w.id !== id);
		try {
			await api.removeWidget(token, id);
		} catch {
			widgets = previous;
			toast.push("Couldn't remove widget");
		}
	}

	function startEditDescription(widget: api.ApiWidget) {
		editingDescriptionId = widget.id;
		descriptionDraft = widget.description ?? "";
	}

	async function saveDescription(widget: api.ApiWidget) {
		editingDescriptionId = null;
		const token = session.token;
		if (!token) return;
		try {
			const updated = await api.updateWidget(token, widget.id, { description: descriptionDraft });
			widgets = widgets.map((w) => (w.id === widget.id ? updated : w));
		} catch {
			toast.push("Couldn't update widget");
		}
	}

	function startAddTag(widget: api.ApiWidget) {
		addingTagId = widget.id;
		tagDraft = "";
	}

	async function saveTag(widget: api.ApiWidget) {
		addingTagId = null;
		const trimmed = tagDraft.trim();
		if (!trimmed) return;
		if (widget.tags.includes(trimmed)) return;
		if (widget.tags.length >= 5) {
			toast.push("Up to 5 tags per widget");
			return;
		}
		const token = session.token;
		if (!token) return;
		try {
			const updated = await api.updateWidget(token, widget.id, { tags: [...widget.tags, trimmed] });
			widgets = widgets.map((w) => (w.id === widget.id ? updated : w));
		} catch {
			toast.push("Couldn't add tag");
		}
	}

	async function removeTag(widget: api.ApiWidget, tag: string) {
		const token = session.token;
		if (!token) return;
		const previous = widgets;
		const nextTags = widget.tags.filter((t) => t !== tag);
		widgets = widgets.map((w) => (w.id === widget.id ? { ...w, tags: nextTags } : w));
		try {
			const updated = await api.updateWidget(token, widget.id, { tags: nextTags });
			widgets = widgets.map((w) => (w.id === widget.id ? updated : w));
		} catch {
			widgets = previous;
			toast.push("Couldn't remove tag");
		}
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") onClose();
	}

	function onOverlayClick() {
		onClose();
	}

	const memberSince = $derived(
		profile?.member_since
			? new Date(profile.member_since).toLocaleDateString(undefined, { month: "long", day: "numeric", year: "numeric" })
			: ""
	);

	const bioLines = $derived((profile?.bio ?? "").split("\n").filter((line) => line.trim().length > 0));
</script>

<svelte:window onkeydown={onKeydown} />

<div class="overlay" role="presentation" onclick={onOverlayClick} transition:fade={{ duration: 140 }}>
<div class="editor" role="dialog" aria-modal="true" aria-label="Edit Profile" tabindex="-1" onclick={(e) => e.stopPropagation()}>
	<button class="close" onclick={onClose} title="Close">
		<X size={20} strokeWidth={2} />
	</button>

	<div class="col-sidebar">
		<button class="nav-header" onclick={() => notAvailable("Additional profile pages")}>
			Main Profile
			<ChevronDown size={14} strokeWidth={2.5} />
		</button>

		<section class="section">
			<p class="section-label">Nameplate</p>
			<div class="nameplate-row">
				<div class="nameplate-preview" style:background={accentDraft}></div>
				<button class="slot small" onclick={() => notAvailable("Nameplates")} title="Nameplates aren't available yet">
					<Plus size={14} strokeWidth={2.5} />
				</button>
			</div>
		</section>

		<section class="section">
			<p class="section-label">Avatar &amp; Decoration</p>
			<div class="slot-row">
				<button class="slot" onclick={() => avatarInput?.click()} disabled={avatarUploading} title="Change avatar">
					<div
						class="slot-avatar"
						style:background-image={profile?.avatar_url ? `url(${api.resolveUrl(profile.avatar_url, session.token)})` : undefined}
					>
						{#if !profile?.avatar_url}{username.slice(0, 2).toUpperCase()}{/if}
					</div>
				</button>
				<input bind:this={avatarInput} type="file" accept="image/*" hidden onchange={onAvatarChosen} />
				<button class="slot" onclick={() => notAvailable("Avatar decorations")} title="Avatar decorations aren't available yet">
					<Plus size={16} strokeWidth={2} />
				</button>
			</div>
		</section>

		<section class="section">
			<div class="section-header-row">
				<p class="section-label">Display Name Style</p>
				<button class="icon-btn" onclick={() => (styleOpen = !styleOpen)} title="Name color">
					<Settings2 size={13} strokeWidth={2} />
				</button>
			</div>
			<div class="nameplate-preview-text" style:color={accentDraft}>
				{profile?.display_name || username}
			</div>
			{#if styleOpen}
				<div class="style-popover">
					<ColorPicker bind:value={accentDraft} />
				</div>
			{/if}
		</section>

		<section class="section">
			<p class="section-label">Theme &amp; Banner</p>
			<div class="slot-row">
				<button
					class="slot theme-slot active"
					onclick={() => bannerInput?.click()}
					disabled={bannerUploading}
					title="Change banner"
					style:background={profile?.banner_url
						? `url(${api.resolveUrl(profile.banner_url, session.token)}) center/cover`
						: profile?.banner_color || accentDraft}
				>
					<span class="theme-check"><Check size={11} strokeWidth={3} /></span>
				</button>
				<input bind:this={bannerInput} type="file" accept="image/*" hidden onchange={onBannerChosen} />
				<button class="slot theme-slot" onclick={() => notAvailable("Alternate themes")} title="More themes aren't available yet">
					<Plus size={16} strokeWidth={2} />
				</button>
			</div>
		</section>
	</div>

	<div class="col-center">
		<div
			class="preview-banner"
			style:background={profile?.banner_url
				? `url(${api.resolveUrl(profile.banner_url, session.token)}) center/cover`
				: profile?.banner_color || profile?.accent_color || "#5865f2"}
		></div>
		<div class="preview-body">
			<div class="preview-identity">
				<div class="preview-top-row">
					<div
						class="preview-avatar avatar-ring on-panel {profile?.presence ?? 'online'}"
						style:background-image={profile?.avatar_url ? `url(${api.resolveUrl(profile.avatar_url, session.token)})` : undefined}
					>
						{#if !profile?.avatar_url}{username.slice(0, 2).toUpperCase()}{/if}
					</div>
					{#if profile?.status_text}
						<div class="status-bubble">{profile.status_text}</div>
					{/if}
				</div>

				<p class="preview-name" style:color={accentDraft}>
					{profile?.display_name || username}
				</p>
				<p class="preview-handle">@{username}</p>
				{#if badges.length > 0}
					<div class="preview-badges"><Badges {badges} /></div>
				{/if}

				<div class="preview-actions">
					<button class="action primary" disabled title="This is your own profile">
						<MessageSquare size={14} strokeWidth={2} />
						Message
					</button>
					<button class="action icon-only" onclick={() => (boardTab = "board")} title="Go to Widgets">
						<LayoutGrid size={14} strokeWidth={2} />
					</button>
					<button class="action icon-only" disabled title="No further actions">
						<MoreHorizontal size={14} strokeWidth={2} />
					</button>
				</div>
			</div>

			<div class="divider"></div>

			<div class="preview-info">
			{#if bioLines.length > 0}
				<div class="info-block">
					{#each bioLines as line}
						<p class="bio-line">{line}</p>
					{/each}
				</div>
			{/if}

			<div class="info-block">
				<p class="info-label">Member Since</p>
				<p class="info-value">{memberSince}</p>
			</div>

			<div class="info-block">
				<p class="info-label">Connections</p>
				{#each connections as connection (connection.id)}
					<div class="connection-row">
						{#if BRAND_ICONS[connection.service]}
							<BrandIcon service={connection.service} size={14} chip />
						{:else}
							{@const Fallback = FALLBACK_ICON[connection.service] ?? Globe}
							<Fallback size={14} strokeWidth={2} />
						{/if}
						<a href={connection.url} target="_blank" rel="noreferrer" class="connection-label">
							{connection.label}
						</a>
						<ExternalLink size={11} strokeWidth={2} class="connection-external" />
						<button class="connection-remove" onclick={() => deleteConnection(connection.id)} title="Remove">
							<Trash2 size={12} strokeWidth={2} />
						</button>
					</div>
				{/each}

				{#if addingConnection}
					<div class="connection-form">
						{#if !selectedService}
							<div class="service-search">
								<Search size={13} strokeWidth={2} />
								<input type="text" placeholder="Search a service…" bind:value={serviceSearch} />
							</div>
							<div class="service-list">
								{#each filteredServices as service (service.id)}
									<button class="service-option" onclick={() => pickService(service)}>
										{#if BRAND_ICONS[service.id]}
											<BrandIcon service={service.id} size={14} chip />
										{:else}
											{@const Fallback = FALLBACK_ICON[service.id] ?? Globe}
											<Fallback size={14} strokeWidth={2} />
										{/if}
										{service.label}
									</button>
								{:else}
									<p class="service-empty">No matching service</p>
								{/each}
							</div>
							<div class="connection-form-actions">
								<button class="ghost small" onclick={cancelAddConnection}>Cancel</button>
							</div>
						{:else}
							<div class="service-picked">
								{#if BRAND_ICONS[selectedService.id]}
									<BrandIcon service={selectedService.id} size={14} chip />
								{:else}
									{@const Fallback = FALLBACK_ICON[selectedService.id] ?? Globe}
									<Fallback size={14} strokeWidth={2} />
								{/if}
								{selectedService.label}
								<button class="service-change" onclick={() => (selectedService = null)}>Change</button>
							</div>
							<input
								class="connection-input"
								type="text"
								placeholder={selectedService.placeholder}
								bind:value={newUrl}
								maxlength="256"
							/>
							<div class="connection-form-actions">
								<button class="ghost small" onclick={cancelAddConnection}>Cancel</button>
								<button class="ghost small" onclick={submitConnection} disabled={savingConnection || !newUrl.trim()}>
									{savingConnection ? "Saving…" : "Save"}
								</button>
							</div>
						{/if}
					</div>
				{:else if connections.length < 5}
					<button class="add-connection" onclick={startAddConnection}>
						<Plus size={13} strokeWidth={2.5} />
						Add Connection
					</button>
				{/if}
			</div>

			<div class="info-block">
				<p class="info-label">Note (only visible to you)</p>
				<p class="note-placeholder">Notes aren't available yet</p>
			</div>
			</div>
		</div>
	</div>

	<div class="col-right">
		<div class="tabs">
			<button class="tab" class:active={boardTab === "board"} onclick={() => (boardTab = "board")}>Board</button>
			<button class="tab" class:active={boardTab === "activity"} onclick={() => (boardTab = "activity")}>Activity</button>
			<button class="tab" class:active={boardTab === "wishlist"} onclick={() => (boardTab = "wishlist")}>Wishlist</button>
		</div>

		{#if boardTab === "board"}
			{#if widgets.length === 0 && !pickingGame}
				<div class="widgets-empty">
					<h3>Customize your profile with Widgets</h3>
					<p>Choose from our library of Widgets to share more about yourself and your interests</p>
				</div>
			{/if}

			<div class="widgets-list">
				{#each widgets as widget (widget.id)}
					<div class="widget-card" class:pinned={widget.pinned}>
						<div class="widget-cover" style:background-image={widget.image_url ? `url(${api.resolveUrl(widget.image_url, session.token)})` : undefined}>
							{#if !widget.image_url}<Gamepad2 size={22} strokeWidth={1.5} />{/if}
						</div>
						<div class="widget-body">
							<span class="widget-kind">{WIDGET_KIND_LABELS[widget.kind]}</span>
							<span class="widget-title">{widget.title}</span>

							{#if editingDescriptionId === widget.id}
								<input
									class="widget-desc-input"
									type="text"
									placeholder="Add a note about this…"
									bind:value={descriptionDraft}
									maxlength="140"
									autofocus
									onblur={() => saveDescription(widget)}
									onkeydown={(e) => e.key === "Enter" && saveDescription(widget)}
								/>
							{:else}
								<button class="widget-desc" class:filled={!!widget.description} onclick={() => startEditDescription(widget)}>
									<Pencil size={13} strokeWidth={2} />
									<span>{widget.description || "Add a note about this…"}</span>
								</button>
							{/if}

							<div class="widget-tags">
								{#each widget.tags as tag (tag)}
									<span class="widget-tag">
										{tag}
										<button onclick={() => removeTag(widget, tag)} title="Remove tag">
											<X size={12} strokeWidth={2.5} />
										</button>
									</span>
								{/each}
								{#if addingTagId === widget.id}
									<input
										class="widget-tag-input"
										type="text"
										placeholder="Tag"
										bind:value={tagDraft}
										maxlength="24"
										autofocus
										onblur={() => saveTag(widget)}
										onkeydown={(e) => e.key === "Enter" && saveTag(widget)}
									/>
								{:else if widget.tags.length < 5}
									<button class="widget-tag-add" onclick={() => startAddTag(widget)}>
										<Plus size={12} strokeWidth={2.5} /> Tags
									</button>
								{/if}
							</div>
						</div>
						<div class="widget-actions">
							<button class="widget-pin" class:active={widget.pinned} onclick={() => togglePinned(widget)} title={widget.pinned ? "Unpin" : "Pin to top"}>
								<Pin size={13} strokeWidth={2} fill={widget.pinned ? "currentColor" : "none"} />
							</button>
							<button class="widget-remove" onclick={() => deleteWidget(widget.id)} title="Remove widget">
								<Trash2 size={13} strokeWidth={2} />
							</button>
						</div>
					</div>
				{/each}

				{#if widgets.length < 6}
					{#if pickingGame}
						<div class="game-picker">
							<div class="game-picker-search">
								<Search size={14} strokeWidth={2} />
								<input type="text" placeholder="Search" bind:value={gameSearch} maxlength="40" autofocus />
								<button class="game-picker-close" onclick={cancelAddWidget} title="Cancel"><X size={14} strokeWidth={2} /></button>
							</div>

							{#if gameSearch.trim()}
								<div class="game-picker-list">
									{#each filteredGames as game (game.id)}
										<button class="game-picker-item" onclick={() => selectGame(game)} disabled={savingWidget}>
											<img src={coverUrl(game.appid)} alt="" />
											<span>{game.name}</span>
										</button>
									{:else}
										<p class="game-picker-empty">No games found</p>
									{/each}
								</div>
							{:else}
								<div class="game-picker-suggested">
									<span class="game-picker-label">Suggested for you</span>
									<div class="game-picker-thumbs">
										{#each suggestedGames as game (game.id)}
											<button class="game-picker-thumb" onclick={() => selectGame(game)} disabled={savingWidget} title={game.name}>
												<img src={coverUrl(game.appid)} alt="" />
											</button>
										{/each}
									</div>
								</div>
							{/if}
						</div>
					{:else}
						<div class="widget-library">
							{#each WIDGET_KIND_OPTIONS as option (option.id)}
								<button class="widget-library-tile" onclick={() => chooseKind(option.id)}>
									<Plus size={22} strokeWidth={2} />
									<span>{option.label}</span>
								</button>
							{/each}
						</div>
					{/if}
				{/if}
			</div>
		{:else if boardTab === "activity"}
			<div class="widgets-empty">
				<h3>No activity yet</h3>
				<p>Live activity (games, music, apps) isn't tracked yet - this tab is a placeholder for now.</p>
			</div>
		{:else}
			<div class="widgets-empty">
				<h3>No wishlist yet</h3>
				<p>Wishlists aren't available yet - this tab is a placeholder for now.</p>
			</div>
		{/if}
	</div>
</div>
</div>

<style>
	.overlay {
		position: absolute;
		inset: 0;
		z-index: 300;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 40px;
		background: rgba(0, 0, 0, 0.6);
		backdrop-filter: blur(3px);
		-webkit-backdrop-filter: blur(3px);
	}

	.editor {
		--radius: 10px;
		--radius-sm: 6px;
		--gap-section: 26px;
		--gap-tight: 10px;
		--sidebar-w: 208px;

		position: relative;
		width: min(1060px, 100%);
		height: min(700px, 100%);
		display: grid;
		grid-template-columns: var(--sidebar-w) 1fr 1.2fr;
		background: var(--void);
		border-radius: 12px;
		overflow: hidden;
		box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
		font-family: var(--font-body);
	}

	.close {
		position: absolute;
		top: 16px;
		right: 16px;
		z-index: 10;
		padding: 8px;
		border-radius: 999px;
		color: var(--ink-dim);
		background: rgba(0, 0, 0, 0.25);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.close:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.col-sidebar {
		min-width: 0;
		background: var(--rail);
		border-right: 1px solid var(--hairline);
		padding: 28px 20px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		justify-content: flex-start;
		gap: var(--gap-section);
	}

	.nav-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		font-size: 15px;
		font-weight: 700;
		color: var(--ink);
		padding-bottom: 4px;
		border-bottom: 1px solid var(--hairline);
	}

	.section {
		display: flex;
		flex-direction: column;
		gap: var(--gap-tight);
	}

	.section-header-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.section-label {
		margin: 0;
		font-size: 13px;
		font-weight: 700;
		color: var(--ink-faint);
	}

	.icon-btn {
		padding: 3px;
		border-radius: var(--radius-sm);
		color: var(--ink-faint);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.icon-btn:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.nameplate-row,
	.slot-row {
		display: flex;
		gap: var(--gap-tight);
	}

	.nameplate-preview {
		flex: 1;
		height: 32px;
		border-radius: var(--radius-sm);
		opacity: 0.85;
	}

	.slot {
		width: 56px;
		height: 56px;
		border-radius: var(--radius);
		background: var(--panel);
		border: 1px solid var(--hairline);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--ink-faint);
		overflow: hidden;
		position: relative;
		flex-shrink: 0;
		transition: border-color 0.15s ease, color 0.15s ease;
	}

	.slot.small {
		width: 32px;
		height: 32px;
		border-radius: var(--radius-sm);
	}

	.slot:hover:not(:disabled) {
		border-color: var(--ink-dim);
		color: var(--ink-dim);
	}

	.slot-avatar {
		width: 100%;
		height: 100%;
		border-radius: inherit;
		background: var(--accent-fill) center/cover;
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		font-size: 14px;
	}

	.theme-slot {
		border: 2px solid transparent;
	}

	.theme-slot.active {
		border-color: var(--ink);
	}

	.theme-check {
		position: absolute;
		bottom: 4px;
		right: 4px;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: var(--ink);
		color: var(--void);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.nameplate-preview-text {
		font-family: var(--font-mono);
		font-weight: 700;
		font-size: 16px;
	}

	.style-popover {
		margin-top: 4px;
	}

	.col-center {
		min-width: 0;
		background: var(--sidebar);
		border-right: 1px solid var(--hairline);
		overflow-y: auto;
	}

	.preview-banner {
		height: 100px;
		background: var(--panel);
	}

	.preview-body {
		padding: 0 20px 28px;
		margin-top: -36px;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.preview-identity {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.preview-badges {
		display: flex;
	}

	.preview-info {
		display: flex;
		flex-direction: column;
		gap: 22px;
	}

	.preview-top-row {
		display: flex;
		align-items: flex-end;
		gap: 10px;
	}

	.preview-avatar {
		width: 72px;
		height: 72px;
		border-radius: 50%;
		background: var(--accent-fill) center/cover;
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		font-size: 22px;
	}

	.status-bubble {
		margin-bottom: 6px;
		padding: 6px 10px;
		border-radius: var(--radius);
		background: var(--panel);
		border: 1px solid var(--hairline);
		font-size: 12px;
		color: var(--ink-dim);
		max-width: 220px;
	}

	.preview-name {
		margin: 12px 0 0;
		display: flex;
		align-items: center;
		gap: 6px;
		font-weight: 700;
		font-size: 18px;
		color: var(--ink);
	}

	.preview-handle {
		margin: 0;
		font-size: 12px;
		color: var(--ink-faint);
	}

	.preview-actions {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-top: 8px;
	}

	.action {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 7px 12px;
		border-radius: var(--radius-sm);
		background: var(--panel);
		border: 1px solid var(--hairline);
		font-size: 12px;
		font-weight: 600;
		color: var(--ink-dim);
	}

	.action.primary {
		background: var(--active);
		color: var(--ink);
	}

	.action.icon-only {
		padding: 7px;
	}

	.action:disabled {
		opacity: 0.55;
		cursor: default;
	}

	.divider {
		height: 1px;
		background: var(--hairline);
		margin: 18px 0;
	}

	.info-block {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}

	.info-label {
		margin: 0;
		font-size: 13px;
		font-weight: 700;
		color: var(--ink-faint);
	}

	.info-value {
		margin: 0;
		font-size: 13px;
		color: var(--ink);
	}

	.bio-line {
		margin: 0;
		font-size: 13px;
		line-height: 1.5;
		color: var(--ink);
	}

	.add-connection {
		align-self: flex-start;
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 10px;
		border-radius: var(--radius-sm);
		background: var(--panel);
		border: 1px solid var(--hairline);
		font-size: 12px;
		font-weight: 600;
		color: var(--ink-dim);
		margin-top: 2px;
	}

	.add-connection:hover {
		color: var(--ink);
		border-color: var(--ink-dim);
	}

	.note-placeholder {
		margin: 0;
		font-size: 13px;
		font-style: italic;
		color: var(--ink-faint);
	}

	.col-right {
		min-width: 0;
		background: var(--sidebar);
		overflow-y: auto;
		padding: 20px;
	}

	.tabs {
		display: flex;
		gap: 20px;
		border-bottom: 1px solid var(--hairline);
		padding-bottom: 10px;
		margin-bottom: 32px;
	}

	.tab {
		font-size: 13px;
		font-weight: 700;
		color: var(--ink-faint);
		padding-bottom: 10px;
		margin-bottom: -11px;
		border-bottom: 2px solid transparent;
	}

	.tab.active {
		color: var(--ink);
		border-bottom-color: var(--ink);
	}

	.widgets-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: 4px;
		margin-top: 48px;
	}

	.widgets-empty h3 {
		margin: 0;
		font-size: 15px;
		font-weight: 700;
		color: var(--ink);
	}

	.widgets-empty p {
		margin: 0 0 18px;
		font-size: 12px;
		color: var(--ink-faint);
		max-width: 320px;
	}

	.widgets-list {
		width: 100%;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.widget-library {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 10px;
		width: 100%;
	}

	.widget-library-tile {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		min-height: 120px;
		padding: 24px 16px;
		border-radius: var(--radius);
		background: transparent;
		border: 1px dashed rgba(255, 255, 255, 0.08);
		color: var(--ink-faint);
		font-size: 13px;
		font-weight: 600;
		transition: border-color 0.15s ease, color 0.15s ease, background 0.15s ease;
	}

	.widget-library-tile:hover {
		border-color: var(--ink-dim);
		color: var(--ink-dim);
		background: var(--hover);
	}

	.widget-card {
		position: relative;
		display: flex;
		gap: 16px;
		padding: 16px;
		border-radius: var(--radius);
		background: transparent;
		border: 1px solid rgba(255, 255, 255, 0.08);
		transition: border-color 0.15s ease, background 0.15s ease;
	}

	.widget-card:hover {
		background: rgba(255, 255, 255, 0.02);
		border-color: var(--ink-faint);
	}

	.widget-card.pinned {
		border-color: var(--accent-fill);
	}

	.widget-cover {
		flex-shrink: 0;
		width: 84px;
		height: 120px;
		border-radius: var(--radius-sm);
		background: var(--void) center/cover;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--ink-faint);
		overflow: hidden;
	}

	.widget-body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding-right: 48px;
	}

	.widget-kind {
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--ink-faint);
	}

	.widget-title {
		font-size: 15px;
		font-weight: 700;
		color: var(--ink);
	}

	.widget-desc {
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--ink-faint);
		transition: color 0.15s ease;
		text-align: left;
	}

	.widget-desc.filled {
		color: var(--ink);
	}

	.widget-desc:hover {
		color: var(--ink);
	}

	.widget-desc span {
		font-size: 13px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.widget-desc-input,
	.widget-tag-input {
		padding: 6px 9px;
		border-radius: var(--radius-sm);
		background: var(--void);
		border: 1px solid var(--hairline);
		color: var(--ink);
		font-size: 13px;
	}

	.widget-desc-input {
		width: 100%;
	}

	.widget-tag-input {
		width: 70px;
	}

	.widget-tags {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
		margin-top: 2px;
	}

	.widget-tag {
		display: flex;
		align-items: center;
		gap: 5px;
		padding: 4px 10px;
		border-radius: 999px;
		background: var(--void);
		border: 1px solid var(--hairline);
		color: var(--ink-dim);
		font-size: 12px;
		font-weight: 600;
	}

	.widget-tag button {
		display: flex;
		color: var(--ink-faint);
		transition: color 0.15s ease;
	}

	.widget-tag button:hover {
		color: var(--danger);
	}

	.widget-tag-add {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 4px 10px;
		border-radius: 999px;
		border: 1px dashed var(--hairline);
		color: var(--ink-faint);
		font-size: 12px;
		font-weight: 600;
		transition: color 0.15s ease, border-color 0.15s ease;
	}

	.widget-tag-add:hover {
		color: var(--ink-dim);
		border-color: var(--ink-dim);
	}

	.widget-actions {
		position: absolute;
		top: 8px;
		right: 8px;
		display: flex;
		gap: 4px;
		opacity: 0;
		transition: opacity 0.15s ease;
	}

	.widget-card:hover .widget-actions,
	.widget-card.pinned .widget-actions {
		opacity: 1;
	}

	.widget-pin,
	.widget-remove {
		padding: 4px;
		border-radius: 999px;
		color: var(--ink-faint);
		transition: color 0.15s ease;
	}

	.widget-pin:hover {
		color: var(--ink);
	}

	.widget-pin.active {
		color: var(--accent-fill);
	}

	.widget-remove:hover {
		color: var(--danger);
	}

	.game-picker {
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 10px;
		border-radius: var(--radius);
		background: var(--panel);
		border: 1px solid var(--hairline);
	}

	.game-picker-search {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 10px;
		border-radius: var(--radius-sm);
		background: var(--void);
		border: 1px solid var(--hairline);
		color: var(--ink-faint);
	}

	.game-picker-search input {
		flex: 1;
		min-width: 0;
		background: transparent;
		border: none;
		color: var(--ink);
		font-size: 12px;
	}

	.game-picker-search input::placeholder {
		color: var(--ink-faint);
	}

	.game-picker-search input:focus {
		outline: none;
	}

	.game-picker-close {
		flex-shrink: 0;
		display: flex;
		color: var(--ink-faint);
		transition: color 0.15s ease;
	}

	.game-picker-close:hover {
		color: var(--ink);
	}

	.game-picker-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
		max-height: 220px;
		overflow-y: auto;
	}

	.game-picker-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 6px 8px;
		border-radius: var(--radius-sm);
		color: var(--ink-dim);
		font-size: 12px;
		text-align: left;
		transition: background 0.15s ease, color 0.15s ease;
	}

	.game-picker-item:hover:not(:disabled) {
		background: var(--hover);
		color: var(--ink);
	}

	.game-picker-item img {
		flex-shrink: 0;
		width: 24px;
		height: 32px;
		border-radius: 3px;
		object-fit: cover;
	}

	.game-picker-empty {
		margin: 6px 0;
		font-size: 12px;
		color: var(--ink-faint);
		text-align: center;
	}

	.game-picker-label {
		font-size: 11px;
		font-weight: 700;
		color: var(--ink-faint);
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}

	.game-picker-thumbs {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		margin-top: 8px;
	}

	.game-picker-thumb {
		width: 56px;
		height: 80px;
		border-radius: var(--radius-sm);
		overflow: hidden;
		transition: transform 0.15s ease;
	}

	.game-picker-thumb:hover:not(:disabled) {
		transform: translateY(-2px);
	}

	.game-picker-thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.ghost {
		padding: 6px 10px;
		border-radius: var(--radius-sm);
		color: var(--ink-dim);
		font-weight: 600;
		font-size: 11px;
		background: var(--panel);
		border: 1px solid var(--hairline);
	}

	.ghost:hover:not(:disabled) {
		color: var(--ink);
		border-color: var(--ink-dim);
	}

	.ghost:disabled {
		opacity: 0.5;
	}

	.connection-row {
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--ink-dim);
	}

	.connection-label {
		font-size: 13px;
		color: var(--ink);
		font-weight: 600;
	}

	.connection-label:hover {
		text-decoration: underline;
	}

	.connection-remove {
		margin-left: auto;
		padding: 4px;
		border-radius: var(--radius-sm);
		color: var(--ink-faint);
		display: flex;
	}

	.connection-remove:hover {
		color: var(--danger);
		background: var(--hover);
	}

	.connection-form {
		display: flex;
		flex-direction: column;
		gap: 6px;
		margin-top: 2px;
		padding: 10px;
		border-radius: var(--radius-sm);
		background: var(--panel);
		border: 1px solid var(--hairline);
	}

	.connection-input {
		width: 100%;
		padding: 6px 8px;
		border-radius: var(--radius-sm);
		background: var(--void);
		border: 1px solid var(--hairline);
		color: var(--ink);
		font-size: 12px;
	}

	.connection-form-actions {
		display: flex;
		justify-content: flex-end;
		gap: 6px;
		margin-top: 2px;
	}

	.service-search {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 8px;
		border-radius: var(--radius-sm);
		background: var(--void);
		border: 1px solid var(--hairline);
		color: var(--ink-faint);
	}

	.service-search input {
		flex: 1;
		background: none;
		border: none;
		color: var(--ink);
		font-size: 12px;
	}

	.service-search input:focus {
		outline: none;
	}

	.service-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
		max-height: 180px;
		overflow-y: auto;
	}

	.service-option {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		border-radius: var(--radius-sm);
		color: var(--ink-dim);
		font-size: 12px;
		text-align: left;
	}

	.service-option:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.service-empty {
		margin: 6px 0;
		font-size: 12px;
		color: var(--ink-faint);
		text-align: center;
	}

	.service-picked {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 12px;
		font-weight: 600;
		color: var(--ink);
	}

	.service-change {
		margin-left: auto;
		font-size: 11px;
		font-weight: 600;
		color: var(--ink-faint);
		text-decoration: underline;
	}

	.service-change:hover {
		color: var(--ink);
	}
</style>
