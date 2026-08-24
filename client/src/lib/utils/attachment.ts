import { fileUrl } from "$lib/api/client";

const blobUrlCache = new Map<string, string>();

export async function loadAttachmentBlobUrl(
	token: string,
	id: string,
	filename: string
): Promise<string> {
	const cached = blobUrlCache.get(id);
	if (cached) return cached;

	const response = await fetch(fileUrl(id, filename), {
		headers: { authorization: `Bearer ${token}` }
	});
	if (!response.ok) throw new Error("failed to load attachment");

	const blob = await response.blob();
	const url = URL.createObjectURL(blob);
	blobUrlCache.set(id, url);
	return url;
}

export function triggerDownload(url: string, filename: string) {
	const a = document.createElement("a");
	a.href = url;
	a.download = filename;
	document.body.appendChild(a);
	a.click();
	a.remove();
}
