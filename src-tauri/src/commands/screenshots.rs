//! Screenshot commands. Images travel as raw bytes in both directions
//! (no base64), using Tauri's binary IPC.

use std::time::{SystemTime, UNIX_EPOCH};

use tauri::State;
use tauri::ipc::{InvokeBody, Request, Response};

use crate::db::Db;
use crate::db::screenshots::Screenshot;
use crate::screenshots;

fn now_ms() -> Result<i64, String> {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis();
    i64::try_from(ms).map_err(|e| e.to_string())
}

/// Body = the image bytes. Headers: `x-trade-id`, and `x-caption`
/// (URI-encoded, since headers must be plain ASCII).
#[tauri::command]
pub fn add_screenshot(db: State<'_, Db>, request: Request<'_>) -> Result<Screenshot, String> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("expected the image as raw bytes".into());
    };
    let header = |name: &str| {
        request
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    };
    let trade_id: i64 = header("x-trade-id")
        .and_then(|v| v.parse().ok())
        .ok_or("missing trade id")?;
    let caption = header("x-caption")
        .map(|c| percent_decode(&c))
        .transpose()?
        .unwrap_or_default();

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let dir = screenshots::dir_for(&db.path);
    screenshots::add(&conn, &dir, trade_id, bytes, &caption, now_ms()?).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_screenshots(db: State<'_, Db>, trade_id: i64) -> Result<Vec<Screenshot>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    screenshots::list(&conn, trade_id).map_err(|e| e.to_string())
}

/// Returns the raw image; the UI turns it into a Blob URL.
#[tauri::command]
pub fn screenshot_image(db: State<'_, Db>, id: i64) -> Result<Response, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let (bytes, _mime) =
        screenshots::read(&conn, &screenshots::dir_for(&db.path), id).map_err(|e| e.to_string())?;
    Ok(Response::new(bytes))
}

#[tauri::command]
pub fn set_screenshot_caption(db: State<'_, Db>, id: i64, caption: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    screenshots::set_caption(&conn, id, &caption).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_screenshot(db: State<'_, Db>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    screenshots::delete(&conn, &screenshots::dir_for(&db.path), id).map_err(|e| e.to_string())
}

/// Decodes `encodeURIComponent` output ("%C3%A9" → "é").
fn percent_decode(s: &str) -> Result<String, String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = s.get(i + 1..i + 3).ok_or("bad caption encoding")?;
            out.push(u8::from_str_radix(hex, 16).map_err(|_| "bad caption encoding")?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| "caption is not valid text".into())
}

#[cfg(test)]
mod tests {
    use super::percent_decode;

    #[test]
    fn decodes_uri_component_text() {
        assert_eq!(
            percent_decode("entry%20on%20retest").unwrap(),
            "entry on retest"
        );
        assert_eq!(percent_decode("caf%C3%A9%20%E2%9C%93").unwrap(), "café ✓");
        assert_eq!(percent_decode("plain").unwrap(), "plain");
        assert!(percent_decode("%zz").is_err());
        assert!(percent_decode("%4").is_err());
        assert!(percent_decode("%FF").is_err());
    }
}
