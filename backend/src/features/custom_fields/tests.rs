//! Custom field tests (in-memory DB, fake data).

use super::*;
use crate::database::{open_in_memory, test_data};

fn opts(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn set(
    conn: &Connection,
    trade: i64,
    field: i64,
    value: &str,
) -> Result<Vec<FieldValue>, FieldError> {
    set_values(
        conn,
        trade,
        &[ValueInput {
            field_id: field,
            value: Some(value.into()),
        }],
    )
}

fn value_of(conn: &Connection, trade: i64, field: i64) -> Option<String> {
    values(conn, trade)
        .unwrap()
        .into_iter()
        .find(|v| v.field_id == field)
        .map(|v| v.value)
}

#[test]
fn create_each_type_and_list_in_order() {
    let conn = open_in_memory().unwrap();
    create(
        &conn,
        " Session ",
        FieldType::Select,
        &opts(&["Asia", " London ", "NY"]),
    )
    .unwrap();
    create(&conn, "Confidence", FieldType::Number, &opts(&["ignored"])).unwrap();
    create(&conn, "Followed plan?", FieldType::Checkbox, &[]).unwrap();
    create(&conn, "Lesson", FieldType::Text, &[]).unwrap();

    let fields = list(&conn).unwrap();
    let names: Vec<_> = fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["Session", "Confidence", "Followed plan?", "Lesson"]);
    assert_eq!(fields[0].options, ["Asia", "London", "NY"]);
    assert!(
        fields[1].options.is_empty(),
        "only choice fields keep options"
    );
    assert!(fields.iter().all(|f| f.used_by == 0));
}

#[test]
fn names_and_options_are_validated() {
    let conn = open_in_memory().unwrap();
    create(&conn, "Session", FieldType::Text, &[]).unwrap();
    assert!(matches!(
        create(&conn, "session", FieldType::Text, &[]),
        Err(FieldError::Duplicate(_))
    ));
    assert!(matches!(
        create(&conn, "  ", FieldType::Text, &[]),
        Err(FieldError::EmptyName)
    ));
    assert!(matches!(
        create(&conn, &"x".repeat(41), FieldType::Text, &[]),
        Err(FieldError::NameTooLong)
    ));
    assert!(matches!(
        create(&conn, "A", FieldType::Select, &[]),
        Err(FieldError::NoOptions)
    ));
    assert!(matches!(
        create(&conn, "B", FieldType::Select, &opts(&["x", "X"])),
        Err(FieldError::BadOption(_))
    ));
    assert!(matches!(
        create(&conn, "C", FieldType::Select, &opts(&["x", " "])),
        Err(FieldError::BadOption(_))
    ));
    let many: Vec<String> = (0..31).map(|i| format!("o{i}")).collect();
    assert!(matches!(
        create(&conn, "D", FieldType::Select, &many),
        Err(FieldError::TooManyOptions)
    ));
}

#[test]
fn values_are_checked_and_normalized_per_type() {
    let conn = open_in_memory().unwrap();
    let trade = db::trades::insert(&conn, &test_data::trade()).unwrap();
    let session = create(
        &conn,
        "Session",
        FieldType::Select,
        &opts(&["Asia", "London"]),
    )
    .unwrap();
    let conf = create(&conn, "Confidence", FieldType::Number, &[]).unwrap();
    let plan = create(&conn, "Followed plan?", FieldType::Checkbox, &[]).unwrap();
    let lesson = create(&conn, "Lesson", FieldType::Text, &[]).unwrap();

    set(&conn, trade, session, "london").unwrap();
    assert_eq!(
        value_of(&conn, trade, session).as_deref(),
        Some("London"),
        "stored as the option's spelling"
    );
    set(&conn, trade, conf, " 7.50 ").unwrap();
    assert_eq!(value_of(&conn, trade, conf).as_deref(), Some("7.5"));
    set(&conn, trade, plan, "true").unwrap();
    assert_eq!(value_of(&conn, trade, plan).as_deref(), Some("true"));
    set(&conn, trade, plan, "false").unwrap();
    assert_eq!(value_of(&conn, trade, plan), None, "unticked = no value");
    set(&conn, trade, lesson, "  wait for the close  ").unwrap();
    assert_eq!(
        value_of(&conn, trade, lesson).as_deref(),
        Some("wait for the close")
    );
    set(&conn, trade, lesson, "   ").unwrap();
    assert_eq!(value_of(&conn, trade, lesson), None, "empty clears");

    let err = set(&conn, trade, conf, "seven").unwrap_err().to_string();
    assert_eq!(err, "Confidence: enter a number like 7 or 2.5");
    assert!(matches!(
        set(&conn, trade, session, "NY"),
        Err(FieldError::BadValue { .. })
    ));
    assert!(matches!(
        set(&conn, trade, lesson, &"x".repeat(501)),
        Err(FieldError::BadValue { .. })
    ));
    assert!(matches!(
        set(&conn, trade, plan, "yes"),
        Err(FieldError::BadValue { .. })
    ));
}

#[test]
fn a_bad_value_rejects_the_whole_batch() {
    let conn = open_in_memory().unwrap();
    let trade = db::trades::insert(&conn, &test_data::trade()).unwrap();
    let conf = create(&conn, "Confidence", FieldType::Number, &[]).unwrap();
    let lesson = create(&conn, "Lesson", FieldType::Text, &[]).unwrap();
    let batch = [
        ValueInput {
            field_id: lesson,
            value: Some("kept?".into()),
        },
        ValueInput {
            field_id: conf,
            value: Some("abc".into()),
        },
    ];
    assert!(set_values(&conn, trade, &batch).is_err());
    assert!(values(&conn, trade).unwrap().is_empty(), "nothing saved");
    assert!(matches!(
        set(&conn, 999, lesson, "x"),
        Err(FieldError::NotFound("trade"))
    ));
    assert!(matches!(
        set(&conn, trade, 999, "x"),
        Err(FieldError::NotFound("field"))
    ));
}

#[test]
fn options_in_use_cannot_be_removed() {
    let conn = open_in_memory().unwrap();
    let trade = db::trades::insert(&conn, &test_data::trade()).unwrap();
    let session = create(
        &conn,
        "Session",
        FieldType::Select,
        &opts(&["Asia", "London", "NY"]),
    )
    .unwrap();
    set(&conn, trade, session, "Asia").unwrap();

    let err = update(&conn, session, "Session", &opts(&["London", "NY"])).unwrap_err();
    assert_eq!(
        err.to_string(),
        "option \"Asia\" is used by 1 trade(s); clear it there first"
    );
    // Removing an unused option and renaming the field is fine.
    update(
        &conn,
        session,
        "Trading session",
        &opts(&["Asia", "London"]),
    )
    .unwrap();
    let f = &list(&conn).unwrap()[0];
    assert_eq!((f.name.as_str(), f.used_by), ("Trading session", 1));
    assert_eq!(f.options, ["Asia", "London"]);
}

#[test]
fn deleting_a_field_or_trade_removes_its_values() {
    let conn = open_in_memory().unwrap();
    let trade = db::trades::insert(&conn, &test_data::trade()).unwrap();
    let a = create(&conn, "A", FieldType::Text, &[]).unwrap();
    let b = create(&conn, "B", FieldType::Text, &[]).unwrap();
    set(&conn, trade, a, "x").unwrap();
    set(&conn, trade, b, "y").unwrap();

    delete(&conn, a).unwrap();
    assert_eq!(values(&conn, trade).unwrap().len(), 1);
    db::trades::delete(&conn, trade).unwrap();
    assert_eq!(list(&conn).unwrap()[0].used_by, 0);
    assert!(matches!(
        delete(&conn, 999),
        Err(FieldError::NotFound("field"))
    ));
}
