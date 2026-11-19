import { describe, expect, it } from "vitest";
import {
  formatBytes,
  percent,
  releaseUrl,
  stepLabel,
  updateErrorText,
} from "./updates";

describe("releaseUrl", () => {
  it("links to the version's tag", () => {
    expect(releaseUrl("1.2.0")).toBe(
      "https://github.com/Dimuth-Wick/tradeledger/releases/tag/v1.2.0",
    );
    expect(releaseUrl("v1.2.0")).toMatch(/\/tag\/v1\.2\.0$/);
  });
});

describe("formatBytes", () => {
  it("picks a unit", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(3_250_000)).toBe("3.1 MB");
  });
});

describe("percent", () => {
  it("is whole and capped", () => {
    expect(percent(50, 200)).toBe(25);
    expect(percent(199, 200)).toBe(99);
    expect(percent(300, 200)).toBe(100);
  });
  it("is null without a size", () => {
    expect(percent(10, null)).toBeNull();
    expect(percent(10, 0)).toBeNull();
  });
});

describe("stepLabel", () => {
  it("describes each step", () => {
    expect(stepLabel({ step: "backup" })).toBe("Backing up your data…");
    expect(stepLabel({ step: "download", done: 30, total: 120 })).toBe(
      "Downloading… 25%",
    );
    expect(stepLabel({ step: "download", done: 2048, total: null })).toBe(
      "Downloading… 2.0 KB",
    );
    expect(stepLabel({ step: "install" })).toMatch(/restart/);
  });
});

describe("updateErrorText", () => {
  it("explains common failures", () => {
    expect(
      updateErrorText(new Error("error sending request: dns error")),
    ).toMatch(/internet/);
    expect(updateErrorText("signature verification failed")).toMatch(
      /safety check/,
    );
    expect(
      updateErrorText("Could not fetch a valid release JSON from the remote"),
    ).toMatch(/No update information/);
    expect(updateErrorText("disk full")).toBe("Update failed: disk full");
  });
});
