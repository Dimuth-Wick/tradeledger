import { invoke } from "@tauri-apps/api/core";

// The single place where the UI talks to Rust. Components import these typed
// functions instead of calling invoke("...") with raw strings, so a renamed
// command breaks in one file instead of silently failing at runtime.

/** Mirrors Rust `commands::DbInfo` (serde camelCase). */
export interface DbInfo {
  path: string;
  schemaVersion: number;
}

export function ping(): Promise<string> {
  return invoke<string>("ping");
}

export function dbInfo(): Promise<DbInfo> {
  return invoke<DbInfo>("db_info");
}
