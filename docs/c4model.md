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
- **Filesystem**: Local backup storage at `~/.local/share/cron_manager/backups/`

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
    │ (crontab CLI)  │    │ ~/.local/share/   │
    └────────────────┘    └──────────────────┘
```

## Level 3: Component Diagram

### GUI Layer (src/gui/)
- **theme.rs**: Color scheme, spacing, font sizes
- **views/job_list.rs**: Main scrollable job list with search and tag filter
- **views/job_editor.rs**: Create/edit form with command, description, tags
- **views/expression_builder.rs**: Visual 5-field cron builder with pick_lists

### Core Engine (src/core/)
- **backend.rs**: `CrontabBackend` trait + `SystemCrontab` impl (system I/O)
- **parser.rs**: Parses `crontab -l` output into `Vec<CrontabLine>`
- **writer.rs**: Serializes `Vec<CrontabLine>` to crontab format (lossless)
- **expression.rs**: Wraps `croner` for validation, description, next occurrences
- **conflict.rs**: SHA-256 checksum comparison for external change detection
- **backup.rs**: Creates timestamped backups with 0600 permissions, retention policy

### Model Layer (src/model/)
- **crontab.rs**: `CrontabLine` enum (Job, Special, Comment, EnvVar, Blank) — source of truth
- **job.rs**: `CronJob` (derived view), `JobDraft` (editor state), `JobId` (counter-based)
- **schedule.rs**: `ScheduleFields`, `FieldValue` — visual builder data types

### App Orchestrator (src/app.rs)
- `App` struct: owns all state
- `Message` enum: all possible user/system events
- `update()`: Elm-style state transitions
- `view()`: renders current view from state

## Data Flow

```
Read:   crontab -l → raw string → parser → Vec<CrontabLine> → extract_jobs → Vec<CronJob>
Write:  Vec<CrontabLine> → writer → raw string → backup → conflict check → crontab -
Edit:   CronJob → JobDraft → UI edits → update_job_from_draft → Vec<CrontabLine>
```

## Key Design Decisions

1. `Vec<CrontabLine>` is the single source of truth — all mutations happen here
2. Environment variables are standalone lines, not job properties
3. Stable counter-based `JobId` — never derived from content
4. SHA-256 conflict detection before every write
5. Lossless round-trips via raw text preservation
6. Visual builder is best-effort — falls back to raw for non-representable expressions
