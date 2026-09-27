# todo

## Done
- 27 Sep 2026: corrected README and Pages claims to match the code (removed undo/redo, keyboard shortcuts, sliders, one-click restore, status filter, env var display; measured binary size; "0 runtime deps" reworded).
- 27 Sep 2026: Rust requirement 1.70+ corrected to 1.87+ (verified 1.86.0 fails, 1.87.0 passes); `rust-version` added to `Cargo.toml`.
- 27 Sep 2026: added GitHub Actions CI (`.github/workflows/ci.yml`, macOS build + test).
- 27 Sep 2026: fixed `docs/c4model.md` (view locations, platform backup path, em dashes).
- 27 Sep 2026: set GitHub description, homepage and topics.
- 27 Sep 2026: `rand` 0.8.5 to 0.8.8 in `Cargo.lock` (Dependabot alert 2); `lru` 0.12.5 alert dismissed as tolerable risk, `IterMut` is never reached through iced 0.13 (see `docs/27092026_security_deps_plan.md`).

## Open
- Features advertised before but not implemented: undo/redo, keyboard shortcuts, backup restore from the GUI, enabled/disabled filter, env var display.
- Linux is not covered by CI.
- Visual builder: switching to Visual with an expression `decompose` cannot represent (e.g. `@daily`) resets to default fields; `is_builder_compatible` exists but is unused by the UI.
- Backups are best-effort: a failed backup only logs a warning and the save proceeds.
- `lru` 0.12.5 stays in the lockfile until a dependency change removes it: 0.13's `iced_glyphon` requires `lru ^0.12`. Likely route is iced 0.14 (`cryoglyph`, `lru ^0.16`); a `tiny-skia`-only renderer is the untried alternative.
