import type { ConnectionService } from "$lib/api/client";

function subdomainHandle(host: string, suffix: string): string | null {
	if (!host.endsWith(suffix) || host === suffix) return null;
	return host.slice(0, host.length - suffix.length - 1);
}

/**
 * Derives the human handle (username, channel, invite code, ...) a
 * connection URL points at, so profiles can show "torvalds" instead of
 * "github.com". Purely presentational - falls back to null (caller shows
 * the stored label instead) whenever the URL doesn't parse or match a
 * known shape for that service.
 */
export function extractConnectionHandle(service: ConnectionService, rawUrl: string): string | null {
	let parsed: URL;
	try {
		parsed = new URL(rawUrl);
	} catch {
		return null;
	}

	const host = parsed.hostname.replace(/^www\./i, "").toLowerCase();

	if (service === "bandcamp") return subdomainHandle(host, "bandcamp.com");
	if (service === "itchio") return subdomainHandle(host, "itch.io");

	const segments = parsed.pathname.split("/").filter(Boolean);
	if (segments.length === 0) return null;

	switch (service) {
		case "reddit":
			return segments[0] === "u" || segments[0] === "user" ? (segments[1] ?? null) : segments[0];
		case "steam":
			return segments[0] === "id" || segments[0] === "profiles" ? (segments[1] ?? null) : segments[0];
		case "spotify":
			return segments[0] === "user" ? (segments[1] ?? null) : segments[0];
		case "epicgames":
			return segments[0] === "u" ? (segments[1] ?? null) : (segments.at(-1) ?? null);
		case "roblox":
			return segments[0] === "users" ? (segments[1] ?? null) : (segments.at(-1) ?? null);
		case "xbox":
		case "playstation":
		case "battlenet":
			return segments.at(-1) ?? null;
		default:
			return segments[0];
	}
}
