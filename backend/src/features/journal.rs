//! Editing a trade's journal: the plan (planned entry, stop, target),
//! leverage and notes. Works the same for manual and imported trades and
//! never touches the numbers that come from fills (qty, prices, PnL).
//! Regrouping keeps these fields (see `grouping::persist::merge_journal`).

use rusqlite::Connection;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::database as db;
use crate::features::risk;
use crate::features::trade_entry::TradeError;
use crate::models::{Trade, TradeStatus, WithId};

/// What the trade detail page sends.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalInput {
    pub planned_entry: Option<Decimal>,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
    pub leverage: Option<u32>,
    #[serde(default)]
    pub notes: String,
}

fn invalid(msg: &str) -> TradeError {
    TradeError::Invalid(msg.to_string())
}

/// Returns the trade with the journal applied and R values recomputed.
/// Unlike a regroup (which quietly drops an outdated plan), a plan typed by
/// the user that doesn't fit is an error, so they can fix it.
pub fn apply(trade: &Trade, j: &JournalInput) -> Result<Trade, TradeError> {
    let positive_if_set = |v: Option<Decimal>| v.is_none_or(|d| d > Decimal::ZERO);
    if ![j.planned_entry, j.stop_loss, j.take_profit]
        .into_iter()
        .all(positive_if_set)
    {
        return Err(invalid(
            "planned entry, stop loss and take profit must be greater than 0",
        ));
    }
    if j.leverage == Some(0) {
        return Err(invalid("leverage must be at least 1"));
    }

    let dir = trade.direction;
    let plan_entry = j.planned_entry.unwrap_or(trade.avg_entry);
    if let Some(stop) = j.stop_loss {
        risk::risk_per_unit(dir, plan_entry, stop)?;
    }
    if let Some(target) = j.take_profit {
        risk::reward_per_unit(dir, plan_entry, target)?;
    }
    let planned_rr = match (j.stop_loss, j.take_profit) {
        (Some(stop), Some(target)) => Some(risk::planned_rr(dir, plan_entry, stop, target)?),
        _ => None,
    };
    let r_multiple = match (j.stop_loss, trade.avg_exit, trade.status) {
        (Some(stop), Some(exit), TradeStatus::Closed) => Some(risk::r_multiple(
            dir,
            plan_entry,
            stop,
            trade.avg_entry,
            exit,
        )?),
        _ => None,
    };

    Ok(Trade {
        planned_entry: j.planned_entry,
        stop_loss: j.stop_loss,
        take_profit: j.take_profit,
        leverage: j.leverage,
        notes: j.notes.clone(),
        planned_rr,
        r_multiple,
        ..trade.clone()
    })
}

fn load(conn: &Connection, id: i64) -> Result<WithId<Trade>, TradeError> {
    db::trades::get(conn, id)?.ok_or(TradeError::NotFound(id))
}

/// What the trade would look like with this journal. Saves nothing.
pub fn preview(conn: &Connection, id: i64, j: &JournalInput) -> Result<Trade, TradeError> {
    apply(&load(conn, id)?.item, j)
}

pub fn update(conn: &Connection, id: i64, j: &JournalInput) -> Result<WithId<Trade>, TradeError> {
    let trade = apply(&load(conn, id)?.item, j)?;
    db::trades::update(conn, id, &trade)?;
    Ok(WithId { id, item: trade })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{open_in_memory, test_data};
    use crate::features::grouping::persist::{Instrument, regroup_instrument};
    use crate::features::risk::RiskError;
    use crate::models::{Direction, Fill, Market, Side};
    use rust_decimal_macros::dec;

    /// Closed long: entry 100 → exit 110.
    fn long() -> Trade {
        Trade {
            direction: Direction::Long,
            status: TradeStatus::Closed,
            avg_entry: dec!(100),
            avg_exit: Some(dec!(110)),
            planned_entry: None,
            stop_loss: None,
            take_profit: None,
            planned_rr: None,
            r_multiple: None,
            notes: String::new(),
            ..test_data::trade()
        }
    }

    fn plan(stop: Option<Decimal>, target: Option<Decimal>) -> JournalInput {
        JournalInput {
            stop_loss: stop,
            take_profit: target,
            notes: "## Setup\nbreakout".into(),
            ..JournalInput::default()
        }
    }

    #[test]
    fn computes_rr_and_r_and_keeps_fill_numbers() {
        let t = apply(&long(), &plan(Some(dec!(95)), Some(dec!(115)))).unwrap();
        assert_eq!(t.planned_rr, Some(dec!(3)));
        assert_eq!(t.r_multiple, Some(dec!(2)));
        assert_eq!(t.notes, "## Setup\nbreakout");
        let before = long();
        assert_eq!(
            (t.qty, t.avg_entry, t.net_pnl),
            (before.qty, before.avg_entry, before.net_pnl)
        );
    }

    #[test]
    fn open_trade_gets_rr_but_no_r() {
        let open = Trade {
            status: TradeStatus::Open,
            avg_exit: None,
            ..long()
        };
        let t = apply(&open, &plan(Some(dec!(95)), Some(dec!(115)))).unwrap();
        assert_eq!((t.planned_rr, t.r_multiple), (Some(dec!(3)), None));
    }

    #[test]
    fn clearing_the_plan_clears_r_values() {
        let planned = apply(&long(), &plan(Some(dec!(95)), Some(dec!(115)))).unwrap();
        let cleared = apply(&planned, &plan(None, None)).unwrap();
        assert_eq!((cleared.planned_rr, cleared.r_multiple), (None, None));
    }

    #[test]
    fn invalid_plans_are_errors() {
        assert!(matches!(
            apply(&long(), &plan(Some(dec!(105)), None)),
            Err(TradeError::Risk(RiskError::StopOnWrongSide))
        ));
        assert!(matches!(
            apply(&long(), &plan(None, Some(dec!(90)))),
            Err(TradeError::Risk(RiskError::TargetOnWrongSide))
        ));
        assert!(apply(&long(), &plan(Some(dec!(-1)), None)).is_err());
        let zero_lev = JournalInput {
            leverage: Some(0),
            ..JournalInput::default()
        };
        assert!(apply(&long(), &zero_lev).is_err());
    }

    #[test]
    fn update_saves_and_preview_does_not() {
        let conn = open_in_memory().unwrap();
        let id = db::trades::insert(&conn, &long()).unwrap();
        let j = plan(Some(dec!(95)), Some(dec!(115)));

        let previewed = preview(&conn, id, &j).unwrap();
        assert_eq!(previewed.r_multiple, Some(dec!(2)));
        assert_eq!(
            db::trades::get(&conn, id).unwrap().unwrap().item.r_multiple,
            None
        );

        update(&conn, id, &j).unwrap();
        assert_eq!(
            db::trades::get(&conn, id).unwrap().unwrap().item.r_multiple,
            Some(dec!(2))
        );
        assert!(matches!(
            update(&conn, 999, &j),
            Err(TradeError::NotFound(999))
        ));
    }

    #[test]
    fn journal_on_imported_trade_survives_regroup() {
        let conn = open_in_memory().unwrap();
        let fill = |hash: &str, time, side, price| Fill {
            exchange: "binance".into(),
            market: Market::Usdm,
            symbol: "BTCUSDT".into(),
            side,
            qty: dec!(1),
            price,
            fee: Decimal::ZERO,
            fee_asset: "USDT".into(),
            realized_pnl: None,
            time,
            dedupe_hash: hash.into(),
            import_batch_id: None,
        };
        db::fills::insert(&conn, &fill("a", 1, Side::Buy, dec!(100))).unwrap();
        db::fills::insert(&conn, &fill("b", 2, Side::Sell, dec!(110))).unwrap();
        let inst = Instrument {
            exchange: "binance".into(),
            market: Market::Usdm,
            symbol: "BTCUSDT".into(),
        };
        regroup_instrument(&conn, &inst).unwrap();
        let id = db::trades::list(&conn).unwrap()[0].id;

        update(&conn, id, &plan(Some(dec!(95)), Some(dec!(115)))).unwrap();
        regroup_instrument(&conn, &inst).unwrap();

        let t = db::trades::get(&conn, id).unwrap().unwrap().item;
        assert_eq!(t.notes, "## Setup\nbreakout");
        assert_eq!((t.planned_rr, t.r_multiple), (Some(dec!(3)), Some(dec!(2))));
    }

    #[test]
    fn input_deserializes_from_ui_json() {
        let j: JournalInput = serde_json::from_value(serde_json::json!({
            "stopLoss": "95.5",
            "leverage": 10
        }))
        .unwrap();
        assert_eq!(j.stop_loss, Some(dec!(95.5)));
        assert_eq!(j.leverage, Some(10));
        assert_eq!(j.notes, "");
    }
}
