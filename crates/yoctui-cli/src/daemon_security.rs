use std::collections::HashMap;

use tokio::sync::mpsc;
use yoctui_bitbake::{
    SecurityMapperCommandSpec, SecurityMapperJobRunner, SecurityMapperRunnerEvent,
    SecurityReportAdapter, SecurityReportCancellation,
};
use yoctui_model::{SecurityOutputStream, SecurityReportRequest, SecuritySessionId};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonSecurityMapperEvent {
    Started {
        job_id: JobId,
        session_id: u64,
    },
    Output {
        job_id: JobId,
        session_id: u64,
        stream: SecurityOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        job_id: JobId,
        session_id: u64,
        exit_code: Option<i32>,
    },
    Failed {
        job_id: JobId,
        session_id: u64,
        exit_code: Option<i32>,
    },
    Cancelled {
        job_id: JobId,
        session_id: u64,
        exit_code: Option<i32>,
    },
    Lost {
        job_id: JobId,
        session_id: u64,
        message: String,
    },
}

pub struct DaemonSecurityMapperSupervisor {
    next_job_id: u64,
    active: HashMap<u64, mpsc::UnboundedSender<()>>,
    tx: mpsc::UnboundedSender<DaemonSecurityMapperEvent>,
    rx: mpsc::UnboundedReceiver<DaemonSecurityMapperEvent>,
}

impl Default for DaemonSecurityMapperSupervisor {
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

impl DaemonSecurityMapperSupervisor {
    pub fn start(
        &mut self,
        session_id: u64,
        executable: String,
        arguments: Vec<String>,
        report_roots: Vec<String>,
    ) -> Result<JobId, String> {
        if session_id == 0 || self.active.contains_key(&session_id) {
            return Err("security package-map session is already active or invalid".into());
        }
        let session = SecuritySessionId(session_id);
        let command = SecurityMapperCommandSpec::from_paths(
            session,
            executable.into(),
            arguments,
            report_roots.into_iter().map(Into::into).collect(),
        )
        .map_err(|error| error.to_string())?;
        let job_id = JobId(self.next_job_id);
        self.next_job_id = self.next_job_id.saturating_add(1);
        let (cancel_tx, mut cancel_rx) = mpsc::unbounded_channel();
        self.active.insert(session_id, cancel_tx);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut runner = SecurityMapperJobRunner::new();
            if let Err(error) = runner.start(command).await {
                let _ = tx.send(DaemonSecurityMapperEvent::Lost {
                    job_id,
                    session_id,
                    message: error.to_string(),
                });
                return;
            }
            loop {
                tokio::select! {
                    cancel = cancel_rx.recv() => { if cancel.is_some() { let _ = runner.cancel(session).await; } }
                    event = runner.next_event() => {
                        let mapped = match event {
                            Ok(SecurityMapperRunnerEvent::Started { .. }) => DaemonSecurityMapperEvent::Started { job_id, session_id },
                            Ok(SecurityMapperRunnerEvent::Output { stream, line, truncated, .. }) => DaemonSecurityMapperEvent::Output { job_id, session_id, stream, line, truncated },
                            Ok(SecurityMapperRunnerEvent::Completed { exit_code, .. }) => DaemonSecurityMapperEvent::Completed { job_id, session_id, exit_code },
                            Ok(SecurityMapperRunnerEvent::Failed { exit_code, .. }) | Ok(SecurityMapperRunnerEvent::TimedOut { exit_code, .. }) => DaemonSecurityMapperEvent::Failed { job_id, session_id, exit_code },
                            Ok(SecurityMapperRunnerEvent::Cancelled { exit_code, .. }) => DaemonSecurityMapperEvent::Cancelled { job_id, session_id, exit_code },
                            Ok(SecurityMapperRunnerEvent::CancellationRequested { .. }) => continue,
                            Ok(SecurityMapperRunnerEvent::CancellationRejected { message, .. }) | Ok(SecurityMapperRunnerEvent::Lost { message, .. }) => DaemonSecurityMapperEvent::Lost { job_id, session_id, message },
                            Err(error) => DaemonSecurityMapperEvent::Lost { job_id, session_id, message: error.to_string() },
                        };
                        let terminal = matches!(mapped, DaemonSecurityMapperEvent::Completed { .. } | DaemonSecurityMapperEvent::Failed { .. } | DaemonSecurityMapperEvent::Cancelled { .. } | DaemonSecurityMapperEvent::Lost { .. });
                        if tx.send(mapped).is_err() || terminal { break; }
                    }
                }
            }
        });
        Ok(job_id)
    }

    pub fn cancel(&mut self, session_id: u64) -> Result<(), String> {
        self.active
            .get(&session_id)
            .ok_or_else(|| format!("unknown security package-map session {session_id}"))?
            .send(())
            .map_err(|_| "security package-map worker is no longer active".into())
    }
    pub fn try_event(&mut self) -> Option<DaemonSecurityMapperEvent> {
        let event = self.rx.try_recv().ok()?;
        if matches!(
            event,
            DaemonSecurityMapperEvent::Completed { .. }
                | DaemonSecurityMapperEvent::Failed { .. }
                | DaemonSecurityMapperEvent::Cancelled { .. }
                | DaemonSecurityMapperEvent::Lost { .. }
        ) {
            let id = match &event {
                DaemonSecurityMapperEvent::Completed { session_id, .. }
                | DaemonSecurityMapperEvent::Failed { session_id, .. }
                | DaemonSecurityMapperEvent::Cancelled { session_id, .. }
                | DaemonSecurityMapperEvent::Lost { session_id, .. } => *session_id,
                _ => 0,
            };
            self.active.remove(&id);
        }
        Some(event)
    }
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

    #[test]
    fn client_runtime_security_mapper_rejects_invalid_session() {
        assert!(
            DaemonSecurityMapperSupervisor::default()
                .start(
                    0,
                    "/missing/cve-check-map-pkgs".into(),
                    vec!["/tmp/report".into()],
                    vec!["/tmp/report".into()],
                )
                .is_err()
        );
    }
}
