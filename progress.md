# TradeLedger — Progress

Status: ⬜ todo · 🟨 in progress · ✅ done

**Current task:** T1.1 — Scaffold Tauri 2 + React + TS

| Task | Description | Status | Commit | Date |
|---|---|---|---|---|
| — | Requirements grilling + plan, README, progress, CLAUDE.md | ✅ | — | 2026-11-14 |
| T0.1 | Install & verify toolchain | ✅ | b236bc7 | 2026-11-14 |
| T0.2 | Git identity + GitHub repo + first push | ✅ | b236bc7 | 2026-11-14 |
| T1.1 | Scaffold Tauri 2 + React + TS | ⬜ | | |
| T1.2 | First `invoke()` command + lint/format | ⬜ | | |
| T2.1 | SQLite in app-data + migrations | ⬜ | | |
| T2.2 | Schema v1 + structs + repository + tests | ⬜ | | |
| T3.1 | Trade CRUD + R:R / R-multiple calculator | ⬜ | | |
| T3.2 | Trade entry form + live R:R preview | ⬜ | | |
| T4.1 | Binance CSV study + fixtures | ⬜ | | |
| T4.2 | Importer trait + Binance spot parser + dedupe | ⬜ | | |
| T4.3 | Binance futures parser + funding | ⬜ | | |
| T4.4 | Fill grouping engine | ⬜ | | |
| T4.5 | Import UI | ⬜ | | |
| T4.6 | Merge/split trades | ⬜ | | |
| T5.1 | Trade log table | ⬜ | | |
| T5.2 | Trade detail + notes | ⬜ | | |
| T5.3 | Tags & tag groups | ⬜ | | |
| T5.4 | Screenshots | ⬜ | | |
| T5.5 | Custom fields | ⬜ | | |
| T6.1 | Stats engine | ⬜ | | |
| T6.2 | Dashboard widgets | ⬜ | | |
| T6.3 | Drag-and-drop dashboard layout | ⬜ | | |
| T7.1 | PnL calendar heatmap | ⬜ | | |
| T8.1 | Reports | ⬜ | | |
| T8.2 | Playbooks | ⬜ | | |
| T9.1 | Themes | ⬜ | | |
| T9.2 | Backup/restore + CSV export | ⬜ | | |
| T10.1 | GitHub Actions release builds | ⬜ | | |
| T10.2 | README screenshots + v1.0.0 | ⬜ | | |

---

## Log
<!-- Newest first. One entry per finished task: what was built, what was learned, quiz result, any follow-ups. -->

### 2026-11-14 — T0.2 Git identity + GitHub repo + first push
- Repo-local git identity set: Dimuth-Wick + GitHub noreply email.
- Repo, remote, `.gitignore` and LICENSE were already in place from the planning commit.
- CLAUDE.md loop replaced with the lean version (explanations and quizzes happen in a separate chat).
- Known: the planning commit `39417d5` carries the gmail address; left as is (no force-push).
- Next: T1.1.

### 2026-11-14 — T0.1 Install & verify toolchain
- Already present: Node 24.11.0 (LTS), npm 11.6.1, git 2.55.0, WebView2 157.0.4341.57, VS Code 1.145.1.
- Installed: rustup 1.29.1 with rustc/cargo 1.100.0 (`stable-x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 (C++ workload) + Windows SDK 10.0.26100.0, VS Code extensions rust-analyzer and Tauri.
- Verified: a test Rust program compiled, linked and ran.
- No repo code changed; the progress entry went in with the T0.2 commit.
- Next: T0.2.

### 2026-11-14 — Planning
- Requirements settled through a grilling session (see plan.md §1).
- Stack: Tauri 2 (Rust) + React + TypeScript + SQLite.
- Understanding check: architecture, fill grouping, R-multiple, profit factor, data safety.
- Next: T0.1.
