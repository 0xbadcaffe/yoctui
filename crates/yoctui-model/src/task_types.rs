//! Task types.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct TaskId(pub String);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TaskState {
    Queued,
    Waiting,
    #[default]
    Active,
    Completed,
    Failed,
    Cancelled,
    Lost,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskStats {
    pub completed: usize,
    pub total: usize,
    pub active: usize,
    pub failed: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaskInfo {
    pub id: TaskId,
    pub recipe: String,
    pub task: String,
    pub progress: Option<u8>,
    #[serde(default)]
    pub state: TaskState,
    #[serde(default)]
    pub worker: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub started: Option<SystemTime>,
    #[serde(default)]
    pub finished: Option<SystemTime>,
    #[serde(default)]
    pub dependencies: Vec<TaskId>,
    #[serde(default)]
    pub log_path: Option<PathBuf>,
    #[serde(default)]
    pub cancellation: Option<String>,
    #[serde(default)]
    pub stats: Option<TaskStats>,
}
impl TaskInfo {
    pub fn active(id: TaskId, recipe: String, task: String) -> Self {
        Self {
            id,
            recipe,
            task,
            progress: None,
            state: TaskState::Active,
            worker: None,
            pid: None,
            started: Some(SystemTime::now()),
            finished: None,
            dependencies: Vec::new(),
            log_path: None,
            cancellation: None,
            stats: None,
        }
    }
    pub fn elapsed_at(&self, now: SystemTime) -> Option<Duration> {
        let end = self
            .finished
            .or_else(|| (self.state == TaskState::Active).then_some(now))?;
        self.started
            .and_then(|started| end.duration_since(started).ok())
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedTask {
    pub task: TaskInfo,
    pub success: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskStateFilter {
    #[default]
    All,
    Active,
    Waiting,
    Completed,
    Failed,
}
impl TaskStateFilter {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::All => Self::Active,
            Self::Active => Self::Waiting,
            Self::Waiting => Self::Completed,
            Self::Completed => Self::Failed,
            Self::Failed => Self::All,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskFilterField {
    #[default]
    Recipe,
    Task,
    Worker,
}
impl TaskFilterField {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Recipe => Self::Task,
            Self::Task => Self::Worker,
            Self::Worker => Self::Recipe,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskFilters {
    pub state: TaskStateFilter,
    pub recipe: String,
    pub task: String,
    pub worker: String,
    pub minimum_duration: Option<Duration>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskRow {
    Task(Box<TaskInfo>),
    WaitingSummary(usize),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskRowRef<'a> {
    Task {
        task: &'a TaskInfo,
        state: TaskState,
    },
    WaitingSummary(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskProjectionKey {
    Active(TaskId, TaskState),
    Completed(usize, TaskState),
    WaitingSummary(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskProjectionCache {
    pub generation: u64,
    pub filters: TaskFilters,
    pub duration_second: Option<u64>,
    pub active_len: usize,
    pub completed_len: usize,
    pub waiting: usize,
    pub rows: Vec<TaskProjectionKey>,
    pub rebuilds: u64,
}

impl Default for TaskProjectionCache {
    fn default() -> Self {
        Self {
            generation: u64::MAX,
            filters: TaskFilters::default(),
            duration_second: None,
            active_len: 0,
            completed_len: 0,
            waiting: 0,
            rows: Vec::new(),
            rebuilds: 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TaskProjectionCacheCell(pub(crate) RefCell<TaskProjectionCache>);

impl PartialEq for TaskProjectionCacheCell {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for TaskProjectionCacheCell {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskInspectorRef<'a> {
    None,
    Waiting {
        count: usize,
    },
    Task {
        task: &'a TaskInfo,
        state: TaskState,
        version: Option<&'a str>,
        revision: Option<&'a str>,
        workdir: Option<&'a Path>,
        recent_logs: Vec<&'a LogEntry>,
    },
}
impl TaskRowRef<'_> {
    pub fn into_owned(self) -> TaskRow {
        match self {
            Self::Task { task, state } => {
                let mut task = task.clone();
                task.state = state;
                TaskRow::Task(Box::new(task))
            }
            Self::WaitingSummary(waiting) => TaskRow::WaitingSummary(waiting),
        }
    }
}
