import { describe, expect, it } from "vitest";
import type { CustomField } from "../connection/backend";
import { changedValues, draftsFrom, parseOptions } from "./customFields";

const field = (
  id: number,
  fieldType: CustomField["fieldType"],
): CustomField => ({
  id,
  name: `F${id}`,
  fieldType,
  options: fieldType === "select" ? ["Asia", "London"] : [],
  position: id,
  usedBy: 0,
});

const fields = [field(1, "select"), field(2, "number"), field(3, "checkbox")];

describe("custom field drafts", () => {
  it("starts from saved values, empty when missing", () => {
    expect(draftsFrom(fields, [{ fieldId: 2, value: "7.5" }])).toEqual({
      1: "",
      2: "7.5",
      3: "",
    });
  });

  it("sends only changed fields, empty as null", () => {
    const saved = [
      { fieldId: 1, value: "Asia" },
      { fieldId: 2, value: "7.5" },
    ];
    const drafts = { 1: "London", 2: " 7.5 ", 3: "true" };
    expect(changedValues(fields, saved, drafts)).toEqual([
      { fieldId: 1, value: "London" },
      { fieldId: 3, value: "true" },
    ]);
    expect(changedValues(fields, saved, { 1: "", 2: "7.5", 3: "" })).toEqual([
      { fieldId: 1, value: null },
    ]);
  });

  it("splits the options box on commas", () => {
    expect(parseOptions("Asia, London ,, NY ")).toEqual([
      "Asia",
      "London",
      "NY",
    ]);
    expect(parseOptions("  ")).toEqual([]);
  });
});
