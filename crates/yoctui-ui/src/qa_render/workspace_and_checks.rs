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
