use yoctui_model::{PtySessionId, RawAttachmentState, RawExecutionEventKind, RawRequestId};

use super::{
    DaemonRawAttachment, DaemonRawCancel, DaemonRawError, DaemonRawEvent, DaemonRawSupervisor,
    RawJobControl, event_reduction::advance_sync,
};

impl DaemonRawSupervisor {
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
