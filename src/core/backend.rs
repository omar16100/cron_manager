use crate::core::error::AppError;
use std::process::Command;
use tracing::{error, info};

/// Trait for reading/writing the system crontab.
/// Enables testing with a mock implementation.
pub trait CrontabBackend: Send + Sync {
    fn read(&self) -> Result<String, AppError>;
    fn write(&self, content: &str) -> Result<(), AppError>;
}

/// Real implementation that wraps the system `crontab` command.
pub struct SystemCrontab;

impl CrontabBackend for SystemCrontab {
    fn read(&self) -> Result<String, AppError> {
        info!("Reading system crontab via `crontab -l`");
        let output = Command::new("crontab")
            .arg("-l")
            .output()
            .map_err(|e| AppError::SystemCommand {
                cmd: "crontab -l".into(),
                source: e,
            })?;

        if output.status.success() {
            let content = String::from_utf8_lossy(&output.stdout).into_owned();
            info!(lines = content.lines().count(), "Crontab read successfully");
            Ok(content)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("no crontab for") {
                info!("No existing crontab found, returning empty");
                Ok(String::new())
            } else {
                error!(stderr = %stderr, "crontab -l failed");
                Err(AppError::CrontabRead(stderr.into_owned()))
            }
        }
    }

    fn write(&self, content: &str) -> Result<(), AppError> {
        info!(bytes = content.len(), "Writing crontab via `crontab -`");

        let mut child = Command::new("crontab")
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| AppError::SystemCommand {
                cmd: "crontab -".into(),
                source: e,
            })?;

        use std::io::Write;
        if let Some(ref mut stdin) = child.stdin {
            stdin.write_all(content.as_bytes()).map_err(|e| {
                AppError::CrontabWrite(format!("Failed to write to crontab stdin: {}", e))
            })?;
        }

        let output = child.wait_with_output().map_err(|e| {
            AppError::CrontabWrite(format!("Failed to wait for crontab: {}", e))
        })?;

        if output.status.success() {
            info!("Crontab written successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!(stderr = %stderr, "crontab write failed");
            Err(AppError::CrontabWrite(stderr.into_owned()))
        }
    }
}

/// In-memory implementation for testing.
#[cfg(test)]
pub struct MemoryCrontab {
    content: std::sync::Mutex<String>,
}

#[cfg(test)]
impl MemoryCrontab {
    pub fn new(initial: &str) -> Self {
        Self {
            content: std::sync::Mutex::new(initial.to_string()),
        }
    }

    pub fn get_content(&self) -> String {
        self.content.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl CrontabBackend for MemoryCrontab {
    fn read(&self) -> Result<String, AppError> {
        Ok(self.content.lock().unwrap().clone())
    }

    fn write(&self, content: &str) -> Result<(), AppError> {
        *self.content.lock().unwrap() = content.to_string();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_backend_read_write() {
        let backend = MemoryCrontab::new("* * * * * /bin/test\n");
        assert_eq!(backend.read().unwrap(), "* * * * * /bin/test\n");

        backend.write("*/5 * * * * /bin/new\n").unwrap();
        assert_eq!(backend.read().unwrap(), "*/5 * * * * /bin/new\n");
    }

    #[test]
    fn test_memory_backend_empty() {
        let backend = MemoryCrontab::new("");
        assert_eq!(backend.read().unwrap(), "");
    }
}
