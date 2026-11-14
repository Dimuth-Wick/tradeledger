//! Binance spot "Trade History" export. Column details: docs/binance-csv.md §1.
//!
//! Two layouts are accepted:
//! - current: `Date(UTC),Pair,Side,Price,Executed,Amount,Fee`
//!   (Executed = "0.3ETH", Fee = "0.0003ETH": number and asset glued together)
//! - older:   `Date(UTC),Market,Type,Price,Amount,Total,Fee,Fee Coin`
//!   (Amount = quantity, Fee and Fee Coin in separate columns)

use rust_decimal::Decimal;

use super::common::{self, Deduper, Header, amount_with_asset, decimal, field, key_dec};
use super::{ExchangeImporter, ImportError, ParseReport};
use crate::models::{Fill, Market};

const NAME: &str = "Binance spot trade history";

// Normalized header names (see `common::normalize_header`).
const TIME: &[&str] = &["date(utc)", "time", "date"];
const SYMBOL: &[&str] = &["pair", "market", "symbol"];
const SIDE: &[&str] = &["side", "type"];
const PRICE: &[&str] = &["price"];

pub struct BinanceSpot;

/// Where quantity and fee come from, depending on the layout.
enum Layout {
    /// Executed = "0.3ETH", Fee = "0.0003ETH".
    Current { executed: usize, fee: usize },
    /// Amount = "0.3", Fee = "0.0003", Fee Coin = "ETH".
    Older {
        amount: usize,
        fee: usize,
        fee_coin: usize,
    },
}

struct Columns {
    time: usize,
    symbol: usize,
    side: usize,
    price: usize,
    layout: Layout,
}

impl Columns {
    fn locate(h: &Header) -> Result<Self, ImportError> {
        let layout = if h.has("executed") {
            Layout::Current {
                executed: h.require(NAME, &["executed"])?,
                fee: h.require(NAME, &["fee"])?,
            }
        } else {
            Layout::Older {
                amount: h.require(NAME, &["amount"])?,
                fee: h.require(NAME, &["fee"])?,
                fee_coin: h.require(NAME, &["feecoin"])?,
            }
        };
        Ok(Columns {
            time: h.require(NAME, TIME)?,
            symbol: h.require(NAME, SYMBOL)?,
            side: h.require(NAME, SIDE)?,
            price: h.require(NAME, PRICE)?,
            layout,
        })
    }
}

impl ExchangeImporter for BinanceSpot {
    fn kind(&self) -> &'static str {
        "binance-spot"
    }

    fn label(&self) -> &'static str {
        NAME
    }

    fn exchange(&self) -> &'static str {
        "binance"
    }

    fn market(&self) -> Market {
        Market::Spot
    }

    fn matches_headers(&self, headers: &[String]) -> bool {
        let has = |names: &[&str]| names.iter().any(|n| headers.iter().any(|h| h == n));
        let current = has(&["executed"]) && has(&["fee"]);
        let older = has(&["amount"]) && has(&["total"]) && has(&["feecoin"]);
        has(TIME) && has(&["pair", "market"]) && has(SIDE) && has(PRICE) && (current || older)
    }

    fn parse(&self, data: &[u8]) -> Result<ParseReport, ImportError> {
        let (header, rdr) = common::open(data)?;
        // A wrong header fails the whole file; a bad row only fails that row.
        let cols = Columns::locate(&header)?;
        let mut deduper = Deduper::new(self.kind());
        let mut fills = Vec::new();
        let errors = common::each_row(rdr, |record| {
            fills.push(parse_row(&cols, record, &mut deduper)?);
            Ok(())
        });
        Ok(ParseReport {
            fills,
            errors,
            ..ParseReport::default()
        })
    }
}

fn parse_row(cols: &Columns, r: &csv::StringRecord, deduper: &mut Deduper) -> Result<Fill, String> {
    let time = common::utc_time(field(r, cols.time))?;
    let symbol = field(r, cols.symbol).to_uppercase();
    if symbol.is_empty() {
        return Err("pair is empty".into());
    }
    let side = common::side(field(r, cols.side))?;
    let price = decimal("price", field(r, cols.price))?;

    let (qty, fee, fee_asset) = match cols.layout {
        Layout::Current { executed, fee } => {
            let (qty, _base) = amount_with_asset("executed", field(r, executed))?;
            let (fee, asset) = amount_with_asset("fee", field(r, fee))?;
            (qty, fee, asset)
        }
        Layout::Older {
            amount,
            fee,
            fee_coin,
        } => (
            decimal("amount", field(r, amount))?,
            decimal("fee", field(r, fee))?,
            field(r, fee_coin).to_uppercase(),
        ),
    };

    if price <= Decimal::ZERO {
        return Err(format!("price must be greater than 0: {price}"));
    }
    if qty <= Decimal::ZERO {
        return Err(format!("quantity must be greater than 0: {qty}"));
    }
    if fee < Decimal::ZERO {
        return Err(format!("fee cannot be negative: {fee}"));
    }

    let dedupe_hash = deduper.hash(&[
        time.to_string(),
        symbol.clone(),
        side.as_str().to_string(),
        key_dec(price),
        key_dec(qty),
        key_dec(fee),
        fee_asset.clone(),
    ]);

    Ok(Fill {
        exchange: "binance".into(),
        market: Market::Spot,
        symbol,
        side,
        qty,
        price,
        fee,
        fee_asset,
        realized_pnl: None,
        time,
        dedupe_hash,
        trade_id: None,
        import_batch_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::importers::RowError;
    use crate::models::Side;
    use rust_decimal_macros::dec;

    const FIXTURE: &[u8] = include_bytes!("../../../tests/fixtures/binance-spot-trades.csv");

    fn ms(text: &str) -> i64 {
        common::utc_time(text).unwrap()
    }

    #[test]
    fn parses_every_fixture_row() {
        let report = BinanceSpot.parse(FIXTURE).unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(report.fills.len(), 8);

        let first = &report.fills[0];
        assert_eq!(first.time, ms("2026-01-05 09:15:02"));
        assert_eq!(first.symbol, "ETHUSDT");
        assert_eq!(first.side, Side::Buy);
        assert_eq!(first.price, dec!(2000.25));
        assert_eq!(first.qty, dec!(0.3));
        assert_eq!(first.fee, dec!(0.0003));
        assert_eq!(first.fee_asset, "ETH");
        assert_eq!(first.market, Market::Spot);
        assert_eq!(first.realized_pnl, None);

        let sell = &report.fills[2];
        assert_eq!(sell.side, Side::Sell);
        assert_eq!(sell.qty, dec!(0.4995));
        assert_eq!((sell.fee, sell.fee_asset.as_str()), (dec!(1.04895), "USDT"));

        let bnb = &report.fills[3];
        assert_eq!((bnb.fee, bnb.fee_asset.as_str()), (dec!(0.000125), "BNB"));
    }

    #[test]
    fn identical_rows_get_different_hashes_and_reparse_is_stable() {
        let a = BinanceSpot.parse(FIXTURE).unwrap();
        let b = BinanceSpot.parse(FIXTURE).unwrap();
        // Rows 4 and 5 (index 3, 4) are identical SOL buys.
        assert_eq!(a.fills[3].qty, a.fills[4].qty);
        assert_ne!(a.fills[3].dedupe_hash, a.fills[4].dedupe_hash);

        let hashes = |r: &ParseReport| {
            r.fills
                .iter()
                .map(|f| f.dedupe_hash.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(hashes(&a), hashes(&b));
    }

    #[test]
    fn number_formatting_does_not_change_the_hash() {
        let plain = b"Date(UTC),Pair,Side,Price,Executed,Amount,Fee\n\
            2026-01-05 09:15:02,ETHUSDT,BUY,2000.25,0.3ETH,600.075USDT,0.0003ETH\n";
        let padded = b"Date(UTC),Pair,Side,Price,Executed,Amount,Fee\n\
            2026-01-05 09:15:02,ethusdt,buy,2000.2500,0.30000000ETH,600.07500000USDT,0.00030000ETH\n";
        let a = BinanceSpot.parse(plain).unwrap();
        let b = BinanceSpot.parse(padded).unwrap();
        assert_eq!(a.fills[0].dedupe_hash, b.fills[0].dedupe_hash);
    }

    #[test]
    fn accepts_bom_shuffled_columns_and_quotes() {
        let data = "\u{feff}Fee,Side,Pair,Executed,Price,Date(UTC),Amount\r\n\
            \"0.00030000ETH\",BUY,ETHUSDT,\"0.30000000ETH\",\"2000.25\",2026-01-05 09:15:02,600.075USDT\r\n";
        let report = BinanceSpot.parse(data.as_bytes()).unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(report.fills[0].qty, dec!(0.3));
        assert_eq!(report.fills[0].price, dec!(2000.25));
    }

    #[test]
    fn parses_older_layout_with_fee_coin_column() {
        let data = b"Date(UTC),Market,Type,Price,Amount,Total,Fee,Fee Coin\n\
            2026-01-05 09:15:02,ETHUSDT,SELL,2000.25,0.3,600.075,0.6,usdt\n";
        assert!(BinanceSpot.matches_headers(&common::read_headers(data).unwrap()));
        let fill = &BinanceSpot.parse(data).unwrap().fills[0];
        assert_eq!(fill.side, Side::Sell);
        assert_eq!(fill.qty, dec!(0.3));
        assert_eq!((fill.fee, fill.fee_asset.as_str()), (dec!(0.6), "USDT"));
    }

    #[test]
    fn bad_rows_are_reported_with_line_numbers_and_others_still_parse() {
        let data = b"Date(UTC),Pair,Side,Price,Executed,Amount,Fee\n\
            2026-01-05 09:15:02,ETHUSDT,BUY,2000.25,0.3ETH,600.075USDT,0.0003ETH\n\
            2026-01-05 09:15:03,ETHUSDT,HOLD,2000.25,0.3ETH,600.075USDT,0.0003ETH\n\
            2026-01-05 09:15:04,ETHUSDT,BUY,abc,0.3ETH,600.075USDT,0.0003ETH\n\
            2026-01-05,ETHUSDT,BUY,2000.25,0.3ETH,600.075USDT,0.0003ETH\n\
            2026-01-05 09:15:06,ETHUSDT,BUY,2000.25,0ETH,0USDT,0ETH\n\
            2026-01-05 09:15:07,ETHUSDT,SELL\n\
            2026-01-05 09:15:08,ETHUSDT,SELL,2001,0.3ETH,600.3USDT,0.6USDT\n";
        let report = BinanceSpot.parse(data).unwrap();
        assert_eq!(report.fills.len(), 2);
        let lines: Vec<u64> = report.errors.iter().map(|e: &RowError| e.line).collect();
        assert_eq!(lines, [3, 4, 5, 6, 7]);
        assert!(report.errors[0].message.contains("BUY or SELL"));
        assert!(report.errors[1].message.contains("price"));
        assert!(report.errors[2].message.contains("time"));
        assert!(report.errors[3].message.contains("quantity"));
    }

    #[test]
    fn header_only_file_is_empty_not_an_error() {
        let report = BinanceSpot
            .parse(b"Date(UTC),Pair,Side,Price,Executed,Amount,Fee\n")
            .unwrap();
        assert_eq!(report, ParseReport::default());
    }

    #[test]
    fn missing_column_is_a_clear_error() {
        let data = b"Date(UTC),Pair,Side,Executed,Amount,Fee\n\
            2026-01-05 09:15:02,ETHUSDT,BUY,0.3ETH,600.075USDT,0.0003ETH\n";
        let err = BinanceSpot.parse(data).unwrap_err();
        assert!(
            matches!(
                err,
                ImportError::MissingColumn {
                    column: "price",
                    ..
                }
            ),
            "{err}"
        );
        // Same check for a file with no data rows.
        assert!(BinanceSpot.parse(b"Date(UTC),Pair\n").is_err());
    }
}
