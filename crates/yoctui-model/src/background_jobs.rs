//! Background jobs.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildState {
    pub status: BuildStatus,
    pub target: Option<String>,
    pub started: Option<SystemTime>,
    pub completed: usize,
    pub total: Option<usize>,
    pub parse_current: Option<u64>,
    pub parse_total: Option<u64>,
    pub warnings: usize,
    pub errors: usize,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildSummary {
    pub completed: usize,
    pub total: Option<usize>,
    pub active: usize,
    pub waiting: usize,
    pub warnings: usize,
    pub errors: usize,
    pub elapsed: Option<Duration>,
}

impl BuildSummary {
    pub fn progress_percent(self) -> Option<u8> {
        let total = self.total.filter(|total| *total > 0)?;
        let percent = self.completed.min(total).saturating_mul(100) / total;
        Some(u8::try_from(percent).unwrap_or(100))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRecord {
    pub target: Option<String>,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub elapsed: Option<Duration>,
    pub completed_tasks: usize,
    pub warnings: usize,
    pub errors: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BackgroundJobId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundJobKind {
    Build,
    CveCheck,
    Spdx,
    Qemu,
    Wic,
    Sdk,
    Test,
    Devtool,
    Maintenance,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundJobStatus {
    Queued,
    Starting,
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
    Lost,
}
impl BackgroundJobStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Lost
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundJobProgress {
    Indeterminate,
    Percent(u8),
    Units { completed: u64, total: u64 },
}
impl BackgroundJobProgress {
    pub(crate) fn is_valid(&self) -> bool {
        match self {
            Self::Indeterminate => true,
            Self::Percent(percent) => *percent <= 100,
            Self::Units { completed, total } => *total > 0 && completed <= total,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackgroundJobContext {
    pub workspace: Option<Screen>,
    pub target: Option<String>,
    pub recipe: Option<String>,
    pub task: Option<String>,
    pub image: Option<String>,
    pub path: Option<PathBuf>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundJobSpec {
    pub id: BackgroundJobId,
    pub kind: BackgroundJobKind,
    pub title: String,
    pub context: BackgroundJobContext,
    pub cancellation_supported: bool,
    pub queued_at: SystemTime,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundJobOutputSource {
    Backend,
    Stdout,
    Stderr,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundJobOutputEntry {
    pub severity: Severity,
    pub message: String,
    pub source: BackgroundJobOutputSource,
    pub truncated: bool,
    pub timestamp: SystemTime,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundJobResult {
    pub summary: String,
    pub artifacts: Vec<PathBuf>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundJobError {
    pub summary: String,
    pub detail: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundJob {
    pub id: BackgroundJobId,
    pub kind: BackgroundJobKind,
    pub title: String,
    pub status: BackgroundJobStatus,
    pub context: BackgroundJobContext,
    pub cancellation_supported: bool,
    pub progress: BackgroundJobProgress,
    pub output: VecDeque<BackgroundJobOutputEntry>,
    pub retained_output_bytes: usize,
    pub dropped_output_entries: usize,
    pub warnings: usize,
    pub errors: usize,
    pub queued_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub finished_at: Option<SystemTime>,
    pub result: Option<BackgroundJobResult>,
    pub error: Option<BackgroundJobError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobHistoryRowRef<'a> {
    Daemon(&'a ClientDaemonJobSummary),
    Background(&'a BackgroundJob),
    Build(&'a BuildRecord),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JobSummary {
    pub active: usize,
    pub queued: usize,
    pub failed: usize,
    pub recent_completed: usize,
    pub daemon_owned: Option<usize>,
}
impl BackgroundJob {
    pub(crate) fn from_spec(spec: BackgroundJobSpec) -> Self {
        Self {
            id: spec.id,
            kind: spec.kind,
            title: spec.title,
            status: BackgroundJobStatus::Queued,
            context: spec.context,
            cancellation_supported: spec.cancellation_supported,
            progress: BackgroundJobProgress::Indeterminate,
            output: VecDeque::new(),
            retained_output_bytes: 0,
            dropped_output_entries: 0,
            warnings: 0,
            errors: 0,
            queued_at: spec.queued_at,
            started_at: None,
            finished_at: None,
            result: None,
            error: None,
        }
    }
}
pub(crate) const MAX_BACKGROUND_JOBS: usize = 128;
pub(crate) const MAX_BACKGROUND_JOB_OUTPUT_ENTRIES: usize = 512;
pub(crate) const MAX_BACKGROUND_JOB_OUTPUT_BYTES: usize = 1024 * 1024;
pub const MAX_SIGNATURE_RECORDS: usize = 256;
pub const MAX_SIGNATURE_DIFFERENCES: usize = 4_096;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundJobs {
    pub jobs: VecDeque<BackgroundJob>,
    pub dropped_jobs: usize,
    pub rejected_jobs: usize,
    pub ignored_transitions: usize,
    pub(crate) max_jobs: usize,
    pub(crate) max_output_entries: usize,
    pub(crate) max_output_bytes: usize,
}
impl BackgroundJobs {
    pub fn new(max_jobs: usize, max_output_entries: usize, max_output_bytes: usize) -> Self {
        Self {
            jobs: VecDeque::new(),
            dropped_jobs: 0,
            rejected_jobs: 0,
            ignored_transitions: 0,
            max_jobs: max_jobs.max(1),
            max_output_entries: max_output_entries.max(1),
            max_output_bytes: max_output_bytes.max(1),
        }
    }

    pub fn get(&self, id: BackgroundJobId) -> Option<&BackgroundJob> {
        self.jobs.iter().find(|job| job.id == id)
    }

    pub(crate) fn queue(&mut self, spec: BackgroundJobSpec) {
        if spec.title.trim().is_empty() || self.get(spec.id).is_some() {
            self.rejected_jobs += 1;
            return;
        }
        while self.jobs.len() >= self.max_jobs {
            let Some(index) = self.jobs.iter().position(|job| job.status.is_terminal()) else {
                self.rejected_jobs += 1;
                return;
            };
            self.jobs.remove(index);
            self.dropped_jobs += 1;
        }
        self.jobs.push_back(BackgroundJob::from_spec(spec));
    }

    pub(crate) fn update_if(
        &mut self,
        id: BackgroundJobId,
        allowed: &[BackgroundJobStatus],
        mutation: impl FnOnce(&mut BackgroundJob),
    ) {
        let Some(job) = self.jobs.iter_mut().find(|job| job.id == id) else {
            self.ignored_transitions += 1;
            return;
        };
        if !allowed.contains(&job.status) {
            self.ignored_transitions += 1;
            return;
        }
        mutation(job);
    }

    pub(crate) fn append_output(&mut self, id: BackgroundJobId, entry: BackgroundJobOutputEntry) {
        let max_entries = self.max_output_entries;
        let max_bytes = self.max_output_bytes;
        self.update_if(
            id,
            &[
                BackgroundJobStatus::Queued,
                BackgroundJobStatus::Starting,
                BackgroundJobStatus::Running,
                BackgroundJobStatus::Cancelling,
            ],
            |job| {
                match entry.severity {
                    Severity::Warning => job.warnings += 1,
                    Severity::Error => job.errors += 1,
                    Severity::Trace | Severity::Info => {}
                }
                job.retained_output_bytes += entry.message.len();
                job.output.push_back(entry);
                while job.output.len() > max_entries || job.retained_output_bytes > max_bytes {
                    let Some(dropped) = job.output.pop_front() else {
                        break;
                    };
                    job.retained_output_bytes = job
                        .retained_output_bytes
                        .saturating_sub(dropped.message.len());
                    job.dropped_output_entries += 1;
                }
            },
        );
    }

    pub(crate) fn request_cancellation(&mut self, id: BackgroundJobId) {
        let Some(job) = self.jobs.iter_mut().find(|job| job.id == id) else {
            self.ignored_transitions += 1;
            return;
        };
        if !job.cancellation_supported
            || !matches!(
                job.status,
                BackgroundJobStatus::Queued
                    | BackgroundJobStatus::Starting
                    | BackgroundJobStatus::Running
            )
        {
            self.ignored_transitions += 1;
            return;
        }
        job.status = BackgroundJobStatus::Cancelling;
    }
}
impl Default for BackgroundJobs {
    fn default() -> Self {
        Self::new(
            MAX_BACKGROUND_JOBS,
            MAX_BACKGROUND_JOB_OUTPUT_ENTRIES,
            MAX_BACKGROUND_JOB_OUTPUT_BYTES,
        )
    }
}
