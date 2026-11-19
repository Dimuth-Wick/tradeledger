//! Binance USDⓈ-M futures exports. Column details: docs/binance-csv.md §2–3.
//!
//! - `BinanceUsdmTrades`: Trade History → fills (with Binance's realized PnL).
//! - `BinanceUsdmTransactions`: Transaction History → funding fees only.
//!   Other transaction types (commission, realized PnL, transfers) are skipped
//!   because the trade history already contains them; importing them again
//!   would count fees and PnL twice.

use rust_decimal::Decimal;

use super::common::{self, Deduper, Header, amount_with_asset, decimal, field, key_dec};
use super::{ExchangeImporter, ImportError, ParseReport};
use crate::models::{Fill, FundingEvent, Market};

const TRADES_NAME: &str = "Binance USDⓈ-M futures trade history";
const TX_NAME: &str = "Binance USDⓈ-M futures transaction history";

const TIME: &[&str] = &["date(utc)", "time", "date"];
const SYMBOL: &[&str] = &["symbol"];
const SIDE: &[&str] = &["side"];
const PRICE: &[&str] = &["price"];
const QTY: &[&str] = &["quantity", "qty"];
const FEE: &[&str] = &["fee", "commission"];
const FEE_COIN: &[&str] = &["feecoin", "commissionasset"];
const REALIZED: &[&str] = &["realizedprofit", "realizedpnl"];
const TYPE: &[&str] = &["type"];
const AMOUNT: &[&str] = &["amount"];
const ASSET: &[&str] = &["asset", "coin"];

fn has_any(headers: &[String], names: &[&str]) -> bool {
    names.iter().any(|n| headers.iter().any(|h| h == n))
}

// ───────────────────────────── trade history ─────────────────────────────

pub struct BinanceUsdmTrades;

struct TradeColumns {
    time: usize,
    symbol: usize,
    side: usize,
    price: usize,
    qty: usize,
    fee: usize,
    /// `None` = fee text carries its asset ("0.1464USDT").
    fee_coin: Option<usize>,
    /// Optional: older exports may not have it.
    realized: Option<usize>,
}

impl TradeColumns {
    fn locate(h: &Header) -> Result<Self, ImportError> {
        Ok(Self {
            time: h.require(TRADES_NAME, TIME)?,
            symbol: h.require(TRADES_NAME, SYMBOL)?,
            side: h.require(TRADES_NAME, SIDE)?,
            price: h.require(TRADES_NAME, PRICE)?,
            qty: h.require(TRADES_NAME, QTY)?,
            fee: h.require(TRADES_NAME, FEE)?,
            fee_coin: h.find(FEE_COIN),
            realized: h.find(REALIZED),
        })
    }
}

impl ExchangeImporter for BinanceUsdmTrades {
    fn kind(&self) -> &'static str {
        "binance-usdm"
    }

    fn label(&self) -> &'static str {
        TRADES_NAME
    }

    fn exchange(&self) -> &'static str {
        "binance"
    }

    fn market(&self) -> Market {
        Market::Usdm
    }

    fn matches_headers(&self, headers: &[String]) -> bool {
        // "Realized Profit" is what tells futures trades apart from spot.
        [TIME, SYMBOL, SIDE, PRICE, QTY, REALIZED]
            .iter()
            .all(|names| has_any(headers, names))
    }

    fn parse(&self, data: &[u8]) -> Result<ParseReport, ImportError> {
        let (header, rdr) = common::open(data)?;
        let cols = TradeColumns::locate(&header)?;
        let mut deduper = Deduper::new(self.kind());
        let mut fills = Vec::new();
        let errors = common::each_row(rdr, |record| {
            fills.push(parse_trade_row(&cols, record, &mut deduper)?);
            Ok(())
        });
        Ok(ParseReport {
            fills,
            errors,
            ..ParseReport::default()
        })
    }
}

fn parse_trade_row(
    cols: &TradeColumns,
    r: &csv::StringRecord,
    deduper: &mut Deduper,
) -> Result<Fill, String> {
    let time = common::utc_time(field(r, cols.time))?;
    let symbol = field(r, cols.symbol).to_uppercase();
    if symbol.is_empty() {
        return Err("symbol is empty".into());
    }
    let side = common::side(field(r, cols.side))?;
    let price = decimal("price", field(r, cols.price))?;
    let qty = decimal("quantity", field(r, cols.qty))?;
    let (fee, fee_asset) = match cols.fee_coin {
        Some(coin) => (
            decimal("fee", field(r, cols.fee))?,
            field(r, coin).to_uppercase(),
        ),
        None => amount_with_asset("fee", field(r, cols.fee))?,
    };
    // Fees may be negative: maker rebates on some fee tiers are paid to you.
    let realized_pnl = match cols.realized {
        Some(idx) => Some(decimal("realized profit", field(r, idx))?),
        None => None,
    };

    if price <= Decimal::ZERO {
        return Err(format!("price must be greater than 0: {price}"));
    }
    if qty <= Decimal::ZERO {
        return Err(format!("quantity must be greater than 0: {qty}"));
    }

    let dedupe_hash = deduper.hash(&[
        time.to_string(),
        symbol.clone(),
        side.as_str().to_string(),
        key_dec(price),
        key_dec(qty),
        key_dec(fee),
        fee_asset.clone(),
        realized_pnl.map(key_dec).unwrap_or_default(),
    ]);

    Ok(Fill {
        exchange: "binance".into(),
        market: Market::Usdm,
        symbol,
        side,
        qty,
        price,
        fee,
        fee_asset,
        realized_pnl,
        time,
        dedupe_hash,
        import_batch_id: None,
    })
}

// ─────────────────────────── transaction history ──────────────────────────

pub struct BinanceUsdmTransactions;

struct TxColumns {
    time: usize,
    kind: usize,
    amount: usize,
    asset: usize,
    symbol: usize,
}

impl TxColumns {
    fn locate(h: &Header) -> Result<Self, ImportError> {
        Ok(Self {
            time: h.require(TX_NAME, TIME)?,
            kind: h.require(TX_NAME, TYPE)?,
            amount: h.require(TX_NAME, AMOUNT)?,
            asset: h.require(TX_NAME, ASSET)?,
            symbol: h.require(TX_NAME, SYMBOL)?,
        })
    }
}

/// "FUNDING_FEE", "Funding Fee", "funding fee" → true.
fn is_funding(kind: &str) -> bool {
    kind.trim().to_ascii_uppercase().replace(' ', "_") == "FUNDING_FEE"
}

impl ExchangeImporter for BinanceUsdmTransactions {
    fn kind(&self) -> &'static str {
        "binance-usdm-tx"
    }

    fn label(&self) -> &'static str {
        TX_NAME
    }

    fn exchange(&self) -> &'static str {
        "binance"
    }

    fn market(&self) -> Market {
        Market::Usdm
    }

    fn matches_headers(&self, headers: &[String]) -> bool {
        // No price column: that's what separates it from trade exports,
        // whose older spot layout also has "Type" and "Amount".
        [TIME, TYPE, AMOUNT, ASSET, SYMBOL]
            .iter()
            .all(|names| has_any(headers, names))
            && !has_any(headers, PRICE)
    }

    fn parse(&self, data: &[u8]) -> Result<ParseReport, ImportError> {
        let (header, rdr) = common::open(data)?;
        let cols = TxColumns::locate(&header)?;
        let mut deduper = Deduper::new(self.kind());
        let mut funding = Vec::new();
        let mut skipped = 0u32;
        let errors = common::each_row(rdr, |record| {
            if !is_funding(field(record, cols.kind)) {
                skipped += 1;
                return Ok(());
            }
            funding.push(parse_funding_row(&cols, record, &mut deduper)?);
            Ok(())
        });
        Ok(ParseReport {
            funding,
            errors,
            skipped,
            ..ParseReport::default()
        })
    }
}

fn parse_funding_row(
    cols: &TxColumns,
    r: &csv::StringRecord,
    deduper: &mut Deduper,
) -> Result<FundingEvent, String> {
    let time = common::utc_time(field(r, cols.time))?;
    let symbol = field(r, cols.symbol).to_uppercase();
    if symbol.is_empty() {
        return Err("funding fee without a symbol".into());
    }
    // Keep Binance's sign: negative = paid, positive = received.
    let amount = decimal("amount", field(r, cols.amount))?;
    let asset = field(r, cols.asset).to_uppercase();

    let dedupe_hash = deduper.hash(&[time.to_string(), symbol.clone(), key_dec(amount), asset]);

    Ok(FundingEvent {
        exchange: "binance".into(),
        symbol,
        amount,
        time,
        dedupe_hash,
        trade_id: None,
        import_batch_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Side;
    use rust_decimal_macros::dec;

    const TRADES: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test-data/binance-futures-trades.csv"
    ));
    const TX: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test-data/binance-futures-transactions.csv"
    ));

    fn ms(text: &str) -> i64 {
        common::utc_time(text).unwrap()
    }

    // ── trades ──

    #[test]
    fn parses_every_trade_fixture_row() {
        let report = BinanceUsdmTrades.parse(TRADES).unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(report.fills.len(), 8);

        let first = &report.fills[0];
        assert_eq!(first.time, ms("2026-02-02 10:00:01"));
        assert_eq!(first.symbol, "BTCUSDT");
        assert_eq!(first.side, Side::Buy);
        assert_eq!(first.price, dec!(61000));
        assert_eq!(first.qty, dec!(0.006));
        assert_eq!(
            (first.fee, first.fee_asset.as_str()),
            (dec!(0.1464), "USDT")
        );
        assert_eq!(first.realized_pnl, Some(dec!(0)));
        assert_eq!(first.market, Market::Usdm);

        assert_eq!(report.fills[2].realized_pnl, Some(dec!(8.505)));
        assert_eq!(report.fills[7].realized_pnl, Some(dec!(-22)));
        assert_eq!(report.fills[7].qty, dec!(10));
    }

    #[test]
    fn fixture_realized_pnl_and_fees_match_the_doc() {
        // docs/binance-csv.md: F1 realized +8.505, fees 0.491402; F2 +20 and +6; F3 −22.
        let fills = BinanceUsdmTrades.parse(TRADES).unwrap().fills;
        let sum = |symbol: &str, f: fn(&Fill) -> Decimal| -> Decimal {
            fills.iter().filter(|x| x.symbol == symbol).map(f).sum()
        };
        let realized = |x: &Fill| x.realized_pnl.unwrap_or_default();
        assert_eq!(sum("BTCUSDT", realized), dec!(8.505));
        assert_eq!(sum("BTCUSDT", |x| x.fee), dec!(0.491402));
        assert_eq!(sum("ETHUSDT", realized), dec!(26));
        assert_eq!(sum("SOLUSDT", realized), dec!(-22));
    }

    #[test]
    fn accepts_fee_with_glued_asset_and_missing_realized_column() {
        let data = b"Date(UTC),Symbol,Side,Price,Quantity,Amount,Fee,Realized Profit\n\
            2026-02-02 10:00:01,btcusdt,buy,61000.0,0.006,366.0,0.1464USDT,0\n";
        let fill = &BinanceUsdmTrades.parse(data).unwrap().fills[0];
        assert_eq!((fill.fee, fill.fee_asset.as_str()), (dec!(0.1464), "USDT"));
        assert_eq!(fill.symbol, "BTCUSDT");

        let no_realized = b"Date(UTC),Symbol,Side,Price,Quantity,Fee,Fee Coin\n\
            2026-02-02 10:00:01,BTCUSDT,BUY,61000,0.006,0.1464,USDT\n";
        let fill = &BinanceUsdmTrades.parse(no_realized).unwrap().fills[0];
        assert_eq!(fill.realized_pnl, None);
    }

    #[test]
    fn negative_fee_rebate_is_accepted() {
        let data = b"Date(UTC),Symbol,Side,Price,Quantity,Fee,Fee Coin,Realized Profit\n\
            2026-02-02 10:00:01,BTCUSDT,BUY,61000,0.006,-0.01,USDT,0\n";
        let report = BinanceUsdmTrades.parse(data).unwrap();
        assert!(report.errors.is_empty());
        assert_eq!(report.fills[0].fee, dec!(-0.01));
    }

    #[test]
    fn bad_trade_rows_are_reported_and_others_kept() {
        let data = b"Date(UTC),Symbol,Side,Price,Quantity,Fee,Fee Coin,Realized Profit\n\
            2026-02-02 10:00:01,BTCUSDT,BUY,61000,0.006,0.1,USDT,0\n\
            2026-02-02 10:00:02,BTCUSDT,BUY,61000,0,0.1,USDT,0\n\
            2026-02-02 10:00:03,,BUY,61000,0.006,0.1,USDT,0\n\
            2026-02-02 10:00:04,BTCUSDT,BUY,61000,0.006,0.1,USDT,n/a\n";
        let report = BinanceUsdmTrades.parse(data).unwrap();
        assert_eq!(report.fills.len(), 1);
        let lines: Vec<u64> = report.errors.iter().map(|e| e.line).collect();
        assert_eq!(lines, [3, 4, 5]);
        assert!(report.errors[2].message.contains("realized profit"));
    }

    #[test]
    fn trade_hashes_are_stable_across_reparse() {
        let a = BinanceUsdmTrades.parse(TRADES).unwrap();
        let b = BinanceUsdmTrades.parse(TRADES).unwrap();
        assert_eq!(a.fills, b.fills);
        let mut hashes: Vec<_> = a.fills.iter().map(|f| &f.dedupe_hash).collect();
        hashes.sort();
        hashes.dedup();
        assert_eq!(hashes.len(), 8, "every row has its own hash");
    }

    // ── transactions / funding ──

    #[test]
    fn imports_only_funding_rows_and_counts_skipped() {
        let report = BinanceUsdmTransactions.parse(TX).unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(report.fills.is_empty());
        assert_eq!(report.funding.len(), 6);
        assert_eq!(report.skipped, 3); // TRANSFER, COMMISSION, REALIZED_PNL

        let first = &report.funding[0];
        assert_eq!(first.time, ms("2026-02-02 16:00:00"));
        assert_eq!(first.symbol, "BTCUSDT");
        assert_eq!(first.amount, dec!(-0.0305)); // Binance sign kept: paid
    }

    #[test]
    fn fixture_funding_sums_match_the_doc() {
        let funding = BinanceUsdmTransactions.parse(TX).unwrap().funding;
        let sum = |symbol: &str| -> Decimal {
            funding
                .iter()
                .filter(|e| e.symbol == symbol)
                .map(|e| e.amount)
                .sum()
        };
        assert_eq!(sum("BTCUSDT"), dec!(-0.05185));
        assert_eq!(sum("ETHUSDT"), dec!(0.07235)); // +0.1025 − 0.03015
        assert_eq!(sum("SOLUSDT"), dec!(0.095));
    }

    #[test]
    fn funding_type_spelling_variants_are_accepted() {
        assert!(is_funding("FUNDING_FEE"));
        assert!(is_funding("Funding Fee"));
        assert!(is_funding(" funding_fee "));
        assert!(!is_funding("COMMISSION"));
        assert!(!is_funding("FUNDING"));
    }

    #[test]
    fn bad_funding_rows_are_reported() {
        let data = b"Date(UTC),Type,Amount,Asset,Symbol\n\
            2026-02-02 16:00:00,FUNDING_FEE,-0.03,USDT,BTCUSDT\n\
            2026-02-02 16:00:00,FUNDING_FEE,-0.03,USDT,\n\
            2026-02-02 16:00:00,FUNDING_FEE,abc,USDT,BTCUSDT\n\
            not a date,TRANSFER,5,USDT,\n";
        let report = BinanceUsdmTransactions.parse(data).unwrap();
        assert_eq!(report.funding.len(), 1);
        assert_eq!(report.skipped, 1, "non-funding rows are skipped unread");
        let lines: Vec<u64> = report.errors.iter().map(|e| e.line).collect();
        assert_eq!(lines, [3, 4]);
    }

    #[test]
    fn missing_columns_are_clear_errors() {
        let err = BinanceUsdmTrades
            .parse(b"Date(UTC),Symbol,Side,Price,Fee\n")
            .unwrap_err();
        assert!(
            matches!(
                err,
                ImportError::MissingColumn {
                    column: "quantity",
                    ..
                }
            ),
            "{err}"
        );

        let err = BinanceUsdmTransactions
            .parse(b"Date(UTC),Type,Amount,Asset\n")
            .unwrap_err();
        assert!(
            matches!(
                err,
                ImportError::MissingColumn {
                    column: "symbol",
                    ..
                }
            ),
            "{err}"
        );
    }
}
