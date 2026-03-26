use std::sync::atomic::{AtomicU64, Ordering};

/// Stable in-memory identifier for a cron job.
/// Counter-based — assigned on parse, never derived from content or position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(pub u64);

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl JobId {
    pub fn next() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    #[cfg(test)]
    pub fn reset() {
        NEXT_ID.store(1, Ordering::Relaxed);
    }
}

/// A derived, read-only view of a cron job for display in the GUI.
/// Never stored — always computed from `Vec<CrontabLine>`.
#[derive(Debug, Clone, PartialEq)]
pub struct CronJob {
    pub id: JobId,
    pub expression: String,
    pub command: String,
    pub enabled: bool,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub is_special: bool,
    pub line_index: usize,
}

/// Draft state for the job editor (create or edit).
#[derive(Debug, Clone, Default)]
pub struct JobDraft {
    pub expression: String,
    pub command: String,
    pub comment: String,
    pub tags_input: String,
    pub enabled: bool,
    pub is_special: bool,
}

impl JobDraft {
    pub fn from_job(job: &CronJob) -> Self {
        Self {
            expression: job.expression.clone(),
            command: job.command.clone(),
            comment: job.description.clone().unwrap_or_default(),
            tags_input: job.tags.join(", "),
            enabled: job.enabled,
            is_special: job.is_special,
        }
    }

    pub fn parsed_tags(&self) -> Vec<String> {
        self.tags_input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_id_increments() {
        let a = JobId::next();
        let b = JobId::next();
        assert_ne!(a, b);
        assert!(b.0 > a.0);
    }

    #[test]
    fn test_draft_parsed_tags() {
        let draft = JobDraft {
            tags_input: "backup, daily, system".into(),
            ..Default::default()
        };
        assert_eq!(draft.parsed_tags(), vec!["backup", "daily", "system"]);
    }

    #[test]
    fn test_draft_parsed_tags_empty() {
        let draft = JobDraft {
            tags_input: "".into(),
            ..Default::default()
        };
        assert!(draft.parsed_tags().is_empty());
    }

    #[test]
    fn test_draft_from_job() {
        let job = CronJob {
            id: JobId::next(),
            expression: "*/5 * * * *".into(),
            command: "/bin/test".into(),
            enabled: true,
            description: Some("Test job".into()),
            tags: vec!["test".into(), "dev".into()],
            is_special: false,
            line_index: 0,
        };
        let draft = JobDraft::from_job(&job);
        assert_eq!(draft.expression, "*/5 * * * *");
        assert_eq!(draft.command, "/bin/test");
        assert_eq!(draft.comment, "Test job");
        assert_eq!(draft.tags_input, "test, dev");
        assert!(draft.enabled);
    }
}
