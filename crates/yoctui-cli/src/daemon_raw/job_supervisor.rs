use tokio::sync::mpsc;
use yoctui_bitbake::{RawJobPlanner, RawJobRunner, RawJobRunnerEvent};
use yoctui_model::{
    RawAttachmentState, RawEventCursor, RawExecutionEventKind, RawExecutionOutcome,
    RawExecutionOwner, RawExecutionState, RawJobId, RawStreamId,
};
use yoctui_protocol::daemon::{JobId, LifecycleState, RawExecutionRequestData};
use yoctui_utils::unix_ms;

use super::{
    DaemonRawError, DaemonRawStart, DaemonRawSupervisor, RAW_JOB_NAMESPACE, RAW_JOB_SEQUENCE_LIMIT,
    RawJobControl,
    event_reduction::{advance_and_send, finish_and_send, lifecycle_for, request_cancellation},
};

impl DaemonRawSupervisor {
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
}
