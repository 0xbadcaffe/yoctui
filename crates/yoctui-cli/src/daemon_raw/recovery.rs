use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use tokio::sync::mpsc;
use yoctui_model::{DaemonCompatibilitySnapshot, RawRequestId};
use yoctui_protocol::daemon::{DaemonSnapshot, JobKind, RawExecutionOwnerData};

use super::{
    DaemonRawError, DaemonRawSupervisor, RAW_JOB_NAMESPACE, RAW_JOB_SEQUENCE_LIMIT,
    RAW_PTY_NAMESPACE, RAW_SUPERVISOR_EVENT_CAPACITY,
};

impl Default for DaemonRawSupervisor {
    fn default() -> Self {
        let (events_tx, events_rx) = mpsc::channel(RAW_SUPERVISOR_EVENT_CAPACITY);
        Self {
            compatibility: None,
            operation_timeout: Duration::from_secs(24 * 60 * 60),
            cancellation_timeout: Duration::from_secs(5),
            next_job_id: 1,
            next_pty_id: 1,
            seen_requests: HashSet::new(),
            active: HashMap::new(),
            job_attachments: HashMap::new(),
            cancellation_requested: HashSet::new(),
            pty_by_request: HashMap::new(),
            pty_states: HashMap::new(),
            events_tx,
            events_rx,
        }
    }
}

impl DaemonRawSupervisor {
    pub fn replace_compatibility(
        &mut self,
        compatibility: Option<DaemonCompatibilitySnapshot>,
    ) -> Result<(), yoctui_model::DaemonStateError> {
        self.compatibility = compatibility
            .map(DaemonCompatibilitySnapshot::normalize)
            .transpose()?;
        Ok(())
    }

    pub fn restore_snapshot(&mut self, snapshot: &DaemonSnapshot) -> Result<(), DaemonRawError> {
        for execution in &snapshot.raw_executions {
            execution
                .validate()
                .map_err(|error| DaemonRawError::InvalidRequest(error.to_string()))?;
            self.seen_requests
                .insert(RawRequestId::new(&execution.request.request_id)?);
            if let Some(RawExecutionOwnerData::Pty(session)) = &execution.owner
                && let Some(sequence) = session
                    .strip_prefix("raw-session:daemon-")
                    .and_then(|sequence| sequence.parse::<u64>().ok())
                && sequence < RAW_JOB_SEQUENCE_LIMIT
            {
                self.next_pty_id = self.next_pty_id.max(sequence.saturating_add(1));
            }
        }
        for job in snapshot.jobs.iter().filter(|job| job.kind == JobKind::Raw) {
            if job.id.0 >> 60 == RAW_JOB_NAMESPACE >> 60 {
                let sequence = job.id.0 & (RAW_JOB_SEQUENCE_LIMIT - 1);
                self.next_job_id = self.next_job_id.max(sequence.saturating_add(1));
            }
        }
        for pty in &snapshot.pty_sessions {
            if pty.id.0 >> 60 == RAW_PTY_NAMESPACE >> 60 {
                let sequence = pty.id.0 & (RAW_JOB_SEQUENCE_LIMIT - 1);
                self.next_pty_id = self.next_pty_id.max(sequence.saturating_add(1));
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn with_timeouts(mut self, operation: Duration, cancellation: Duration) -> Self {
        self.operation_timeout = operation;
        self.cancellation_timeout = cancellation;
        self
    }
}
