import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  addScreenshot,
  confirmImport,
  createCustomField,
  createTag,
  setCustomValues,
  createTagGroup,
  createTrade,
  deleteTagGroup,
  getCalendarMonth,
  getDayTrades,
  getReport,
  createPlaybook,
  updatePlaybook,
  setTradePlaybook,
  getTradePlaybook,
  getStats,
  saveDashboardLayout,
  setTimeZone,
  setTradeTags,
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
  previewJournal,
  previewTrade,
  queryTrades,
  updateJournal,
  type JournalInput,
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

  const journal: JournalInput = {
    plannedEntry: null,
    stopLoss: "36000",
    takeProfit: null,
    leverage: 10,
    notes: "fake note",
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
    [
      "query_trades",
      () => queryTrades({ symbol: "btc" }),
      { query: { symbol: "btc" } },
    ],
    [
      "update_journal",
      () => updateJournal(7, journal),
      { id: 7, input: journal },
    ],
    [
      "preview_journal",
      () => previewJournal(7, journal),
      { id: 7, input: journal },
    ],
    [
      "create_tag_group",
      () => createTagGroup("Setup", "#396cd8"),
      { name: "Setup", color: "#396cd8" },
    ],
    [
      "create_tag",
      () => createTag(1, "Breakout", ""),
      { groupId: 1, name: "Breakout", color: "" },
    ],
    [
      "set_trade_tags",
      () => setTradeTags(7, [1, 2]),
      { tradeId: 7, tagIds: [1, 2] },
    ],
    ["delete_tag_group", () => deleteTagGroup(3), { id: 3 }],
    [
      "get_stats",
      () => getStats({ from: 1, to: 2 }),
      { query: { from: 1, to: 2 } },
    ],
    [
      "create_playbook",
      () => createPlaybook("Breakout", "", ["Trend agrees"]),
      { name: "Breakout", description: "", rules: ["Trend agrees"] },
    ],
    [
      "update_playbook",
      () => updatePlaybook(3, "B", "d", [{ id: 1, text: "x" }]),
      { id: 3, name: "B", description: "d", rules: [{ id: 1, text: "x" }] },
    ],
    [
      "set_trade_playbook",
      () => setTradePlaybook(7, 3, [1, 2]),
      { tradeId: 7, playbookId: 3, checked: [1, 2] },
    ],
    ["get_trade_playbook", () => getTradePlaybook(7), { tradeId: 7 }],
    [
      "get_report",
      () => getReport({ from: 1 }, "tag", 2),
      { query: { from: 1 }, groupBy: "tag", tagGroup: 2 },
    ],
    [
      "get_calendar_month",
      () => getCalendarMonth(2026, 3),
      { year: 2026, month: 3 },
    ],
    [
      "get_day_trades",
      () => getDayTrades("2026-03-02"),
      { date: "2026-03-02" },
    ],
    ["set_time_zone", () => setTimeZone("+05:30"), { value: "+05:30" }],
    [
      "save_dashboard_layout",
      () =>
        saveDashboardLayout([
          { i: "kpis", x: 0, y: 0, w: 12, h: 6, minW: 3, minH: 3 },
        ]),
      { items: [{ i: "kpis", x: 0, y: 0, w: 12, h: 6, minW: 3, minH: 3 }] },
    ],
    [
      "create_custom_field",
      () => createCustomField("Session", "select", ["Asia", "NY"]),
      { name: "Session", fieldType: "select", options: ["Asia", "NY"] },
    ],
    [
      "set_custom_values",
      () => setCustomValues(7, [{ fieldId: 1, value: null }]),
      { tradeId: 7, values: [{ fieldId: 1, value: null }] },
    ],
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

  it("add_screenshot sends raw bytes with id and encoded caption headers", async () => {
    const bytes = new Uint8Array([0x89, 0x50]);
    await addScreenshot(7, bytes, "entry ✓");
    expect(invokeMock).toHaveBeenCalledWith("add_screenshot", bytes, {
      headers: { "x-trade-id": "7", "x-caption": "entry%20%E2%9C%93" },
    });
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
