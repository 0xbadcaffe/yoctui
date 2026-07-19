//! Domain model and pure state transitions. BitBake remains authoritative.
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    path::PathBuf,
    time::{Duration, SystemTime},
};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AppError {
    #[error("{category}: {message}. {remedy}")]
    Message {
        category: &'static str,
        message: String,
        remedy: String,
    },
}
impl AppError {
    pub fn new(
        category: &'static str,
        message: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self::Message {
            category,
            message: message.into(),
            remedy: remedy.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Screen {
    Dashboard,
    Recipes,
    Layers,
    Configuration,
    Logs,
    Errors,
    Help,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildStatus {
    Idle,
    LoadingWorkspace,
    Parsing,
    Running,
    Cancelling,
    Completed,
    Failed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Trace,
    Info,
    Warning,
    Error,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRequest {
    pub targets: Vec<String>,
    pub task: Option<String>,
}
impl BuildRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.targets.is_empty()
            || self.targets.iter().any(|x| {
                x.is_empty()
                    || !x
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+'))
            })
        {
            return Err(AppError::new(
                "Configuration",
                "invalid build target",
                "pass one or more BitBake target names",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: TaskId,
    pub recipe: String,
    pub task: String,
    pub progress: Option<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub severity: Severity,
    pub message: String,
    pub recipe: Option<String>,
    pub task: Option<String>,
    pub path: Option<PathBuf>,
    pub timestamp: SystemTime,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Workspace {
    pub build_dir: Option<PathBuf>,
    pub source_dir: Option<PathBuf>,
    pub variables: HashMap<String, String>,
    pub bitbake_version: Option<String>,
    pub layers: Vec<Layer>,
    pub recipes: Vec<Recipe>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub path: PathBuf,
    pub priority: Option<i32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipe {
    pub name: String,
    pub version: Option<String>,
    pub layer: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildState {
    pub status: BuildStatus,
    pub target: Option<String>,
    pub started: Option<SystemTime>,
    pub completed: usize,
    pub total: Option<usize>,
    pub warnings: usize,
    pub errors: usize,
}
impl Default for BuildState {
    fn default() -> Self {
        Self {
            status: BuildStatus::Idle,
            target: None,
            started: None,
            completed: 0,
            total: None,
            warnings: 0,
            errors: 0,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogState {
    pub entries: VecDeque<LogEntry>,
    pub max_entries: usize,
    pub max_bytes: usize,
    pub retained_bytes: usize,
    pub dropped: usize,
    pub follow: bool,
    pub wrap: bool,
    pub filter: Option<Severity>,
    pub query: String,
}
impl LogState {
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
            max_bytes,
            retained_bytes: 0,
            dropped: 0,
            follow: true,
            wrap: false,
            filter: None,
            query: String::new(),
        }
    }
    pub fn insert(&mut self, entry: LogEntry) {
        let bytes = entry.message.len();
        self.retained_bytes += bytes;
        self.entries.push_back(entry);
        while self.entries.len() > self.max_entries || self.retained_bytes > self.max_bytes {
            if let Some(old) = self.entries.pop_front() {
                self.retained_bytes = self.retained_bytes.saturating_sub(old.message.len());
                self.dropped += 1;
            } else {
                break;
            }
        }
    }
    pub fn filtered(&self) -> impl Iterator<Item = &LogEntry> {
        self.entries.iter().filter(move |e| {
            self.filter.is_none_or(|s| s == e.severity)
                && (self.query.is_empty()
                    || e.message
                        .to_lowercase()
                        .contains(&self.query.to_lowercase()))
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub screen: Screen,
    pub workspace: Workspace,
    pub build: BuildState,
    pub tasks: HashMap<TaskId, TaskInfo>,
    pub logs: LogState,
    pub should_quit: bool,
    pub quit_confirm: bool,
    pub notification: Option<String>,
}
impl App {
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            screen: Screen::Dashboard,
            workspace: Workspace::default(),
            build: BuildState::default(),
            tasks: HashMap::new(),
            logs: LogState::new(max_entries, max_bytes),
            should_quit: false,
            quit_confirm: false,
            notification: None,
        }
    }
    pub fn elapsed(&self) -> Option<Duration> {
        self.build
            .started
            .and_then(|s| SystemTime::now().duration_since(s).ok())
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Tick,
    Open(Screen),
    Start(BuildRequest),
    BuildStarted,
    ParseProgress,
    TaskStarted(TaskInfo),
    TaskProgress { id: TaskId, progress: u8 },
    TaskCompleted { id: TaskId, success: bool },
    Log(LogEntry),
    BuildCompleted { success: bool },
    Cancel,
    Quit,
    ConfirmQuit,
    WorkspaceLoaded(Workspace),
    Failure(AppError),
}
pub fn update(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::Open(s) => app.screen = s,
        Action::Start(r) => {
            if let Err(e) = r.validate() {
                app.notification = Some(e.to_string())
            } else {
                app.build.status = BuildStatus::LoadingWorkspace;
                app.build.target = r.targets.first().cloned();
                return Some(Effect::Start(r));
            }
        }
        Action::BuildStarted => {
            app.build.status = BuildStatus::Running;
            app.build.started = Some(SystemTime::now());
        }
        Action::ParseProgress => app.build.status = BuildStatus::Parsing,
        Action::TaskStarted(t) => {
            app.tasks.insert(t.id.clone(), t);
        }
        Action::TaskProgress { id, progress } => {
            if let Some(t) = app.tasks.get_mut(&id) {
                t.progress = Some(progress)
            }
        }
        Action::TaskCompleted { id, .. } => {
            app.tasks.remove(&id);
            app.build.completed += 1
        }
        Action::Log(l) => {
            match l.severity {
                Severity::Warning => app.build.warnings += 1,
                Severity::Error => app.build.errors += 1,
                _ => {}
            }
            app.logs.insert(l);
        }
        Action::BuildCompleted { success } => {
            app.build.status = if success {
                BuildStatus::Completed
            } else {
                BuildStatus::Failed
            }
        }
        Action::Cancel => {
            if matches!(
                app.build.status,
                BuildStatus::Running | BuildStatus::Parsing
            ) {
                app.build.status = BuildStatus::Cancelling;
                return Some(Effect::Cancel);
            }
        }
        Action::Quit => {
            if matches!(
                app.build.status,
                BuildStatus::Running | BuildStatus::Parsing | BuildStatus::Cancelling
            ) {
                app.quit_confirm = true
            } else {
                app.should_quit = true
            }
        }
        Action::ConfirmQuit => app.should_quit = true,
        Action::WorkspaceLoaded(w) => app.workspace = w,
        Action::Failure(e) => {
            app.notification = Some(e.to_string());
            app.build.status = BuildStatus::Failed
        }
        Action::Tick => {}
    }
    None
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Start(BuildRequest),
    Cancel,
}
pub fn format_duration(duration: Duration) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        duration.as_secs() / 3600,
        duration.as_secs() / 60 % 60,
        duration.as_secs() % 60
    )
}
impl fmt::Display for BuildStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn log(message: &str) -> LogEntry {
        LogEntry {
            severity: Severity::Info,
            message: message.into(),
            recipe: None,
            task: None,
            path: None,
            timestamp: SystemTime::now(),
        }
    }
    #[test]
    fn bounded_logs_report_eviction() {
        let mut l = LogState::new(2, 100);
        l.insert(log("a"));
        l.insert(log("b"));
        l.insert(log("c"));
        assert_eq!(l.entries.len(), 2);
        assert_eq!(l.dropped, 1)
    }
    #[test]
    fn running_build_requires_confirmation() {
        let mut a = App::new(2, 10);
        a.build.status = BuildStatus::Running;
        update(&mut a, Action::Quit);
        assert!(a.quit_confirm);
        assert!(!a.should_quit)
    }
    #[test]
    fn request_validation() {
        assert!(
            BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None
            }
            .validate()
            .is_ok()
        );
        assert!(
            BuildRequest {
                targets: vec!["bad target".into()],
                task: None
            }
            .validate()
            .is_err()
        )
    }
}
