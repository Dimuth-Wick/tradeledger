import type { CustomField, FieldValue, ValueInput } from "./api";

// Custom field form state on the trade detail page. Every value is kept as
// the text Rust stores ("true" for a ticked checkbox, "" for no value).
// Validation is done in Rust when saving.

/** field id → value text */
export type FieldDrafts = Record<number, string>;

export function draftsFrom(
  fields: CustomField[],
  saved: FieldValue[],
): FieldDrafts {
  const drafts: FieldDrafts = {};
  for (const f of fields) {
    drafts[f.id] = saved.find((v) => v.fieldId === f.id)?.value ?? "";
  }
  return drafts;
}

/** Only the fields whose text differs from what's saved. */
export function changedValues(
  fields: CustomField[],
  saved: FieldValue[],
  drafts: FieldDrafts,
): ValueInput[] {
  const before = draftsFrom(fields, saved);
  return fields
    .filter((f) => (drafts[f.id] ?? "").trim() !== before[f.id].trim())
    .map((f) => {
      const v = (drafts[f.id] ?? "").trim();
      return { fieldId: f.id, value: v === "" ? null : v };
    });
}

/** "Asia, London ,, NY" → ["Asia", "London", "NY"] (Rust checks repeats). */
export function parseOptions(text: string): string[] {
  return text
    .split(",")
    .map((o) => o.trim())
    .filter((o) => o !== "");
}

export const FIELD_TYPE_LABELS: Record<CustomField["fieldType"], string> = {
  text: "Text",
  number: "Number",
  select: "Choice list",
  checkbox: "Checkbox",
};
