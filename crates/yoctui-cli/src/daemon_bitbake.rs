use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    time::Duration,
};

use tokio::sync::mpsc;
use yoctui_bitbake::{BackendEvent, BitBakeBackend};
use yoctui_model::{BuildRequest, DaemonCompatibilitySnapshot};
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
    cancellation_terminal_tx: mpsc::UnboundedSender<DaemonBitBakeEvent>,
    cancellation_terminal_rx: mpsc::UnboundedReceiver<DaemonBitBakeEvent>,
    compatibility: Option<DaemonCompatibilitySnapshot>,
    bridge_environment: Option<BTreeMap<String, String>>,
}

impl Default for DaemonBitBakeSupervisor {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let (cancellation_terminal_tx, cancellation_terminal_rx) = mpsc::unbounded_channel();
        Self {
            next_job_id: 1,
            active: HashMap::new(),
            tx,
            rx,
            cancellation_terminal_tx,
            cancellation_terminal_rx,
            compatibility: None,
            bridge_environment: None,
        }
    }
}

impl DaemonBitBakeSupervisor {
    #[cfg(test)]
    fn with_bridge_environment(mut self, environment: BTreeMap<String, String>) -> Self {
        self.bridge_environment = Some(environment);
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
        let job_id = JobId(self.next_job_id);
        self.next_job_id = self.next_job_id.saturating_add(1);
        let (cancel_tx, mut cancel_rx) = mpsc::unbounded_channel();
        self.active.insert(job_id, cancel_tx);
        let tx = self.tx.clone();
        let cancellation_terminal_tx = self.cancellation_terminal_tx.clone();
        let bridge_environment = self.bridge_environment.clone();
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
                    let _ = tx.send(DaemonBitBakeEvent::Failed {
                        job_id,
                        message: format!("BitBake bridge could not be started: {error}"),
                    });
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
                    // A continuously ready event stream must never win over
                    // an already queued user cancellation.  The bridge also
                    // bounds each native-event poll, but this supervisor is
                    // the authority that guarantees command priority.
                    biased;
                    cancel = cancel_rx.recv() => {
                        if cancel.is_some() {
                            terminate_server = true;
                            if let Err(error) = backend.cancel_build().await {
                                let _ = backend.terminate_server().await;
                                backend_closed = true;
                                let _ = cancellation_terminal_tx.send(DaemonBitBakeEvent::Failed {
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
                                // A release-specific Tinfoil server can take
                                // an unbounded amount of time to acknowledge
                                // post-terminal cleanup.  The native terminal
                                // event remains authoritative, but cleanup is
                                // bounded before it is published so an older
                                // server cannot leave the shared job Running.
                                backend_closed = true;
                            }
                            let event = DaemonBitBakeEvent::Backend { job_id, event: Box::new(event) };
                            if terminal && terminate_server {
                                // Cancellation is a control-plane boundary.
                                // Deliver its terminal ahead of native/log
                                // records already queued before the request;
                                // try_event discards those stale records so
                                // they cannot resurrect the cancelled job.
                                let _ = cancellation_terminal_tx.send(event);
                            } else {
                                let _ = tx.send(event);
                            }
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
        if let Ok(event) = self.cancellation_terminal_rx.try_recv() {
            // Only one BitBake build may be active, so every queued regular
            // record belongs to the now-cancelled job and is stale by the
            // authoritative cancellation terminal.
            while self.rx.try_recv().is_ok() {}
            let id = match event {
                DaemonBitBakeEvent::Backend { job_id, .. }
                | DaemonBitBakeEvent::Failed { job_id, .. } => job_id,
            };
            self.active.remove(&id);
            return Some(event);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::Path,
        sync::atomic::{AtomicU64, Ordering},
        time::{Duration, Instant},
    };
    use yoctui_model::{
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityId, CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
        CapabilitySnapshot, CapabilityState, IdentityAuthority, YoctoEnvironmentIdentity,
    };

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    fn cancellation_authority(build: &Path) -> DaemonCompatibilitySnapshot {
        let capabilities = [
            CapabilityId::BitBakeWorkspaceInspection,
            CapabilityId::BitBakeRecipeInventory,
            CapabilityId::BitBakeLayerInventory,
            CapabilityId::BitBakeBuild,
            CapabilityId::BitBakeCancellation,
            CapabilityId::BitBakeNativeEvents,
            CapabilityId::BitBakeServerSocket,
        ];
        DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation: 1,
                environment: YoctoEnvironmentIdentity {
                    build_directory: AuthoritativeValue::detected(
                        build.to_path_buf(),
                        IdentityAuthority::InitializedEnvironment,
                    ),
                    bitbake_version: AuthoritativeValue::detected(
                        "2.18.0".into(),
                        IdentityAuthority::BitBakeVersionProbe,
                    ),
                    ..YoctoEnvironmentIdentity::default()
                },
                capabilities: capabilities
                    .into_iter()
                    .map(|id| CapabilityRecord {
                        id,
                        state: CapabilityState::Available,
                        evidence: vec![CapabilityEvidence {
                            kind: CapabilityEvidenceKind::BackendNegotiation,
                            outcome: CapabilityEvidenceOutcome::Positive,
                            subject: id.as_str().into(),
                            detail: "Fake initialized backend exposes the required operation."
                                .into(),
                            argv: Vec::new(),
                        }],
                    })
                    .collect(),
            },
            implementations: capabilities
                .into_iter()
                .map(|id| {
                    (
                        id,
                        CapabilityImplementation {
                            id: "tinfoil.adapter.modern".into(),
                            kind: CapabilityImplementationKind::BackendApi,
                        },
                    )
                })
                .collect(),
        }
        .normalize()
        .unwrap()
    }

    #[test]
    fn daemon_compatibility_runtime_bitbake_rejects_missing_authority_before_spawn() {
        let mut supervisor = DaemonBitBakeSupervisor::default();
        let error = supervisor
            .start(
                "/work/build".into(),
                BuildRequest {
                    targets: vec!["base-files".into()],
                    task: Some("listtasks".into()),
                    force: false,
                },
            )
            .unwrap_err();
        assert!(error.contains("requires current environment capability authority"));
    }

    #[tokio::test]
    async fn daemon_compatibility_cancellation_preempts_event_flood_and_terminates_once() {
        let root = std::env::temp_dir().join(format!(
            "yoctui-daemon-cancellation-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let build = root.join("build");
        let python = root.join("python");
        let marker = root.join("cancelled");
        fs::create_dir_all(&build).unwrap();
        fs::create_dir_all(&python).unwrap();
        let build = build.canonicalize().unwrap();
        fs::write(
            python.join("bb.py"),
            format!(
                r#"import time
__version__ = "2.18.0"
class Connection:
 native_event_stream = True
 def __init__(self): self.cancelled = False
 def inspect_workspace(self):
  return {{"build_dir": {build:?}, "source_dir": None, "variables": {{}}, "variable_provenance": {{}}, "variable_provenance_chain": {{}}, "bitbake_version": "2.18.0", "release": "6.0.2", "layers": [], "recipes": []}}
 def list_recipes(self, filter_value):
  return [{{"name": "busybox", "version": "1.36", "layer": "core", "preferred_version": None, "file": None, "append_count": 0}}]
 def list_layers(self):
  return [{{"name": "core", "path": "/layer", "priority": 5}}]
 def start_build(self, targets, task, force=False): pass
 def cancel_build(self):
  self.cancelled = True
  open({marker:?}, "w", encoding="utf-8").write("cancelled")
 def drain_events(self):
  def events():
   for index in range(10000):
    if self.cancelled:
     yield {{"type": "build_completed", "success": False, "exit_code": 1}}
     yield {{"type": "build_started"}}
     return
    yield {{"type": "parse_progress", "parsed": index, "total": 10000}}
  return events()
 def terminate_server(self): time.sleep(30)
 def shutdown(self): pass
class Server:
 def __init__(self): self.connection = Connection()
 def connect(self): return self.connection
server = Server()
"#,
                build = build.display().to_string(),
                marker = marker.display().to_string(),
            ),
        )
        .unwrap();

        let mut supervisor = DaemonBitBakeSupervisor::default().with_bridge_environment(
            BTreeMap::from([("PYTHONPATH".into(), python.display().to_string())]),
        );
        supervisor
            .replace_compatibility(Some(cancellation_authority(&build)))
            .unwrap();
        let job_id = supervisor
            .start(
                build,
                BuildRequest {
                    targets: vec!["base-files".into()],
                    task: None,
                    force: false,
                },
            )
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut cancellation_sent = false;
        let mut terminal_count = 0;
        let mut late_started = false;
        let mut saw_inventory = false;
        while Instant::now() < deadline && terminal_count == 0 {
            while let Some(event) = supervisor.try_event() {
                match event {
                    DaemonBitBakeEvent::Backend { event, .. } => match *event {
                        BackendEvent::Workspace(workspace) => {
                            assert_eq!(workspace.recipes.len(), 1);
                            assert_eq!(workspace.recipes[0].name, "busybox");
                            assert_eq!(workspace.layers.len(), 1);
                            assert_eq!(workspace.layers[0].name, "core");
                            saw_inventory = true;
                        }
                        BackendEvent::ParseProgress { .. } if !cancellation_sent => {
                            supervisor.cancel(job_id).unwrap();
                            cancellation_sent = true;
                        }
                        BackendEvent::BuildStarted if cancellation_sent => late_started = true,
                        BackendEvent::BuildCompleted { success, .. } => {
                            assert!(!success);
                            terminal_count += 1;
                        }
                        _ => {}
                    },
                    DaemonBitBakeEvent::Failed { message, .. } => panic!("{message}"),
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        assert!(cancellation_sent, "event stream never became active");
        assert!(saw_inventory, "daemon workspace omitted metadata inventory");
        assert_eq!(terminal_count, 1);
        assert!(!late_started);
        assert!(
            supervisor.try_event().is_none(),
            "pre-cancellation native records survived the terminal boundary"
        );
        assert_eq!(fs::read_to_string(&marker).unwrap(), "cancelled");
        assert!(supervisor.cancel(job_id).is_err());
        fs::remove_dir_all(&root).unwrap();
    }
}
