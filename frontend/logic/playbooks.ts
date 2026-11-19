import type { Playbook, RuleInput, TradePlaybook } from "../connection/backend";

// Playbook editor and checklist state. Rust validates and saves; this file
// only tracks what is being typed and whether it differs from what's saved.

/** A rule row in the editor. `key` is for React; `id` is Rust's (null = new). */
export interface RuleDraft {
  key: string;
  id: number | null;
  text: string;
}

let nextKey = 0;
const newKey = () => `r${nextKey++}`;

export function newRule(text = ""): RuleDraft {
  return { key: newKey(), id: null, text };
}

export function draftsFrom(p: Playbook | null): RuleDraft[] {
  if (!p) return [newRule()];
  return p.rules.map((r) => ({ key: `id${r.id}`, id: r.id, text: r.text }));
}

/** Moves the rule at `index` up (-1) or down (+1); out of range = no change. */
export function moveRule(
  rules: RuleDraft[],
  index: number,
  delta: number,
): RuleDraft[] {
  const to = index + delta;
  if (to < 0 || to >= rules.length) return rules;
  const next = [...rules];
  [next[index], next[to]] = [next[to], next[index]];
  return next;
}

/** Blank rows are dropped (an empty last row is normal while typing). */
export function toRuleInputs(rules: RuleDraft[]): RuleInput[] {
  return rules
    .filter((r) => r.text.trim() !== "")
    .map((r) => ({ id: r.id, text: r.text }));
}

export interface PlaybookDraft {
  name: string;
  description: string;
  rules: RuleDraft[];
}

export function draftOf(p: Playbook | null): PlaybookDraft {
  return {
    name: p?.name ?? "",
    description: p?.description ?? "",
    rules: draftsFrom(p),
  };
}

/** Differs from the saved playbook (or, for a new one, has any input)? */
export function playbookDirty(p: Playbook | null, d: PlaybookDraft): boolean {
  const rules = toRuleInputs(d.rules);
  if (!p) {
    return (
      d.name.trim() !== "" || d.description.trim() !== "" || rules.length > 0
    );
  }
  const same =
    d.name.trim() === p.name &&
    d.description.trim() === p.description &&
    rules.length === p.rules.length &&
    rules.every(
      (r, i) => r.id === p.rules[i].id && r.text.trim() === p.rules[i].text,
    );
  return !same;
}

/** The trade page's checklist edits vs what's saved. */
export function checklistDirty(
  saved: TradePlaybook,
  playbookId: number | null,
  checked: number[],
): boolean {
  if (saved.playbookId !== playbookId) return true;
  const a = [...saved.checked].sort((x, y) => x - y);
  const b = [...checked].sort((x, y) => x - y);
  return a.length !== b.length || a.some((v, i) => v !== b[i]);
}

/** "2 of 3 rules followed". */
export function followedText(checked: number, total: number): string {
  return `${checked} of ${total} rule${total === 1 ? "" : "s"} followed`;
}
