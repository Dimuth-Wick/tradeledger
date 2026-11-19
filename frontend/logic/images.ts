// Small helpers for getting images out of drops, pastes and file pickers.
// The real check (is it really an image?) happens in Rust on the bytes.

/** Formats Rust accepts. */
export const IMAGE_TYPES = [
  "image/png",
  "image/jpeg",
  "image/webp",
  "image/gif",
] as const;

export const MAX_IMAGE_BYTES = 20 * 1024 * 1024;

/** Keeps only files that look like supported images. */
export function imageFiles<T extends { type: string }>(
  files: Iterable<T>,
): T[] {
  return [...files].filter((f) =>
    (IMAGE_TYPES as readonly string[]).includes(f.type),
  );
}

/** MIME type from our generated file names ("t7-…-0.png"). */
export function mimeFromFileName(name: string): string {
  const ext = name.split(".").pop()?.toLowerCase();
  switch (ext) {
    case "png":
      return "image/png";
    case "jpg":
    case "jpeg":
      return "image/jpeg";
    case "webp":
      return "image/webp";
    case "gif":
      return "image/gif";
    default:
      return "application/octet-stream";
  }
}

/** Friendly reason a file is refused before uploading, or null if fine. */
export function rejectReason(file: {
  type: string;
  size: number;
}): string | null {
  if (!(IMAGE_TYPES as readonly string[]).includes(file.type)) {
    return "Only PNG, JPEG, WebP or GIF images can be added.";
  }
  if (file.size > MAX_IMAGE_BYTES) {
    return "Image is too large (max 20 MB).";
  }
  return null;
}
