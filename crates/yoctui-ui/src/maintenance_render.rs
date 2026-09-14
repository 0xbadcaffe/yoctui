//! Maintenance render.
use super::*;

pub(crate) fn maintenance_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let mut lines = vec![Line::from(
        [
            MaintenanceView::Sstate,
            MaintenanceView::Services,
            MaintenanceView::Release,
            MaintenanceView::Integrations,
        ]
        .into_iter()
        .flat_map(|view| {
            let style = if app.maintenance.view == view {
                palette.role(palette.accent, Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                palette.role(palette.disabled, Modifier::DIM)
            };
            [
                Span::styled(format!(" {} ", maintenance_view_label(view)), style),
                Span::raw(" "),
            ]
        })
        .collect::<Vec<_>>(),
    )];
    lines.push(Line::from(""));
    match &app.maintenance.capability {
        MaintenanceCapability::NotInspected => lines.push(Line::styled(
            "Capability not inspected; press r to inspect.",
            palette.role(palette.disabled, Modifier::DIM),
        )),
        MaintenanceCapability::Loading(request) => lines.push(Line::styled(
            format!("Inspecting capability (request {request})…"),
            palette.role(palette.informational, Modifier::BOLD),
        )),
        MaintenanceCapability::Available { snapshot, .. } => {
            maintenance_capability_lines(app, snapshot, &mut lines, palette)
        }
        MaintenanceCapability::Partial {
            snapshot,
            limitations,
            ..
        } => {
            lines.push(Line::styled(
                format!("Partial capability: {} limitation(s)", limitations.len()),
                palette.role(palette.warning, Modifier::BOLD),
            ));
            maintenance_capability_lines(app, snapshot, &mut lines, palette);
        }
        MaintenanceCapability::Failed { message, .. } => lines.push(Line::styled(
            format!("Capability inspection failed: {message}"),
            palette.role(palette.error, Modifier::BOLD),
        )),
    }
    if app.maintenance.view == MaintenanceView::Services {
        lines.push(Line::from(""));
        lines.push(Line::styled("Service diagnostics", palette.focus()));
        match &app.maintenance.services {
            MaintenanceServiceDiagnostics::NotInspected => lines.push(Line::raw("not inspected")),
            MaintenanceServiceDiagnostics::Loading(request) => {
                lines.push(Line::raw(format!("loading request {request}")))
            }
            MaintenanceServiceDiagnostics::Available { services, .. }
            | MaintenanceServiceDiagnostics::Partial { services, .. } => {
                for service in services {
                    lines.push(Line::styled(
                        format!(
                            "  {:?}: {:?} ({} endpoint(s), {} process(es))",
                            service.kind,
                            service.state,
                            service.endpoints.len(),
                            service.process_evidence.len()
                        ),
                        service_state_style(app, service.state),
                    ));
                }
            }
            MaintenanceServiceDiagnostics::Failed { message, .. } => lines.push(Line::styled(
                format!("failed: {message}"),
                palette.role(palette.error, Modifier::BOLD),
            )),
        }
    }
    if app.maintenance.view == MaintenanceView::Integrations {
        lines.push(Line::from(""));
        lines.push(Line::styled("Integration readiness", palette.focus()));
        match &app.maintenance.integrations {
            MaintenanceIntegrationDiagnostics::NotInspected => {
                lines.push(Line::raw("not inspected"))
            }
            MaintenanceIntegrationDiagnostics::Loading(request) => {
                lines.push(Line::raw(format!("loading request {request}")))
            }
            MaintenanceIntegrationDiagnostics::Available { snapshot, .. }
            | MaintenanceIntegrationDiagnostics::Partial { snapshot, .. } => {
                for (label, state) in maintenance_integration_rows(snapshot) {
                    lines.push(Line::styled(
                        format!("  {label}: {state:?}"),
                        optional_state_style(app, state),
                    ));
                }
            }
            MaintenanceIntegrationDiagnostics::Failed { message, .. } => lines.push(Line::styled(
                format!("failed: {message}"),
                palette.role(palette.error, Modifier::BOLD),
            )),
        }
    }
    if let Some(session) = app.maintenance.sessions.back() {
        lines.push(Line::from(""));
        lines.push(Line::styled(
            format!(
                "Session {}: {:?}  exit {}  dropped {}",
                session.id.0,
                session.status,
                session
                    .exit_code
                    .map_or_else(|| "--".into(), |value| value.to_string()),
                session.dropped_lines,
            ),
            maintenance_session_style(app, session.status),
        ));
        for output in session.output.iter().rev().take(3).rev() {
            lines.push(Line::raw(format!("  {:?}: {}", output.stream, output.text)));
        }
    }
    yocto_logs::render(
        frame,
        area,
        Text::from(lines),
        pane_block(
            app,
            &format!(
                "Maintenance · {}",
                maintenance_view_label(app.maintenance.view)
            ),
            app.focus == FocusTarget::Workspace,
        ),
        true,
    );
}

pub(crate) fn maintenance_capability_lines(
    app: &App,
    snapshot: &MaintenanceCapabilitySnapshot,
    lines: &mut Vec<Line<'static>>,
    palette: ThemePalette,
) {
    for (index, tool) in maintenance_tools_for_view(app.maintenance.view)
        .iter()
        .enumerate()
    {
        let (text, style) = match snapshot.capability(*tool) {
            Some(MaintenanceToolCapability::Available {
                executable,
                interface,
                ..
            }) => (
                format!(
                    "{}  available ({})  {}",
                    maintenance_tool_label(*tool),
                    maintenance_interface_label(*interface),
                    executable.path.display()
                ),
                palette.role(palette.success, Modifier::BOLD),
            ),
            Some(MaintenanceToolCapability::Unavailable { reason, .. }) => (
                format!("{}  unavailable: {reason}", maintenance_tool_label(*tool)),
                palette.role(palette.disabled, Modifier::DIM),
            ),
            None => (
                format!(
                    "{}  unavailable: capability not reported",
                    maintenance_tool_label(*tool)
                ),
                palette.role(palette.disabled, Modifier::DIM),
            ),
        };
        let selected = index == app.maintenance.selection();
        lines.push(Line::styled(
            format!("{} {text}", if selected { "▶" } else { " " }),
            if selected {
                selected_style(app, true)
            } else {
                style
            },
        ));
    }
}

pub(crate) fn maintenance_inspector_text(app: &App) -> String {
    let mut sections = vec![format!(
        "View: {}",
        maintenance_view_label(app.maintenance.view)
    )];
    match &app.maintenance.capability {
        MaintenanceCapability::NotInspected => sections.push("Capability: not inspected".into()),
        MaintenanceCapability::Loading(request) => {
            sections.push(format!("Capability: loading request {request}"));
        }
        MaintenanceCapability::Failed { request, message } => {
            sections.push(format!("Capability request {request} failed: {message}"));
        }
        MaintenanceCapability::Available { request, snapshot }
        | MaintenanceCapability::Partial {
            request, snapshot, ..
        } => {
            sections.push(format!("Capability request: {request}"));
            sections.push(maintenance_metadata_text(snapshot));
            if let Some(tool) =
                maintenance_tools_for_view(app.maintenance.view).get(app.maintenance.selection())
            {
                sections.push(maintenance_tool_detail(snapshot, *tool));
            }
            if !snapshot.limitations.is_empty() {
                sections.push(format!(
                    "Capability limitations:\n- {}",
                    snapshot.limitations.join("\n- ")
                ));
            }
        }
    }
    if app.maintenance.view == MaintenanceView::Services {
        sections.push(maintenance_services_text(&app.maintenance.services));
    }
    if app.maintenance.view == MaintenanceView::Integrations {
        sections.push(maintenance_integrations_text(&app.maintenance.integrations));
    }
    if let Some(preview) = app.maintenance.pending.as_ref() {
        sections.push(maintenance_preview_text(preview));
    }
    if let Some(session) = app.maintenance.sessions.back() {
        let output = session
            .output
            .iter()
            .map(|line| format!("{:?}: {}", line.stream, line.text))
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(format!(
            "Latest session {}\nOperation: {}\nStatus: {:?}\nStarted: {}\nFinished: {}\nExit: {}\nDropped lines: {}\nMessage: {}\nOutput:\n{}",
            session.id.0,
            maintenance_operation_label(&session.preview.operation),
            session.status,
            session
                .started_at
                .map(timestamp_text)
                .unwrap_or_else(|| "not started".into()),
            session
                .finished_at
                .map(timestamp_text)
                .unwrap_or_else(|| "not finished".into()),
            session
                .exit_code
                .map_or_else(|| "unavailable".into(), |value| value.to_string()),
            session.dropped_lines,
            session.message.as_deref().unwrap_or("none"),
            if output.is_empty() { "none" } else { &output },
        ));
    }
    if let Some(evidence) = app.maintenance.selected_evidence() {
        sections.push(format!(
            "Selected evidence\nLabel: {}\nPath: {}\nBytes: {}\nModified: {}",
            evidence.label,
            evidence.identity.path.display(),
            evidence.identity.byte_size,
            timestamp_text(evidence.identity.modified_at),
        ));
    } else {
        sections.push("Evidence: none".into());
    }
    sections.join("\n\n")
}

pub(crate) fn maintenance_metadata_text(snapshot: &MaintenanceCapabilitySnapshot) -> String {
    let metadata = &snapshot.metadata;
    let path = |value: Option<&std::path::PathBuf>| {
        value.map_or_else(|| "unavailable".into(), |path| path.display().to_string())
    };
    let stamps = if metadata.stamps_dirs.is_empty() {
        "unavailable".into()
    } else {
        metadata
            .stamps_dirs
            .iter()
            .map(|value| value.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "Metadata\nBuild: {}\nSstate: {}\nTmp: {}\nStamps: {}\nBuild history: {}\nPR service: {}\nHash service: {}\nHash upstream: {}\nSignature handler: {}\nNative LSB: {}\nMachine: {}\nDistro: {}",
        path(metadata.build_dir.as_ref()),
        path(metadata.sstate_dir.as_ref()),
        path(metadata.tmp_dir.as_ref()),
        stamps,
        path(metadata.buildhistory_dir.as_ref()),
        metadata.prserv_host.as_deref().unwrap_or("unavailable"),
        metadata.hashserve.as_deref().unwrap_or("unavailable"),
        metadata
            .hashserve_upstream
            .as_deref()
            .unwrap_or("unavailable"),
        metadata
            .signature_handler
            .as_deref()
            .unwrap_or("unavailable"),
        metadata.native_lsb.as_deref().unwrap_or("unavailable"),
        metadata.machine.as_deref().unwrap_or("unavailable"),
        metadata.distro.as_deref().unwrap_or("unavailable"),
    )
}

pub(crate) fn maintenance_tool_detail(
    snapshot: &MaintenanceCapabilitySnapshot,
    tool: MaintenanceTool,
) -> String {
    match snapshot.capability(tool) {
        Some(MaintenanceToolCapability::Available {
            executable,
            interface,
            ..
        }) => format!(
            "Selected capability\nTool: {}\nState: available\nInterface: {}\nExecutable: {}\nBytes: {}\nModified: {}",
            maintenance_tool_label(tool),
            maintenance_interface_label(*interface),
            executable.path.display(),
            executable.byte_size,
            timestamp_text(executable.modified_at),
        ),
        Some(MaintenanceToolCapability::Unavailable { reason, .. }) => format!(
            "Selected capability\nTool: {}\nState: unavailable\nReason: {reason}",
            maintenance_tool_label(tool)
        ),
        None => format!(
            "Selected capability\nTool: {}\nState: unavailable\nReason: capability not reported",
            maintenance_tool_label(tool)
        ),
    }
}

pub(crate) fn maintenance_services_text(state: &MaintenanceServiceDiagnostics) -> String {
    match state {
        MaintenanceServiceDiagnostics::NotInspected => "Service diagnostics: not inspected".into(),
        MaintenanceServiceDiagnostics::Loading(request) => {
            format!("Service diagnostics: loading request {request}")
        }
        MaintenanceServiceDiagnostics::Failed { request, message } => {
            format!("Service diagnostics request {request} failed: {message}")
        }
        MaintenanceServiceDiagnostics::Available { request, services } => {
            maintenance_service_records(*request, services, &[])
        }
        MaintenanceServiceDiagnostics::Partial {
            request,
            services,
            limitations,
        } => maintenance_service_records(*request, services, limitations),
    }
}

pub(crate) fn maintenance_service_records(
    request: u64,
    services: &[yoctui_model::ServiceDiagnostic],
    limitations: &[String],
) -> String {
    let details = services
        .iter()
        .map(|service| {
            let endpoints = service
                .endpoints
                .iter()
                .map(|endpoint| {
                    format!(
                        "  {:?} {} [{:?}, {:?}]{}",
                        endpoint.role,
                        endpoint.value,
                        endpoint.location,
                        endpoint.reachability,
                        endpoint
                            .limitation
                            .as_deref()
                            .map_or_else(String::new, |value| format!(" — {value}")),
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let processes = service
                .process_evidence
                .iter()
                .map(|process| {
                    format!(
                        "  PID {} {} (observational)",
                        process.pid, process.executable
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "{:?}: {:?}\nEndpoints:\n{}\nProcesses:\n{}\nLimitations:\n- {}",
                service.kind,
                service.state,
                if endpoints.is_empty() {
                    "  none"
                } else {
                    &endpoints
                },
                if processes.is_empty() {
                    "  none"
                } else {
                    &processes
                },
                if service.limitations.is_empty() {
                    "none".into()
                } else {
                    service.limitations.join("\n- ")
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "Service diagnostics request {request}\n{details}\nInspection limitations:\n- {}",
        if limitations.is_empty() {
            "none".into()
        } else {
            limitations.join("\n- ")
        }
    )
}

pub(crate) fn maintenance_integrations_text(state: &MaintenanceIntegrationDiagnostics) -> String {
    match state {
        MaintenanceIntegrationDiagnostics::NotInspected => {
            "Integration details: not inspected".into()
        }
        MaintenanceIntegrationDiagnostics::Loading(request) => {
            format!("Integration details: loading request {request}")
        }
        MaintenanceIntegrationDiagnostics::Failed { request, message } => {
            format!("Integration request {request} failed: {message}")
        }
        MaintenanceIntegrationDiagnostics::Available { request, snapshot }
        | MaintenanceIntegrationDiagnostics::Partial {
            request, snapshot, ..
        } => {
            let limitations = snapshot
                .limitations
                .iter()
                .chain(snapshot.pull_request.limitations.iter())
                .chain(snapshot.error_report.limitations.iter())
                .chain(snapshot.repo_manifest.limitations.iter())
                .chain(snapshot.toaster.limitations.iter())
                .cloned()
                .collect::<Vec<_>>();
            format!(
                "Integration request {request}\nPull request: {:?}\n  create: {}\n  send: {}\n  worktree: {}\n  HEAD: {}\nError report: {:?}\n  helper: {}\n  candidate: {}\nRepo manifest: {:?}\n  repo: {}\n  workspace: {}\n  manifest: {}\nToaster: {:?}\n  executable: {}\n  configurations: {}\n  observed processes: {}\n  process evidence is observational only",
                snapshot.pull_request.state,
                optional_file_path(snapshot.pull_request.create_helper.as_ref()),
                optional_file_path(snapshot.pull_request.send_helper.as_ref()),
                snapshot.pull_request.worktree.as_ref().map_or_else(
                    || "unavailable".into(),
                    |value| value.root.path.display().to_string()
                ),
                snapshot.pull_request.worktree.as_ref().map_or_else(
                    || "unavailable".into(),
                    |value| value.head.path.display().to_string()
                ),
                snapshot.error_report.state,
                optional_file_path(snapshot.error_report.helper.as_ref()),
                optional_file_path(snapshot.error_report.candidate_report.as_ref()),
                snapshot.repo_manifest.state,
                optional_file_path(snapshot.repo_manifest.repo_executable.as_ref()),
                snapshot.repo_manifest.workspace.as_ref().map_or_else(
                    || "unavailable".into(),
                    |value| value.path.display().to_string()
                ),
                optional_file_path(snapshot.repo_manifest.manifest.as_ref()),
                snapshot.toaster.state,
                optional_file_path(snapshot.toaster.executable.as_ref()),
                if snapshot.toaster.configurations.is_empty() {
                    "none".into()
                } else {
                    snapshot
                        .toaster
                        .configurations
                        .iter()
                        .map(|value| value.path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                },
                if snapshot.toaster.observed_processes.is_empty() {
                    "none".into()
                } else {
                    snapshot
                        .toaster
                        .observed_processes
                        .iter()
                        .map(|value| format!("{}:{}", value.pid, value.executable))
                        .collect::<Vec<_>>()
                        .join(", ")
                },
            ) + &format!(
                "\nLimitations:\n- {}",
                if limitations.is_empty() {
                    "none".into()
                } else {
                    limitations.join("\n- ")
                }
            )
        }
    }
}

pub(crate) fn optional_file_path(
    identity: Option<&yoctui_model::MaintenanceFileIdentity>,
) -> String {
    identity.map_or_else(
        || "unavailable".into(),
        |value| value.path.display().to_string(),
    )
}

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
