use std::{
    collections::{HashMap, HashSet},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use thiserror::Error;
use tokio::sync::mpsc;
use yoctui_bitbake::{
    RawJobPlanner, RawJobPlannerError, RawJobRunner, RawJobRunnerEvent, RawPtyCommandSpec,
    RawPtyPlanner,
};
use yoctui_model::{
    DaemonCompatibilitySnapshot, PtySessionId, RawAttachmentState, RawDurableReferenceId,
    RawEventCursor, RawExecutionEvent, RawExecutionEventKind, RawExecutionOutcome,
    RawExecutionOwner, RawExecutionResult, RawExecutionState, RawJobId, RawRequestId, RawSessionId,
    RawStreamId, reduce_raw_execution,
};
use yoctui_protocol::daemon::{
    DaemonSnapshot, JobId, JobKind, LifecycleState, RawExecutionOwnerData, RawExecutionRequestData,
};

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

    pub fn start(
        &mut self,
        wire: RawExecutionRequestData,
    ) -> Result<DaemonRawStart, DaemonRawError> {
        let request = yoctui_app::raw_execution_request_from_protocol(&wire)
            .map_err(DaemonRawError::InvalidRequest)?;
        if self.seen_requests.contains(&request.id) {
            return Err(DaemonRawError::DuplicateRequest(request.id));
        }
        let sequence = self.next_job_id;
        if sequence >= RAW_JOB_SEQUENCE_LIMIT {
            return Err(DaemonRawError::JobSpaceExhausted);
        }
        let job_id = JobId(RAW_JOB_NAMESPACE | sequence);
        let raw_job_id = RawJobId::new(format!("raw-job:daemon-{sequence}"))?;
        let stdout = RawStreamId::new(format!("raw-stream:daemon-{sequence}-stdout"))?;
        let stderr = RawStreamId::new(format!("raw-stream:daemon-{sequence}-stderr"))?;
        let compatibility = self
            .compatibility
            .as_ref()
            .ok_or(DaemonRawError::CompatibilityUnavailable)?;
        let command = RawJobPlanner::new(compatibility).plan(
            &request,
            raw_job_id.clone(),
            stdout.clone(),
            stderr.clone(),
        )?;
        let state = RawExecutionState::queued(
            request.clone(),
            stdout,
            stderr,
            unix_ms(),
            RawEventCursor::default(),
        )?;
        self.next_job_id = self
            .next_job_id
            .checked_add(1)
            .ok_or(DaemonRawError::JobSpaceExhausted)?;
        self.seen_requests.insert(request.id.clone());
        let (control_tx, mut control_rx) = mpsc::unbounded_channel();
        self.active.insert(request.id.clone(), control_tx);
        self.job_attachments
            .insert(request.id.clone(), RawAttachmentState::Attached);

        let events = self.events_tx.clone();
        let operation_timeout = self.operation_timeout;
        let cancellation_timeout = self.cancellation_timeout;
        let task_state = state.clone();
        tokio::spawn(async move {
            let mut state = task_state;
            if !advance_and_send(
                &events,
                job_id,
                &mut state,
                RawExecutionEventKind::Starting {
                    owner: RawExecutionOwner::Job(raw_job_id),
                },
                LifecycleState::Connecting,
                None,
            )
            .await
            {
                return;
            }
            let mut runner = RawJobRunner::new()
                .with_operation_timeout(operation_timeout)
                .with_cancellation_timeout(cancellation_timeout);
            if let Err(error) = runner.start(command).await {
                let _ = finish_and_send(
                    &events,
                    job_id,
                    &mut state,
                    RawExecutionOutcome::Failed,
                    None,
                    Some(error.to_string()),
                )
                .await;
                return;
            }
            match runner.next_event().await {
                Ok(RawJobRunnerEvent::Started) => {
                    if !advance_and_send(
                        &events,
                        job_id,
                        &mut state,
                        RawExecutionEventKind::Running {
                            started_unix_ms: unix_ms(),
                        },
                        LifecycleState::Running,
                        None,
                    )
                    .await
                    {
                        return;
                    }
                }
                Ok(_) => {
                    let _ = finish_and_send(
                        &events,
                        job_id,
                        &mut state,
                        RawExecutionOutcome::Lost,
                        None,
                        Some("Raw runner omitted its start acknowledgement".into()),
                    )
                    .await;
                    return;
                }
                Err(error) => {
                    let _ = finish_and_send(
                        &events,
                        job_id,
                        &mut state,
                        RawExecutionOutcome::Lost,
                        None,
                        Some(error.to_string()),
                    )
                    .await;
                    return;
                }
            }
            let started = std::time::Instant::now();
            let mut control_open = true;
            loop {
                tokio::select! {
                    control = control_rx.recv(), if control_open => {
                        let Some(control) = control else {
                            control_open = false;
                            if !request_cancellation(&events, job_id, &mut state).await {
                                return;
                            }
                            if let Err(error) = runner.cancel().await {
                                let _ = finish_and_send(&events, job_id, &mut state, RawExecutionOutcome::Lost, None, Some(error.to_string())).await;
                                return;
                            }
                            continue;
                        };
                        match control {
                            RawJobControl::Cancel => {
                                if !request_cancellation(&events, job_id, &mut state).await {
                                    return;
                                }
                                if let Err(error) = runner.cancel().await {
                                    let _ = finish_and_send(&events, job_id, &mut state, RawExecutionOutcome::Lost, None, Some(error.to_string())).await;
                                    return;
                                }
                            }
                            RawJobControl::Attachment(attachment) => {
                                let lifecycle = lifecycle_for(&state);
                                if state.attachment != attachment
                                    && !advance_and_send(
                                        &events,
                                        job_id,
                                        &mut state,
                                        RawExecutionEventKind::AttachmentChanged { attachment },
                                        lifecycle,
                                        None,
                                    )
                                    .await
                                {
                                    return;
                                }
                            }
                        }
                    }
                    event = runner.next_event() => {
                        let event = match event {
                            Ok(event) => event,
                            Err(error) => {
                                let _ = finish_and_send(
                                    &events,
                                    job_id,
                                    &mut state,
                                    RawExecutionOutcome::Lost,
                                    None,
                                    Some(error.to_string()),
                                )
                                .await;
                                return;
                            }
                        };
                        match event {
                            RawJobRunnerEvent::Started => return,
                            RawJobRunnerEvent::Output(chunk) => {
                                let lifecycle = lifecycle_for(&state);
                                if !advance_and_send(
                                    &events,
                                    job_id,
                                    &mut state,
                                    RawExecutionEventKind::Output { chunk },
                                    lifecycle,
                                    None,
                                )
                                .await
                                {
                                    return;
                                }
                            }
                            RawJobRunnerEvent::Completed { exit_code } => {
                                let _ = finish_and_send(
                                    &events,
                                    job_id,
                                    &mut state,
                                    RawExecutionOutcome::Succeeded,
                                    Some(exit_code),
                                    None,
                                )
                                .await;
                                return;
                            }
                            RawJobRunnerEvent::Failed { exit_code, message } => {
                                let _ = finish_and_send(
                                    &events,
                                    job_id,
                                    &mut state,
                                    RawExecutionOutcome::Failed,
                                    exit_code,
                                    Some(message),
                                )
                                .await;
                                return;
                            }
                            RawJobRunnerEvent::TimedOut { forced, exit_code } => {
                                let _ = finish_and_send(
                                    &events,
                                    job_id,
                                    &mut state,
                                    RawExecutionOutcome::Failed,
                                    exit_code,
                                    Some(format!("Raw command timed out (forced termination: {forced})")),
                                )
                                .await;
                                return;
                            }
                            RawJobRunnerEvent::Cancelled { forced, exit_code } => {
                                let _ = finish_and_send(
                                    &events,
                                    job_id,
                                    &mut state,
                                    RawExecutionOutcome::Cancelled,
                                    exit_code,
                                    Some(format!("Raw command cancelled (forced termination: {forced})")),
                                )
                                .await;
                                return;
                            }
                            RawJobRunnerEvent::Lost { message } => {
                                let _ = finish_and_send(
                                    &events,
                                    job_id,
                                    &mut state,
                                    RawExecutionOutcome::Lost,
                                    None,
                                    Some(message),
                                )
                                .await;
                                return;
                            }
                        }
                        let elapsed = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                        let lifecycle = lifecycle_for(&state);
                        if !state.phase.is_terminal() && elapsed > state.elapsed_ms
                            && !advance_and_send(
                                &events,
                                job_id,
                                &mut state,
                                RawExecutionEventKind::Elapsed { elapsed_ms: elapsed },
                                lifecycle,
                                None,
                            )
                            .await
                        {
                            return;
                        }
                    }
                }
            }
        });
        Ok(DaemonRawStart { job_id, state })
    }

    pub fn prepare_pty(
        &self,
        wire: RawExecutionRequestData,
    ) -> Result<DaemonRawPtyStart, DaemonRawError> {
        let request = yoctui_app::raw_execution_request_from_protocol(&wire)
            .map_err(DaemonRawError::InvalidRequest)?;
        if self.seen_requests.contains(&request.id) {
            return Err(DaemonRawError::DuplicateRequest(request.id));
        }
        let sequence = self.next_pty_id;
        if sequence >= RAW_JOB_SEQUENCE_LIMIT {
            return Err(DaemonRawError::PtySpaceExhausted);
        }
        let pty_id = PtySessionId(RAW_PTY_NAMESPACE | sequence);
        let raw_session = RawSessionId::new(format!("raw-session:daemon-{sequence}"))?;
        let compatibility = self
            .compatibility
            .as_ref()
            .ok_or(DaemonRawError::CompatibilityUnavailable)?;
        let command = RawPtyPlanner::new(compatibility).plan(&request, raw_session.clone())?;
        let stdout = RawStreamId::new(format!("raw-stream:daemon-pty-{sequence}-stdout"))?;
        let stderr = RawStreamId::new(format!("raw-stream:daemon-pty-{sequence}-stderr"))?;
        let mut state = RawExecutionState::queued(
            request,
            stdout,
            stderr,
            unix_ms(),
            RawEventCursor::default(),
        )?;
        advance_sync(
            &mut state,
            RawExecutionEventKind::Starting {
                owner: RawExecutionOwner::Pty(raw_session),
            },
        )?;
        Ok(DaemonRawPtyStart {
            pty_id,
            command,
            state,
        })
    }

    pub fn activate_pty(&mut self, start: &DaemonRawPtyStart) -> Result<(), DaemonRawError> {
        let owner_matches = matches!(
            start.state.owner.as_ref(),
            Some(RawExecutionOwner::Pty(session)) if session == start.command.session_id()
        );
        if start.command.request_id() != &start.state.request.id
            || start.command.capability_generation() != start.state.request.capability_generation
            || start.command.current_directory() != start.state.request.build_directory
            || !owner_matches
        {
            return Err(DaemonRawError::PtyIdentityMismatch);
        }
        if self.seen_requests.contains(&start.state.request.id)
            || self.pty_states.contains_key(&start.pty_id)
            || start.pty_id.0 != RAW_PTY_NAMESPACE | self.next_pty_id
        {
            return Err(DaemonRawError::DuplicateRequest(
                start.state.request.id.clone(),
            ));
        }
        self.next_pty_id = self
            .next_pty_id
            .checked_add(1)
            .ok_or(DaemonRawError::PtySpaceExhausted)?;
        self.seen_requests.insert(start.state.request.id.clone());
        self.pty_by_request
            .insert(start.state.request.id.clone(), start.pty_id);
        self.pty_states.insert(start.pty_id, start.state.clone());
        Ok(())
    }

    pub fn pty_started(
        &mut self,
        pty_id: PtySessionId,
    ) -> Result<Option<RawExecutionState>, DaemonRawError> {
        let Some(state) = self.pty_states.get_mut(&pty_id) else {
            return Ok(None);
        };
        advance_sync(
            state,
            RawExecutionEventKind::Running {
                started_unix_ms: unix_ms(),
            },
        )?;
        Ok(Some(state.clone()))
    }

    pub fn pty_attachment(
        &mut self,
        pty_id: PtySessionId,
        attached: bool,
    ) -> Result<Option<RawExecutionState>, DaemonRawError> {
        let Some(state) = self.pty_states.get_mut(&pty_id) else {
            return Ok(None);
        };
        let attachment = if attached {
            RawAttachmentState::Attached
        } else {
            RawAttachmentState::Detached
        };
        if state.attachment == attachment || state.phase.is_terminal() {
            return Ok(None);
        }
        advance_sync(
            state,
            RawExecutionEventKind::AttachmentChanged { attachment },
        )?;
        Ok(Some(state.clone()))
    }

    pub fn pty_finished(
        &mut self,
        pty_id: PtySessionId,
        exit_code: Option<i32>,
        lost: Option<String>,
    ) -> Result<Option<RawExecutionState>, DaemonRawError> {
        let Some(mut state) = self.pty_states.remove(&pty_id) else {
            return Ok(None);
        };
        self.pty_by_request.remove(&state.request.id);
        self.cancellation_requested.remove(&state.request.id);
        let (outcome, code, message) = if let Some(message) = lost {
            (RawExecutionOutcome::Lost, None, Some(message))
        } else if state.cancellation_requested {
            (
                RawExecutionOutcome::Cancelled,
                exit_code,
                Some("Raw PTY terminated".into()),
            )
        } else if exit_code == Some(0) {
            (RawExecutionOutcome::Succeeded, Some(0), None)
        } else {
            (
                RawExecutionOutcome::Failed,
                exit_code,
                Some("Raw PTY exited unsuccessfully".into()),
            )
        };
        let elapsed_ms = state.started_unix_ms.map_or(state.elapsed_ms, |started| {
            unix_ms().saturating_sub(started)
        });
        advance_sync(
            &mut state,
            RawExecutionEventKind::Finished {
                result: RawExecutionResult {
                    outcome,
                    exit_code: code,
                    message,
                    elapsed_ms,
                    durable_reference: None,
                },
            },
        )?;
        Ok(Some(state))
    }

    pub fn cancel(&mut self, request_id: &str) -> Result<DaemonRawCancel, DaemonRawError> {
        let request_id = RawRequestId::new(request_id)?;
        if let Some(pty_id) = self.pty_by_request.get(&request_id).copied() {
            if !self.cancellation_requested.insert(request_id.clone()) {
                return Err(DaemonRawError::CancellationAlreadyRequested(request_id));
            }
            let state = self
                .pty_states
                .get_mut(&pty_id)
                .ok_or_else(|| DaemonRawError::UnknownRequest(request_id.clone()))?;
            advance_sync(state, RawExecutionEventKind::CancellationRequested)?;
            advance_sync(state, RawExecutionEventKind::Cancelling)?;
            return Ok(DaemonRawCancel::Pty {
                pty_id,
                state: Box::new(state.clone()),
            });
        }
        if !self.active.contains_key(&request_id) {
            return Err(DaemonRawError::UnknownRequest(request_id));
        }
        if !self.cancellation_requested.insert(request_id.clone()) {
            return Err(DaemonRawError::CancellationAlreadyRequested(request_id));
        }
        self.active
            .get(&request_id)
            .expect("active request was checked")
            .send(RawJobControl::Cancel)
            .map_err(|_| DaemonRawError::UnknownRequest(request_id))?;
        Ok(DaemonRawCancel::Job)
    }

    pub fn set_attachment(
        &mut self,
        request_id: &str,
        attachment: RawAttachmentState,
    ) -> Result<DaemonRawAttachment, DaemonRawError> {
        let request_id = RawRequestId::new(request_id)?;
        if let Some(pty_id) = self.pty_by_request.get(&request_id).copied() {
            let state = self
                .pty_states
                .get_mut(&pty_id)
                .ok_or_else(|| DaemonRawError::UnknownRequest(request_id.clone()))?;
            if state.attachment == attachment || state.phase.is_terminal() {
                return Err(DaemonRawError::AttachmentUnchanged(request_id));
            }
            return Ok(DaemonRawAttachment::Pty { pty_id });
        }
        let control = self
            .active
            .get(&request_id)
            .ok_or_else(|| DaemonRawError::UnknownRequest(request_id.clone()))?;
        if self.job_attachments.get(&request_id) == Some(&attachment) {
            return Err(DaemonRawError::AttachmentUnchanged(request_id));
        }
        control
            .send(RawJobControl::Attachment(attachment))
            .map_err(|_| DaemonRawError::UnknownRequest(request_id.clone()))?;
        self.job_attachments.insert(request_id, attachment);
        Ok(DaemonRawAttachment::Job)
    }

    pub fn cancel_pty(
        &mut self,
        pty_id: PtySessionId,
    ) -> Result<Option<DaemonRawCancel>, DaemonRawError> {
        let request = self
            .pty_by_request
            .iter()
            .find_map(|(request, candidate)| (*candidate == pty_id).then(|| request.clone()));
        request
            .map(|request| self.cancel(request.as_str()))
            .transpose()
    }

    pub fn try_event(&mut self) -> Option<DaemonRawEvent> {
        let event = self.events_rx.try_recv().ok()?;
        if event.state.phase.is_terminal() {
            self.active.remove(&event.state.request.id);
            self.job_attachments.remove(&event.state.request.id);
            self.cancellation_requested.remove(&event.state.request.id);
        }
        Some(event)
    }
}

fn advance_sync(
    state: &mut RawExecutionState,
    kind: RawExecutionEventKind,
) -> Result<(), yoctui_model::RawExecutionError> {
    reduce_raw_execution(
        state,
        RawExecutionEvent {
            request_id: state.request.id.clone(),
            sequence: state.cursor.sequence.saturating_add(1),
            generation: state.cursor.generation.saturating_add(1),
            kind,
        },
    )?;
    Ok(())
}

async fn request_cancellation(
    events: &mpsc::Sender<DaemonRawEvent>,
    job_id: JobId,
    state: &mut RawExecutionState,
) -> bool {
    if state.cancellation_requested {
        return true;
    }
    let lifecycle = lifecycle_for(state);
    advance_and_send(
        events,
        job_id,
        state,
        RawExecutionEventKind::CancellationRequested,
        lifecycle,
        None,
    )
    .await
        && advance_and_send(
            events,
            job_id,
            state,
            RawExecutionEventKind::Cancelling,
            LifecycleState::Stopping,
            None,
        )
        .await
}

async fn finish_and_send(
    events: &mpsc::Sender<DaemonRawEvent>,
    job_id: JobId,
    state: &mut RawExecutionState,
    outcome: RawExecutionOutcome,
    exit_code: Option<i32>,
    message: Option<String>,
) -> bool {
    let elapsed_ms = state
        .started_unix_ms
        .map_or(state.elapsed_ms, |started| {
            unix_ms().saturating_sub(started)
        })
        .max(state.elapsed_ms);
    let durable_reference = RawDurableReferenceId::new(format!(
        "raw-durable:daemon-{}",
        job_id.0 & !RAW_JOB_NAMESPACE
    ))
    .ok();
    let lifecycle = match outcome {
        RawExecutionOutcome::Succeeded | RawExecutionOutcome::Cancelled => LifecycleState::Exited,
        RawExecutionOutcome::Failed => LifecycleState::Failed,
        RawExecutionOutcome::Lost => LifecycleState::Lost,
    };
    advance_and_send(
        events,
        job_id,
        state,
        RawExecutionEventKind::Finished {
            result: RawExecutionResult {
                outcome,
                exit_code,
                message,
                elapsed_ms,
                durable_reference,
            },
        },
        lifecycle,
        exit_code,
    )
    .await
}

async fn advance_and_send(
    events: &mpsc::Sender<DaemonRawEvent>,
    job_id: JobId,
    state: &mut RawExecutionState,
    kind: RawExecutionEventKind,
    lifecycle: LifecycleState,
    exit_code: Option<i32>,
) -> bool {
    let event = RawExecutionEvent {
        request_id: state.request.id.clone(),
        sequence: state.cursor.sequence.saturating_add(1),
        generation: state.cursor.generation.saturating_add(1),
        kind,
    };
    if reduce_raw_execution(state, event).is_err() {
        return false;
    }
    events
        .send(DaemonRawEvent {
            job_id,
            state: state.clone(),
            lifecycle,
            exit_code,
        })
        .await
        .is_ok()
}

fn lifecycle_for(state: &RawExecutionState) -> LifecycleState {
    use yoctui_model::RawExecutionPhase;
    match state.phase {
        RawExecutionPhase::Queued | RawExecutionPhase::Starting => LifecycleState::Connecting,
        RawExecutionPhase::Running => LifecycleState::Running,
        RawExecutionPhase::Cancelling => LifecycleState::Stopping,
        RawExecutionPhase::Terminal(
            RawExecutionOutcome::Succeeded | RawExecutionOutcome::Cancelled,
        ) => LifecycleState::Exited,
        RawExecutionPhase::Terminal(RawExecutionOutcome::Failed) => LifecycleState::Failed,
        RawExecutionPhase::Terminal(RawExecutionOutcome::Lost) => LifecycleState::Lost,
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
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
mod tests {
    use super::*;
    use std::{collections::BTreeMap, fs, os::unix::fs::PermissionsExt, path::PathBuf};
    use yoctui_model::{
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
        CapabilitySnapshot, CapabilityState, IdentityAuthority, RawAdditionalArguments,
        RawCapabilityRequirement, RawExecutionPolicy, RawInteractionMode, RawParameterValue,
        RawPreviewRequest, ToolIdentity, YoctoEnvironmentIdentity, builtin_raw_catalog,
    };

    struct Fixture(PathBuf);

    impl Fixture {
        fn new(name: &str, body: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-daemon-raw-{name}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&path).unwrap();
            let executable = path.join("bitbake");
            fs::write(&executable, body).unwrap();
            let mut permissions = fs::metadata(&executable).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&executable, permissions).unwrap();
            Self(path)
        }

        fn executable(&self) -> PathBuf {
            self.0.join("bitbake")
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn authority_for(
        fixture: &Fixture,
        interaction: RawInteractionMode,
    ) -> DaemonCompatibilitySnapshot {
        let catalog = builtin_raw_catalog();
        let command = catalog
            .commands
            .iter()
            .find(|command| {
                matches!(
                    command.execution,
                    RawExecutionPolicy::Executable { ref template }
                        if template.interaction == interaction
                            && (interaction == RawInteractionMode::InteractivePty
                                || command.parameters.is_empty())
                )
            })
            .unwrap();
        let RawExecutionPolicy::Executable { template } = &command.execution else {
            unreachable!();
        };
        let required = match &template.capabilities {
            RawCapabilityRequirement::All { capabilities }
            | RawCapabilityRequirement::Any { capabilities } => capabilities.clone(),
        };
        DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation: 17,
                environment: YoctoEnvironmentIdentity {
                    build_directory: AuthoritativeValue::detected(
                        fixture.0.clone(),
                        IdentityAuthority::InitializedEnvironment,
                    ),
                    available_tools: AuthoritativeValue::detected(
                        vec![ToolIdentity {
                            id: "bitbake".into(),
                            executable: fixture.executable(),
                            version: Some("fixture".into()),
                        }],
                        IdentityAuthority::ExecutableProbe,
                    ),
                    ..YoctoEnvironmentIdentity::default()
                },
                capabilities: required
                    .iter()
                    .copied()
                    .map(|id| CapabilityRecord {
                        id,
                        state: CapabilityState::Available,
                        evidence: vec![CapabilityEvidence {
                            kind: CapabilityEvidenceKind::DirectProbe,
                            outcome: CapabilityEvidenceOutcome::Positive,
                            subject: id.as_str().into(),
                            detail: "Raw daemon fixture".into(),
                            argv: vec!["bitbake".into(), "--help".into()],
                        }],
                    })
                    .collect(),
            },
            implementations: required
                .into_iter()
                .map(|id| {
                    (
                        id,
                        CapabilityImplementation {
                            id: format!("{}.fixture", id.as_str()),
                            kind: CapabilityImplementationKind::Command,
                        },
                    )
                })
                .collect(),
        }
        .normalize()
        .unwrap()
    }

    fn authority(fixture: &Fixture) -> DaemonCompatibilitySnapshot {
        authority_for(fixture, RawInteractionMode::NoninteractiveJob)
    }

    fn request(authority: &DaemonCompatibilitySnapshot, id: &str) -> RawExecutionRequestData {
        let catalog = builtin_raw_catalog();
        let command = catalog
            .commands
            .iter()
            .find(|command| {
                matches!(
                    command.execution,
                    RawExecutionPolicy::Executable { ref template }
                        if template.interaction == RawInteractionMode::NoninteractiveJob
                            && command.parameters.is_empty()
                )
            })
            .unwrap();
        let preview_request = RawPreviewRequest {
            catalog_version: catalog.version,
            command: command.id.clone(),
            parameters: BTreeMap::<_, RawParameterValue>::new(),
            additional_arguments: RawAdditionalArguments::default(),
            capability_generation: authority.snapshot.generation,
            build_directory: authority
                .snapshot
                .environment
                .build_directory
                .value()
                .unwrap()
                .clone(),
        };
        let preview = catalog.preview(&preview_request, Some(authority)).unwrap();
        let confirmed = yoctui_model::RawConfirmedExecutionRequest::from_reviewed_preview(
            RawRequestId::new(id).unwrap(),
            catalog,
            &preview_request,
            &preview,
        )
        .unwrap();
        yoctui_app::raw_execution_request_to_protocol(&confirmed).unwrap()
    }

    fn pty_request(authority: &DaemonCompatibilitySnapshot, id: &str) -> RawExecutionRequestData {
        let catalog = builtin_raw_catalog();
        let command = catalog
            .commands
            .iter()
            .find(|command| {
                matches!(
                    command.execution,
                    RawExecutionPolicy::Executable { ref template }
                        if template.interaction == RawInteractionMode::InteractivePty
                            && command.parameters.len() == 1
                )
            })
            .unwrap();
        let parameter = command.parameters.first().unwrap();
        let parameters = BTreeMap::from([(
            parameter.id.clone(),
            RawParameterValue::Target("core-image-minimal".into()),
        )]);
        let preview_request = RawPreviewRequest {
            catalog_version: catalog.version,
            command: command.id.clone(),
            parameters,
            additional_arguments: RawAdditionalArguments::default(),
            capability_generation: authority.snapshot.generation,
            build_directory: authority
                .snapshot
                .environment
                .build_directory
                .value()
                .unwrap()
                .clone(),
        };
        let preview = catalog.preview(&preview_request, Some(authority)).unwrap();
        let confirmed = yoctui_model::RawConfirmedExecutionRequest::from_reviewed_preview(
            RawRequestId::new(id).unwrap(),
            catalog,
            &preview_request,
            &preview,
        )
        .unwrap();
        yoctui_app::raw_execution_request_to_protocol(&confirmed).unwrap()
    }

    async fn next_event(supervisor: &mut DaemonRawSupervisor) -> DaemonRawEvent {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if let Some(event) = supervisor.try_event() {
                    return event;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap()
    }

    async fn next_pty_event(
        supervisor: &mut crate::daemon_pty::DaemonPtySupervisor,
    ) -> crate::daemon_pty::DaemonPtyEvent {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if let Some(event) = supervisor.try_event() {
                    return event;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn raw_pty_uses_authorized_native_argv_and_survives_detach_resize_and_input() {
        let fixture = Fixture::new(
            "pty",
            "#!/bin/sh\nprintf 'cwd=%s argv=%s\\n' \"$PWD\" \"$*\"\nIFS= read -r line\nstty size\nprintf 'input=%s\\n' \"$line\"\n",
        );
        let authority = authority_for(&fixture, RawInteractionMode::InteractivePty);
        let wire = pty_request(&authority, "raw-request:daemon-pty");
        let mut raw = DaemonRawSupervisor::default();
        raw.replace_compatibility(Some(authority)).unwrap();
        let start = raw.prepare_pty(wire.clone()).unwrap();
        assert_eq!(start.pty_id.0 >> 60, RAW_PTY_NAMESPACE >> 60);
        assert_eq!(start.command.executable(), fixture.executable());
        assert_eq!(
            start.command.arguments(),
            ["-u", "knotty", "core-image-minimal"]
        );
        assert_eq!(start.command.current_directory(), fixture.0);
        assert!(
            raw.prepare_pty(wire.clone()).is_ok(),
            "an unactivated authorization remains spawn-free and retryable"
        );

        let dimensions = yoctui_protocol::daemon::TerminalDimensions {
            columns: 90,
            rows: 30,
        };
        let client = yoctui_model::PtyClientId([41; 16]);
        let mut pty = crate::daemon_pty::DaemonPtySupervisor::default();
        pty.start_raw(start.pty_id, &start.command, dimensions)
            .unwrap();
        raw.activate_pty(&start).unwrap();
        assert!(matches!(
            raw.prepare_pty(wire),
            Err(DaemonRawError::DuplicateRequest(_))
        ));
        pty.attach(start.pty_id, client).unwrap();

        loop {
            match next_pty_event(&mut pty).await {
                crate::daemon_pty::DaemonPtyEvent::Started { session_id, .. } => {
                    assert_eq!(session_id, start.pty_id);
                    let state = raw.pty_started(session_id).unwrap().unwrap();
                    assert_eq!(state.phase, yoctui_model::RawExecutionPhase::Running);
                    break;
                }
                crate::daemon_pty::DaemonPtyEvent::Lost { message, .. } => panic!("{message}"),
                _ => {}
            }
        }
        let epoch = pty.take(start.pty_id, client, 0).unwrap();
        pty.resize(
            start.pty_id,
            client,
            epoch,
            yoctui_model::PtyDimensions {
                columns: 100,
                rows: 35,
            },
        )
        .unwrap();
        pty.detach(start.pty_id, client).unwrap();
        let detached = raw.pty_attachment(start.pty_id, false).unwrap().unwrap();
        assert_eq!(detached.attachment, RawAttachmentState::Detached);
        assert_eq!(detached.phase, yoctui_model::RawExecutionPhase::Running);
        pty.attach(start.pty_id, client).unwrap();
        let attached = raw.pty_attachment(start.pty_id, true).unwrap().unwrap();
        assert_eq!(attached.attachment, RawAttachmentState::Attached);
        let epoch = pty
            .take(start.pty_id, client, epoch.saturating_add(1))
            .unwrap();
        pty.input(start.pty_id, client, epoch, b"hello raw pty\n".to_vec())
            .unwrap();

        let mut output = Vec::new();
        let exit_code = loop {
            match next_pty_event(&mut pty).await {
                crate::daemon_pty::DaemonPtyEvent::Output { bytes, .. } => output.extend(bytes),
                crate::daemon_pty::DaemonPtyEvent::Exited { exit_code, .. } => break exit_code,
                crate::daemon_pty::DaemonPtyEvent::Lost { message, .. } => panic!("{message}"),
                _ => {}
            }
        };
        let text = String::from_utf8_lossy(&output);
        assert!(text.contains(&format!("cwd={}", fixture.0.display())));
        assert!(text.contains("argv=-u knotty core-image-minimal"));
        assert!(text.contains("35 100"));
        assert!(text.contains("input=hello raw pty"));
        let terminal = raw
            .pty_finished(start.pty_id, exit_code, None)
            .unwrap()
            .unwrap();
        assert_eq!(
            terminal.phase,
            yoctui_model::RawExecutionPhase::Terminal(RawExecutionOutcome::Succeeded)
        );
        assert!(terminal.stdout.chunks.is_empty());
        assert!(terminal.stderr.chunks.is_empty());
    }

    #[test]
    fn raw_pty_rejects_cross_route_stale_tampered_duplicate_and_tracks_cancel_and_loss() {
        let fixture = Fixture::new("pty-denial", "#!/bin/sh\nexit 99\n");
        let authority = authority_for(&fixture, RawInteractionMode::InteractivePty);
        let wire = pty_request(&authority, "raw-request:daemon-pty-denial");
        let mut raw = DaemonRawSupervisor::default();
        raw.replace_compatibility(Some(authority)).unwrap();

        assert!(matches!(
            raw.start(wire.clone()),
            Err(DaemonRawError::Planner(
                RawJobPlannerError::InteractiveRequest
            ))
        ));
        let mut tampered = wire.clone();
        let replacement = if tampered.preview_digest.starts_with("00") {
            "ff"
        } else {
            "00"
        };
        tampered.preview_digest.replace_range(0..2, replacement);
        assert!(raw.prepare_pty(tampered).is_err());
        let mut stale = wire.clone();
        stale.capability_generation += 1;
        assert!(raw.prepare_pty(stale).is_err());

        let start = raw.prepare_pty(wire.clone()).unwrap();
        let other = raw
            .prepare_pty(pty_request(
                &raw.compatibility.clone().unwrap(),
                "raw-request:daemon-pty-other",
            ))
            .unwrap();
        let mut mismatched = start.clone();
        mismatched.command = other.command;
        assert!(matches!(
            raw.activate_pty(&mismatched),
            Err(DaemonRawError::PtyIdentityMismatch)
        ));
        raw.activate_pty(&start).unwrap();
        assert!(matches!(
            raw.activate_pty(&start),
            Err(DaemonRawError::DuplicateRequest(_))
        ));
        raw.pty_started(start.pty_id).unwrap();
        let DaemonRawCancel::Pty { pty_id, state } = raw.cancel_pty(start.pty_id).unwrap().unwrap()
        else {
            panic!("expected Raw PTY cancellation target");
        };
        assert_eq!(pty_id, start.pty_id);
        assert_eq!(state.phase, yoctui_model::RawExecutionPhase::Cancelling);
        let cancelled = raw.pty_finished(start.pty_id, None, None).unwrap().unwrap();
        assert_eq!(
            cancelled.phase,
            yoctui_model::RawExecutionPhase::Terminal(RawExecutionOutcome::Cancelled)
        );
        let snapshot = DaemonSnapshot {
            daemon_instance_id: yoctui_protocol::daemon::DaemonInstanceId([7; 16]),
            sequence: 0,
            generation: 0,
            workspace: None,
            project_profile: yoctui_protocol::daemon::ProjectProfileSummary::NotLoaded,
            bitbake: yoctui_protocol::daemon::BitBakeState {
                lifecycle: LifecycleState::Disconnected,
                version: None,
                capabilities: Vec::new(),
                diagnostic: None,
            },
            compatibility: None,
            jobs: Vec::new(),
            raw_executions: vec![
                yoctui_app::raw_execution_snapshot_to_protocol(&cancelled).unwrap(),
            ],
            pty_sessions: Vec::new(),
            pty_screens: Vec::new(),
            clients: Vec::new(),
            recent_logs: Vec::new(),
            build_events: Vec::new(),
            recovery_warnings: Vec::new(),
        };
        let mut recovered = DaemonRawSupervisor::default();
        recovered.restore_snapshot(&snapshot).unwrap();
        recovered
            .replace_compatibility(raw.compatibility.clone())
            .unwrap();
        let recovered_start = recovered
            .prepare_pty(pty_request(
                &recovered.compatibility.clone().unwrap(),
                "raw-request:daemon-pty-recovered",
            ))
            .unwrap();
        assert_eq!(recovered_start.pty_id.0, RAW_PTY_NAMESPACE | 2);

        let lost_wire = pty_request(
            &raw.compatibility.clone().unwrap(),
            "raw-request:daemon-pty-lost",
        );
        let lost_start = raw.prepare_pty(lost_wire).unwrap();
        raw.activate_pty(&lost_start).unwrap();
        let lost = raw
            .pty_finished(lost_start.pty_id, None, Some("daemon restarted".into()))
            .unwrap()
            .unwrap();
        assert_eq!(
            lost.phase,
            yoctui_model::RawExecutionPhase::Terminal(RawExecutionOutcome::Lost)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn raw_pty_explicit_termination_is_required_and_reports_cancelled() {
        let fixture = Fixture::new(
            "pty-terminate",
            "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
        );
        let authority = authority_for(&fixture, RawInteractionMode::InteractivePty);
        let wire = pty_request(&authority, "raw-request:daemon-pty-terminate");
        let mut raw = DaemonRawSupervisor::default();
        raw.replace_compatibility(Some(authority)).unwrap();
        let start = raw.prepare_pty(wire).unwrap();
        let mut pty = crate::daemon_pty::DaemonPtySupervisor::default();
        pty.start_raw(
            start.pty_id,
            &start.command,
            yoctui_protocol::daemon::TerminalDimensions {
                columns: 80,
                rows: 24,
            },
        )
        .unwrap();
        raw.activate_pty(&start).unwrap();
        loop {
            if let crate::daemon_pty::DaemonPtyEvent::Started { session_id, .. } =
                next_pty_event(&mut pty).await
            {
                raw.pty_started(session_id).unwrap();
                break;
            }
        }

        let DaemonRawCancel::Pty { pty_id, state } =
            raw.cancel(start.state.request.id.as_str()).unwrap()
        else {
            panic!("expected PTY cancellation");
        };
        assert_eq!(state.phase, yoctui_model::RawExecutionPhase::Cancelling);
        pty.terminate(pty_id).unwrap();
        let exit_code = loop {
            match next_pty_event(&mut pty).await {
                crate::daemon_pty::DaemonPtyEvent::Exited { exit_code, .. } => break exit_code,
                crate::daemon_pty::DaemonPtyEvent::Lost { message, .. } => panic!("{message}"),
                _ => {}
            }
        };
        let terminal = raw.pty_finished(pty_id, exit_code, None).unwrap().unwrap();
        assert_eq!(
            terminal.phase,
            yoctui_model::RawExecutionPhase::Terminal(RawExecutionOutcome::Cancelled)
        );
    }

    #[tokio::test]
    async fn raw_output_job_attachment_is_ordered_idempotent_and_does_not_cancel() {
        let fixture = Fixture::new(
            "output-attachment",
            "#!/bin/sh\nprintf 'ready\\n'\nsleep 1\nexit 0\n",
        );
        let authority = authority(&fixture);
        let wire = request(&authority, "raw-request:daemon-output-attachment");
        let mut supervisor = DaemonRawSupervisor::default();
        supervisor.replace_compatibility(Some(authority)).unwrap();
        supervisor.start(wire.clone()).unwrap();
        loop {
            if next_event(&mut supervisor).await.state.phase
                == yoctui_model::RawExecutionPhase::Running
            {
                break;
            }
        }
        assert!(matches!(
            supervisor.set_attachment(&wire.request_id, RawAttachmentState::Detached),
            Ok(DaemonRawAttachment::Job)
        ));
        let detached = loop {
            let event = next_event(&mut supervisor).await;
            if event.state.attachment == RawAttachmentState::Detached {
                break event.state;
            }
        };
        assert_eq!(detached.phase, yoctui_model::RawExecutionPhase::Running);
        assert!(!detached.cancellation_requested);
        assert!(matches!(
            supervisor.set_attachment(&wire.request_id, RawAttachmentState::Detached),
            Err(DaemonRawError::AttachmentUnchanged(_))
        ));
        assert!(matches!(
            supervisor.set_attachment(&wire.request_id, RawAttachmentState::Attached),
            Ok(DaemonRawAttachment::Job)
        ));
        let attached = loop {
            let event = next_event(&mut supervisor).await;
            if event.state.attachment == RawAttachmentState::Attached {
                break event.state;
            }
        };
        assert_eq!(attached.phase, yoctui_model::RawExecutionPhase::Running);
        assert!(!attached.cancellation_requested);
    }

    #[tokio::test]
    async fn raw_job_supervisor_survives_detach_replays_output_and_rejects_duplicate() {
        let fixture = Fixture::new(
            "complete",
            "#!/bin/sh\nprintf 'detached output 界\\n'\nexit 0\n",
        );
        let authority = authority(&fixture);
        let wire = request(&authority, "raw-request:daemon-complete");
        let mut supervisor = DaemonRawSupervisor::default();
        supervisor.replace_compatibility(Some(authority)).unwrap();
        let started = supervisor.start(wire.clone()).unwrap();
        assert_eq!(started.state.phase, yoctui_model::RawExecutionPhase::Queued);
        assert!(matches!(
            supervisor.start(wire),
            Err(DaemonRawError::DuplicateRequest(_))
        ));

        // No connection owns or polls the worker during this interval.
        tokio::time::sleep(Duration::from_millis(20)).await;
        let terminal = loop {
            let event = next_event(&mut supervisor).await;
            if event.state.phase.is_terminal() {
                break event.state;
            }
        };
        assert_eq!(
            terminal.phase,
            yoctui_model::RawExecutionPhase::Terminal(RawExecutionOutcome::Succeeded)
        );
        assert!(
            terminal
                .stdout
                .chunks
                .iter()
                .any(|chunk| chunk.text == "detached output 界")
        );
        assert!(matches!(
            supervisor.cancel(terminal.request.id.as_str()),
            Err(DaemonRawError::UnknownRequest(_))
        ));
    }

    #[tokio::test]
    async fn raw_job_supervisor_journals_single_graceful_cancellation() {
        let fixture = Fixture::new(
            "cancel",
            "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
        );
        let authority = authority(&fixture);
        let wire = request(&authority, "raw-request:daemon-cancel");
        let mut supervisor = DaemonRawSupervisor::default()
            .with_timeouts(Duration::from_secs(2), Duration::from_secs(1));
        supervisor.replace_compatibility(Some(authority)).unwrap();
        supervisor.start(wire.clone()).unwrap();
        loop {
            let event = next_event(&mut supervisor).await;
            if event.state.phase == yoctui_model::RawExecutionPhase::Running
                && !event.state.stdout.chunks.is_empty()
            {
                break;
            }
        }
        supervisor.cancel(&wire.request_id).unwrap();
        assert!(matches!(
            supervisor.cancel(&wire.request_id),
            Err(DaemonRawError::CancellationAlreadyRequested(_))
        ));
        let mut saw_cancelling = false;
        loop {
            let event = next_event(&mut supervisor).await;
            saw_cancelling |= event.state.phase == yoctui_model::RawExecutionPhase::Cancelling;
            if event.state.phase.is_terminal() {
                assert_eq!(
                    event.state.phase,
                    yoctui_model::RawExecutionPhase::Terminal(RawExecutionOutcome::Cancelled)
                );
                assert!(saw_cancelling);
                break;
            }
        }
    }
}
