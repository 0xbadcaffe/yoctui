pub(crate) fn security_scope_text(scope: Option<&SecurityScope>) -> String {
    match scope {
        Some(SecurityScope::Recipe(identity)) => {
            format!("recipe {} ({})", identity.name, identity.file.display())
        }
        Some(SecurityScope::Image {
            target,
            machine,
            distro,
        }) => format!("image {target} | MACHINE={machine} | DISTRO={distro}"),
        None => "unavailable".into(),
    }
}

pub(crate) fn security_capability_summary(capability: &SecurityCapability) -> String {
    match capability {
        SecurityCapability::NotInspected => {
            "not inspected; entering Security requests inspection".into()
        }
        SecurityCapability::Inspecting => "inspection in progress".into(),
        SecurityCapability::Failed(message) => format!("inspection failed: {message}"),
        SecurityCapability::Available(capability) => format!(
            "{} | build={} | CVE={} | recipe SBOM={} | image SBOM={} | mapper={}",
            capability
                .release
                .as_deref()
                .unwrap_or("release unavailable"),
            capability.build_directory.display(),
            capability.cve_task.as_deref().unwrap_or("unavailable"),
            capability
                .recipe_sbom_task
                .as_deref()
                .unwrap_or("unavailable"),
            capability
                .image_sbom_task
                .as_deref()
                .unwrap_or(if capability.image_build_emits_sbom {
                    "ordinary image build"
                } else {
                    "unavailable"
                }),
            capability.mapper.as_ref().map_or_else(
                || "unavailable".into(),
                |mapper| mapper.executable.display().to_string()
            ),
        ),
    }
}

pub(crate) fn security_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let active = |view| {
        if app.security.view == view {
            palette.focus()
        } else {
            Style::default()
        }
    };
    let (search_selection, search_total) = if app.security.view == SecurityView::Cves {
        let visible = app.security.visible_findings();
        (
            visible.iter().position(|finding| {
                app.security.finding_selection.as_ref() == Some(&finding.identity)
            }),
            visible.len(),
        )
    } else if app.security.drilled {
        let visible = app.security.visible_components();
        (
            visible.iter().position(|component| {
                app.security.component_selection.as_deref() == Some(component.identity.as_str())
            }),
            visible.len(),
        )
    } else {
        let visible = app.security.visible_reports();
        (
            visible.iter().position(|report| {
                app.security.report_selection.as_ref() == Some(report.identity())
            }),
            visible.len(),
        )
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(" CVEs ", active(SecurityView::Cves)),
            Span::raw(" | "),
            Span::styled(" SBOM ", active(SecurityView::Sbom)),
        ]),
        Line::from(format!(
            "Scope: {}",
            security_scope_text(app.security.scope.as_ref())
        )),
        Line::from(format!(
            "Capability: {}",
            security_capability_summary(&app.security.capability)
        )),
    ];
    lines.push(search_line(
        app,
        &app.security.query,
        app.security.searching,
        search_selection,
        search_total,
        SearchNavigation::Results,
        SearchExit::Done,
        area.width.saturating_sub(2),
    ));
    lines.push(Line::from(""));
    let collection_capacity = usize::from(area.height.saturating_sub(14)).max(1);
    security_inventory_lines(app, &palette, &mut lines, collection_capacity);
    security_session_lines(app, &palette, &mut lines);
    yocto_logs::render(
        frame,
        area,
        Text::from(lines),
        pane_block(app, "Security", app.focus == FocusTarget::Workspace),
        true,
    );
}

pub(crate) fn security_inventory_lines(
    app: &App,
    palette: &ThemePalette,
    lines: &mut Vec<Line<'static>>,
    capacity: usize,
) {
    match &app.security.inventory {
        SecurityInventoryState::NotLoaded => lines.push(Line::from(
            "Reports are not loaded. Press I to import or R after capability discovery.",
        )),
        SecurityInventoryState::Loading { request } => lines.push(Line::styled(
            format!(
                "Loading report generation {} from {} exact path(s)…",
                request.generation,
                request.paths.len()
            ),
            security_info_style(palette),
        )),
        SecurityInventoryState::AvailableEmpty { request } => lines.push(Line::from(format!(
            "Report generation {} is available-empty: no reports or findings.",
            request.generation
        ))),
        SecurityInventoryState::Available { .. } | SecurityInventoryState::Partial { .. } => {
            match app.security.view {
                SecurityView::Cves => security_cve_lines(app, palette, lines, capacity),
                SecurityView::Sbom => security_sbom_lines(app, palette, lines, capacity),
            }
            if let SecurityInventoryState::Partial { limitations, .. } = &app.security.inventory {
                lines.push(Line::styled(
                    format!("Partial: {}", limitations.join(" | ")),
                    security_warning_style(palette),
                ));
            }
        }
        SecurityInventoryState::Failed { message, .. } => lines.push(Line::styled(
            format!("Security report acquisition failed: {message}"),
            security_error_style(palette),
        )),
        SecurityInventoryState::Cancelled { .. } => lines.push(Line::styled(
            "Security report acquisition cancelled.",
            security_warning_style(palette),
        )),
        SecurityInventoryState::TimedOut { .. } => lines.push(Line::styled(
            "Security report acquisition timed out.",
            security_error_style(palette),
        )),
        SecurityInventoryState::Lost { message, .. } => lines.push(Line::styled(
            format!("Security report worker lost: {message}"),
            security_error_style(palette),
        )),
    }
}

pub(crate) fn security_cve_lines(
    app: &App,
    palette: &ThemePalette,
    lines: &mut Vec<Line<'static>>,
    capacity: usize,
) {
    lines.push(Line::from(format!(
        "Filter: {} | {} visible finding(s)",
        security_filter_label(app.security.cve_filter),
        app.security.visible_findings().len()
    )));
    lines.push(Line::from(
        "  CVE             Status        Recipe / package          Severity/score  Exact source",
    ));
    let findings = app.security.visible_findings();
    let selection = findings
        .iter()
        .position(|finding| app.security.finding_selection.as_ref() == Some(&finding.identity));
    let viewport = yoctui_model::centered_viewport_range(selection, findings.len(), capacity);
    for finding in &findings[viewport] {
        let selected = app.security.finding_selection.as_ref() == Some(&finding.identity);
        let source = cve_source_for_finding(app, &finding.identity).map_or_else(
            || "unavailable".into(),
            |report| report.identity.path.display().to_string(),
        );
        lines.push(
            Line::from(format!(
                "{} {:<15} {:<13} {:<25} {:<15} {}",
                if selected { "▶" } else { " " },
                finding.identity.cve,
                security_cve_status_label(finding.status),
                format!(
                    "{} / {}",
                    finding.identity.recipe,
                    finding.identity.package.as_deref().unwrap_or("—")
                ),
                format!(
                    "{}/{}",
                    finding.severity.as_deref().unwrap_or("—"),
                    finding.score.as_deref().unwrap_or("—")
                ),
                source,
            ))
            .style(if selected {
                palette.selected()
            } else {
                security_cve_status_style(palette, finding.status)
            }),
        );
    }
    if findings.is_empty() {
        lines.push(Line::from(
            "No findings match the active view, status filter, and search.",
        ));
    }
}
