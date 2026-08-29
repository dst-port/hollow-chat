<script lang="ts">
	import { fly } from "svelte/transition";
	import { cubicOut } from "svelte/easing";
	import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
	import Check from "@lucide/svelte/icons/check";
	import Copy from "@lucide/svelte/icons/copy";
	import { register, login, completeTotpLogin, ApiError } from "$lib/api/client";
	import { session } from "$lib/stores/session.svelte";
	import Logo from "$lib/components/Logo.svelte";
	import { t } from "$lib/i18n/index.svelte";

	type Mode = "login" | "register" | "reveal" | "totp";

	let mode = $state<Mode>("login");
	let username = $state("");
	let password = $state("");
	let error = $state("");
	let loading = $state(false);
	let revealedPassword = $state("");
	let revealedUsername = $state("");
	let confirmed = $state(false);
	let copied = $state(false);
	let challengeId = $state("");
	let totpCode = $state("");
	let pendingUsername = $state("");

	async function copyPassword() {
		await navigator.clipboard.writeText(revealedPassword);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}

	async function submitLogin(event: SubmitEvent) {
		event.preventDefault();
		error = "";
		loading = true;
		try {
			const result = await login(username, password);
			if (result.requires_totp && result.challenge_id) {
				challengeId = result.challenge_id;
				pendingUsername = username;
				totpCode = "";
				mode = "totp";
			} else if (result.token) {
				session.set(result.token, username);
			}
		} catch (err) {
			error = err instanceof ApiError ? err.message : t("auth.error.generic");
		} finally {
			loading = false;
		}
	}

	async function submitTotp(event: SubmitEvent) {
		event.preventDefault();
		error = "";
		loading = true;
		try {
			const result = await completeTotpLogin(challengeId, totpCode.trim());
			if (result.token) session.set(result.token, pendingUsername);
		} catch (err) {
			error = err instanceof ApiError ? err.message : t("auth.error.invalidCode");
		} finally {
			loading = false;
		}
	}

	async function submitRegister(event: SubmitEvent) {
		event.preventDefault();
		error = "";
		loading = true;
		try {
			const result = await register(username);
			revealedUsername = result.username;
			revealedPassword = result.password;
			confirmed = false;
			mode = "reveal";
		} catch (err) {
			error = err instanceof ApiError ? err.message : t("auth.error.generic");
		} finally {
			loading = false;
		}
	}

	async function continueAfterReveal() {
		error = "";
		loading = true;
		try {
			const result = await login(revealedUsername, revealedPassword);
			if (result.requires_totp && result.challenge_id) {
				challengeId = result.challenge_id;
				pendingUsername = revealedUsername;
				totpCode = "";
				mode = "totp";
			} else if (result.token) {
				session.set(result.token, revealedUsername, true);
			}
		} catch (err) {
			error = err instanceof ApiError ? err.message : t("auth.error.generic");
		} finally {
			loading = false;
		}
	}

	function switchMode(next: Mode) {
		mode = next;
		error = "";
		username = "";
		password = "";
	}
</script>

<div class="window-frame screen">
	<div class="card">
		<div class="brand">
			<span class="mark"><Logo size={22} /></span>
			<span class="name">HollowChat</span>
		</div>

		{#key mode}
			<div class="pane" in:fly={{ y: 8, duration: 260, easing: cubicOut }}>
				{#if mode === "login"}
					<h1>{t("auth.login.title")}</h1>
					<p class="subtitle">{t("auth.login.subtitle")}</p>

					<form onsubmit={submitLogin}>
						<label>
							{t("auth.field.username")}
							<input type="text" bind:value={username} autocomplete="username" required />
						</label>
						<label>
							{t("auth.field.password")}
							<input
								type="password"
								bind:value={password}
								autocomplete="current-password"
								required
							/>
						</label>

						{#if error}<p class="error">{error}</p>{/if}

						<button type="submit" disabled={loading}>
							{loading ? t("auth.login.submitting") : t("auth.login.submit")}
						</button>
					</form>

					<p class="switch">
						{t("auth.login.switchPrompt")}
						<button type="button" class="link" onclick={() => switchMode("register")}>
							{t("auth.login.switchAction")}
						</button>
					</p>
				{:else if mode === "register"}
					<h1>{t("auth.register.title")}</h1>
					<p class="subtitle">{t("auth.register.subtitle")}</p>

					<form onsubmit={submitRegister}>
						<label>
							{t("auth.field.username")}
							<input
								type="text"
								bind:value={username}
								autocomplete="off"
								required
								minlength="3"
								maxlength="32"
							/>
						</label>

						{#if error}<p class="error">{error}</p>{/if}

						<button type="submit" disabled={loading}>
							{loading ? t("auth.register.submitting") : t("auth.register.submit")}
						</button>
					</form>

					<p class="switch">
						{t("auth.register.switchPrompt")}
						<button type="button" class="link" onclick={() => switchMode("login")}>
							{t("auth.register.switchAction")}
						</button>
					</p>
				{:else if mode === "totp"}
					<h1>{t("auth.totp.title")}</h1>
					<p class="subtitle">{t("auth.totp.subtitle")}</p>

					<form onsubmit={submitTotp}>
						<label>
							{t("auth.field.code")}
							<input
								type="text"
								bind:value={totpCode}
								autocomplete="one-time-code"
								placeholder="123456"
								required
							/>
						</label>

						{#if error}<p class="error">{error}</p>{/if}

						<button type="submit" disabled={loading || !totpCode.trim()}>
							{loading ? t("auth.totp.submitting") : t("auth.totp.submit")}
						</button>
					</form>

					<p class="switch">
						<button type="button" class="link" onclick={() => switchMode("login")}>
							{t("auth.totp.back")}
						</button>
					</p>
				{:else}
					<h1>{t("auth.reveal.title")}</h1>
					<p class="subtitle warning">
						<TriangleAlert size={14} strokeWidth={2.5} />
						{t("auth.reveal.warning")}
					</p>

					<div class="password-box">
						<code>{revealedPassword}</code>
						<button type="button" class="copy" onclick={copyPassword} title={t("common.copy")}>
							{#if copied}
								<Check size={15} strokeWidth={2.5} />
							{:else}
								<Copy size={15} strokeWidth={2} />
							{/if}
						</button>
					</div>

					<label class="confirm">
						<input type="checkbox" bind:checked={confirmed} />
						<span class="checkbox">
							{#if confirmed}<Check size={12} strokeWidth={3} />{/if}
						</span>
						{t("auth.reveal.confirm")}
					</label>

					{#if error}<p class="error">{error}</p>{/if}

					<button type="button" disabled={!confirmed || loading} onclick={continueAfterReveal}>
						{loading ? t("auth.reveal.continuing") : t("auth.reveal.continue")}
					</button>
				{/if}
			</div>
		{/key}
	</div>
</div>

<style>
	.screen {
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.card {
		width: 380px;
		background: var(--sidebar);
		border: 1px solid var(--hairline);
		border-radius: 12px;
		padding: 32px;
		overflow: hidden;
	}

	.pane {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-bottom: 24px;
	}

	.mark {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border-radius: 10px;
		background: var(--accent-soft);
		color: var(--accent-fill);
	}

	.brand .name {
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 15px;
		letter-spacing: 0.01em;
	}

	h1 {
		margin: 0 0 4px;
		font-family: var(--font-display);
		font-weight: 700;
		font-size: 19px;
	}

	.subtitle {
		margin: 0 0 20px;
		color: var(--ink-dim);
		font-size: 13px;
		line-height: 1.5;
	}

	.subtitle.warning {
		display: flex;
		gap: 8px;
		align-items: flex-start;
		color: var(--idle);
	}

	.subtitle.warning :global(svg) {
		flex-shrink: 0;
		margin-top: 2px;
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	label {
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

	input[type="text"],
	input[type="password"] {
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 10px 12px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 14px;
		transition: border-color 0.15s ease, box-shadow 0.15s ease;
	}

	input[type="text"]:focus,
	input[type="password"]:focus {
		outline: none;
		border-color: var(--ink-dim);
		box-shadow: 0 0 0 3px var(--accent-soft);
	}

	button {
		transition: background-color 0.15s ease, color 0.15s ease, box-shadow 0.15s ease,
			transform 0.05s ease;
	}

	button:active:not(:disabled) {
		transform: scale(0.98);
	}

	button[type="submit"] {
		margin-top: 4px;
		padding: 10px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-weight: 700;
		font-size: 14px;
	}

	button[type="submit"]:hover:not(:disabled) {
		box-shadow: 0 0 0 3px var(--accent-soft);
	}

	button[type="submit"]:disabled {
		background: var(--active);
		color: var(--ink-faint);
	}

	.switch {
		margin: 20px 0 0;
		font-size: 13px;
		color: var(--ink-dim);
		text-align: center;
	}

	.link {
		color: var(--ink);
		font-weight: 600;
	}

	.link:hover {
		color: var(--ink);
	}

	.error {
		margin: 0;
		color: var(--danger);
		font-size: 13px;
	}

	.password-box {
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 14px 12px 14px 16px;
		margin-bottom: 16px;
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

	.confirm {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: 10px;
		text-transform: none;
		font-family: var(--font-body);
		font-weight: 500;
		font-size: 13px;
		color: var(--ink);
		margin-bottom: 16px;
		cursor: pointer;
	}

	.confirm input[type="checkbox"] {
		position: absolute;
		width: 1px;
		height: 1px;
		opacity: 0;
	}

	.checkbox {
		flex-shrink: 0;
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

	.confirm input[type="checkbox"]:checked + .checkbox {
		background: var(--accent-fill);
		border-color: var(--accent-fill);
	}

	.confirm input[type="checkbox"]:focus-visible + .checkbox {
		box-shadow: 0 0 0 3px var(--accent-soft);
	}

	button[type="button"]:not(.link):not(.copy) {
		padding: 10px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-weight: 700;
		font-size: 14px;
	}

	button[type="button"]:not(.link):not(.copy):hover:not(:disabled) {
		box-shadow: 0 0 0 3px var(--accent-soft);
	}

	button[type="button"]:not(.link):not(.copy):disabled {
		background: var(--active);
		color: var(--ink-faint);
	}
</style>
