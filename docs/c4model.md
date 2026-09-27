# Cron Manager - C4 Architecture Model

## Level 1: System Context

```
[User] --> [Cron Manager App]
[Cron Manager App] --> [System Crontab (crontab CLI)]
[Cron Manager App] --> [Filesystem (backups)]
```

- **User**: Developer, sysadmin, or DevOps engineer managing cron jobs
- **Cron Manager App**: Native Rust GUI for CRUD operations on cron jobs
- **System Crontab**: The OS crontab service, accessed via `crontab -l` / `crontab -`
- **Filesystem**: Local backup storage at `<data_local_dir>/cron_manager/backups/` (`dirs::data_local_dir()`: `~/Library/Application Support` on macOS, `~/.local/share` on Linux)

## Level 2: Container Diagram

```
┌─────────────────────────────────────────────┐
│              Cron Manager App               │
│                                             │
│  ┌────────────┐  ┌────────────────────────┐ │
│  │ GUI Layer  │  │     Core Engine        │ │
│  │  (iced)    │──│                        │ │
│  │            │  │  Parser / Writer       │ │
│  │ Views:     │  │  Expression Engine     │ │
│  │  Job List  │  │  Conflict Detection    │ │
│  │  Editor    │  │  Backup Manager        │ │
│  │  Builder   │  │  Backend Trait         │ │
│  └────────────┘  └───────────┬────────────┘ │
│                              │              │
│  ┌────────────┐              │              │
│  │ Model      │──────────────┘              │
│  │ CrontabLine│                             │
│  │ CronJob    │                             │
│  │ Schedule   │                             │
│  └────────────┘                             │
└──────────────────────────┬──────────────────┘
                           │
              ┌────────────┴────────────┐
              │                         │
    ┌─────────▼──────┐    ┌────────────▼─────┐
    │ System Crontab │    │ Backup Filesystem │
    │ (crontab CLI)  │    │ data_local_dir    │
    └────────────────┘    └──────────────────┘
```

## Level 3: Component Diagram

### GUI Layer (src/gui/)
- **theme.rs**: Color scheme, spacing, font sizes
- **views/, components/**: placeholder modules (empty); all views currently live in `src/app.rs`:
  - `view_job_list` / `view_job_card`: scrollable job list with keyword search and tag filter
  - `view_job_editor`: create/edit form with command, description, tags, schedule preview
  - `view_visual_builder` / `view_field_picker`: visual 5-field cron builder with pick_lists

### Core Engine (src/core/)
- **backend.rs**: `CrontabBackend` trait + `SystemCrontab` impl (system I/O)
- **parser.rs**: Parses `crontab -l` output into `Vec<CrontabLine>`
- **writer.rs**: Serializes `Vec<CrontabLine>` to crontab format (lossless)
- **expression.rs**: Wraps `croner` for validation, description, next occurrences
- **conflict.rs**: SHA-256 checksum comparison for external change detection
- **backup.rs**: Creates timestamped backups with 0600 permissions, retention policy

### Model Layer (src/model/)
- **crontab.rs**: `CrontabLine` enum (Job, Special, Comment, EnvVar, Blank), the source of truth
- **job.rs**: `CronJob` (derived view), `JobDraft` (editor state), `JobId` (counter-based)
- **schedule.rs**: `ScheduleFields`, `FieldValue`: visual builder data types

### App Orchestrator (src/app.rs)
- `App` struct: owns all state
- `Message` enum: all possible user/system events
- `update()`: Elm-style state transitions
- `view()`: renders current view from state

## Data Flow

```
Read:   crontab -l → raw string → parser → Vec<CrontabLine> → extract_jobs → Vec<CronJob>
Write:  Vec<CrontabLine> → writer → raw string → conflict check (abort + reload on mismatch) → best-effort backup of live crontab → crontab -
Edit:   CronJob → JobDraft → UI edits → update_job_from_draft → Vec<CrontabLine>
```

## Key Design Decisions

1. `Vec<CrontabLine>` is the single source of truth; all mutations happen here
2. Environment variables are standalone lines, not job properties
3. Stable counter-based `JobId`, never derived from content
4. SHA-256 conflict detection before every write
5. Lossless round-trips via raw text preservation
6. Visual builder only represents expressions that `expression::decompose` accepts; switching to Visual with any other expression (e.g. a macro) starts from default fields rather than falling back to raw (`is_builder_compatible` exists but the UI does not call it yet)
