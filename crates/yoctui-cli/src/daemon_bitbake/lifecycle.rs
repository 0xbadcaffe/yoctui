use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

#[cfg(test)]
use std::collections::BTreeMap;

use tokio::sync::mpsc;
use yoctui_bitbake::{BackendEvent, BitBakeBackend};
use yoctui_model::{BuildRequest, DaemonCompatibilitySnapshot};
use yoctui_protocol::daemon::JobId;

use super::{
    BITBAKE_COSMETIC_EVENT_CAPACITY, BITBAKE_RELIABLE_EVENT_CAPACITY,
    DEFAULT_CANCELLATION_TERMINAL_TIMEOUT, DaemonBitBakeEvent, DaemonBitBakePressureShared,
    DaemonBitBakeSupervisor, ingress::send_bitbake_event, notification::ActivityNotification,
};

impl DaemonBitBakeSupervisor {
    pub fn new(job_ids: crate::daemon_job_ids::DaemonJobIds) -> Self {
        let (reliable_tx, reliable_rx) = mpsc::channel(BITBAKE_RELIABLE_EVENT_CAPACITY);
        let (cosmetic_tx, cosmetic_rx) = mpsc::channel(BITBAKE_COSMETIC_EVENT_CAPACITY);
        let (cancellation_terminal_tx, cancellation_terminal_rx) = mpsc::channel(1);
        Self {
            job_ids,
            active: HashMap::new(),
            reliable_tx,
            reliable_rx,
            cosmetic_tx,
            cosmetic_rx,
            cancellation_terminal_tx,
            cancellation_terminal_rx,
            post_cancellation_diagnostics: VecDeque::new(),
            pressure: Arc::new(DaemonBitBakePressureShared::default()),
            compatibility: None,
            bridge_environment: None,
            cancellation_terminal_timeout: DEFAULT_CANCELLATION_TERMINAL_TIMEOUT,
            activity: ActivityNotification::new().ok(),
        }
    }
}

impl DaemonBitBakeSupervisor {
    #[cfg(test)]
    pub(super) fn with_bridge_environment(mut self, environment: BTreeMap<String, String>) -> Self {
        self.bridge_environment = Some(environment);
        self
    }

    #[cfg(test)]
    pub(super) fn with_cancellation_terminal_timeout(mut self, timeout: Duration) -> Self {
        self.cancellation_terminal_timeout = timeout;
        self
    }

    pub fn replace_compatibility(
        &mut self,
        compatibility: Option<DaemonCompatibilitySnapshot>,
    ) -> Result<(), String> {
        self.compatibility = compatibility
            .map(DaemonCompatibilitySnapshot::normalize)
            .transpose()
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn start(&mut self, build_dir: PathBuf, request: BuildRequest) -> Result<JobId, String> {
        request.validate().map_err(|error| error.to_string())?;
        let compatibility = self.compatibility.clone().ok_or_else(|| {
            "daemon BitBake build requires current environment capability authority".to_owned()
        })?;
        if compatibility
            .snapshot
            .environment
            .build_directory
            .value()
            .map(PathBuf::as_path)
            != Some(build_dir.as_path())
        {
            return Err(
                "daemon BitBake build directory does not match capability authority".into(),
            );
        }
        if self.active.values().len() >= 1 {
            return Err("another daemon-owned BitBake build is already active".into());
        }
        let job_id = self.job_ids.allocate().map_err(str::to_owned)?;
        let (cancel_tx, mut cancel_rx) = mpsc::unbounded_channel();
        self.active.insert(job_id, cancel_tx);
        let reliable_tx = self.reliable_tx.clone();
        let cosmetic_tx = self.cosmetic_tx.clone();
        let pressure = Arc::clone(&self.pressure);
        let cancellation_terminal_tx = self.cancellation_terminal_tx.clone();
        let activity = self
            .activity
            .as_ref()
            .map(|notification| notification.sender.clone());
        let bridge_environment = self.bridge_environment.clone();
        let cancellation_terminal_timeout = self.cancellation_terminal_timeout;
        tokio::spawn(async move {
            let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".into());
            let mut backend = match crate::spawn_configured_bridge_with_compatibility(
                &python,
                build_dir,
                bridge_environment,
                compatibility,
            )
            .await
            {
                Ok(backend) => backend,
                Err(error) => {
                    send_bitbake_event(
                        &reliable_tx,
                        &cosmetic_tx,
                        &pressure,
                        activity.as_ref(),
                        DaemonBitBakeEvent::Failed {
                            job_id,
                            message: format!("BitBake bridge could not be started: {error}"),
                        },
                    )
                    .await;
                    return;
                }
            };
            match backend.inspect_workspace().await {
                Ok(mut workspace) => {
                    // The lightweight workspace response intentionally omits
                    // metadata collections.  Populate them through the same
                    // daemon-owned bridge before the build starts so attached
                    // clients retain authoritative Recipes and Layers views
                    // without opening a competing BitBake server.
                    match backend.list_recipes(None).await {
                        Ok(recipes) => workspace.recipes = recipes,
                        Err(error) => tracing::warn!(
                            %error,
                            "daemon BitBake recipe inventory is unavailable"
                        ),
                    }
                    match backend.list_layers().await {
                        Ok(layers) => workspace.layers = layers,
                        Err(error) => tracing::warn!(
                            %error,
                            "daemon BitBake layer inventory is unavailable"
                        ),
                    }
                    send_bitbake_event(
                        &reliable_tx,
                        &cosmetic_tx,
                        &pressure,
                        activity.as_ref(),
                        DaemonBitBakeEvent::Backend {
                            job_id,
                            event: Box::new(BackendEvent::Workspace(workspace)),
                        },
                    )
                    .await;
                }
                Err(error) => {
                    send_bitbake_event(
                        &reliable_tx,
                        &cosmetic_tx,
                        &pressure,
                        activity.as_ref(),
                        DaemonBitBakeEvent::Failed {
                            job_id,
                            message: format!("BitBake workspace could not be inspected: {error}"),
                        },
                    )
                    .await;
                    let _ = backend.shutdown().await;
                    return;
                }
            }
            if let Err(error) = backend.start_build(request).await {
                send_bitbake_event(
                    &reliable_tx,
                    &cosmetic_tx,
                    &pressure,
                    activity.as_ref(),
                    DaemonBitBakeEvent::Failed {
                        job_id,
                        message: format!("BitBake build could not be started: {error}"),
                    },
                )
                .await;
                let _ = backend.shutdown().await;
                return;
            }
            let mut terminate_server = false;
            let mut backend_closed = false;
            let cancellation_deadline = tokio::time::sleep(cancellation_terminal_timeout);
            tokio::pin!(cancellation_deadline);
            let mut cancellation_deadline_armed = false;
            loop {
                tokio::select! {
                    // A continuously ready event stream must never win over
                    // an already queued user cancellation.  The bridge also
                    // bounds each native-event poll, but this supervisor is
                    // the authority that guarantees command priority.
                    biased;
                    _ = &mut cancellation_deadline, if cancellation_deadline_armed => {
                        let sent = cancellation_terminal_tx.send(DaemonBitBakeEvent::Backend {
                            job_id,
                            event: Box::new(BackendEvent::BuildCompleted {
                                success: false,
                                exit_code: Some(130),
                            }),
                        }).await.is_ok();
                        if sent && let Some(activity) = &activity {
                            activity.signal();
                        }
                        // Terminal publication is a correctness boundary and
                        // must not wait behind release-specific Tinfoil/server
                        // cleanup on a saturated host.
                        let _ = tokio::time::timeout(
                            Duration::from_secs(2),
                            backend.terminate_server(),
                        )
                        .await;
                        backend_closed = true;
                        break;
                    }
                    cancel = cancel_rx.recv() => {
                        if cancel.is_some() {
                            terminate_server = true;
                            if let Err(error) = backend.cancel_build().await {
                                let _ = backend.terminate_server().await;
                                backend_closed = true;
                                let sent = cancellation_terminal_tx.send(DaemonBitBakeEvent::Failed {
                                    job_id,
                                    message: format!("BitBake cancellation failed: {error}"),
                                }).await.is_ok();
                                if sent && let Some(activity) = &activity {
                                    activity.signal();
                                }
                                break;
                            }
                            cancellation_deadline
                                .as_mut()
                                .reset(tokio::time::Instant::now() + cancellation_terminal_timeout);
                            cancellation_deadline_armed = true;
                        }
                    }
                    event = backend.next_event() => match event {
                        Ok(event) => {
                            let terminal = matches!(event, BackendEvent::BuildCompleted { .. } | BackendEvent::CommandFailed { .. } | BackendEvent::Disconnected);
                            if terminal {
                                let event = DaemonBitBakeEvent::Backend {
                                    job_id,
                                    event: Box::new(event),
                                };
                                if terminate_server {
                                    // Cancellation is a control-plane
                                    // boundary. Deliver its terminal ahead of
                                    // native/log records queued before the
                                    // request; try_event discards those stale
                                    // records so they cannot resurrect the
                                    // cancelled job.
                                    let sent = cancellation_terminal_tx.send(event).await.is_ok();
                                    if sent && let Some(activity) = &activity {
                                        activity.signal();
                                    }
                                } else {
                                    send_bitbake_event(
                                        &reliable_tx,
                                        &cosmetic_tx,
                                        &pressure,
                                        activity.as_ref(),
                                        event,
                                    )
                                    .await;
                                }

                                // Cleanup starts only after the critical event
                                // is queued. It remains bounded because a
                                // release-specific Tinfoil server can take an
                                // unbounded time to acknowledge shutdown.
                                if terminate_server {
                                    let _ = tokio::time::timeout(
                                        Duration::from_secs(2),
                                        backend.terminate_server(),
                                    )
                                    .await;
                                } else {
                                    let _ = tokio::time::timeout(
                                        Duration::from_secs(2),
                                        backend.shutdown(),
                                    )
                                    .await;
                                }
                                backend_closed = true;
                                break;
                            }
                            send_bitbake_event(
                                &reliable_tx,
                                &cosmetic_tx,
                                &pressure,
                                activity.as_ref(),
                                DaemonBitBakeEvent::Backend {
                                    job_id,
                                    event: Box::new(event),
                                },
                            )
                            .await;
                        }
                        Err(error) => {
                            send_bitbake_event(
                                &reliable_tx,
                                &cosmetic_tx,
                                &pressure,
                                activity.as_ref(),
                                DaemonBitBakeEvent::Failed { job_id, message: error.to_string() },
                            )
                            .await;
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
}
