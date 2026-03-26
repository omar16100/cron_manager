use crate::core::error::AppError;
use std::fs;
use std::path::PathBuf;
use tracing::{error, info, warn};

/// Maximum number of backups to retain.
const MAX_BACKUPS: usize = 20;

/// Manages crontab backup files with retention policy.
pub struct BackupManager {
    backup_dir: PathBuf,
}

impl BackupManager {
    /// Create a new BackupManager. Creates the backup directory if needed.
    pub fn new() -> Result<Self, AppError> {
        let backup_dir = Self::default_backup_dir()?;
        fs::create_dir_all(&backup_dir).map_err(|e| {
            AppError::BackupFailed(format!(
                "Cannot create backup directory {}: {}",
                backup_dir.display(),
                e
            ))
        })?;

        // Set directory permissions to 0700 (owner only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o700);
            let _ = fs::set_permissions(&backup_dir, perms);
        }

        Ok(Self { backup_dir })
    }

    #[cfg(test)]
    pub fn with_dir(dir: PathBuf) -> Self {
        Self { backup_dir: dir }
    }

    /// Default backup directory: ~/.local/share/cron_manager/backups/
    fn default_backup_dir() -> Result<PathBuf, AppError> {
        let data_dir = dirs::data_local_dir().ok_or_else(|| {
            AppError::BackupFailed("Cannot determine local data directory".into())
        })?;
        Ok(data_dir.join("cron_manager").join("backups"))
    }

    /// Create a backup of the given crontab content.
    /// Returns the path to the backup file.
    pub fn create_backup(&self, content: &str) -> Result<PathBuf, AppError> {
        let now = chrono::Local::now();
        let timestamp = now.format("%Y%m%d_%H%M%S");
        let millis = now.timestamp_subsec_millis();
        let filename = format!("crontab_{}_{:03}.bak", timestamp, millis);
        let path = self.backup_dir.join(&filename);

        fs::write(&path, content).map_err(|e| {
            error!(path = %path.display(), error = %e, "Failed to write backup");
            AppError::BackupFailed(format!("Failed to write {}: {}", path.display(), e))
        })?;

        // Set file permissions to 0600 (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&path, perms).map_err(|e| {
                warn!(path = %path.display(), error = %e, "Failed to set backup permissions");
                AppError::BackupFailed(format!(
                    "Failed to set permissions on {}: {}",
                    path.display(),
                    e
                ))
            })?;
        }

        info!(path = %path.display(), "Backup created");
        self.enforce_retention()?;
        Ok(path)
    }

    /// List all backup files, sorted by modification time (newest first).
    pub fn list_backups(&self) -> Result<Vec<PathBuf>, AppError> {
        let mut entries: Vec<(PathBuf, std::time::SystemTime)> = fs::read_dir(&self.backup_dir)
            .map_err(|e| {
                AppError::BackupFailed(format!(
                    "Cannot read backup directory: {}",
                    e
                ))
            })?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("bak") {
                    let modified = entry.metadata().ok()?.modified().ok()?;
                    Some((path, modified))
                } else {
                    None
                }
            })
            .collect();

        entries.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(entries.into_iter().map(|(p, _)| p).collect())
    }

    /// Read a backup file's content.
    pub fn read_backup(&self, path: &PathBuf) -> Result<String, AppError> {
        fs::read_to_string(path).map_err(|e| {
            AppError::BackupFailed(format!("Failed to read backup {}: {}", path.display(), e))
        })
    }

    /// Enforce retention policy: keep only the most recent MAX_BACKUPS files.
    fn enforce_retention(&self) -> Result<(), AppError> {
        let backups = self.list_backups()?;
        if backups.len() > MAX_BACKUPS {
            for old_backup in &backups[MAX_BACKUPS..] {
                if let Err(e) = fs::remove_file(old_backup) {
                    warn!(
                        path = %old_backup.display(),
                        error = %e,
                        "Failed to remove old backup"
                    );
                } else {
                    info!(path = %old_backup.display(), "Pruned old backup");
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_backup_dir() -> (tempfile::TempDir, BackupManager) {
        let dir = tempfile::tempdir().unwrap();
        let mgr = BackupManager::with_dir(dir.path().to_path_buf());
        (dir, mgr)
    }

    #[test]
    fn test_create_backup() {
        let (_dir, mgr) = temp_backup_dir();
        let path = mgr.create_backup("* * * * * /bin/test\n").unwrap();
        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "* * * * * /bin/test\n");
    }

    #[test]
    fn test_list_backups() {
        let (_dir, mgr) = temp_backup_dir();
        mgr.create_backup("content1").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        mgr.create_backup("content2").unwrap();

        let backups = mgr.list_backups().unwrap();
        assert_eq!(backups.len(), 2);
    }

    #[test]
    fn test_read_backup() {
        let (_dir, mgr) = temp_backup_dir();
        let path = mgr.create_backup("test content").unwrap();
        let content = mgr.read_backup(&path).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_retention_policy() {
        let (_dir, mgr) = temp_backup_dir();

        // Create MAX_BACKUPS + 5 backups
        for i in 0..MAX_BACKUPS + 5 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            mgr.create_backup(&format!("content {}", i)).unwrap();
        }

        let backups = mgr.list_backups().unwrap();
        assert!(backups.len() <= MAX_BACKUPS);
    }

    #[cfg(unix)]
    #[test]
    fn test_backup_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let (_dir, mgr) = temp_backup_dir();
        let path = mgr.create_backup("secret content").unwrap();
        let perms = fs::metadata(&path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600);
    }
}
