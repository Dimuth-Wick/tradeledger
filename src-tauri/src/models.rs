//! Domain types shared by the DB layer, the core logic and the UI.
//!
//! Serialized with camelCase so TypeScript sees `avgEntry`, not `avg_entry`.
//! `Decimal` serializes as a string ("0.00012345") so JavaScript never turns
//! money into a rounded float.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Milliseconds since 1970-01-01 UTC (same unit as JavaScript `Date.now()`).
pub type TimestampMs = i64;

/// A stored row: the database id plus the item's own fields.
/// `flatten` puts `id` next to the other fields in JSON: `{ "id": 1, "symbol": ... }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithId<T> {
    pub id: i64,
    #[serde(flatten)]
    pub item: T,
}

/// Defines an enum stored as fixed lowercase text, both in SQLite and JSON.
/// Generates `as_str()`, `FromStr`, and the rusqlite read/write impls, so
/// every enum maps to its text the same way.
macro_rules! text_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub enum $name {
            $( #[serde(rename = $text)] $variant ),+
        }

        impl $name {
            pub fn as_str(self) -> &'static str {
                match self { $( Self::$variant => $text ),+ }
            }
        }

        impl std::str::FromStr for $name {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $( $text => Ok(Self::$variant), )+
                    other => Err(format!("invalid {}: {other:?}", stringify!($name))),
                }
            }
        }

        impl rusqlite::types::ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
                Ok(self.as_str().into())
            }
        }

        impl rusqlite::types::FromSql for $name {
            fn column_result(
                value: rusqlite::types::ValueRef<'_>,
            ) -> rusqlite::types::FromSqlResult<Self> {
                value
                    .as_str()?
                    .parse()
                    .map_err(|e: String| rusqlite::types::FromSqlError::Other(e.into()))
            }
        }
    };
}

text_enum!(
    /// Which Binance product the trade happened on.
    Market { Spot => "spot", Usdm => "usdm" }
);
text_enum!(Side { Buy => "buy", Sell => "sell" });
text_enum!(Direction { Long => "long", Short => "short" });
text_enum!(TradeStatus { Open => "open", Closed => "closed" });

/// One execution reported by the exchange (a CSV row).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fill {
    pub exchange: String,
    pub market: Market,
    pub symbol: String,
    pub side: Side,
    pub qty: Decimal,
    pub price: Decimal,
    pub fee: Decimal,
    pub fee_asset: String,
    /// Futures only: PnL the exchange already computed for this fill.
    pub realized_pnl: Option<Decimal>,
    pub time: TimestampMs,
    /// Fingerprint of the source row; identical rows are imported once.
    pub dedupe_hash: String,
    // Which trade(s) use this fill lives in `fill_allocations`, because a
    // fill that flips a position belongs to two trades.
    pub import_batch_id: Option<i64>,
}

/// A position from open to flat, plus the journal data the user adds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trade {
    pub exchange: String,
    pub market: Market,
    pub symbol: String,
    pub direction: Direction,
    pub status: TradeStatus,
    pub open_time: TimestampMs,
    pub close_time: Option<TimestampMs>,
    pub qty: Decimal,
    pub avg_entry: Decimal,
    pub avg_exit: Option<Decimal>,
    pub leverage: Option<u32>,
    pub gross_pnl: Decimal,
    pub fees: Decimal,
    pub funding: Decimal,
    /// gross_pnl − fees − funding (computed in Rust, stored for fast queries).
    pub net_pnl: Decimal,
    // Risk plan (all optional: the user may not have planned the trade).
    pub planned_entry: Option<Decimal>,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
    pub planned_rr: Option<Decimal>,
    pub r_multiple: Option<Decimal>,
    pub notes: String,
    pub playbook_id: Option<i64>,
    /// true = entered by hand, false = built from imported fills.
    pub is_manual: bool,
}

/// Futures funding payment. Negative = paid, positive = received.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingEvent {
    pub exchange: String,
    pub symbol: String,
    pub amount: Decimal,
    pub time: TimestampMs,
    pub dedupe_hash: String,
    pub trade_id: Option<i64>,
    pub import_batch_id: Option<i64>,
}

/// Record of one CSV import.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportBatch {
    pub exchange: String,
    pub market: Market,
    pub file_name: String,
    pub imported_at: TimestampMs,
    pub rows_total: u32,
    pub rows_inserted: u32,
    pub rows_duplicate: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn enums_round_trip_through_text() {
        assert_eq!(Market::Usdm.as_str(), "usdm");
        assert_eq!(Market::from_str("spot"), Ok(Market::Spot));
        assert_eq!(Side::from_str("sell"), Ok(Side::Sell));
        assert_eq!(Direction::from_str("short"), Ok(Direction::Short));
        assert_eq!(TradeStatus::from_str("open"), Ok(TradeStatus::Open));
        assert!(Market::from_str("SPOT").is_err());
    }

    #[test]
    fn json_uses_camel_case_and_decimal_strings() {
        let item = WithId {
            id: 7,
            item: FundingEvent {
                exchange: "binance".into(),
                symbol: "BTCUSDT".into(),
                amount: Decimal::from_str("-0.00012345").unwrap(),
                time: 1_700_000_000_000,
                dedupe_hash: "h".into(),
                trade_id: None,
                import_batch_id: Some(3),
            },
        };
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["id"], 7);
        assert_eq!(json["amount"], "-0.00012345");
        assert_eq!(json["importBatchId"], 3);
        assert!(json.get("import_batch_id").is_none());
    }
}
