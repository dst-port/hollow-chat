<script lang="ts">
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
					<input type="password" bind:value={password} autocomplete="current-password" required />
				</label>

				{#if error}<p class="error">{error}</p>{/if}

				<button type="submit" disabled={loading}>
					{loading ? "Logging in…" : "Log in"}
				</button>
			</form>

			<p class="switch">
				Don't have an account?
				<button type="button" class="link" onclick={() => switchMode("register")}>Register</button>
			</p>
		{:else if mode === "register"}
			<h1>Create an account</h1>
			<p class="subtitle">Just a username — no email, no phone.</p>

			<form onsubmit={submitRegister}>
				<label>
					Username
					<input type="text" bind:value={username} autocomplete="off" required minlength="3" maxlength="32" />
				</label>

				{#if error}<p class="error">{error}</p>{/if}

				<button type="submit" disabled={loading}>
					{loading ? "Creating…" : "Create account"}
				</button>
			</form>

			<p class="switch">
				Already have an account?
				<button type="button" class="link" onclick={() => switchMode("login")}>Log in</button>
			</p>
		{:else}
			<h1>Save your password now</h1>
			<p class="subtitle warning">
				This is the only time we'll show it. There is no email or phone number to recover it —
				if you lose it, the account is gone for good.
			</p>

			<div class="password-box">
				<code>{revealedPassword}</code>
			</div>

			<label class="confirm">
				<input type="checkbox" bind:checked={confirmed} />
				I saved my password
			</label>

			{#if error}<p class="error">{error}</p>{/if}

			<button type="button" disabled={!confirmed || loading} onclick={continueAfterReveal}>
				{loading ? "Continuing…" : "Continue"}
			</button>
		{/if}
	</div>
</div>

<style>
	.screen {
		height: 100vh;
		width: 100vw;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--bg-rail);
	}

	.card {
		width: 380px;
		background: var(--bg-sidebar);
		border-radius: 12px;
		padding: 32px;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-bottom: 20px;
	}

	.mark {
		width: 32px;
		height: 32px;
		border-radius: 10px;
		background: var(--accent-soft);
		color: var(--accent);
		font-weight: 700;
		font-size: 13px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.brand .name {
		font-weight: 700;
		font-size: 16px;
	}

	h1 {
		margin: 0 0 4px;
		font-size: 20px;
	}

	.subtitle {
		margin: 0 0 20px;
		color: var(--text-muted);
		font-size: 13px;
		line-height: 1.5;
	}

	.subtitle.warning {
		color: #fbbf24;
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
		font-size: 12px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.02em;
		color: var(--text-muted);
	}

	input[type="text"],
	input[type="password"] {
		background: var(--bg-main);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 10px 12px;
		color: var(--text-primary);
		font-size: 14px;
	}

	input[type="text"]:focus,
	input[type="password"]:focus {
		outline: none;
		border-color: var(--accent);
	}

	button[type="submit"] {
		margin-top: 4px;
		padding: 10px;
		border-radius: 6px;
		background: var(--accent);
		color: white;
		font-weight: 600;
		font-size: 14px;
	}

	button[type="submit"]:disabled {
		background: var(--bg-active);
		color: var(--text-faint);
	}

	.switch {
		margin: 20px 0 0;
		font-size: 13px;
		color: var(--text-muted);
		text-align: center;
	}

	.link {
		color: var(--accent);
		font-weight: 600;
	}

	.error {
		margin: 0;
		color: var(--danger);
		font-size: 13px;
	}

	.password-box {
		background: var(--bg-main);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 16px;
		margin-bottom: 16px;
		text-align: center;
	}

	.password-box code {
		font-size: 16px;
		font-weight: 600;
		letter-spacing: 0.03em;
		word-break: break-all;
	}

	.confirm {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: 8px;
		text-transform: none;
		font-weight: 500;
		color: var(--text-primary);
		margin-bottom: 16px;
	}

	button[type="button"]:not(.link) {
		padding: 10px;
		border-radius: 6px;
		background: var(--accent);
		color: white;
		font-weight: 600;
		font-size: 14px;
	}

	button[type="button"]:not(.link):disabled {
		background: var(--bg-active);
		color: var(--text-faint);
	}
</style>
