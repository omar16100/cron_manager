use crate::core::error::AppError;
use crate::model::schedule::ScheduleFields;
use chrono::{DateTime, Local};
use croner::Cron;
use tracing::debug;

/// Known special macros that are valid cron but may not be supported by croner.
const VALID_MACROS: &[&str] = &[
    "@reboot", "@yearly", "@annually", "@monthly", "@weekly", "@daily", "@midnight", "@hourly",
];

/// Validate a cron expression string.
/// Handles special macros (@reboot, @midnight) that croner may not support.
pub fn validate(expression: &str) -> Result<(), AppError> {
    // Special macros are always valid
    if VALID_MACROS.iter().any(|m| m.eq_ignore_ascii_case(expression)) {
        return Ok(());
    }

    Cron::new(expression).parse().map_err(|e| AppError::InvalidExpression {
        expression: expression.to_string(),
        reason: e.to_string(),
    })?;
    Ok(())
}

/// Get human-readable description of a cron expression.
pub fn describe(expression: &str) -> Result<String, AppError> {
    // Handle special macros
    match expression {
        "@reboot" => return Ok("Run once at startup".into()),
        "@yearly" | "@annually" => return Ok("Once a year (midnight, Jan 1)".into()),
        "@monthly" => return Ok("Once a month (midnight, 1st)".into()),
        "@weekly" => return Ok("Once a week (midnight, Sunday)".into()),
        "@daily" | "@midnight" => return Ok("Once a day (midnight)".into()),
        "@hourly" => return Ok("Once an hour (at minute 0)".into()),
        _ => {}
    }

    // Parse fields and build description
    let parts: Vec<&str> = expression.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(AppError::InvalidExpression {
            expression: expression.to_string(),
            reason: "Expected 5 fields".into(),
        });
    }

    // Validate first
    validate(expression)?;

    let mut desc_parts = Vec::new();

    // Minute
    match parts[0] {
        "*" => {}
        s if s.starts_with("*/") => {
            let n = &s[2..];
            desc_parts.push(format!("Every {} minutes", n));
        }
        s => desc_parts.push(format!("At minute {}", s)),
    }

    // Hour
    match parts[1] {
        "*" => {}
        s if s.starts_with("*/") => {
            let n = &s[2..];
            desc_parts.push(format!("every {} hours", n));
        }
        s => desc_parts.push(format!("at hour {}", s)),
    }

    // Day of month
    match parts[2] {
        "*" => {}
        s => desc_parts.push(format!("on day {} of month", s)),
    }

    // Month
    match parts[3] {
        "*" => {}
        s => desc_parts.push(format!("in month {}", s)),
    }

    // Day of week
    match parts[4] {
        "*" => {}
        "1-5" | "MON-FRI" => desc_parts.push("weekdays only".into()),
        "0,6" | "SAT,SUN" => desc_parts.push("weekends only".into()),
        s => desc_parts.push(format!("on weekday {}", s)),
    }

    if desc_parts.is_empty() {
        Ok("Every minute".into())
    } else {
        Ok(desc_parts.join(", "))
    }
}

/// Calculate the next N execution times from now.
pub fn next_occurrences(
    expression: &str,
    count: usize,
) -> Result<Vec<DateTime<Local>>, AppError> {
    // Special macros don't have predictable schedules (except @reboot)
    match expression {
        "@reboot" => {
            return Ok(vec![]);
        }
        "@yearly" | "@annually" => {
            return next_occurrences("0 0 1 1 *", count);
        }
        "@monthly" => {
            return next_occurrences("0 0 1 * *", count);
        }
        "@weekly" => {
            return next_occurrences("0 0 * * 0", count);
        }
        "@daily" | "@midnight" => {
            return next_occurrences("0 0 * * *", count);
        }
        "@hourly" => {
            return next_occurrences("0 * * * *", count);
        }
        _ => {}
    }

    let cron = Cron::new(expression).parse().map_err(|e| AppError::InvalidExpression {
        expression: expression.to_string(),
        reason: e.to_string(),
    })?;

    let now = Local::now();
    let times: Vec<DateTime<Local>> = cron
        .iter_from(now)
        .take(count)
        .filter_map(|t| {
            t.with_timezone(&Local).into()
        })
        .collect();

    debug!(
        expression = expression,
        count = times.len(),
        "Calculated next occurrences"
    );
    Ok(times)
}

/// Build a cron expression from ScheduleFields.
pub fn build_from_fields(fields: &ScheduleFields) -> String {
    fields.to_expression()
}

/// Parse a cron expression into ScheduleFields for the visual builder.
/// Returns None if the expression is not representable in the builder.
pub fn decompose(expression: &str) -> Option<ScheduleFields> {
    ScheduleFields::from_expression(expression)
}

/// Check if an expression can be represented in the visual builder.
pub fn is_builder_compatible(expression: &str) -> bool {
    decompose(expression).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid() {
        assert!(validate("*/5 * * * *").is_ok());
        assert!(validate("0 2 * * 1-5").is_ok());
        assert!(validate("0 0 1 1 *").is_ok());
    }

    #[test]
    fn test_validate_invalid() {
        assert!(validate("not a cron").is_err());
        assert!(validate("*/5 * *").is_err());
        assert!(validate("").is_err());
    }

    #[test]
    fn test_describe_every_minute() {
        let desc = describe("* * * * *").unwrap();
        assert_eq!(desc, "Every minute");
    }

    #[test]
    fn test_describe_every_5_minutes() {
        let desc = describe("*/5 * * * *").unwrap();
        assert!(desc.contains("5 minutes"));
    }

    #[test]
    fn test_describe_special_macros() {
        assert_eq!(describe("@reboot").unwrap(), "Run once at startup");
        assert_eq!(describe("@daily").unwrap(), "Once a day (midnight)");
        assert_eq!(describe("@hourly").unwrap(), "Once an hour (at minute 0)");
    }

    #[test]
    fn test_next_occurrences_returns_correct_count() {
        let times = next_occurrences("* * * * *", 5).unwrap();
        assert_eq!(times.len(), 5);
    }

    #[test]
    fn test_next_occurrences_reboot() {
        let times = next_occurrences("@reboot", 5).unwrap();
        assert!(times.is_empty());
    }

    #[test]
    fn test_validate_reboot() {
        // Codex bug: @reboot was rejected by croner
        assert!(validate("@reboot").is_ok());
        assert!(validate("@midnight").is_ok());
        assert!(validate("@daily").is_ok());
        assert!(validate("@weekly").is_ok());
    }

    #[test]
    fn test_is_builder_compatible() {
        assert!(is_builder_compatible("*/5 * * * *"));
        assert!(is_builder_compatible("0 2 * * *"));
        assert!(!is_builder_compatible("0 2 * * MON"));
        assert!(!is_builder_compatible("@daily"));
    }

    #[test]
    fn test_decompose_roundtrip() {
        let expr = "*/5 * 1-15 * *";
        let fields = decompose(expr).unwrap();
        assert_eq!(build_from_fields(&fields), expr);
    }
}
