import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  confirmImport,
  createTrade,
  dbInfo,
  deleteTrade,
  getTrade,
  listTradeFills,
  listTrades,
  mergeTradeWithPrevious,
  resetGrouping,
  splitTradeAtFill,
  ping,
  previewImport,
  previewTrade,
  updateTrade,
  type TradeInput,
} from "./api";

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

describe("trade commands", () => {
  // Fake trade input (not real data).
  const input: TradeInput = {
    exchange: "binance",
    market: "usdm",
    symbol: "BTCUSDT",
    direction: "long",
    openTime: 1_700_000_000_000,
    qty: "0.01",
    avgEntry: "36500",
    stopLoss: "36000",
  };

  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
  });

  // Command names and argument keys must match the Rust #[tauri::command]
  // function and parameter names exactly, or the call fails at runtime.
  it.each([
    ["preview_trade", () => previewTrade(input), { input }],
    ["create_trade", () => createTrade(input), { input }],
    ["update_trade", () => updateTrade(7, input), { id: 7, input }],
    ["delete_trade", () => deleteTrade(7), { id: 7 }],
    ["get_trade", () => getTrade(7), { id: 7 }],
    ["list_trade_fills", () => listTradeFills(7), { tradeId: 7 }],
    ["merge_trade_with_previous", () => mergeTradeWithPrevious(7), { id: 7 }],
    [
      "split_trade_at_fill",
      () => splitTradeAtFill(7, 3),
      { tradeId: 7, fillId: 3 },
    ],
    ["reset_grouping", () => resetGrouping(7), { tradeId: 7 }],
    [
      "preview_import",
      () => previewImport("a.csv", "x"),
      { fileName: "a.csv", content: "x" },
    ],
    [
      "confirm_import",
      () => confirmImport("a.csv", "x"),
      { fileName: "a.csv", content: "x" },
    ],
  ] as const)("%s sends the right arguments", async (name, call, args) => {
    await call();
    expect(invokeMock).toHaveBeenCalledWith(name, args);
  });

  it("list_trades takes no arguments", async () => {
    invokeMock.mockResolvedValue([]);
    await expect(listTrades()).resolves.toEqual([]);
    expect(invokeMock).toHaveBeenCalledWith("list_trades");
  });

  it("validation errors from Rust reject the promise", async () => {
    invokeMock.mockRejectedValue("quantity must be greater than 0");
    await expect(createTrade({ ...input, qty: "0" })).rejects.toBe(
      "quantity must be greater than 0",
    );
  });
});
