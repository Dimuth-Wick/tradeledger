//! Backup tests with real temp folders and zips (fake data only).

use super::*;
use crate::database::test_data;

/// A fresh folder for one test: app data (db + screenshots) and outputs.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("tl-backup-{name}-{nanos}"));
        fs::create_dir_all(root.join("app")).unwrap();
        Sandbox { root }
    }
    fn db_path(&self) -> PathBuf {
        self.root.join("app").join("tradeledger.db")
    }
    fn shots(&self) -> PathBuf {
        self.root.join("app").join("screenshots")
    }
    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn count_trades(conn: &Connection) -> u32 {
    conn.query_row("SELECT COUNT(*) FROM trades", [], |r| r.get(0))
        .unwrap()
}

fn add_trade(conn: &Connection) {
    db::trades::insert(conn, &test_data::trade()).unwrap();
}

/// A zip with exactly these entries, for hand-made bad backups.
fn make_zip(path: &Path, entries: &[(&str, &[u8])]) {
    let mut zip = ZipWriter::new(File::create(path).unwrap());
    for (name, bytes) in entries {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
}

fn manifest_json(schema_version: u32) -> Vec<u8> {
    serde_json::to_vec(&Manifest {
        app: APP.into(),
        format: FORMAT,
        schema_version,
        created_at: 1,
        trades: 0,
        screenshots: 0,
    })
    .unwrap()
}

#[test]
fn backup_then_restore_brings_everything_back() {
    let sb = Sandbox::new("roundtrip");
    let mut conn = db::open(&sb.db_path()).unwrap();
    add_trade(&conn);
    add_trade(&conn);
    fs::create_dir_all(sb.shots()).unwrap();
    fs::write(sb.shots().join("t1-1000-0.png"), [1u8, 2, 3]).unwrap();
    fs::write(sb.shots().join("notes.txt"), b"not ours").unwrap(); // left out

    let zip_path = sb.path("backup.zip");
    let made = create(&conn, &sb.shots(), &zip_path, 1_000).unwrap();
    assert_eq!((made.trades, made.screenshots), (2, 1));
    assert_eq!(made.schema_version, db::migrations_latest());
    assert_eq!(inspect(&zip_path).unwrap(), made);
    assert!(!sibling(&zip_path, ".part").exists());

    // Life goes on: a new trade, a different image.
    add_trade(&conn);
    fs::remove_file(sb.shots().join("t1-1000-0.png")).unwrap();
    fs::write(sb.shots().join("t9-9000-0.png"), [9u8]).unwrap();

    let result = restore(&mut conn, &sb.shots(), &zip_path, &sb.path("safety"), 2_000).unwrap();
    assert_eq!(result.restored, made);

    // The same connection now sees the backup's data.
    assert_eq!(count_trades(&conn), 2);
    let names: Vec<String> = fs::read_dir(sb.shots())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["t1-1000-0.png"]);
    assert_eq!(
        fs::read(sb.shots().join("t1-1000-0.png")).unwrap(),
        [1, 2, 3]
    );

    // What was there before the restore was kept.
    let safety = inspect(Path::new(&result.safety_backup)).unwrap();
    assert_eq!((safety.trades, safety.screenshots), (3, 1));
    assert!(!sibling(&sb.shots(), ".restoring").exists());
    assert!(!sibling(&sb.shots(), ".old").exists());
}

#[test]
fn backup_works_without_a_screenshot_folder() {
    let sb = Sandbox::new("noshots");
    let conn = db::open(&sb.db_path()).unwrap();
    let made = create(&conn, &sb.shots(), &sb.path("b.zip"), 1).unwrap();
    assert_eq!((made.trades, made.screenshots), (0, 0));
}

/// Restores that must fail leave the current data exactly as it was.
fn assert_refused(sb: &Sandbox, zip_path: &Path, expected: &str) {
    let mut conn = db::open(&sb.db_path()).unwrap();
    add_trade(&conn);
    let err = restore(&mut conn, &sb.shots(), zip_path, &sb.path("safety"), 5)
        .unwrap_err()
        .to_string();
    assert!(err.contains(expected), "{err}");
    assert_eq!(count_trades(&conn), 1);
    assert!(
        !sb.path("safety").exists(),
        "no safety copy for a refused file"
    );
}

#[test]
fn refuses_unexpected_files_in_the_zip() {
    let sb = Sandbox::new("slip");
    let schema = manifest_json(db::migrations_latest());
    for bad in [
        "../evil.txt",
        "screenshots/../../evil.png",
        "screenshots/evil.exe",
        "other.db",
    ] {
        let zip_path = sb.path("bad.zip");
        make_zip(
            &zip_path,
            &[(MANIFEST, &schema), (DB_ENTRY, b"x"), (bad, b"payload")],
        );
        assert!(
            matches!(inspect(&zip_path), Err(BackupError::NotABackup)),
            "{bad}"
        );
    }
    assert_refused(&sb, &sb.path("bad.zip"), "not a TradeLedger backup");
    assert!(!sb.root.join("evil.txt").exists());
}

#[test]
fn refuses_a_backup_from_a_newer_app() {
    let sb = Sandbox::new("newer");
    let zip_path = sb.path("new.zip");
    make_zip(
        &zip_path,
        &[(MANIFEST, &manifest_json(999)), (DB_ENTRY, b"x")],
    );
    assert_refused(&sb, &zip_path, "newer TradeLedger");
}

#[test]
fn refuses_files_that_are_not_backups() {
    let sb = Sandbox::new("notzip");
    let txt = sb.path("trades.csv");
    fs::write(&txt, b"Date,Pair\n").unwrap();
    assert!(matches!(inspect(&txt), Err(BackupError::NotABackup)));

    // A zip without our manifest is not a backup either.
    let other = sb.path("other.zip");
    make_zip(&other, &[(DB_ENTRY, b"x")]);
    assert_refused(&sb, &other, "not a TradeLedger backup");
}

#[test]
fn refuses_a_damaged_database() {
    let sb = Sandbox::new("damaged");
    let zip_path = sb.path("damaged.zip");
    make_zip(
        &zip_path,
        &[
            (MANIFEST, &manifest_json(db::migrations_latest())),
            (DB_ENTRY, b"this is not a sqlite file at all"),
        ],
    );
    assert_refused(&sb, &zip_path, "damaged");
}
