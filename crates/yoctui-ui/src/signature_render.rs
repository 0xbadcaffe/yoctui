//! Signature render.
use super::*;

pub(crate) fn signatures_workspace(frame: &mut Frame, app: &App, area: Rect, terminal_width: u16) {
    if terminal_width >= 110 {
        let panes = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);
        signature_records(frame, app, panes[0]);
        signature_detail(frame, app, panes[1]);
    } else {
        let panes =
            Layout::vertical([Constraint::Percentage(44), Constraint::Percentage(56)]).split(area);
        signature_records(frame, app, panes[0]);
        signature_detail(frame, app, panes[1]);
    }
}

pub(crate) fn signature_target_label(app: &App) -> String {
    app.signature_dump.target().map_or_else(
        || "no target".into(),
        |target| format!("{}:{}", target.recipe, target.task),
    )
}

pub(crate) fn signature_comparison_sides(
    state: &SignatureComparisonState,
) -> (
    Option<&yoctui_model::SignatureIdentity>,
    Option<&yoctui_model::SignatureIdentity>,
) {
    match state {
        SignatureComparisonState::NotSelected => (None, None),
        SignatureComparisonState::Ready { left, right } => (left.as_ref(), right.as_ref()),
        SignatureComparisonState::Loading { request }
        | SignatureComparisonState::AvailableEmpty { request }
        | SignatureComparisonState::Available { request, .. }
        | SignatureComparisonState::Partial { request, .. }
        | SignatureComparisonState::Failed { request, .. } => {
            (Some(&request.left), Some(&request.right))
        }
    }
}

pub(crate) fn signature_records(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!("Signatures — {}", signature_target_label(app));
    let block = pane_block(app, &title, app.focus == FocusTarget::Workspace);
    let records = app.signature_dump.records();
    let text = match &app.signature_dump {
        SignatureDumpState::NotLoaded => {
            "Signatures have not been loaded.\n\nReturn to Recipes and press Z.".into()
        }
        SignatureDumpState::Loading { .. } => {
            "Loading authoritative signature artifacts…\n\nEsc requests cancellation.".into()
        }
        SignatureDumpState::AvailableEmpty { .. } => {
            "BitBake reported no signature artifacts for this recipe/task.".into()
        }
        SignatureDumpState::Failed { message, .. } => {
            format!("Signature dump failed:\n{message}\n\nr retries; Esc returns to Recipes.")
        }
        SignatureDumpState::Available { .. } | SignatureDumpState::Partial { .. } => {
            let (left, right) = signature_comparison_sides(&app.signature_comparison);
            let records = records.unwrap_or_default();
            let selection = records
                .iter()
                .position(|record| app.signature_selection.as_ref() == Some(&record.identity));
            let capacity = (usize::from(area.height.saturating_sub(2)) / 2).max(1);
            let viewport =
                yoctui_model::centered_viewport_range(selection, records.len(), capacity);
            let mut lines = records[viewport]
                .iter()
                .map(|record| {
                    let selected = app.signature_selection.as_ref() == Some(&record.identity);
                    let side = match (
                        left == Some(&record.identity),
                        right == Some(&record.identity),
                    ) {
                        (true, true) => "12",
                        (true, false) => "1 ",
                        (false, true) => " 2",
                        (false, false) => "  ",
                    };
                    format!(
                        "{} [{}] {}\n    {}",
                        if selected { ">" } else { " " },
                        side,
                        record
                            .identity
                            .hash
                            .as_deref()
                            .unwrap_or("hash unavailable"),
                        record.identity.path.as_ref().map_or_else(
                            || "path unavailable".into(),
                            |path| path.display().to_string()
                        )
                    )
                })
                .collect::<Vec<_>>();
            if let SignatureDumpState::Partial { limitations, .. } = &app.signature_dump {
                lines.push(String::new());
                lines.push("Partial result:".into());
                lines.extend(limitations.iter().map(|value| format!("! {value}")));
            }
            lines.join("\n")
        }
    };
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: false }),
        area,
    );
}

pub(crate) fn signature_detail(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(
        Paragraph::new(signature_detail_text(app))
            .block(pane_block(
                app,
                "Selected record and comparison",
                app.focus == FocusTarget::Inspector,
            ))
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(crate) fn signature_detail_text(app: &App) -> String {
    let selected = app.signature_dump.records().and_then(|records| {
        app.signature_selection
            .as_ref()
            .and_then(|identity| records.iter().find(|record| &record.identity == identity))
    });
    let mut lines = Vec::new();
    if let Some(record) = selected {
        lines.extend([
            format!(
                "Hash: {}",
                record.identity.hash.as_deref().unwrap_or("unavailable")
            ),
            format!(
                "Base hash: {}",
                record.base_hash.as_deref().unwrap_or("unavailable")
            ),
            format!(
                "Task hash: {}",
                record.task_hash.as_deref().unwrap_or("unavailable")
            ),
            String::new(),
            format!("Variables ({})", record.variables.len()),
        ]);
        lines.extend(record.variables.iter().take(120).map(|value| {
            format!(
                "{} = {}",
                value.name,
                value.value.as_deref().unwrap_or("unavailable")
            )
        }));
        if record.variables.len() > 120 {
            lines.push(format!(
                "… {} more bounded variables",
                record.variables.len() - 120
            ));
        }
        lines.push(String::new());
        lines.push(format!("Task dependencies ({})", record.dependencies.len()));
        lines.extend(
            record
                .dependencies
                .iter()
                .take(80)
                .map(|dependency| format!("• {dependency}")),
        );
        if record.dependencies.len() > 80 {
            lines.push(format!(
                "… {} more bounded dependencies",
                record.dependencies.len() - 80
            ));
        }
    } else {
        lines.push("No current signature record is selected.".into());
    }
    lines.push(String::new());
    lines.push("Comparison".into());
    match &app.signature_comparison {
        SignatureComparisonState::NotSelected => {
            lines.push("Assign two records with 1 and 2.".into());
        }
        SignatureComparisonState::Ready { left, right } => {
            lines.push(format!(
                "1: {}\n2: {}",
                left.as_ref()
                    .and_then(|identity| identity.hash.as_deref())
                    .unwrap_or("not selected"),
                right
                    .as_ref()
                    .and_then(|identity| identity.hash.as_deref())
                    .unwrap_or("not selected")
            ));
        }
        SignatureComparisonState::Loading { .. } => {
            lines.push("Comparing authoritative signature artifacts…".into());
        }
        SignatureComparisonState::AvailableEmpty { .. } => {
            lines.push("No typed differences were found.".into());
        }
        SignatureComparisonState::Failed { message, .. } => {
            lines.push(format!("Comparison failed: {message}"));
        }
        SignatureComparisonState::Available { differences, .. }
        | SignatureComparisonState::Partial { differences, .. } => {
            lines.extend(differences.iter().take(160).map(|difference| {
                let category = match difference.category {
                    SignatureDifferenceCategory::BaseHash => "hash",
                    SignatureDifferenceCategory::ChangedValue => "value",
                    SignatureDifferenceCategory::Dependency => "dependency",
                    SignatureDifferenceCategory::Unavailable => "unavailable",
                };
                format!(
                    "[{category}] {}: {} → {}",
                    difference.key,
                    difference.left.as_deref().unwrap_or("unavailable"),
                    difference.right.as_deref().unwrap_or("unavailable")
                )
            }));
            if differences.len() > 160 {
                lines.push(format!(
                    "… {} more bounded differences",
                    differences.len() - 160
                ));
            }
            if let SignatureComparisonState::Partial { limitations, .. } = &app.signature_comparison
            {
                lines.push(String::new());
                lines.push("Partial comparison:".into());
                lines.extend(limitations.iter().map(|value| format!("! {value}")));
            }
        }
    }
    lines.join("\n")
}
