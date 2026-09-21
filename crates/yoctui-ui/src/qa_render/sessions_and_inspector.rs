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
