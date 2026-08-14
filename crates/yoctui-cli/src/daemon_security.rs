use std::collections::HashMap;

use tokio::sync::mpsc;
use yoctui_bitbake::{SecurityReportAdapter, SecurityReportCancellation};
use yoctui_model::SecurityReportRequest;
use yoctui_protocol::daemon::JobId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonSecurityEvent {
    Started {
        job_id: JobId,
        generation: u64,
    },
    Completed {
        job_id: JobId,
        generation: u64,
        reports: Vec<String>,
        limitations: Vec<String>,
    },
    Failed {
        job_id: JobId,
        generation: u64,
        message: String,
    },
    Cancelled {
        job_id: JobId,
        generation: u64,
    },
}

pub struct DaemonSecuritySupervisor {
    next_job_id: u64,
    active: HashMap<u64, mpsc::UnboundedSender<()>>,
    tx: mpsc::UnboundedSender<DaemonSecurityEvent>,
    rx: mpsc::UnboundedReceiver<DaemonSecurityEvent>,
}

impl Default for DaemonSecuritySupervisor {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            next_job_id: 1,
            active: HashMap::new(),
            tx,
            rx,
        }
    }
}

impl DaemonSecuritySupervisor {
    pub fn start(&mut self, generation: u64, paths: Vec<String>) -> Result<JobId, String> {
        if generation == 0 || self.active.contains_key(&generation) {
            return Err("security report generation is already active or invalid".into());
        }
        let request =
            SecurityReportRequest::new(generation, paths.into_iter().map(Into::into).collect())
                .map_err(str::to_owned)?;
        let job_id = JobId(self.next_job_id);
        self.next_job_id = self.next_job_id.saturating_add(1);
        let (cancel_tx, mut cancel_rx) = mpsc::unbounded_channel();
        self.active.insert(generation, cancel_tx);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(DaemonSecurityEvent::Started { job_id, generation });
            let cancellation = SecurityReportCancellation::default();
            let worker_cancel = cancellation.clone();
            let mut worker = tokio::spawn(async move {
                SecurityReportAdapter::new()
                    .scan_with_cancellation(request, worker_cancel)
                    .await
            });
            tokio::select! {
                cancel = cancel_rx.recv() => {
                    if cancel.is_some() { cancellation.cancel(); }
                    let _ = worker.await;
                    let _ = tx.send(DaemonSecurityEvent::Cancelled { job_id, generation });
                }
                result = &mut worker => match result {
                    Ok(Ok(response)) => {
                        let reports = response.outcome.reports().iter().map(|report| report.identity().path.display().to_string()).collect();
                        let limitations = response.outcome.limitations().to_vec();
                        let _ = tx.send(DaemonSecurityEvent::Completed { job_id, generation, reports, limitations });
                    }
                    Ok(Err(error)) => { let _ = tx.send(DaemonSecurityEvent::Failed { job_id, generation, message: error.to_string() }); }
                    Err(error) => { let _ = tx.send(DaemonSecurityEvent::Failed { job_id, generation, message: error.to_string() }); }
                }
            }
        });
        Ok(job_id)
    }

    pub fn cancel(&mut self, generation: u64) -> Result<(), String> {
        self.active
            .get(&generation)
            .ok_or_else(|| format!("unknown security report generation {generation}"))?
            .send(())
            .map_err(|_| "security report worker is no longer active".into())
    }

    pub fn try_event(&mut self) -> Option<DaemonSecurityEvent> {
        let event = self.rx.try_recv().ok()?;
        if matches!(
            event,
            DaemonSecurityEvent::Completed { .. }
                | DaemonSecurityEvent::Failed { .. }
                | DaemonSecurityEvent::Cancelled { .. }
        ) {
            let generation = match &event {
                DaemonSecurityEvent::Completed { generation, .. }
                | DaemonSecurityEvent::Failed { generation, .. }
                | DaemonSecurityEvent::Cancelled { generation, .. } => *generation,
                _ => 0,
            };
            self.active.remove(&generation);
        }
        Some(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_runtime_security_rejects_invalid_generation() {
        assert!(
            DaemonSecuritySupervisor::default()
                .start(0, vec!["/tmp/report.json".into()])
                .is_err()
        );
    }
}
