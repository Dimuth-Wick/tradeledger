//! Trade screenshots: image files in the app-data folder, one DB row each.
//!
//! Safety rules:
//! - The app generates every file name (`t<trade>-<ms>-<n>.<ext>`); names
//!   coming from the user are never used as paths.
//! - The type is detected from the file's first bytes, not its name, and only
//!   common image formats are accepted.
//! - Files are only ever read or deleted inside the screenshots folder, and
//!   only if the name matches the generated pattern.

use std::collections::HashSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use rusqlite::Connection;
use thiserror::Error;

use crate::db::screenshots::Screenshot;
use crate::db::{self, DbError};
use crate::models::TimestampMs;

pub const MAX_BYTES: usize = 20 * 1024 * 1024;
pub const MAX_CAPTION: usize = 200;
const FOLDER: &str = "screenshots";

#[derive(Debug, Error)]
pub enum ShotError {
    #[error("this file isn't a supported image (PNG, JPEG, WebP or GIF)")]
    NotAnImage,
    #[error("image is too large (max 20 MB)")]
    TooLarge,
    #[error("caption is too long (max 200 characters)")]
    CaptionTooLong,
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("could not save the image file: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Db(#[from] DbError),
}

impl From<rusqlite::Error> for ShotError {
    fn from(e: rusqlite::Error) -> Self {
        ShotError::Db(e.into())
    }
}

/// The screenshots folder for a database file (same folder as the DB).
pub fn dir_for(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .map_or_else(|| PathBuf::from(FOLDER), |p| p.join(FOLDER))
}

/// File extension for supported image bytes, from their signature.
pub fn detect_image(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

pub fn mime_for(file_name: &str) -> &'static str {
    match file_name.rsplit('.').next() {
        Some("png") => "image/png",
        Some("jpg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

/// Only names this module generates: letters, digits, '-', one '.ext'.
pub(crate) fn is_generated_name(name: &str) -> bool {
    let Some((stem, ext)) = name.rsplit_once('.') else {
        return false;
    };
    name.starts_with('t')
        && !stem.is_empty()
        && stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && matches!(ext, "png" | "jpg" | "gif" | "webp")
}

fn new_file_name(trade_id: i64, now: TimestampMs, ext: &str) -> String {
    // The counter keeps two pastes in the same millisecond apart.
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("t{trade_id}-{now}-{n}.{ext}")
}

fn clean_caption(caption: &str) -> Result<String, ShotError> {
    let c = caption.trim();
    if c.chars().count() > MAX_CAPTION {
        return Err(ShotError::CaptionTooLong);
    }
    Ok(c.to_string())
}

/// Saves the image file, then its row. If the row can't be saved, the file
/// is removed again so nothing is left half-done.
pub fn add(
    conn: &Connection,
    dir: &Path,
    trade_id: i64,
    bytes: &[u8],
    caption: &str,
    now: TimestampMs,
) -> Result<Screenshot, ShotError> {
    if bytes.len() > MAX_BYTES {
        return Err(ShotError::TooLarge);
    }
    let ext = detect_image(bytes).ok_or(ShotError::NotAnImage)?;
    let caption = clean_caption(caption)?;
    if db::trades::get(conn, trade_id)?.is_none() {
        return Err(ShotError::NotFound("trade"));
    }

    fs::create_dir_all(dir)?;
    let file_name = new_file_name(trade_id, now, ext);
    let path = dir.join(&file_name);
    fs::write(&path, bytes)?;

    match db::screenshots::insert(conn, trade_id, &file_name, &caption, now) {
        Ok(id) => Ok(Screenshot {
            id,
            trade_id,
            file_name,
            caption,
            created_at: now,
        }),
        Err(e) => {
            let _ = fs::remove_file(&path);
            Err(e.into())
        }
    }
}

pub fn list(conn: &Connection, trade_id: i64) -> Result<Vec<Screenshot>, ShotError> {
    Ok(db::screenshots::list_for_trade(conn, trade_id)?)
}

/// The image bytes and their MIME type, for display.
pub fn read(conn: &Connection, dir: &Path, id: i64) -> Result<(Vec<u8>, &'static str), ShotError> {
    let shot = db::screenshots::get(conn, id)?.ok_or(ShotError::NotFound("screenshot"))?;
    if !is_generated_name(&shot.file_name) {
        return Err(ShotError::NotFound("screenshot file"));
    }
    match fs::read(dir.join(&shot.file_name)) {
        Ok(bytes) => Ok((bytes, mime_for(&shot.file_name))),
        Err(e) if e.kind() == ErrorKind::NotFound => Err(ShotError::NotFound("screenshot file")),
        Err(e) => Err(e.into()),
    }
}

pub fn set_caption(conn: &Connection, id: i64, caption: &str) -> Result<(), ShotError> {
    let caption = clean_caption(caption)?;
    db::screenshots::set_caption(conn, id, &caption)?
        .then_some(())
        .ok_or(ShotError::NotFound("screenshot"))
}

/// Removes the row, then the file. A file that's already gone is fine.
pub fn delete(conn: &Connection, dir: &Path, id: i64) -> Result<(), ShotError> {
    let shot = db::screenshots::get(conn, id)?.ok_or(ShotError::NotFound("screenshot"))?;
    db::screenshots::delete(conn, id)?;
    if is_generated_name(&shot.file_name) {
        match fs::remove_file(dir.join(&shot.file_name)) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// Deletes image files no row points to any more (their trade was deleted,
/// which removes the rows but can't remove files). Run at startup.
/// Only files with generated names are touched. Returns how many were removed.
pub fn prune_orphans(conn: &Connection, dir: &Path) -> Result<usize, ShotError> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e.into()),
    };
    let referenced: HashSet<String> = db::screenshots::all_file_names(conn)?.into_iter().collect();
    let mut removed = 0;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type()?.is_file() && is_generated_name(&name) && !referenced.contains(&name) {
            fs::remove_file(entry.path())?;
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests;
