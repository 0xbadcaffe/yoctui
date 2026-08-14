use std::{collections::HashMap, path::PathBuf};

use tokio::sync::mpsc;
use yoctui_bitbake::{BackendEvent, BitBakeBackend, ProcessBackend};
use yoctui_model::BuildRequest;
use yoctui_protocol::daemon::JobId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonBitBakeEvent {
    Started {
        job_id: JobId,
    },
    Log {
        job_id: JobId,
        message: String,
    },
    Completed {
        job_id: JobId,
        success: bool,
        exit_code: Option<i32>,
    },
    Failed {
        job_id: JobId,
        message: String,
    },
}

pub struct DaemonBitBakeSupervisor {
    next_job_id: u64,
    active: HashMap<JobId, mpsc::UnboundedSender<()>>,
    tx: mpsc::UnboundedSender<DaemonBitBakeEvent>,
    rx: mpsc::UnboundedReceiver<DaemonBitBakeEvent>,
}

impl Default for DaemonBitBakeSupervisor {
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

impl DaemonBitBakeSupervisor {
    pub fn start(&mut self, build_dir: PathBuf, request: BuildRequest) -> Result<JobId, String> {
        request.validate().map_err(|error| error.to_string())?;
        if self.active.values().len() >= 1 {
            return Err("another daemon-owned BitBake build is already active".into());
        }
        let job_id = JobId(self.next_job_id);
        self.next_job_id = self.next_job_id.saturating_add(1);
        let (cancel_tx, mut cancel_rx) = mpsc::unbounded_channel();
        self.active.insert(job_id, cancel_tx);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut backend = ProcessBackend::new(build_dir);
            if backend.start_build(request).await.is_err() {
                let _ = tx.send(DaemonBitBakeEvent::Failed {
                    job_id,
                    message: "BitBake build could not be started".into(),
                });
                return;
            }
            let _ = tx.send(DaemonBitBakeEvent::Started { job_id });
            loop {
                tokio::select! {
                    cancel = cancel_rx.recv() => {
                        if cancel.is_some() { let _ = backend.cancel_build().await; }
                    }
                    event = backend.next_event() => match event {
                        Ok(BackendEvent::Log(entry)) => { let _ = tx.send(DaemonBitBakeEvent::Log { job_id, message: entry.message }); }
                        Ok(BackendEvent::BuildCompleted { success, exit_code }) => {
                            let _ = tx.send(DaemonBitBakeEvent::Completed { job_id, success, exit_code });
                            break;
                        }
                        Ok(_) => {}
                        Err(error) => {
                            let _ = tx.send(DaemonBitBakeEvent::Failed { job_id, message: error.to_string() });
                            break;
                        }
                    }
                }
            }
        });
        Ok(job_id)
    }

    pub fn cancel(&mut self, job_id: JobId) -> Result<(), String> {
        self.active
            .get(&job_id)
            .ok_or_else(|| format!("unknown BitBake job {}", job_id.0))?
            .send(())
            .map_err(|_| "BitBake worker is no longer active".into())
    }

    pub fn try_event(&mut self) -> Option<DaemonBitBakeEvent> {
        let event = self.rx.try_recv().ok()?;
        if matches!(
            event,
            DaemonBitBakeEvent::Completed { .. } | DaemonBitBakeEvent::Failed { .. }
        ) {
            let id = match event {
                DaemonBitBakeEvent::Completed { job_id, .. }
                | DaemonBitBakeEvent::Failed { job_id, .. } => job_id,
                _ => unreachable!(),
            };
            self.active.remove(&id);
        }
        Some(event)
    }
}
