use std::{
    collections::{HashMap, HashSet},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use thiserror::Error;
use tokio::sync::mpsc;
use yoctui_bitbake::{RawJobPlanner, RawJobPlannerError, RawJobRunner, RawJobRunnerEvent};
use yoctui_model::{
    DaemonCompatibilitySnapshot, RawDurableReferenceId, RawEventCursor, RawExecutionEvent,
    RawExecutionEventKind, RawExecutionOutcome, RawExecutionOwner, RawExecutionResult,
    RawExecutionState, RawJobId, RawRequestId, RawStreamId, reduce_raw_execution,
};
use yoctui_protocol::daemon::{
    DaemonSnapshot, JobId, JobKind, LifecycleState, RawExecutionRequestData,
};

const RAW_JOB_NAMESPACE: u64 = 5 << 60;
const RAW_JOB_SEQUENCE_LIMIT: u64 = 1 << 60;
const RAW_SUPERVISOR_EVENT_CAPACITY: usize = 64;

#[derive(Debug, Clone)]
pub struct DaemonRawStart {
    pub job_id: JobId,
    pub state: RawExecutionState,
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
    seen_requests: HashSet<RawRequestId>,
    active: HashMap<RawRequestId, mpsc::UnboundedSender<()>>,
    cancellation_requested: HashSet<RawRequestId>,
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
            seen_requests: HashSet::new(),
            active: HashMap::new(),
            cancellation_requested: HashSet::new(),
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
        }
        for job in snapshot.jobs.iter().filter(|job| job.kind == JobKind::Raw) {
            if job.id.0 >> 60 == RAW_JOB_NAMESPACE >> 60 {
                let sequence = job.id.0 & (RAW_JOB_SEQUENCE_LIMIT - 1);
                self.next_job_id = self.next_job_id.max(sequence.saturating_add(1));
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
        let (cancel_tx, mut cancel_rx) = mpsc::unbounded_channel();
        self.active.insert(request.id.clone(), cancel_tx);

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
            let mut cancellation_open = true;
            loop {
                tokio::select! {
                    cancellation = cancel_rx.recv(), if cancellation_open => {
                        if cancellation.is_none() {
                            cancellation_open = false;
                            if !request_cancellation(&events, job_id, &mut state).await {
                                return;
                            }
                        } else if !request_cancellation(&events, job_id, &mut state).await {
                            return;
                        }
                        match runner.cancel().await {
                            Ok(_) => {}
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

    pub fn cancel(&mut self, request_id: &str) -> Result<(), DaemonRawError> {
        let request_id = RawRequestId::new(request_id)?;
        if !self.active.contains_key(&request_id) {
            return Err(DaemonRawError::UnknownRequest(request_id));
        }
        if !self.cancellation_requested.insert(request_id.clone()) {
            return Err(DaemonRawError::CancellationAlreadyRequested(request_id));
        }
        self.active
            .get(&request_id)
            .expect("active request was checked")
            .send(())
            .map_err(|_| DaemonRawError::UnknownRequest(request_id))
    }

    pub fn try_event(&mut self) -> Option<DaemonRawEvent> {
        let event = self.events_rx.try_recv().ok()?;
        if event.state.phase.is_terminal() {
            self.active.remove(&event.state.request.id);
            self.cancellation_requested.remove(&event.state.request.id);
        }
        Some(event)
    }
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
    #[error("daemon Raw job ID space exhausted")]
    JobSpaceExhausted,
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

    fn authority(fixture: &Fixture) -> DaemonCompatibilitySnapshot {
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
