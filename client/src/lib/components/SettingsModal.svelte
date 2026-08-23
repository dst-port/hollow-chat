<script lang="ts">
	import X from "@lucide/svelte/icons/x";
	import UserRound from "@lucide/svelte/icons/user-round";
	import ShieldCheck from "@lucide/svelte/icons/shield-check";
	import LogOut from "@lucide/svelte/icons/log-out";

	let { username, onClose, onLogout }: {
		username: string;
		onClose: () => void;
		onLogout: () => void;
	} = $props();

	type Section = "account" | "privacy";
	let section = $state<Section>("account");

	function onKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") onClose();
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="overlay" role="presentation" onclick={onClose}>
	<div
		class="modal"
		role="dialog"
		aria-modal="true"
		aria-label="User settings"
		tabindex="-1"
		onclick={(e) => e.stopPropagation()}
		onkeydown={(e) => e.stopPropagation()}
	>
		<nav class="nav">
			<p class="nav-label">User Settings</p>
			<button class="nav-item" class:active={section === "account"} onclick={() => (section = "account")}>
				<UserRound size={16} strokeWidth={2} />
				My Account
			</button>
			<button class="nav-item" class:active={section === "privacy"} onclick={() => (section = "privacy")}>
				<ShieldCheck size={16} strokeWidth={2} />
				Privacy
			</button>

			<div class="nav-spacer"></div>

			<button class="nav-item danger" onclick={onLogout}>
				<LogOut size={16} strokeWidth={2} />
				Log Out
			</button>
		</nav>

		<div class="content">
			<button class="close" onclick={onClose} title="Close">
				<X size={20} strokeWidth={2} />
			</button>

			{#if section === "account"}
				<h2>My Account</h2>

				<div class="card">
					<div class="identity">
						<div class="avatar">{username.slice(0, 2).toUpperCase()}</div>
						<div>
							<p class="username">{username}</p>
							<p class="hint">HollowChat account</p>
						</div>
					</div>

					<div class="row">
						<div>
							<p class="row-label">Username</p>
							<p class="row-value">{username}</p>
						</div>
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

				<div class="card">
					<p class="row-label">Password recovery</p>
					<p class="row-value muted">
						There is no email or phone number on file, so there is no password reset. Losing
						your password means losing the account.
					</p>
				</div>
			{:else}
				<h2>Privacy</h2>

				<div class="card">
					<div class="row">
						<div>
							<p class="row-label">Data collected</p>
							<p class="row-value muted">Username and a password hash. Nothing else.</p>
						</div>
					</div>
					<div class="row">
						<div>
							<p class="row-label">Message storage</p>
							<p class="row-value muted">
								End-to-end encrypted. The server only ever sees ciphertext.
							</p>
						</div>
					</div>
					<div class="row">
						<div>
							<p class="row-label">IP logging</p>
							<p class="row-value muted">Never logged, on any layer of the stack.</p>
						</div>
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>

<style>
	.overlay {
		position: fixed;
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
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-display);
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
</style>
