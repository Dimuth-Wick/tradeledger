//! Tag management tests (in-memory DB, fake data).

use super::*;
use crate::database::{open_in_memory, test_data};

fn group_names(conn: &Connection) -> Vec<String> {
    list(conn).unwrap().into_iter().map(|g| g.name).collect()
}

#[test]
fn seeds_default_groups_once_and_never_again() {
    let conn = open_in_memory().unwrap();
    seed_defaults(&conn).unwrap();
    assert_eq!(group_names(&conn), ["Setup", "Mistake", "Emotion"]);

    // User deletes them all; a later start must not bring them back.
    for g in list(&conn).unwrap() {
        delete_group(&conn, g.id).unwrap();
    }
    seed_defaults(&conn).unwrap();
    assert!(group_names(&conn).is_empty());
}

#[test]
fn seeding_skips_when_groups_already_exist() {
    let conn = open_in_memory().unwrap();
    create_group(&conn, "My setups", "").unwrap();
    seed_defaults(&conn).unwrap();
    assert_eq!(group_names(&conn), ["My setups"]);
}

#[test]
fn create_list_rename_and_delete() {
    let conn = open_in_memory().unwrap();
    let g = create_group(&conn, "  Setup  ", "#396CD8").unwrap();
    let b = create_tag(&conn, g, "breakout", "").unwrap();
    create_tag(&conn, g, "Asia range", "#00AA11").unwrap();

    let groups = list(&conn).unwrap();
    assert_eq!(groups[0].name, "Setup", "names are trimmed");
    assert_eq!(groups[0].color, "#396cd8", "colors stored lowercase");
    let names: Vec<_> = groups[0].tags.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(
        names,
        ["Asia range", "breakout"],
        "tags sorted A→Z, any case"
    );

    update_tag(&conn, b, "Breakout", "#123456").unwrap();
    update_group(&conn, g, "Setups", "").unwrap();
    let groups = list(&conn).unwrap();
    assert_eq!(groups[0].name, "Setups");
    assert!(
        groups[0]
            .tags
            .iter()
            .any(|t| t.name == "Breakout" && t.color == "#123456")
    );

    delete_tag(&conn, b).unwrap();
    assert_eq!(list(&conn).unwrap()[0].tags.len(), 1);
}

#[test]
fn duplicate_names_get_friendly_errors() {
    let conn = open_in_memory().unwrap();
    let setup = create_group(&conn, "Setup", "").unwrap();
    let mistake = create_group(&conn, "Mistake", "").unwrap();
    let err = create_group(&conn, "Setup", "").unwrap_err();
    assert_eq!(err.to_string(), "Tag group \"Setup\" already exists");
    assert!(matches!(
        update_group(&conn, mistake, "Setup", ""),
        Err(TagError::Duplicate(_))
    ));

    create_tag(&conn, setup, "FOMO", "").unwrap();
    assert!(matches!(
        create_tag(&conn, setup, "FOMO", ""),
        Err(TagError::Duplicate(_))
    ));
    // The same tag name in another group is fine.
    create_tag(&conn, mistake, "FOMO", "").unwrap();
}

#[test]
fn names_are_unique_ignoring_case() {
    let conn = open_in_memory().unwrap();
    let setup = create_group(&conn, "Setup", "").unwrap();
    assert!(matches!(
        create_group(&conn, "SETUP", ""),
        Err(TagError::Duplicate(_))
    ));

    let b = create_tag(&conn, setup, "Breakout", "").unwrap();
    assert!(matches!(
        create_tag(&conn, setup, "breakout", ""),
        Err(TagError::Duplicate(_))
    ));
    let r = create_tag(&conn, setup, "Range", "").unwrap();
    assert!(matches!(
        update_tag(&conn, r, "BREAKOUT", ""),
        Err(TagError::Duplicate(_))
    ));

    // Renaming a tag to a different case of its own name is fine.
    update_tag(&conn, b, "BREAKOUT", "").unwrap();
    update_group(&conn, setup, "setup", "").unwrap();
}

#[test]
fn invalid_input_is_rejected() {
    let conn = open_in_memory().unwrap();
    assert!(matches!(
        create_group(&conn, "   ", ""),
        Err(TagError::EmptyName)
    ));
    assert!(matches!(
        create_group(&conn, &"x".repeat(41), ""),
        Err(TagError::NameTooLong)
    ));
    for bad in ["red", "#12345", "#1234567", "#zzzzzz", "123456"] {
        assert!(
            matches!(create_group(&conn, "G", bad), Err(TagError::BadColor)),
            "{bad}"
        );
    }
    assert!(matches!(
        create_tag(&conn, 999, "T", ""),
        Err(TagError::NotFound("tag group"))
    ));
    assert!(matches!(
        update_tag(&conn, 999, "T", ""),
        Err(TagError::NotFound("tag"))
    ));
    assert!(matches!(
        delete_group(&conn, 999),
        Err(TagError::NotFound("tag group"))
    ));
}

#[test]
fn set_trade_tags_replaces_the_whole_set() {
    let conn = open_in_memory().unwrap();
    let trade = db::trades::insert(&conn, &test_data::trade()).unwrap();
    let g = create_group(&conn, "Setup", "").unwrap();
    let a = create_tag(&conn, g, "A", "").unwrap();
    let b = create_tag(&conn, g, "B", "").unwrap();

    assert_eq!(set_trade_tags(&conn, trade, &[b, a, a]).unwrap(), [a, b]);
    assert_eq!(set_trade_tags(&conn, trade, &[b]).unwrap(), [b]);
    assert_eq!(
        set_trade_tags(&conn, trade, &[]).unwrap(),
        Vec::<i64>::new()
    );
}

#[test]
fn unknown_tag_or_trade_changes_nothing() {
    let conn = open_in_memory().unwrap();
    let trade = db::trades::insert(&conn, &test_data::trade()).unwrap();
    let g = create_group(&conn, "Setup", "").unwrap();
    let a = create_tag(&conn, g, "A", "").unwrap();
    set_trade_tags(&conn, trade, &[a]).unwrap();

    assert!(matches!(
        set_trade_tags(&conn, trade, &[a, 999]),
        Err(TagError::NotFound("tag"))
    ));
    assert_eq!(trade_tags(&conn, trade).unwrap(), [a], "rolled back");
    assert!(matches!(
        set_trade_tags(&conn, 999, &[a]),
        Err(TagError::NotFound("trade"))
    ));
}

#[test]
fn deleting_a_group_removes_its_tags_from_trades() {
    let conn = open_in_memory().unwrap();
    let trade = db::trades::insert(&conn, &test_data::trade()).unwrap();
    let setup = create_group(&conn, "Setup", "").unwrap();
    let mistake = create_group(&conn, "Mistake", "").unwrap();
    let a = create_tag(&conn, setup, "A", "").unwrap();
    let m = create_tag(&conn, mistake, "M", "").unwrap();
    set_trade_tags(&conn, trade, &[a, m]).unwrap();

    delete_group(&conn, setup).unwrap();
    assert_eq!(trade_tags(&conn, trade).unwrap(), [m]);
    assert_eq!(group_names(&conn), ["Mistake"]);
}

#[test]
fn trade_log_page_includes_tag_ids() {
    use crate::features::trade_query::{TradeQuery, query};
    let conn = open_in_memory().unwrap();
    let t1 = db::trades::insert(&conn, &test_data::trade()).unwrap();
    let t2 = db::trades::insert(&conn, &test_data::trade()).unwrap();
    let g = create_group(&conn, "Setup", "").unwrap();
    let a = create_tag(&conn, g, "A", "").unwrap();
    set_trade_tags(&conn, t1, &[a]).unwrap();

    let page = query(&conn, &TradeQuery::default()).unwrap();
    assert_eq!(page.tags.get(&t1), Some(&vec![a]));
    assert_eq!(page.tags.get(&t2), None);
}
