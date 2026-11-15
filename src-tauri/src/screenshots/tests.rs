//! Screenshot tests: in-memory DB + a fresh temp folder per test (fake images).

use super::*;
use crate::db::{open_in_memory, test_data};

/// Smallest valid-looking PNG header + some bytes (enough for detection).
const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4];
const JPG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0, 0x10];

fn temp_dir(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tradeledger-shots-{name}-{}-{nanos}",
        std::process::id()
    ))
}

fn setup(name: &str) -> (Connection, PathBuf, i64) {
    let conn = open_in_memory().unwrap();
    let trade = db::trades::insert(&conn, &test_data::trade()).unwrap();
    (conn, temp_dir(name), trade)
}

fn files_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map(|it| {
            it.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

#[test]
fn detects_images_by_content_not_name() {
    assert_eq!(detect_image(PNG), Some("png"));
    assert_eq!(detect_image(JPG), Some("jpg"));
    assert_eq!(detect_image(b"GIF89a....."), Some("gif"));
    assert_eq!(detect_image(b"RIFF\0\0\0\0WEBPVP8 "), Some("webp"));
    assert_eq!(detect_image(b"MZ\x90\0 not an image"), None);
    assert_eq!(detect_image(b""), None);
}

#[test]
fn add_writes_file_and_row_then_read_and_delete() {
    let (conn, dir, trade) = setup("roundtrip");
    let shot = add(
        &conn,
        &dir,
        trade,
        PNG,
        "  entry on retest  ",
        1_700_000_000_000,
    )
    .unwrap();
    assert!(
        shot.file_name
            .starts_with(&format!("t{trade}-1700000000000-"))
    );
    assert!(shot.file_name.ends_with(".png"));
    assert_eq!(shot.caption, "entry on retest");
    assert_eq!(files_in(&dir), std::slice::from_ref(&shot.file_name));

    let (bytes, mime) = read(&conn, &dir, shot.id).unwrap();
    assert_eq!((bytes.as_slice(), mime), (PNG, "image/png"));
    assert_eq!(list(&conn, trade).unwrap(), vec![shot.clone()]);

    set_caption(&conn, shot.id, "exit too early").unwrap();
    assert_eq!(list(&conn, trade).unwrap()[0].caption, "exit too early");

    delete(&conn, &dir, shot.id).unwrap();
    assert!(files_in(&dir).is_empty());
    assert!(list(&conn, trade).unwrap().is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn two_images_in_the_same_millisecond_get_different_files() {
    let (conn, dir, trade) = setup("same-ms");
    let a = add(&conn, &dir, trade, PNG, "", 5).unwrap();
    let b = add(&conn, &dir, trade, JPG, "", 5).unwrap();
    assert_ne!(a.file_name, b.file_name);
    assert_eq!(files_in(&dir).len(), 2);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rejects_bad_input_without_leaving_files() {
    let (conn, dir, trade) = setup("reject");
    assert!(matches!(
        add(&conn, &dir, trade, b"hello", "", 1),
        Err(ShotError::NotAnImage)
    ));
    let big = [PNG, &vec![0u8; MAX_BYTES]].concat();
    assert!(matches!(
        add(&conn, &dir, trade, &big, "", 1),
        Err(ShotError::TooLarge)
    ));
    let long = "x".repeat(MAX_CAPTION + 1);
    assert!(matches!(
        add(&conn, &dir, trade, PNG, &long, 1),
        Err(ShotError::CaptionTooLong)
    ));
    assert!(matches!(
        add(&conn, &dir, 999, PNG, "", 1),
        Err(ShotError::NotFound("trade"))
    ));
    assert!(files_in(&dir).is_empty());
}

#[test]
fn failed_row_insert_removes_the_written_file() {
    let (conn, dir, trade) = setup("rollback");
    conn.execute_batch(
        "CREATE TEMP TRIGGER no_shots BEFORE INSERT ON screenshots
         BEGIN SELECT RAISE(ABORT, 'boom'); END;",
    )
    .unwrap();
    assert!(add(&conn, &dir, trade, PNG, "", 1).is_err());
    assert!(files_in(&dir).is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn missing_file_and_unknown_ids_are_clear_errors() {
    let (conn, dir, trade) = setup("missing");
    let shot = add(&conn, &dir, trade, PNG, "", 1).unwrap();
    fs::remove_file(dir.join(&shot.file_name)).unwrap();
    assert!(matches!(
        read(&conn, &dir, shot.id),
        Err(ShotError::NotFound("screenshot file"))
    ));
    // Deleting still works when the file is already gone.
    delete(&conn, &dir, shot.id).unwrap();
    assert!(matches!(
        read(&conn, &dir, 999),
        Err(ShotError::NotFound("screenshot"))
    ));
    assert!(matches!(
        set_caption(&conn, 999, "x"),
        Err(ShotError::NotFound(_))
    ));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_tampered_file_name_is_never_used_as_a_path() {
    let (conn, dir, trade) = setup("tamper");
    let id = db::screenshots::insert(&conn, trade, "../../secret.txt", "", 1).unwrap();
    assert!(matches!(read(&conn, &dir, id), Err(ShotError::NotFound(_))));
    delete(&conn, &dir, id).unwrap(); // row goes, nothing outside the folder is touched
}

#[test]
fn deleting_a_trade_leaves_files_that_startup_prunes() {
    let (conn, dir, trade) = setup("prune");
    let keep_trade = db::trades::insert(&conn, &test_data::trade()).unwrap();
    let gone = add(&conn, &dir, trade, PNG, "", 1).unwrap();
    let kept = add(&conn, &dir, keep_trade, PNG, "", 2).unwrap();
    // A file the app didn't create must survive pruning.
    fs::write(dir.join("my-notes.txt"), b"user file").unwrap();

    db::trades::delete(&conn, trade).unwrap(); // rows cascade, files stay
    assert_eq!(files_in(&dir).len(), 3);

    assert_eq!(prune_orphans(&conn, &dir).unwrap(), 1);
    let left = files_in(&dir);
    assert!(left.contains(&kept.file_name));
    assert!(left.contains(&"my-notes.txt".to_string()));
    assert!(!left.contains(&gone.file_name));
    assert_eq!(
        prune_orphans(&conn, &temp_dir("does-not-exist")).unwrap(),
        0
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn folder_sits_next_to_the_database() {
    assert_eq!(
        dir_for(Path::new("C:/data/TradeLedger/tradeledger.db")),
        Path::new("C:/data/TradeLedger/screenshots")
    );
}
