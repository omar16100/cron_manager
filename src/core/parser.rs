use crate::model::crontab::CrontabLine;
use crate::model::job::{CronJob, JobId};
use tracing::{debug, info, warn};

/// Known cron macros that replace the 5-field expression.
const SPECIAL_MACROS: &[&str] = &[
    "@reboot", "@yearly", "@annually", "@monthly", "@weekly", "@daily", "@midnight", "@hourly",
];

/// Parse raw crontab text into a structured `Vec<CrontabLine>`.
/// Preserves every line for lossless round-trips.
pub fn parse_crontab(raw: &str) -> Vec<CrontabLine> {
    let mut lines = Vec::new();

    for (line_num, raw_line) in raw.lines().enumerate() {
        let trimmed = raw_line.trim();

        if trimmed.is_empty() {
            lines.push(CrontabLine::Blank);
            continue;
        }

        // Try to parse as disabled job (commented-out cron line)
        if let Some(disabled) = try_parse_disabled_job(trimmed, raw_line) {
            lines.push(disabled);
            continue;
        }

        // Comment line
        if trimmed.starts_with('#') {
            lines.push(CrontabLine::Comment(raw_line.to_string()));
            continue;
        }

        // Environment variable: KEY=VALUE (no spaces in key, before '=')
        if let Some(env) = try_parse_env_var(trimmed, raw_line) {
            lines.push(env);
            continue;
        }

        // Special macro (@reboot, @daily, etc.)
        if let Some(special) = try_parse_special(trimmed, raw_line) {
            lines.push(special);
            continue;
        }

        // Standard 5-field cron job
        if let Some(job) = try_parse_job(trimmed, raw_line) {
            lines.push(job);
            continue;
        }

        // Unknown line — preserve as comment to avoid data loss
        warn!(
            line = line_num + 1,
            content = trimmed,
            "Unrecognized crontab line, preserving as comment"
        );
        lines.push(CrontabLine::Comment(raw_line.to_string()));
    }

    info!(
        total_lines = lines.len(),
        jobs = lines.iter().filter(|l| l.is_job()).count(),
        "Parsed crontab"
    );
    lines
}

/// Try to parse a commented-out cron job: `# */5 * * * * /cmd` or `# @daily /cmd`
fn try_parse_disabled_job(trimmed: &str, raw_line: &str) -> Option<CrontabLine> {
    if !trimmed.starts_with('#') {
        return None;
    }

    let after_hash = trimmed[1..].trim_start();

    // Check for disabled special macro
    for mac in SPECIAL_MACROS {
        if after_hash.starts_with(mac) {
            let rest = after_hash[mac.len()..].trim_start();
            if !rest.is_empty() {
                debug!(macro_name = mac, "Parsed disabled special job");
                return Some(CrontabLine::Special {
                    id: JobId::next(),
                    macro_name: mac.to_string(),
                    command: rest.to_string(),
                    enabled: false,
                    raw: raw_line.to_string(),
                });
            }
        }
    }

    // Check for disabled standard job: # field field field field field command
    // Use split_whitespace to handle multiple spaces/tabs robustly
    let words: Vec<&str> = after_hash.split_whitespace().collect();
    if words.len() >= 6 && looks_like_cron_fields(&words[..5]) {
        let expression = words[..5].join(" ");
        let command = words[5..].join(" ");
        if !command.is_empty() {
            debug!(expression = %expression, "Parsed disabled job");
            return Some(CrontabLine::Job {
                id: JobId::next(),
                expression,
                command,
                enabled: false,
                raw: raw_line.to_string(),
            });
        }
    }

    None
}

/// Try to parse an environment variable line: `KEY=VALUE`
fn try_parse_env_var(trimmed: &str, raw_line: &str) -> Option<CrontabLine> {
    // Must have '=' and key must be a valid env var name (alphanumeric + underscore, starts with letter/underscore)
    let eq_pos = trimmed.find('=')?;
    let key = &trimmed[..eq_pos];

    // Validate key: non-empty, starts with letter/underscore, rest alphanumeric/underscore
    if key.is_empty() {
        return None;
    }
    let first = key.chars().next()?;
    if !first.is_alphabetic() && first != '_' {
        return None;
    }
    if !key.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }

    let value = trimmed[eq_pos + 1..].to_string();
    debug!(key = key, "Parsed env var");
    Some(CrontabLine::EnvVar {
        key: key.to_string(),
        value,
        raw: raw_line.to_string(),
    })
}

/// Try to parse a special macro line: `@reboot /usr/bin/cmd`
fn try_parse_special(trimmed: &str, raw_line: &str) -> Option<CrontabLine> {
    for mac in SPECIAL_MACROS {
        if trimmed.starts_with(mac) {
            let rest = trimmed[mac.len()..].trim_start();
            if !rest.is_empty() {
                debug!(macro_name = mac, "Parsed special job");
                return Some(CrontabLine::Special {
                    id: JobId::next(),
                    macro_name: mac.to_string(),
                    command: rest.to_string(),
                    enabled: true,
                    raw: raw_line.to_string(),
                });
            }
        }
    }
    None
}

/// Try to parse a standard 5-field cron job: `*/5 * * * * /usr/bin/cmd args`
/// Uses split_whitespace to handle multiple spaces/tabs robustly.
/// The command portion preserves original spacing by finding its start position.
fn try_parse_job(trimmed: &str, raw_line: &str) -> Option<CrontabLine> {
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() < 6 {
        return None;
    }

    if !looks_like_cron_fields(&words[..5]) {
        return None;
    }

    let expression = words[..5].join(" ");

    // Find command start: skip past 5 whitespace-separated fields in the original string
    let mut pos = 0;
    for _ in 0..5 {
        // skip whitespace
        while pos < trimmed.len() && trimmed.as_bytes()[pos].is_ascii_whitespace() {
            pos += 1;
        }
        // skip field
        while pos < trimmed.len() && !trimmed.as_bytes()[pos].is_ascii_whitespace() {
            pos += 1;
        }
    }
    // skip whitespace between last field and command
    while pos < trimmed.len() && trimmed.as_bytes()[pos].is_ascii_whitespace() {
        pos += 1;
    }
    let command = trimmed[pos..].to_string();

    if command.is_empty() {
        return None;
    }

    debug!(expression = %expression, "Parsed job");
    Some(CrontabLine::Job {
        id: JobId::next(),
        expression,
        command,
        enabled: true,
        raw: raw_line.to_string(),
    })
}

/// Heuristic: does this look like 5 cron fields?
/// Each field should contain only digits, *, -, /, and commas.
/// Also allows day/month names (MON, JAN, etc.)
fn looks_like_cron_fields(fields: &[&str]) -> bool {
    if fields.len() != 5 {
        return false;
    }
    for field in fields {
        if field.is_empty() {
            return false;
        }
        let valid = field.chars().all(|c| {
            c.is_alphanumeric() || c == '*' || c == '-' || c == '/' || c == ','
        });
        if !valid {
            return false;
        }
    }
    true
}

/// Extract derived CronJob views from parsed crontab lines.
/// Looks at preceding Comment lines for description and tags.
pub fn extract_jobs(lines: &[CrontabLine]) -> Vec<CronJob> {
    let mut jobs = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        match line {
            CrontabLine::Job {
                id,
                expression,
                command,
                enabled,
                ..
            } => {
                let (description, tags) = extract_metadata(lines, i);
                jobs.push(CronJob {
                    id: *id,
                    expression: expression.clone(),
                    command: command.clone(),
                    enabled: *enabled,
                    description,
                    tags,
                    is_special: false,
                    line_index: i,
                });
            }
            CrontabLine::Special {
                id,
                macro_name,
                command,
                enabled,
                ..
            } => {
                let (description, tags) = extract_metadata(lines, i);
                jobs.push(CronJob {
                    id: *id,
                    expression: macro_name.clone(),
                    command: command.clone(),
                    enabled: *enabled,
                    description,
                    tags,
                    is_special: true,
                    line_index: i,
                });
            }
            _ => {}
        }
    }

    jobs
}

/// Look at the preceding Comment line(s) for description and tags.
/// Tag convention: `# [tag1, tag2] Description text`
fn extract_metadata(lines: &[CrontabLine], job_index: usize) -> (Option<String>, Vec<String>) {
    if job_index == 0 {
        return (None, Vec::new());
    }

    // Check the line immediately before the job
    if let Some(CrontabLine::Comment(text)) = lines.get(job_index - 1) {
        let content = text.trim_start_matches('#').trim();
        if content.is_empty() {
            return (None, Vec::new());
        }

        // Parse tags: [tag1, tag2] rest of description
        if content.starts_with('[') {
            if let Some(bracket_end) = content.find(']') {
                let tag_str = &content[1..bracket_end];
                let tags: Vec<String> = tag_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let description = content[bracket_end + 1..].trim();
                let desc = if description.is_empty() {
                    None
                } else {
                    Some(description.to_string())
                };
                return (desc, tags);
            }
        }

        // No tags, just description
        return (Some(content.to_string()), Vec::new());
    }

    (None, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_standard_job() {
        let lines = parse_crontab("*/5 * * * * /usr/bin/backup.sh\n");
        assert_eq!(lines.len(), 1);
        match &lines[0] {
            CrontabLine::Job {
                expression,
                command,
                enabled,
                ..
            } => {
                assert_eq!(expression, "*/5 * * * *");
                assert_eq!(command, "/usr/bin/backup.sh");
                assert!(enabled);
            }
            other => panic!("Expected Job, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_disabled_job() {
        let lines = parse_crontab("# 0 2 * * * /usr/bin/cleanup.sh\n");
        assert_eq!(lines.len(), 1);
        match &lines[0] {
            CrontabLine::Job {
                expression,
                command,
                enabled,
                ..
            } => {
                assert_eq!(expression, "0 2 * * *");
                assert_eq!(command, "/usr/bin/cleanup.sh");
                assert!(!enabled);
            }
            other => panic!("Expected disabled Job, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_special_macro() {
        let lines = parse_crontab("@reboot /usr/bin/startup.sh\n");
        assert_eq!(lines.len(), 1);
        match &lines[0] {
            CrontabLine::Special {
                macro_name,
                command,
                enabled,
                ..
            } => {
                assert_eq!(macro_name, "@reboot");
                assert_eq!(command, "/usr/bin/startup.sh");
                assert!(enabled);
            }
            other => panic!("Expected Special, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_disabled_special() {
        let lines = parse_crontab("# @daily /usr/bin/daily.sh\n");
        assert_eq!(lines.len(), 1);
        match &lines[0] {
            CrontabLine::Special {
                macro_name,
                enabled,
                ..
            } => {
                assert_eq!(macro_name, "@daily");
                assert!(!enabled);
            }
            other => panic!("Expected disabled Special, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_env_var() {
        let lines = parse_crontab("SHELL=/bin/bash\n");
        assert_eq!(lines.len(), 1);
        match &lines[0] {
            CrontabLine::EnvVar { key, value, .. } => {
                assert_eq!(key, "SHELL");
                assert_eq!(value, "/bin/bash");
            }
            other => panic!("Expected EnvVar, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_comment() {
        let lines = parse_crontab("# This is a comment\n");
        assert_eq!(lines.len(), 1);
        assert!(matches!(&lines[0], CrontabLine::Comment(_)));
    }

    #[test]
    fn test_parse_blank() {
        let lines = parse_crontab("\n");
        assert_eq!(lines.len(), 1);
        assert!(matches!(&lines[0], CrontabLine::Blank));
    }

    #[test]
    fn test_parse_complex_crontab() {
        let raw = "\
SHELL=/bin/bash
MAILTO=admin@example.com

# [backup, daily] Run nightly backup
0 2 * * * /usr/bin/backup.sh --full

# Disabled cleanup job
# 0 0 1 * * /cleanup.sh

@reboot /usr/bin/startup.sh
";
        let lines = parse_crontab(raw);
        // SHELL, MAILTO, blank, comment, job, blank, comment, disabled job, blank, special
        assert_eq!(lines.len(), 10);

        // Verify env vars
        assert!(matches!(&lines[0], CrontabLine::EnvVar { key, .. } if key == "SHELL"));
        assert!(matches!(&lines[1], CrontabLine::EnvVar { key, .. } if key == "MAILTO"));

        // Verify job with tags
        let jobs = extract_jobs(&lines);
        assert_eq!(jobs.len(), 3);

        assert_eq!(jobs[0].expression, "0 2 * * *");
        assert_eq!(jobs[0].tags, vec!["backup", "daily"]);
        assert_eq!(
            jobs[0].description,
            Some("Run nightly backup".to_string())
        );
        assert!(jobs[0].enabled);

        assert_eq!(jobs[1].expression, "0 0 1 * *");
        assert!(!jobs[1].enabled);

        assert_eq!(jobs[2].expression, "@reboot");
        assert!(jobs[2].is_special);
    }

    #[test]
    fn test_parse_job_with_percent() {
        let lines = parse_crontab("* * * * * /bin/echo hello%world\n");
        assert_eq!(lines.len(), 1);
        match &lines[0] {
            CrontabLine::Job { command, .. } => {
                assert_eq!(command, "/bin/echo hello%world");
            }
            other => panic!("Expected Job, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_job_with_multiple_spaces() {
        // Codex bug: splitn with whitespace predicate doesn't collapse spaces
        let lines = parse_crontab("0  2  *  *  *  /usr/bin/backup.sh\n");
        assert_eq!(lines.len(), 1);
        match &lines[0] {
            CrontabLine::Job {
                expression,
                command,
                enabled,
                ..
            } => {
                assert_eq!(expression, "0 2 * * *");
                assert_eq!(command, "/usr/bin/backup.sh");
                assert!(enabled);
            }
            other => panic!("Expected Job with multi-spaces, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_job_with_tabs() {
        let lines = parse_crontab("0\t2\t*\t*\t*\t/usr/bin/backup.sh --full\n");
        assert_eq!(lines.len(), 1);
        match &lines[0] {
            CrontabLine::Job {
                expression,
                command,
                ..
            } => {
                assert_eq!(expression, "0 2 * * *");
                assert_eq!(command, "/usr/bin/backup.sh --full");
            }
            other => panic!("Expected Job with tabs, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_disabled_job_with_multiple_spaces() {
        let lines = parse_crontab("#  0  2  *  *  *  /usr/bin/cleanup.sh\n");
        assert_eq!(lines.len(), 1);
        match &lines[0] {
            CrontabLine::Job {
                expression,
                command,
                enabled,
                ..
            } => {
                assert_eq!(expression, "0 2 * * *");
                assert_eq!(command, "/usr/bin/cleanup.sh");
                assert!(!enabled);
            }
            other => panic!("Expected disabled Job with multi-spaces, got {:?}", other),
        }
    }

    #[test]
    fn test_extract_metadata_with_tags() {
        let raw = "# [test, dev] My test job\n*/5 * * * * /bin/test\n";
        let lines = parse_crontab(raw);
        let jobs = extract_jobs(&lines);
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].tags, vec!["test", "dev"]);
        assert_eq!(jobs[0].description, Some("My test job".to_string()));
    }

    #[test]
    fn test_extract_metadata_tags_only() {
        let raw = "# [backup]\n0 2 * * * /backup.sh\n";
        let lines = parse_crontab(raw);
        let jobs = extract_jobs(&lines);
        assert_eq!(jobs[0].tags, vec!["backup"]);
        assert_eq!(jobs[0].description, None);
    }

    #[test]
    fn test_extract_metadata_description_only() {
        let raw = "# Run daily backup\n0 2 * * * /backup.sh\n";
        let lines = parse_crontab(raw);
        let jobs = extract_jobs(&lines);
        assert!(jobs[0].tags.is_empty());
        assert_eq!(
            jobs[0].description,
            Some("Run daily backup".to_string())
        );
    }

    #[test]
    fn test_env_var_not_confused_with_job() {
        // PATH= shouldn't be parsed as a job
        let lines = parse_crontab("PATH=/usr/bin:/bin\n");
        assert_eq!(lines.len(), 1);
        assert!(matches!(&lines[0], CrontabLine::EnvVar { key, .. } if key == "PATH"));
    }
}
