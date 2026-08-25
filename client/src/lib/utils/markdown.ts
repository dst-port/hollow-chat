function escapeHtml(text: string): string {
	return text
		.replace(/&/g, "&amp;")
		.replace(/</g, "&lt;")
		.replace(/>/g, "&gt;")
		.replace(/"/g, "&quot;")
		.replace(/'/g, "&#39;");
}

const BLOCK_MARK = "";
const SPAN_MARK = "";

export function renderMarkdown(raw: string): string {
	let text = escapeHtml(raw);

	const blocks: string[] = [];
	text = text.replace(/```([\s\S]*?)```/g, (_match, code: string) => {
		blocks.push(`<pre class="md-block"><code>${code.trim()}</code></pre>`);
		return `${BLOCK_MARK}${blocks.length - 1}${BLOCK_MARK}`;
	});

	const spans: string[] = [];
	text = text.replace(/`([^`\n]+)`/g, (_match, code: string) => {
		spans.push(`<code class="md-inline">${code}</code>`);
		return `${SPAN_MARK}${spans.length - 1}${SPAN_MARK}`;
	});

	text = text.replace(/\*\*\*([^*]+)\*\*\*/g, "<strong><em>$1</em></strong>");
	text = text.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
	text = text.replace(/(?<![*\w])\*([^*\n]+)\*(?!\w)/g, "<em>$1</em>");
	text = text.replace(/(?<![_\w])_([^_\n]+)_(?!\w)/g, "<em>$1</em>");
	text = text.replace(/__([^_]+)__/g, "<u>$1</u>");
	text = text.replace(/~~([^~]+)~~/g, "<del>$1</del>");
	text = text.replace(
		/\|\|([^|]+)\|\|/g,
		'<span class="md-spoiler" onclick="this.classList.toggle(\'revealed\')">$1</span>'
	);

	text = text.replace(
		/(https?:\/\/[^\s<>"']+)/g,
		'<a href="$1" class="md-link" target="_blank" rel="noreferrer">$1</a>'
	);

	text = text.replace(/\n/g, "<br />");

	text = text.replace(
		new RegExp(`${SPAN_MARK}(\\d+)${SPAN_MARK}`, "g"),
		(_match, index: string) => spans[Number(index)]
	);
	text = text.replace(
		new RegExp(`${BLOCK_MARK}(\\d+)${BLOCK_MARK}`, "g"),
		(_match, index: string) => blocks[Number(index)]
	);

	return text;
}
