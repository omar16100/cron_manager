# Security dependency alerts plan (27 Sep 2026)

## Status
In review: `rand` lockfile update on branch `chore/security-deps-27092026`; `lru` alert to be dismissed as tolerable risk after merge.

## Goal
Clear the two low-severity Dependabot alerts opened when alerts were enabled on 27 Sep 2026, without changing app code.

## Findings
- Alert 2, `rand` 0.8.5 (GHSA-cq8v-f236-94qc, unsound only with the `log` feature, a custom logger that itself draws from `ThreadRng`, and a reseed during that call). Pulled in by `zbus` 4.4.0 via `dark-light` 1.1.1 and `iced_core` 0.13.2. Fixed by `cargo update -p rand@0.8.5 --precise 0.8.8`; the lockfile diff is identical to Dependabot PR #2.
- Alert 1, `lru` 0.12.5 (GHSA-rhfx-m35p-ff5j, `IterMut::next`/`next_back` violate Stacked Borrows). The fix is `lru` 0.16.3, outside the `^0.12` range that `iced_glyphon` 0.6.0 (via `iced_wgpu` 0.13.5) requires, so no lockfile update can reach it. Moving off it means upgrading iced to 0.14 (its `iced_wgpu` 0.14.0 uses `cryoglyph` 0.1, which requires `lru ^0.16`), a breaking API upgrade.
- Reachability of `IterMut`: `lru` 0.12.5 builds an `IterMut` only through `LruCache::iter_mut` and `IntoIterator for &mut LruCache`. `iced_glyphon` keeps its `LruCache` in the crate-private `InnerAtlas` and calls `unbounded_with_hasher`, `contains`, `peek`, `peek_lru`, `pop_lru`, `promote`, `put` and a shared `for .. in &self.glyph_cache` (which is `Iter`, not `IterMut`). No other crate in the lockfile uses `lru`, and this app has no direct dependency on it.

## Decisions
- One PR with the `rand` update plus docs; Dependabot PR #2 closed as superseded.
- `lru` alert dismissed as `tolerable_risk` with the reachability reason above. Revisit when upgrading iced.

## Verification
- `cargo build --locked` and `cargo test --locked` pass (61 tests, macOS arm64, rustc 1.95.0).
- CI on the PR and on `main` after merge; open alerts re-queried after merge.

## Deviations
None.
