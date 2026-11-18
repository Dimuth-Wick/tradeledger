import { useEffect, useState } from "react";
import {
  getCalendarMonth,
  getDayTrades,
  listTimeZones,
  setTimeZone,
  type CalendarDay,
  type CalendarMonth,
  type Trade,
  type WithId,
  type ZoneInfo,
} from "../lib/api";
import {
  WEEKDAYS,
  cellTone,
  dayTitle,
  monthTitle,
  shiftMonth,
  timeIn,
} from "../lib/calendar";
import { formatR, pnlClass, round2, signed } from "../lib/format";

interface Props {
  /** Opens the trade on the Trades tab. */
  onOpenTrade: (id: number) => void;
}

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

/** Month of daily closed PnL; click a day to list its trades. */
export default function CalendarPage({ onOpenTrade }: Props) {
  const now = new Date();
  const [ym, setYm] = useState({
    year: now.getFullYear(),
    month: now.getMonth() + 1,
  });
  const [data, setData] = useState<CalendarMonth | null>(null);
  const [zones, setZones] = useState<ZoneInfo[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [dayTrades, setDayTrades] = useState<WithId<Trade>[]>([]);
  const [error, setError] = useState<string | null>(null);
  // Bumped after a time-zone change so the month and day reload.
  const [version, setVersion] = useState(0);

  useEffect(() => {
    let cancelled = false;
    getCalendarMonth(ym.year, ym.month).then(
      (m) => {
        if (cancelled) return;
        setData(m);
        setError(null);
      },
      (err) => !cancelled && setError(String(err)),
    );
    return () => {
      cancelled = true;
    };
  }, [ym, version]);

  useEffect(() => {
    if (selected === null) return;
    let cancelled = false;
    getDayTrades(selected).then(
      (t) => !cancelled && setDayTrades(t),
      (err) => !cancelled && setError(String(err)),
    );
    return () => {
      cancelled = true;
    };
  }, [selected, version]);

  useEffect(() => {
    listTimeZones().then(setZones, () => setZones([]));
  }, []);

  function go(delta: number) {
    setYm((c) => shiftMonth(c.year, c.month, delta));
    setSelected(null);
  }

  function changeZone(value: string) {
    setTimeZone(value).then(
      () => setVersion((v) => v + 1),
      (err) => setError(String(err)),
    );
  }

  const selectedDay: CalendarDay | undefined = data?.weeks
    .flatMap((w) => w.days)
    .find((d) => d.date === selected);

  return (
    <main className="page calendar-page">
      <div className="calendar-bar">
        <button
          type="button"
          aria-label="Previous month"
          onClick={() => go(-1)}
        >
          ‹
        </button>
        <h1>{monthTitle(ym.year, ym.month)}</h1>
        <button type="button" aria-label="Next month" onClick={() => go(1)}>
          ›
        </button>
        <button
          type="button"
          onClick={() => {
            setYm({ year: now.getFullYear(), month: now.getMonth() + 1 });
            setSelected(null);
          }}
        >
          This month
        </button>
        <span className="spacer" />
        <label className="inline">
          Days in
          <select
            value={data?.zone.value ?? "local"}
            onChange={(e) => changeZone(e.target.value)}
          >
            {zones.map((z) => (
              <option key={z.value} value={z.value}>
                {z.label}
              </option>
            ))}
          </select>
        </label>
      </div>

      {error && <p className="status error">{error}</p>}

      {data && (
        <>
          <p className="log-summary">
            Net{" "}
            <strong
              className={pnlClass(data.netPnl)}
              title={signed(data.netPnl)}
            >
              {signed(round2(data.netPnl))}
            </strong>{" "}
            · {plural(data.trades, "trade")} on{" "}
            {plural(data.tradingDays, "day")} · {data.greenDays} green /{" "}
            {data.redDays} red
            {data.bestDay && (
              <>
                {" "}
                · best {dayTitle(data.bestDay.date)}{" "}
                {signed(round2(data.bestDay.netPnl))}
              </>
            )}
            {data.worstDay && data.worstDay.date !== data.bestDay?.date && (
              <>
                {" "}
                · worst {dayTitle(data.worstDay.date)}{" "}
                {signed(round2(data.worstDay.netPnl))}
              </>
            )}
          </p>

          <table className="calendar">
            <thead>
              <tr>
                {WEEKDAYS.map((d) => (
                  <th key={d} scope="col">
                    {d}
                  </th>
                ))}
                <th scope="col">Week</th>
              </tr>
            </thead>
            <tbody>
              {data.weeks.map((w) => (
                <tr key={w.days[0].date}>
                  {w.days.map((d) => {
                    const { tone, level } = cellTone(d.netPnl, data.maxAbsDay);
                    const cls = [
                      "day",
                      d.inMonth ? "" : "out",
                      tone ? `${tone} l${level}` : "",
                      d.date === data.today ? "today" : "",
                      d.date === selected ? "selected" : "",
                    ]
                      .filter(Boolean)
                      .join(" ");
                    return (
                      <td key={d.date} className={cls}>
                        {d.trades > 0 ? (
                          <button
                            type="button"
                            aria-pressed={d.date === selected}
                            aria-label={`${dayTitle(d.date)}: ${plural(d.trades, "trade")}, net ${signed(d.netPnl)}`}
                            onClick={() =>
                              setSelected((s) => (s === d.date ? null : d.date))
                            }
                          >
                            <span className="day-num">{d.day}</span>
                            <span className="day-pnl">
                              {signed(round2(d.netPnl))}
                            </span>
                            <span className="day-count">
                              {plural(d.trades, "trade")}
                            </span>
                          </button>
                        ) : (
                          <span className="day-num">{d.day}</span>
                        )}
                      </td>
                    );
                  })}
                  <td className="week">
                    {w.trades > 0 ? (
                      <>
                        <span className={`day-pnl ${pnlClass(w.netPnl)}`}>
                          {signed(round2(w.netPnl))}
                        </span>
                        <span className="day-count">
                          {plural(w.trades, "trade")}
                        </span>
                      </>
                    ) : (
                      <span className="day-count">—</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="chart-note">
            Each trade counts on the day it closed ({data.zone.label}). Open
            trades are not shown.
          </p>
        </>
      )}

      {selected && selectedDay && (
        <section className="day-panel">
          <h2>
            {dayTitle(selected)} · {plural(selectedDay.trades, "trade")} · net{" "}
            <span className={pnlClass(selectedDay.netPnl)}>
              {signed(selectedDay.netPnl)}
            </span>
          </h2>
          <table className="trade-list">
            <thead>
              <tr>
                <th>Closed</th>
                <th>Symbol</th>
                <th>Side</th>
                <th className="num">Net PnL</th>
                <th className="num">R</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {dayTrades.map((t) => (
                <tr key={t.id}>
                  <td>
                    {t.closeTime !== null &&
                      timeIn(t.closeTime, data?.zone.value ?? "local")}
                  </td>
                  <td>{t.symbol}</td>
                  <td className={t.direction}>{t.direction}</td>
                  <td className={`num ${pnlClass(t.netPnl)}`}>
                    {signed(t.netPnl)}
                  </td>
                  <td className={`num ${pnlClass(t.rMultiple)}`}>
                    {formatR(t.rMultiple)}
                  </td>
                  <td>
                    <button
                      type="button"
                      aria-label={`Open ${t.symbol} trade #${t.id}`}
                      onClick={() => onOpenTrade(t.id)}
                    >
                      Open
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </main>
  );
}
