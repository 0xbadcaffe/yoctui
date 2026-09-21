use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use thiserror::Error;
use tokio::sync::mpsc;
use yoctui_bitbake::{RawJobPlannerError, RawPtyCommandSpec};
use yoctui_model::{
    DaemonCompatibilitySnapshot, PtySessionId, RawAttachmentState, RawExecutionState, RawRequestId,
};
use yoctui_protocol::daemon::{JobId, LifecycleState};

mod control;
mod event_reduction;
mod job_supervisor;
mod pty_supervisor;
mod recovery;

const RAW_JOB_NAMESPACE: u64 = 5 << 60;
const RAW_PTY_NAMESPACE: u64 = 6 << 60;
const RAW_JOB_SEQUENCE_LIMIT: u64 = 1 << 60;
const RAW_SUPERVISOR_EVENT_CAPACITY: usize = 64;

#[derive(Debug, Clone)]
pub struct DaemonRawStart {
    pub job_id: JobId,
    pub state: RawExecutionState,
}

#[derive(Debug, Clone)]
pub struct DaemonRawPtyStart {
    pub pty_id: PtySessionId,
    pub command: RawPtyCommandSpec,
    pub state: RawExecutionState,
}

#[derive(Debug, Clone)]
pub enum DaemonRawCancel {
    Job,
    Pty {
        pty_id: PtySessionId,
        state: Box<RawExecutionState>,
    },
}

#[derive(Debug, Clone)]
pub enum DaemonRawAttachment {
    Job,
    Pty { pty_id: PtySessionId },
}

#[derive(Debug, Clone, Copy)]
enum RawJobControl {
    Cancel,
    Attachment(RawAttachmentState),
}

#[derive(Debug, Clone)]
pub struct DaemonRawEvent {
    pub job_id: JobId,
    pub state: RawExecutionState,
    pub lifecycle: LifecycleState,
    pub exit_code: Option<i32>,
}

pub struct DaemonRawSupervisor {
    compatibility: Option<DaemonCompatibilitySnapshot>,
    operation_timeout: Duration,
    cancellation_timeout: Duration,
    next_job_id: u64,
    next_pty_id: u64,
    seen_requests: HashSet<RawRequestId>,
    active: HashMap<RawRequestId, mpsc::UnboundedSender<RawJobControl>>,
    job_attachments: HashMap<RawRequestId, RawAttachmentState>,
    cancellation_requested: HashSet<RawRequestId>,
    pty_by_request: HashMap<RawRequestId, PtySessionId>,
    pty_states: HashMap<PtySessionId, RawExecutionState>,
    events_tx: mpsc::Sender<DaemonRawEvent>,
    events_rx: mpsc::Receiver<DaemonRawEvent>,
}

#[derive(Debug, Error)]
pub enum DaemonRawError {
    #[error("invalid Raw execution request: {0}")]
    InvalidRequest(String),
    #[error("daemon Raw execution requires current compatibility authority")]
    CompatibilityUnavailable,
    #[error("duplicate Raw execution request {0}")]
    DuplicateRequest(RawRequestId),
    #[error("unknown or terminal Raw execution request {0}")]
    UnknownRequest(RawRequestId),
    #[error("Raw execution cancellation was already requested for {0}")]
    CancellationAlreadyRequested(RawRequestId),
    #[error("Raw execution attachment is already in the requested state for {0}")]
    AttachmentUnchanged(RawRequestId),
    #[error("daemon Raw job ID space exhausted")]
    JobSpaceExhausted,
    #[error("daemon Raw PTY ID space exhausted")]
    PtySpaceExhausted,
    #[error("Raw request, session, and daemon PTY identities do not match")]
    PtyIdentityMismatch,
    #[error(transparent)]
    Planner(#[from] RawJobPlannerError),
    #[error(transparent)]
    Model(#[from] yoctui_model::RawExecutionError),
}

#[cfg(test)]
#[path = "tests/daemon_raw/mod.rs"]
mod tests;
