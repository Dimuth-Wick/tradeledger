import type { MilestoneProgress } from "../connection/backend";
import Badge from "../components/shared/Badge";
import { currentBadge, nextBadge, nextText } from "../logic/milestones";

/** Opened from the badge in the top bar: the badge you have, how far the
 *  next one is, and the whole ladder. */
export default function BadgesPage({
  progress,
}: {
  progress: MilestoneProgress;
}) {
  const current = currentBadge(progress);
  const next = nextBadge(progress);

  return (
    <section className="badges-page">
      <div className="badges-hero card">
        <div className="badges-hero-badge">
          <Badge
            trades={(current ?? next)?.trades ?? 50}
            reached={current !== null}
            size={120}
          />
        </div>
        <div className="badges-hero-text">
          <p className="badges-kicker">
            {current ? "Your badge" : "Your first badge"}
          </p>
          <h2>{current ? current.name : next?.name}</h2>
          <p className="hint">
            {current
              ? current.message
              : `Log ${next?.trades} trades to earn it. Every trade you save or import counts.`}
          </p>
          <div className="badges-next">
            <div
              className="milestone-bar"
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={progress.percent}
              aria-label="Progress to the next badge"
            >
              <span style={{ width: `${progress.percent}%` }} />
            </div>
            <p>
              <strong>{progress.trades}</strong> trades logged ·{" "}
              {nextText(progress)}
            </p>
          </div>
        </div>
      </div>

      <h3 className="form-subhead">All badges</h3>
      <ol className="badge-ladder">
        {progress.milestones.map((m) => {
          const isNext = m.trades === next?.trades;
          const state = m.reached ? "earned" : isNext ? "next" : "locked";
          const left = m.trades - progress.trades;
          return (
            <li key={m.trades} className={`ladder-step ${state}`}>
              <Badge trades={m.trades} reached={m.reached} size={52} />
              <div className="ladder-text">
                <strong>{m.name}</strong>
                <span className="hint">
                  {m.reached
                    ? m.message
                    : `Log ${m.trades.toLocaleString()} trades`}
                </span>
              </div>
              <span className={`ladder-status ${state}`}>
                {m.reached
                  ? "Earned"
                  : isNext
                    ? `${left} to go`
                    : `${left.toLocaleString()} to go`}
              </span>
            </li>
          );
        })}
      </ol>
      <p className="hint badges-more">
        More badges are coming in future versions.
      </p>
    </section>
  );
}
