//! In-app updates: the safety backup made before a new version installs.
//!
//! ```text
//! app opens ──► updater plugin reads latest.json from the newest
//!               published GitHub Release
//!                 │ newer version?
//!                 ▼
//!               pop-up: Update now / Later
//!                 │ Update now
//!                 ▼
//!               backup_before_update  (this file)
//!                 ▼
//!               download ─► check signature ─► install ─► restart
//! ```
//!
//! Checking, downloading and the signature check are done by
//! tauri-plugin-updater; this module only keeps the journal safe.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::features::backup::{self, BackupError, Manifest};
use crate::models::TimestampMs;

const PREFIX: &str = "before-update-";
/// Older update backups beyond this many are deleted.
pub const KEEP: usize = 5;

/// `backups/before-update-<time>.zip` inside `safety_dir`.
pub fn backup_path(safety_dir: &Path, now: TimestampMs) -> PathBuf {
    safety_dir.join(format!("{PREFIX}{now}.zip"))
}

/// Saves everything to a new update backup, then trims old ones.
pub fn backup_before_update(
    conn: &Connection,
    shots_dir: &Path,
    safety_dir: &Path,
    now: TimestampMs,
) -> Result<Manifest, BackupError> {
    fs::create_dir_all(safety_dir)?;
    let manifest = backup::create(conn, shots_dir, &backup_path(safety_dir, now), now)?;
    prune(safety_dir, KEEP)?;
    Ok(manifest)
}

/// Deletes all but the newest `keep` update backups. Other files in the
/// folder (manual or before-restore backups) are never touched.
pub fn prune(safety_dir: &Path, keep: usize) -> io::Result<()> {
    let mut found: Vec<(TimestampMs, PathBuf)> = fs::read_dir(safety_dir)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let time = name.strip_prefix(PREFIX)?.strip_suffix(".zip")?;
            Some((time.parse().ok()?, entry.path()))
        })
        .collect();
    found.sort_by_key(|(time, _)| std::cmp::Reverse(*time));
    for (_, path) in found.into_iter().skip(keep) {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tradeledger-updates-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn path_has_time_in_name() {
        let p = backup_path(Path::new("backups"), 1_700_000_000_000);
        assert_eq!(
            p,
            Path::new("backups").join("before-update-1700000000000.zip")
        );
    }

    #[test]
    fn prune_keeps_newest_and_ignores_other_files() {
        let dir = temp_dir("prune");
        for t in [1, 5, 3, 2, 4] {
            fs::write(backup_path(&dir, t), b"x").unwrap();
        }
        fs::write(dir.join("before-restore-1.zip"), b"x").unwrap();
        fs::write(dir.join("before-update-notanumber.zip"), b"x").unwrap();

        prune(&dir, 2).unwrap();

        let mut names: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "before-restore-1.zip",
                "before-update-4.zip",
                "before-update-5.zip",
                "before-update-notanumber.zip",
            ]
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn backup_writes_a_readable_zip() {
        let dir = temp_dir("backup");
        let conn = crate::database::open_in_memory().unwrap();
        let safety = dir.join("backups");
        let now = 4_242_424;

        let manifest = backup_before_update(&conn, &dir.join("shots"), &safety, now).unwrap();

        assert_eq!(manifest.trades, 0);
        let again = backup::inspect(&backup_path(&safety, now)).unwrap();
        assert_eq!(again, manifest);
        fs::remove_dir_all(&dir).unwrap();
    }
}
