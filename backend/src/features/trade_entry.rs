//! Creating, editing and deleting trades from user input.
//!
//! The UI sends only what a person types (`TradeInput`). Everything derived
//! (status, gross/net PnL, planned R:R, R-multiple) is computed here, so the
//! numbers can't disagree with each other and the UI never does money math.

use rusqlite::Connection;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::database::{self as db, DbError};
use crate::features::risk::{self, RiskError};
use crate::models::{Direction, Market, TimestampMs, Trade, TradeStatus, WithId};

/// What the trade form sends. Decimals arrive as strings ("36500.10").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeInput {
    pub exchange: String,
    pub market: Market,
    pub symbol: String,
    pub direction: Direction,
    pub open_time: TimestampMs,
    /// Set together with `avg_exit` when the trade is closed.
    pub close_time: Option<TimestampMs>,
    /// Coins bought or sold. Ignored when `position_size` is given.
    #[serde(default)]
    pub qty: Decimal,
    /// Position size in USDT at entry. When set, the quantity is worked out
    /// from it: size ÷ entry price.
    #[serde(default)]
    pub position_size: Option<Decimal>,
    pub avg_entry: Decimal,
    pub avg_exit: Option<Decimal>,
    pub leverage: Option<u32>,
    /// Total fees paid (≥ 0). Ignored when `auto_fees` is true.
    #[serde(default)]
    pub fees: Decimal,
    /// Work the fees out as `FEE_RATE` of the position size.
    #[serde(default)]
    pub auto_fees: bool,
    /// Net funding cost: positive = paid, negative = received.
    #[serde(default)]
    pub funding: Decimal,
    pub planned_entry: Option<Decimal>,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
    #[serde(default)]
    pub notes: String,
    pub playbook_id: Option<i64>,
}

#[derive(Debug, Error)]
pub enum TradeError {
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Risk(#[from] RiskError),
    #[error("trade {0} not found")]
    NotFound(i64),
    #[error(transparent)]
    Db(#[from] DbError),
}

/// Fee used when the trade form leaves fees empty: 0.1% of position size.
pub const FEE_RATE: Decimal = Decimal::from_parts(1, 0, 0, false, 3);

/// Quantity and fee precision (crypto amounts).
const AMOUNT_DP: u32 = 8;

fn round_amount(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(AMOUNT_DP, RoundingStrategy::MidpointAwayFromZero)
        .normalize()
}

/// The quantity and fees to store: typed in, or worked out from the
/// position size (call after `validate`).
pub fn resolve_size(input: &TradeInput) -> Result<(Decimal, Decimal), TradeError> {
    let qty = match input.position_size {
        Some(size) => {
            let qty = round_amount(size / input.avg_entry);
            if qty <= Decimal::ZERO {
                return Err(invalid("position size is too small for this price"));
            }
            qty
        }
        None => input.qty,
    };
    let size = input.position_size.unwrap_or(qty * input.avg_entry);
    let fees = if input.auto_fees {
        round_amount(size * FEE_RATE)
    } else {
        input.fees
    };
    Ok((qty, fees))
}

/// What the risk-plan boxes send while you type (before the trade is saved).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanInput {
    pub direction: Direction,
    /// Planned entry price.
    pub entry: Decimal,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
    /// Position size in USDT; without it only the R:R can be worked out.
    pub position_size: Option<Decimal>,
}

/// The plan's numbers, shown under the risk-plan boxes.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanPreview {
    /// Reward ÷ risk; needs both stop loss and take profit.
    pub planned_rr: Option<Decimal>,
    /// Result if the stop is hit, after fees (negative, to the cent).
    /// Needs stop + size.
    pub loss_at_stop: Option<Decimal>,
    /// Result if the target is hit, after fees. Needs target + size.
    pub profit_at_target: Option<Decimal>,
    /// The automatic fee (`FEE_RATE` of the size) included above.
    pub fees: Option<Decimal>,
}

/// Works out R:R and the profit/loss at the target and the stop. Fees are
/// the same automatic fee a saved trade gets, so the numbers match.
pub fn plan_preview(input: &PlanInput) -> Result<PlanPreview, TradeError> {
    if input.entry <= Decimal::ZERO {
        return Err(invalid("entry price must be greater than 0"));
    }
    if [input.stop_loss, input.take_profit, input.position_size]
        .into_iter()
        .flatten()
        .any(|v| v <= Decimal::ZERO)
    {
        return Err(invalid(
            "stop loss, take profit and position size must be greater than 0",
        ));
    }
    let d = input.direction;
    let risk = input
        .stop_loss
        .map(|stop| risk::risk_per_unit(d, input.entry, stop))
        .transpose()?;
    let reward = input
        .take_profit
        .map(|target| risk::reward_per_unit(d, input.entry, target))
        .transpose()?;
    let planned_rr = match (input.stop_loss, input.take_profit) {
        (Some(stop), Some(target)) => Some(risk::planned_rr(d, input.entry, stop, target)?),
        _ => None,
    };

    let sized = input.position_size.map(|size| {
        let qty = size / input.entry;
        (qty, round_amount(size * FEE_RATE))
    });
    let (loss_at_stop, profit_at_target) = match sized {
        Some((qty, fees)) => (
            risk.map(|r| round_cents(-(qty * r) - fees)),
            reward.map(|r| round_cents(qty * r - fees)),
        ),
        None => (None, None),
    };
    Ok(PlanPreview {
        planned_rr,
        loss_at_stop,
        profit_at_target,
        fees: sized.map(|(_, fees)| fees),
    })
}

/// Dollar amounts in the plan preview are shown to the cent.
fn round_cents(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
        .normalize()
}

fn invalid(msg: &str) -> TradeError {
    TradeError::Invalid(msg.to_string())
}

/// Rejects input that would produce a meaningless trade.
pub fn validate(input: &TradeInput) -> Result<(), TradeError> {
    let positive = |v: Decimal| v > Decimal::ZERO;
    let positive_if_set = |v: Option<Decimal>| v.is_none_or(positive);

    if input.exchange.trim().is_empty() {
        return Err(invalid("exchange is required"));
    }
    if input.symbol.trim().is_empty() {
        return Err(invalid("symbol is required"));
    }
    match input.position_size {
        Some(size) if !positive(size) => {
            return Err(invalid("position size must be greater than 0"));
        }
        None if !positive(input.qty) => {
            return Err(invalid("quantity must be greater than 0"));
        }
        _ => {}
    }
    if !positive(input.avg_entry) {
        return Err(invalid("entry price must be greater than 0"));
    }
    if !positive_if_set(input.avg_exit) {
        return Err(invalid("exit price must be greater than 0"));
    }
    if ![input.planned_entry, input.stop_loss, input.take_profit]
        .into_iter()
        .all(positive_if_set)
    {
        return Err(invalid(
            "planned entry, stop loss and take profit must be greater than 0",
        ));
    }
    if input.fees < Decimal::ZERO {
        return Err(invalid("fees cannot be negative"));
    }
    if input.leverage == Some(0) {
        return Err(invalid("leverage must be at least 1"));
    }
    // Half-closed states (exit price but no time, or the reverse) can't be
    // classified as open or closed, so require both or neither.
    match (input.avg_exit, input.close_time) {
        (Some(_), None) | (None, Some(_)) => {
            return Err(invalid(
                "a closed trade needs both an exit price and a close time",
            ));
        }
        (Some(_), Some(close)) if close < input.open_time => {
            return Err(invalid("close time cannot be before open time"));
        }
        _ => {}
    }
    Ok(())
}

/// Validates the input and computes every derived field.
pub fn build_trade(input: &TradeInput, is_manual: bool) -> Result<Trade, TradeError> {
    validate(input)?;
    let (qty, fees) = resolve_size(input)?;
    let dir = input.direction;

    let (status, gross_pnl) = match input.avg_exit {
        Some(exit) => (
            TradeStatus::Closed,
            risk::favourable_move(dir, input.avg_entry, exit) * qty,
        ),
        // Unrealized PnL needs a live price; an open trade has none yet.
        None => (TradeStatus::Open, Decimal::ZERO),
    };
    let net_pnl = gross_pnl - fees - input.funding;

    // The plan is measured from the planned entry; if the user didn't give
    // one, their actual entry is the plan.
    let plan_entry = input.planned_entry.unwrap_or(input.avg_entry);
    if let Some(stop) = input.stop_loss {
        risk::risk_per_unit(dir, plan_entry, stop)?;
    }
    if let Some(target) = input.take_profit {
        risk::reward_per_unit(dir, plan_entry, target)?;
    }
    let planned_rr = match (input.stop_loss, input.take_profit) {
        (Some(stop), Some(target)) => Some(risk::planned_rr(dir, plan_entry, stop, target)?),
        _ => None,
    };
    let r_multiple = match (input.stop_loss, input.avg_exit) {
        (Some(stop), Some(exit)) => Some(risk::r_multiple(
            dir,
            plan_entry,
            stop,
            input.avg_entry,
            exit,
        )?),
        _ => None,
    };

    Ok(Trade {
        exchange: input.exchange.trim().to_lowercase(),
        market: input.market,
        // "btcusdt " and "BTCUSDT" must group and filter as the same coin.
        symbol: input.symbol.trim().to_uppercase(),
        direction: dir,
        status,
        open_time: input.open_time,
        close_time: input.close_time,
        qty,
        avg_entry: input.avg_entry,
        avg_exit: input.avg_exit,
        leverage: input.leverage,
        gross_pnl,
        fees,
        funding: input.funding,
        net_pnl,
        planned_entry: input.planned_entry,
        stop_loss: input.stop_loss,
        take_profit: input.take_profit,
        planned_rr,
        r_multiple,
        notes: input.notes.clone(),
        playbook_id: input.playbook_id,
        is_manual,
    })
}

/// New manual trade.
pub fn create(conn: &Connection, input: &TradeInput) -> Result<WithId<Trade>, TradeError> {
    let trade = build_trade(input, true)?;
    let id = db::trades::insert(conn, &trade)?;
    Ok(WithId { id, item: trade })
}

/// Replaces a trade with recomputed values. Keeps whether it was manual or
/// imported: editing an imported trade doesn't make it manual.
pub fn update(conn: &Connection, id: i64, input: &TradeInput) -> Result<WithId<Trade>, TradeError> {
    let existing = db::trades::get(conn, id)?.ok_or(TradeError::NotFound(id))?;
    let trade = build_trade(input, existing.item.is_manual)?;
    db::trades::update(conn, id, &trade)?;
    Ok(WithId { id, item: trade })
}

/// What the "Exit" box on a saved trade sends: the exit price and when it
/// was closed, or both empty to reopen the trade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseInput {
    pub exit_price: Option<Decimal>,
    pub close_time: Option<TimestampMs>,
}

/// The saved trade as form input, so it can be rebuilt with a new exit.
/// Quantity and fees stay exactly as saved.
fn input_from(t: &Trade) -> TradeInput {
    TradeInput {
        exchange: t.exchange.clone(),
        market: t.market,
        symbol: t.symbol.clone(),
        direction: t.direction,
        open_time: t.open_time,
        close_time: t.close_time,
        qty: t.qty,
        position_size: None,
        avg_entry: t.avg_entry,
        avg_exit: t.avg_exit,
        leverage: t.leverage,
        fees: t.fees,
        auto_fees: false,
        funding: t.funding,
        planned_entry: t.planned_entry,
        stop_loss: t.stop_loss,
        take_profit: t.take_profit,
        notes: t.notes.clone(),
        playbook_id: t.playbook_id,
    }
}

/// The trade with `exit` applied, recomputed but not saved. Only manual
/// trades: imported ones are closed by their fills.
pub fn with_exit(existing: &Trade, exit: &CloseInput) -> Result<Trade, TradeError> {
    if !existing.is_manual {
        return Err(invalid(
            "imported trades close from their fills; import the closing trades instead",
        ));
    }
    if exit.exit_price.is_some() && exit.close_time.is_none() {
        return Err(invalid("add the date the trade was closed"));
    }
    let mut input = input_from(existing);
    input.avg_exit = exit.exit_price;
    input.close_time = exit.exit_price.and(exit.close_time);
    build_trade(&input, true)
}

/// What `close` would save, for the live preview.
pub fn preview_close(conn: &Connection, id: i64, exit: &CloseInput) -> Result<Trade, TradeError> {
    let existing = db::trades::get(conn, id)?.ok_or(TradeError::NotFound(id))?;
    with_exit(&existing.item, exit)
}

/// Saves an exit price on an open manual trade (or changes or clears it).
pub fn close(conn: &Connection, id: i64, exit: &CloseInput) -> Result<WithId<Trade>, TradeError> {
    let trade = preview_close(conn, id, exit)?;
    db::trades::update(conn, id, &trade)?;
    Ok(WithId { id, item: trade })
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), TradeError> {
    if db::trades::delete(conn, id)? {
        Ok(())
    } else {
        Err(TradeError::NotFound(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::open_in_memory;
    use rust_decimal_macros::dec;

    fn plan(
        direction: Direction,
        stop: Option<Decimal>,
        target: Option<Decimal>,
        size: Option<Decimal>,
    ) -> PlanInput {
        PlanInput {
            direction,
            entry: dec!(100),
            stop_loss: stop,
            take_profit: target,
            position_size: size,
        }
    }

    #[test]
    fn plan_preview_long_with_size() {
        // $500 at 100 = 5 coins, fee 0.1% = $0.50. Stop 95: −25 −0.5;
        // target 115: +75 −0.5; R:R 15 ÷ 5 = 3.
        let p = plan_preview(&plan(
            Direction::Long,
            Some(dec!(95)),
            Some(dec!(115)),
            Some(dec!(500)),
        ))
        .unwrap();
        assert_eq!(p.planned_rr, Some(dec!(3)));
        assert_eq!(p.loss_at_stop, Some(dec!(-25.5)));
        assert_eq!(p.profit_at_target, Some(dec!(74.5)));
        assert_eq!(p.fees, Some(dec!(0.5)));
    }

    #[test]
    fn plan_preview_short_and_partial_plans() {
        // Short: stop above, target below.
        let p = plan_preview(&plan(
            Direction::Short,
            Some(dec!(104)),
            Some(dec!(90)),
            Some(dec!(1000)),
        ))
        .unwrap();
        assert_eq!(p.planned_rr, Some(dec!(2.5)));
        assert_eq!(p.loss_at_stop, Some(dec!(-41))); // 10 × 4 + 1 fee
        assert_eq!(p.profit_at_target, Some(dec!(99))); // 10 × 10 − 1 fee

        // No size: R:R only.
        let p = plan_preview(&plan(
            Direction::Long,
            Some(dec!(95)),
            Some(dec!(110)),
            None,
        ))
        .unwrap();
        assert_eq!(p.planned_rr, Some(dec!(2)));
        assert_eq!(
            (p.loss_at_stop, p.profit_at_target, p.fees),
            (None, None, None)
        );

        // Stop only: loss, no R:R.
        let p = plan_preview(&plan(
            Direction::Long,
            Some(dec!(95)),
            None,
            Some(dec!(500)),
        ))
        .unwrap();
        assert_eq!(p.planned_rr, None);
        assert_eq!(p.loss_at_stop, Some(dec!(-25.5)));
        assert_eq!(p.profit_at_target, None);

        // Uneven quantity: $500 at 150 = 3.333… coins; stop 145 → −16.67 − 0.5.
        let mut uneven = plan(Direction::Long, Some(dec!(145)), None, Some(dec!(500)));
        uneven.entry = dec!(150);
        assert_eq!(
            plan_preview(&uneven).unwrap().loss_at_stop,
            Some(dec!(-17.17))
        );
    }

    #[test]
    fn plan_preview_rejects_bad_levels() {
        assert!(matches!(
            plan_preview(&plan(Direction::Long, Some(dec!(105)), None, None)),
            Err(TradeError::Risk(RiskError::StopOnWrongSide))
        ));
        assert!(matches!(
            plan_preview(&plan(Direction::Short, None, Some(dec!(110)), None)),
            Err(TradeError::Risk(RiskError::TargetOnWrongSide))
        ));
        assert!(matches!(
            plan_preview(&plan(Direction::Long, Some(dec!(0)), None, None)),
            Err(TradeError::Invalid(_))
        ));
        let mut zero_entry = plan(Direction::Long, None, None, None);
        zero_entry.entry = dec!(0);
        assert!(plan_preview(&zero_entry).is_err());
    }

    /// Closed long: in 100 → out 110, qty 2, stop 95, target 115.
    fn closed_long() -> TradeInput {
        TradeInput {
            position_size: None,
            auto_fees: false,
            exchange: "Binance".into(),
            market: Market::Usdm,
            symbol: " btcusdt ".into(),
            direction: Direction::Long,
            open_time: 1_000,
            close_time: Some(2_000),
            qty: dec!(2),
            avg_entry: dec!(100),
            avg_exit: Some(dec!(110)),
            leverage: Some(5),
            fees: dec!(0.4),
            funding: dec!(0.1),
            planned_entry: None,
            stop_loss: Some(dec!(95)),
            take_profit: Some(dec!(115)),
            notes: "test".into(),
            playbook_id: None,
        }
    }

    fn open_long() -> TradeInput {
        TradeInput {
            close_time: None,
            avg_exit: None,
            ..closed_long()
        }
    }

    fn exit(price: Option<Decimal>, time: Option<TimestampMs>) -> CloseInput {
        CloseInput {
            exit_price: price,
            close_time: time,
        }
    }

    #[test]
    fn close_later_works_out_pnl_and_r() {
        let conn = open_in_memory().unwrap();
        let saved = create(&conn, &open_long()).unwrap();
        assert_eq!(saved.item.status, TradeStatus::Open);

        let closed = close(&conn, saved.id, &exit(Some(dec!(110)), Some(5_000))).unwrap();

        // 2 coins × +10 = 20 gross; − 0.4 fees − 0.1 funding (as saved).
        let t = &closed.item;
        assert_eq!(t.status, TradeStatus::Closed);
        assert_eq!(t.avg_exit, Some(dec!(110)));
        assert_eq!(t.close_time, Some(5_000));
        assert_eq!(t.gross_pnl, dec!(20));
        assert_eq!(t.net_pnl, dec!(19.5));
        assert_eq!(t.r_multiple, Some(dec!(2)));
        // Everything else stays as it was.
        assert_eq!(
            (t.qty, t.fees, t.notes.as_str()),
            (dec!(2), dec!(0.4), "test")
        );
        assert_eq!(db::trades::get(&conn, saved.id).unwrap().unwrap().item, *t);
    }

    #[test]
    fn clearing_the_exit_reopens() {
        let conn = open_in_memory().unwrap();
        let saved = create(&conn, &closed_long()).unwrap();
        let t = close(&conn, saved.id, &exit(None, Some(5_000)))
            .unwrap()
            .item;
        assert_eq!(t.status, TradeStatus::Open);
        assert_eq!((t.avg_exit, t.close_time), (None, None));
        assert_eq!(t.net_pnl, dec!(-0.5));
        assert_eq!(t.r_multiple, None);
    }

    #[test]
    fn close_rejects_bad_exits() {
        let conn = open_in_memory().unwrap();
        let saved = create(&conn, &open_long()).unwrap();
        let err = |e: CloseInput| close(&conn, saved.id, &e).unwrap_err().to_string();
        assert!(err(exit(Some(dec!(110)), None)).contains("date"));
        assert!(err(exit(Some(dec!(0)), Some(5_000))).contains("exit price"));
        assert!(err(exit(Some(dec!(110)), Some(500))).contains("before open"));
        assert!(matches!(
            close(&conn, 999, &exit(None, None)),
            Err(TradeError::NotFound(999))
        ));
    }

    #[test]
    fn imported_trades_cannot_be_closed_by_hand() {
        let mut t = build_trade(&open_long(), false).unwrap();
        t.is_manual = false;
        let e = with_exit(&t, &exit(Some(dec!(110)), Some(5_000))).unwrap_err();
        assert!(e.to_string().contains("imported"));
    }

    #[test]
    fn position_size_sets_quantity_and_auto_fees() {
        // 1,000 USDT at 100 → 10 coins; fee 0.1% of 1,000 = 1.
        let input = TradeInput {
            qty: Decimal::ZERO, // ignored when a size is given
            position_size: Some(dec!(1000)),
            fees: dec!(99), // ignored with auto fees
            auto_fees: true,
            ..closed_long()
        };
        let t = build_trade(&input, true).unwrap();
        assert_eq!(t.qty, dec!(10));
        assert_eq!(t.fees, dec!(1));
        assert_eq!(t.gross_pnl, dec!(100)); // (110 − 100) × 10
        assert_eq!(t.net_pnl, dec!(98.9)); // 100 − 1 − 0.1 funding

        // Quantity keeps 8 decimals: 250 USDT at 64,123.45 BTC.
        let btc = TradeInput {
            avg_entry: dec!(64123.45),
            avg_exit: None,
            close_time: None,
            stop_loss: None,
            take_profit: None,
            position_size: Some(dec!(250)),
            ..input.clone()
        };
        let t = build_trade(&btc, true).unwrap();
        assert_eq!(t.qty, dec!(0.00389873)); // 250 / 64123.45 = 0.003898728…
        assert_eq!(t.fees, dec!(0.25));
    }

    #[test]
    fn auto_fees_from_quantity_and_typed_fees_win_otherwise() {
        // No size: fee = 0.1% of qty × entry = 0.1% of 200 = 0.2.
        let auto = TradeInput {
            auto_fees: true,
            ..closed_long()
        };
        assert_eq!(build_trade(&auto, true).unwrap().fees, dec!(0.2));
        // auto_fees false keeps the typed 0.4.
        assert_eq!(build_trade(&closed_long(), true).unwrap().fees, dec!(0.4));
    }

    #[test]
    fn position_size_must_be_positive_and_large_enough() {
        let zero = TradeInput {
            position_size: Some(Decimal::ZERO),
            ..closed_long()
        };
        assert_eq!(
            build_trade(&zero, true).unwrap_err().to_string(),
            "position size must be greater than 0"
        );
        let tiny = TradeInput {
            position_size: Some(dec!(0.000000001)),
            ..closed_long()
        };
        assert_eq!(
            build_trade(&tiny, true).unwrap_err().to_string(),
            "position size is too small for this price"
        );
    }

    #[test]
    fn closed_long_derives_pnl_and_r() {
        let t = build_trade(&closed_long(), true).unwrap();
        assert_eq!(t.status, TradeStatus::Closed);
        assert_eq!(t.gross_pnl, dec!(20)); // (110 − 100) × 2
        assert_eq!(t.net_pnl, dec!(19.5)); // 20 − 0.4 − 0.1
        assert_eq!(t.planned_rr, Some(dec!(3))); // 15 / 5
        assert_eq!(t.r_multiple, Some(dec!(2))); // 10 / 5
        assert_eq!(t.symbol, "BTCUSDT");
        assert_eq!(t.exchange, "binance");
        assert!(t.is_manual);
    }

    #[test]
    fn closed_short_loss() {
        let input = TradeInput {
            direction: Direction::Short,
            avg_entry: dec!(2000),
            avg_exit: Some(dec!(2060)),
            qty: dec!(0.5),
            stop_loss: Some(dec!(2040)),
            take_profit: Some(dec!(1900)),
            fees: dec!(1),
            funding: dec!(-0.25), // received funding increases net PnL
            ..closed_long()
        };
        let t = build_trade(&input, true).unwrap();
        assert_eq!(t.gross_pnl, dec!(-30)); // (2000 − 2060) × 0.5
        assert_eq!(t.net_pnl, dec!(-30.75)); // −30 − 1 + 0.25
        assert_eq!(t.planned_rr, Some(dec!(2.5)));
        assert_eq!(t.r_multiple, Some(dec!(-1.5)));
    }

    #[test]
    fn planned_entry_is_used_for_risk_when_given() {
        let input = TradeInput {
            planned_entry: Some(dec!(100)),
            avg_entry: dec!(101),
            ..closed_long()
        };
        let t = build_trade(&input, true).unwrap();
        assert_eq!(t.planned_rr, Some(dec!(3))); // plan: 100 → 115 vs 100 → 95
        assert_eq!(t.r_multiple, Some(dec!(1.8))); // (110 − 101) / 5
    }

    #[test]
    fn open_trade_has_no_pnl_or_r() {
        let input = TradeInput {
            avg_exit: None,
            close_time: None,
            ..closed_long()
        };
        let t = build_trade(&input, true).unwrap();
        assert_eq!(t.status, TradeStatus::Open);
        assert_eq!(t.gross_pnl, Decimal::ZERO);
        assert_eq!(t.net_pnl, dec!(-0.5)); // fees + funding already paid
        assert_eq!(t.planned_rr, Some(dec!(3)));
        assert_eq!(t.r_multiple, None);
    }

    #[test]
    fn no_stop_means_no_r_values() {
        let input = TradeInput {
            stop_loss: None,
            ..closed_long()
        };
        let t = build_trade(&input, true).unwrap();
        assert_eq!(t.planned_rr, None);
        assert_eq!(t.r_multiple, None);
        assert_eq!(t.gross_pnl, dec!(20));
    }

    #[test]
    fn invalid_inputs_are_rejected() {
        let cases: Vec<(&str, TradeInput)> = vec![
            (
                "symbol",
                TradeInput {
                    symbol: "  ".into(),
                    ..closed_long()
                },
            ),
            (
                "quantity",
                TradeInput {
                    qty: dec!(0),
                    ..closed_long()
                },
            ),
            (
                "entry",
                TradeInput {
                    avg_entry: dec!(-1),
                    ..closed_long()
                },
            ),
            (
                "exit",
                TradeInput {
                    avg_exit: Some(dec!(0)),
                    ..closed_long()
                },
            ),
            (
                "fees",
                TradeInput {
                    fees: dec!(-0.01),
                    ..closed_long()
                },
            ),
            (
                "leverage",
                TradeInput {
                    leverage: Some(0),
                    ..closed_long()
                },
            ),
            (
                "both",
                TradeInput {
                    close_time: None,
                    ..closed_long()
                },
            ),
            (
                "both",
                TradeInput {
                    avg_exit: None,
                    ..closed_long()
                },
            ),
            (
                "before",
                TradeInput {
                    close_time: Some(500),
                    ..closed_long()
                },
            ),
            (
                "stop loss",
                TradeInput {
                    stop_loss: Some(dec!(-5)),
                    ..closed_long()
                },
            ),
        ];
        for (expected, input) in cases {
            let err = build_trade(&input, true).unwrap_err().to_string();
            assert!(err.contains(expected), "expected {expected:?} in {err:?}");
        }
    }

    #[test]
    fn stop_or_target_on_wrong_side_is_rejected() {
        let bad_stop = TradeInput {
            stop_loss: Some(dec!(105)),
            ..closed_long()
        };
        assert!(matches!(
            build_trade(&bad_stop, true),
            Err(TradeError::Risk(RiskError::StopOnWrongSide))
        ));
        // Checked even without a stop, so a typo is caught early.
        let bad_target = TradeInput {
            stop_loss: None,
            take_profit: Some(dec!(90)),
            ..closed_long()
        };
        assert!(matches!(
            build_trade(&bad_target, true),
            Err(TradeError::Risk(RiskError::TargetOnWrongSide))
        ));
    }

    #[test]
    fn create_update_delete_round_trip() {
        let conn = open_in_memory().unwrap();
        let created = create(&conn, &closed_long()).unwrap();
        assert_eq!(
            db::trades::get(&conn, created.id).unwrap(),
            Some(created.clone())
        );

        let edited = TradeInput {
            avg_exit: Some(dec!(90)),
            notes: "stopped out".into(),
            ..closed_long()
        };
        let updated = update(&conn, created.id, &edited).unwrap();
        assert_eq!(updated.item.gross_pnl, dec!(-20));
        assert_eq!(updated.item.r_multiple, Some(dec!(-2)));
        assert_eq!(db::trades::get(&conn, created.id).unwrap(), Some(updated));

        delete(&conn, created.id).unwrap();
        assert_eq!(db::trades::get(&conn, created.id).unwrap(), None);
    }

    #[test]
    fn update_keeps_imported_flag() {
        let conn = open_in_memory().unwrap();
        let imported = build_trade(&closed_long(), false).unwrap();
        let id = db::trades::insert(&conn, &imported).unwrap();

        let updated = update(&conn, id, &closed_long()).unwrap();
        assert!(!updated.item.is_manual);
    }

    #[test]
    fn update_and_delete_unknown_id_report_not_found() {
        let conn = open_in_memory().unwrap();
        assert!(matches!(
            update(&conn, 42, &closed_long()),
            Err(TradeError::NotFound(42))
        ));
        assert!(matches!(delete(&conn, 42), Err(TradeError::NotFound(42))));
    }

    #[test]
    fn invalid_update_leaves_stored_trade_unchanged() {
        let conn = open_in_memory().unwrap();
        let created = create(&conn, &closed_long()).unwrap();
        let bad = TradeInput {
            qty: dec!(0),
            ..closed_long()
        };
        assert!(update(&conn, created.id, &bad).is_err());
        assert_eq!(db::trades::get(&conn, created.id).unwrap(), Some(created));
    }

    #[test]
    fn input_deserializes_from_ui_json() {
        let json = serde_json::json!({
            "exchange": "binance",
            "market": "spot",
            "symbol": "ETHUSDT",
            "direction": "long",
            "openTime": 1_700_000_000_000_i64,
            "qty": "0.5",
            "avgEntry": "2000.25",
            "stopLoss": "1950"
        });
        let input: TradeInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.avg_entry, dec!(2000.25));
        assert_eq!(input.stop_loss, Some(dec!(1950)));
        assert_eq!(input.fees, Decimal::ZERO); // defaulted
        assert_eq!(input.avg_exit, None);
    }
}
