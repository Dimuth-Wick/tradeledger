# CLAUDE.md — rules for working on TradeLedger

Read `plan.md` and `progress.md` at the start of every session. The current task is at the top of `progress.md`.
Both files are **local only** (git-ignored): never commit them, and never make a commit just because they changed.

## Who you're working with

- The owner (GitHub: **Dimuth-Wick**) wants to **fully understand** the project, not vibe-code it. You write the code; he must understand every piece.
- He learns best from **visuals** (ASCII/mermaid diagrams, tables). Keep text short.
- Don't ask what he already knows or guess his skill level. Teach the new subject through the task itself.

## The loop (for every task)

1. Implement the task I name. Follow plan.md. No concept explanations, no quizzes.
2. Add tests; run cargo test / clippy / lint / npm test where they exist. Fix failures.
3. Update progress.md (mark ✅, commit hash, date, a short log entry). Local file only, not committed.
4. Commit and push per the Git rules.
5. Reply with a SHORT summary only:
   - files changed (one line each: what + why)
   - commands I need to run myself (if any)
   - anything that needs my decision

   Then stop. Never start the next task until I say so.

6. If something needs my decision mid-task, ask briefly and wait.

## Git rules

- Commits must be authored by **Dimuth-Wick** using his **GitHub noreply email** (`<id>+Dimuth-Wick@users.noreply.github.com`).
  - Before the first commit of a session, run `git config user.name` and `git config user.email`. If they aren't his name + noreply email, **stop and ask him** for the correct values. Never invent them.
- **Do NOT** add `Co-Authored-By`, "Generated with Claude", or any AI attribution to commits or PRs.
- One commit per task (more is fine if logical), using Conventional Commits:
  `feat(import): binance spot csv parser (T4.2)`, `docs: update progress (T4.2)`, `test: ...`, `fix: ...`.
- Push to `origin main` after each task (`git push`). If the push fails, report the error. Don't force-push.

## Safety rules

- **Never commit real trading data**: no `*.db`, `*.sqlite`, real `*.csv` exports, screenshots of his account, `.env`, or API keys. Test fixtures must use **fake** data and live in `tests/fixtures/`.
- The app stores data only in the OS app-data folder (`%APPDATA%\TradeLedger`), never inside the repo.
- Money math uses `rust_decimal`, never `f64`.
- All business logic lives in Rust (`src-tauri/`) and is unit-tested. React only displays data and calls `invoke()` commands.
- Don't add new dependencies without explaining why in step 2.

## Project conventions

- Rust: `rustfmt` + `clippy` clean. Errors via `thiserror`; commands return `Result<T, String>` to the UI.
- TypeScript: strict mode, ESLint + Prettier. Shared types mirror Rust structs (serde camelCase).
- Folder layout (target):
  ```
  src/                 React UI (pages/, components/, lib/)
  src-tauri/src/       Rust core: commands/, db/, importers/, grouping/, stats/
  tests/fixtures/      fake CSV samples
  docs/                extra notes (e.g. binance-csv.md)
  ```
