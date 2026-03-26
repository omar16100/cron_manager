# Cron Manager

A fast, native desktop cron job manager built in Rust. Visual expression builder, safe editing with backup and conflict detection, lossless crontab round-trips. Single binary, zero runtime dependencies.

## Features

- **CRUD Operations** - Create, read, update, delete cron jobs with a clean GUI
- **Visual Expression Builder** - Build cron schedules visually or type raw expressions
- **Schedule Preview** - See next N execution times before saving
- **Conflict Detection** - SHA-256 checksum detects external crontab changes before overwriting
- **Automatic Backups** - Timestamped backups with 20-file retention, one-click restore
- **Lossless Round-Trips** - Preserves comments, whitespace, env vars, and `%` in commands verbatim
- **Search & Filter** - Filter jobs by keyword, tag, or enabled/disabled status
- **Tag System** - Organize jobs with `# [tag1, tag2] description` comments
- **Undo/Redo** - In-session edit history
- **Keyboard Shortcuts** - Power-user friendly
- **Special Macros** - Full support for `@reboot`, `@daily`, `@weekly`, `@monthly`, `@yearly`, `@hourly`
- **Environment Variables** - Visibility for `MAILTO`, `PATH`, `SHELL`, `CRON_TZ`

## Install

### Build from source

```bash
git clone https://github.com/omar16100/cron_manager.git
cd cron_manager
cargo build --release
./target/release/cron_manager
```

### Requirements

- Rust 1.70+
- A system with `crontab` (Linux, macOS)

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

61 unit tests covering parser, writer, roundtrips, expression validation, conflict detection, and backup.

## License

MIT - see [LICENSE](LICENSE)

## Built by

[omarshabab.com](https://omarshabab.com)
