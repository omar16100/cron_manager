use crate::model::crontab::CrontabLine;
use crate::model::job::JobDraft;
use tracing::{debug, info};

/// Serialize `Vec<CrontabLine>` back to crontab format.
/// For unmodified lines, outputs the original `raw` text for lossless round-trips.
pub fn serialize_crontab(lines: &[CrontabLine]) -> String {
    let mut output = String::with_capacity(lines.len() * 40);

    for line in lines {
        match line {
            CrontabLine::Job { raw, .. } => {
                output.push_str(raw);
                output.push('\n');
            }
            CrontabLine::Special { raw, .. } => {
                output.push_str(raw);
                output.push('\n');
            }
            CrontabLine::Comment(text) => {
                output.push_str(text);
                output.push('\n');
            }
            CrontabLine::EnvVar { raw, .. } => {
                output.push_str(raw);
                output.push('\n');
            }
            CrontabLine::Blank => {
                output.push('\n');
            }
        }
    }

    debug!(bytes = output.len(), lines = lines.len(), "Serialized crontab");
    output
}

/// Build new CrontabLine entries from a JobDraft.
/// Returns a vec because it may include a preceding comment line for description/tags.
pub fn build_lines_from_draft(draft: &JobDraft) -> Vec<CrontabLine> {
    let mut lines = Vec::new();
    let tags = draft.parsed_tags();

    // Add comment line if there are tags or a description
    if !tags.is_empty() || !draft.comment.is_empty() {
        let comment = if !tags.is_empty() {
            let tag_str = tags.join(", ");
            if draft.comment.is_empty() {
                format!("# [{}]", tag_str)
            } else {
                format!("# [{}] {}", tag_str, draft.comment)
            }
        } else {
            format!("# {}", draft.comment)
        };
        lines.push(CrontabLine::Comment(comment));
    }

    // Build the job line
    if draft.is_special {
        let raw = if draft.enabled {
            format!("{} {}", draft.expression, draft.command)
        } else {
            format!("# {} {}", draft.expression, draft.command)
        };
        lines.push(CrontabLine::Special {
            id: crate::model::job::JobId::next(),
            macro_name: draft.expression.clone(),
            command: draft.command.clone(),
            enabled: draft.enabled,
            raw,
        });
    } else {
        let raw = if draft.enabled {
            format!("{} {}", draft.expression, draft.command)
        } else {
            format!("# {} {}", draft.expression, draft.command)
        };
        lines.push(CrontabLine::Job {
            id: crate::model::job::JobId::next(),
            expression: draft.expression.clone(),
            command: draft.command.clone(),
            enabled: draft.enabled,
            raw,
        });
    }

    lines
}

/// Update an existing job line in-place from a draft.
/// Also updates or inserts the preceding comment line for description/tags.
/// Returns the new lines that should replace the range [comment_start..=job_index].
pub fn update_job_from_draft(
    lines: &[CrontabLine],
    job_index: usize,
    draft: &JobDraft,
) -> (usize, Vec<CrontabLine>) {
    // Only replace preceding comment if it's a managed metadata comment (has [tags] pattern)
    let comment_start = if job_index > 0 {
        if let Some(CrontabLine::Comment(text)) = lines.get(job_index - 1) {
            if is_managed_comment(text) {
                job_index - 1
            } else {
                job_index
            }
        } else {
            job_index
        }
    } else {
        job_index
    };

    let new_lines = build_lines_from_draft(draft);
    info!(
        old_start = comment_start,
        old_end = job_index,
        new_count = new_lines.len(),
        "Updated job from draft"
    );

    (comment_start, new_lines)
}

/// Remove a job and its managed metadata comment from the lines.
/// Only removes preceding comment if it matches the `# [tags]` pattern.
pub fn job_removal_range(lines: &[CrontabLine], job_index: usize) -> std::ops::RangeInclusive<usize> {
    let start = if job_index > 0 {
        if let Some(CrontabLine::Comment(text)) = lines.get(job_index - 1) {
            if is_managed_comment(text) {
                job_index - 1
            } else {
                job_index
            }
        } else {
            job_index
        }
    } else {
        job_index
    };

    start..=job_index
}

/// Check if a comment is a managed metadata comment (has `# [tags]` pattern).
/// Only managed comments are considered "owned" by a job and subject to edit/delete.
fn is_managed_comment(text: &str) -> bool {
    let content = text.trim_start_matches('#').trim();
    content.starts_with('[') && content.contains(']')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::parser::parse_crontab;

    #[test]
    fn test_serialize_roundtrip() {
        let raw = "\
SHELL=/bin/bash

# [backup] Nightly backup
0 2 * * * /usr/bin/backup.sh
# 0 0 1 * * /cleanup.sh

@reboot /usr/bin/startup.sh
";
        let lines = parse_crontab(raw);
        let output = serialize_crontab(&lines);
        assert_eq!(output, raw);
    }

    #[test]
    fn test_serialize_empty() {
        let lines: Vec<CrontabLine> = Vec::new();
        assert_eq!(serialize_crontab(&lines), "");
    }

    #[test]
    fn test_serialize_preserves_whitespace() {
        let raw = "  * * * * * /bin/test  \n";
        let lines = parse_crontab(raw);
        let output = serialize_crontab(&lines);
        // Parser trims and reconstructs, but raw is preserved
        assert_eq!(output, raw);
    }

    #[test]
    fn test_build_lines_from_draft_with_tags() {
        let draft = JobDraft {
            expression: "*/5 * * * *".into(),
            command: "/bin/test".into(),
            comment: "Test job".into(),
            tags_input: "test, dev".into(),
            enabled: true,
            is_special: false,
        };
        let lines = build_lines_from_draft(&draft);
        assert_eq!(lines.len(), 2);
        assert!(matches!(&lines[0], CrontabLine::Comment(c) if c == "# [test, dev] Test job"));
        assert!(matches!(&lines[1], CrontabLine::Job { expression, command, enabled: true, .. }
            if expression == "*/5 * * * *" && command == "/bin/test"));
    }

    #[test]
    fn test_build_lines_from_draft_no_metadata() {
        let draft = JobDraft {
            expression: "0 2 * * *".into(),
            command: "/bin/backup".into(),
            comment: "".into(),
            tags_input: "".into(),
            enabled: true,
            is_special: false,
        };
        let lines = build_lines_from_draft(&draft);
        assert_eq!(lines.len(), 1);
        assert!(matches!(&lines[0], CrontabLine::Job { .. }));
    }

    #[test]
    fn test_build_lines_from_draft_disabled() {
        let draft = JobDraft {
            expression: "0 2 * * *".into(),
            command: "/bin/backup".into(),
            comment: "".into(),
            tags_input: "".into(),
            enabled: false,
            is_special: false,
        };
        let lines = build_lines_from_draft(&draft);
        assert_eq!(lines.len(), 1);
        match &lines[0] {
            CrontabLine::Job { raw, enabled, .. } => {
                assert!(!enabled);
                assert!(raw.starts_with("# "));
            }
            other => panic!("Expected Job, got {:?}", other),
        }
    }

    #[test]
    fn test_build_lines_from_draft_special() {
        let draft = JobDraft {
            expression: "@reboot".into(),
            command: "/bin/startup".into(),
            comment: "".into(),
            tags_input: "".into(),
            enabled: true,
            is_special: true,
        };
        let lines = build_lines_from_draft(&draft);
        assert_eq!(lines.len(), 1);
        assert!(matches!(&lines[0], CrontabLine::Special { macro_name, enabled: true, .. }
            if macro_name == "@reboot"));
    }

    #[test]
    fn test_job_removal_range_with_managed_comment() {
        // Managed comment (with [tags]) should be removed alongside the job
        let raw = "# [test] My job\n0 2 * * * /bin/test\n";
        let lines = parse_crontab(raw);
        let range = job_removal_range(&lines, 1);
        assert_eq!(range, 0..=1);
    }

    #[test]
    fn test_job_removal_range_without_comment() {
        let raw = "0 2 * * * /bin/test\n";
        let lines = parse_crontab(raw);
        let range = job_removal_range(&lines, 0);
        assert_eq!(range, 0..=0);
    }

    #[test]
    fn test_job_removal_range_preserves_plain_comment() {
        // Plain comment (no [tags]) should NOT be removed with the job
        let raw = "# This is documentation\n0 2 * * * /bin/test\n";
        let lines = parse_crontab(raw);
        let range = job_removal_range(&lines, 1);
        // Should only remove the job, not the plain comment
        assert_eq!(range, 1..=1);
    }

    #[test]
    fn test_job_removal_range_removes_managed_comment() {
        // Managed comment with [tags] SHOULD be removed with the job
        let raw = "# [backup] My job\n0 2 * * * /bin/test\n";
        let lines = parse_crontab(raw);
        let range = job_removal_range(&lines, 1);
        assert_eq!(range, 0..=1);
    }
}
