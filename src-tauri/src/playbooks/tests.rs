//! Playbook tests with fake data.

use rust_decimal_macros::dec;

use super::*;
use crate::db::{open_in_memory, test_data};

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn new_trade(conn: &Connection, net: Decimal, status: TradeStatus) -> i64 {
    let t = Trade {
        net_pnl: net,
        status,
        close_time: (status == TradeStatus::Closed).then_some(1_700_003_600_000),
        ..test_data::trade()
    };
    db::trades::insert(conn, &t).unwrap()
}

fn rule_ids(conn: &Connection, id: i64) -> Vec<i64> {
    repo::rules(conn, id)
        .unwrap()
        .iter()
        .map(|r| r.id)
        .collect()
}

#[test]
fn create_validates_and_lists_rules_in_order() {
    let conn = open_in_memory().unwrap();
    let id = create(
        &conn,
        "  Breakout retest ",
        "Wait for the retest",
        &strings(&["HTF trend agrees", " Retest holds ", "Stop under the level"]),
    )
    .unwrap();
    let list = list(&conn).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "Breakout retest");
    let texts: Vec<&str> = list[0].rules.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        texts,
        ["HTF trend agrees", "Retest holds", "Stop under the level"]
    );
    assert_eq!(list[0].id, id);

    let err = |r: Result<i64, PlaybookError>| r.unwrap_err().to_string();
    assert_eq!(err(create(&conn, " ", "", &[])), "name can't be empty");
    assert_eq!(
        err(create(&conn, "BREAKOUT RETEST", "", &[])),
        "a playbook named \"BREAKOUT RETEST\" already exists"
    );
    assert_eq!(
        err(create(&conn, "Range", "", &strings(&["a", " "]))),
        "rule 2 is empty"
    );
    assert_eq!(
        err(create(&conn, "Range", "", &strings(&["Fade", "fade"]))),
        "rule \"fade\" is listed twice"
    );
    let many: Vec<String> = (0..31).map(|i| format!("rule {i}")).collect();
    assert_eq!(
        err(create(&conn, "Range", "", &many)),
        "too many rules (max 30)"
    );
    assert_eq!(list_len(&conn), 1); // nothing half-saved
}

fn list_len(conn: &Connection) -> usize {
    list(conn).unwrap().len()
}

#[test]
fn update_keeps_rule_ids_so_ticks_survive() {
    let conn = open_in_memory().unwrap();
    let id = create(&conn, "Breakout", "", &strings(&["A", "B", "C"])).unwrap();
    let [a, b, c] = rule_ids(&conn, id)[..] else {
        panic!()
    };
    let trade = new_trade(&conn, dec!(5), TradeStatus::Closed);
    set_for_trade(&conn, trade, Some(id), &[a, b, c]).unwrap();

    // Reorder to B, D (new), A; drop C; rename B.
    let rules = [
        RuleInput {
            id: Some(b),
            text: "B edited".into(),
        },
        RuleInput {
            id: None,
            text: "D".into(),
        },
        RuleInput {
            id: Some(a),
            text: "A".into(),
        },
    ];
    update(&conn, id, "Breakout v2", "new text", &rules).unwrap();

    let saved = &list(&conn).unwrap()[0];
    let texts: Vec<&str> = saved.rules.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(texts, ["B edited", "D", "A"]);
    assert_eq!((saved.rules[0].id, saved.rules[2].id), (b, a));
    assert_eq!(saved.used_by, 1);
    // C's tick went with C; A and B kept theirs.
    assert_eq!(for_trade(&conn, trade).unwrap().checked, [a, b]);

    // A rule id from elsewhere is refused and nothing changes.
    let other = create(&conn, "Range", "", &strings(&["X"])).unwrap();
    let x = rule_ids(&conn, other)[0];
    let bad = [RuleInput {
        id: Some(x),
        text: "X".into(),
    }];
    assert!(matches!(
        update(&conn, id, "Breakout v2", "", &bad),
        Err(PlaybookError::ForeignRule(_))
    ));
    assert_eq!(rule_ids(&conn, id).len(), 3);
}

#[test]
fn trade_checklist_rules() {
    let conn = open_in_memory().unwrap();
    let p1 = create(&conn, "Breakout", "", &strings(&["A", "B"])).unwrap();
    let p2 = create(&conn, "Range", "", &strings(&["X"])).unwrap();
    let [a, _b] = rule_ids(&conn, p1)[..] else {
        panic!()
    };
    let x = rule_ids(&conn, p2)[0];
    let trade = new_trade(&conn, dec!(1), TradeStatus::Closed);

    assert_eq!(
        for_trade(&conn, trade).unwrap(),
        TradePlaybook {
            playbook_id: None,
            checked: vec![]
        }
    );
    let saved = set_for_trade(&conn, trade, Some(p1), &[a, a]).unwrap();
    assert_eq!(saved.checked, [a]); // duplicates collapse

    // A rule of another playbook, ticks without a playbook, unknown ids.
    assert!(set_for_trade(&conn, trade, Some(p1), &[x]).is_err());
    assert!(set_for_trade(&conn, trade, None, &[a]).is_err());
    assert!(set_for_trade(&conn, trade, Some(999), &[]).is_err());
    assert!(set_for_trade(&conn, 999, Some(p1), &[]).is_err());
    assert_eq!(for_trade(&conn, trade).unwrap().checked, [a]); // unchanged

    // Switching playbook replaces the ticks.
    set_for_trade(&conn, trade, Some(p2), &[x]).unwrap();
    assert_eq!(
        for_trade(&conn, trade).unwrap(),
        TradePlaybook {
            playbook_id: Some(p2),
            checked: vec![x]
        }
    );
}

#[test]
fn deleting_a_playbook_unlinks_its_trades() {
    let conn = open_in_memory().unwrap();
    let p = create(&conn, "Breakout", "", &strings(&["A"])).unwrap();
    let a = rule_ids(&conn, p)[0];
    let trade = new_trade(&conn, dec!(1), TradeStatus::Closed);
    set_for_trade(&conn, trade, Some(p), &[a]).unwrap();
    delete(&conn, p).unwrap();
    assert_eq!(
        for_trade(&conn, trade).unwrap(),
        TradePlaybook {
            playbook_id: None,
            checked: vec![]
        }
    );
    assert!(repo::all_checked(&conn).unwrap().is_empty());
    assert!(db::trades::get(&conn, trade).unwrap().is_some()); // trade kept
    assert!(delete(&conn, p).is_err());
}

#[test]
fn stats_compare_followed_and_skipped() {
    let conn = open_in_memory().unwrap();
    let p = create(&conn, "Breakout", "", &strings(&["A", "B"])).unwrap();
    let empty = create(&conn, "No rules yet", "", &[]).unwrap();
    let [a, b] = rule_ids(&conn, p)[..] else {
        panic!()
    };
    let t1 = new_trade(&conn, dec!(10), TradeStatus::Closed); // both ticked
    let t2 = new_trade(&conn, dec!(-4), TradeStatus::Closed); // one ticked
    let t3 = new_trade(&conn, dec!(2), TradeStatus::Closed); // none ticked
    let t4 = new_trade(&conn, dec!(99), TradeStatus::Open); // open: left out
    let t5 = new_trade(&conn, dec!(3), TradeStatus::Closed); // other playbook
    set_for_trade(&conn, t1, Some(p), &[a, b]).unwrap();
    set_for_trade(&conn, t2, Some(p), &[b]).unwrap();
    set_for_trade(&conn, t3, Some(p), &[]).unwrap();
    set_for_trade(&conn, t4, Some(p), &[a, b]).unwrap();
    set_for_trade(&conn, t5, Some(empty), &[]).unwrap();

    let all = stats(&conn).unwrap();
    let s = all.iter().find(|s| s.playbook_id == p).unwrap();
    assert_eq!((s.all.trades, s.all.net_pnl), (3, dec!(8))); // 10 − 4 + 2
    assert_eq!((s.followed.trades, s.followed.net_pnl), (1, dec!(10)));
    assert_eq!((s.broke.trades, s.broke.net_pnl), (2, dec!(-2)));
    assert_eq!(s.adherence, Some(dec!(50))); // (2/2 + 1/2 + 0/2) / 3
    assert_eq!(s.all.win_rate, Some(dec!(66.67)));

    // No rules: nothing counts as "followed" and adherence is unknown.
    let e = all.iter().find(|s| s.playbook_id == empty).unwrap();
    assert_eq!((e.all.trades, e.followed.trades, e.broke.trades), (1, 0, 1));
    assert_eq!(e.adherence, None);
}
