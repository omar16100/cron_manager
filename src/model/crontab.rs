use crate::model::job::JobId;

/// A single line from the crontab file.
/// This is the **source of truth** — all mutations happen here.
/// CronJob views are derived from this, never stored independently.
#[derive(Debug, Clone, PartialEq)]
pub enum CrontabLine {
    /// A standard 5-field cron job: `*/5 * * * * /usr/bin/cmd`
    Job {
        id: JobId,
        expression: String,
        command: String,
        enabled: bool,
        raw: String,
    },
    /// A special macro job: `@reboot /usr/bin/cmd`
    Special {
        id: JobId,
        macro_name: String,
        command: String,
        enabled: bool,
        raw: String,
    },
    /// A standalone comment line: `# some comment`
    Comment(String),
    /// An environment variable assignment: `SHELL=/bin/bash`
    EnvVar {
        key: String,
        value: String,
        raw: String,
    },
    /// A blank line
    Blank,
}

impl CrontabLine {
    /// Returns the job ID if this line is a Job or Special.
    pub fn job_id(&self) -> Option<JobId> {
        match self {
            Self::Job { id, .. } | Self::Special { id, .. } => Some(*id),
            _ => None,
        }
    }

    /// Returns true if this line is a job (standard or special).
    pub fn is_job(&self) -> bool {
        matches!(self, Self::Job { .. } | Self::Special { .. })
    }

    /// Returns true if this line is an enabled job.
    pub fn is_enabled_job(&self) -> bool {
        match self {
            Self::Job { enabled, .. } | Self::Special { enabled, .. } => *enabled,
            _ => false,
        }
    }

    /// Toggle the enabled state of a job line. No-op for non-job lines.
    pub fn toggle_enabled(&mut self) {
        match self {
            Self::Job { enabled, raw, expression, command, .. } => {
                *enabled = !*enabled;
                if *enabled {
                    *raw = format!("{} {}", expression, command);
                } else {
                    *raw = format!("# {} {}", expression, command);
                }
            }
            Self::Special { enabled, raw, macro_name, command, .. } => {
                *enabled = !*enabled;
                if *enabled {
                    *raw = format!("{} {}", macro_name, command);
                } else {
                    *raw = format!("# {} {}", macro_name, command);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_id_returns_some_for_jobs() {
        let id = JobId::next();
        let line = CrontabLine::Job {
            id,
            expression: "* * * * *".into(),
            command: "/bin/true".into(),
            enabled: true,
            raw: "* * * * * /bin/true".into(),
        };
        assert_eq!(line.job_id(), Some(id));
    }

    #[test]
    fn test_job_id_returns_none_for_comment() {
        let line = CrontabLine::Comment("hello".into());
        assert_eq!(line.job_id(), None);
    }

    #[test]
    fn test_toggle_enabled() {
        let id = JobId::next();
        let mut line = CrontabLine::Job {
            id,
            expression: "*/5 * * * *".into(),
            command: "/usr/bin/backup.sh".into(),
            enabled: true,
            raw: "*/5 * * * * /usr/bin/backup.sh".into(),
        };

        line.toggle_enabled();
        assert!(!line.is_enabled_job());
        if let CrontabLine::Job { raw, .. } = &line {
            assert_eq!(raw, "# */5 * * * * /usr/bin/backup.sh");
        }

        line.toggle_enabled();
        assert!(line.is_enabled_job());
        if let CrontabLine::Job { raw, .. } = &line {
            assert_eq!(raw, "*/5 * * * * /usr/bin/backup.sh");
        }
    }
}
