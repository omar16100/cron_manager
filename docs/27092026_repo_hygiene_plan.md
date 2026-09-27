# Repo hygiene, claims check and CI plan (27 Sep 2026)

## Status
Done: README, Pages landing page (`docs/index.html`) and `docs/c4model.md` corrected; CI added; repo metadata set.

## Goal
Make every claim in the README and on the Pages site trace to the code or to a dated measurement, fix the Rust version requirement, and add CI so `main` is built and tested on every push.

## Findings
- "Rust 1.70+" was wrong. With the committed `Cargo.lock`, Rust 1.86.0 fails (`unicode-segmentation` 1.13.1 calls `is_multiple_of` on an unsigned integer, stable since 1.87) and 1.87.0 builds and passes all 61 tests (macOS arm64, 27 Sep 2026). iced 0.13.1 alone declares `rust-version = 1.80`, but transitive crates need more. Set `rust-version = "1.87"` in `Cargo.toml`.
- "~9MB" binary: measured 9,627,232 bytes (9.2 MiB) for `cargo build --release`, rustc 1.95.0, macOS 26.3 arm64, 27 Sep 2026. Kept on the landing page with that note.
- "0 runtime deps": `otool -L` on the release binary lists only macOS system frameworks plus `libSystem`, `libobjc` and `libiconv` from `/usr/lib`. Reworded to "links only system frameworks and libraries on macOS". Linux was not measured.
- Features claimed but not in the code (checked `src/app.rs`): undo/redo, keyboard shortcuts, sliders in the builder, one-click backup restore, filter by enabled/disabled status, GUI display of environment variables. Removed or reworded. Environment variable lines are parsed and preserved on write; backups are restored manually with `crontab <file>`.
- `docs/c4model.md` listed `src/gui/views/*.rs` files that do not exist (views live in `src/app.rs`), gave a Linux-only backup path, had the write flow as backup before conflict check (code does conflict check first) and claimed the visual builder falls back to raw mode (it resets to default fields). Corrected.
- Codex review: backups are best-effort (save proceeds if backup fails) and round-trips are not fully byte-exact (whitespace-only lines, CRLF, final newline). README and landing page now say so. Removed unmeasured marketing ("Fast", "Install in seconds", "safety features the command line can't").

## Decisions
- CI runs on `macos-latest` only (checkout, stable toolchain, rust-cache, `cargo build --locked`, `cargo test --locked`). Linux is not covered by CI, so docs say Linux is expected to work but untested.
- GitHub Pages builds from `main` `/docs` (legacy build), so merging to `main` redeploys the landing page.
- Repo metadata: description, homepage (Pages URL) and topics set with `gh repo edit`.

## Deviations
None.
