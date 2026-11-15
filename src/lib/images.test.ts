import { describe, expect, it } from "vitest";
import { imageFiles, mimeFromFileName, rejectReason } from "./images";

describe("image helpers", () => {
  it("keeps only supported image files", () => {
    const files = [
      { name: "a.png", type: "image/png" },
      { name: "b.txt", type: "text/plain" },
      { name: "c.jpg", type: "image/jpeg" },
      { name: "d.bmp", type: "image/bmp" },
    ];
    expect(imageFiles(files).map((f) => f.name)).toEqual(["a.png", "c.jpg"]);
  });

  it("knows the MIME type of generated names", () => {
    expect(mimeFromFileName("t7-1700000000000-0.png")).toBe("image/png");
    expect(mimeFromFileName("t7-1-2.jpg")).toBe("image/jpeg");
    expect(mimeFromFileName("t7-1-2.webp")).toBe("image/webp");
    expect(mimeFromFileName("weird")).toBe("application/octet-stream");
  });

  it("explains why a file is refused", () => {
    expect(rejectReason({ type: "image/png", size: 1000 })).toBeNull();
    expect(rejectReason({ type: "application/pdf", size: 10 })).toMatch(
      /PNG, JPEG/,
    );
    expect(rejectReason({ type: "image/png", size: 21 * 1024 * 1024 })).toMatch(
      /too large/,
    );
  });
});
