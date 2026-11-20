import { describe, expect, it } from "vitest";
import type { MilestoneProgress } from "../connection/backend";
import {
  celebration,
  currentBadge,
  nextBadge,
  nextText,
  shortCount,
  tierOf,
} from "./milestones";

const LEVELS = [50, 100, 300, 500, 1000, 2000, 5000];
const NAMES = [
  "Getting started",
  "Committed",
  "Disciplined",
  "Seasoned",
  "Veteran",
  "Elite",
  "Legend",
];

function at(trades: number, unseen: number[] = []): MilestoneProgress {
  const next = LEVELS.find((l) => trades < l) ?? null;
  return {
    trades,
    milestones: LEVELS.map((l, i) => ({
      trades: l,
      name: NAMES[i],
      message: "",
      reached: trades >= l,
    })),
    next,
    percent: 0,
    unseen,
  };
}

describe("milestones", () => {
  it("celebrates the highest new badge", () => {
    expect(celebration(at(73))).toBeNull();
    const c = celebration(at(320, [100, 300]));
    expect(c?.badge.name).toBe("Disciplined");
    expect(c?.alsoUnlocked).toBe(1);
  });

  it("says what's next", () => {
    expect(nextText(at(73))).toBe("27 more trades to Committed");
    expect(nextText(at(99))).toBe("1 more trade to Committed");
    expect(nextText(at(6000))).toMatch(/Every badge/);
  });

  it("finds the current and next badge", () => {
    expect(currentBadge(at(10))).toBeNull();
    expect(nextBadge(at(10))?.trades).toBe(50);
    expect(currentBadge(at(320))?.name).toBe("Disciplined");
    expect(nextBadge(at(320))?.name).toBe("Seasoned");
    expect(nextBadge(at(6000))).toBeNull();
  });

  it("names tiers and short counts", () => {
    expect(tierOf(50)).toBe("bronze");
    expect(tierOf(5000)).toBe("legend");
    expect(shortCount(300)).toBe("300");
    expect(shortCount(1000)).toBe("1K");
    expect(shortCount(5000)).toBe("5K");
  });
});
