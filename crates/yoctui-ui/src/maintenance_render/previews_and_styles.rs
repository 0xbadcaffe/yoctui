pub(crate) fn maintenance_preview_text(preview: &MaintenanceOperationPreview) -> String {
    format!(
        "Pending preview {}\nOperation: {}\nDestructive: {}\nNetwork: {}\nIndexed native vector:\n{}\nLimitations:\n- {}",
        preview.id,
        maintenance_operation_label(&preview.operation),
        preview.operation.destructive(),
        preview.operation.network_side_effect(),
        indexed_arguments(&preview.arguments),
        if preview.limitations.is_empty() {
            "none".into()
        } else {
            preview.limitations.join("\n- ")
        },
    )
}

pub(crate) fn maintenance_tools_for_view(view: MaintenanceView) -> &'static [MaintenanceTool] {
    match view {
        MaintenanceView::Sstate => &[
            MaintenanceTool::OeCheckSstate,
            MaintenanceTool::SstateCacheManagement,
        ],
        MaintenanceView::Services => &[MaintenanceTool::PrServiceTool],
        MaintenanceView::Release => &[
            MaintenanceTool::LockedSignatureCache,
            MaintenanceTool::BuildHistoryDiff,
            MaintenanceTool::BuildCompare,
            MaintenanceTool::GitArchive,
        ],
        MaintenanceView::Integrations => &[
            MaintenanceTool::CreatePullRequest,
            MaintenanceTool::SendPullRequest,
            MaintenanceTool::SendErrorReport,
            MaintenanceTool::Toaster,
        ],
    }
}

pub(crate) fn maintenance_view_label(view: MaintenanceView) -> &'static str {
    match view {
        MaintenanceView::Sstate => "Sstate",
        MaintenanceView::Services => "Services",
        MaintenanceView::Release => "Release",
        MaintenanceView::Integrations => "Integrations",
    }
}

pub(crate) fn maintenance_tool_label(tool: MaintenanceTool) -> &'static str {
    match tool {
        MaintenanceTool::OeCheckSstate => "oe-check-sstate",
        MaintenanceTool::SstateCacheManagement => "sstate cache management",
        MaintenanceTool::PrServiceTool => "bitbake-prserv-tool",
        MaintenanceTool::LockedSignatureCache => "gen-lockedsig-cache",
        MaintenanceTool::BuildHistoryDiff => "buildhistory-diff",
        MaintenanceTool::BuildCompare => "build-compare",
        MaintenanceTool::GitArchive => "oe-git-archive",
        MaintenanceTool::CreatePullRequest => "create-pull-request",
        MaintenanceTool::SendPullRequest => "send-pull-request",
        MaintenanceTool::SendErrorReport => "send-error-report",
        MaintenanceTool::Toaster => "Toaster",
    }
}

pub(crate) fn maintenance_interface_label(interface: MaintenanceToolInterface) -> &'static str {
    match interface {
        MaintenanceToolInterface::Native => "native",
        MaintenanceToolInterface::SstatePython => "current Python",
        MaintenanceToolInterface::SstateLegacyShell => "legacy shell",
        MaintenanceToolInterface::DetectionOnly => "detection only",
    }
}

pub(crate) fn maintenance_integration_rows(
    snapshot: &MaintenanceIntegrationsSnapshot,
) -> [(&'static str, yoctui_model::OptionalIntegrationState); 4] {
    [
        ("Pull request", snapshot.pull_request.state),
        ("Error report", snapshot.error_report.state),
        ("Repo manifest", snapshot.repo_manifest.state),
        ("Toaster", snapshot.toaster.state),
    ]
}

pub(crate) fn service_state_style(app: &App, state: yoctui_model::ServiceState) -> Style {
    let palette = ThemePalette::for_app(app);
    match state {
        yoctui_model::ServiceState::Reachable => palette.role(palette.success, Modifier::BOLD),
        yoctui_model::ServiceState::Unreachable => palette.role(palette.error, Modifier::BOLD),
        yoctui_model::ServiceState::Partial => palette.role(palette.warning, Modifier::BOLD),
        yoctui_model::ServiceState::Configured => {
            palette.role(palette.informational, Modifier::BOLD)
        }
        yoctui_model::ServiceState::Disabled | yoctui_model::ServiceState::Unavailable => {
            palette.role(palette.disabled, Modifier::DIM)
        }
    }
}

pub(crate) fn optional_state_style(
    app: &App,
    state: yoctui_model::OptionalIntegrationState,
) -> Style {
    let palette = ThemePalette::for_app(app);
    match state {
        yoctui_model::OptionalIntegrationState::Available => {
            palette.role(palette.success, Modifier::BOLD)
        }
        yoctui_model::OptionalIntegrationState::Partial => {
            palette.role(palette.warning, Modifier::BOLD)
        }
        yoctui_model::OptionalIntegrationState::Unavailable => {
            palette.role(palette.disabled, Modifier::DIM)
        }
    }
}

pub(crate) fn maintenance_session_style(app: &App, status: MaintenanceSessionStatus) -> Style {
    let palette = ThemePalette::for_app(app);
    match status {
        MaintenanceSessionStatus::Succeeded => palette.role(palette.success, Modifier::BOLD),
        MaintenanceSessionStatus::Failed
        | MaintenanceSessionStatus::TimedOut
        | MaintenanceSessionStatus::Lost => palette.role(palette.error, Modifier::BOLD),
        MaintenanceSessionStatus::Cancelled => palette.role(palette.warning, Modifier::BOLD),
        MaintenanceSessionStatus::Queued
        | MaintenanceSessionStatus::Running
        | MaintenanceSessionStatus::Cancelling => {
            palette.role(palette.informational, Modifier::BOLD)
        }
    }
}
