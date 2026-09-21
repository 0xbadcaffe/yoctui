use yoctui_bitbake::RawPtyPlanner;
use yoctui_model::{
    PtySessionId, RawAttachmentState, RawEventCursor, RawExecutionEventKind, RawExecutionOutcome,
    RawExecutionOwner, RawExecutionResult, RawExecutionState, RawSessionId, RawStreamId,
};
use yoctui_protocol::daemon::RawExecutionRequestData;
use yoctui_utils::unix_ms;

use super::{
    DaemonRawError, DaemonRawPtyStart, DaemonRawSupervisor, RAW_JOB_SEQUENCE_LIMIT,
    RAW_PTY_NAMESPACE, event_reduction::advance_sync,
};

impl DaemonRawSupervisor {
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
}
