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

const DOMAIN_COLORS: { host: string; color: string }[] = [
	{ host: "youtube.com", color: "#ff0033" },
	{ host: "youtu.be", color: "#ff0033" },
	{ host: "roblox.com", color: "#00a2ff" },
	{ host: "twitch.tv", color: "#9146ff" },
	{ host: "twitter.com", color: "#1d9bf0" },
	{ host: "x.com", color: "#1d9bf0" },
	{ host: "discord.com", color: "#5865f2" },
	{ host: "discord.gg", color: "#5865f2" },
	{ host: "github.com", color: "#8b949e" },
	{ host: "reddit.com", color: "#ff4500" },
	{ host: "spotify.com", color: "#1db954" },
	{ host: "soundcloud.com", color: "#ff7700" },
	{ host: "instagram.com", color: "#e1306c" },
	{ host: "tiktok.com", color: "#25f4ee" },
	{ host: "steampowered.com", color: "#66c0f4" },
	{ host: "telegram.org", color: "#2aabee" },
	{ host: "t.me", color: "#2aabee" }
];

function hostnameOf(url: string): string | null {
	const match = url.match(/^https?:\/\/([^/?#]+)/i);
	return match ? match[1].replace(/^www\./i, "").toLowerCase() : null;
}

function colorForUrl(url: string): string | null {
	const host = hostnameOf(url);
	if (!host) return null;
	for (const entry of DOMAIN_COLORS) {
		if (host === entry.host || host.endsWith(`.${entry.host}`)) return entry.color;
	}
	return null;
}

const MENTION_RE = /@(everyone|here|[a-zA-Z0-9_]{3,32})\b/g;

export function renderMarkdown(raw: string, myUsername?: string): string {
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

	text = text.replace(/(https?:\/\/[^\s<>"']+)/g, (match: string) => {
		const color = colorForUrl(match);
		const style = color ? ` style="color:${color}"` : "";
		return `<a href="${match}" class="md-link" target="_blank" rel="noreferrer"${style}>${match}</a>`;
	});

	text = text.replace(MENTION_RE, (_match, target: string) => {
		const isSpecial = target === "everyone" || target === "here";
		const isMe = !isSpecial && !!myUsername && target.toLowerCase() === myUsername.toLowerCase();
		const cls = isSpecial ? "md-mention md-mention-special" : isMe ? "md-mention md-mention-self" : "md-mention";
		return `<span class="${cls}">@${target}</span>`;
	});

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
