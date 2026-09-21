#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskEvent {
    Started(TaskInfo),
    ObservedStarted(TaskInfo),
    Queued(TaskInfo),
    Progress {
        id: TaskId,
        progress: Option<u8>,
    },
    Completed {
        id: TaskId,
        success: bool,
    },
    ObservedCompleted {
        id: TaskId,
        success: bool,
        timing: ObservedTaskTiming,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservedTaskTiming {
    pub started: Option<SystemTime>,
    pub finished: Option<SystemTime>,
}
