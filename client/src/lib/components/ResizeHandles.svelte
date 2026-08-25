<script lang="ts">
	import { getCurrentWindow } from "@tauri-apps/api/window";

	const appWindow = getCurrentWindow();

	const edges = [
		{ dir: "North", class: "edge-n" },
		{ dir: "South", class: "edge-s" },
		{ dir: "East", class: "edge-e" },
		{ dir: "West", class: "edge-w" },
		{ dir: "NorthEast", class: "corner-ne" },
		{ dir: "NorthWest", class: "corner-nw" },
		{ dir: "SouthEast", class: "corner-se" },
		{ dir: "SouthWest", class: "corner-sw" },
	] as const;

	function startResize(direction: (typeof edges)[number]["dir"]) {
		return (event: MouseEvent) => {
			if (event.button !== 0) return;
			appWindow.startResizeDragging(direction);
		};
	}
</script>

{#each edges as edge (edge.dir)}
	<div
		role="presentation"
		class="handle {edge.class}"
		onmousedown={startResize(edge.dir)}
	></div>
{/each}

<style>
	.handle {
		position: fixed;
		z-index: 9999;
	}

	.edge-n,
	.edge-s {
		left: 8px;
		right: 8px;
		height: 4px;
		cursor: ns-resize;
	}

	.edge-n {
		top: 0;
	}

	.edge-s {
		bottom: 0;
	}

	.edge-e,
	.edge-w {
		top: 8px;
		bottom: 8px;
		width: 4px;
		cursor: ew-resize;
	}

	.edge-w {
		left: 0;
	}

	.edge-e {
		right: 0;
	}

	.corner-ne,
	.corner-nw,
	.corner-se,
	.corner-sw {
		width: 8px;
		height: 8px;
	}

	.corner-nw {
		top: 0;
		left: 0;
		cursor: nwse-resize;
	}

	.corner-se {
		bottom: 0;
		right: 0;
		cursor: nwse-resize;
	}

	.corner-ne {
		top: 0;
		right: 0;
		cursor: nesw-resize;
	}

	.corner-sw {
		bottom: 0;
		left: 0;
		cursor: nesw-resize;
	}
</style>
