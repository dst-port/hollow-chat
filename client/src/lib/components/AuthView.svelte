<script lang="ts">
	import { fly } from "svelte/transition";
	import { cubicOut } from "svelte/easing";
	import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
	import Check from "@lucide/svelte/icons/check";
	import Copy from "@lucide/svelte/icons/copy";
	import { register, login, ApiError } from "$lib/api/client";
	import { session } from "$lib/stores/session.svelte";

	type Mode = "login" | "register" | "reveal";

	let mode = $state<Mode>("login");
	let username = $state("");
	let password = $state("");
	let error = $state("");
	let loading = $state(false);
	let revealedPassword = $state("");
	let revealedUsername = $state("");
	let confirmed = $state(false);
	let copied = $state(false);

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
			session.set(result.token, username);
		} catch (err) {
			error = err instanceof ApiError ? err.message : "something went wrong";
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
			error = err instanceof ApiError ? err.message : "something went wrong";
		} finally {
			loading = false;
		}
	}

	async function continueAfterReveal() {
		error = "";
		loading = true;
		try {
			const result = await login(revealedUsername, revealedPassword);
			session.set(result.token, revealedUsername);
		} catch (err) {
			error = err instanceof ApiError ? err.message : "something went wrong";
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

<div class="screen">
	<div class="card">
		<div class="brand">
			<span class="mark">HC</span>
			<span class="name">HollowChat</span>
		</div>

		{#key mode}
			<div class="pane" in:fly={{ y: 8, duration: 260, easing: cubicOut }}>
				{#if mode === "login"}
					<h1>Welcome back</h1>
					<p class="subtitle">Log in with your username and password.</p>

					<form onsubmit={submitLogin}>
						<label>
							Username
							<input type="text" bind:value={username} autocomplete="username" required />
						</label>
						<label>
							Password
							<input
								type="password"
								bind:value={password}
								autocomplete="current-password"
								required
							/>
						</label>

						{#if error}<p class="error">{error}</p>{/if}

						<button type="submit" disabled={loading}>
							{loading ? "Logging in…" : "Log in"}
						</button>
					</form>

					<p class="switch">
						Don't have an account?
						<button type="button" class="link" onclick={() => switchMode("register")}>
							Register
						</button>
					</p>
				{:else if mode === "register"}
					<h1>Create an account</h1>
					<p class="subtitle">Just a username — no email, no phone.</p>

					<form onsubmit={submitRegister}>
						<label>
							Username
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
							{loading ? "Creating…" : "Create account"}
						</button>
					</form>

					<p class="switch">
						Already have an account?
						<button type="button" class="link" onclick={() => switchMode("login")}>
							Log in
						</button>
					</p>
				{:else}
					<h1>Save your password now</h1>
					<p class="subtitle warning">
						<TriangleAlert size={14} strokeWidth={2.5} />
						This is the only time we'll show it. There is no email or phone number to recover
						it — if you lose it, the account is gone for good.
					</p>

					<div class="password-box">
						<code>{revealedPassword}</code>
						<button type="button" class="copy" onclick={copyPassword} title="Copy password">
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
						I saved my password
					</label>

					{#if error}<p class="error">{error}</p>{/if}

					<button type="button" disabled={!confirmed || loading} onclick={continueAfterReveal}>
						{loading ? "Continuing…" : "Continue"}
					</button>
				{/if}
			</div>
		{/key}
	</div>
</div>

<style>
	.screen {
		height: 100vh;
		width: 100vw;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--void);
		background-image: radial-gradient(circle at 50% 30%, rgba(156, 147, 194, 0.06), transparent 60%);
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
		width: 32px;
		height: 32px;
		border-radius: 10px;
		background: var(--wraith-soft);
		color: var(--wraith);
		font-family: var(--font-display);
		font-weight: 700;
		font-size: 12px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.brand .name {
		font-family: var(--font-display);
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
		border-color: var(--wraith);
		box-shadow: 0 0 0 3px var(--wraith-soft);
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
		background: var(--ember);
		color: var(--void);
		font-weight: 700;
		font-size: 14px;
	}

	button[type="submit"]:hover:not(:disabled) {
		box-shadow: 0 0 0 3px var(--ember-soft);
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
		color: var(--wraith);
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
		color: var(--ember);
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
		background: var(--ember);
		border-color: var(--ember);
	}

	.confirm input[type="checkbox"]:focus-visible + .checkbox {
		box-shadow: 0 0 0 3px var(--wraith-soft);
	}

	button[type="button"]:not(.link):not(.copy) {
		padding: 10px;
		border-radius: 6px;
		background: var(--ember);
		color: var(--void);
		font-weight: 700;
		font-size: 14px;
	}

	button[type="button"]:not(.link):not(.copy):hover:not(:disabled) {
		box-shadow: 0 0 0 3px var(--ember-soft);
	}

	button[type="button"]:not(.link):not(.copy):disabled {
		background: var(--active);
		color: var(--ink-faint);
	}
</style>
