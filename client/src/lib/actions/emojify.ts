import twemoji from "@discordapp/twemoji";
import { base } from "$app/paths";

export function emojify(node: HTMLElement) {
	twemoji.parse(node, {
		folder: "svg",
		ext: ".svg",
		base: `${base}/emoji/`,
		className: "emoji"
	});
}
