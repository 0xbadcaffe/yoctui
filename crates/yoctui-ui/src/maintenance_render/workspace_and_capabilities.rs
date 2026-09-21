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
