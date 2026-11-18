import { describe, expect, it } from "vitest";
import type { Playbook } from "./api";
import {
  checklistDirty,
  draftOf,
  followedText,
  moveRule,
  newRule,
  playbookDirty,
  toRuleInputs,
} from "./playbooks";

// Fake playbook.
const pb: Playbook = {
  id: 3,
  name: "Breakout retest",
  description: "Wait for the retest",
  rules: [
    { id: 1, text: "Trend agrees", position: 0 },
    { id: 2, text: "Retest holds", position: 1 },
  ],
  usedBy: 4,
};

describe("rule editor", () => {
  it("starts from the saved rules and is clean", () => {
    const d = draftOf(pb);
    expect(d.rules.map((r) => [r.id, r.text])).toEqual([
      [1, "Trend agrees"],
      [2, "Retest holds"],
    ]);
    expect(playbookDirty(pb, d)).toBe(false);
  });

  it("detects reorder, edits and new rules; ignores blank rows", () => {
    const d = draftOf(pb);
    expect(playbookDirty(pb, { ...d, rules: moveRule(d.rules, 0, 1) })).toBe(
      true,
    );
    expect(
      playbookDirty(pb, { ...d, rules: [...d.rules, newRule("  ")] }),
    ).toBe(false);
    expect(
      playbookDirty(pb, { ...d, rules: [...d.rules, newRule("Volume")] }),
    ).toBe(true);
    expect(playbookDirty(pb, { ...d, name: " Breakout retest " })).toBe(false);
  });

  it("moves stay inside the list", () => {
    const d = draftOf(pb).rules;
    expect(moveRule(d, 0, -1)).toBe(d);
    expect(moveRule(d, 1, 1)).toBe(d);
    expect(moveRule(d, 1, -1).map((r) => r.id)).toEqual([2, 1]);
  });

  it("sends ids for kept rules, null for new ones", () => {
    const rules = [...draftOf(pb).rules, newRule("Volume"), newRule("")];
    expect(toRuleInputs(rules)).toEqual([
      { id: 1, text: "Trend agrees" },
      { id: 2, text: "Retest holds" },
      { id: null, text: "Volume" },
    ]);
  });

  it("a new playbook is dirty once anything is typed", () => {
    expect(playbookDirty(null, draftOf(null))).toBe(false);
    expect(playbookDirty(null, { ...draftOf(null), name: "Range" })).toBe(true);
  });
});

describe("trade checklist", () => {
  const saved = { playbookId: 3, checked: [2, 1] };

  it("order of ticks doesn't matter", () => {
    expect(checklistDirty(saved, 3, [1, 2])).toBe(false);
    expect(checklistDirty(saved, 3, [1])).toBe(true);
    expect(checklistDirty(saved, null, [])).toBe(true);
  });

  it("text", () => {
    expect(followedText(2, 3)).toBe("2 of 3 rules followed");
    expect(followedText(1, 1)).toBe("1 of 1 rule followed");
  });
});
