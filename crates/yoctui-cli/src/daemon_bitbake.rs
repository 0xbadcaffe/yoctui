use std::{collections::HashMap, path::PathBuf};

use tokio::sync::mpsc;
use yoctui_bitbake::{BackendEvent, BitBakeBackend};
use yoctui_model::BuildRequest;
use yoctui_protocol::daemon::JobId;

#[derive(Debug, Clone)]
pub enum DaemonBitBakeEvent {
    Backend {
        job_id: JobId,
        event: Box<BackendEvent>,
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
            let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".into());
            let mut backend = match crate::spawn_configured_bridge(&python, build_dir, None).await {
                Ok(backend) => backend,
                Err(error) => {
                    let _ = tx.send(DaemonBitBakeEvent::Failed {
                        job_id,
                        message: format!("BitBake bridge could not be started: {error}"),
                    });
                    return;
                }
            };
            match backend.inspect_workspace().await {
                Ok(workspace) => {
                    let _ = tx.send(DaemonBitBakeEvent::Backend {
                        job_id,
                        event: Box::new(BackendEvent::Workspace(workspace)),
                    });
                }
                Err(error) => {
                    let _ = tx.send(DaemonBitBakeEvent::Failed {
                        job_id,
                        message: format!("BitBake workspace could not be inspected: {error}"),
                    });
                    let _ = backend.shutdown().await;
                    return;
                }
            }
            if let Err(error) = backend.start_build(request).await {
                let _ = tx.send(DaemonBitBakeEvent::Failed {
                    job_id,
                    message: format!("BitBake build could not be started: {error}"),
                });
                let _ = backend.shutdown().await;
                return;
            }
            let mut terminate_server = false;
            let mut backend_closed = false;
            loop {
                tokio::select! {
                    cancel = cancel_rx.recv() => {
                        if cancel.is_some() {
                            terminate_server = true;
                            if let Err(error) = backend.cancel_build().await {
                                let _ = backend.terminate_server().await;
                                backend_closed = true;
                                let _ = tx.send(DaemonBitBakeEvent::Failed {
                                    job_id,
                                    message: format!("BitBake cancellation failed: {error}"),
                                });
                                break;
                            }
                        }
                    }
                    event = backend.next_event() => match event {
                        Ok(event) => {
                            let terminal = matches!(event, BackendEvent::BuildCompleted { .. } | BackendEvent::CommandFailed { .. } | BackendEvent::Disconnected);
                            if terminal {
                                if terminate_server {
                                    let _ = backend.terminate_server().await;
                                } else {
                                    let _ = backend.shutdown().await;
                                }
                                backend_closed = true;
                            }
                            let _ = tx.send(DaemonBitBakeEvent::Backend { job_id, event: Box::new(event) });
                            if terminal { break; }
                        }
                        Err(error) => {
                            let _ = tx.send(DaemonBitBakeEvent::Failed { job_id, message: error.to_string() });
                            break;
                        }
                    }
                }
            }
            if !backend_closed {
                let _ = backend.shutdown().await;
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
        let terminal = match &event {
            DaemonBitBakeEvent::Backend { event, .. } => matches!(
                event.as_ref(),
                BackendEvent::BuildCompleted { .. }
                    | BackendEvent::CommandFailed { .. }
                    | BackendEvent::Disconnected
            ),
            DaemonBitBakeEvent::Failed { .. } => true,
        };
        if terminal {
            let id = match event {
                DaemonBitBakeEvent::Backend { job_id, .. }
                | DaemonBitBakeEvent::Failed { job_id, .. } => job_id,
            };
            self.active.remove(&id);
        }
        Some(event)
    }
}
