use tokio::sync::mpsc;
use yoctui_model::{
    RawDurableReferenceId, RawExecutionEvent, RawExecutionEventKind, RawExecutionOutcome,
    RawExecutionResult, RawExecutionState, reduce_raw_execution,
};
use yoctui_protocol::daemon::{JobId, LifecycleState};
use yoctui_utils::unix_ms;

use super::{DaemonRawEvent, RAW_JOB_NAMESPACE};

pub(super) fn advance_sync(
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

pub(super) async fn request_cancellation(
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

pub(super) async fn finish_and_send(
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

pub(super) async fn advance_and_send(
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

pub(super) fn lifecycle_for(state: &RawExecutionState) -> LifecycleState {
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
