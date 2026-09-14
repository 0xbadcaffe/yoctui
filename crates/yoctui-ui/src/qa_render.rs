//! Qa render.
use super::*;

pub(crate) fn sdk_kind_label(kind: SdkArtifactKind) -> &'static str {
    match kind {
        SdkArtifactKind::Installer => "installer",
        SdkArtifactKind::Checksum => "checksum",
        SdkArtifactKind::Manifest => "manifest",
        SdkArtifactKind::Other => "other",
    }
}

pub(crate) fn sdk_type_label(kind: Option<SdkKind>) -> &'static str {
    match kind {
        Some(SdkKind::Standard) => "standard",
        Some(SdkKind::Extensible) => "extensible",
        None => "unavailable",
    }
}

pub(crate) fn sdk_inventory_root(app: &App) -> String {
    let request_root = match &app.sdk_artifacts {
        SdkArtifactInventoryState::Loading { request }
        | SdkArtifactInventoryState::AvailableEmpty { request }
        | SdkArtifactInventoryState::Available { request, .. }
        | SdkArtifactInventoryState::Partial { request, .. }
        | SdkArtifactInventoryState::Failed { request, .. } => Some(&request.root),
        SdkArtifactInventoryState::NotLoaded => None,
    };
    request_root
        .map(|path| path.display().to_string())
        .or_else(|| app.workspace.variables.get("SDK_DEPLOY").cloned())
        .unwrap_or_else(|| "unavailable".into())
}

pub(crate) fn qa_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let active = |view| {
        if app.qa.view == view {
            palette.focus()
        } else {
            Style::default()
        }
    };
    let (search_selection, search_total) = if app.qa.drilled {
        let visible = app.qa.visible_findings();
        (
            visible
                .iter()
                .position(|finding| app.qa.finding_selection.as_ref() == Some(&finding.identity)),
            visible.len(),
        )
    } else if app.qa.view == QaView::LayerQa {
        let visible = app.qa.visible_layers();
        (
            visible
                .iter()
                .position(|layer| app.qa.layer_selection.as_ref() == Some(&layer.identity)),
            visible.len(),
        )
    } else {
        let visible = app.qa.visible_checks();
        (
            visible
                .iter()
                .position(|check| app.qa.check_selection.as_ref() == Some(&check.id)),
            visible.len(),
        )
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(" Recipe & Kernel ", active(QaView::RecipeKernel)),
            Span::raw(" | "),
            Span::styled(" Layer QA ", active(QaView::LayerQa)),
        ]),
        Line::from(format!(
            "Scope: {} | Filter: {}",
            qa_scope_label(app),
            qa_filter_label(app.qa.status_filter),
        )),
        search_line(
            app,
            &app.qa.query,
            app.qa.searching,
            search_selection,
            search_total,
            SearchNavigation::Results,
            SearchExit::Done,
            area.width.saturating_sub(2),
        ),
        Line::from(""),
    ];
    let collection_capacity = usize::from(area.height.saturating_sub(12)).max(1);
    if app.qa.drilled {
        qa_finding_lines(app, &palette, &mut lines, collection_capacity);
    } else {
        match app.qa.view {
            QaView::RecipeKernel => qa_check_lines(app, &palette, &mut lines, collection_capacity),
            QaView::LayerQa => qa_layer_lines(app, &palette, &mut lines, collection_capacity),
        }
    }
    qa_inventory_lines(app, &palette, &mut lines);
    qa_session_lines(app, &palette, &mut lines);
    yocto_logs::render(
        frame,
        area,
        Text::from(lines),
        pane_block(app, "QA", app.focus == FocusTarget::Workspace),
        true,
    );
}

pub(crate) fn qa_scope_label(app: &App) -> String {
    match app.qa.view {
        QaView::RecipeKernel => app.qa.scope.as_ref().map_or_else(
            || "unavailable".into(),
            |scope| format!("{} ({})", scope.recipe.name, scope.recipe.file.display()),
        ),
        QaView::LayerQa => app.qa.layer_selection.as_ref().map_or_else(
            || "unavailable".into(),
            |layer| format!("{} ({})", layer.name, layer.root.display()),
        ),
    }
}

pub(crate) fn qa_check_lines(
    app: &App,
    palette: &ThemePalette,
    lines: &mut Vec<Line<'static>>,
    capacity: usize,
) {
    match &app.qa.capability {
        QaCapability::NotInspected => {
            lines.push(Line::from("QA capability is not inspected."));
            return;
        }
        QaCapability::Inspecting => {
            lines.push(Line::styled(
                "Inspecting recipe and kernel QA capability…",
                palette.informational,
            ));
            return;
        }
        QaCapability::Failed(message) => {
            lines.push(Line::styled(
                format!("QA capability failed: {message}"),
                palette.error,
            ));
            return;
        }
        QaCapability::Partial { limitations, .. } => lines.push(Line::styled(
            format!("Partial capability: {}", limitations.join(" | ")),
            palette.warning,
        )),
        QaCapability::Available(_) => {}
    }
    lines.push(Line::from(
        "  Family                Exact task             Availability       Findings  Label",
    ));
    let checks = app.qa.visible_checks();
    let selection = checks
        .iter()
        .position(|check| app.qa.check_selection.as_ref() == Some(&check.id));
    let viewport = yoctui_model::centered_viewport_range(selection, checks.len(), capacity);
    for check in &checks[viewport] {
        let selected = app.qa.check_selection.as_ref() == Some(&check.id);
        let findings = app.qa.findings_for_check(&check.id);
        let status = qa_worst_status(findings.iter().map(|finding| finding.status));
        let availability = match &check.availability {
            QaCheckAvailability::Available => "available",
            QaCheckAvailability::Disabled(_) => "disabled",
        };
        lines.push(
            Line::from(format!(
                "{} {:<21} {:<22} {:<18} {:<9} {}",
                if selected { "▶" } else { " " },
                qa_family_label(check.family),
                check.task.as_deref().unwrap_or("unavailable"),
                availability,
                qa_status_label(status),
                check.label,
            ))
            .style(if selected {
                palette.selected()
            } else {
                qa_status_style(palette, status)
            }),
        );
    }
    if checks.is_empty() {
        lines.push(Line::from(
            "No checks match the exact scope, status filter, and search.",
        ));
    }
}

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

pub(crate) fn qa_session_lines(app: &App, palette: &ThemePalette, lines: &mut Vec<Line<'static>>) {
    let session = match app.qa.view {
        QaView::RecipeKernel => app.qa.sessions.back().map(|session| {
            (
                session.id.0,
                session.status,
                session.message.as_deref(),
                session
                    .output
                    .iter()
                    .map(|line| (line.stream, line.line.as_str(), line.truncated))
                    .collect::<Vec<_>>(),
            )
        }),
        QaView::LayerQa => app.qa.layer_sessions.back().map(|session| {
            (
                session.id.0,
                session.status,
                session.message.as_deref(),
                session
                    .output
                    .iter()
                    .map(|line| (line.stream, line.line.as_str(), line.truncated))
                    .collect::<Vec<_>>(),
            )
        }),
    };
    let Some((id, status, message, output)) = session else {
        return;
    };
    lines.push(Line::from(""));
    lines.push(Line::styled(
        format!("Latest session {id}: {}", qa_session_status_label(status)),
        qa_session_style(palette, status),
    ));
    if let Some(message) = message {
        lines.push(Line::styled(
            format!("Session detail: {message}"),
            palette.warning,
        ));
    }
    for (stream, line, truncated) in output.iter().rev().take(4).rev() {
        lines.push(Line::styled(
            format!(
                "[{}] {}{}",
                match stream {
                    QaOutputStream::Stdout => "stdout",
                    QaOutputStream::Stderr => "stderr",
                },
                line,
                if *truncated { " [truncated]" } else { "" }
            ),
            if *stream == QaOutputStream::Stderr {
                security_warning_style(palette)
            } else {
                Style::default()
            },
        ));
    }
}

pub(crate) fn qa_inspector_text(app: &App) -> String {
    let mut lines = vec![format!(
        "View: {}\nExact scope: {}",
        match app.qa.view {
            QaView::RecipeKernel => "Recipe & Kernel",
            QaView::LayerQa => "Layer QA",
        },
        qa_scope_label(app)
    )];
    if let Some(finding) = app.qa.selected_finding() {
        lines.push(format!(
            "\nFinding {}\nStatus: {}\nSeverity: {}\nMessage: {}\nTask: {}\nTest: {}\nRule: {}\nSuggestion: {}\nSource: {}\nMetadata: {}",
            finding.identity.fingerprint,
            qa_status_label(Some(finding.status)),
            finding.severity.as_deref().unwrap_or("unavailable"),
            finding.message,
            finding.task.as_deref().unwrap_or("unavailable"),
            finding.test_name.as_deref().unwrap_or("unavailable"),
            finding.rule.as_deref().unwrap_or("unavailable"),
            finding.suggestion.as_deref().unwrap_or("unavailable"),
            finding.source.as_ref().map_or_else(
                || "unavailable".into(),
                |source| format!(
                    "{}:{}:{}",
                    source.path.display(),
                    source.line.map_or_else(|| "unavailable".into(), |line| line.to_string()),
                    source.column.map_or_else(|| "unavailable".into(), |column| column.to_string())
                )
            ),
            if finding.metadata.is_empty() {
                "unavailable".into()
            } else {
                finding.metadata.iter().map(|item| format!("{}={}", item.key, item.value)).collect::<Vec<_>>().join(", ")
            }
        ));
    } else if let Some(report) = app.qa.selected_report() {
        lines.push(format!(
            "\nReport\nPath: {}\nFormat: {:?}\nBytes: {}\nFingerprint: {}\nModified: {}\nFindings: {}\nLimitations: {}",
            report.identity.path.display(),
            report.identity.format,
            report.identity.byte_size,
            report.identity.fingerprint,
            timestamp_text(report.identity.modified_at),
            report.findings.len(),
            if report.limitations.is_empty() { "none".into() } else { report.limitations.join(" | ") },
        ));
    } else {
        match app.qa.view {
            QaView::RecipeKernel => {
                if let Some(check) = app.qa.selected_check() {
                    lines.push(format!(
                        "\nCheck {}\nFamily: {}\nTask: {}\nAvailability: {}\nProvider: {}\nReport roots: {}\nLimitations: {}",
                        check.id.0,
                        qa_family_label(check.family),
                        check.task.as_deref().unwrap_or("unavailable"),
                        match &check.availability {
                            QaCheckAvailability::Available => "available",
                            QaCheckAvailability::Disabled(reason) => reason,
                        },
                        check.scope.recipe.file.display(),
                        if check.report_roots.is_empty() { "unavailable".into() } else { check.report_roots.iter().map(|path| path.display().to_string()).collect::<Vec<_>>().join(", ") },
                        if check.limitations.is_empty() { "none".into() } else { check.limitations.join(" | ") },
                    ));
                } else {
                    lines.push("\nNo QA check selected.".into());
                }
            }
            QaView::LayerQa => {
                if let Some(layer) = app.qa.selected_layer() {
                    lines.push(format!(
                        "\nLayer {}\nRoot: {}\nCompatible series: {}\nCapability: {}\nLimitations: {}",
                        layer.identity.name,
                        layer.identity.root.display(),
                        if layer.compatible_series.is_empty() { "unavailable".into() } else { layer.compatible_series.join(", ") },
                        match &layer.run {
                            QaLayerRunCapability::Available { executable, arguments, report_roots } => format!("{} | argv [{}] | reports {}", executable.path.display(), arguments.join(", "), report_roots.len()),
                            QaLayerRunCapability::Disabled(reason) => format!("disabled: {reason}"),
                        },
                        if layer.limitations.is_empty() { "none".into() } else { layer.limitations.join(" | ") },
                    ));
                } else {
                    lines.push("\nNo configured layer selected.".into());
                }
            }
        }
    }
    lines.join("\n")
}

pub(crate) fn qa_dialog(frame: &mut Frame, app: &App, dialog: &QaDialog, area: Rect) {
    let (title, tone, body) = match dialog {
        QaDialog::Operation(preview) => (
            "Confirm recipe/kernel QA",
            DialogTone::Confirmation,
            format!(
                "Operation {}\nCheck: {}\nRecipe: {}\nProvider: {}\n\nIndexed BitBake request:\n{}\n\nReport roots:\n{}\n\nEnter runs | Esc cancels",
                preview.id.0,
                preview.check.0,
                preview.scope.recipe.name,
                preview.scope.recipe.file.display(),
                preview.indexed_arguments.join("\n"),
                preview
                    .report_roots
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        ),
        QaDialog::LayerOperation(preview) => (
            "Confirm layer QA",
            DialogTone::Confirmation,
            format!(
                "Operation {}\nLayer: {} ({})\nExecutable: {}\n\nIndexed native vector:\n{}\n\nEnter runs | Esc cancels",
                preview.id.0,
                preview.layer.name,
                preview.layer.root.display(),
                preview.executable.path.display(),
                preview.indexed_arguments.join("\n")
            ),
        ),
        QaDialog::Import { .. } => unreachable!("QA import uses the shared editor"),
        QaDialog::Cancellation {
            session,
            background_job,
        } => (
            "Cancel managed QA",
            DialogTone::Confirmation,
            format!(
                "Cancel QA session {} attached to build job {}?\n\nEnter confirms | Esc keeps running",
                session.0, background_job.0
            ),
        ),
        QaDialog::LayerCancellation(session) => (
            "Cancel layer QA",
            DialogTone::Confirmation,
            format!(
                "Cancel exact layer-QA session {}?\n\nEnter confirms | Esc keeps running",
                session.0
            ),
        ),
    };
    let popup = dialog_popup_rect(area, 78, 18);
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(body)
            .block(dialog_block(app, title, tone))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn qa_family_label(family: QaCheckFamily) -> &'static str {
    match family {
        QaCheckFamily::KernelConfiguration => "kernel configuration",
        QaCheckFamily::UriFetch => "URI / fetch",
        QaCheckFamily::Patch => "patch",
        QaCheckFamily::License => "license",
        QaCheckFamily::RecipePackage => "recipe / package",
    }
}

pub(crate) fn qa_filter_label(filter: QaStatusFilter) -> &'static str {
    match filter {
        QaStatusFilter::All => "all",
        QaStatusFilter::Failed => "failed",
        QaStatusFilter::Warning => "warning",
        QaStatusFilter::Passed => "passed",
        QaStatusFilter::Skipped => "skipped",
        QaStatusFilter::Unknown => "unknown",
    }
}

pub(crate) fn qa_status_label(status: Option<QaFindingStatus>) -> &'static str {
    match status {
        Some(QaFindingStatus::Passed) => "passed",
        Some(QaFindingStatus::Warning) => "warning",
        Some(QaFindingStatus::Failed) => "failed",
        Some(QaFindingStatus::Skipped) => "skipped",
        Some(QaFindingStatus::Unknown) => "unknown",
        None => "unavailable",
    }
}

pub(crate) fn qa_worst_status(
    values: impl Iterator<Item = QaFindingStatus>,
) -> Option<QaFindingStatus> {
    values.max_by_key(|status| match status {
        QaFindingStatus::Failed => 5,
        QaFindingStatus::Warning => 4,
        QaFindingStatus::Unknown => 3,
        QaFindingStatus::Skipped => 2,
        QaFindingStatus::Passed => 1,
    })
}

pub(crate) fn qa_failure_label(kind: QaReportFailureKind) -> &'static str {
    match kind {
        QaReportFailureKind::Missing => "missing",
        QaReportFailureKind::PermissionDenied => "permission denied",
        QaReportFailureKind::Stale => "stale",
        QaReportFailureKind::Malformed => "malformed",
        QaReportFailureKind::Failed => "failed",
    }
}

pub(crate) fn qa_session_status_label(status: QaSessionStatus) -> &'static str {
    match status {
        QaSessionStatus::Starting => "starting",
        QaSessionStatus::Running => "running",
        QaSessionStatus::Cancelling => "cancelling",
        QaSessionStatus::Succeeded => "succeeded",
        QaSessionStatus::Failed => "failed",
        QaSessionStatus::Cancelled => "cancelled",
        QaSessionStatus::TimedOut => "timed out",
        QaSessionStatus::Lost => "lost",
    }
}

pub(crate) fn qa_status_style(palette: &ThemePalette, status: Option<QaFindingStatus>) -> Style {
    match status {
        Some(QaFindingStatus::Passed) => palette.role(palette.success, Modifier::BOLD),
        Some(QaFindingStatus::Warning | QaFindingStatus::Skipped) => {
            security_warning_style(palette)
        }
        Some(QaFindingStatus::Failed) => security_error_style(palette),
        Some(QaFindingStatus::Unknown) | None => Style::default(),
    }
}

pub(crate) fn qa_session_style(palette: &ThemePalette, status: QaSessionStatus) -> Style {
    match status {
        QaSessionStatus::Succeeded => palette.role(palette.success, Modifier::BOLD),
        QaSessionStatus::Failed | QaSessionStatus::TimedOut | QaSessionStatus::Lost => {
            security_error_style(palette)
        }
        QaSessionStatus::Cancelled | QaSessionStatus::Cancelling => security_warning_style(palette),
        QaSessionStatus::Starting | QaSessionStatus::Running => security_info_style(palette),
    }
}
