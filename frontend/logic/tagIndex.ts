import type { Tag, TagGroup } from "../connection/backend";

/** Neutral color for tags in groups without a color. */
export const FALLBACK_COLOR = "#6b7280";

export interface TagInfo {
  tag: Tag;
  group: TagGroup;
  /** The tag's own color, else its group's, else neutral gray. */
  color: string;
}

/** Tag id → tag + group + display color, for showing chips anywhere. */
export function indexTags(groups: TagGroup[]): Map<number, TagInfo> {
  const map = new Map<number, TagInfo>();
  for (const group of groups) {
    for (const tag of group.tags) {
      map.set(tag.id, {
        tag,
        group,
        color: tag.color || group.color || FALLBACK_COLOR,
      });
    }
  }
  return map;
}

/** Turns on/off one tag in a set of ids (kept sorted for stable saving). */
export function toggleTag(ids: number[], id: number): number[] {
  const next = ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id];
  return next.sort((a, b) => a - b);
}
