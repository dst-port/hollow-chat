// Center-crop an image File to a square and downscale it. Used for server
// icons and avatars so a rectangular (or letterboxed) upload fills a
// round/rounded frame instead of showing a slice or black bars.
async function loadBitmap(file: File): Promise<ImageBitmap | HTMLImageElement | null> {
	try {
		return await createImageBitmap(file);
	} catch {
		// Safari / odd formats: fall back to <img> decode.
		const url = URL.createObjectURL(file);
		try {
			const img = new Image();
			img.src = url;
			await img.decode();
			return img;
		} catch {
			return null;
		} finally {
			URL.revokeObjectURL(url);
		}
	}
}

export async function squareCrop(file: File, size = 512): Promise<File> {
	if (!file.type.startsWith("image/") || file.type === "image/gif") {
		return file; // leave animated GIFs and non-images alone
	}

	const source = await loadBitmap(file);
	if (!source) return file;

	const sw = "width" in source ? source.width : (source as HTMLImageElement).naturalWidth;
	const sh = "height" in source ? source.height : (source as HTMLImageElement).naturalHeight;
	if (!sw || !sh) return file;

	const side = Math.min(sw, sh);
	const sx = (sw - side) / 2;
	const sy = (sh - side) / 2;
	const target = Math.min(size, side);

	const canvas = document.createElement("canvas");
	canvas.width = target;
	canvas.height = target;
	const ctx = canvas.getContext("2d");
	if (!ctx) return file;
	ctx.drawImage(source as CanvasImageSource, sx, sy, side, side, 0, 0, target, target);
	if ("close" in source) source.close();

	const outType = file.type === "image/png" || file.type === "image/webp" ? "image/png" : "image/jpeg";
	const blob: Blob | null = await new Promise((resolve) => canvas.toBlob(resolve, outType, 0.9));
	if (!blob) return file;

	const ext = outType === "image/png" ? "png" : "jpg";
	const base = file.name.replace(/\.[^.]+$/, "") || "image";
	return new File([blob], `${base}.${ext}`, { type: outType });
}
