use crate::security::{AuditAction, AuditEntry};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Mutex;

/// Filter criteria for querying audit log entries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditFilter {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub action_type: Option<AuditAction>,
    pub connection_id: Option<String>,
}

/// Thread-safe audit logger that writes JSON-lines to a file.
pub struct AuditLogger {
    log_path: PathBuf,
    lock: Mutex<()>,
}

impl AuditLogger {
    /// Create a new AuditLogger that writes to the given file path.
    pub fn new(log_path: PathBuf) -> Self {
        Self {
            log_path,
            lock: Mutex::new(()),
        }
    }

    /// Append an audit entry as a JSON line to the log file.
    pub fn log_action(&self, entry: &AuditEntry) -> Result<(), AuditError> {
        let _guard = self.lock.lock().map_err(|e| AuditError::LockError(e.to_string()))?;

        // Ensure parent directory exists
        if let Some(parent) = self.log_path.parent() {
            fs::create_dir_all(parent).map_err(AuditError::Io)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .map_err(AuditError::Io)?;

        let json = serde_json::to_string(entry).map_err(AuditError::Serialization)?;
        writeln!(file, "{}", json).map_err(AuditError::Io)?;

        Ok(())
    }

    /// Read all entries from the log file, applying the given filter.
    pub fn get_entries(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>, AuditError> {
        let _guard = self.lock.lock().map_err(|e| AuditError::LockError(e.to_string()))?;

        if !self.log_path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&self.log_path).map_err(AuditError::Io)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(AuditError::Io)?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<AuditEntry>(trimmed) {
                Ok(entry) => {
                    if self.matches_filter(&entry, filter) {
                        entries.push(entry);
                    }
                }
                Err(_) => {
                    // Skip malformed lines rather than failing the whole query
                    continue;
                }
            }
        }

        Ok(entries)
    }

    /// Remove entries older than `max_age_days` days. Returns the number of entries purged.
    pub fn purge_old_entries(&self, max_age_days: u32) -> Result<usize, AuditError> {
        let _guard = self.lock.lock().map_err(|e| AuditError::LockError(e.to_string()))?;

        if !self.log_path.exists() {
            return Ok(0);
        }

        let file = fs::File::open(&self.log_path).map_err(AuditError::Io)?;
        let reader = BufReader::new(file);

        let cutoff = Utc::now() - chrono::Duration::days(max_age_days as i64);
        let cutoff_str = cutoff.format("%Y-%m-%dT%H:%M:%S").to_string();

        let mut kept = Vec::new();
        let mut purged = 0usize;

        for line in reader.lines() {
            let line = line.map_err(AuditError::Io)?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<AuditEntry>(trimmed) {
                Ok(entry) => {
                    if entry.timestamp >= cutoff_str {
                        kept.push(line);
                    } else {
                        purged += 1;
                    }
                }
                Err(_) => {
                    // Keep malformed lines to avoid data loss
                    kept.push(line);
                }
            }
        }

        // Rewrite the file with only the kept entries
        let mut file = fs::File::create(&self.log_path).map_err(AuditError::Io)?;
        for line in &kept {
            writeln!(file, "{}", line).map_err(AuditError::Io)?;
        }

        Ok(purged)
    }

    /// Check if an entry matches the given filter criteria.
    fn matches_filter(&self, entry: &AuditEntry, filter: &AuditFilter) -> bool {
        // Filter by date_from
        if let Some(ref from) = filter.date_from {
            if entry.timestamp.as_str() < from.as_str() {
                return false;
            }
        }

        // Filter by date_to
        if let Some(ref to) = filter.date_to {
            if entry.timestamp.as_str() > to.as_str() {
                return false;
            }
        }

        // Filter by action_type
        if let Some(ref action) = filter.action_type {
            if std::mem::discriminant(&entry.action) != std::mem::discriminant(action) {
                return false;
            }
        }

        // Filter by connection_id (matches source or target)
        if let Some(ref conn_id) = filter.connection_id {
            let matches_source = entry
                .source_connection
                .as_ref() == Some(conn_id);
            let matches_target = entry
                .target_connection
                .as_ref() == Some(conn_id);
            if !matches_source && !matches_target {
                return false;
            }
        }

        true
    }
}

/// Parse a date string in YYYY-MM-DD format. Used internally for date comparisons.
#[allow(dead_code)]
fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Errors that can occur during audit logging operations.
#[derive(Debug)]
pub enum AuditError {
    Io(std::io::Error),
    Serialization(serde_json::Error),
    LockError(String),
}

impl std::fmt::Display for AuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditError::Io(e) => write!(f, "Audit I/O error: {}", e),
            AuditError::Serialization(e) => write!(f, "Audit serialization error: {}", e),
            AuditError::LockError(e) => write!(f, "Audit lock error: {}", e),
        }
    }
}

impl std::error::Error for AuditError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn make_entry(id: &str, timestamp: &str, action: AuditAction) -> AuditEntry {
        AuditEntry {
            id: id.to_string(),
            timestamp: timestamp.to_string(),
            user: "test_user".to_string(),
            action,
            source_connection: Some("conn1".to_string()),
            target_connection: None,
            affected_rows: None,
            details: Some("test details".to_string()),
        }
    }

    #[test]
    fn test_log_and_read_entries() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.jsonl");
        let logger = AuditLogger::new(log_path.clone());

        let entry1 = make_entry("1", "2025-01-15T10:00:00Z", AuditAction::ConnectionCreated);
        let entry2 = make_entry("2", "2025-01-16T12:00:00Z", AuditAction::SchemaCompared);

        logger.log_action(&entry1).unwrap();
        logger.log_action(&entry2).unwrap();

        let entries = logger.get_entries(&AuditFilter::default()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "1");
        assert_eq!(entries[1].id, "2");
    }

    #[test]
    fn test_filter_by_action_type() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.jsonl");
        let logger = AuditLogger::new(log_path);

        logger
            .log_action(&make_entry(
                "1",
                "2025-01-15T10:00:00Z",
                AuditAction::ConnectionCreated,
            ))
            .unwrap();
        logger
            .log_action(&make_entry(
                "2",
                "2025-01-16T12:00:00Z",
                AuditAction::SchemaCompared,
            ))
            .unwrap();
        logger
            .log_action(&make_entry(
                "3",
                "2025-01-17T14:00:00Z",
                AuditAction::ConnectionCreated,
            ))
            .unwrap();

        let filter = AuditFilter {
            action_type: Some(AuditAction::ConnectionCreated),
            ..Default::default()
        };
        let entries = logger.get_entries(&filter).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "1");
        assert_eq!(entries[1].id, "3");
    }

    #[test]
    fn test_filter_by_date_range() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.jsonl");
        let logger = AuditLogger::new(log_path);

        logger
            .log_action(&make_entry(
                "1",
                "2025-01-10T10:00:00Z",
                AuditAction::ConnectionCreated,
            ))
            .unwrap();
        logger
            .log_action(&make_entry(
                "2",
                "2025-01-15T12:00:00Z",
                AuditAction::SchemaCompared,
            ))
            .unwrap();
        logger
            .log_action(&make_entry(
                "3",
                "2025-01-20T14:00:00Z",
                AuditAction::DataCompared,
            ))
            .unwrap();

        let filter = AuditFilter {
            date_from: Some("2025-01-12T00:00:00Z".to_string()),
            date_to: Some("2025-01-18T00:00:00Z".to_string()),
            ..Default::default()
        };
        let entries = logger.get_entries(&filter).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "2");
    }

    #[test]
    fn test_filter_by_connection_id() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.jsonl");
        let logger = AuditLogger::new(log_path);

        let mut entry1 = make_entry("1", "2025-01-15T10:00:00Z", AuditAction::ConnectionCreated);
        entry1.source_connection = Some("conn_a".to_string());

        let mut entry2 = make_entry("2", "2025-01-16T12:00:00Z", AuditAction::SchemaCompared);
        entry2.source_connection = Some("conn_b".to_string());
        entry2.target_connection = Some("conn_a".to_string());

        let mut entry3 = make_entry("3", "2025-01-17T14:00:00Z", AuditAction::DataCompared);
        entry3.source_connection = Some("conn_c".to_string());

        logger.log_action(&entry1).unwrap();
        logger.log_action(&entry2).unwrap();
        logger.log_action(&entry3).unwrap();

        let filter = AuditFilter {
            connection_id: Some("conn_a".to_string()),
            ..Default::default()
        };
        let entries = logger.get_entries(&filter).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_get_entries_no_file() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("nonexistent.jsonl");
        let logger = AuditLogger::new(log_path);

        let entries = logger.get_entries(&AuditFilter::default()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_entry_json_serialization() {
        let entry = make_entry("1", "2025-01-15T10:00:00Z", AuditAction::MigrationStarted);
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "1");
        assert_eq!(deserialized.timestamp, "2025-01-15T10:00:00Z");
    }

    #[test]
    fn test_purge_old_entries() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.jsonl");
        let logger = AuditLogger::new(log_path);

        // Write an old entry and a recent entry
        let old_entry = make_entry("old", "2020-01-01T00:00:00Z", AuditAction::ConnectionCreated);
        let recent_entry = make_entry(
            "recent",
            "2099-01-01T00:00:00Z",
            AuditAction::SchemaCompared,
        );

        logger.log_action(&old_entry).unwrap();
        logger.log_action(&recent_entry).unwrap();

        let purged = logger.purge_old_entries(30).unwrap();
        assert_eq!(purged, 1);

        let entries = logger.get_entries(&AuditFilter::default()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "recent");
    }

    #[test]
    fn test_malformed_lines_skipped() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.jsonl");

        // Write a valid entry and a malformed line
        let entry = make_entry("1", "2025-01-15T10:00:00Z", AuditAction::ConnectionCreated);
        let json = serde_json::to_string(&entry).unwrap();
        fs::write(&log_path, format!("{}\nthis is not json\n", json)).unwrap();

        let logger = AuditLogger::new(log_path);
        let entries = logger.get_entries(&AuditFilter::default()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "1");
    }
}
