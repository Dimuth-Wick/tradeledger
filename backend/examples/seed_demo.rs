//! Fills a NEW database with fake demo trades, to look at the UI with data.
//!
//!     cargo run --example seed_demo -- <path\to\demo.db>
//!
//! Then start the app on it:  $env:TRADELEDGER_DB_PATH="<path>"; npm run tauri dev
//!
//! Everything here is invented (prices, times, results). It refuses to touch
//! an existing file, so it can never write into your real journal.

use std::path::PathBuf;

use rust_decimal::{Decimal, RoundingStrategy};
use tradeledger_lib::database;
use tradeledger_lib::features::trade_entry::{self, TradeInput};
use tradeledger_lib::features::{playbooks, tags};
use tradeledger_lib::models::{Direction, Market};

/// Small deterministic random generator (same demo every run, no extra crate).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Integer in lo..=hi.
    fn int(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % ((hi - lo + 1) as u64)) as i64
    }
    /// True with probability pct / 100.
    fn chance(&mut self, pct: i64) -> bool {
        self.int(1, 100) <= pct
    }
    /// Decimal in lo..=hi given in thousandths (e.g. 800..3200 = 0.8..3.2).
    fn dec(&mut self, lo_milli: i64, hi_milli: i64) -> Decimal {
        Decimal::new(self.int(lo_milli, hi_milli), 3)
    }
}

/// Fake coins: symbol, rough price, decimals for price and quantity.
const COINS: [(&str, i64, u32, u32); 6] = [
    ("BTCUSDT", 64_000, 1, 3),
    ("ETHUSDT", 3_100, 2, 3),
    ("SOLUSDT", 150, 2, 1),
    ("BNBUSDT", 580, 2, 2),
    ("XRPUSDT", 1, 4, 0), // ~0.6 after the factor below
    ("DOGEUSDT", 1, 5, 0),
];

fn round(d: Decimal, dp: u32) -> Decimal {
    d.round_dp_with_strategy(dp, RoundingStrategy::MidpointAwayFromZero)
        .normalize()
}

fn find_or_create_tag(conn: &rusqlite::Connection, group: &str, name: &str) -> i64 {
    let groups = tags::list(conn).expect("list tags");
    let g = groups
        .iter()
        .find(|g| g.name == group)
        .expect("default tag group");
    if let Some(t) = g.tags.iter().find(|t| t.name == name) {
        return t.id;
    }
    tags::create_tag(conn, g.id, name, "").expect("create tag")
}

fn main() {
    let path: PathBuf = std::env::args()
        .nth(1)
        .expect("usage: cargo run --example seed_demo -- <new demo.db path>")
        .into();
    if path.exists() {
        eprintln!(
            "{} already exists; delete it first (this tool only creates new demo files)",
            path.display()
        );
        std::process::exit(1);
    }
    let conn = database::open(&path).expect("open demo db");
    tags::seed_defaults(&conn).expect("default tag groups");

    // Tags and playbooks the fake trades use.
    let setups = [
        find_or_create_tag(&conn, "Setup", "Breakout"),
        find_or_create_tag(&conn, "Setup", "Pullback"),
        find_or_create_tag(&conn, "Setup", "Range"),
    ];
    let mistakes = [
        find_or_create_tag(&conn, "Mistake", "FOMO"),
        find_or_create_tag(&conn, "Mistake", "Moved stop"),
        find_or_create_tag(&conn, "Mistake", "Early exit"),
    ];
    let calm = find_or_create_tag(&conn, "Emotion", "Calm");
    let tilted = find_or_create_tag(&conn, "Emotion", "Tilted");

    let breakout = playbooks::create(
        &conn,
        "Breakout retest",
        "Enter on the retest of a broken level, not on the first push.",
        &[
            "Higher timeframe trend agrees".into(),
            "Level retested and held".into(),
            "Stop beyond the level".into(),
            "Risk 1% or less".into(),
        ],
    )
    .expect("playbook");
    let range = playbooks::create(
        &conn,
        "Range fade",
        "Fade the edges of a clear range back to the middle.",
        &[
            "Range clear on the 1h chart".into(),
            "Rejection wick at the edge".into(),
            "Target the middle of the range".into(),
        ],
    )
    .expect("playbook");
    let rules = |id: i64| -> Vec<i64> {
        playbooks::list(&conn)
            .unwrap()
            .into_iter()
            .find(|p| p.id == id)
            .unwrap()
            .rules
            .iter()
            .map(|r| r.id)
            .collect()
    };
    let (breakout_rules, range_rules) = (rules(breakout), rules(range));

    let mut rng = Rng(0x7EAD_1ED6_E2C0_FFEE);
    // 2026-08-17 00:00 UTC, about three months before "today" in the app.
    let start_ms: i64 = 1_786_924_800_000;
    let day_ms: i64 = 86_400_000;
    let mut made = 0;

    for day in 0..91 {
        // 0 to 2 trades a day, fewer at weekends.
        let weekend = day % 7 >= 5;
        let count = if weekend {
            rng.int(0, 1)
        } else {
            rng.int(0, 2)
        };
        for _ in 0..count {
            let (symbol, base, price_dp, qty_dp) = COINS[rng.int(0, 5) as usize];
            let market = if rng.chance(80) {
                Market::Usdm
            } else {
                Market::Spot
            };
            let direction = if market == Market::Spot || rng.chance(58) {
                Direction::Long
            } else {
                Direction::Short
            };

            // Price drifts over the three months (±8%).
            let mut entry = Decimal::from(base) * rng.dec(920, 1080);
            if symbol == "XRPUSDT" {
                entry *= Decimal::new(6, 1);
            }
            if symbol == "DOGEUSDT" {
                entry *= Decimal::new(14, 2);
            }
            let entry = round(entry, price_dp);

            // Risk 0.8%–2.5% of price to the stop; about 20 USDT risked.
            let risk = entry * rng.dec(8, 25); // dec(8, 25) = 0.008..0.025
            let sign = if direction == Direction::Long {
                Decimal::ONE
            } else {
                -Decimal::ONE
            };
            let stop = round(entry - sign * risk, price_dp);
            let qty = round(Decimal::from(20) / risk, qty_dp).max(Decimal::new(1, qty_dp));

            // Outcome in R: 50% wins, 42% losses, 8% about breakeven.
            let roll = rng.int(1, 100);
            let r = if roll <= 50 {
                rng.dec(600, 3400)
            } else if roll <= 92 {
                -rng.dec(500, 1150)
            } else {
                rng.dec(-50, 50)
            };
            let exit = round(entry + sign * risk * r, price_dp);
            let target = round(entry + sign * risk * Decimal::from(2), price_dp);

            let open_time =
                start_ms + day * day_ms + rng.int(1, 22) * 3_600_000 + rng.int(0, 59) * 60_000;
            let hold = rng.int(15, 60 * 40) * 60_000;
            // The last few days keep some trades open.
            let still_open = day > 86 && rng.chance(50);
            let notional = entry * qty;
            let fees = round(notional * Decimal::new(8, 4), 4); // 0.04% in + out
            let funding = if market == Market::Usdm {
                // −0.02%..+0.06% of the position (paid or received).
                round(notional * rng.dec(-20, 60) / Decimal::ONE_HUNDRED, 4)
            } else {
                Decimal::ZERO
            };

            let pb = if rng.chance(70) {
                Some(if rng.chance(55) { breakout } else { range })
            } else {
                None
            };
            let input = TradeInput {
                position_size: None,
                auto_fees: false,
                exchange: "binance".into(),
                market,
                symbol: symbol.into(),
                direction,
                open_time,
                close_time: (!still_open).then_some(open_time + hold),
                qty,
                avg_entry: entry,
                avg_exit: (!still_open).then_some(exit),
                leverage: (market == Market::Usdm).then(|| rng.int(3, 20) as u32),
                fees: if still_open {
                    fees / Decimal::TWO
                } else {
                    fees
                },
                funding,
                planned_entry: None,
                stop_loss: if rng.chance(85) { Some(stop) } else { None },
                take_profit: rng.chance(60).then_some(target),
                notes: String::new(),
                playbook_id: None,
            };
            let trade = trade_entry::create(&conn, &input).expect("create fake trade");
            let won = r > Decimal::ZERO;

            // Tags: one setup; a mistake more often on losses; a mood.
            let mut ids = vec![setups[rng.int(0, 2) as usize]];
            if rng.chance(if won { 10 } else { 40 }) {
                ids.push(mistakes[rng.int(0, 2) as usize]);
            }
            if rng.chance(50) {
                ids.push(if won || rng.chance(50) { calm } else { tilted });
            }
            tags::set_trade_tags(&conn, trade.id, &ids).expect("tags");

            // Playbook checklist: winners follow the rules more often.
            if let Some(p) = pb {
                let all = if p == breakout {
                    &breakout_rules
                } else {
                    &range_rules
                };
                let follow = if won { 90 } else { 60 };
                let ticked: Vec<i64> = all.iter().copied().filter(|_| rng.chance(follow)).collect();
                playbooks::set_for_trade(&conn, trade.id, Some(p), &ticked).expect("checklist");
            }
            made += 1;
        }
    }
    println!("Created {made} fake trades in {}", path.display());
}
