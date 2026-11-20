import { useId } from "react";
import { shortCount, tierOf } from "../../logic/milestones";

/** A hexagon medal with the trade count. Locked badges are grey outlines.
 *  Colors come from the tier class (badge-bronze, badge-gold, …). */
export default function Badge({
  trades,
  reached,
  size = 56,
}: {
  trades: number;
  reached: boolean;
  size?: number;
}) {
  const id = useId();
  const tier = tierOf(trades);
  return (
    <svg
      className={`badge badge-${tier}${reached ? "" : " badge-locked"}`}
      viewBox="0 0 100 110"
      width={size}
      height={size * 1.1}
      aria-hidden="true"
    >
      <defs>
        <linearGradient id={`${id}-fill`} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" className="badge-stop-1" />
          <stop offset="1" className="badge-stop-2" />
        </linearGradient>
      </defs>
      <path
        className="badge-shape"
        d="M50 4 L92 28 L92 82 L50 106 L8 82 L8 28 Z"
        fill={reached ? `url(#${id}-fill)` : "none"}
      />
      <path
        className="badge-inner"
        d="M50 16 L82 34 L82 76 L50 94 L18 76 L18 34 Z"
      />
      {reached && (
        <path className="badge-shine" d="M8 28 L50 4 L60 10 L16 36 Z" />
      )}
      <text
        x="50"
        y="58"
        dy="0.35em"
        textAnchor="middle"
        className="badge-count"
        fontSize={trades >= 1000 ? 26 : 28}
      >
        {shortCount(trades)}
      </text>
    </svg>
  );
}
