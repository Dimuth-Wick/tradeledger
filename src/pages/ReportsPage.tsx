import { useEffect, useMemo, useState } from "react";
import Segmented from "../components/Segmented";
import StatsFilters from "../components/StatsFilters";
import {
  getReport,
  listTagGroups,
  type GroupBy,
  type Report,
  type ReportRow,
  type TagGroup,
} from "../lib/api";
import { EMPTY_DASH, dashQuery, type DashFilters } from "../lib/dashboard";
import { formatR, isNegative, pnlClass, usd } from "../lib/format";
import {
  GROUPS,
  barPercent,
  maxAbsNet,
  profitFactorText,
} from "../lib/reports";

const money = (d: string | null) =>
  d === null ? "—" : usd(d, { round: true });

/** Stats split by tag, symbol, weekday, hour, market or side. */
export default function ReportsPage() {
  const [filters, setFilters] = useState<DashFilters>(EMPTY_DASH);
  const [groupBy, setGroupBy] = useState<GroupBy>("tag");
  const [tagGroup, setTagGroup] = useState<number | null>(null);
  const [tagGroups, setTagGroups] = useState<TagGroup[]>([]);
  const [report, setReport] = useState<Report | null>(null);
  const [error, setError] = useState<string | null>(null);

  const query = useMemo(() => dashQuery(filters), [filters]);

  useEffect(() => {
    let cancelled = false;
    getReport(query, groupBy, groupBy === "tag" ? tagGroup : null).then(
      (r) => {
        if (cancelled) return;
        setReport(r);
        setError(null);
      },
      (err) => !cancelled && setError(String(err)),
    );
    return () => {
      cancelled = true;
    };
  }, [query, groupBy, tagGroup]);

  useEffect(() => {
    listTagGroups().then(setTagGroups, () => setTagGroups([]));
  }, []);

  const maxAbs = report ? maxAbsNet(report.rows) : 0;

  return (
    <main className="page reports">
      <StatsFilters filters={filters} onChange={setFilters} />

      <div className="report-bar">
        <Segmented
          label="Group by"
          options={GROUPS}
          value={groupBy}
          onChange={setGroupBy}
        />
        {groupBy === "tag" && (
          <label className="inline">
            Tag group
            <select
              value={tagGroup ?? ""}
              onChange={(e) =>
                setTagGroup(
                  e.target.value === "" ? null : Number(e.target.value),
                )
              }
            >
              <option value="">All groups</option>
              {tagGroups.map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name}
                </option>
              ))}
            </select>
          </label>
        )}
      </div>

      {error && <p className="status error">{error}</p>}

      {report && (
        <>
          {report.rows.length === 0 ? (
            <p className="chart-empty">No closed trades match these filters.</p>
          ) : (
            <table className="trade-list report-table">
              <thead>
                <tr>
                  <th scope="col">
                    {GROUPS.find(([id]) => id === groupBy)?.[1]}
                  </th>
                  <th scope="col" className="num">
                    Trades
                  </th>
                  <th scope="col" className="num">
                    Win rate
                  </th>
                  <th scope="col" className="num">
                    Net PnL
                  </th>
                  <th scope="col" className="bar-col">
                    <span className="sr-only">Net PnL bar</span>
                  </th>
                  <th scope="col" className="num">
                    Avg / trade
                  </th>
                  <th scope="col" className="num">
                    Profit factor
                  </th>
                  <th scope="col" className="num">
                    Avg R
                  </th>
                </tr>
              </thead>
              <tbody>
                {report.rows.map((r) => (
                  <Row key={r.key} r={r} maxAbs={maxAbs} />
                ))}
              </tbody>
              <tfoot>
                <Row r={report.total} maxAbs={0} />
              </tfoot>
            </table>
          )}
          <p className="chart-note">
            Closed trades only
            {report.openTrades > 0 &&
              ` (${report.openTrades} open trade${report.openTrades === 1 ? "" : "s"} left out)`}
            .
            {groupBy === "tag" &&
              " A trade with several tags counts under each of them, so rows can add up to more than the total."}
            {(groupBy === "weekday" || groupBy === "hour") &&
              " Weekday and hour use the time the trade was opened, in the calendar's time zone."}
          </p>
        </>
      )}
    </main>
  );
}

function Row({ r, maxAbs }: { r: ReportRow; maxAbs: number }) {
  const empty = r.trades === 0;
  const pct = barPercent(r.netPnl, maxAbs);
  return (
    <tr className={empty ? "empty" : ""}>
      <th scope="row">{r.label}</th>
      <td className="num">{r.trades}</td>
      <td className="num">{r.winRate === null ? "—" : `${r.winRate}%`}</td>
      <td className={`num ${pnlClass(r.netPnl)}`} title={usd(r.netPnl)}>
        {empty ? "—" : money(r.netPnl)}
      </td>
      <td className="bar-col" aria-hidden="true">
        {maxAbs > 0 && !empty && (
          <div className="pnl-bar">
            <span
              className={isNegative(r.netPnl) ? "neg-bar" : "pos-bar"}
              style={{ width: `${pct / 2}%` }}
            />
          </div>
        )}
      </td>
      <td className={`num ${pnlClass(r.avgPnl)}`}>{money(r.avgPnl)}</td>
      <td className="num">{profitFactorText(r)}</td>
      <td className={`num ${pnlClass(r.avgR)}`}>{formatR(r.avgR)}</td>
    </tr>
  );
}
