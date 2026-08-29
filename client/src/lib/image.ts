// Center-crop an image File to a square and downscale it. Used for server
// icons and avatars so a rectangular upload fills a round/rounded frame
// cleanly instead of showing a zoomed-in slice at its original scale.
export async function squareCrop(file: File, size = 512): Promise<File> {
	if (!file.type.startsWith("image/") || file.type === "image/gif") {
		// leave GIFs (animated) and non-images untouched
		return file;
	}

	const bitmap = await createImageBitmap(file).catch(() => null);
	if (!bitmap) return file;

	const side = Math.min(bitmap.width, bitmap.height);
	const sx = (bitmap.width - side) / 2;
	const sy = (bitmap.height - side) / 2;
	const target = Math.min(size, side);

	const canvas = document.createElement("canvas");
	canvas.width = target;
	canvas.height = target;
	const ctx = canvas.getContext("2d");
	if (!ctx) return file;
	ctx.drawImage(bitmap, sx, sy, side, side, 0, 0, target, target);
	bitmap.close();

	const hasAlpha = file.type === "image/png" || file.type === "image/webp";
	const outType = hasAlpha ? "image/png" : "image/jpeg";
	const blob: Blob | null = await new Promise((resolve) =>
		canvas.toBlob(resolve, outType, 0.9)
	);
	if (!blob) return file;

	const ext = outType === "image/png" ? "png" : "jpg";
	const base = file.name.replace(/\.[^.]+$/, "") || "image";
	return new File([blob], `${base}.${ext}`, { type: outType });
}
