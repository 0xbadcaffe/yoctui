pub(crate) fn security_sbom_lines(
    app: &App,
    palette: &ThemePalette,
    lines: &mut Vec<Line<'static>>,
    capacity: usize,
) {
    if app.security.drilled {
        let Some(report) = app.security.selected_report() else {
            lines.push(Line::styled(
                "The selected SBOM or package manifest is no longer available.",
                security_warning_style(palette),
            ));
            return;
        };
        let (identity, kind) = match report {
            SecurityReport::Spdx(document) => (&document.identity, "SPDX"),
            SecurityReport::CycloneDx(document) => (&document.identity, "CycloneDX"),
            SecurityReport::PackageManifest(document) => (&document.identity, "package manifest"),
            SecurityReport::Cve(_) => return,
        };
        lines.push(Line::from(format!(
            "{kind}: {} | fingerprint {}",
            identity.path.display(),
            identity.fingerprint
        )));
        lines.push(Line::from(
            "  Component identity        Name                    Version       Supplier / license",
        ));
        let components = app.security.visible_components();
        let selection = components.iter().position(|component| {
            app.security.component_selection.as_deref() == Some(component.identity.as_str())
        });
        let viewport = yoctui_model::centered_viewport_range(selection, components.len(), capacity);
        for component in &components[viewport] {
            let selected =
                app.security.component_selection.as_deref() == Some(component.identity.as_str());
            lines.push(
                Line::from(format!(
                    "{} {:<25} {:<23} {:<13} {} / {}",
                    if selected { "▶" } else { " " },
                    component.identity,
                    component.name,
                    component.version.as_deref().unwrap_or("—"),
                    component.supplier.as_deref().unwrap_or("—"),
                    component.license.as_deref().unwrap_or("—"),
                ))
                .style(if selected {
                    palette.selected()
                } else {
                    Style::default()
                }),
            );
        }
        if components.is_empty() {
            lines.push(Line::from(
                "No components match the active document and search.",
            ));
        }
        return;
    }
    lines.push(Line::from(
        "  Format       Version        Document                 Components  Exact artifact",
    ));
    let reports = app.security.visible_reports();
    let selection = reports
        .iter()
        .position(|report| app.security.report_selection.as_ref() == Some(report.identity()));
    let viewport = yoctui_model::centered_viewport_range(selection, reports.len(), capacity);
    for report in &reports[viewport] {
        let (identity, format, version, name, components, limited) = match report {
            SecurityReport::Spdx(document) => (
                &document.identity,
                "SPDX",
                document.spdx_version.as_deref().unwrap_or("unavailable"),
                document.name.as_deref().unwrap_or("unavailable"),
                document.components.len(),
                !document.limitations.is_empty(),
            ),
            SecurityReport::CycloneDx(document) => (
                &document.identity,
                "CycloneDX",
                document.spec_version.as_deref().unwrap_or("unavailable"),
                "software BOM",
                document.components.len(),
                !document.limitations.is_empty(),
            ),
            SecurityReport::PackageManifest(document) => (
                &document.identity,
                "Manifest",
                "fallback",
                "package inventory",
                document.components.len(),
                !document.limitations.is_empty(),
            ),
            SecurityReport::Cve(_) => continue,
        };
        let selected = app.security.report_selection.as_ref() == Some(identity);
        lines.push(
            Line::from(format!(
                "{} {:<12} {:<14} {:<24} {:<11} {}",
                if selected { "▶" } else { " " },
                format,
                version,
                name,
                components,
                identity.path.display(),
            ))
            .style(if selected {
                palette.selected()
            } else if !limited {
                Style::default()
            } else {
                security_warning_style(palette)
            }),
        );
    }
    if reports.is_empty() {
        lines.push(Line::from(
            "No SPDX, CycloneDX, or package-manifest documents match the active view and search.",
        ));
    }
}

pub(crate) fn security_session_lines(
    app: &App,
    palette: &ThemePalette,
    lines: &mut Vec<Line<'static>>,
) {
    let Some(session) = app.security.sessions.last() else {
        return;
    };
    let operation = security_operation_label(&session.preview.operation);
    lines.push(Line::from(""));
    lines.push(Line::styled(
        format!(
            "Latest session {} | {operation} | {} | scope {}",
            session.preview.id.0,
            security_session_status_label(session.status),
            security_scope_text(Some(&session.preview.scope))
        ),
        security_session_style(palette, session.status),
    ));
    if let Some(background_job_id) = session.background_job_id
        && let Some(job) = app.background_jobs.get(background_job_id)
    {
        lines.push(Line::from(format!(
            "Managed build {:?} | warnings={} | errors={} | output dropped={}",
            job.status, job.warnings, job.errors, job.dropped_output_entries
        )));
    }
    if let Some(message) = &session.message {
        lines.push(Line::styled(
            format!("Session detail: {message}"),
            security_warning_style(palette),
        ));
    }
    for output in session.output.iter().rev().take(4).rev() {
        let stream = match output.stream {
            SecurityOutputStream::Stdout => "stdout",
            SecurityOutputStream::Stderr => "stderr",
        };
        let style = if output.stream == SecurityOutputStream::Stderr {
            security_warning_style(palette)
        } else {
            Style::default()
        };
        lines.push(Line::styled(
            format!(
                "[{stream}] {}{}",
                output.line,
                if output.truncated { " [truncated]" } else { "" }
            ),
            style,
        ));
    }
}

pub(crate) fn security_inspector_text(app: &App) -> String {
    let capability = security_capability_detail(&app.security.capability);
    let selected = match app.security.view {
        SecurityView::Cves => security_cve_inspector(app),
        SecurityView::Sbom => security_sbom_inspector(app),
    };
    let session = app.security.sessions.last().map_or_else(
        || "No Security operation has run.".into(),
        |session| {
            format!(
                "Session {}: {} ({})\nStarted: {}\nFinished: {}\nResult paths: {}\nRetained mapper output: {}",
                session.preview.id.0,
                security_operation_label(&session.preview.operation),
                security_session_status_label(session.status),
                timestamp_text(session.started_at),
                session
                    .finished_at
                    .map(timestamp_text)
                    .unwrap_or_else(|| "not finished".into()),
                session.result_paths.len(),
                session.output.len(),
            )
        },
    );
    format!("{selected}\n\n{capability}\n\n{session}")
}

pub(crate) fn security_capability_detail(capability: &SecurityCapability) -> String {
    match capability {
        SecurityCapability::Available(capability) => {
            let limitations = if capability.limitations.is_empty() {
                "none".into()
            } else {
                capability.limitations.join("\n")
            };
            format!(
                "Capability: available\nRelease: {}\nBuild: {}\nScope: {}\nCVE task: {}\nRecipe SBOM task: {}\nImage SBOM task: {}\nImage build emits SBOM: {}\nMapper: {}\nCVE roots: {}\nSBOM roots: {}\nLimitations:\n{}",
                capability.release.as_deref().unwrap_or("unavailable"),
                capability.build_directory.display(),
                security_scope_text(Some(&capability.scope)),
                capability.cve_task.as_deref().unwrap_or("unavailable"),
                capability
                    .recipe_sbom_task
                    .as_deref()
                    .unwrap_or("unavailable"),
                capability
                    .image_sbom_task
                    .as_deref()
                    .unwrap_or("unavailable"),
                capability.image_build_emits_sbom,
                capability.mapper.as_ref().map_or_else(
                    || "unavailable".into(),
                    |mapper| mapper.executable.display().to_string()
                ),
                display_security_paths(&capability.cve_roots),
                display_security_paths(&capability.sbom_roots),
                limitations,
            )
        }
        _ => format!("Capability: {}", security_capability_summary(capability)),
    }
}
