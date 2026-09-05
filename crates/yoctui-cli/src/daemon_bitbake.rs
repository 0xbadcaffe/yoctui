use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};

use tokio::sync::mpsc;
use yoctui_bitbake::{BackendEvent, BitBakeBackend};
use yoctui_model::{BuildRequest, DaemonCompatibilitySnapshot};
use yoctui_protocol::daemon::JobId;

const DEFAULT_CANCELLATION_TERMINAL_TIMEOUT: Duration = Duration::from_secs(3);
const BITBAKE_RELIABLE_EVENT_CAPACITY: usize = 512;
const BITBAKE_COSMETIC_EVENT_CAPACITY: usize = 512;

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DaemonBitBakePressure {
    pub reliable_enqueued: u64,
    pub cosmetic_enqueued: u64,
    pub cosmetic_dropped: u64,
    pub reliable_waits: u64,
    pub cancellation_stale_discarded: u64,
    pub maximum_queue_depth: usize,
    pub current_queue_depth: usize,
}

#[derive(Debug, Default)]
struct DaemonBitBakePressureShared {
    reliable_enqueued: AtomicU64,
    cosmetic_enqueued: AtomicU64,
    cosmetic_dropped: AtomicU64,
    reliable_waits: AtomicU64,
    cancellation_stale_discarded: AtomicU64,
    maximum_queue_depth: AtomicUsize,
}

pub struct DaemonBitBakeSupervisor {
    next_job_id: u64,
    active: HashMap<JobId, mpsc::UnboundedSender<()>>,
    reliable_tx: mpsc::Sender<DaemonBitBakeEvent>,
    reliable_rx: mpsc::Receiver<DaemonBitBakeEvent>,
    cosmetic_tx: mpsc::Sender<DaemonBitBakeEvent>,
    cosmetic_rx: mpsc::Receiver<DaemonBitBakeEvent>,
    cancellation_terminal_tx: mpsc::Sender<DaemonBitBakeEvent>,
    cancellation_terminal_rx: mpsc::Receiver<DaemonBitBakeEvent>,
    post_cancellation_diagnostics: VecDeque<DaemonBitBakeEvent>,
    pressure: Arc<DaemonBitBakePressureShared>,
    compatibility: Option<DaemonCompatibilitySnapshot>,
    bridge_environment: Option<BTreeMap<String, String>>,
    cancellation_terminal_timeout: Duration,
}

impl Default for DaemonBitBakeSupervisor {
    fn default() -> Self {
        let (reliable_tx, reliable_rx) = mpsc::channel(BITBAKE_RELIABLE_EVENT_CAPACITY);
        let (cosmetic_tx, cosmetic_rx) = mpsc::channel(BITBAKE_COSMETIC_EVENT_CAPACITY);
        let (cancellation_terminal_tx, cancellation_terminal_rx) = mpsc::channel(1);
        Self {
            next_job_id: 1,
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
        }
    }
}

impl DaemonBitBakeSupervisor {
    #[cfg(test)]
    fn with_bridge_environment(mut self, environment: BTreeMap<String, String>) -> Self {
        self.bridge_environment = Some(environment);
        self
    }

    #[cfg(test)]
    fn with_cancellation_terminal_timeout(mut self, timeout: Duration) -> Self {
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
        let job_id = JobId(self.next_job_id);
        self.next_job_id = self.next_job_id.saturating_add(1);
        let (cancel_tx, mut cancel_rx) = mpsc::unbounded_channel();
        self.active.insert(job_id, cancel_tx);
        let reliable_tx = self.reliable_tx.clone();
        let cosmetic_tx = self.cosmetic_tx.clone();
        let pressure = Arc::clone(&self.pressure);
        let cancellation_terminal_tx = self.cancellation_terminal_tx.clone();
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
                        let _ = tokio::time::timeout(
                            Duration::from_secs(2),
                            backend.terminate_server(),
                        )
                        .await;
                        backend_closed = true;
                        let _ = cancellation_terminal_tx.send(DaemonBitBakeEvent::Backend {
                            job_id,
                            event: Box::new(BackendEvent::BuildCompleted {
                                success: false,
                                exit_code: Some(130),
                            }),
                        }).await;
                        break;
                    }
                    cancel = cancel_rx.recv() => {
                        if cancel.is_some() {
                            terminate_server = true;
                            if let Err(error) = backend.cancel_build().await {
                                let _ = backend.terminate_server().await;
                                backend_closed = true;
                                let _ = cancellation_terminal_tx.send(DaemonBitBakeEvent::Failed {
                                    job_id,
                                    message: format!("BitBake cancellation failed: {error}"),
                                }).await;
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
                                let _ = cancellation_terminal_tx.send(event).await;
                            } else {
                                send_bitbake_event(
                                    &reliable_tx,
                                    &cosmetic_tx,
                                    &pressure,
                                    event,
                                )
                                .await;
                            }
                            if terminal { break; }
                        }
                        Err(error) => {
                            send_bitbake_event(
                                &reliable_tx,
                                &cosmetic_tx,
                                &pressure,
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
                    self.post_cancellation_diagnostics.push_back(stale);
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
                | DaemonBitBakeEvent::Failed { job_id, .. } => job_id,
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

async fn send_bitbake_event(
    reliable: &mpsc::Sender<DaemonBitBakeEvent>,
    cosmetic: &mpsc::Sender<DaemonBitBakeEvent>,
    pressure: &DaemonBitBakePressureShared,
    event: DaemonBitBakeEvent,
) {
    if bitbake_event_is_cosmetic(&event) {
        match cosmetic.try_send(event) {
            Ok(()) => {
                pressure.cosmetic_enqueued.fetch_add(1, Ordering::Relaxed);
                record_queue_depth(reliable, cosmetic, pressure);
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                pressure.cosmetic_dropped.fetch_add(1, Ordering::Relaxed);
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {}
        }
    } else {
        if reliable.capacity() == 0 {
            pressure.reliable_waits.fetch_add(1, Ordering::Relaxed);
        }
        if reliable.send(event).await.is_ok() {
            pressure.reliable_enqueued.fetch_add(1, Ordering::Relaxed);
            record_queue_depth(reliable, cosmetic, pressure);
        }
    }
}

fn record_queue_depth(
    reliable: &mpsc::Sender<DaemonBitBakeEvent>,
    cosmetic: &mpsc::Sender<DaemonBitBakeEvent>,
    pressure: &DaemonBitBakePressureShared,
) {
    let depth = (reliable.max_capacity() - reliable.capacity())
        .saturating_add(cosmetic.max_capacity() - cosmetic.capacity());
    pressure
        .maximum_queue_depth
        .fetch_max(depth, Ordering::Relaxed);
}

fn bitbake_event_is_cosmetic(event: &DaemonBitBakeEvent) -> bool {
    matches!(
        event,
        DaemonBitBakeEvent::Backend { event, .. }
            if matches!(
                event.as_ref(),
                BackendEvent::ParseProgress { .. }
                    | BackendEvent::TaskProgress { .. }
                    | BackendEvent::Log(yoctui_model::LogEntry {
                        severity: yoctui_model::Severity::Trace | yoctui_model::Severity::Info,
                        ..
                    })
                    | BackendEvent::Ignored
            )
    )
}

fn bitbake_event_is_diagnostic(event: &DaemonBitBakeEvent) -> bool {
    matches!(event, DaemonBitBakeEvent::Failed { .. })
        || matches!(
            event,
            DaemonBitBakeEvent::Backend { event, .. }
                if matches!(
                    event.as_ref(),
                    BackendEvent::CommandFailed { .. }
                        | BackendEvent::Disconnected
                        | BackendEvent::Log(yoctui_model::LogEntry {
                            severity: yoctui_model::Severity::Warning
                                | yoctui_model::Severity::Error,
                            ..
                        })
                )
        )
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

    fn log_event(severity: yoctui_model::Severity, message: &str) -> DaemonBitBakeEvent {
        DaemonBitBakeEvent::Backend {
            job_id: JobId(1),
            event: Box::new(BackendEvent::Log(yoctui_model::LogEntry {
                id: 0,
                severity,
                message: message.into(),
                recipe: None,
                task: None,
                path: None,
                timestamp: std::time::SystemTime::UNIX_EPOCH,
                build: None,
                protected: false,
                diagnostic: None,
            })),
        }
    }

    #[tokio::test]
    async fn bounded_priority_ingress_drops_only_cosmetic_events() {
        let (reliable_tx, mut reliable_rx) = mpsc::channel(2);
        let (cosmetic_tx, cosmetic_rx) = mpsc::channel(2);
        let pressure = DaemonBitBakePressureShared::default();

        for message in ["ordinary-1", "ordinary-2", "ordinary-dropped"] {
            send_bitbake_event(
                &reliable_tx,
                &cosmetic_tx,
                &pressure,
                log_event(yoctui_model::Severity::Info, message),
            )
            .await;
        }
        send_bitbake_event(
            &reliable_tx,
            &cosmetic_tx,
            &pressure,
            log_event(yoctui_model::Severity::Warning, "warning-retained"),
        )
        .await;
        send_bitbake_event(
            &reliable_tx,
            &cosmetic_tx,
            &pressure,
            DaemonBitBakeEvent::Backend {
                job_id: JobId(1),
                event: Box::new(BackendEvent::BuildCompleted {
                    success: false,
                    exit_code: Some(1),
                }),
            },
        )
        .await;

        assert_eq!(pressure.cosmetic_dropped.load(Ordering::Relaxed), 1);
        assert_eq!(pressure.reliable_enqueued.load(Ordering::Relaxed), 2);
        assert_eq!(pressure.cosmetic_enqueued.load(Ordering::Relaxed), 2);
        assert_eq!(pressure.maximum_queue_depth.load(Ordering::Relaxed), 4);
        assert!(bitbake_event_is_diagnostic(
            &reliable_rx.try_recv().unwrap()
        ));
        assert!(matches!(
            reliable_rx.try_recv().unwrap(),
            DaemonBitBakeEvent::Backend { event, .. }
                if matches!(event.as_ref(), BackendEvent::BuildCompleted { .. })
        ));
        assert_eq!(cosmetic_rx.len(), 2);
    }

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

    #[tokio::test]
    async fn daemon_cancellation_times_out_to_one_terminal_event() {
        let root = std::env::temp_dir().join(format!(
            "yoctui-daemon-cancellation-timeout-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let build = root.join("build");
        let python = root.join("python");
        let cancelled = root.join("cancelled");
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
 def inspect_workspace(self):
  return {{"build_dir": {build:?}, "source_dir": None, "variables": {{}}, "variable_provenance": {{}}, "variable_provenance_chain": {{}}, "bitbake_version": "2.18.0", "release": "6.0.2", "layers": [], "recipes": []}}
 def list_recipes(self, filter_value): return []
 def list_layers(self): return []
 def start_build(self, targets, task, force=False): pass
 def cancel_build(self):
  open({cancelled:?}, "w", encoding="utf-8").write("cancelled")
  time.sleep(30)
 def drain_events(self): return [{{"type": "parse_progress", "current": 1, "total": 2}}]
 def terminate_server(self): pass
 def shutdown(self): pass
class Server:
 def connect(self): return Connection()
server = Server()
"#,
                build = build.display().to_string(),
                cancelled = cancelled.display().to_string(),
            ),
        )
        .unwrap();

        let mut supervisor = DaemonBitBakeSupervisor::default()
            .with_bridge_environment(BTreeMap::from([(
                "PYTHONPATH".into(),
                python.display().to_string(),
            )]))
            .with_cancellation_terminal_timeout(Duration::from_millis(40));
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

        let deadline = Instant::now() + Duration::from_secs(5);
        let mut sent = false;
        let mut terminals = 0;
        while Instant::now() < deadline && terminals == 0 {
            while let Some(event) = supervisor.try_event() {
                if matches!(
                    &event,
                    DaemonBitBakeEvent::Backend {
                        event,
                        ..
                    } if matches!(event.as_ref(), BackendEvent::ParseProgress { .. })
                ) && !sent
                {
                    supervisor.cancel(job_id).unwrap();
                    sent = true;
                } else if matches!(
                    &event,
                    DaemonBitBakeEvent::Backend {
                        event,
                        ..
                    } if matches!(event.as_ref(), BackendEvent::BuildCompleted {
                        success: false,
                        exit_code: Some(130)
                    })
                ) {
                    terminals += 1;
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        assert!(sent, "cancellation was never submitted");
        assert_eq!(terminals, 1);
        assert_eq!(fs::read_to_string(cancelled).unwrap(), "cancelled");
        assert!(supervisor.try_event().is_none());
        assert!(supervisor.cancel(job_id).is_err());
        fs::remove_dir_all(&root).unwrap();
    }
}
