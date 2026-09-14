//! Maintenance fixtures.
use super::*;

pub(crate) fn maintenance_identity(path: &str) -> yoctui_model::MaintenanceFileIdentity {
    yoctui_model::MaintenanceFileIdentity::new(path.into(), 42, UNIX_EPOCH).unwrap()
}

pub(crate) fn maintenance_preview(id: u64) -> yoctui_model::MaintenanceOperationPreview {
    yoctui_model::MaintenanceOperationPreview::new(
        id,
        7,
        yoctui_model::MaintenanceOperation::SstateReadiness(
            yoctui_model::SstateReadinessRequest::new(
                vec!["core-image-minimal".into()],
                yoctui_model::SstateReadinessMode::IsolatedTmpdir,
                Some("/build/sstate-report.txt".into()),
                None,
                60,
            )
            .unwrap(),
        ),
        vec![
            "/tools/oe-check-sstate".into(),
            "--output".into(),
            "/build/sstate-report.txt".into(),
            "core-image-minimal".into(),
        ],
        vec!["fixture evidence is not live compatibility".into()],
    )
    .unwrap()
}

pub(crate) fn maintenance_workflow_ui_app() -> App {
    let mut app = App::new(100, 10_000);
    app.screen = Screen::Maintenance;
    app.focus = FocusTarget::Workspace;
    let available = |tool, path| MaintenanceToolCapability::Available {
        tool,
        executable: maintenance_identity(path),
        interface: if matches!(
            tool,
            MaintenanceTool::CreatePullRequest
                | MaintenanceTool::SendPullRequest
                | MaintenanceTool::SendErrorReport
                | MaintenanceTool::Toaster
        ) {
            MaintenanceToolInterface::DetectionOnly
        } else {
            MaintenanceToolInterface::Native
        },
    };
    let snapshot = yoctui_model::MaintenanceCapabilitySnapshot::new(
        yoctui_model::MaintenanceMetadata::new(yoctui_model::MaintenanceMetadata {
            build_dir: Some("/build".into()),
            sstate_dir: Some("/cache/sstate".into()),
            tmp_dir: Some("/build/tmp".into()),
            stamps_dirs: vec!["/build/tmp/stamps".into()],
            buildhistory_dir: Some("/build/buildhistory".into()),
            prserv_host: Some("localhost:8585".into()),
            hashserve: Some("auto".into()),
            hashserve_upstream: None,
            signature_handler: Some("OEEquivHash".into()),
            native_lsb: Some("ubuntu-24.04".into()),
            machine: Some("qemux86-64".into()),
            distro: Some("poky".into()),
        })
        .unwrap(),
        vec![
            available(MaintenanceTool::OeCheckSstate, "/tools/oe-check-sstate"),
            available(
                MaintenanceTool::SstateCacheManagement,
                "/tools/sstate-cache-management.py",
            ),
            available(MaintenanceTool::PrServiceTool, "/tools/bitbake-prserv-tool"),
            available(
                MaintenanceTool::LockedSignatureCache,
                "/tools/gen-lockedsig-cache",
            ),
            available(
                MaintenanceTool::BuildHistoryDiff,
                "/tools/buildhistory-diff",
            ),
            MaintenanceToolCapability::Unavailable {
                tool: MaintenanceTool::BuildCompare,
                reason: "distinct optional interface is unsupported".into(),
            },
            available(MaintenanceTool::GitArchive, "/tools/oe-git-archive"),
            available(
                MaintenanceTool::CreatePullRequest,
                "/tools/create-pull-request",
            ),
            available(MaintenanceTool::SendPullRequest, "/tools/send-pull-request"),
            available(MaintenanceTool::SendErrorReport, "/tools/send-error-report"),
            available(MaintenanceTool::Toaster, "/tools/toaster"),
        ],
        vec!["bounded fixture snapshot".into()],
    )
    .unwrap();
    app.maintenance.capability = MaintenanceCapability::Partial {
        request: 7,
        limitations: snapshot.limitations.clone(),
        snapshot,
    };

    let endpoint = yoctui_model::ServiceEndpointDiagnostic::new(
        yoctui_model::ServiceEndpointRole::Primary,
        "localhost:8585".into(),
        yoctui_model::ServiceLocation::Local,
        yoctui_model::ServiceReachability::Reachable,
        None,
    )
    .unwrap();
    let service = yoctui_model::ServiceDiagnostic::new(
        yoctui_model::ServiceKind::Pr,
        yoctui_model::ServiceState::Reachable,
        vec![endpoint],
        vec![yoctui_model::ServiceProcessEvidence::new(42, "bitbake-prserv".into()).unwrap()],
        vec!["process evidence is observational".into()],
    )
    .unwrap();
    app.maintenance.services = MaintenanceServiceDiagnostics::Partial {
        request: 8,
        services: vec![service],
        limitations: vec!["remote endpoint was not probed".into()],
    };

    let directory = |path: &str| {
        yoctui_model::MaintenanceDirectoryIdentity::new(path.into(), UNIX_EPOCH).unwrap()
    };
    let integrations = yoctui_model::MaintenanceIntegrationsSnapshot::new(
        yoctui_model::MaintenanceIntegrationsSnapshot {
            pull_request: yoctui_model::OptionalPullRequestIntegration {
                state: yoctui_model::OptionalIntegrationState::Available,
                create_helper: Some(maintenance_identity("/tools/create-pull-request")),
                send_helper: Some(maintenance_identity("/tools/send-pull-request")),
                worktree: Some(yoctui_model::MaintenanceGitWorktreeIdentity {
                    root: directory("/sources/poky"),
                    head: maintenance_identity("/sources/poky/.git/HEAD"),
                }),
                limitations: Vec::new(),
            },
            error_report: yoctui_model::OptionalErrorReportIntegration {
                state: yoctui_model::OptionalIntegrationState::Partial,
                helper: Some(maintenance_identity("/tools/send-error-report")),
                candidate_report: None,
                limitations: vec!["candidate report unavailable".into()],
            },
            repo_manifest: yoctui_model::OptionalRepoManifestIntegration {
                state: yoctui_model::OptionalIntegrationState::Available,
                repo_executable: Some(maintenance_identity("/tools/repo")),
                workspace: Some(directory("/workspace")),
                manifest: Some(maintenance_identity(
                    "/workspace/.repo/manifests/default.xml",
                )),
                limitations: Vec::new(),
            },
            toaster: yoctui_model::OptionalToasterIntegration {
                state: yoctui_model::OptionalIntegrationState::Available,
                executable: Some(maintenance_identity("/tools/toaster")),
                configurations: vec![maintenance_identity("/config/toaster.conf")],
                observed_processes: vec![
                    yoctui_model::ServiceProcessEvidence::new(84, "toaster".into()).unwrap(),
                ],
                limitations: vec!["observational only".into()],
            },
            limitations: vec!["detection only".into()],
        },
    )
    .unwrap();
    app.maintenance.integrations = MaintenanceIntegrationDiagnostics::Partial {
        request: 9,
        limitations: integrations.limitations.clone(),
        snapshot: integrations,
    };
    app.maintenance.pending = Some(maintenance_preview(10));
    app.maintenance
        .sessions
        .push_back(yoctui_model::MaintenanceSession {
            id: yoctui_model::MaintenanceSessionId(11),
            preview: maintenance_preview(11),
            status: MaintenanceSessionStatus::Succeeded,
            started_at: Some(UNIX_EPOCH),
            finished_at: Some(UNIX_EPOCH),
            output: std::collections::VecDeque::from([
                yoctui_model::MaintenanceOutputLine {
                    stream: yoctui_model::MaintenanceOutputStream::Stdout,
                    text: "created report".into(),
                },
                yoctui_model::MaintenanceOutputLine {
                    stream: yoctui_model::MaintenanceOutputStream::Stderr,
                    text: "bounded warning".into(),
                },
            ]),
            dropped_lines: 3,
            exit_code: Some(0),
            message: None,
        });
    app.maintenance.evidence = vec![
        yoctui_model::MaintenanceEvidence::new(
            maintenance_identity("/build/sstate-report.txt"),
            "sstate report".into(),
        )
        .unwrap(),
    ];
    app
}
