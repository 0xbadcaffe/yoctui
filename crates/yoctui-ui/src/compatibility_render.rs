//! Compatibility render.
use super::*;

pub(crate) fn compatibility_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let projection = app
        .compatibility_ui
        .project(&app.workspace_compatibility, app.daemon.status);
    let sections = Layout::vertical([Constraint::Length(11), Constraint::Min(5)]).split(area);
    let identity_title = match &projection.authority {
        CompatibilityUiAuthorityStatus::Current { generation, mode } => {
            format!("Environment / Compatibility · generation {generation} · {mode:?}")
        }
        CompatibilityUiAuthorityStatus::Unavailable { .. } => {
            "Environment / Compatibility · snapshot unavailable".into()
        }
    };
    let identity = compatibility_identity_text(&projection);
    frame.render_widget(
        Paragraph::new(identity)
            .block(pane_block(
                app,
                &identity_title,
                app.focus == FocusTarget::Workspace,
            ))
            .wrap(Wrap { trim: false }),
        sections[0],
    );

    let palette = ThemePalette::for_app(app);
    let filtered_selection = projection
        .rows
        .iter()
        .position(|row| projection.selected == Some(row.id));
    let capabilities =
        Layout::vertical([Constraint::Length(1), Constraint::Min(4)]).split(sections[1]);
    let capacity = usize::from(capabilities[1].height.saturating_sub(3)).max(1);
    let viewport =
        yoctui_model::centered_viewport_range(filtered_selection, projection.rows.len(), capacity);
    let rows = projection.rows[viewport.clone()].iter().map(|row| {
        let implementation = row
            .implementation
            .as_ref()
            .map_or("--", |implementation| implementation.id.as_str());
        let style = if projection.selected == Some(row.id) {
            selected_style(app, true)
        } else {
            compatibility_state_style(app, row.state)
        };
        Row::new([
            Cell::from(row.id.as_str()),
            Cell::from(compatibility_state_label(row.state)),
            Cell::from(implementation),
        ])
        .style(style)
    });
    frame.render_widget(
        Paragraph::new(search_line(
            app,
            &app.compatibility_ui.query,
            app.compatibility_ui.searching,
            filtered_selection,
            projection.rows.len(),
            SearchNavigation::Results,
            SearchExit::Done,
            capabilities[0].width,
        )),
        capabilities[0],
    );
    let viewport_cue =
        BoundedScrollIndicator::new(viewport.start, viewport.len(), projection.rows.len())
            .title_label(
                filtered_selection,
                true,
                app.preferences.symbols == SymbolPreference::Unicode,
            );
    let table_title = viewport_cue.map_or_else(
        || {
            format!(
                "Capabilities · {} · {}/{} match",
                compatibility_filter_label(app.compatibility_ui.filter),
                projection.rows.len(),
                projection.total_capabilities,
            )
        },
        |cue| {
            format!(
                "Capabilities · {} · {cue} · {}/{} match",
                compatibility_filter_label(app.compatibility_ui.filter),
                projection.rows.len(),
                projection.total_capabilities,
            )
        },
    );
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Percentage(42),
                Constraint::Length(12),
                Constraint::Min(12),
            ],
        )
        .header(
            Row::new(["Capability", "State", "Implementation"])
                .style(palette.role(palette.informational, Modifier::BOLD)),
        )
        .block(pane_block(
            app,
            &table_title,
            app.focus == FocusTarget::Workspace,
        )),
        capabilities[1],
    );
}

pub(crate) fn compatibility_identity_text(
    projection: &yoctui_model::CompatibilityUiProjection,
) -> String {
    let summary = &projection.summary;
    let status = match &projection.authority {
        CompatibilityUiAuthorityStatus::Current { generation, mode } => {
            format!("Snapshot {generation} · {mode:?}")
        }
        CompatibilityUiAuthorityStatus::Unavailable { reason } => {
            format!("Snapshot unavailable · {reason}")
        }
    };
    let Some(environment) = projection.environment.as_ref() else {
        return format!(
            "{status}\nAvailable {} · Limited {} · Unavailable {} · Unknown {} · Unsupported {}\nBuild unknown\nSource roots unknown\nOE-Core unknown · Poky unknown · BitBake unknown\nDISTRO unknown · MACHINE unknown\nLayer series unknown\nBackend unknown · Protocol unknown",
            summary.available,
            summary.limited,
            summary.unavailable,
            summary.unknown,
            summary.unsupported,
        );
    };
    let release = |value: &yoctui_model::AuthoritativeValue<yoctui_model::ReleaseIdentity>| {
        value.value().map_or_else(
            || "unknown".into(),
            |release| match (&release.name, &release.version) {
                (Some(name), Some(version)) => format!("{name} {version}"),
                (Some(name), None) => name.clone(),
                (None, Some(version)) => version.clone(),
                (None, None) => "unknown".into(),
            },
        )
    };
    let source_roots = environment.source_roots.value().map_or_else(
        || "unknown".into(),
        |roots| {
            roots
                .iter()
                .map(|root| format!("{:?}:{}", root.kind, root.path.display()))
                .collect::<Vec<_>>()
                .join(", ")
        },
    );
    let layer_series = environment.layer_series.value().map_or_else(
        || "unknown".into(),
        |layers| {
            layers
                .iter()
                .map(|layer| format!("{}:[{}]", layer.layer, layer.compatible_series.join(",")))
                .collect::<Vec<_>>()
                .join(", ")
        },
    );
    let distro = environment.distro.value().map_or_else(
        || "unknown".into(),
        |distro| {
            distro.version.as_ref().map_or_else(
                || distro.name.clone(),
                |version| format!("{} {version}", distro.name),
            )
        },
    );
    let backend = environment.backend.value().map_or_else(
        || "unknown".into(),
        |backend| {
            backend.version.as_ref().map_or_else(
                || backend.name.clone(),
                |version| format!("{} {version}", backend.name),
            )
        },
    );
    let protocol = environment.protocol.value().map_or_else(
        || "unknown".into(),
        |protocol| format!("{} {}", protocol.name, protocol.version),
    );
    format!(
        "{status}\nAvailable {} · Limited {} · Unavailable {} · Unknown {} · Unsupported {}\nBuild {}\nSource roots {source_roots}\nOE-Core {} · Poky {} · BitBake {}\nDISTRO {distro} · MACHINE {}\nLayer series {layer_series}\nBackend {backend} · Protocol {protocol}",
        summary.available,
        summary.limited,
        summary.unavailable,
        summary.unknown,
        summary.unsupported,
        environment
            .build_directory
            .value()
            .map_or_else(|| "unknown".into(), |path| path.display().to_string()),
        release(&environment.oe_core),
        release(&environment.poky),
        environment
            .bitbake_version
            .value()
            .map_or("unknown", String::as_str),
        environment
            .machine
            .value()
            .map_or("unknown", String::as_str),
    )
}

pub(crate) fn compatibility_filter_label(filter: CompatibilityUiFilter) -> &'static str {
    match filter {
        CompatibilityUiFilter::All => "1 All",
        CompatibilityUiFilter::Available => "2 Available",
        CompatibilityUiFilter::Limited => "3 Limited",
        CompatibilityUiFilter::Unavailable => "4 Unavailable",
        CompatibilityUiFilter::Attention => "5 Attention",
    }
}

pub(crate) fn compatibility_state_label(state: CompatibilityUiCapabilityState) -> &'static str {
    match state {
        CompatibilityUiCapabilityState::Available => "Available",
        CompatibilityUiCapabilityState::Limited => "Limited",
        CompatibilityUiCapabilityState::Unavailable => "Unavailable",
        CompatibilityUiCapabilityState::Unknown => "Unknown",
        CompatibilityUiCapabilityState::Unsupported => "Unsupported",
    }
}

pub(crate) fn compatibility_state_style(app: &App, state: CompatibilityUiCapabilityState) -> Style {
    let palette = ThemePalette::for_app(app);
    match state {
        CompatibilityUiCapabilityState::Available => palette.role(palette.success, Modifier::BOLD),
        CompatibilityUiCapabilityState::Limited => palette.role(palette.warning, Modifier::BOLD),
        CompatibilityUiCapabilityState::Unavailable => palette.role(palette.error, Modifier::BOLD),
        CompatibilityUiCapabilityState::Unknown => {
            palette.role(palette.informational, Modifier::ITALIC)
        }
        CompatibilityUiCapabilityState::Unsupported => {
            palette.role(palette.disabled, Modifier::DIM)
        }
    }
}

pub(crate) fn compatibility_workspace_state_label(
    state: WorkspaceAvailabilityState,
) -> &'static str {
    match state {
        WorkspaceAvailabilityState::Available => "Available",
        WorkspaceAvailabilityState::AvailableWithLimitations => "Limited",
        WorkspaceAvailabilityState::Unavailable => "Unavailable",
        WorkspaceAvailabilityState::Unknown => "Unknown",
        WorkspaceAvailabilityState::Unsupported => "Unsupported",
    }
}

pub(crate) fn compatibility_workspace_state_style(
    app: &App,
    state: WorkspaceAvailabilityState,
) -> Style {
    let palette = ThemePalette::for_app(app);
    match state {
        WorkspaceAvailabilityState::Available => palette.role(palette.success, Modifier::BOLD),
        WorkspaceAvailabilityState::AvailableWithLimitations => {
            palette.role(palette.warning, Modifier::BOLD)
        }
        WorkspaceAvailabilityState::Unavailable => palette.role(palette.error, Modifier::BOLD),
        WorkspaceAvailabilityState::Unknown => {
            palette.role(palette.informational, Modifier::ITALIC)
        }
        WorkspaceAvailabilityState::Unsupported => palette.role(palette.disabled, Modifier::DIM),
    }
}

pub(crate) fn compatibility_inspector_text(app: &App) -> String {
    let projection = app
        .compatibility_ui
        .project(&app.workspace_compatibility, app.daemon.status);
    let CompatibilityUiAuthorityStatus::Current { generation, mode } = projection.authority else {
        let CompatibilityUiAuthorityStatus::Unavailable { reason } = projection.authority else {
            unreachable!()
        };
        return format!(
            "Capability snapshot\nState: unavailable\nReason: {reason}\n\nNo probe is run by this client. Reconnect or wait for daemon synchronization."
        );
    };
    let Some(row) = projection.selected_row() else {
        return format!(
            "Capability snapshot\nGeneration: {generation}\nMode: {mode:?}\n\nNo capability matches the current filter/search."
        );
    };
    compatibility_capability_detail(generation, mode, row)
}

pub(crate) fn compatibility_capability_detail(
    generation: u64,
    mode: yoctui_model::EnvironmentOperatingMode,
    row: &CompatibilityUiCapabilityRow,
) -> String {
    let mut lines = vec![
        format!("Capability: {}", row.id.as_str()),
        format!("State: {}", compatibility_state_label(row.state)),
        format!("Snapshot: {generation} · {mode:?}"),
    ];
    if let Some(reason) = row.reason.as_ref() {
        lines.extend([
            format!("Reason code: {}", reason.code.as_str()),
            format!("Reason: {}", reason.message),
            format!(
                "Requirement: {}",
                reason.requirement.as_deref().unwrap_or("not specified")
            ),
        ]);
    }
    if !row.limitations.is_empty() {
        lines.push(String::new());
        lines.push("Limitations".into());
        lines.extend(row.limitations.iter().map(|value| format!("• {value}")));
    }
    lines.push(String::new());
    lines.push(format!(
        "Implementation: {}",
        row.implementation.as_ref().map_or_else(
            || "none selected".into(),
            |implementation| format!("{} ({:?})", implementation.id, implementation.kind),
        )
    ));
    lines.push(String::new());
    lines.push(format!("Evidence ({})", row.evidence.len()));
    if row.evidence.is_empty() {
        lines.push("No evidence records retained.".into());
    } else {
        for (index, evidence) in row.evidence.iter().enumerate() {
            lines.push(format!(
                "{}. {:?} / {:?} · {}",
                index + 1,
                evidence.kind,
                evidence.outcome,
                evidence.subject
            ));
            lines.push(format!("   {}", evidence.detail));
            if !evidence.argv.is_empty() {
                lines.push(format!("   argv: {}", evidence.argv.join(" ")));
            }
        }
    }
    lines.join("\n")
}
