/// Application error type with user-facing messages.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Failed to execute '{cmd}': {source}")]
    SystemCommand {
        cmd: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Could not read crontab: {0}")]
    CrontabRead(String),

    #[error("Could not write crontab: {0}")]
    CrontabWrite(String),

    #[error("Line {line_number}: {reason}")]
    ParseError {
        line_number: usize,
        content: String,
        reason: String,
    },

    #[error("Invalid cron expression '{expression}': {reason}")]
    InvalidExpression { expression: String, reason: String },

    #[error("Command cannot be empty")]
    EmptyCommand,

    #[error("Backup failed: {0}")]
    BackupFailed(String),

    #[error("Crontab was modified externally")]
    ConflictDetected {
        expected_checksum: String,
        actual_checksum: String,
    },

    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl AppError {
    /// User-facing message suitable for display in the GUI.
    /// Avoids exposing internal details or potentially secret command contents.
    pub fn user_message(&self) -> String {
        match self {
            Self::SystemCommand { cmd, .. } => {
                format!("Failed to execute system command: {}", cmd)
            }
            Self::CrontabRead(msg) => format!("Could not read your crontab: {}", msg),
            Self::CrontabWrite(msg) => format!("Could not save crontab: {}", msg),
            Self::ParseError {
                line_number,
                reason,
                ..
            } => format!("Parse error on line {}: {}", line_number, reason),
            Self::InvalidExpression { reason, .. } => {
                format!("Invalid cron expression: {}", reason)
            }
            Self::EmptyCommand => "Command cannot be empty".into(),
            Self::BackupFailed(msg) => format!("Backup failed: {}", msg),
            Self::ConflictDetected { .. } => {
                "Crontab was modified externally. Please reload and try again.".into()
            }
            Self::Io(e) => format!("I/O error: {}", e),
        }
    }
}

