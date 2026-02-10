use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// A saved job configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    pub id: String,
    pub name: String,
    pub job_type: JobType,
    pub schedule: Option<CronSchedule>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Type of job
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobType {
    Comparison,
    Migration,
}

/// Cron-style schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronSchedule {
    pub expression: String,
    pub timezone: String,
}

/// Parsed cron fields: minute, hour, day-of-month, month, day-of-week
#[derive(Debug, Clone, PartialEq)]
pub struct CronFields {
    pub minutes: Vec<u32>,
    pub hours: Vec<u32>,
    pub days_of_month: Vec<u32>,
    pub months: Vec<u32>,
    pub days_of_week: Vec<u32>,
}

/// Execution record for a job run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobExecution {
    pub id: String,
    pub job_id: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: JobStatus,
    pub result_summary: Option<String>,
    pub error_message: Option<String>,
}

/// Status of a job execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Behavior when a job in a chain fails
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChainBehavior {
    Stop,
    Skip,
    Custom(String),
}

/// A chain of jobs to execute in order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobChain {
    pub jobs: Vec<String>,
    pub on_failure: ChainBehavior,
}

impl JobChain {
    /// Create a new job chain with the given job IDs and failure behavior.
    pub fn new(jobs: Vec<String>, on_failure: ChainBehavior) -> Self {
        Self { jobs, on_failure }
    }

    /// Add a job ID to the end of the chain.
    pub fn add_job(&mut self, job_id: String) {
        self.jobs.push(job_id);
    }

    /// Return the number of jobs in the chain.
    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    /// Return true if the chain has no jobs.
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }
}

// ---------------------------------------------------------------------------
// CronSchedule methods
// ---------------------------------------------------------------------------

impl CronSchedule {
    /// Parse a single cron field (e.g. "0", "*/15", "1,3,5", "1-5") into a
    /// sorted vector of matching values within [min, max].
    fn parse_field(field: &str, min: u32, max: u32) -> Result<Vec<u32>, String> {
        let mut values = Vec::new();

        for part in field.split(',') {
            let part = part.trim();

            if part == "*" {
                // Every value
                values.extend(min..=max);
            } else if let Some(step_str) = part.strip_prefix("*/") {
                // Step values: */N
                let step: u32 = step_str
                    .parse()
                    .map_err(|_| format!("Invalid step value: {}", step_str))?;
                if step == 0 {
                    return Err("Step value cannot be zero".to_string());
                }
                let mut v = min;
                while v <= max {
                    values.push(v);
                    v += step;
                }
            } else if part.contains('-') {
                // Range: N-M
                let bounds: Vec<&str> = part.split('-').collect();
                if bounds.len() != 2 {
                    return Err(format!("Invalid range: {}", part));
                }
                let start: u32 = bounds[0]
                    .parse()
                    .map_err(|_| format!("Invalid range start: {}", bounds[0]))?;
                let end: u32 = bounds[1]
                    .parse()
                    .map_err(|_| format!("Invalid range end: {}", bounds[1]))?;
                if start > end || start < min || end > max {
                    return Err(format!("Range out of bounds: {}-{}", start, end));
                }
                values.extend(start..=end);
            } else {
                // Single value
                let v: u32 = part
                    .parse()
                    .map_err(|_| format!("Invalid value: {}", part))?;
                if v < min || v > max {
                    return Err(format!("Value {} out of range [{}, {}]", v, min, max));
                }
                values.push(v);
            }
        }

        values.sort();
        values.dedup();
        Ok(values)
    }

    /// Parse the cron expression into its five component fields.
    /// Standard cron format: minute hour day-of-month month day-of-week
    pub fn parse_expression(&self) -> Result<CronFields, String> {
        let parts: Vec<&str> = self.expression.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(format!(
                "Cron expression must have 5 fields, got {}",
                parts.len()
            ));
        }

        let minutes = Self::parse_field(parts[0], 0, 59)?;
        let hours = Self::parse_field(parts[1], 0, 23)?;
        let days_of_month = Self::parse_field(parts[2], 1, 31)?;
        let months = Self::parse_field(parts[3], 1, 12)?;
        let days_of_week = Self::parse_field(parts[4], 0, 6)?;

        Ok(CronFields {
            minutes,
            hours,
            days_of_month,
            months,
            days_of_week,
        })
    }

    /// Check whether the given datetime string matches this cron schedule.
    /// Accepts RFC 3339 / ISO 8601 datetime strings (e.g. "2025-06-15T14:30:00").
    pub fn matches_datetime(&self, dt: &str) -> bool {
        let fields = match self.parse_expression() {
            Ok(f) => f,
            Err(_) => return false,
        };

        // Parse the datetime using chrono
        let naive = if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(dt, "%Y-%m-%dT%H:%M:%S")
        {
            ndt
        } else if let Ok(ndt) =
            chrono::NaiveDateTime::parse_from_str(dt, "%Y-%m-%dT%H:%M:%S%.f")
        {
            ndt
        } else if let Ok(fixed) = chrono::DateTime::parse_from_rfc3339(dt) {
            fixed.naive_local()
        } else {
            return false;
        };

        use chrono::{Datelike, Timelike};
        let minute = naive.minute();
        let hour = naive.hour();
        let day = naive.day();
        let month = naive.month();
        let weekday = naive.weekday().num_days_from_sunday(); // 0=Sun .. 6=Sat

        fields.minutes.contains(&minute)
            && fields.hours.contains(&hour)
            && fields.days_of_month.contains(&day)
            && fields.months.contains(&month)
            && fields.days_of_week.contains(&weekday)
    }

    /// Validate that the cron expression is syntactically correct.
    pub fn is_valid(&self) -> bool {
        self.parse_expression().is_ok()
    }
}

// ---------------------------------------------------------------------------
// JobStore — file-based storage for job configurations
// ---------------------------------------------------------------------------

/// File-based storage for job configurations. Each job is stored as an
/// individual JSON file named `{id}.json` inside `base_dir`.
pub struct JobStore {
    base_dir: PathBuf,
}

impl JobStore {
    /// Create a new `JobStore` backed by the given directory. The directory
    /// is created if it does not already exist.
    pub fn new(base_dir: &str) -> Self {
        let path = PathBuf::from(base_dir);
        if !path.exists() {
            fs::create_dir_all(&path).expect("Failed to create job store directory");
        }
        Self { base_dir: path }
    }

    /// Return the path for a job file given its ID.
    fn job_path(&self, id: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", id))
    }

    /// Save (create or update) a job configuration to disk.
    pub fn save_job(&self, config: &JobConfig) {
        let path = self.job_path(&config.id);
        let json =
            serde_json::to_string_pretty(config).expect("Failed to serialize job config");
        fs::write(path, json).expect("Failed to write job config file");
    }

    /// Load a single job configuration by ID. Returns `None` if the file
    /// does not exist or cannot be parsed.
    pub fn load_job(&self, id: &str) -> Option<JobConfig> {
        let path = self.job_path(id);
        if !path.exists() {
            return None;
        }
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    /// List all saved job configurations in the store directory.
    pub fn list_jobs(&self) -> Vec<JobConfig> {
        let mut jobs = Vec::new();
        let entries = match fs::read_dir(&self.base_dir) {
            Ok(e) => e,
            Err(_) => return jobs,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(data) = fs::read_to_string(&path) {
                    if let Ok(config) = serde_json::from_str::<JobConfig>(&data) {
                        jobs.push(config);
                    }
                }
            }
        }
        jobs
    }

    /// Delete a job configuration file from disk.
    pub fn delete_job(&self, id: &str) {
        let path = self.job_path(id);
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }
}

// ---------------------------------------------------------------------------
// ExecutionStore — tracks job execution history
// ---------------------------------------------------------------------------

/// File-based storage for job execution records. Executions for each job
/// are stored in a sub-directory named after the job ID, one JSON file per
/// execution.
pub struct ExecutionStore {
    base_dir: PathBuf,
}

impl ExecutionStore {
    /// Create a new `ExecutionStore` backed by the given directory.
    pub fn new(base_dir: &str) -> Self {
        let path = PathBuf::from(base_dir);
        if !path.exists() {
            fs::create_dir_all(&path).expect("Failed to create execution store directory");
        }
        Self { base_dir: path }
    }

    /// Return the directory that holds execution files for a given job.
    fn job_exec_dir(&self, job_id: &str) -> PathBuf {
        self.base_dir.join(job_id)
    }

    /// Save an execution record to disk.
    pub fn record_execution(&self, exec: &JobExecution) {
        let dir = self.job_exec_dir(&exec.job_id);
        if !dir.exists() {
            fs::create_dir_all(&dir).expect("Failed to create execution directory");
        }
        let path = dir.join(format!("{}.json", exec.id));
        let json =
            serde_json::to_string_pretty(exec).expect("Failed to serialize execution record");
        fs::write(path, json).expect("Failed to write execution record file");
    }

    /// Get all execution records for a given job, sorted by `started_at`
    /// ascending.
    pub fn get_executions(&self, job_id: &str) -> Vec<JobExecution> {
        let dir = self.job_exec_dir(job_id);
        if !dir.exists() {
            return Vec::new();
        }
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };
        let mut execs = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(data) = fs::read_to_string(&path) {
                    if let Ok(ex) = serde_json::from_str::<JobExecution>(&data) {
                        execs.push(ex);
                    }
                }
            }
        }
        execs.sort_by(|a, b| a.started_at.cmp(&b.started_at));
        execs
    }

    /// Get the most recent execution for a job (by `started_at`).
    pub fn get_latest_execution(&self, job_id: &str) -> Option<JobExecution> {
        self.get_executions(job_id).into_iter().last()
    }

    /// Purge execution records older than `max_age_days` days. Comparison is
    /// done against `started_at` timestamps using UTC now.
    pub fn purge_old_executions(&self, max_age_days: u32) {
        let cutoff = chrono::Utc::now()
            - chrono::Duration::days(i64::from(max_age_days));
        let cutoff_str = cutoff.format("%Y-%m-%dT%H:%M:%S").to_string();

        let entries = match fs::read_dir(&self.base_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let job_dir = entry.path();
            if !job_dir.is_dir() {
                continue;
            }
            let files = match fs::read_dir(&job_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for file_entry in files.flatten() {
                let path = file_entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                if let Ok(data) = fs::read_to_string(&path) {
                    if let Ok(exec) = serde_json::from_str::<JobExecution>(&data) {
                        if exec.started_at < cutoff_str {
                            let _ = fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper to create a simple JobConfig for testing.
    fn make_job(id: &str, name: &str, job_type: JobType) -> JobConfig {
        JobConfig {
            id: id.to_string(),
            name: name.to_string(),
            job_type,
            schedule: None,
            enabled: true,
            created_at: "2025-01-01T00:00:00".to_string(),
            updated_at: "2025-01-01T00:00:00".to_string(),
        }
    }

    /// Helper to create a JobExecution for testing.
    fn make_execution(id: &str, job_id: &str, started_at: &str, status: JobStatus) -> JobExecution {
        JobExecution {
            id: id.to_string(),
            job_id: job_id.to_string(),
            started_at: started_at.to_string(),
            completed_at: None,
            status,
            result_summary: None,
            error_message: None,
        }
    }

    // ---- JobStore CRUD tests ----

    #[test]
    fn test_job_store_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let store = JobStore::new(tmp.path().to_str().unwrap());

        let job = make_job("job-1", "Test Job", JobType::Comparison);
        store.save_job(&job);

        let loaded = store.load_job("job-1");
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.id, "job-1");
        assert_eq!(loaded.name, "Test Job");
    }

    #[test]
    fn test_job_store_load_missing() {
        let tmp = TempDir::new().unwrap();
        let store = JobStore::new(tmp.path().to_str().unwrap());

        let result = store.load_job("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_job_store_list_jobs() {
        let tmp = TempDir::new().unwrap();
        let store = JobStore::new(tmp.path().to_str().unwrap());

        store.save_job(&make_job("j1", "Job One", JobType::Comparison));
        store.save_job(&make_job("j2", "Job Two", JobType::Migration));
        store.save_job(&make_job("j3", "Job Three", JobType::Comparison));

        let jobs = store.list_jobs();
        assert_eq!(jobs.len(), 3);
    }

    #[test]
    fn test_job_store_list_empty() {
        let tmp = TempDir::new().unwrap();
        let store = JobStore::new(tmp.path().to_str().unwrap());

        let jobs = store.list_jobs();
        assert!(jobs.is_empty());
    }

    #[test]
    fn test_job_store_delete_job() {
        let tmp = TempDir::new().unwrap();
        let store = JobStore::new(tmp.path().to_str().unwrap());

        let job = make_job("d1", "Delete Me", JobType::Migration);
        store.save_job(&job);
        assert!(store.load_job("d1").is_some());

        store.delete_job("d1");
        assert!(store.load_job("d1").is_none());
    }

    #[test]
    fn test_job_store_delete_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let store = JobStore::new(tmp.path().to_str().unwrap());

        // Should not panic when deleting a job that doesn't exist.
        store.delete_job("does-not-exist");
    }

    #[test]
    fn test_job_store_update_job() {
        let tmp = TempDir::new().unwrap();
        let store = JobStore::new(tmp.path().to_str().unwrap());

        let mut job = make_job("u1", "Original", JobType::Comparison);
        store.save_job(&job);

        job.name = "Updated".to_string();
        job.updated_at = "2025-06-01T00:00:00".to_string();
        store.save_job(&job);

        let loaded = store.load_job("u1").unwrap();
        assert_eq!(loaded.name, "Updated");
        assert_eq!(loaded.updated_at, "2025-06-01T00:00:00");

        // Still only one file
        assert_eq!(store.list_jobs().len(), 1);
    }

    // ---- ExecutionStore tests ----

    #[test]
    fn test_execution_store_record_and_get() {
        let tmp = TempDir::new().unwrap();
        let store = ExecutionStore::new(tmp.path().to_str().unwrap());

        let exec = make_execution("e1", "job-1", "2025-03-01T10:00:00", JobStatus::Completed);
        store.record_execution(&exec);

        let execs = store.get_executions("job-1");
        assert_eq!(execs.len(), 1);
        assert_eq!(execs[0].id, "e1");
        assert_eq!(execs[0].status, JobStatus::Completed);
    }

    #[test]
    fn test_execution_store_get_empty() {
        let tmp = TempDir::new().unwrap();
        let store = ExecutionStore::new(tmp.path().to_str().unwrap());

        let execs = store.get_executions("no-such-job");
        assert!(execs.is_empty());
    }

    #[test]
    fn test_execution_store_latest() {
        let tmp = TempDir::new().unwrap();
        let store = ExecutionStore::new(tmp.path().to_str().unwrap());

        store.record_execution(&make_execution(
            "e1",
            "job-1",
            "2025-03-01T10:00:00",
            JobStatus::Completed,
        ));
        store.record_execution(&make_execution(
            "e2",
            "job-1",
            "2025-03-02T10:00:00",
            JobStatus::Failed,
        ));
        store.record_execution(&make_execution(
            "e3",
            "job-1",
            "2025-03-03T10:00:00",
            JobStatus::Running,
        ));

        let latest = store.get_latest_execution("job-1");
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().id, "e3");
    }

    #[test]
    fn test_execution_store_latest_missing_job() {
        let tmp = TempDir::new().unwrap();
        let store = ExecutionStore::new(tmp.path().to_str().unwrap());

        assert!(store.get_latest_execution("ghost").is_none());
    }

    #[test]
    fn test_execution_store_purge_old() {
        let tmp = TempDir::new().unwrap();
        let store = ExecutionStore::new(tmp.path().to_str().unwrap());

        // Record one very old execution and one recent one.
        store.record_execution(&make_execution(
            "old",
            "job-1",
            "2020-01-01T00:00:00",
            JobStatus::Completed,
        ));

        let recent_ts = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();
        store.record_execution(&make_execution(
            "new",
            "job-1",
            &recent_ts,
            JobStatus::Completed,
        ));

        assert_eq!(store.get_executions("job-1").len(), 2);

        // Purge anything older than 30 days.
        store.purge_old_executions(30);

        let remaining = store.get_executions("job-1");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "new");
    }

    // ---- CronSchedule tests ----

    #[test]
    fn test_cron_valid_expression() {
        let cron = CronSchedule {
            expression: "0 12 * * 1".to_string(),
            timezone: "UTC".to_string(),
        };
        assert!(cron.is_valid());
    }

    #[test]
    fn test_cron_invalid_expression_too_few_fields() {
        let cron = CronSchedule {
            expression: "0 12 *".to_string(),
            timezone: "UTC".to_string(),
        };
        assert!(!cron.is_valid());
    }

    #[test]
    fn test_cron_invalid_expression_bad_value() {
        let cron = CronSchedule {
            expression: "99 12 * * 1".to_string(),
            timezone: "UTC".to_string(),
        };
        assert!(!cron.is_valid());
    }

    #[test]
    fn test_cron_parse_wildcard() {
        let cron = CronSchedule {
            expression: "* * * * *".to_string(),
            timezone: "UTC".to_string(),
        };
        let fields = cron.parse_expression().unwrap();
        assert_eq!(fields.minutes.len(), 60); // 0..59
        assert_eq!(fields.hours.len(), 24);   // 0..23
        assert_eq!(fields.days_of_month.len(), 31); // 1..31
        assert_eq!(fields.months.len(), 12);  // 1..12
        assert_eq!(fields.days_of_week.len(), 7); // 0..6
    }

    #[test]
    fn test_cron_parse_step() {
        let cron = CronSchedule {
            expression: "*/15 * * * *".to_string(),
            timezone: "UTC".to_string(),
        };
        let fields = cron.parse_expression().unwrap();
        assert_eq!(fields.minutes, vec![0, 15, 30, 45]);
    }

    #[test]
    fn test_cron_parse_range() {
        let cron = CronSchedule {
            expression: "0 9-17 * * *".to_string(),
            timezone: "UTC".to_string(),
        };
        let fields = cron.parse_expression().unwrap();
        assert_eq!(fields.hours, vec![9, 10, 11, 12, 13, 14, 15, 16, 17]);
    }

    #[test]
    fn test_cron_parse_list() {
        let cron = CronSchedule {
            expression: "0 0 1,15 * *".to_string(),
            timezone: "UTC".to_string(),
        };
        let fields = cron.parse_expression().unwrap();
        assert_eq!(fields.days_of_month, vec![1, 15]);
    }

    #[test]
    fn test_cron_matches_datetime() {
        // "At minute 30 past hour 14 on every day" -> should match 14:30
        let cron = CronSchedule {
            expression: "30 14 * * *".to_string(),
            timezone: "UTC".to_string(),
        };
        assert!(cron.matches_datetime("2025-06-15T14:30:00")); // Sunday
        assert!(!cron.matches_datetime("2025-06-15T14:31:00"));
        assert!(!cron.matches_datetime("2025-06-15T15:30:00"));
    }

    #[test]
    fn test_cron_matches_specific_weekday() {
        // "At 09:00 on Monday" (1 = Monday in 0=Sun convention)
        let cron = CronSchedule {
            expression: "0 9 * * 1".to_string(),
            timezone: "UTC".to_string(),
        };
        // 2025-06-16 is a Monday
        assert!(cron.matches_datetime("2025-06-16T09:00:00"));
        // 2025-06-15 is a Sunday
        assert!(!cron.matches_datetime("2025-06-15T09:00:00"));
    }

    #[test]
    fn test_cron_matches_invalid_datetime() {
        let cron = CronSchedule {
            expression: "0 12 * * *".to_string(),
            timezone: "UTC".to_string(),
        };
        assert!(!cron.matches_datetime("not-a-date"));
    }

    // ---- JobChain tests ----

    #[test]
    fn test_job_chain_creation() {
        let chain = JobChain::new(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            ChainBehavior::Stop,
        );
        assert_eq!(chain.len(), 3);
        assert!(!chain.is_empty());
        assert_eq!(chain.on_failure, ChainBehavior::Stop);
    }

    #[test]
    fn test_job_chain_add_job() {
        let mut chain = JobChain::new(vec![], ChainBehavior::Skip);
        assert!(chain.is_empty());

        chain.add_job("first".to_string());
        chain.add_job("second".to_string());
        assert_eq!(chain.len(), 2);
        assert_eq!(chain.jobs[0], "first");
        assert_eq!(chain.jobs[1], "second");
    }

    #[test]
    fn test_job_chain_custom_behavior() {
        let chain = JobChain::new(
            vec!["x".to_string()],
            ChainBehavior::Custom("retry-3".to_string()),
        );
        assert_eq!(
            chain.on_failure,
            ChainBehavior::Custom("retry-3".to_string())
        );
    }

    // ---- Additional edge-case tests ----

    #[test]
    fn test_job_store_creates_directory() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("deep").join("nested").join("jobs");
        let store = JobStore::new(nested.to_str().unwrap());

        store.save_job(&make_job("x", "Nested", JobType::Comparison));
        assert!(store.load_job("x").is_some());
    }

    #[test]
    fn test_execution_multiple_jobs_isolation() {
        let tmp = TempDir::new().unwrap();
        let store = ExecutionStore::new(tmp.path().to_str().unwrap());

        store.record_execution(&make_execution(
            "e1", "job-a", "2025-01-01T00:00:00", JobStatus::Completed,
        ));
        store.record_execution(&make_execution(
            "e2", "job-b", "2025-01-01T00:00:00", JobStatus::Failed,
        ));

        assert_eq!(store.get_executions("job-a").len(), 1);
        assert_eq!(store.get_executions("job-b").len(), 1);
        assert_eq!(store.get_executions("job-a")[0].id, "e1");
        assert_eq!(store.get_executions("job-b")[0].id, "e2");
    }

    #[test]
    fn test_job_config_with_schedule() {
        let tmp = TempDir::new().unwrap();
        let store = JobStore::new(tmp.path().to_str().unwrap());

        let job = JobConfig {
            id: "sched-1".to_string(),
            name: "Scheduled Job".to_string(),
            job_type: JobType::Comparison,
            schedule: Some(CronSchedule {
                expression: "0 */6 * * *".to_string(),
                timezone: "UTC".to_string(),
            }),
            enabled: true,
            created_at: "2025-01-01T00:00:00".to_string(),
            updated_at: "2025-01-01T00:00:00".to_string(),
        };

        store.save_job(&job);
        let loaded = store.load_job("sched-1").unwrap();
        assert!(loaded.schedule.is_some());
        let sched = loaded.schedule.unwrap();
        assert_eq!(sched.expression, "0 */6 * * *");
        assert!(sched.is_valid());
    }
}
