//! Security render.
use super::*;

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

pub(crate) fn security_cve_inspector(app: &App) -> String {
    let Some((report, finding)) = selected_security_finding(app) else {
        return "Select a typed CVE finding to inspect it.".into();
    };
    let mapping = display_security_metadata(&finding.mapping);
    let metadata = display_security_metadata(&report.metadata);
    let limitations = if report.limitations.is_empty() {
        "none".into()
    } else {
        report.limitations.join("\n")
    };
    format!(
        "Finding: {}\nStatus: {}\nRecipe: {}\nPackage: {}\nProduct: {}\nVersion: {}\nSeverity: {}\nScore: {}\nVector: {}\nAdvisory: {}\nSummary: {}\n\nExact report: {}\nFingerprint: {}\nBytes: {}\nModified: {}\nReport scope: {}\n\nPackage mapping:\n{}\n\nReport metadata:\n{}\n\nLimitations:\n{}",
        finding.identity.cve,
        security_cve_status_label(finding.status),
        finding.identity.recipe,
        finding.identity.package.as_deref().unwrap_or("unavailable"),
        finding.product.as_deref().unwrap_or("unavailable"),
        finding.version.as_deref().unwrap_or("unavailable"),
        finding.severity.as_deref().unwrap_or("unavailable"),
        finding.score.as_deref().unwrap_or("unavailable"),
        finding.vector.as_deref().unwrap_or("unavailable"),
        finding.advisory_url.as_deref().unwrap_or("unavailable"),
        finding.summary.as_deref().unwrap_or("unavailable"),
        report.identity.path.display(),
        report.identity.fingerprint,
        report.identity.byte_size,
        timestamp_text(report.identity.modified_at),
        security_scope_text(report.scope.as_ref()),
        mapping,
        metadata,
        limitations,
    )
}

pub(crate) fn security_sbom_inspector(app: &App) -> String {
    let Some(report) = app.security.selected_report() else {
        return "Select an exact SPDX, CycloneDX, or package-manifest document to inspect it."
            .into();
    };
    if let SecurityReport::CycloneDx(document) = report {
        return format!(
            "Exact artifact: {}\nFingerprint: {}\nBytes: {}\nModified: {}\nFormat: CycloneDX\nSpec version: {}\nSerial number: {}\nDocument version: {}\nComponents: {}\nDependencies: {}\n\nLimitations:\n{}",
            document.identity.path.display(),
            document.identity.fingerprint,
            document.identity.byte_size,
            timestamp_text(document.identity.modified_at),
            document.spec_version.as_deref().unwrap_or("unavailable"),
            document.serial_number.as_deref().unwrap_or("unavailable"),
            document
                .version
                .map_or_else(|| "unavailable".into(), |value| value.to_string()),
            document.components.len(),
            document
                .dependency_count
                .map_or_else(|| "unavailable".into(), |value| value.to_string()),
            if document.limitations.is_empty() {
                "none".into()
            } else {
                document.limitations.join("\n")
            },
        );
    }
    if let SecurityReport::PackageManifest(document) = report {
        return format!(
            "Exact artifact: {}\nFingerprint: {}\nBytes: {}\nModified: {}\nFormat: Yocto package manifest fallback\nComponents: {}\n\nThis fallback supplies package names and versions only. License, supplier, file, and relationship claims are unavailable.\n\nLimitations:\n{}",
            document.identity.path.display(),
            document.identity.fingerprint,
            document.identity.byte_size,
            timestamp_text(document.identity.modified_at),
            document.components.len(),
            if document.limitations.is_empty() {
                "none".into()
            } else {
                document.limitations.join("\n")
            },
        );
    }
    let SecurityReport::Spdx(document) = report else {
        return "Select an SBOM document.".into();
    };
    let creators = if document.creators.is_empty() {
        "unavailable".into()
    } else {
        document.creators.join("\n")
    };
    let checksums = display_security_metadata(&document.checksums);
    let limitations = if document.limitations.is_empty() {
        "none".into()
    } else {
        document.limitations.join("\n")
    };
    let component = app
        .security
        .visible_components()
        .into_iter()
        .find(|component| {
            app.security.component_selection.as_deref() == Some(component.identity.as_str())
        })
        .map_or_else(
            || "No component selected.".into(),
            |component| {
                format!(
                    "Component: {}\nName: {}\nVersion: {}\nSupplier: {}\nLicense: {}",
                    component.identity,
                    component.name,
                    component.version.as_deref().unwrap_or("unavailable"),
                    component.supplier.as_deref().unwrap_or("unavailable"),
                    component.license.as_deref().unwrap_or("unavailable"),
                )
            },
        );
    format!(
        "Exact artifact: {}\nFingerprint: {}\nBytes: {}\nModified: {}\nKind: {}\nScope: {}\nSPDX version: {}\nDocument: {}\nNamespace: {}\nData license: {}\nCreators:\n{}\nComponents: {}\nFiles: {}\nRelationships: {}\nChecksums:\n{}\n\n{}\n\nLimitations:\n{}",
        document.identity.path.display(),
        document.identity.fingerprint,
        document.identity.byte_size,
        timestamp_text(document.identity.modified_at),
        security_spdx_kind_label(document.kind),
        security_scope_text(document.scope.as_ref()),
        document.spdx_version.as_deref().unwrap_or("unavailable"),
        document.name.as_deref().unwrap_or("unavailable"),
        document.namespace.as_deref().unwrap_or("unavailable"),
        document.data_license.as_deref().unwrap_or("unavailable"),
        creators,
        document.components.len(),
        document
            .file_count
            .map_or_else(|| "unavailable".into(), |value| value.to_string()),
        document
            .relationship_count
            .map_or_else(|| "unavailable".into(), |value| value.to_string()),
        checksums,
        component,
        limitations,
    )
}

pub(crate) fn selected_security_finding(
    app: &App,
) -> Option<(&yoctui_model::CveReport, &yoctui_model::CveFinding)> {
    let identity = app.security.finding_selection.as_ref()?;
    app.security
        .inventory
        .reports()?
        .iter()
        .find_map(|report| match report {
            SecurityReport::Cve(report) => report
                .findings
                .iter()
                .find(|finding| &finding.identity == identity)
                .map(|finding| (report, finding)),
            SecurityReport::Spdx(_)
            | SecurityReport::CycloneDx(_)
            | SecurityReport::PackageManifest(_) => None,
        })
}

pub(crate) fn cve_source_for_finding<'a>(
    app: &'a App,
    identity: &yoctui_model::CveFindingIdentity,
) -> Option<&'a yoctui_model::CveReport> {
    app.security
        .inventory
        .reports()?
        .iter()
        .find_map(|report| match report {
            SecurityReport::Cve(report)
                if report
                    .findings
                    .iter()
                    .any(|finding| &finding.identity == identity) =>
            {
                Some(report)
            }
            _ => None,
        })
}

pub(crate) fn display_security_metadata(values: &[yoctui_model::SecurityMetadata]) -> String {
    if values.is_empty() {
        "unavailable".into()
    } else {
        values
            .iter()
            .map(|value| format!("{}={}", value.key, value.value))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub(crate) fn display_security_paths(paths: &[std::path::PathBuf]) -> String {
    if paths.is_empty() {
        "unavailable".into()
    } else {
        paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub(crate) fn security_dialog(frame: &mut Frame, app: &App, dialog: &SecurityDialog, area: Rect) {
    let (title, text, height) = match dialog {
        SecurityDialog::Operation(preview) => {
            let roots = display_security_paths(&preview.report_roots);
            (
                format!("Confirm {}", security_operation_label(&preview.operation)),
                format!(
                    "Session: {}\nScope: {}\n\nExact indexed shell-free operation:\n{}\n\nAuthoritative report roots:\n{}\n\nEnter starts; Esc cancels.",
                    preview.id.0,
                    security_scope_text(Some(&preview.scope)),
                    preview.indexed_arguments.join("\n"),
                    roots,
                ),
                18,
            )
        }
        SecurityDialog::Import { .. } => unreachable!("Security import uses the shared editor"),
        SecurityDialog::Cancellation(id) => (
            "Confirm Security cancellation".into(),
            format!(
                "Cancel Security session {} only?\n\nEnter requests cancellation; Esc keeps it running.",
                id.0
            ),
            7,
        ),
    };
    let popup = dialog_popup_rect(area, 94, height);
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(text)
            .block(dialog_block(app, title, DialogTone::Confirmation))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn security_operation_label(operation: &SecurityOperation) -> &'static str {
    match operation {
        SecurityOperation::CveCheck(_) => "CVE check",
        SecurityOperation::SbomBuild(_) => "SBOM generation",
        SecurityOperation::PackageMap { .. } => "CVE package mapping",
    }
}

pub(crate) fn security_filter_label(filter: yoctui_model::CveStatusFilter) -> &'static str {
    match filter {
        yoctui_model::CveStatusFilter::All => "all",
        yoctui_model::CveStatusFilter::Vulnerable => "vulnerable",
        yoctui_model::CveStatusFilter::Patched => "patched",
        yoctui_model::CveStatusFilter::Ignored => "ignored",
        yoctui_model::CveStatusFilter::NotAffected => "not affected",
        yoctui_model::CveStatusFilter::Unknown => "unknown",
    }
}

pub(crate) fn security_cve_status_label(status: yoctui_model::CveStatus) -> &'static str {
    match status {
        yoctui_model::CveStatus::Vulnerable => "vulnerable",
        yoctui_model::CveStatus::Patched => "patched",
        yoctui_model::CveStatus::Ignored => "ignored",
        yoctui_model::CveStatus::NotAffected => "not affected",
        yoctui_model::CveStatus::Unknown => "unknown",
    }
}

pub(crate) fn security_spdx_kind_label(kind: SpdxArtifactKind) -> &'static str {
    match kind {
        SpdxArtifactKind::Json => "JSON",
        SpdxArtifactKind::Archive => "archive",
    }
}

pub(crate) fn security_session_status_label(status: SecuritySessionStatus) -> &'static str {
    match status {
        SecuritySessionStatus::Starting => "starting",
        SecuritySessionStatus::Running => "running",
        SecuritySessionStatus::Cancelling => "cancelling",
        SecuritySessionStatus::Succeeded => "succeeded",
        SecuritySessionStatus::Failed => "failed",
        SecuritySessionStatus::Cancelled => "cancelled",
        SecuritySessionStatus::TimedOut => "timed out",
        SecuritySessionStatus::Lost => "lost",
    }
}

pub(crate) fn security_info_style(palette: &ThemePalette) -> Style {
    palette.role(palette.informational, Modifier::ITALIC)
}

pub(crate) fn security_warning_style(palette: &ThemePalette) -> Style {
    palette.role(palette.warning, Modifier::BOLD)
}

pub(crate) fn security_error_style(palette: &ThemePalette) -> Style {
    palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED)
}

pub(crate) fn security_cve_status_style(
    palette: &ThemePalette,
    status: yoctui_model::CveStatus,
) -> Style {
    match status {
        yoctui_model::CveStatus::Vulnerable => security_error_style(palette),
        yoctui_model::CveStatus::Patched | yoctui_model::CveStatus::NotAffected => {
            palette.role(palette.success, Modifier::BOLD)
        }
        yoctui_model::CveStatus::Ignored | yoctui_model::CveStatus::Unknown => {
            security_warning_style(palette)
        }
    }
}

pub(crate) fn security_session_style(
    palette: &ThemePalette,
    status: SecuritySessionStatus,
) -> Style {
    match status {
        SecuritySessionStatus::Succeeded => palette.role(palette.success, Modifier::BOLD),
        SecuritySessionStatus::Failed | SecuritySessionStatus::Lost => {
            security_error_style(palette)
        }
        SecuritySessionStatus::Cancelled | SecuritySessionStatus::TimedOut => {
            security_warning_style(palette)
        }
        SecuritySessionStatus::Starting
        | SecuritySessionStatus::Running
        | SecuritySessionStatus::Cancelling => palette.role(palette.progress, Modifier::BOLD),
    }
}
