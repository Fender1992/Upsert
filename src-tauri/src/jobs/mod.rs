use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
