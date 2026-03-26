use crate::core::backend::CrontabBackend;
use crate::core::error::AppError;
use sha2::{Digest, Sha256};
use tracing::{info, warn};

/// Tracks the last-known state of the crontab for conflict detection.
#[derive(Debug, Clone)]
pub struct CrontabState {
    pub content: String,
    pub checksum: [u8; 32],
}

impl CrontabState {
    /// Create a new state from content.
    pub fn new(content: &str) -> Self {
        Self {
            content: content.to_string(),
            checksum: compute_checksum(content),
        }
    }

    /// Check if the live crontab matches our last-known state.
    /// Returns Ok(None) if no conflict, Ok(Some(current_content)) if conflict detected.
    pub fn detect_conflict(
        &self,
        backend: &dyn CrontabBackend,
    ) -> Result<Option<String>, AppError> {
        let current = backend.read()?;
        let current_checksum = compute_checksum(&current);

        if current_checksum == self.checksum {
            info!("No conflict detected");
            Ok(None)
        } else {
            warn!("Conflict detected: crontab was modified externally");
            Ok(Some(current))
        }
    }

    /// Update the stored state after a successful write.
    pub fn update(&mut self, content: &str) {
        self.content = content.to_string();
        self.checksum = compute_checksum(content);
    }
}

/// Compute SHA-256 checksum of content.
fn compute_checksum(content: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::backend::MemoryCrontab;

    #[test]
    fn test_no_conflict_when_unchanged() {
        let content = "* * * * * /bin/test\n";
        let backend = MemoryCrontab::new(content);
        let state = CrontabState::new(content);
        assert!(state.detect_conflict(&backend).unwrap().is_none());
    }

    #[test]
    fn test_conflict_when_changed() {
        let original = "* * * * * /bin/test\n";
        let backend = MemoryCrontab::new(original);
        let state = CrontabState::new(original);

        // Simulate external modification
        backend.write("*/5 * * * * /bin/new\n").unwrap();

        let result = state.detect_conflict(&backend).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "*/5 * * * * /bin/new\n");
    }

    #[test]
    fn test_update_clears_conflict() {
        let original = "* * * * * /bin/test\n";
        let new_content = "*/5 * * * * /bin/new\n";
        let backend = MemoryCrontab::new(new_content);

        let mut state = CrontabState::new(original);
        assert!(state.detect_conflict(&backend).unwrap().is_some());

        state.update(new_content);
        assert!(state.detect_conflict(&backend).unwrap().is_none());
    }

    #[test]
    fn test_checksum_deterministic() {
        let content = "test content";
        let a = compute_checksum(content);
        let b = compute_checksum(content);
        assert_eq!(a, b);
    }

    #[test]
    fn test_checksum_differs_for_different_content() {
        let a = compute_checksum("hello");
        let b = compute_checksum("world");
        assert_ne!(a, b);
    }
}
