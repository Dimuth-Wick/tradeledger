//! The symbol menu on the new-trade form: popular Binance pairs for the
//! chosen market, plus any symbol already in the journal for that market.

use rusqlite::Connection;
use serde::Serialize;

use crate::database::{self as db, DbError};
use crate::models::Market;

/// Popular Binance spot pairs, most traded first.
pub const SPOT: &[&str] = &[
    "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT", "DOGEUSDT", "ADAUSDT", "TRXUSDT",
    "TONUSDT", "AVAXUSDT", "LINKUSDT", "DOTUSDT", "SUIUSDT", "LTCUSDT", "BCHUSDT", "NEARUSDT",
    "APTUSDT", "UNIUSDT", "PEPEUSDT", "SHIBUSDT", "ETHBTC", "BNBBTC", "SOLBTC", "BTCUSDC",
    "ETHUSDC", "BTCFDUSD", "ETHFDUSD",
];

/// Popular Binance USDⓈ-M perpetual futures (all settle in USDT).
pub const USDM: &[&str] = &[
    "BTCUSDT",
    "ETHUSDT",
    "BNBUSDT",
    "SOLUSDT",
    "XRPUSDT",
    "DOGEUSDT",
    "ADAUSDT",
    "TRXUSDT",
    "TONUSDT",
    "AVAXUSDT",
    "LINKUSDT",
    "DOTUSDT",
    "SUIUSDT",
    "LTCUSDT",
    "BCHUSDT",
    "NEARUSDT",
    "APTUSDT",
    "UNIUSDT",
    "1000PEPEUSDT",
    "1000SHIBUSDT",
];

/// Quote coins, longest first so "FDUSD" wins over "USD…" lookalikes.
const QUOTES: &[&str] = &["FDUSD", "USDT", "USDC", "BTC", "ETH", "BNB"];

/// One menu entry: `BTCUSDT` shown as BTC / USDT.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolChoice {
    pub symbol: String,
    pub base: String,
    pub quote: String,
}

/// Splits `BTCUSDT` into ("BTC", "USDT"). Unknown quotes keep the whole
/// symbol as the base and an empty quote.
pub fn split_pair(symbol: &str) -> (String, String) {
    for quote in QUOTES {
        if let Some(base) = symbol.strip_suffix(quote)
            && !base.is_empty()
        {
            return (base.to_string(), (*quote).to_string());
        }
    }
    (symbol.to_string(), String::new())
}

/// Popular pairs first (in their order), then symbols you already traded
/// on this market that aren't in the list (A→Z). No duplicates.
pub fn choices(market: Market, used: &[String]) -> Vec<SymbolChoice> {
    let popular = match market {
        Market::Spot => SPOT,
        Market::Usdm => USDM,
    };
    let mut extra: Vec<&str> = used
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !popular.contains(s))
        .collect();
    extra.sort_unstable();
    extra.dedup();

    popular
        .iter()
        .copied()
        .chain(extra)
        .map(|symbol| {
            let (base, quote) = split_pair(symbol);
            SymbolChoice {
                symbol: symbol.to_string(),
                base,
                quote,
            }
        })
        .collect()
}

/// The menu for `market`, including symbols already in the journal.
pub fn choices_from_db(conn: &Connection, market: Market) -> Result<Vec<SymbolChoice>, DbError> {
    Ok(choices(
        market,
        &db::trades::symbols_for_market(conn, market)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[SymbolChoice]) -> Vec<&str> {
        list.iter().map(|c| c.symbol.as_str()).collect()
    }

    #[test]
    fn splits_pairs_by_quote_coin() {
        assert_eq!(split_pair("BTCUSDT"), ("BTC".into(), "USDT".into()));
        assert_eq!(split_pair("ETHBTC"), ("ETH".into(), "BTC".into()));
        assert_eq!(split_pair("BTCFDUSD"), ("BTC".into(), "FDUSD".into()));
        assert_eq!(
            split_pair("1000PEPEUSDT"),
            ("1000PEPE".into(), "USDT".into())
        );
        // Unknown quote, or nothing left for the base.
        assert_eq!(split_pair("XYZ"), ("XYZ".into(), String::new()));
        assert_eq!(split_pair("USDT"), ("USDT".into(), String::new()));
    }

    #[test]
    fn spot_and_futures_have_their_own_lists() {
        let spot = choices(Market::Spot, &[]);
        let usdm = choices(Market::Usdm, &[]);
        assert_eq!(spot[0].symbol, "BTCUSDT");
        assert!(names(&spot).contains(&"ETHBTC"));
        assert!(!names(&usdm).contains(&"ETHBTC"));
        assert!(names(&usdm).contains(&"1000PEPEUSDT"));
        // Futures here are USDT-margined only.
        assert!(usdm.iter().all(|c| c.quote == "USDT"));
    }

    #[test]
    fn traded_symbols_are_added_once_after_the_popular_ones() {
        let used = vec![
            "WIFUSDT".to_string(),
            "BTCUSDT".to_string(), // already popular: not repeated
            "ARBUSDT".to_string(),
            "WIFUSDT".to_string(),
            " ".to_string(),
        ];
        let list = choices(Market::Usdm, &used);
        let n = USDM.len();
        assert_eq!(list.len(), n + 2);
        assert_eq!(names(&list[n..]), vec!["ARBUSDT", "WIFUSDT"]);
        assert_eq!(names(&list).iter().filter(|s| **s == "BTCUSDT").count(), 1);
    }

    #[test]
    fn popular_lists_have_no_duplicates() {
        for list in [SPOT, USDM] {
            let mut sorted = list.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), list.len());
        }
    }
}
