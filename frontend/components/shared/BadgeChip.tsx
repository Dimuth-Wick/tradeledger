import type { MilestoneProgress } from "../../connection/backend";
import { currentBadge, nextBadge, nextText } from "../../logic/milestones";
import Badge from "./Badge";

const R = 19;
const CIRC = 2 * Math.PI * R;

/** Top-bar button: your current badge inside a ring that fills up towards
 *  the next one. Opens the Badges page. */
export default function BadgeChip({
  progress,
  active,
  onOpen,
}: {
  progress: MilestoneProgress;
  active: boolean;
  onOpen: () => void;
}) {
  const current = currentBadge(progress);
  const next = nextBadge(progress);
  // Before the first badge, show the first one locked.
  const shown = current ?? next;
  if (!shown) return null;
  const label = `${current ? current.name : "No badge yet"}. ${nextText(progress)}. Open badges`;
  return (
    <button
      type="button"
      className={`badge-chip${active ? " active" : ""}`}
      onClick={onOpen}
      title={nextText(progress)}
      aria-label={label}
    >
      <span className="badge-chip-ring">
        <svg viewBox="0 0 44 44" aria-hidden="true">
          <circle className="ring-track" cx="22" cy="22" r={R} />
          <circle
            className="ring-fill"
            cx="22"
            cy="22"
            r={R}
            strokeDasharray={CIRC}
            strokeDashoffset={CIRC * (1 - progress.percent / 100)}
          />
        </svg>
        <span className="badge-chip-badge">
          <Badge trades={shown.trades} reached={current !== null} size={26} />
        </span>
      </span>
      <span className="badge-chip-text">
        <strong>{progress.trades}</strong>
        {next ? <span>/ {next.trades}</span> : <span>trades</span>}
      </span>
    </button>
  );
}
