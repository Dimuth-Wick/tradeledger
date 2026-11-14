//! Tauri commands: the only functions the React UI can call via `invoke()`.
//!
//! Commands stay thin: they validate input, call into the core modules
//! (db, importers, grouping, stats) and turn errors into `String` so the UI
//! gets a plain message instead of a Rust type it can't understand.

/// Health check used by the UI to prove the Rust core is reachable.
///
/// Returns `Result` even though it can't fail, so every command has the same
/// shape on the TypeScript side (a promise that resolves or rejects).
#[tauri::command]
pub fn ping() -> Result<String, String> {
    Ok(format!(
        "pong from TradeLedger core v{}",
        env!("CARGO_PKG_VERSION")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_answers_with_pong_and_version() {
        let reply = ping().expect("ping never fails");
        assert!(reply.starts_with("pong"));
        assert!(reply.contains(env!("CARGO_PKG_VERSION")));
    }
}
