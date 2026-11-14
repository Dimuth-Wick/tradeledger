import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { dbInfo, ping } from "./api";

// Tests run in Node, not inside the Tauri window, so there is no Rust core to
// answer. We replace invoke() with a fake and check what the wrapper asks for.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

describe("api.ping", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("calls the Rust `ping` command and returns its reply", async () => {
    invokeMock.mockResolvedValue("pong from TradeLedger core v0.1.0");

    await expect(ping()).resolves.toBe("pong from TradeLedger core v0.1.0");
    expect(invokeMock).toHaveBeenCalledWith("ping");
  });

  it("passes Rust errors through as a rejected promise", async () => {
    invokeMock.mockRejectedValue("core unavailable");

    await expect(ping()).rejects.toBe("core unavailable");
  });
});

describe("api.dbInfo", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("calls the Rust `db_info` command and returns path + schema version", async () => {
    const info = {
      path: "C:/data/TradeLedger/tradeledger.db",
      schemaVersion: 1,
    };
    invokeMock.mockResolvedValue(info);

    await expect(dbInfo()).resolves.toEqual(info);
    expect(invokeMock).toHaveBeenCalledWith("db_info");
  });
});
