pub(crate) fn pane_focus_shortcuts(app: &App) -> Option<String> {
    if matches!(app.focus, FocusTarget::Dialog | FocusTarget::CommandPalette) {
        return None;
    }
    let mut targets = [FocusTarget::Navigator; 3];
    let mut target_count = 0;
    for target in yoctui_model::pane_focus_targets(app) {
        targets[target_count] = target;
        target_count += 1;
    }
    let current = targets[..target_count]
        .iter()
        .position(|target| *target == app.focus)
        .unwrap_or_default();
    let route = if target_count == 1 {
        "no other actionable panes".to_owned()
    } else {
        let next = targets[(current + 1) % target_count];
        let previous = targets[(current + target_count - 1) % target_count];
        format!("Tab {} | Shift+Tab {}", next.label(), previous.label())
    };
    Some(format!(
        "Focus {}{} | {route}",
        app.pane_focus_label(),
        if app.zoomed_pane.is_some() {
            " [ZOOM]"
        } else {
            ""
        }
    ))
}

pub(crate) fn with_focus_shortcuts(app: &App, shortcuts: &str) -> String {
    pane_focus_shortcuts(app).map_or_else(
        || shortcuts.to_owned(),
        |focus| format!("{focus} | {shortcuts}"),
    )
}

pub(crate) fn workspace_destination_label(destination: WorkspaceDestination) -> &'static str {
    destination.label()
}

pub(crate) fn compatibility_destination_detail(
    app: &App,
    destination: WorkspaceDestination,
    width: u16,
) -> String {
    let availability = compatibility_ui_workspace_destination_action_availability(
        &app.workspace_compatibility,
        destination,
    );
    let mut lines = vec![
        format!("Destination: {}", workspace_destination_label(destination)),
        format!(
            "Compatibility: {}",
            compatibility_workspace_state_label(availability.state)
        ),
        String::new(),
        "Navigation remains available so this environment state can be inspected.".into(),
    ];
    if let Some(reason) = availability.exact_reason() {
        lines.extend([String::new(), format!("Reason: {reason}")]);
    }
    lines.extend(
        availability
            .limitations
            .iter()
            .map(|limitation| format!("Limitation: {limitation}")),
    );
    if !availability.implementations.is_empty() {
        lines.extend([
            String::new(),
            format!(
                "Implementation: {}",
                availability
                    .implementations
                    .iter()
                    .map(|(_, implementation)| implementation.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ]);
    }
    let actions = compatibility_workspace_actions(app, destination);
    if !actions.is_empty() {
        lines.extend([
            String::new(),
            "Actions / Compatibility".into(),
            action_list_plain(&actions, width),
        ]);
    }
    lines.join("\n")
}

pub(crate) fn compatibility_workspace_actions(
    app: &App,
    destination: WorkspaceDestination,
) -> Vec<ActionListItem> {
    yoctui_model::compatibility_ui_workspace_action_presentations(
        &app.workspace_compatibility,
        destination,
    )
    .into_iter()
    .map(|action| {
        let availability = action.availability;
        let state = if availability.state == WorkspaceAvailabilityState::Available
            && availability.implementations.is_empty()
        {
            "Local"
        } else {
            compatibility_workspace_state_label(availability.state)
        };
        let marker = match availability.state {
            WorkspaceAvailabilityState::Available => "✓",
            WorkspaceAvailabilityState::AvailableWithLimitations => "~",
            WorkspaceAvailabilityState::Unavailable => "×",
            WorkspaceAvailabilityState::Unknown => "?",
            WorkspaceAvailabilityState::Unsupported => "!",
        };
        let mut details = Vec::new();
        if let Some(reason) = availability.exact_reason() {
            details.push(format!("Reason: {reason}"));
        }
        details.extend(
            availability
                .limitations
                .iter()
                .map(|limitation| format!("Limitation: {limitation}")),
        );
        if !availability.implementations.is_empty() {
            details.push(format!(
                "Implementation: {}",
                availability
                    .implementations
                    .iter()
                    .map(|(_, implementation)| implementation.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        ActionListItem {
            marker,
            label: action.label.into(),
            shortcut: action.shortcut.into(),
            state: state.into(),
            enabled: availability.enabled,
            details,
        }
    })
    .collect()
}

pub(crate) fn with_compatibility_footer(
    app: &App,
    destination: WorkspaceDestination,
    shortcuts: String,
) -> String {
    let availability = compatibility_ui_workspace_destination_action_availability(
        &app.workspace_compatibility,
        destination,
    );
    if availability.state == WorkspaceAvailabilityState::Available {
        return shortcuts;
    }
    let reason = availability.exact_reason().unwrap_or_else(|| {
        "The connected environment did not provide an exact compatibility reason.".into()
    });
    format!(
        "{shortcuts} | {}: {reason}",
        compatibility_workspace_state_label(availability.state)
    )
}
