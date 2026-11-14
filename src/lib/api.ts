import { invoke } from "@tauri-apps/api/core";

// The single place where the UI talks to Rust. Components import these typed
// functions instead of calling invoke("...") with raw strings, so a renamed
// command breaks in one file instead of silently failing at runtime.

export function ping(): Promise<string> {
  return invoke<string>("ping");
}
