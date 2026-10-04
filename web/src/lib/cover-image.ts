const MAX_SIDE = 640;
const JPEG_QUALITY = 0.9;

interface NormalizedCover {
  ext: "jpg" | "png";
  data: number[];
}

async function decode(file: File): Promise<ImageBitmap> {
  try {
    return await createImageBitmap(file, { imageOrientation: "from-image" });
  } catch {
    return await createImageBitmap(file);
  }
}

function usesAlpha(bitmap: ImageBitmap): boolean {
  const probe = document.createElement("canvas");
  probe.width = 1;
  probe.height = 1;
  const ctx = probe.getContext("2d", { willReadFrequently: true });
  if (!ctx) return false;
  ctx.drawImage(bitmap, 0, 0, 1, 1);
  return ctx.getImageData(0, 0, 1, 1).data[3] < 255;
}

export async function normalizeCoverImage(
  file: File,
): Promise<NormalizedCover> {
  const bitmap = await decode(file);
  try {
    const side = Math.min(bitmap.width, bitmap.height);
    if (side < 1) throw new Error("image has no pixels");
    const srcX = (bitmap.width - side) / 2;
    const srcY = (bitmap.height - side) / 2;
    const out = Math.min(MAX_SIDE, side);

    const canvas = document.createElement("canvas");
    canvas.width = out;
    canvas.height = out;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("canvas is unavailable");
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = "high";
    ctx.drawImage(bitmap, srcX, srcY, side, side, 0, 0, out, out);

    const alpha = usesAlpha(bitmap);
    const mime = alpha ? "image/png" : "image/jpeg";
    const blob = await new Promise<Blob | null>((resolve) =>
      canvas.toBlob(resolve, mime, JPEG_QUALITY),
    );
    if (!blob) throw new Error("failed to encode cover image");

    return {
      ext: alpha ? "png" : "jpg",
      data: Array.from(new Uint8Array(await blob.arrayBuffer())),
    };
  } finally {
    bitmap.close();
  }
}
