//! Popup render.
use super::*;

pub(crate) fn notification_popup(frame: &mut Frame, app: &App, area: Rect) {
    let Some(message) = popup_notification(app) else {
        return;
    };
    let width = area.width.saturating_sub(8).min(76);
    let content_width = usize::from(width.saturating_sub(4).max(1));
    let wrapped_lines = message
        .lines()
        .map(|line| line.chars().count().max(1).div_ceil(content_width))
        .sum::<usize>();
    let height = u16::try_from(wrapped_lines.saturating_add(4))
        .unwrap_or(u16::MAX)
        .clamp(5, 10);
    let popup = bounded_dialog_rect(area, width, height);
    clear_popup(frame, app, popup);
    let actionable =
        app.build.status == BuildStatus::Failed && app.logs.diagnostics().next().is_some();
    let hint = if actionable {
        "Enter view errors · Esc dismiss"
    } else {
        "Esc dismiss"
    };
    frame.render_widget(
        Paragraph::new(format!("{message}\n\n{hint}"))
            .block(dialog_block(app, "Notice", DialogTone::Standard))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn popup_notification(app: &App) -> Option<&str> {
    let message = app.notification.as_deref()?;
    notification_requires_acknowledgement(message).then_some(message)
}

pub(crate) fn dialog_compatibility_overlay(
    frame: &mut Frame,
    app: &App,
    dialog: &Dialog,
    area: Rect,
) {
    let availability = yoctui_model::compatibility_ui_dialog_action_availability(
        &app.workspace_compatibility,
        dialog,
    );
    let local = availability.state == WorkspaceAvailabilityState::Available
        && availability.implementations.is_empty();
    if local {
        return;
    }
    let state = if local {
        "Local"
    } else {
        compatibility_workspace_state_label(availability.state)
    };
    let mut state_line = format!(
        "Dialog compatibility · State: {state} · Confirmation {}",
        if availability.enabled {
            "available"
        } else {
            "disabled"
        }
    );
    if !availability.implementations.is_empty() {
        state_line.push_str(&format!(
            " · Implementation: {}",
            availability
                .implementations
                .iter()
                .map(|(_, implementation)| implementation.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let mut detail = Vec::new();
    if let Some(reason) = availability.exact_reason() {
        detail.push(reason.to_owned());
    }
    detail.extend(
        availability
            .limitations
            .iter()
            .map(|limitation| format!("Limitation: {limitation}")),
    );
    let popup = Rect::new(area.x, area.y, area.width, 2);
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(format!("{state_line}\n{}", detail.join(" · ")))
            .style(compatibility_workspace_state_style(app, availability.state)),
        popup,
    );
}

pub(crate) fn theme_picker(frame: &mut Frame, app: &App, selection: usize, area: Rect) {
    let popup = Rect::new(
        area.width / 4,
        area.height / 5,
        area.width / 2,
        area.height * 3 / 5,
    );
    clear_popup(frame, app, popup);
    let rows = yoctui_model::THEMES
        .iter()
        .enumerate()
        .map(|(index, theme)| {
            Row::new([format!(
                "{} {}",
                if index == selection { "▶" } else { " " },
                theme.display_name()
            )])
            .style(selected_style(app, index == selection))
        });
    frame.render_widget(
        Table::new(rows, [Constraint::Min(1)])
            .block(dialog_block(
                app,
                if app.color_forced_off {
                    "Theme — preview locked by --no-color"
                } else {
                    "Theme — applies immediately"
                },
                DialogTone::Standard,
            ))
            .footer(Row::new(["↑/↓ select  Enter apply  Esc restore"])),
        popup,
    );
}

pub(crate) fn build_environment_editor(
    frame: &mut Frame,
    app: &App,
    editor: &yoctui_model::PopupEditor,
    area: Rect,
) {
    toml_popup_editor(frame, app, area, "Build environment.toml", editor, None);
}

pub(crate) fn build_environment_clone_editor(
    frame: &mut Frame,
    app: &App,
    editor: &yoctui_model::PopupEditor,
    area: Rect,
) {
    toml_popup_editor(frame, app, area, "Clone Poky.toml", editor, None);
}

pub(crate) fn build_environment_clone_review(
    frame: &mut Frame,
    app: &App,
    plan: &yoctui_model::BuildEnvironmentClonePlan,
    area: Rect,
) {
    let popup = Rect::new(
        area.width / 8,
        area.height / 4,
        area.width * 3 / 4,
        area.height / 2,
    );
    clear_popup(frame, app, popup);
    let clone = if plan.request.revision.is_some() {
        format!(
            "git clone --no-checkout {} {}",
            plan.request.repository,
            plan.request.destination.display()
        )
    } else {
        format!(
            "git clone {} {}",
            plan.request.repository,
            plan.request.destination.display()
        )
    };
    let checkout = plan
        .request
        .revision
        .as_ref()
        .map_or_else(String::new, |revision| {
            format!(
                "\ngit -C {} checkout {revision}",
                plan.request.destination.display()
            )
        });
    frame.render_widget(
        Paragraph::new(format!("Review before cloning:\n\n{clone}{checkout}\n\nBuild directory: {}\n\nEnter confirms this network operation. Esc cancels.", plan.build_dir.display()))
            .block(dialog_block(
                app,
                "Clone Poky review",
                DialogTone::Confirmation,
            ))
            .style(ThemePalette::for_app(app).base())
            .wrap(Wrap { trim: true }),
        popup,
    );
}
