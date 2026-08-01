use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use tokio::task::JoinHandle;
use yoctui_bitbake::{
    GitArchiveLocalResult, MaintenanceOptionalCapabilityInput,
    MaintenanceOptionalCapabilityInspector, MaintenanceReleaseCapabilityInput,
    MaintenanceReleaseCapabilityInspector, MaintenanceReleaseEvidenceSnapshot,
    MaintenanceServiceCapabilityInput, MaintenanceServiceCapabilityInspector,
    MaintenanceSstateCapabilityInput, MaintenanceSstateCapabilityInspector,
    MaintenanceSstateCommandSpec, MaintenanceSstateJobRunner, MaintenanceSstateRunnerEvent,
    build_compare_command, buildhistory_command, git_archive_local_command,
    git_archive_push_command, locked_signature_command, parse_cleanup_preview, pr_service_command,
};
use yoctui_model::{
    Action, App, BuildComparisonRequest, Effect, GitArchiveRequest, LockedSignatureCacheRequest,
    MAX_MAINTENANCE_PATHS, MaintenanceAction, MaintenanceCapabilitySnapshot, MaintenanceEffect,
    MaintenanceEvidence, MaintenanceFileIdentity, MaintenanceIntegrationsSnapshot,
    MaintenanceMetadata, MaintenanceOperation, MaintenanceOperationPreview, MaintenanceSessionId,
    MaintenanceTool, MaintenanceToolCapability, PrServiceOperation, ServiceDiagnostic,
    SstateCleanupRequest, SstateReadinessRequest, update,
};

const INSPECTION_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
struct MaintenanceCliContext {
    metadata: MaintenanceMetadata,
    search_path: Vec<PathBuf>,
    git_candidates: Vec<PathBuf>,
    repo_candidates: Vec<PathBuf>,
    toaster_candidates: Vec<PathBuf>,
}

impl MaintenanceCliContext {
    fn from_app(app: &App, build_dir: &Path, search_path: Vec<PathBuf>) -> Result<Self, String> {
        let build_dir = canonical_directory(build_dir)?;
        let variable_path = |name: &str| {
            app.workspace
                .variables
                .get(name)
                .and_then(|value| canonical_directory(Path::new(value)).ok())
        };
        let stamps_dirs = app
            .workspace
            .variables
            .get("STAMPS_DIR")
            .or_else(|| app.workspace.variables.get("STAMP"))
            .and_then(|value| canonical_directory(Path::new(value)).ok())
            .into_iter()
            .collect();
        let text = |name: &str| {
            app.workspace
                .variables
                .get(name)
                .filter(|value| !value.is_empty())
                .cloned()
        };
        let metadata = MaintenanceMetadata::new(MaintenanceMetadata {
            build_dir: Some(build_dir.clone()),
            sstate_dir: variable_path("SSTATE_DIR"),
            tmp_dir: variable_path("TMPDIR"),
            stamps_dirs,
            buildhistory_dir: variable_path("BUILDHISTORY_DIR"),
            prserv_host: text("PRSERV_HOST"),
            hashserve: text("BB_HASHSERVE"),
            hashserve_upstream: text("BB_HASHSERVE_UPSTREAM"),
            signature_handler: text("BB_SIGNATURE_HANDLER"),
            native_lsb: text("NATIVELSBSTRING"),
            machine: text("MACHINE"),
            distro: text("DISTRO"),
        })
        .map_err(str::to_owned)?;

        let mut roots = app
            .workspace
            .layers
            .iter()
            .filter_map(|layer| canonical_directory(&layer.path).ok())
            .collect::<Vec<_>>();
        if let Some(parent) = build_dir
            .parent()
            .and_then(|path| canonical_directory(path).ok())
        {
            roots.push(parent);
        }
        roots.sort();
        roots.dedup();
        let toaster_candidates = [build_dir.join("conf/toaster.conf")]
            .into_iter()
            .filter_map(|path| canonical_file(&path).ok())
            .collect();
        Ok(Self {
            metadata,
            search_path,
            git_candidates: roots.clone(),
            repo_candidates: roots,
            toaster_candidates,
        })
    }

    fn build_dir(&self) -> PathBuf {
        self.metadata
            .build_dir
            .clone()
            .expect("validated Maintenance context owns BUILDDIR")
    }
}

struct MaintenanceInspection {
    capability: Result<MaintenanceCapabilitySnapshot, String>,
    services: Result<(Vec<ServiceDiagnostic>, Vec<String>), String>,
    integrations: Result<MaintenanceIntegrationsSnapshot, String>,
}

#[derive(Clone)]
enum InspectionPurpose {
    Refresh(u64),
    Services(u64),
    Start {
        id: MaintenanceSessionId,
        preview: Box<MaintenanceOperationPreview>,
    },
    PreviewReadiness {
        capability_request: u64,
        request: Box<SstateReadinessRequest>,
    },
    PreviewCleanup {
        capability_request: u64,
        request: Box<SstateCleanupRequest>,
    },
    PreviewPrService {
        capability_request: u64,
        request: Box<yoctui_model::PrServiceRequest>,
    },
    PreviewLockedSignatureCache {
        capability_request: u64,
        request: Box<LockedSignatureCacheRequest>,
    },
    PreviewBuildHistoryComparison {
        capability_request: u64,
        request: Box<BuildComparisonRequest>,
    },
    PreviewGitArchive {
        capability_request: u64,
        request: Box<GitArchiveRequest>,
    },
    PreviewGitArchivePush {
        capability_request: u64,
        request: Box<GitArchiveRequest>,
    },
}

struct InspectionWorker {
    purpose: InspectionPurpose,
    deadline: tokio::time::Instant,
    handle: JoinHandle<MaintenanceInspection>,
}

#[derive(Clone)]
enum EvidencePlan {
    None,
    Release(MaintenanceReleaseEvidenceSnapshot),
    GitArchive(yoctui_model::GitArchiveRequest),
    PrExport(PathBuf),
}

struct CleanupPreviewStage {
    capability_request: u64,
    operation_id: u64,
    snapshot: MaintenanceCapabilitySnapshot,
    confirmed: yoctui_model::SstateCleanupPreview,
    stdout: Vec<String>,
}

enum OperationStage {
    Execute(Box<EvidencePlan>),
    CleanupPreview(Box<CleanupPreviewStage>),
}

struct MaintenanceCliOperation {
    id: MaintenanceSessionId,
    runner: Option<MaintenanceSstateJobRunner>,
    cancellation: Option<JoinHandle<(MaintenanceSstateJobRunner, Result<bool, String>)>>,
    stage: OperationStage,
}

struct MaintenanceCleanupPreviewOperation {
    id: MaintenanceSessionId,
    capability_request: u64,
    snapshot: MaintenanceCapabilitySnapshot,
    request: SstateCleanupRequest,
    runner: MaintenanceSstateJobRunner,
    stdout: Vec<String>,
    last_stderr: Option<String>,
}

pub(crate) struct MaintenanceCliCoordinator {
    context: MaintenanceCliContext,
    inspection: Option<InspectionWorker>,
    cleanup_preview: Option<MaintenanceCleanupPreviewOperation>,
    operation: Option<MaintenanceCliOperation>,
    snapshot: Option<MaintenanceCapabilitySnapshot>,
    local_archive: Option<GitArchiveLocalResult>,
    archive_intent: Option<(u64, GitArchiveRequest)>,
    deferred_archive_push: Option<GitArchiveRequest>,
    next_preview_id: u64,
}

impl MaintenanceCliCoordinator {
    pub(crate) fn new(
        app: &App,
        build_dir: &Path,
        search_path: Vec<PathBuf>,
    ) -> Result<Self, String> {
        Ok(Self {
            context: MaintenanceCliContext::from_app(app, build_dir, search_path)?,
            inspection: None,
            cleanup_preview: None,
            operation: None,
            snapshot: None,
            local_archive: None,
            archive_intent: None,
            deferred_archive_push: None,
            next_preview_id: 1,
        })
    }

    pub(crate) fn operation_active(&self) -> bool {
        self.cleanup_preview.is_some() || self.operation.is_some()
    }

    pub(crate) async fn shutdown(&mut self) {
        if let Some(worker) = self.inspection.take() {
            worker.handle.abort();
            let _ = worker.handle.await;
        }
        if let Some(mut active) = self.operation.take() {
            if let Some(wait) = active.cancellation.take() {
                wait.abort();
                let _ = wait.await;
            } else if let Some(mut runner) = active.runner.take() {
                let _ = runner.cancel(active.id).await;
            }
        }
        if let Some(mut preview) = self.cleanup_preview.take() {
            let _ = preview.runner.cancel(preview.id).await;
        }
    }

    pub(crate) async fn handle_effect(&mut self, app: &mut App, effect: Effect) -> bool {
        let Effect::Maintenance(effect) = effect else {
            return false;
        };
        match effect {
            MaintenanceEffect::InspectCapability { request } => {
                self.start_inspection(InspectionPurpose::Refresh(request));
            }
            MaintenanceEffect::InspectServices { request } => {
                self.start_inspection(InspectionPurpose::Services(request));
            }
            MaintenanceEffect::PreviewReadiness {
                capability_request,
                request,
            } => self.start_inspection(InspectionPurpose::PreviewReadiness {
                capability_request,
                request: Box::new(request),
            }),
            MaintenanceEffect::PreviewCleanup {
                capability_request,
                request,
            } => {
                if self.cleanup_preview.is_some() || self.operation.is_some() {
                    app.notification =
                        Some("another Maintenance operation is already active".into());
                } else {
                    self.start_inspection(InspectionPurpose::PreviewCleanup {
                        capability_request,
                        request: Box::new(request),
                    });
                }
            }
            MaintenanceEffect::PreviewPrService {
                capability_request,
                request,
            } => self.start_inspection(InspectionPurpose::PreviewPrService {
                capability_request,
                request: Box::new(request),
            }),
            MaintenanceEffect::StartOperation { id, preview } => {
                if self.operation.is_some() {
                    fail(app, id, "another Maintenance operation is already active");
                } else {
                    self.start_inspection(InspectionPurpose::Start { id, preview });
                }
            }
            MaintenanceEffect::CancelOperation(id) => self.cancel(app, id),
            MaintenanceEffect::PreviewLockedSignatureCache {
                capability_request,
                request,
            } => self.start_inspection(InspectionPurpose::PreviewLockedSignatureCache {
                capability_request,
                request: Box::new(request),
            }),
            MaintenanceEffect::PreviewBuildHistoryComparison {
                capability_request,
                request,
            } => self.start_inspection(InspectionPurpose::PreviewBuildHistoryComparison {
                capability_request,
                request: Box::new(request),
            }),
            MaintenanceEffect::PreviewGitArchive {
                capability_request,
                request,
            } => self.start_inspection(InspectionPurpose::PreviewGitArchive {
                capability_request,
                request: Box::new(request),
            }),
            MaintenanceEffect::OpenEvidence(_) | MaintenanceEffect::Navigate(_) => return false,
        }
        true
    }

    fn start_inspection(&mut self, purpose: InspectionPurpose) {
        if let Some(worker) = self.inspection.take() {
            worker.handle.abort();
        }
        let context = self.context.clone();
        self.inspection = Some(InspectionWorker {
            purpose,
            deadline: tokio::time::Instant::now() + INSPECTION_TIMEOUT,
            handle: tokio::task::spawn_blocking(move || inspect(context)),
        });
    }

    fn cancel(&mut self, app: &mut App, id: MaintenanceSessionId) {
        let Some(active) = self.operation.as_mut() else {
            reject_cancellation(app, id, "no Maintenance operation is active");
            return;
        };
        if active.id != id || active.cancellation.is_some() {
            reject_cancellation(app, id, "the exact Maintenance session is not cancellable");
            return;
        }
        let Some(mut runner) = active.runner.take() else {
            reject_cancellation(app, id, "Maintenance cancellation is already in progress");
            return;
        };
        active.cancellation = Some(tokio::spawn(async move {
            let result = runner.cancel(id).await.map_err(|error| error.to_string());
            (runner, result)
        }));
    }

    pub(crate) async fn poll(&mut self, app: &mut App) {
        self.poll_inspection(app).await;
        self.poll_cleanup_preview(app).await;
        self.poll_operation(app).await;
    }

    async fn poll_inspection(&mut self, app: &mut App) {
        let Some(worker) = self.inspection.as_ref() else {
            return;
        };
        if !worker.handle.is_finished() && tokio::time::Instant::now() < worker.deadline {
            return;
        }
        let worker = self
            .inspection
            .take()
            .expect("inspection worker was checked");
        if !worker.handle.is_finished() {
            worker.handle.abort();
            self.inspection_failed(
                app,
                worker.purpose,
                "Maintenance inspection timed out".into(),
            );
            return;
        }
        match worker.handle.await {
            Ok(result) => self.finish_inspection(app, worker.purpose, result).await,
            Err(error) => self.inspection_failed(
                app,
                worker.purpose,
                format!("Maintenance inspection worker was lost: {error}"),
            ),
        }
    }

    fn inspection_failed(&mut self, app: &mut App, purpose: InspectionPurpose, message: String) {
        match purpose {
            InspectionPurpose::Refresh(request) => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::CapabilityFailed {
                        request,
                        message: message.clone(),
                    }),
                );
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::ServicesFailed {
                        request,
                        message: message.clone(),
                    }),
                );
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::IntegrationsFailed { request, message }),
                );
            }
            InspectionPurpose::Services(request) => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::ServicesFailed { request, message }),
                );
            }
            InspectionPurpose::Start { id, .. } => fail(app, id, &message),
            InspectionPurpose::PreviewReadiness { .. }
            | InspectionPurpose::PreviewCleanup { .. }
            | InspectionPurpose::PreviewPrService { .. }
            | InspectionPurpose::PreviewLockedSignatureCache { .. }
            | InspectionPurpose::PreviewBuildHistoryComparison { .. }
            | InspectionPurpose::PreviewGitArchive { .. }
            | InspectionPurpose::PreviewGitArchivePush { .. } => {
                app.notification = Some(message);
            }
        }
    }

    async fn finish_inspection(
        &mut self,
        app: &mut App,
        purpose: InspectionPurpose,
        result: MaintenanceInspection,
    ) {
        match purpose {
            InspectionPurpose::Refresh(request) => {
                match result.capability {
                    Ok(snapshot) => {
                        let partial = !snapshot.limitations.is_empty()
                            || snapshot.tools.iter().any(|tool| {
                                matches!(tool, MaintenanceToolCapability::Unavailable { .. })
                            });
                        self.snapshot = Some(snapshot.clone());
                        let _ = update(
                            app,
                            Action::Maintenance(MaintenanceAction::CapabilityLoaded {
                                request,
                                snapshot,
                                partial,
                            }),
                        );
                    }
                    Err(message) => {
                        let _ = update(
                            app,
                            Action::Maintenance(MaintenanceAction::CapabilityFailed {
                                request,
                                message,
                            }),
                        );
                    }
                }
                apply_services(app, request, result.services);
                apply_integrations(app, request, result.integrations);
            }
            InspectionPurpose::Services(request) => apply_services(app, request, result.services),
            InspectionPurpose::Start { id, preview } => match result.capability {
                Ok(snapshot) => self.begin_operation(app, id, *preview, snapshot).await,
                Err(message) => fail(app, id, &message),
            },
            InspectionPurpose::PreviewReadiness {
                capability_request,
                request,
            } => match result.capability {
                Ok(snapshot) => self.preview_readiness(app, capability_request, *request, snapshot),
                Err(message) => app.notification = Some(message),
            },
            InspectionPurpose::PreviewCleanup {
                capability_request,
                request,
            } => match result.capability {
                Ok(snapshot) => {
                    self.begin_cleanup_preview(app, capability_request, *request, snapshot)
                        .await;
                }
                Err(message) => app.notification = Some(message),
            },
            InspectionPurpose::PreviewPrService {
                capability_request,
                request,
            } => match result.capability {
                Ok(snapshot) => {
                    self.preview_pr_service(app, capability_request, *request, snapshot)
                }
                Err(message) => app.notification = Some(message),
            },
            InspectionPurpose::PreviewLockedSignatureCache {
                capability_request,
                request,
            } => match result.capability {
                Ok(snapshot) => {
                    self.preview_locked_signature_cache(app, capability_request, *request, snapshot)
                }
                Err(message) => app.notification = Some(message),
            },
            InspectionPurpose::PreviewBuildHistoryComparison {
                capability_request,
                request,
            } => match result.capability {
                Ok(snapshot) => {
                    self.preview_buildhistory(app, capability_request, *request, snapshot)
                }
                Err(message) => app.notification = Some(message),
            },
            InspectionPurpose::PreviewGitArchive {
                capability_request,
                request,
            } => match result.capability {
                Ok(snapshot) => {
                    self.preview_git_archive(app, capability_request, *request, snapshot)
                }
                Err(message) => app.notification = Some(message),
            },
            InspectionPurpose::PreviewGitArchivePush {
                capability_request,
                request,
            } => match result.capability {
                Ok(snapshot) => {
                    self.preview_git_archive_push(app, capability_request, *request, snapshot)
                }
                Err(message) => app.notification = Some(message),
            },
        }
    }

    fn next_id(&mut self) -> MaintenanceSessionId {
        let id = MaintenanceSessionId(self.next_preview_id);
        self.next_preview_id = self.next_preview_id.wrapping_add(1).max(1);
        id
    }

    fn exact_snapshot(
        &self,
        app: &App,
        capability_request: u64,
        fresh: &MaintenanceCapabilitySnapshot,
    ) -> bool {
        app.maintenance.capability.request() == Some(capability_request)
            && self.snapshot.as_ref() == Some(fresh)
    }

    fn preview_readiness(
        &mut self,
        app: &mut App,
        capability_request: u64,
        request: SstateReadinessRequest,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if !self.exact_snapshot(app, capability_request, &snapshot) {
            app.notification = Some(
                "Maintenance capability changed; refresh and reopen the readiness form".into(),
            );
            return;
        }
        let id = self.next_id();
        match MaintenanceSstateCommandSpec::readiness(
            id,
            capability_request,
            &snapshot,
            id.0,
            request,
        ) {
            Ok((preview, _)) => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::BeginOperation(preview)),
                );
            }
            Err(error) => app.notification = Some(error.to_string()),
        }
    }

    fn preview_pr_service(
        &mut self,
        app: &mut App,
        capability_request: u64,
        request: yoctui_model::PrServiceRequest,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if !self.exact_snapshot(app, capability_request, &snapshot) {
            app.notification = Some(
                "Maintenance capability changed; refresh and reopen the PR service form".into(),
            );
            return;
        }
        let id = self.next_id();
        match pr_service_command(id, capability_request, &snapshot, id.0, request) {
            Ok((preview, _)) => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::BeginOperation(preview)),
                );
            }
            Err(error) => app.notification = Some(error.to_string()),
        }
    }

    fn preview_locked_signature_cache(
        &mut self,
        app: &mut App,
        capability_request: u64,
        request: LockedSignatureCacheRequest,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if !self.exact_snapshot(app, capability_request, &snapshot) {
            app.notification = Some(
                "Maintenance capability changed; refresh and reopen the locked-cache form".into(),
            );
            return;
        }
        let id = self.next_id();
        match locked_signature_command(id, capability_request, &snapshot, id.0, request) {
            Ok((preview, _, _)) => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::BeginOperation(preview)),
                );
            }
            Err(error) => app.notification = Some(error.to_string()),
        }
    }

    fn preview_buildhistory(
        &mut self,
        app: &mut App,
        capability_request: u64,
        request: BuildComparisonRequest,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if !self.exact_snapshot(app, capability_request, &snapshot) {
            app.notification = Some(
                "Maintenance capability changed; refresh and reopen the build-history form".into(),
            );
            return;
        }
        let id = self.next_id();
        match buildhistory_command(id, capability_request, &snapshot, id.0, request) {
            Ok((preview, _)) => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::BeginOperation(preview)),
                );
            }
            Err(error) => app.notification = Some(error.to_string()),
        }
    }

    fn preview_git_archive(
        &mut self,
        app: &mut App,
        capability_request: u64,
        request: GitArchiveRequest,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if !self.exact_snapshot(app, capability_request, &snapshot) {
            app.notification =
                Some("Maintenance capability changed; refresh and reopen the archive form".into());
            return;
        }
        let id = self.next_id();
        match git_archive_local_command(id, capability_request, &snapshot, id.0, &request) {
            Ok((preview, _)) => {
                self.archive_intent = Some((preview.id, request));
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::BeginOperation(preview)),
                );
            }
            Err(error) => app.notification = Some(error.to_string()),
        }
    }

    fn preview_git_archive_push(
        &mut self,
        app: &mut App,
        capability_request: u64,
        request: GitArchiveRequest,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if !self.exact_snapshot(app, capability_request, &snapshot) {
            app.notification = Some(
                "Maintenance capability changed; refresh before confirming archive push".into(),
            );
            return;
        }
        let Some(local) = self.local_archive.clone() else {
            app.notification = Some("local Git archive evidence is unavailable".into());
            return;
        };
        let id = self.next_id();
        match git_archive_push_command(id, capability_request, &snapshot, id.0, request, &local) {
            Ok((preview, _)) => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::BeginOperation(preview)),
                );
            }
            Err(error) => app.notification = Some(error.to_string()),
        }
    }

    async fn begin_cleanup_preview(
        &mut self,
        app: &mut App,
        capability_request: u64,
        request: SstateCleanupRequest,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if !self.exact_snapshot(app, capability_request, &snapshot) {
            app.notification =
                Some("Maintenance capability changed; refresh and reopen the cleanup form".into());
            return;
        }
        let id = self.next_id();
        let command =
            match MaintenanceSstateCommandSpec::cleanup_preview(id, &snapshot, request.clone()) {
                Ok(command) => command,
                Err(error) => {
                    app.notification = Some(error.to_string());
                    return;
                }
            };
        let mut runner = MaintenanceSstateJobRunner::new();
        if let Err(error) = runner.start(command).await {
            app.notification = Some(error.to_string());
            return;
        }
        self.cleanup_preview = Some(MaintenanceCleanupPreviewOperation {
            id,
            capability_request,
            snapshot,
            request,
            runner,
            stdout: Vec::new(),
            last_stderr: None,
        });
    }

    async fn poll_cleanup_preview(&mut self, app: &mut App) {
        let Some(active) = self.cleanup_preview.as_mut() else {
            return;
        };
        let event = match tokio::time::timeout(Duration::from_millis(1), active.runner.next_event())
            .await
        {
            Ok(Ok(event)) => event,
            Ok(Err(error)) => {
                app.notification = Some(format!("sstate cleanup preview was lost: {error}"));
                self.cleanup_preview = None;
                return;
            }
            Err(_) => return,
        };
        match event {
            MaintenanceSstateRunnerEvent::Started { .. }
            | MaintenanceSstateRunnerEvent::CancellationRequested { .. } => {}
            MaintenanceSstateRunnerEvent::Output {
                stream,
                line,
                truncated,
                ..
            } => match stream {
                yoctui_model::MaintenanceOutputStream::Stdout
                    if !truncated && active.stdout.len() <= MAX_MAINTENANCE_PATHS =>
                {
                    active.stdout.push(line);
                }
                yoctui_model::MaintenanceOutputStream::Stderr => {
                    active.last_stderr = Some(line);
                }
                _ => {}
            },
            MaintenanceSstateRunnerEvent::Completed { .. } => {
                let active = self.cleanup_preview.take().expect("preview was checked");
                if !self.exact_snapshot(app, active.capability_request, &active.snapshot) {
                    app.notification = Some(
                        "Maintenance capability changed during cleanup discovery; reopen the form"
                            .into(),
                    );
                    return;
                }
                let preview =
                    parse_cleanup_preview(active.request, &active.stdout).and_then(|fresh| {
                        MaintenanceSstateCommandSpec::cleanup_execution(
                            active.id,
                            active.capability_request,
                            &active.snapshot,
                            active.id.0,
                            &fresh,
                            &fresh,
                        )
                        .map(|(preview, _)| preview)
                    });
                match preview {
                    Ok(preview) => {
                        app.notification = None;
                        let _ = update(
                            app,
                            Action::Maintenance(MaintenanceAction::BeginOperation(preview)),
                        );
                    }
                    Err(error) => app.notification = Some(error.to_string()),
                }
            }
            MaintenanceSstateRunnerEvent::Failed { exit_code, .. } => {
                let stderr = active
                    .last_stderr
                    .clone()
                    .unwrap_or_else(|| "no stderr".into());
                app.notification = Some(format!(
                    "sstate cleanup preview failed with status {exit_code:?}: {stderr}"
                ));
                self.cleanup_preview = None;
            }
            MaintenanceSstateRunnerEvent::Cancelled { .. } => {
                app.notification = Some("sstate cleanup preview cancelled".into());
                self.cleanup_preview = None;
            }
            MaintenanceSstateRunnerEvent::CancellationRejected { message, .. }
            | MaintenanceSstateRunnerEvent::Lost { message, .. } => {
                app.notification = Some(message);
                self.cleanup_preview = None;
            }
            MaintenanceSstateRunnerEvent::TimedOut { .. } => {
                app.notification = Some("sstate cleanup preview timed out".into());
                self.cleanup_preview = None;
            }
        }
    }

    async fn begin_operation(
        &mut self,
        app: &mut App,
        id: MaintenanceSessionId,
        confirmed: MaintenanceOperationPreview,
        snapshot: MaintenanceCapabilitySnapshot,
    ) {
        if confirmed.capability_request == 0
            || self
                .snapshot
                .as_ref()
                .is_none_or(|cached| cached != &snapshot)
        {
            fail(
                app,
                id,
                "Maintenance capability changed; refresh and confirm again",
            );
            return;
        }
        let operation = confirmed.operation.clone();
        let built = match operation {
            MaintenanceOperation::SstateReadiness(request) => {
                MaintenanceSstateCommandSpec::readiness(
                    id,
                    confirmed.capability_request,
                    &snapshot,
                    confirmed.id,
                    request,
                )
                .map(|(preview, command)| {
                    (
                        preview,
                        command,
                        OperationStage::Execute(Box::new(EvidencePlan::None)),
                    )
                })
                .map_err(|error| error.to_string())
            }
            MaintenanceOperation::SstateCleanup(preview) => {
                MaintenanceSstateCommandSpec::cleanup_preview(
                    id,
                    &snapshot,
                    preview.request.clone(),
                )
                .map(|command| {
                    (
                        confirmed.clone(),
                        command,
                        OperationStage::CleanupPreview(Box::new(CleanupPreviewStage {
                            capability_request: confirmed.capability_request,
                            operation_id: confirmed.id,
                            snapshot,
                            confirmed: preview,
                            stdout: Vec::new(),
                        })),
                    )
                })
                .map_err(|error| error.to_string())
            }
            MaintenanceOperation::PrService(request) => {
                let evidence = match request.operation {
                    PrServiceOperation::Export => EvidencePlan::PrExport(request.file.clone()),
                    PrServiceOperation::Import => EvidencePlan::None,
                };
                pr_service_command(
                    id,
                    confirmed.capability_request,
                    &snapshot,
                    confirmed.id,
                    request,
                )
                .map(|(preview, command)| {
                    (
                        preview,
                        command,
                        OperationStage::Execute(Box::new(evidence)),
                    )
                })
                .map_err(|error| error.to_string())
            }
            MaintenanceOperation::LockedSignatureCache(request) => locked_signature_command(
                id,
                confirmed.capability_request,
                &snapshot,
                confirmed.id,
                request,
            )
            .map(|(preview, command, before)| {
                (
                    preview,
                    command,
                    OperationStage::Execute(Box::new(EvidencePlan::Release(before))),
                )
            })
            .map_err(|error| error.to_string()),
            MaintenanceOperation::BuildHistoryComparison(request) => buildhistory_command(
                id,
                confirmed.capability_request,
                &snapshot,
                confirmed.id,
                request,
            )
            .map(|(preview, command)| {
                (
                    preview,
                    command,
                    OperationStage::Execute(Box::new(EvidencePlan::None)),
                )
            })
            .map_err(|error| error.to_string()),
            MaintenanceOperation::BuildCompare(request) => {
                build_compare_command(id, &snapshot, request)
                    .map(|command| {
                        (
                            confirmed.clone(),
                            command,
                            OperationStage::Execute(Box::new(EvidencePlan::None)),
                        )
                    })
                    .map_err(|error| error.to_string())
            }
            MaintenanceOperation::GitArchive(request) if request.push_remote.is_some() => self
                .local_archive
                .as_ref()
                .ok_or_else(|| {
                    "local Git archive evidence is unavailable; run local archive first".to_owned()
                })
                .and_then(|local| {
                    git_archive_push_command(
                        id,
                        confirmed.capability_request,
                        &snapshot,
                        confirmed.id,
                        request,
                        local,
                    )
                    .map(|(preview, command)| {
                        (
                            preview,
                            command,
                            OperationStage::Execute(Box::new(EvidencePlan::None)),
                        )
                    })
                    .map_err(|error| error.to_string())
                }),
            MaintenanceOperation::GitArchive(request) => {
                let evidence_request = self
                    .archive_intent
                    .take()
                    .filter(|(preview_id, original)| {
                        let mut local = original.clone();
                        local.push_remote = None;
                        *preview_id == confirmed.id && local == request
                    })
                    .map(|(_, original)| original)
                    .unwrap_or_else(|| request.clone());
                git_archive_local_command(
                    id,
                    confirmed.capability_request,
                    &snapshot,
                    confirmed.id,
                    &evidence_request,
                )
                .map(|(preview, command)| {
                    (
                        preview,
                        command,
                        OperationStage::Execute(Box::new(EvidencePlan::GitArchive(
                            evidence_request,
                        ))),
                    )
                })
                .map_err(|error| error.to_string())
            }
        };
        let (fresh_preview, command, stage) = match built {
            Ok(value) => value,
            Err(message) => {
                fail(app, id, &message);
                return;
            }
        };
        if fresh_preview != confirmed {
            fail(
                app,
                id,
                "Maintenance preview changed; inspect and confirm the exact operation again",
            );
            return;
        }
        let mut runner = MaintenanceSstateJobRunner::new();
        if let Err(error) = runner.start(command).await {
            fail(app, id, &error.to_string());
            return;
        }
        self.operation = Some(MaintenanceCliOperation {
            id,
            runner: Some(runner),
            cancellation: None,
            stage,
        });
    }

    async fn poll_operation(&mut self, app: &mut App) {
        let Some(active) = self.operation.as_mut() else {
            return;
        };
        if active
            .cancellation
            .as_ref()
            .is_some_and(JoinHandle::is_finished)
        {
            let wait = active
                .cancellation
                .take()
                .expect("cancellation was checked");
            match wait.await {
                Ok((runner, Ok(_))) => active.runner = Some(runner),
                Ok((runner, Err(message))) => {
                    active.runner = Some(runner);
                    reject_cancellation(app, active.id, &message);
                }
                Err(error) => {
                    let id = active.id;
                    lose(
                        app,
                        id,
                        &format!("Maintenance cancellation worker was lost: {error}"),
                    );
                    self.operation = None;
                    return;
                }
            }
        }
        let Some(runner) = active.runner.as_mut() else {
            return;
        };
        let event = match tokio::time::timeout(Duration::from_millis(1), runner.next_event()).await
        {
            Ok(Ok(event)) => event,
            Ok(Err(error)) => {
                let id = active.id;
                lose(app, id, &error.to_string());
                self.operation = None;
                return;
            }
            Err(_) => return,
        };
        self.apply_runner_event(app, event).await;
    }

    async fn apply_runner_event(&mut self, app: &mut App, event: MaintenanceSstateRunnerEvent) {
        let Some(active) = self.operation.as_mut() else {
            return;
        };
        match event {
            MaintenanceSstateRunnerEvent::Started { id } => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::SessionRunning {
                        id,
                        started_at: SystemTime::now(),
                    }),
                );
            }
            MaintenanceSstateRunnerEvent::Output {
                id,
                stream,
                line,
                truncated,
            } => {
                if let OperationStage::CleanupPreview(stage) = &mut active.stage
                    && stream == yoctui_model::MaintenanceOutputStream::Stdout
                {
                    stage.stdout.push(line.clone());
                }
                let text = if truncated {
                    format!("{line} [truncated]")
                } else {
                    line
                };
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::SessionOutput { id, stream, text }),
                );
            }
            MaintenanceSstateRunnerEvent::Completed { id, exit_code } => {
                if matches!(active.stage, OperationStage::CleanupPreview(_)) {
                    self.advance_cleanup(app).await;
                } else {
                    let evidence = match self.capture_evidence() {
                        Ok(evidence) => evidence,
                        Err(message) => {
                            fail(
                                app,
                                id,
                                &format!("Maintenance evidence validation failed: {message}"),
                            );
                            self.operation = None;
                            return;
                        }
                    };
                    let _ = update(
                        app,
                        Action::Maintenance(MaintenanceAction::CompleteSession {
                            id,
                            exit_code: exit_code.unwrap_or(0),
                            evidence,
                            finished_at: SystemTime::now(),
                        }),
                    );
                    self.operation = None;
                    if let Some(request) = self.deferred_archive_push.take()
                        && let Some(capability_request) = app.maintenance.capability.request()
                    {
                        self.start_inspection(InspectionPurpose::PreviewGitArchivePush {
                            capability_request,
                            request: Box::new(request),
                        });
                    }
                }
            }
            MaintenanceSstateRunnerEvent::Failed { id, exit_code } => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::FailSession {
                        id,
                        message: "Maintenance command exited unsuccessfully".into(),
                        exit_code,
                        finished_at: SystemTime::now(),
                    }),
                );
                self.operation = None;
            }
            MaintenanceSstateRunnerEvent::CancellationRequested { .. } => {}
            MaintenanceSstateRunnerEvent::Cancelled { id, .. } => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::CancelSession {
                        id,
                        finished_at: SystemTime::now(),
                    }),
                );
                self.operation = None;
            }
            MaintenanceSstateRunnerEvent::CancellationRejected { id, message } => {
                reject_cancellation(app, id, &message);
            }
            MaintenanceSstateRunnerEvent::TimedOut { id, .. } => {
                let _ = update(
                    app,
                    Action::Maintenance(MaintenanceAction::TimeoutSession {
                        id,
                        finished_at: SystemTime::now(),
                    }),
                );
                self.operation = None;
            }
            MaintenanceSstateRunnerEvent::Lost { id, message } => {
                lose(app, id, &message);
                self.operation = None;
            }
        }
    }

    async fn advance_cleanup(&mut self, app: &mut App) {
        let active = self
            .operation
            .as_mut()
            .expect("cleanup operation is active");
        let OperationStage::CleanupPreview(stage) = &active.stage else {
            return;
        };
        let result = parse_cleanup_preview(stage.confirmed.request.clone(), &stage.stdout)
            .and_then(|fresh| {
                MaintenanceSstateCommandSpec::cleanup_execution(
                    active.id,
                    stage.capability_request,
                    &stage.snapshot,
                    stage.operation_id,
                    &stage.confirmed,
                    &fresh,
                )
            });
        let (_, command) = match result {
            Ok(value) => value,
            Err(error) => {
                fail(app, active.id, &error.to_string());
                self.operation = None;
                return;
            }
        };
        let runner = active.runner.as_mut().expect("cleanup runner is owned");
        if let Err(error) = runner.start(command).await {
            fail(app, active.id, &error.to_string());
            self.operation = None;
            return;
        }
        active.stage = OperationStage::Execute(Box::new(EvidencePlan::None));
    }

    fn capture_evidence(&mut self) -> Result<Vec<MaintenanceEvidence>, String> {
        let active = self
            .operation
            .as_ref()
            .expect("completed operation is active");
        let OperationStage::Execute(plan) = &active.stage else {
            return Ok(Vec::new());
        };
        let plan = plan.as_ref().clone();
        match plan {
            EvidencePlan::None => Ok(Vec::new()),
            EvidencePlan::Release(before) => {
                before.changed_evidence().map_err(|error| error.to_string())
            }
            EvidencePlan::GitArchive(request) => {
                let result =
                    GitArchiveLocalResult::capture(&request).map_err(|error| error.to_string())?;
                let evidence =
                    MaintenanceEvidence::new(result.head.clone(), "Git archive HEAD".into())
                        .map_err(str::to_owned)?;
                self.local_archive = Some(result);
                if request.push_remote.is_some() {
                    self.deferred_archive_push = Some(request);
                }
                Ok(vec![evidence])
            }
            EvidencePlan::PrExport(path) => Ok(vec![file_evidence(&path, "PR service export")?]),
        }
    }

    pub(crate) fn revalidate_evidence(
        &self,
        identity: &MaintenanceFileIdentity,
    ) -> Result<PathBuf, String> {
        let current = file_identity(&identity.path)?;
        if &current != identity {
            return Err(format!(
                "Maintenance evidence changed: {}",
                identity.path.display()
            ));
        }
        Ok(current.path)
    }
}

fn inspect(context: MaintenanceCliContext) -> MaintenanceInspection {
    let build_dir = context.build_dir();
    let sstate = MaintenanceSstateCapabilityInspector::inspect(MaintenanceSstateCapabilityInput {
        build_dir: build_dir.clone(),
        sstate_dir: context.metadata.sstate_dir.clone(),
        tmp_dir: context.metadata.tmp_dir.clone(),
        stamps_dirs: context.metadata.stamps_dirs.clone(),
        executable_search_path: context.search_path.clone(),
    });
    let service =
        MaintenanceServiceCapabilityInspector::inspect(MaintenanceServiceCapabilityInput {
            build_dir: build_dir.clone(),
            prserv_host: context.metadata.prserv_host.clone(),
            hashserve: context.metadata.hashserve.clone(),
            hashserve_upstream: context.metadata.hashserve_upstream.clone(),
            signature_handler: context.metadata.signature_handler.clone(),
            executable_search_path: context.search_path.clone(),
            process_root: PathBuf::from("/proc"),
            endpoint_probe_timeout: Duration::from_millis(100),
            endpoint_observations: Vec::new(),
        });
    let release =
        MaintenanceReleaseCapabilityInspector::inspect(MaintenanceReleaseCapabilityInput {
            build_dir: build_dir.clone(),
            buildhistory_dir: context.metadata.buildhistory_dir.clone(),
            native_lsb: context.metadata.native_lsb.clone(),
            executable_search_path: context.search_path.clone(),
        });
    let optional =
        MaintenanceOptionalCapabilityInspector::inspect(MaintenanceOptionalCapabilityInput {
            build_dir,
            executable_search_path: context.search_path,
            git_worktree_candidates: context.git_candidates,
            error_report_candidates: Vec::new(),
            repo_workspace_candidates: context.repo_candidates,
            toaster_configuration_candidates: context.toaster_candidates,
            process_root: PathBuf::from("/proc"),
        });

    let services = service
        .as_ref()
        .map(|inspection| (inspection.services.clone(), inspection.limitations.clone()))
        .map_err(ToString::to_string);
    let integrations = optional
        .as_ref()
        .map_err(ToString::to_string)
        .and_then(|inspection| {
            inspection
                .integrations_snapshot()
                .map_err(|error| error.to_string())
        });
    let capability = merge_capabilities(
        context.metadata,
        [
            sstate.map_err(|error| error.to_string()),
            service
                .map(|inspection| inspection.capability)
                .map_err(|error| error.to_string()),
            release.map_err(|error| error.to_string()),
            optional
                .map(|inspection| inspection.capability)
                .map_err(|error| error.to_string()),
        ],
    );
    MaintenanceInspection {
        capability,
        services,
        integrations,
    }
}

fn merge_capabilities(
    metadata: MaintenanceMetadata,
    groups: impl IntoIterator<Item = Result<MaintenanceCapabilitySnapshot, String>>,
) -> Result<MaintenanceCapabilitySnapshot, String> {
    let mut tools = BTreeMap::new();
    let mut limitations = Vec::new();
    for group in groups {
        match group {
            Ok(snapshot) => {
                for capability in snapshot.tools {
                    tools.insert(capability.tool(), capability);
                }
                limitations.extend(snapshot.limitations);
            }
            Err(message) => limitations.push(message),
        }
    }
    for tool in [
        MaintenanceTool::OeCheckSstate,
        MaintenanceTool::SstateCacheManagement,
        MaintenanceTool::PrServiceTool,
        MaintenanceTool::LockedSignatureCache,
        MaintenanceTool::BuildHistoryDiff,
        MaintenanceTool::BuildCompare,
        MaintenanceTool::GitArchive,
        MaintenanceTool::CreatePullRequest,
        MaintenanceTool::SendPullRequest,
        MaintenanceTool::SendErrorReport,
        MaintenanceTool::Toaster,
    ] {
        tools
            .entry(tool)
            .or_insert_with(|| MaintenanceToolCapability::Unavailable {
                tool,
                reason: "capability adapter did not return authoritative evidence".into(),
            });
    }
    MaintenanceCapabilitySnapshot::new(metadata, tools.into_values().collect(), limitations)
        .map_err(str::to_owned)
}

fn apply_services(
    app: &mut App,
    request: u64,
    result: Result<(Vec<ServiceDiagnostic>, Vec<String>), String>,
) {
    let action = match result {
        Ok((services, limitations)) => MaintenanceAction::ServicesLoaded {
            request,
            services,
            limitations,
        },
        Err(message) => MaintenanceAction::ServicesFailed { request, message },
    };
    let _ = update(app, Action::Maintenance(action));
}

fn apply_integrations(
    app: &mut App,
    request: u64,
    result: Result<MaintenanceIntegrationsSnapshot, String>,
) {
    let action = match result {
        Ok(snapshot) => MaintenanceAction::IntegrationsLoaded {
            request,
            partial: !snapshot.limitations.is_empty(),
            snapshot: Box::new(snapshot),
        },
        Err(message) => MaintenanceAction::IntegrationsFailed { request, message },
    };
    let _ = update(app, Action::Maintenance(action));
}

fn fail(app: &mut App, id: MaintenanceSessionId, message: &str) {
    let _ = update(
        app,
        Action::Maintenance(MaintenanceAction::FailSession {
            id,
            message: message.into(),
            exit_code: None,
            finished_at: SystemTime::now(),
        }),
    );
}

fn lose(app: &mut App, id: MaintenanceSessionId, message: &str) {
    let _ = update(
        app,
        Action::Maintenance(MaintenanceAction::LoseSession {
            id,
            message: message.into(),
            finished_at: SystemTime::now(),
        }),
    );
}

fn reject_cancellation(app: &mut App, id: MaintenanceSessionId, message: &str) {
    let _ = update(
        app,
        Action::Maintenance(MaintenanceAction::RejectCancellation {
            id,
            message: message.into(),
        }),
    );
}

fn canonical_directory(path: &Path) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path).map_err(|error| error.to_string())?;
    if canonical == Path::new("/") || !canonical.is_dir() {
        return Err(format!("unsafe Maintenance directory: {}", path.display()));
    }
    Ok(canonical)
}

fn canonical_file(path: &Path) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path).map_err(|error| error.to_string())?;
    if !canonical.is_file() {
        return Err(format!("unsafe Maintenance file: {}", path.display()));
    }
    Ok(canonical)
}

fn file_identity(path: &Path) -> Result<MaintenanceFileIdentity, String> {
    let path = canonical_file(path)?;
    let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
    MaintenanceFileIdentity::new(
        path,
        metadata.len(),
        metadata.modified().map_err(|error| error.to_string())?,
    )
    .map_err(str::to_owned)
}

fn file_evidence(path: &Path, label: &str) -> Result<MaintenanceEvidence, String> {
    MaintenanceEvidence::new(file_identity(path)?, label.into()).map_err(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use yoctui_model::{
        MAX_MAINTENANCE_OUTPUT, MaintenanceCapability, MaintenanceSessionStatus,
        SstateReadinessMode, SstateReadinessRequest,
    };

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);

    struct Fixture {
        root: PathBuf,
        build: PathBuf,
        bin: PathBuf,
    }

    impl Fixture {
        fn new(script: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "yoctui-maintenance-workflow-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
            ));
            let build = root.join("build");
            let bin = root.join("bin");
            fs::create_dir_all(&build).unwrap();
            fs::create_dir_all(&bin).unwrap();
            let executable = bin.join("oe-check-sstate");
            Self::publish_executable(&executable, script);
            Self { root, build, bin }
        }

        fn publish_executable(executable: &Path, script: &str) {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                let temporary = executable.with_extension(format!(
                    "fixture-write-{}",
                    NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
                ));
                fs::write(&temporary, script).unwrap();
                fs::set_permissions(&temporary, fs::Permissions::from_mode(0o755)).unwrap();
                fs::rename(temporary, executable).unwrap();
            }
            #[cfg(not(unix))]
            fs::write(executable, script).unwrap();
        }

        fn install(&self, name: &str, script: &str) -> PathBuf {
            let executable = self.bin.join(name);
            Self::publish_executable(&executable, script);
            executable
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    async fn poll_until(
        coordinator: &mut MaintenanceCliCoordinator,
        app: &mut App,
        complete: impl Fn(&App, &MaintenanceCliCoordinator) -> bool,
    ) {
        for _ in 0..2_400 {
            coordinator.poll(app).await;
            if complete(app, coordinator) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("Maintenance CLI workflow did not complete");
    }

    async fn refreshed_coordinator(fixture: &Fixture) -> (App, MaintenanceCliCoordinator, u64) {
        let mut app = App::new(100, 64 * 1024);
        app.workspace.build_dir = Some(fixture.build.clone());
        let mut coordinator =
            MaintenanceCliCoordinator::new(&app, &fixture.build, vec![fixture.bin.clone()])
                .unwrap();
        let effect = update(
            &mut app,
            Action::Maintenance(MaintenanceAction::InspectCapability),
        )
        .unwrap();
        assert!(coordinator.handle_effect(&mut app, effect).await);
        poll_until(&mut coordinator, &mut app, |app, _| {
            matches!(
                app.maintenance.capability,
                MaintenanceCapability::Available { .. } | MaintenanceCapability::Partial { .. }
            )
        })
        .await;
        let request = app.maintenance.capability.request().unwrap();
        (app, coordinator, request)
    }

    async fn refreshed_cleanup_coordinator(
        fixture: &Fixture,
        script: &str,
    ) -> (App, MaintenanceCliCoordinator, u64, PathBuf, PathBuf) {
        fixture.install("sstate-cache-management.py", script);
        let cache = fixture.root.join("sstate-cache");
        let stamps = fixture.root.join("stamps");
        fs::create_dir_all(&cache).unwrap();
        fs::create_dir_all(&stamps).unwrap();
        let mut app = App::new(100, 64 * 1024);
        app.workspace
            .variables
            .insert("SSTATE_DIR".into(), cache.display().to_string());
        app.workspace
            .variables
            .insert("STAMPS_DIR".into(), stamps.display().to_string());
        let mut coordinator =
            MaintenanceCliCoordinator::new(&app, &fixture.build, vec![fixture.bin.clone()])
                .unwrap();
        let effect = update(
            &mut app,
            Action::Maintenance(MaintenanceAction::InspectCapability),
        )
        .unwrap();
        coordinator.handle_effect(&mut app, effect).await;
        poll_until(&mut coordinator, &mut app, |app, _| {
            app.maintenance
                .capability
                .snapshot()
                .is_some_and(|snapshot| snapshot.supports(MaintenanceTool::SstateCacheManagement))
        })
        .await;
        let request = app.maintenance.capability.request().unwrap();
        (app, coordinator, request, cache, stamps)
    }

    async fn refreshed_service_coordinator(
        fixture: &Fixture,
        script: &str,
    ) -> (App, MaintenanceCliCoordinator, u64) {
        fixture.install("bitbake-prserv-tool", script);
        let mut app = App::new(100, 64 * 1024);
        app.workspace
            .variables
            .insert("PRSERV_HOST".into(), "localhost:8585".into());
        let mut coordinator =
            MaintenanceCliCoordinator::new(&app, &fixture.build, vec![fixture.bin.clone()])
                .unwrap();
        let effect = update(
            &mut app,
            Action::Maintenance(MaintenanceAction::InspectCapability),
        )
        .unwrap();
        coordinator.handle_effect(&mut app, effect).await;
        poll_until(&mut coordinator, &mut app, |app, _| {
            app.maintenance
                .capability
                .snapshot()
                .is_some_and(|snapshot| snapshot.supports(MaintenanceTool::PrServiceTool))
        })
        .await;
        let request = app.maintenance.capability.request().unwrap();
        (app, coordinator, request)
    }

    async fn refreshed_release_coordinator(
        fixture: &Fixture,
        locked_script: &str,
        buildhistory_script: &str,
        archive_script: &str,
    ) -> (App, MaintenanceCliCoordinator, u64, PathBuf) {
        fixture.install("gen-lockedsig-cache", locked_script);
        fixture.install("buildhistory-diff", buildhistory_script);
        fixture.install("oe-git-archive", archive_script);
        let history = fixture.root.join("buildhistory");
        fs::create_dir_all(history.join(".git")).unwrap();
        fs::write(history.join(".git/HEAD"), b"ref: refs/heads/main\n").unwrap();
        let mut app = App::new(100, 64 * 1024);
        app.workspace
            .variables
            .insert("NATIVELSBSTRING".into(), "ubuntu".into());
        app.workspace
            .variables
            .insert("BUILDHISTORY_DIR".into(), history.display().to_string());
        let mut coordinator =
            MaintenanceCliCoordinator::new(&app, &fixture.build, vec![fixture.bin.clone()])
                .unwrap();
        let effect = update(
            &mut app,
            Action::Maintenance(MaintenanceAction::InspectCapability),
        )
        .unwrap();
        coordinator.handle_effect(&mut app, effect).await;
        poll_until(&mut coordinator, &mut app, |app, _| {
            app.maintenance
                .capability
                .snapshot()
                .is_some_and(|snapshot| {
                    [
                        MaintenanceTool::LockedSignatureCache,
                        MaintenanceTool::BuildHistoryDiff,
                        MaintenanceTool::GitArchive,
                    ]
                    .into_iter()
                    .all(|tool| snapshot.supports(tool))
                })
        })
        .await;
        let request = app.maintenance.capability.request().unwrap();
        (app, coordinator, request, history)
    }

    async fn begin_readiness(
        app: &mut App,
        coordinator: &mut MaintenanceCliCoordinator,
        request: u64,
        id: MaintenanceSessionId,
        timeout_seconds: u64,
    ) {
        let snapshot = app.maintenance.capability.snapshot().unwrap().clone();
        let readiness = SstateReadinessRequest::new(
            vec!["core-image-minimal".into()],
            SstateReadinessMode::IsolatedTmpdir,
            None,
            None,
            timeout_seconds,
        )
        .unwrap();
        let (preview, _) =
            MaintenanceSstateCommandSpec::readiness(id, request, &snapshot, id.0, readiness)
                .unwrap();
        let _ = update(
            app,
            Action::Maintenance(MaintenanceAction::BeginOperation(preview.clone())),
        );
        let effect = update(
            app,
            Action::Maintenance(MaintenanceAction::ConfirmOperation(preview)),
        )
        .unwrap();
        assert!(coordinator.handle_effect(app, effect).await);
    }

    #[tokio::test]
    async fn maintenance_workflow_refresh_is_correlated_and_preserves_unavailable_tools() {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let (app, mut coordinator, request) = refreshed_coordinator(&fixture).await;
        let snapshot = app.maintenance.capability.snapshot().unwrap();
        assert_eq!(request, 1);
        assert!(snapshot.supports(MaintenanceTool::OeCheckSstate));
        assert!(matches!(
            snapshot.capability(MaintenanceTool::BuildCompare),
            Some(MaintenanceToolCapability::Unavailable { .. })
        ));
        assert_eq!(app.maintenance.services.request(), Some(request));
        assert_eq!(app.maintenance.integrations.request(), Some(request));
        coordinator.shutdown().await;
    }

    #[tokio::test]
    async fn maintenance_workflow_runner_maps_success_and_bounded_output() {
        let fixture = Fixture::new("#!/bin/sh\nprintf 'cache ready\\n'\nexit 0\n");
        let (mut app, mut coordinator, request) = refreshed_coordinator(&fixture).await;
        let snapshot = app.maintenance.capability.snapshot().unwrap().clone();
        let id = MaintenanceSessionId(91);
        let readiness = SstateReadinessRequest::new(
            vec!["core-image-minimal".into()],
            SstateReadinessMode::IsolatedTmpdir,
            None,
            None,
            30,
        )
        .unwrap();
        let (preview, _) =
            MaintenanceSstateCommandSpec::readiness(id, request, &snapshot, id.0, readiness)
                .unwrap();
        let _ = update(
            &mut app,
            Action::Maintenance(MaintenanceAction::BeginOperation(preview.clone())),
        );
        let effect = update(
            &mut app,
            Action::Maintenance(MaintenanceAction::ConfirmOperation(preview)),
        )
        .unwrap();
        assert!(coordinator.handle_effect(&mut app, effect).await);
        poll_until(&mut coordinator, &mut app, |app, coordinator| {
            !coordinator.operation_active()
                && app
                    .maintenance
                    .sessions
                    .back()
                    .is_some_and(|session| session.status.is_terminal())
        })
        .await;
        let session = app.maintenance.sessions.back().unwrap();
        assert_eq!(session.status, MaintenanceSessionStatus::Succeeded);
        assert!(session.output.iter().any(|line| line.text == "cache ready"));
        coordinator.shutdown().await;
    }

    #[tokio::test]
    async fn maintenance_workflow_rejects_cancellation_for_an_unowned_session() {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let (mut app, mut coordinator, _) = refreshed_coordinator(&fixture).await;
        let handled = coordinator
            .handle_effect(
                &mut app,
                Effect::Maintenance(MaintenanceEffect::CancelOperation(MaintenanceSessionId(7))),
            )
            .await;
        assert!(handled);
        assert!(!coordinator.operation_active());
        coordinator.shutdown().await;
    }

    #[tokio::test]
    async fn maintenance_workflow_runner_maps_nonzero_and_timeout_distinctly() {
        for (script, id, timeout, expected) in [
            (
                "#!/bin/sh\nexit 7\n",
                MaintenanceSessionId(92),
                30,
                MaintenanceSessionStatus::Failed,
            ),
            (
                "#!/bin/sh\nsleep 5\n",
                MaintenanceSessionId(93),
                1,
                MaintenanceSessionStatus::TimedOut,
            ),
        ] {
            let fixture = Fixture::new(script);
            let (mut app, mut coordinator, request) = refreshed_coordinator(&fixture).await;
            begin_readiness(&mut app, &mut coordinator, request, id, timeout).await;
            poll_until(&mut coordinator, &mut app, |app, coordinator| {
                !coordinator.operation_active()
                    && app
                        .maintenance
                        .sessions
                        .back()
                        .is_some_and(|session| session.status.is_terminal())
            })
            .await;
            assert_eq!(app.maintenance.sessions.back().unwrap().status, expected);
            coordinator.shutdown().await;
        }
    }

    #[tokio::test]
    async fn maintenance_workflow_cancels_only_the_exact_active_session() {
        let fixture = Fixture::new("#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n");
        let (mut app, mut coordinator, request) = refreshed_coordinator(&fixture).await;
        let id = MaintenanceSessionId(94);
        begin_readiness(&mut app, &mut coordinator, request, id, 30).await;
        poll_until(&mut coordinator, &mut app, |app, coordinator| {
            coordinator.operation_active()
                && app
                    .maintenance
                    .sessions
                    .back()
                    .is_some_and(|session| session.status == MaintenanceSessionStatus::Running)
        })
        .await;

        coordinator
            .handle_effect(
                &mut app,
                Effect::Maintenance(MaintenanceEffect::CancelOperation(MaintenanceSessionId(
                    999,
                ))),
            )
            .await;
        assert!(coordinator.operation_active());
        let _ = update(
            &mut app,
            Action::Maintenance(MaintenanceAction::BeginCancellation),
        );
        let effect = update(
            &mut app,
            Action::Maintenance(MaintenanceAction::ConfirmCancellation(id)),
        )
        .unwrap();
        coordinator.handle_effect(&mut app, effect).await;
        poll_until(&mut coordinator, &mut app, |app, coordinator| {
            !coordinator.operation_active()
                && app
                    .maintenance
                    .sessions
                    .back()
                    .is_some_and(|session| session.status.is_terminal())
        })
        .await;
        assert_eq!(
            app.maintenance.sessions.back().unwrap().status,
            MaintenanceSessionStatus::Cancelled
        );
        coordinator.shutdown().await;
    }

    #[tokio::test]
    async fn maintenance_sstate_workspace_builds_exact_readiness_confirmation() {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let (mut app, mut coordinator, capability_request) = refreshed_coordinator(&fixture).await;
        let request = SstateReadinessRequest::new(
            vec!["core-image-minimal".into()],
            SstateReadinessMode::SameTmpdir,
            Some(fixture.build.join("readiness.txt")),
            None,
            30,
        )
        .unwrap();
        assert!(
            coordinator
                .handle_effect(
                    &mut app,
                    Effect::Maintenance(MaintenanceEffect::PreviewReadiness {
                        capability_request,
                        request,
                    }),
                )
                .await
        );
        poll_until(&mut coordinator, &mut app, |app, _| {
            matches!(
                app.active_dialog(),
                Some(yoctui_model::Dialog::Maintenance(dialog))
                    if matches!(dialog.as_ref(), yoctui_model::MaintenanceDialog::Confirm(_))
            )
        })
        .await;
        let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog() else {
            panic!("readiness confirmation is absent");
        };
        let yoctui_model::MaintenanceDialog::Confirm(preview) = dialog.as_ref() else {
            panic!("wrong readiness confirmation");
        };
        assert!(
            preview
                .arguments
                .iter()
                .any(|argument| argument.ends_with(": --same-tmpdir"))
        );
        assert!(
            preview
                .arguments
                .iter()
                .any(|argument| argument.ends_with(": core-image-minimal"))
        );
        coordinator.shutdown().await;
    }

    #[tokio::test]
    async fn maintenance_sstate_workspace_discovers_exact_cleanup_candidates_before_phrase() {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let candidate = fixture.root.join("sstate-cache/candidate.tgz");
        let script = format!("#!/bin/sh\nprintf '%s\\n' '{}'\n", candidate.display());
        let (mut app, mut coordinator, capability_request, cache, stamps) =
            refreshed_cleanup_coordinator(&fixture, &script).await;
        fs::write(&candidate, b"candidate").unwrap();
        let request = SstateCleanupRequest::new(
            cache.clone(),
            vec![stamps],
            vec![yoctui_model::SstateCleanupMode::Duplicates],
            1,
        )
        .unwrap();
        coordinator
            .handle_effect(
                &mut app,
                Effect::Maintenance(MaintenanceEffect::PreviewCleanup {
                    capability_request,
                    request,
                }),
            )
            .await;
        poll_until(&mut coordinator, &mut app, |app, coordinator| {
            coordinator.cleanup_preview.is_none()
                && matches!(
                    app.active_dialog(),
                    Some(yoctui_model::Dialog::Maintenance(dialog))
                        if matches!(dialog.as_ref(), yoctui_model::MaintenanceDialog::CleanupPhrase { .. })
                )
        })
        .await;
        let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog() else {
            panic!("cleanup phrase dialog is absent");
        };
        let yoctui_model::MaintenanceDialog::CleanupPhrase { preview, .. } = dialog.as_ref() else {
            panic!("wrong cleanup dialog");
        };
        let MaintenanceOperation::SstateCleanup(preview) = &preview.operation else {
            panic!("wrong cleanup operation");
        };
        assert_eq!(preview.candidates.len(), 1);
        assert_eq!(preview.candidates[0].path, candidate);
        assert_eq!(
            preview.required_phrase(),
            format!("DELETE 1 FROM {}", cache.display())
        );
        coordinator.shutdown().await;
    }

    #[tokio::test]
    async fn maintenance_sstate_workspace_preview_failure_never_opens_destructive_dialog() {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let (mut app, mut coordinator, capability_request, cache, _) =
            refreshed_cleanup_coordinator(&fixture, "#!/bin/sh\necho denied >&2\nexit 9\n").await;
        let request = SstateCleanupRequest::new(
            cache,
            Vec::new(),
            vec![yoctui_model::SstateCleanupMode::Duplicates],
            1,
        )
        .unwrap();
        coordinator
            .handle_effect(
                &mut app,
                Effect::Maintenance(MaintenanceEffect::PreviewCleanup {
                    capability_request,
                    request,
                }),
            )
            .await;
        poll_until(&mut coordinator, &mut app, |app, coordinator| {
            coordinator.cleanup_preview.is_none()
                && app
                    .notification
                    .as_deref()
                    .is_some_and(|message| message.contains("status Some(9)"))
        })
        .await;
        assert!(app.active_dialog().is_none());
        coordinator.shutdown().await;
    }

    #[tokio::test]
    async fn maintenance_service_workspace_builds_distinct_exact_previews() {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let (mut app, mut coordinator, capability_request) =
            refreshed_service_coordinator(&fixture, "#!/bin/sh\nexit 0\n").await;
        for operation in [PrServiceOperation::Export, PrServiceOperation::Import] {
            app.dialogs.clear();
            let file = fixture.build.join(match operation {
                PrServiceOperation::Export => "export.conf",
                PrServiceOperation::Import => "import.inc",
            });
            if operation == PrServiceOperation::Import {
                fs::write(&file, b"PRSERV_DUMP = \"1\"\n").unwrap();
            }
            let request = yoctui_model::PrServiceRequest::new(
                operation,
                file.clone(),
                fixture.build.clone(),
                "localhost:8585".into(),
            )
            .unwrap();
            coordinator
                .handle_effect(
                    &mut app,
                    Effect::Maintenance(MaintenanceEffect::PreviewPrService {
                        capability_request,
                        request,
                    }),
                )
                .await;
            poll_until(&mut coordinator, &mut app, |app, _| {
                matches!(
                    app.active_dialog(),
                    Some(yoctui_model::Dialog::Maintenance(dialog))
                        if matches!(dialog.as_ref(), yoctui_model::MaintenanceDialog::Confirm(_))
                )
            })
            .await;
            let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog() else {
                panic!("PR preview is absent");
            };
            let yoctui_model::MaintenanceDialog::Confirm(preview) = dialog.as_ref() else {
                panic!("wrong PR preview dialog");
            };
            let expected = match operation {
                PrServiceOperation::Export => "export",
                PrServiceOperation::Import => "import",
            };
            assert!(
                preview
                    .arguments
                    .iter()
                    .any(|argument| argument.ends_with(&format!(": {expected}")))
            );
            assert!(
                preview
                    .arguments
                    .iter()
                    .any(|argument| argument.ends_with(&format!(": {}", file.display())))
            );
        }
        coordinator.shutdown().await;
    }

    #[tokio::test]
    async fn maintenance_service_workspace_success_installs_exact_export_evidence() {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let script = "#!/bin/sh\nprintf 'PRSERV_DUMP = \\\"1\\\"\\n' > \"$2\"\n";
        let (mut app, mut coordinator, capability_request) =
            refreshed_service_coordinator(&fixture, script).await;
        let destination = fixture.build.join("export.inc");
        let request = yoctui_model::PrServiceRequest::new(
            PrServiceOperation::Export,
            destination.clone(),
            fixture.build.clone(),
            "localhost:8585".into(),
        )
        .unwrap();
        coordinator
            .handle_effect(
                &mut app,
                Effect::Maintenance(MaintenanceEffect::PreviewPrService {
                    capability_request,
                    request,
                }),
            )
            .await;
        poll_until(&mut coordinator, &mut app, |app, _| {
            app.active_dialog().is_some()
        })
        .await;
        let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog().cloned() else {
            panic!("PR confirmation is absent");
        };
        let yoctui_model::MaintenanceDialog::Confirm(preview) = dialog.as_ref() else {
            panic!("wrong PR confirmation");
        };
        let effect = update(
            &mut app,
            Action::Maintenance(MaintenanceAction::ConfirmOperation(preview.clone())),
        )
        .unwrap();
        coordinator.handle_effect(&mut app, effect).await;
        poll_until(&mut coordinator, &mut app, |app, coordinator| {
            !coordinator.operation_active()
                && app
                    .maintenance
                    .sessions
                    .back()
                    .is_some_and(|session| session.status.is_terminal())
        })
        .await;
        assert_eq!(
            app.maintenance.sessions.back().unwrap().status,
            MaintenanceSessionStatus::Succeeded
        );
        assert_eq!(app.maintenance.evidence.len(), 1);
        assert_eq!(app.maintenance.evidence[0].identity.path, destination);
        coordinator.shutdown().await;
    }

    #[tokio::test]
    async fn maintenance_service_workspace_rejects_stale_and_invalid_previews() {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let (mut app, mut coordinator, capability_request) =
            refreshed_service_coordinator(&fixture, "#!/bin/sh\nexit 0\n").await;
        let valid = yoctui_model::PrServiceRequest::new(
            PrServiceOperation::Export,
            fixture.build.join("export.conf"),
            fixture.build.clone(),
            "localhost:8585".into(),
        )
        .unwrap();
        coordinator
            .handle_effect(
                &mut app,
                Effect::Maintenance(MaintenanceEffect::PreviewPrService {
                    capability_request: capability_request + 1,
                    request: valid,
                }),
            )
            .await;
        poll_until(&mut coordinator, &mut app, |app, _| {
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("capability changed"))
        })
        .await;
        assert!(app.active_dialog().is_none());

        app.notification = None;
        let invalid = yoctui_model::PrServiceRequest {
            operation: PrServiceOperation::Export,
            file: PathBuf::from("relative.conf"),
            build_dir: fixture.build.clone(),
            endpoint: "localhost:8585".into(),
        };
        coordinator
            .handle_effect(
                &mut app,
                Effect::Maintenance(MaintenanceEffect::PreviewPrService {
                    capability_request,
                    request: invalid,
                }),
            )
            .await;
        poll_until(&mut coordinator, &mut app, |app, _| {
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("absolute") || message.contains("unsafe"))
        })
        .await;
        assert!(app.active_dialog().is_none());
        coordinator.shutdown().await;
    }

    #[tokio::test]
    async fn maintenance_service_workspace_reports_nonzero_without_export_evidence() {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let (mut app, mut coordinator, capability_request) =
            refreshed_service_coordinator(&fixture, "#!/bin/sh\necho denied >&2\nexit 7\n").await;
        let destination = fixture.build.join("failed-export.conf");
        let request = yoctui_model::PrServiceRequest::new(
            PrServiceOperation::Export,
            destination,
            fixture.build.clone(),
            "localhost:8585".into(),
        )
        .unwrap();
        coordinator
            .handle_effect(
                &mut app,
                Effect::Maintenance(MaintenanceEffect::PreviewPrService {
                    capability_request,
                    request,
                }),
            )
            .await;
        poll_until(&mut coordinator, &mut app, |app, _| {
            app.active_dialog().is_some()
        })
        .await;
        let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog().cloned() else {
            panic!("PR confirmation is absent");
        };
        let yoctui_model::MaintenanceDialog::Confirm(preview) = dialog.as_ref() else {
            panic!("wrong PR confirmation");
        };
        let effect = update(
            &mut app,
            Action::Maintenance(MaintenanceAction::ConfirmOperation(preview.clone())),
        )
        .unwrap();
        coordinator.handle_effect(&mut app, effect).await;
        poll_until(&mut coordinator, &mut app, |app, coordinator| {
            !coordinator.operation_active()
                && app
                    .maintenance
                    .sessions
                    .back()
                    .is_some_and(|session| session.status.is_terminal())
        })
        .await;
        let session = app.maintenance.sessions.back().unwrap();
        assert_eq!(session.status, MaintenanceSessionStatus::Failed);
        assert!(session.output.iter().any(|line| line.text == "denied"));
        assert!(app.maintenance.evidence.is_empty());
        coordinator.shutdown().await;
    }

    #[tokio::test]
    async fn maintenance_release_workspace_builds_exact_previews_for_every_form() {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let (mut app, mut coordinator, capability_request, history) =
            refreshed_release_coordinator(
                &fixture,
                "#!/bin/sh\nexit 0\n",
                "#!/bin/sh\nprintf 'comparison\\n'\n",
                "#!/bin/sh\nexit 0\n",
            )
            .await;

        let locked = fixture.root.join("locked.inc");
        let input = fixture.root.join("input-cache");
        let output = fixture.root.join("output-cache");
        fs::write(&locked, b"SIGGEN_LOCKEDSIGS = \"\"\n").unwrap();
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&output).unwrap();
        let requests = [
            Effect::Maintenance(MaintenanceEffect::PreviewLockedSignatureCache {
                capability_request,
                request: LockedSignatureCacheRequest::new(
                    locked,
                    input,
                    output,
                    "ubuntu".into(),
                    None,
                )
                .unwrap(),
            }),
            Effect::Maintenance(MaintenanceEffect::PreviewBuildHistoryComparison {
                capability_request,
                request: BuildComparisonRequest::new(BuildComparisonRequest {
                    repository: history,
                    from_revision: Some("HEAD^".into()),
                    to_revision: Some("HEAD".into()),
                    report_version: true,
                    report_all: false,
                    signatures: true,
                    signature_diff: false,
                    exclude_paths: vec!["images/*".into()],
                    no_colour: true,
                })
                .unwrap(),
            }),
            Effect::Maintenance(MaintenanceEffect::PreviewGitArchive {
                capability_request,
                request: GitArchiveRequest::new(GitArchiveRequest {
                    data_dir: fixture.root.clone(),
                    git_dir: fixture.root.join("release.git"),
                    create: true,
                    bare: false,
                    create_tag: false,
                    branch_name: "release".into(),
                    tag_name: None,
                    commit_subject: "Release".into(),
                    commit_body: String::new(),
                    tag_subject: "Tag".into(),
                    tag_body: String::new(),
                    exclusions: Vec::new(),
                    notes: Vec::new(),
                    push_remote: None,
                })
                .unwrap(),
            }),
        ];
        let expected = [
            MaintenanceTool::LockedSignatureCache,
            MaintenanceTool::BuildHistoryDiff,
            MaintenanceTool::GitArchive,
        ];
        for (index, (effect, tool)) in requests.into_iter().zip(expected).enumerate() {
            app.dialogs.clear();
            coordinator.handle_effect(&mut app, effect).await;
            poll_until(&mut coordinator, &mut app, |app, _| {
                matches!(
                    app.active_dialog(),
                    Some(yoctui_model::Dialog::Maintenance(dialog))
                        if matches!(dialog.as_ref(), yoctui_model::MaintenanceDialog::Confirm(_))
                )
            })
            .await;
            let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog() else {
                panic!("release confirmation is absent");
            };
            let yoctui_model::MaintenanceDialog::Confirm(preview) = dialog.as_ref() else {
                panic!("wrong release confirmation");
            };
            assert_eq!(preview.operation.tool(), tool);
            assert!(!preview.arguments.is_empty());
            if index == 2 {
                assert!(!preview.operation.network_side_effect());
            }
        }
        coordinator.shutdown().await;
    }

    #[tokio::test]
    async fn maintenance_release_workspace_defers_push_until_local_head_success() {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let script = "#!/bin/sh\nmkdir -p \"$2\"\nprintf 'ref: refs/heads/main\\n' > \"$2/HEAD\"\n";
        let (mut app, mut coordinator, capability_request, _) = refreshed_release_coordinator(
            &fixture,
            "#!/bin/sh\nexit 0\n",
            "#!/bin/sh\nprintf 'comparison\\n'\n",
            script,
        )
        .await;
        let request = GitArchiveRequest::new(GitArchiveRequest {
            data_dir: fixture.root.clone(),
            git_dir: fixture.root.join("push-release.git"),
            create: true,
            bare: true,
            create_tag: false,
            branch_name: "release".into(),
            tag_name: None,
            commit_subject: "Release".into(),
            commit_body: String::new(),
            tag_subject: "Tag".into(),
            tag_body: String::new(),
            exclusions: Vec::new(),
            notes: Vec::new(),
            push_remote: Some("origin".into()),
        })
        .unwrap();
        coordinator
            .handle_effect(
                &mut app,
                Effect::Maintenance(MaintenanceEffect::PreviewGitArchive {
                    capability_request,
                    request,
                }),
            )
            .await;
        poll_until(&mut coordinator, &mut app, |app, _| {
            app.active_dialog().is_some()
        })
        .await;
        let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog().cloned() else {
            panic!("local archive confirmation is absent");
        };
        let yoctui_model::MaintenanceDialog::Confirm(local) = dialog.as_ref() else {
            panic!("wrong local archive confirmation");
        };
        assert!(!local.operation.network_side_effect());
        let effect = update(
            &mut app,
            Action::Maintenance(MaintenanceAction::ConfirmOperation(local.clone())),
        )
        .unwrap();
        coordinator.handle_effect(&mut app, effect).await;
        poll_until(&mut coordinator, &mut app, |app, coordinator| {
            !coordinator.operation_active()
                && matches!(
                    app.active_dialog(),
                    Some(yoctui_model::Dialog::Maintenance(dialog))
                        if matches!(dialog.as_ref(), yoctui_model::MaintenanceDialog::Confirm(preview) if preview.operation.network_side_effect())
                )
        })
        .await;
        assert!(
            app.maintenance
                .evidence
                .iter()
                .any(|evidence| evidence.label == "Git archive HEAD")
        );

        let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog().cloned() else {
            panic!("push confirmation is absent");
        };
        let yoctui_model::MaintenanceDialog::Confirm(push) = dialog.as_ref() else {
            panic!("wrong push confirmation");
        };
        let _ = update(
            &mut app,
            Action::Maintenance(MaintenanceAction::ConfirmOperation(push.clone())),
        );
        assert!(matches!(
            app.active_dialog(),
            Some(yoctui_model::Dialog::Maintenance(dialog))
                if matches!(dialog.as_ref(), yoctui_model::MaintenanceDialog::ConfirmNetworkPush(_))
        ));
        fs::write(
            fixture.root.join("push-release.git/HEAD"),
            b"changed local head\n",
        )
        .unwrap();
        let effect = update(
            &mut app,
            Action::Maintenance(MaintenanceAction::ConfirmNetworkPush(push.clone())),
        )
        .unwrap();
        coordinator.handle_effect(&mut app, effect).await;
        poll_until(&mut coordinator, &mut app, |app, coordinator| {
            !coordinator.operation_active()
                && app.maintenance.sessions.len() == 2
                && app
                    .maintenance
                    .sessions
                    .back()
                    .is_some_and(|session| session.status.is_terminal())
        })
        .await;
        let push_session = app.maintenance.sessions.back().unwrap();
        assert_eq!(push_session.status, MaintenanceSessionStatus::Failed);
        assert!(
            push_session
                .message
                .as_deref()
                .is_some_and(|message| message.contains("changed"))
        );
        coordinator.shutdown().await;
    }

    #[tokio::test]
    async fn maintenance_release_workspace_installs_changed_locked_cache_evidence() {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let locked_script = "#!/bin/sh\nmkdir -p \"$3/aa\"\nprintf 'sig\\n' > \"$3/aa/new.siginfo\"\nprintf 'locked cache ready\\n'\n";
        let (mut app, mut coordinator, capability_request, _) = refreshed_release_coordinator(
            &fixture,
            locked_script,
            "#!/bin/sh\nexit 0\n",
            "#!/bin/sh\nexit 0\n",
        )
        .await;
        let locked = fixture.root.join("locked.inc");
        let input = fixture.root.join("input-cache");
        let output = fixture.root.join("output-cache");
        fs::write(&locked, b"SIGGEN_LOCKEDSIGS = \"\"\n").unwrap();
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&output).unwrap();
        coordinator
            .handle_effect(
                &mut app,
                Effect::Maintenance(MaintenanceEffect::PreviewLockedSignatureCache {
                    capability_request,
                    request: LockedSignatureCacheRequest::new(
                        locked,
                        input,
                        output.clone(),
                        "ubuntu".into(),
                        None,
                    )
                    .unwrap(),
                }),
            )
            .await;
        poll_until(&mut coordinator, &mut app, |app, _| {
            app.active_dialog().is_some()
        })
        .await;
        let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog().cloned() else {
            panic!("locked-cache confirmation is absent");
        };
        let yoctui_model::MaintenanceDialog::Confirm(preview) = dialog.as_ref() else {
            panic!("wrong locked-cache confirmation");
        };
        let effect = update(
            &mut app,
            Action::Maintenance(MaintenanceAction::ConfirmOperation(preview.clone())),
        )
        .unwrap();
        coordinator.handle_effect(&mut app, effect).await;
        poll_until(&mut coordinator, &mut app, |app, coordinator| {
            !coordinator.operation_active()
                && app
                    .maintenance
                    .sessions
                    .back()
                    .is_some_and(|session| session.status.is_terminal())
        })
        .await;
        let session = app.maintenance.sessions.back().unwrap();
        assert_eq!(session.status, MaintenanceSessionStatus::Succeeded);
        assert!(
            session
                .output
                .iter()
                .any(|line| line.text == "locked cache ready")
        );
        assert!(app.maintenance.evidence.iter().any(|evidence| {
            evidence.identity.path == output.join("aa/new.siginfo")
                && evidence.label == "created locked-signature cache evidence"
        }));
        coordinator.shutdown().await;
    }

    #[tokio::test]
    async fn maintenance_release_workspace_bounds_comparison_output_and_reports_nonzero() {
        for (script, expected, expected_dropped) in [
            (
                "#!/bin/sh\ni=0\nwhile [ \"$i\" -lt 520 ]; do printf 'comparison-%s\\n' \"$i\"; i=$((i + 1)); done\n",
                MaintenanceSessionStatus::Succeeded,
                8,
            ),
            (
                "#!/bin/sh\nprintf 'comparison denied\\n' >&2\nexit 7\n",
                MaintenanceSessionStatus::Failed,
                0,
            ),
        ] {
            let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
            let (mut app, mut coordinator, capability_request, history) =
                refreshed_release_coordinator(
                    &fixture,
                    "#!/bin/sh\nexit 0\n",
                    script,
                    "#!/bin/sh\nexit 0\n",
                )
                .await;
            let request = BuildComparisonRequest::new(BuildComparisonRequest {
                repository: history,
                from_revision: Some("HEAD^".into()),
                to_revision: Some("HEAD".into()),
                report_version: false,
                report_all: false,
                signatures: false,
                signature_diff: false,
                exclude_paths: Vec::new(),
                no_colour: true,
            })
            .unwrap();
            coordinator
                .handle_effect(
                    &mut app,
                    Effect::Maintenance(MaintenanceEffect::PreviewBuildHistoryComparison {
                        capability_request,
                        request,
                    }),
                )
                .await;
            poll_until(&mut coordinator, &mut app, |app, _| {
                app.active_dialog().is_some()
            })
            .await;
            let Some(yoctui_model::Dialog::Maintenance(dialog)) = app.active_dialog().cloned()
            else {
                panic!("comparison confirmation is absent");
            };
            let yoctui_model::MaintenanceDialog::Confirm(preview) = dialog.as_ref() else {
                panic!("wrong comparison confirmation");
            };
            let effect = update(
                &mut app,
                Action::Maintenance(MaintenanceAction::ConfirmOperation(preview.clone())),
            )
            .unwrap();
            coordinator.handle_effect(&mut app, effect).await;
            poll_until(&mut coordinator, &mut app, |app, coordinator| {
                !coordinator.operation_active()
                    && app
                        .maintenance
                        .sessions
                        .back()
                        .is_some_and(|session| session.status.is_terminal())
            })
            .await;
            let session = app.maintenance.sessions.back().unwrap();
            assert_eq!(session.status, expected);
            assert_eq!(session.dropped_lines, expected_dropped);
            assert!(session.output.len() <= MAX_MAINTENANCE_OUTPUT);
            assert!(app.maintenance.evidence.is_empty());
            coordinator.shutdown().await;
        }
    }

    #[tokio::test]
    async fn maintenance_release_workspace_rejects_stale_and_invalid_requests() {
        let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
        let (mut app, mut coordinator, capability_request, _) = refreshed_release_coordinator(
            &fixture,
            "#!/bin/sh\nexit 0\n",
            "#!/bin/sh\nexit 0\n",
            "#!/bin/sh\nexit 0\n",
        )
        .await;
        let locked = fixture.root.join("locked.inc");
        let input = fixture.root.join("input-cache");
        let output = fixture.root.join("output-cache");
        fs::write(&locked, b"SIGGEN_LOCKEDSIGS = \"\"\n").unwrap();
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&output).unwrap();
        let valid =
            LockedSignatureCacheRequest::new(locked, input, output, "ubuntu".into(), None).unwrap();
        coordinator
            .handle_effect(
                &mut app,
                Effect::Maintenance(MaintenanceEffect::PreviewLockedSignatureCache {
                    capability_request: capability_request + 1,
                    request: valid,
                }),
            )
            .await;
        poll_until(&mut coordinator, &mut app, |app, _| {
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("capability changed"))
        })
        .await;
        assert!(app.active_dialog().is_none());

        app.notification = None;
        coordinator
            .handle_effect(
                &mut app,
                Effect::Maintenance(MaintenanceEffect::PreviewLockedSignatureCache {
                    capability_request,
                    request: LockedSignatureCacheRequest {
                        locked_signatures: PathBuf::from("relative.inc"),
                        input_cache: fixture.root.clone(),
                        output_cache: fixture.build.clone(),
                        native_lsb: "ubuntu".into(),
                        filter: None,
                    },
                }),
            )
            .await;
        poll_until(&mut coordinator, &mut app, |app, _| {
            app.notification.as_deref().is_some_and(|message| {
                message.contains("invalid")
                    || message.contains("absolute")
                    || message.contains("unsafe")
            })
        })
        .await;
        assert!(app.active_dialog().is_none());
        coordinator.shutdown().await;
    }
}
