//! Fake sample rows shared by the repository tests. Not real trading data.

use rust_decimal_macros::dec;

use crate::models::{Direction, Fill, FundingEvent, ImportBatch, Market, Side, Trade, TradeStatus};

pub fn trade() -> Trade {
    Trade {
        exchange: "binance".into(),
        market: Market::Usdm,
        symbol: "BTCUSDT".into(),
        direction: Direction::Long,
        status: TradeStatus::Closed,
        open_time: 1_700_000_000_000,
        close_time: Some(1_700_003_600_000),
        qty: dec!(0.015),
        avg_entry: dec!(36500.10),
        avg_exit: Some(dec!(37010.55)),
        leverage: Some(10),
        gross_pnl: dec!(7.65675),
        fees: dec!(0.55125),
        funding: dec!(0.0123),
        net_pnl: dec!(7.0932),
        planned_entry: Some(dec!(36500)),
        stop_loss: Some(dec!(36200)),
        take_profit: Some(dec!(37400)),
        planned_rr: Some(dec!(3)),
        r_multiple: Some(dec!(1.70)),
        notes: "Breakout retest".into(),
        playbook_id: None,
        is_manual: false,
    }
}

pub fn fill(hash: &str, time: i64) -> Fill {
    Fill {
        exchange: "binance".into(),
        market: Market::Spot,
        symbol: "ETHUSDT".into(),
        side: Side::Buy,
        qty: dec!(0.5),
        price: dec!(2000.25),
        fee: dec!(0.0005),
        fee_asset: "ETH".into(),
        realized_pnl: None,
        time,
        dedupe_hash: hash.into(),
        trade_id: None,
        import_batch_id: None,
    }
}

pub fn funding(hash: &str) -> FundingEvent {
    FundingEvent {
        exchange: "binance".into(),
        symbol: "BTCUSDT".into(),
        amount: dec!(-0.0123),
        time: 1_700_001_000_000,
        dedupe_hash: hash.into(),
        trade_id: None,
        import_batch_id: None,
    }
}

pub fn import_batch() -> ImportBatch {
    ImportBatch {
        exchange: "binance".into(),
        market: Market::Spot,
        file_name: "fake-spot-export.csv".into(),
        imported_at: 1_700_000_000_000,
        rows_total: 10,
        rows_inserted: 8,
        rows_duplicate: 2,
    }
}
