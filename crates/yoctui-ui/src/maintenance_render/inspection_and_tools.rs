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
