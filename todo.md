# todo

## Done
- 27 Sep 2026: corrected README and Pages claims to match the code (removed undo/redo, keyboard shortcuts, sliders, one-click restore, status filter, env var display; measured binary size; "0 runtime deps" reworded).
- 27 Sep 2026: Rust requirement 1.70+ corrected to 1.87+ (verified 1.86.0 fails, 1.87.0 passes); `rust-version` added to `Cargo.toml`.
- 27 Sep 2026: added GitHub Actions CI (`.github/workflows/ci.yml`, macOS build + test).
- 27 Sep 2026: fixed `docs/c4model.md` (view locations, platform backup path, em dashes).
- 27 Sep 2026: set GitHub description, homepage and topics.

## Open
- Features advertised before but not implemented: undo/redo, keyboard shortcuts, backup restore from the GUI, enabled/disabled filter, env var display.
- Linux is not covered by CI.
- Visual builder: switching to Visual with an expression `decompose` cannot represent (e.g. `@daily`) resets to default fields; `is_builder_compatible` exists but is unused by the UI.
- Backups are best-effort: a failed backup only logs a warning and the save proceeds.
