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

mod coordinator;
mod operations;
mod previews;

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
#[path = "tests/maintenance_cli/mod.rs"]
mod tests;
