pub(crate) fn qa_layer_lines(
    app: &App,
    palette: &ThemePalette,
    lines: &mut Vec<Line<'static>>,
    capacity: usize,
) {
    match &app.qa.layer_capability {
        QaLayerCapability::NotInspected => {
            lines.push(Line::from("Layer-QA capability is not inspected."));
            return;
        }
        QaLayerCapability::Inspecting => {
            lines.push(Line::styled(
                "Inspecting configured layer-QA capability…",
                palette.informational,
            ));
            return;
        }
        QaLayerCapability::Failed(message) => {
            lines.push(Line::styled(
                format!("Layer-QA capability failed: {message}"),
                palette.error,
            ));
            return;
        }
        QaLayerCapability::Partial { limitations, .. } => lines.push(Line::styled(
            format!("Partial capability: {}", limitations.join(" | ")),
            palette.warning,
        )),
        QaLayerCapability::Available(_) => {}
    }
    lines.push(Line::from(
        "  Layer                 Capability       Pass Warn Fail Skip Unknown  Exact root",
    ));
    let layers = app.qa.visible_layers();
    let selection = layers
        .iter()
        .position(|layer| app.qa.layer_selection.as_ref() == Some(&layer.identity));
    let viewport = yoctui_model::centered_viewport_range(selection, layers.len(), capacity);
    for layer in &layers[viewport] {
        let selected = app.qa.layer_selection.as_ref() == Some(&layer.identity);
        let counts = app.qa.layer_finding_counts(&layer.identity);
        let capability = match layer.run {
            QaLayerRunCapability::Available { .. } => "available",
            QaLayerRunCapability::Disabled(_) => "disabled",
        };
        let status = qa_worst_status(
            app.qa
                .findings_for_layer(&layer.identity)
                .iter()
                .map(|finding| finding.status),
        );
        lines.push(
            Line::from(format!(
                "{} {:<21} {:<16} {:>4} {:>4} {:>4} {:>4} {:>7}  {}",
                if selected { "▶" } else { " " },
                layer.identity.name,
                capability,
                counts.passed,
                counts.warnings,
                counts.failed,
                counts.skipped,
                counts.unknown,
                layer.identity.root.display(),
            ))
            .style(if selected {
                palette.selected()
            } else {
                qa_status_style(palette, status)
            }),
        );
    }
    if layers.is_empty() {
        lines.push(Line::from(
            "No configured layers match the status filter and search.",
        ));
    }
}

pub(crate) fn qa_finding_lines(
    app: &App,
    palette: &ThemePalette,
    lines: &mut Vec<Line<'static>>,
    capacity: usize,
) {
    lines.push(Line::from(
        "Findings (Esc returns) — Status     Severity      Rule / test                 Message",
    ));
    let findings = app.qa.visible_findings();
    let selection = findings
        .iter()
        .position(|finding| app.qa.finding_selection.as_ref() == Some(&finding.identity));
    let viewport = yoctui_model::centered_viewport_range(selection, findings.len(), capacity);
    for finding in &findings[viewport] {
        let selected = app.qa.finding_selection.as_ref() == Some(&finding.identity);
        lines.push(
            Line::from(format!(
                "{} {:<10} {:<13} {:<27} {}",
                if selected { "▶" } else { " " },
                qa_status_label(Some(finding.status)),
                finding.severity.as_deref().unwrap_or("unavailable"),
                finding
                    .rule
                    .as_deref()
                    .or(finding.test_name.as_deref())
                    .unwrap_or("unavailable"),
                finding.message,
            ))
            .style(if selected {
                palette.selected()
            } else {
                qa_status_style(palette, Some(finding.status))
            }),
        );
    }
    if findings.is_empty() {
        lines.push(Line::from(
            "No findings match the active filter and search.",
        ));
    }
}

pub(crate) fn qa_inventory_lines(
    app: &App,
    palette: &ThemePalette,
    lines: &mut Vec<Line<'static>>,
) {
    match &app.qa.inventory {
        QaReportInventoryState::NotLoaded => lines.push(Line::from(
            "Reports not loaded. I imports; R refreshes exact paths.",
        )),
        QaReportInventoryState::Loading { request } => lines.push(Line::styled(
            format!(
                "Loading report generation {} from {} exact path(s)…",
                request.generation,
                request.paths.len()
            ),
            palette.informational,
        )),
        QaReportInventoryState::AvailableEmpty { request } => lines.push(Line::from(format!(
            "Report generation {} available-empty: no reports or findings.",
            request.generation
        ))),
        QaReportInventoryState::Available { reports, .. } => lines.push(Line::from(format!(
            "{} exact report(s) available.",
            reports.len()
        ))),
        QaReportInventoryState::Partial {
            reports,
            limitations,
            ..
        } => lines.push(Line::styled(
            format!(
                "Partial: {} report(s) | {}",
                reports.len(),
                limitations.join(" | ")
            ),
            palette.warning,
        )),
        QaReportInventoryState::Failed { kind, message, .. } => lines.push(Line::styled(
            format!("Report acquisition {}: {message}", qa_failure_label(*kind)),
            palette.error,
        )),
        QaReportInventoryState::Cancelled { .. } => lines.push(Line::styled(
            "Report acquisition cancelled.",
            palette.warning,
        )),
        QaReportInventoryState::TimedOut { .. } => {
            lines.push(Line::styled("Report acquisition timed out.", palette.error))
        }
        QaReportInventoryState::Lost { message, .. } => lines.push(Line::styled(
            format!("Report worker lost: {message}"),
            palette.error,
        )),
    }
}
