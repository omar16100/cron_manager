use std::fmt;

/// Represents a single field value in a cron expression for the visual builder.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// `*` — matches every value
    Any,
    /// `1,5,15` — specific values
    Specific(Vec<u32>),
    /// `1-5` — inclusive range
    Range(u32, u32),
    /// `*/5` — every N from start
    Step(u32),
    /// `1-30/5` — every N within a range
    RangeWithStep(u32, u32, u32),
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => write!(f, "*"),
            Self::Specific(vals) => {
                let s: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
                write!(f, "{}", s.join(","))
            }
            Self::Range(start, end) => write!(f, "{}-{}", start, end),
            Self::Step(step) => write!(f, "*/{}", step),
            Self::RangeWithStep(start, end, step) => {
                write!(f, "{}-{}/{}", start, end, step)
            }
        }
    }
}

impl FieldValue {
    /// Parse a single cron field string into a FieldValue.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s == "*" {
            return Some(Self::Any);
        }
        // */N
        if let Some(step) = s.strip_prefix("*/") {
            return step.parse::<u32>().ok().map(Self::Step);
        }
        // Range with step: N-M/S
        if s.contains('-') && s.contains('/') {
            let parts: Vec<&str> = s.splitn(2, '/').collect();
            if parts.len() == 2 {
                let range_parts: Vec<&str> = parts[0].splitn(2, '-').collect();
                if range_parts.len() == 2 {
                    if let (Ok(start), Ok(end), Ok(step)) = (
                        range_parts[0].parse::<u32>(),
                        range_parts[1].parse::<u32>(),
                        parts[1].parse::<u32>(),
                    ) {
                        return Some(Self::RangeWithStep(start, end, step));
                    }
                }
            }
            return None;
        }
        // Range: N-M
        if s.contains('-') {
            let parts: Vec<&str> = s.splitn(2, '-').collect();
            if parts.len() == 2 {
                if let (Ok(start), Ok(end)) =
                    (parts[0].parse::<u32>(), parts[1].parse::<u32>())
                {
                    return Some(Self::Range(start, end));
                }
            }
            return None;
        }
        // Specific values: N,M,O
        if s.contains(',') {
            let vals: Result<Vec<u32>, _> = s.split(',').map(|v| v.trim().parse::<u32>()).collect();
            return vals.ok().map(Self::Specific);
        }
        // Single value
        s.parse::<u32>().ok().map(|v| Self::Specific(vec![v]))
    }
}

/// The five standard cron fields for the visual builder.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleFields {
    pub minute: FieldValue,
    pub hour: FieldValue,
    pub day_of_month: FieldValue,
    pub month: FieldValue,
    pub day_of_week: FieldValue,
}

impl Default for ScheduleFields {
    fn default() -> Self {
        Self {
            minute: FieldValue::Any,
            hour: FieldValue::Any,
            day_of_month: FieldValue::Any,
            month: FieldValue::Any,
            day_of_week: FieldValue::Any,
        }
    }
}

impl ScheduleFields {
    /// Convert to a cron expression string: `*/5 * * * *`
    pub fn to_expression(&self) -> String {
        format!(
            "{} {} {} {} {}",
            self.minute, self.hour, self.day_of_month, self.month, self.day_of_week
        )
    }

    /// Parse a 5-field cron expression into ScheduleFields.
    /// Returns None if the expression uses features not representable in the builder
    /// (e.g., names like MON, L, W, #).
    pub fn from_expression(expr: &str) -> Option<Self> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return None;
        }

        // Reject expressions with non-numeric features (names, L, W, #)
        for part in &parts {
            let has_alpha = part.chars().any(|c| c.is_alphabetic());
            if has_alpha {
                return None;
            }
        }

        Some(Self {
            minute: FieldValue::parse(parts[0])?,
            hour: FieldValue::parse(parts[1])?,
            day_of_month: FieldValue::parse(parts[2])?,
            month: FieldValue::parse(parts[3])?,
            day_of_week: FieldValue::parse(parts[4])?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_value_display() {
        assert_eq!(FieldValue::Any.to_string(), "*");
        assert_eq!(FieldValue::Step(5).to_string(), "*/5");
        assert_eq!(FieldValue::Range(1, 5).to_string(), "1-5");
        assert_eq!(FieldValue::Specific(vec![1, 5, 15]).to_string(), "1,5,15");
        assert_eq!(FieldValue::RangeWithStep(1, 30, 5).to_string(), "1-30/5");
    }

    #[test]
    fn test_field_value_parse() {
        assert_eq!(FieldValue::parse("*"), Some(FieldValue::Any));
        assert_eq!(FieldValue::parse("*/5"), Some(FieldValue::Step(5)));
        assert_eq!(FieldValue::parse("1-5"), Some(FieldValue::Range(1, 5)));
        assert_eq!(
            FieldValue::parse("1,5,15"),
            Some(FieldValue::Specific(vec![1, 5, 15]))
        );
        assert_eq!(
            FieldValue::parse("1-30/5"),
            Some(FieldValue::RangeWithStep(1, 30, 5))
        );
        assert_eq!(FieldValue::parse("30"), Some(FieldValue::Specific(vec![30])));
    }

    #[test]
    fn test_schedule_fields_roundtrip() {
        let fields = ScheduleFields {
            minute: FieldValue::Step(5),
            hour: FieldValue::Any,
            day_of_month: FieldValue::Range(1, 15),
            month: FieldValue::Specific(vec![1, 6, 12]),
            day_of_week: FieldValue::Any,
        };
        let expr = fields.to_expression();
        assert_eq!(expr, "*/5 * 1-15 1,6,12 *");
        let parsed = ScheduleFields::from_expression(&expr).unwrap();
        assert_eq!(parsed, fields);
    }

    #[test]
    fn test_schedule_fields_rejects_names() {
        assert!(ScheduleFields::from_expression("0 0 * * MON").is_none());
        assert!(ScheduleFields::from_expression("@daily").is_none());
    }

    #[test]
    fn test_schedule_fields_rejects_wrong_count() {
        assert!(ScheduleFields::from_expression("* * *").is_none());
        assert!(ScheduleFields::from_expression("* * * * * *").is_none());
    }
}
