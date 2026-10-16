//! Read-only Raw history and local favorite management; never launches commands.
use super::*;

pub(crate) fn raw_phase_label(phase: yoctui_model::RawExecutionPhase) -> &'static str {
    use yoctui_model::{RawExecutionOutcome as Outcome, RawExecutionPhase as Phase};
    match phase {
        Phase::Queued => "QUEUED",
        Phase::Starting => "STARTING",
        Phase::Running => "RUNNING",
        Phase::Cancelling => "CANCELLING",
        Phase::Terminal(Outcome::Succeeded) => "SUCCEEDED",
        Phase::Terminal(Outcome::Failed) => "FAILED",
        Phase::Terminal(Outcome::Cancelled) => "CANCELLED",
        Phase::Terminal(Outcome::Lost) => "LOST",
    }
}

pub(crate) fn raw_favorites_workspace(frame: &mut Frame, app: &App, area: Rect, _width: u16) {
    let state = &app.raw_mode;
    let count = state.favorites.len();
    let range = yoctui_model::centered_viewport_range(
        (count > 0).then_some(state.favorite_selection),
        count,
        (usize::from(area.height.saturating_sub(3)) / 4).max(1),
    );
    let title = format!(
        "Raw Mode / Favorites · {}",
        BoundedScrollIndicator::new(range.start, range.len(), count).label()
    );
    let block = pane_block(app, &title, app.focus == FocusTarget::Workspace);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if count == 0 {
        frame.render_widget(Paragraph::new("No Raw favorites.\nMark a command with f in the catalog.\nEsc returns to the catalog."), inner);
        return;
    }
    let palette = ThemePalette::for_app(app);
    let mut lines = Vec::new();
    for index in range {
        let favorite = &state.favorites[index];
        let projection = favorite.project(
            yoctui_model::builtin_raw_catalog(),
            app.workspace_compatibility.authority(),
        );
        let command = yoctui_model::builtin_raw_catalog()
            .command(&favorite.command)
            .map(raw_command_template)
            .unwrap_or_else(|| "<command absent from catalog>".into());
        let selected = index == state.favorite_selection;
        let style = if selected {
            palette.selected()
        } else {
            palette.base()
        };
        for text in [
            format!(
                "{} {} · {}{}",
                if selected { "▶" } else { " " },
                favorite.name,
                raw_availability_label(projection.availability.state),
                if projection.stale { " · STALE" } else { "" }
            ),
            format!("    {command}"),
            format!(
                "    defaults: {} · argv: {}",
                favorite.parameter_defaults.len(),
                favorite.additional_arguments.as_slice().join(" ")
            ),
            format!(
                "    {}",
                projection
                    .reason
                    .as_deref()
                    .unwrap_or("Current catalog and capability authority")
            ),
        ] {
            lines.push(Line::styled(bounded_cell_text(&text, inner.width), style));
        }
    }
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(inner);
    frame.render_widget(Paragraph::new(lines), rows[0]);
    frame.render_widget(
        Paragraph::new("Enter review · i inspect · r rename · [/] reorder · x remove · Esc back"),
        rows[1],
    );
}

pub(crate) fn raw_history_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.raw_mode;
    let ids = state.history_request_ids();
    let rows = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(if area.height >= 18 { 7 } else { 3 }),
    ])
    .split(area);
    let range = yoctui_model::centered_viewport_range(
        (!ids.is_empty()).then_some(state.history_selection),
        ids.len(),
        (usize::from(rows[0].height.saturating_sub(2)) / 2).max(1),
    );
    let title = format!(
        "Raw Mode / History · {}",
        BoundedScrollIndicator::new(range.start, range.len(), ids.len()).label()
    );
    let palette = ThemePalette::for_app(app);
    let mut lines = Vec::new();
    for index in range {
        let id = &ids[index];
        let execution = state.execution_states.get(id);
        let record = state.history.iter().find(|record| &record.request_id == id);
        let Some(command) = execution
            .map(|e| &e.request.command)
            .or_else(|| record.map(|r| &r.command))
        else {
            continue;
        };
        let label = yoctui_model::builtin_raw_catalog()
            .command(command)
            .map(|c| c.label.as_str())
            .unwrap_or(command.as_str());
        let phase = execution
            .map(|e| e.phase)
            .or_else(|| record.map(|r| yoctui_model::RawExecutionPhase::Terminal(r.outcome)));
        let code = execution
            .and_then(|e| e.result.as_ref().and_then(|r| r.exit_code))
            .or_else(|| record.and_then(|r| r.exit_code));
        let timestamp = execution
            .map(|e| e.started_unix_ms.unwrap_or(e.queued_unix_ms))
            .or_else(|| record.map(|r| r.started_unix_ms));
        let selected = index == state.history_selection;
        let style = if selected {
            palette.selected()
        } else {
            palette.base()
        };
        lines.push(Line::styled(
            bounded_cell_text(
                &format!("{} {label}", if selected { "▶" } else { " " }),
                area.width.saturating_sub(2),
            ),
            style,
        ));
        let phase_style = match phase {
            Some(yoctui_model::RawExecutionPhase::Terminal(
                yoctui_model::RawExecutionOutcome::Succeeded,
            )) => palette.role(palette.success, Modifier::BOLD),
            Some(yoctui_model::RawExecutionPhase::Terminal(
                yoctui_model::RawExecutionOutcome::Failed | yoctui_model::RawExecutionOutcome::Lost,
            )) => palette.role(palette.error, Modifier::BOLD),
            Some(yoctui_model::RawExecutionPhase::Running) => {
                palette.role(palette.running, Modifier::BOLD)
            }
            _ => palette.role(palette.warning, Modifier::BOLD),
        };
        lines.push(Line::styled(
            bounded_cell_text(
                &format!(
                    "  {} · exit {} · {} · {}",
                    phase.map(raw_phase_label).unwrap_or("UNKNOWN"),
                    code.map_or("--".into(), |code| code.to_string()),
                    crate::image_inspector::image_artifact_timestamp(timestamp.map(|ms| ms / 1000)),
                    if execution.is_some() {
                        "output retained"
                    } else {
                        "metadata only"
                    }
                ),
                area.width.saturating_sub(2),
            ),
            if selected { style } else { phase_style },
        ));
    }
    if ids.is_empty() {
        lines.push(Line::raw(
            "No Raw executions yet. Reviewed commands appear here when accepted.",
        ));
    }
    frame.render_widget(
        Paragraph::new(lines).block(pane_block(app, &title, app.focus == FocusTarget::Workspace)),
        rows[0],
    );
    let detail = state.selected_history_request().map_or_else(|| "No execution selected.".into(), |id| {
        let execution = state.execution_states.get(&id);
        let record = state.history.iter().find(|record| record.request_id == id);
        let params = execution.map(|e| &e.request.parameters).or_else(|| record.map(|r| &r.parameters));
        format!("Request: {id}\nParameters: {}\n{}\nEnter views retained output; never reruns. r reviews a NEW command. Esc back.",
            params.map(|params| params.iter().map(|(key, value)| format!("{key}={}", value.argument())).collect::<Vec<_>>().join(" ")).unwrap_or_default(),
            if execution.is_some() { "Active/detached sessions can be reopened. Only c requests cancellation." }
            else { "Only metadata remains; original output cannot be reconstructed." })
    });
    frame.render_widget(
        Paragraph::new(detail)
            .wrap(Wrap { trim: false })
            .block(pane_block(app, "Selected execution", false)),
        rows[1],
    );
}

pub(crate) fn raw_management_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let popup = dialog_popup_rect(area, 85, 11);
    clear_popup(frame, app, popup);
    let state = &app.raw_mode;
    let (title, text) = if let Some(name) = &state.favorite_rename {
        (
            "Rename Raw favorite",
            format!(
                "Name: {name}▏\n\n{}\nEnter save · Esc cancel · Ctrl+U clear · Ctrl+V paste",
                state
                    .notification
                    .as_deref()
                    .unwrap_or("Changes affect this local favorite only.")
            ),
        )
    } else {
        let favorite = state
            .favorite_confirmation
            .as_ref()
            .and_then(|confirmation| {
                state
                    .favorites
                    .iter()
                    .find(|favorite| favorite.command == confirmation.command)
            });
        (
            "Remove Raw favorite?",
            format!(
                "Name: {}\nCommand: {}\n\nRemoves only this favorite, not recipes or build files.\nEnter remove · Esc cancel",
                favorite.map_or("<unavailable>", |favorite| favorite.name.as_str()),
                state
                    .favorite_confirmation
                    .as_ref()
                    .map_or("<unavailable>", |c| c.command.as_str())
            ),
        )
    };
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .block(dialog_block(app, title, DialogTone::Standard)),
        popup,
    );
}
