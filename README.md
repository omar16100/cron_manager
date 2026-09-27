# Cron Manager

A native desktop cron job manager built in Rust with [iced](https://github.com/iced-rs/iced). Visual expression builder, safe editing with backup and conflict detection, lossless crontab round-trips. Ships as a single binary; on macOS it links only system frameworks and libraries (checked with `otool -L`).

## Features

- **CRUD Operations** - Create, read, update, delete, and enable/disable cron jobs with a GUI
- **Visual Expression Builder** - Build cron schedules visually or type raw expressions
- **Schedule Preview** - Plain-language description and the next 5 execution times before saving
- **Conflict Detection** - SHA-256 checksum detects external crontab changes before overwriting
- **Automatic Backups** - Before each save the app attempts to copy the current crontab to a timestamped backup file (0600 permissions), keeping the 20 most recent. Backups are best-effort: if one fails, a warning is logged and the save still proceeds. Restore is manual: `crontab <backup-file>`
- **Lossless Round-Trips** - Lines you do not edit (comments, env vars, jobs, including `%` in commands) are written back exactly as read. Exceptions: whitespace-only lines become empty lines, CRLF line endings become LF, and non-empty output always ends with a newline
- **Search & Filter** - Filter jobs by keyword (command, schedule, description, tags) or by tag
- **Tag System** - Organize jobs with `# [tag1, tag2] description` comments
- **Special Macros** - `@reboot`, `@yearly`, `@annually`, `@monthly`, `@weekly`, `@daily`, `@midnight`, `@hourly`
- **Environment Variables** - Lines such as `MAILTO`, `PATH`, `SHELL`, `CRON_TZ` are parsed and written back unchanged (not yet shown in the GUI)

## Install

### Build from source

```bash
git clone https://github.com/omar16100/cron_manager.git
cd cron_manager
cargo build --release
./target/release/cron_manager
```

### Requirements

- Rust 1.87+ (`rust-version` in `Cargo.toml`). Verified 27 Sep 2026 on macOS with the committed `Cargo.lock`: 1.87.0 builds and passes `cargo test --locked`; 1.86.0 fails because `unicode-segmentation` 1.13.1 calls `is_multiple_of` on an unsigned integer (stable since 1.87). iced 0.13 alone declares 1.80
- A system with the `crontab` command. CI builds and tests on macOS; Linux is expected to work but is not tested in CI

## Architecture

- **GUI**: [iced](https://github.com/iced-rs/iced) v0.13 (Elm-inspired reactive framework)
- **Cron Parsing**: [croner](https://crates.io/crates/croner) for validation/preview only
- **Single Source of Truth**: `Vec<CrontabLine>` - never derived views
- **Backend Trait**: `CrontabBackend` for testability with mock implementation
- **Stable IDs**: Counter-based `JobId(u64)`, not position or content derived

## Testing

```bash
cargo test
```

61 unit tests covering parser, writer, roundtrips, expression validation, conflict detection, and backup. Tests use an in-memory backend and temporary directories; they never touch your real crontab. CI runs `cargo test --locked` on macOS for pushes to `main` and for pull requests.

## License

MIT - see [LICENSE](LICENSE)

## Built by

[omarshabab.com](https://omarshabab.com)
