pub(crate) fn current_search_state(app: &App) -> Option<(bool, bool)> {
    let state = match app.screen {
        Screen::Logs => match app.log_workspace_view {
            LogWorkspaceView::BitBake => (app.logs.searching, !app.logs.query.is_empty()),
            LogWorkspaceView::Yoctui => (
                app.internal_logs.searching,
                !app.internal_logs.query.is_empty(),
            ),
        },
        Screen::Recipes | Screen::Layers | Screen::Configuration => {
            (app.metadata_searching, !app.metadata_query.is_empty())
        }
        Screen::Packages => (app.package_searching, !app.package_query.is_empty()),
        Screen::Images => (
            app.image_artifact_searching,
            !app.image_artifact_query.is_empty(),
        ),
        Screen::Sdk => (
            app.sdk_artifact_searching,
            !app.sdk_artifact_query.is_empty(),
        ),
        Screen::Testing if app.test_view == TestWorkspaceView::Results => {
            (app.test_result_searching, !app.test_result_query.is_empty())
        }
        Screen::Security => (app.security.searching, !app.security.query.is_empty()),
        Screen::Qa => (app.qa.searching, !app.qa.query.is_empty()),
        Screen::RawMode if app.raw_mode.view == yoctui_model::RawModeView::Execution => (
            app.raw_mode.output.searching,
            !app.raw_mode.output.query.is_empty(),
        ),
        Screen::RawMode => (
            app.raw_mode.search.editing,
            !app.raw_mode.search.query.is_empty(),
        ),
        Screen::TerminalSessions => {
            let editing = app.terminal.mode == yoctui_model::TerminalWorkbenchMode::Search;
            (editing, editing && !app.terminal.query.is_empty())
        }
        Screen::Compatibility => (
            app.compatibility_ui.searching,
            !app.compatibility_ui.query.is_empty(),
        ),
        _ => return None,
    };
    Some(state)
}

pub(crate) fn footer_context_items(app: &App, width: u16) -> Vec<String> {
    if app.command_palette_open || app.focus == FocusTarget::CommandPalette {
        return [
            "Type search",
            "↑/↓ select",
            "Ctrl+U clear",
            "Enter run",
            "Esc close",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
    }
    if app.active_dialog().is_some() || app.focus == FocusTarget::Dialog {
        return ["Enter select/confirm", "Esc cancel"]
            .into_iter()
            .map(str::to_owned)
            .collect();
    }
    if current_search_state(app).is_some_and(|(editing, _)| editing) {
        return [
            "Type query",
            "Backspace edit",
            "Ctrl+U clear",
            "Enter done",
            "Esc done",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
    }

    let responsive_override = (app.screen == Screen::Images && width <= 129)
        || (matches!(
            app.screen,
            Screen::Sdk | Screen::Testing | Screen::Security | Screen::Qa
        ) && width <= 90);
    let source = if width < WIDE_WORKBENCH_MIN_WIDTH {
        responsive_footer_shortcuts(app, width)
    } else {
        footer_shortcuts(app)
    };
    let mut items = source.split(" | ").map(str::to_owned).collect::<Vec<_>>();
    if current_search_state(app).is_some_and(|(_, filtered)| filtered) {
        items.insert(0, "Ctrl+U clear".into());
    }
    if !responsive_override && width >= 100 {
        let focus_prefix_len = pane_focus_shortcuts(app)
            .map(|shortcuts| shortcuts.split(" | ").count())
            .unwrap_or_default();
        if items.len() >= focus_prefix_len {
            items.drain(..focus_prefix_len);
        }
    }
    items.retain(|item| {
        !matches!(
            item.split_once(' ').map_or(item.as_str(), |(key, _)| key),
            "?" | "q" | "Ctrl+P"
        )
    });
    items.truncate(6);
    let focus = (yoctui_model::pane_focus_targets(app).count() > 1).then_some("Tab Focus");
    if let Some(focus) = focus {
        items.insert(0, focus.into());
    }
    items
}

pub(crate) fn function_footer_item(key: FunctionKey) -> String {
    let shortcut = FUNCTION_SHORTCUTS
        .iter()
        .find(|shortcut| shortcut.key == key)
        .expect("the closed function-key catalog contains every FunctionKey");
    format!("{} {}", shortcut.key_label, shortcut.action_label)
}

pub(crate) fn footer_item_width(item: &str) -> usize {
    Line::from(item).width()
}

pub(crate) fn footer_items_width(items: &[String]) -> usize {
    items
        .iter()
        .map(|item| footer_item_width(item))
        .sum::<usize>()
        + items.len().saturating_sub(1) * 3
}

pub(crate) fn push_footer_items_that_fit(
    output: &mut Vec<String>,
    candidates: &[String],
    budget: usize,
) {
    for candidate in candidates {
        let added = footer_item_width(candidate) + usize::from(!output.is_empty()) * 3;
        if footer_items_width(output).saturating_add(added) > budget {
            break;
        }
        output.push(candidate.clone());
    }
}

pub(crate) fn footer_rail_shortcuts(app: &App, width: u16) -> String {
    let budget = usize::from(width);
    let compact = width < 100;
    if compact
        && matches!(
            app.screen,
            Screen::Sdk | Screen::Testing | Screen::Security | Screen::Qa
        )
        && app.active_dialog().is_none()
        && !app.command_palette_open
    {
        return responsive_footer_shortcuts(app, width);
    }
    let essentials = if compact {
        vec!["? Help".into(), "Ctrl+P Menu".into(), "q Quit".into()]
    } else {
        vec![
            function_footer_item(FunctionKey::F1),
            function_footer_item(FunctionKey::F10),
            "q Quit".into(),
        ]
    };
    let essentials_width = footer_items_width(&essentials);
    if essentials_width >= budget {
        let mut output = Vec::new();
        push_footer_items_that_fit(&mut output, &essentials, budget);
        return output.join(" | ");
    }

    let prefix_budget = budget.saturating_sub(essentials_width + 3);
    let mut prefix = Vec::new();
    push_footer_items_that_fit(
        &mut prefix,
        &footer_context_items(app, width),
        prefix_budget,
    );

    if !compact {
        let optional = FUNCTION_SHORTCUTS
            .iter()
            .filter(|shortcut| {
                !matches!(shortcut.key, FunctionKey::F1 | FunctionKey::F9 | FunctionKey::F10)
                    && !matches!(shortcut.route, FunctionShortcutRoute::Open(screen) if screen == app.screen)
                    && !prefix.iter().any(|item| {
                        item.split_once(' ')
                            .map_or(item.as_str(), |(key, _)| key)
                            == shortcut.key_label
                    })
            })
            .map(|shortcut| format!("{} {}", shortcut.key_label, shortcut.action_label))
            .collect::<Vec<_>>();
        push_footer_items_that_fit(&mut prefix, &optional, prefix_budget);
    }

    prefix.extend(essentials);
    prefix.join(" | ")
}
