import { useEffect, useRef } from "react";
import type { Milestone } from "../../connection/backend";
import Badge from "./Badge";

/** Pieces of confetti: angle (deg), distance (px), delay (ms), color index. */
const CONFETTI = Array.from({ length: 28 }, (_, i) => ({
  angle: (i * 360) / 28 + (i % 3) * 7,
  distance: 110 + ((i * 37) % 70),
  delay: (i % 7) * 30,
  color: i % 5,
}));

/** "Badge unlocked" celebration, shown once per badge. */
export default function MilestonePopup({
  badge,
  alsoUnlocked,
  trades,
  onClose,
  onSeeAll,
}: {
  badge: Milestone;
  alsoUnlocked: number;
  trades: number;
  onClose: () => void;
  onSeeAll: () => void;
}) {
  const button = useRef<HTMLButtonElement>(null);
  // Latest onClose without re-running the effect on every render.
  const close = useRef(onClose);
  useEffect(() => {
    close.current = onClose;
  });
  useEffect(() => {
    button.current?.focus();
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close.current();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="modal-backdrop milestone-backdrop">
      <div
        className="modal milestone-popup"
        role="dialog"
        aria-modal="true"
        aria-labelledby="milestone-title"
      >
        <div className="milestone-burst" aria-hidden="true">
          {CONFETTI.map((c, i) => (
            <span
              key={i}
              className={`confetti confetti-${c.color}`}
              style={
                {
                  "--angle": `${c.angle}deg`,
                  "--distance": `${c.distance}px`,
                  animationDelay: `${c.delay}ms`,
                } as React.CSSProperties
              }
            />
          ))}
          <div className="milestone-badge">
            <Badge trades={badge.trades} reached size={112} />
          </div>
        </div>
        <p className="milestone-kicker">Badge unlocked</p>
        <h2 id="milestone-title">{badge.name}</h2>
        <p>{badge.message}</p>
        <p className="hint">
          {trades} trades logged
          {alsoUnlocked > 0 &&
            ` · ${alsoUnlocked} more badge${alsoUnlocked === 1 ? "" : "s"} unlocked too`}
          . Your badge now shows in the top bar.
        </p>
        <div className="milestone-actions">
          <button
            ref={button}
            type="button"
            className="primary"
            onClick={onClose}
          >
            Keep going
          </button>
          <button type="button" onClick={onSeeAll}>
            See all badges
          </button>
        </div>
      </div>
    </div>
  );
}
