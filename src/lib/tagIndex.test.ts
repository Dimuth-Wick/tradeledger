import { describe, expect, it } from "vitest";
import type { TagGroup } from "./api";
import { FALLBACK_COLOR, indexTags, toggleTag } from "./tagIndex";

const groups: TagGroup[] = [
  {
    id: 1,
    name: "Setup",
    color: "#396cd8",
    position: 0,
    tags: [
      { id: 10, groupId: 1, name: "Breakout", color: "" },
      { id: 11, groupId: 1, name: "Range", color: "#00aa11" },
    ],
  },
  {
    id: 2,
    name: "Other",
    color: "",
    position: 1,
    tags: [{ id: 20, groupId: 2, name: "Misc", color: "" }],
  },
];

describe("tag index", () => {
  it("finds tags by id with a display color", () => {
    const idx = indexTags(groups);
    expect(idx.get(10)?.color).toBe("#396cd8"); // group color
    expect(idx.get(11)?.color).toBe("#00aa11"); // own color wins
    expect(idx.get(20)?.color).toBe(FALLBACK_COLOR);
    expect(idx.get(20)?.group.name).toBe("Other");
    expect(idx.get(99)).toBeUndefined();
  });

  it("toggles tags and keeps ids sorted", () => {
    expect(toggleTag([20, 10], 11)).toEqual([10, 11, 20]);
    expect(toggleTag([10, 11], 10)).toEqual([11]);
    expect(toggleTag([], 5)).toEqual([5]);
  });
});
