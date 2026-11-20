//! Where the money comes from: closed-trade results split by market
//! (futures / spot), side (long / short) and coin. Shown as bar charts on
//! the dashboard; uses the same trades as the rest of the stats.

use std::collections::BTreeMap;

use rust_decimal::{Decimal, RoundingStrategy};
use serde::Serialize;

use crate::models::{Direction, Market, Trade, TradeStatus};

/// One bar: the closed trades of one group.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Slice {
    /// Stable id: "usdm", "spot", "long", "short" or the symbol.
    pub key: String,
    pub label: String,
    /// Sum of net PnL (after fees and funding).
    pub net_pnl: Decimal,
    pub closed: u32,
    pub wins: u32,
    pub losses: u32,
    /// Percent 0–100, 2 decimals; `None` without closed trades.
    pub win_rate: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Breakdown {
    /// Futures then spot, always both (a market with no trades shows 0).
    pub by_market: Vec<Slice>,
    /// Long then short, always both.
    pub by_side: Vec<Slice>,
    /// Coins with closed trades, best result first.
    pub by_symbol: Vec<Slice>,
}

#[derive(Default, Clone, Copy)]
struct Tally {
    net: Decimal,
    closed: u32,
    wins: u32,
    losses: u32,
}

impl Tally {
    fn add(&mut self, t: &Trade) {
        self.net += t.net_pnl;
        self.closed += 1;
        if t.net_pnl > Decimal::ZERO {
            self.wins += 1;
        } else if t.net_pnl < Decimal::ZERO {
            self.losses += 1;
        }
    }

    fn slice(self, key: &str, label: &str) -> Slice {
        Slice {
            key: key.to_string(),
            label: label.to_string(),
            net_pnl: self
                .net
                .round_dp_with_strategy(8, RoundingStrategy::MidpointAwayFromZero)
                .normalize(),
            closed: self.closed,
            wins: self.wins,
            losses: self.losses,
            win_rate: (self.closed > 0).then(|| {
                (Decimal::from(self.wins) * Decimal::ONE_HUNDRED / Decimal::from(self.closed))
                    .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
                    .normalize()
            }),
        }
    }
}

/// "BTCUSDT" → "BTC/USDT" when the quote is a known one.
fn pair_label(symbol: &str) -> String {
    const QUOTES: [&str; 6] = ["FDUSD", "USDT", "USDC", "BTC", "ETH", "BNB"];
    QUOTES
        .iter()
        .find_map(|q| {
            symbol
                .strip_suffix(q)
                .filter(|base| !base.is_empty())
                .map(|base| format!("{base}/{q}"))
        })
        .unwrap_or_else(|| symbol.to_string())
}

pub fn compute(trades: &[(i64, &Trade)]) -> Breakdown {
    let (mut futures, mut spot, mut long, mut short) = Default::default();
    let mut coins: BTreeMap<&str, Tally> = BTreeMap::new();
    for (_, t) in trades
        .iter()
        .filter(|(_, t)| t.status == TradeStatus::Closed)
    {
        match t.market {
            Market::Usdm => Tally::add(&mut futures, t),
            Market::Spot => Tally::add(&mut spot, t),
        }
        match t.direction {
            Direction::Long => Tally::add(&mut long, t),
            Direction::Short => Tally::add(&mut short, t),
        }
        coins.entry(t.symbol.as_str()).or_default().add(t);
    }

    let mut by_symbol: Vec<Slice> = coins
        .into_iter()
        .map(|(sym, tally)| tally.slice(sym, &pair_label(sym)))
        .collect();
    // Best first; equal results keep A→Z order (the sort is stable).
    by_symbol.sort_by_key(|s| std::cmp::Reverse(s.net_pnl));

    Breakdown {
        by_market: vec![
            Tally::slice(futures, "usdm", "Futures"),
            Tally::slice(spot, "spot", "Spot"),
        ],
        by_side: vec![
            Tally::slice(long, "long", "Long"),
            Tally::slice(short, "short", "Short"),
        ],
        by_symbol,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::trade_entry::{TradeInput, build_trade};
    use rust_decimal_macros::dec;

    fn trade(symbol: &str, market: Market, dir: Direction, exit: Option<Decimal>) -> Trade {
        build_trade(
            &TradeInput {
                exchange: "binance".into(),
                market,
                symbol: symbol.into(),
                direction: dir,
                open_time: 1_000,
                close_time: exit.map(|_| 2_000),
                qty: dec!(1),
                position_size: None,
                avg_entry: dec!(100),
                avg_exit: exit,
                leverage: None,
                fees: dec!(0),
                auto_fees: false,
                funding: dec!(0),
                planned_entry: None,
                stop_loss: None,
                take_profit: None,
                notes: String::new(),
                playbook_id: None,
            },
            true,
        )
        .unwrap()
    }

    #[test]
    fn splits_by_market_side_and_coin() {
        let ts = [
            trade("BTCUSDT", Market::Usdm, Direction::Long, Some(dec!(110))), // +10
            trade("BTCUSDT", Market::Usdm, Direction::Short, Some(dec!(104))), // −4
            trade("HYPEUSDT", Market::Spot, Direction::Long, Some(dec!(125))), // +25
            trade("ETHUSDT", Market::Spot, Direction::Long, Some(dec!(100))), // 0
            trade("SOLUSDT", Market::Usdm, Direction::Long, None),            // open
        ];
        let refs: Vec<(i64, &Trade)> = ts.iter().enumerate().map(|(i, t)| (i as i64, t)).collect();
        let b = compute(&refs);

        let m: Vec<(&str, Decimal, u32)> = b
            .by_market
            .iter()
            .map(|s| (s.key.as_str(), s.net_pnl, s.closed))
            .collect();
        assert_eq!(m, [("usdm", dec!(6), 2), ("spot", dec!(25), 2)]);

        assert_eq!(b.by_side[0].net_pnl, dec!(35)); // 10 + 25 + 0
        assert_eq!((b.by_side[0].wins, b.by_side[0].losses), (2, 0));
        assert_eq!(b.by_side[0].win_rate, Some(dec!(66.67)));
        assert_eq!(b.by_side[1].net_pnl, dec!(-4));
        assert_eq!(b.by_side[1].win_rate, Some(dec!(0)));

        let coins: Vec<(&str, &str, Decimal)> = b
            .by_symbol
            .iter()
            .map(|s| (s.key.as_str(), s.label.as_str(), s.net_pnl))
            .collect();
        assert_eq!(
            coins,
            [
                ("HYPEUSDT", "HYPE/USDT", dec!(25)),
                ("BTCUSDT", "BTC/USDT", dec!(6)),
                ("ETHUSDT", "ETH/USDT", dec!(0)),
            ]
        );
    }

    #[test]
    fn empty_still_has_both_markets_and_sides() {
        let b = compute(&[]);
        assert_eq!(b.by_market.len(), 2);
        assert_eq!(b.by_side.len(), 2);
        assert!(b.by_symbol.is_empty());
        assert_eq!(b.by_market[0].win_rate, None);
    }

    #[test]
    fn pair_labels() {
        assert_eq!(pair_label("BTCUSDT"), "BTC/USDT");
        assert_eq!(pair_label("ETHBTC"), "ETH/BTC");
        assert_eq!(pair_label("USDT"), "USDT");
        assert_eq!(pair_label("ODDCOIN"), "ODDCOIN");
    }
}
