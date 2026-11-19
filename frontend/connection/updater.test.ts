import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent } from "@tauri-apps/plugin-updater";
import { checkForUpdate, type UpdateStep } from "./backend";

// No Tauri window in tests: fake the core and both updater plugins.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: vi.fn() }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: vi.fn() }));

const order: string[] = [];

beforeEach(() => {
  order.length = 0;
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation(async (cmd) => {
    order.push(cmd);
    return {};
  });
  vi.mocked(relaunch).mockReset();
  vi.mocked(relaunch).mockImplementation(async () => {
    order.push("relaunch");
  });
});

describe("checkForUpdate", () => {
  it("is null when the app is up to date", async () => {
    vi.mocked(check).mockResolvedValue(null);
    expect(await checkForUpdate()).toBeNull();
  });

  it("backs up, downloads with progress, then restarts", async () => {
    const downloadAndInstall = vi.fn(
      async (onEvent?: (e: DownloadEvent) => void) => {
        order.push("download");
        onEvent?.({ event: "Started", data: { contentLength: 200 } });
        onEvent?.({ event: "Progress", data: { chunkLength: 50 } });
        onEvent?.({ event: "Progress", data: { chunkLength: 150 } });
        onEvent?.({ event: "Finished" });
      },
    );
    vi.mocked(check).mockResolvedValue({
      version: "1.2.0",
      downloadAndInstall,
    } as unknown as Awaited<ReturnType<typeof check>>);

    const update = await checkForUpdate();
    expect(update?.version).toBe("1.2.0");

    const steps: UpdateStep[] = [];
    await update!.install((s) => steps.push(s));

    expect(order).toEqual(["backup_before_update", "download", "relaunch"]);
    expect(steps).toEqual([
      { step: "backup" },
      { step: "download", done: 0, total: 200 },
      { step: "download", done: 50, total: 200 },
      { step: "download", done: 200, total: 200 },
      { step: "install" },
    ]);
  });

  it("installs nothing if the backup fails", async () => {
    const downloadAndInstall = vi.fn();
    vi.mocked(check).mockResolvedValue({
      version: "1.2.0",
      downloadAndInstall,
    } as unknown as Awaited<ReturnType<typeof check>>);
    vi.mocked(invoke).mockRejectedValue("disk full");

    const update = await checkForUpdate();
    await expect(update!.install(() => {})).rejects.toBe("disk full");
    expect(downloadAndInstall).not.toHaveBeenCalled();
    expect(relaunch).not.toHaveBeenCalled();
  });
});
