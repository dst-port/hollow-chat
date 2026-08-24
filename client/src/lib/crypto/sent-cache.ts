const STORAGE_KEY = "hollowchat_sent_cache";
const MAX_ENTRIES = 5000;

type Cache = { entries: Record<string, string>; order: string[] };

function load(): Cache {
	const raw = localStorage.getItem(STORAGE_KEY);
	if (!raw) return { entries: {}, order: [] };
	try {
		return JSON.parse(raw) as Cache;
	} catch {
		return { entries: {}, order: [] };
	}
}

function save(cache: Cache) {
	localStorage.setItem(STORAGE_KEY, JSON.stringify(cache));
}

export function rememberDecrypted(messageId: string, plaintext: string) {
	const cache = load();
	if (!(messageId in cache.entries)) cache.order.push(messageId);
	cache.entries[messageId] = plaintext;
	while (cache.order.length > MAX_ENTRIES) {
		const oldest = cache.order.shift();
		if (oldest) delete cache.entries[oldest];
	}
	save(cache);
}

export function recallDecrypted(messageId: string): string | null {
	return load().entries[messageId] ?? null;
}
