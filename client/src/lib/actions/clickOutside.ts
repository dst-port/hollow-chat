export function clickOutside(node: HTMLElement, callback: () => void) {
	function handleClick(event: MouseEvent) {
		if (!node.contains(event.target as Node)) {
			callback();
		}
	}

	document.addEventListener("mousedown", handleClick, true);

	return {
		destroy() {
			document.removeEventListener("mousedown", handleClick, true);
		}
	};
}
