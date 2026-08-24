import twemoji from "@discordapp/twemoji";

export function emojify(node: HTMLElement) {
	twemoji.parse(node, {
		folder: "svg",
		ext: ".svg",
		base: "/emoji/",
		className: "emoji"
	});
}
