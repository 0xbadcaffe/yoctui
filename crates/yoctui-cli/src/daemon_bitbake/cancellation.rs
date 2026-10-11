use std::sync::atomic::Ordering;

use yoctui_bitbake::BackendEvent;
use yoctui_protocol::daemon::JobId;

use super::{
    BITBAKE_COSMETIC_EVENT_CAPACITY, BITBAKE_RELIABLE_EVENT_CAPACITY, DaemonBitBakeEvent,
    DaemonBitBakePressure, DaemonBitBakeSupervisor, ingress::bitbake_event_is_diagnostic,
};

impl DaemonBitBakeSupervisor {
    pub fn cancel(&mut self, job_id: JobId) -> Result<(), String> {
        self.active
            .get(&job_id)
            .ok_or_else(|| format!("unknown BitBake job {}", job_id.0))?
            .send(())
            .map_err(|_| "BitBake worker is no longer active".into())
    }

    pub fn try_event(&mut self) -> Option<DaemonBitBakeEvent> {
        if let Ok(event) = self.cancellation_terminal_rx.try_recv() {
            // Only one BitBake build may be active, so every queued regular
            // record belongs to the now-cancelled job and is stale by the
            // authoritative cancellation terminal.
            while let Ok(stale) = self.reliable_rx.try_recv() {
                if bitbake_event_is_diagnostic(&stale) {
                    if let Some(diagnostic) = cancellation_diagnostic(stale) {
                        self.post_cancellation_diagnostics.push_back(diagnostic);
                    }
                } else {
                    self.pressure
                        .cancellation_stale_discarded
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
            while self.cosmetic_rx.try_recv().is_ok() {
                self.pressure
                    .cancellation_stale_discarded
                    .fetch_add(1, Ordering::Relaxed);
            }
            let id = match event {
                DaemonBitBakeEvent::Backend { job_id, .. }
                | DaemonBitBakeEvent::Failed { job_id, .. }
                | DaemonBitBakeEvent::Cancelled { job_id, .. } => job_id,
            };
            self.active.remove(&id);
            return Some(event);
        }
        let event = self
            .post_cancellation_diagnostics
            .pop_front()
            .or_else(|| self.reliable_rx.try_recv().ok())
            .or_else(|| self.cosmetic_rx.try_recv().ok())?;
        let terminal = match &event {
            DaemonBitBakeEvent::Backend { event, .. } => matches!(
                event.as_ref(),
                BackendEvent::BuildCompleted { .. }
                    | BackendEvent::CommandFailed { .. }
                    | BackendEvent::Disconnected
            ),
            DaemonBitBakeEvent::Failed { .. } | DaemonBitBakeEvent::Cancelled { .. } => true,
        };
        if terminal {
            let id = match event {
                DaemonBitBakeEvent::Backend { job_id, .. }
                | DaemonBitBakeEvent::Failed { job_id, .. }
                | DaemonBitBakeEvent::Cancelled { job_id, .. } => job_id,
            };
            self.active.remove(&id);
        }
        Some(event)
    }

    pub fn pressure(&self) -> DaemonBitBakePressure {
        let current_queue_depth = (BITBAKE_RELIABLE_EVENT_CAPACITY - self.reliable_tx.capacity())
            .saturating_add(BITBAKE_COSMETIC_EVENT_CAPACITY - self.cosmetic_tx.capacity())
            .saturating_add(self.post_cancellation_diagnostics.len());
        DaemonBitBakePressure {
            reliable_enqueued: self.pressure.reliable_enqueued.load(Ordering::Relaxed),
            cosmetic_enqueued: self.pressure.cosmetic_enqueued.load(Ordering::Relaxed),
            cosmetic_dropped: self.pressure.cosmetic_dropped.load(Ordering::Relaxed),
            reliable_waits: self.pressure.reliable_waits.load(Ordering::Relaxed),
            cancellation_stale_discarded: self
                .pressure
                .cancellation_stale_discarded
                .load(Ordering::Relaxed),
            maximum_queue_depth: self.pressure.maximum_queue_depth.load(Ordering::Relaxed),
            current_queue_depth,
        }
    }
}

fn cancellation_diagnostic(event: DaemonBitBakeEvent) -> Option<DaemonBitBakeEvent> {
    let (job_id, message) = match event {
        DaemonBitBakeEvent::Backend { job_id, event } => match *event {
            BackendEvent::Log(entry) => {
                return Some(DaemonBitBakeEvent::Backend {
                    job_id,
                    event: Box::new(BackendEvent::Log(entry)),
                });
            }
            BackendEvent::CommandFailed { code, message } => (job_id, format!("{code}: {message}")),
            BackendEvent::Disconnected => return None,
            _ => return None,
        },
        DaemonBitBakeEvent::Failed { job_id, message } => (job_id, message),
        DaemonBitBakeEvent::Cancelled { .. } => return None,
    };
    // Preserve the diagnostic text, not a stale terminal that would overwrite
    // the already published authoritative cancellation and history outcome.
    Some(DaemonBitBakeEvent::Backend {
        job_id,
        event: Box::new(BackendEvent::Log(yoctui_model::LogEntry {
            id: 0,
            severity: yoctui_model::Severity::Error,
            message,
            recipe: None,
            task: None,
            path: None,
            build: None,
            timestamp: std::time::SystemTime::now(),
            protected: true,
            diagnostic: None,
        })),
    })
}
