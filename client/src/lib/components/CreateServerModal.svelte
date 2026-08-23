<script lang="ts">
	import Modal from "$lib/components/Modal.svelte";

	let { onClose, onCreate }: {
		onClose: () => void;
		onCreate: (name: string) => void;
	} = $props();

	let name = $state("");

	function submit(event: SubmitEvent) {
		event.preventDefault();
		const trimmed = name.trim();
		if (!trimmed) return;
		onCreate(trimmed);
	}
</script>

<Modal title="Create a server" {onClose}>
	<form onsubmit={submit}>
		<p class="hint">Give your server a name. You can invite people once it exists.</p>
		<label>
			Server name
			<input type="text" bind:value={name} required maxlength="48" placeholder="Void Raiders" />
		</label>
		<button type="submit" disabled={!name.trim()}>Create</button>
	</form>
</Modal>

<style>
	form {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.hint {
		margin: 0;
		font-size: 13px;
		color: var(--ink-dim);
		line-height: 1.5;
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

	input {
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 10px 12px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 14px;
	}

	input:focus {
		outline: none;
		border-color: var(--ink-dim);
	}

	button[type="submit"] {
		padding: 10px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-weight: 700;
		font-size: 14px;
	}

	button[type="submit"]:disabled {
		background: var(--active);
		color: var(--ink-faint);
	}
</style>
