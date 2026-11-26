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
        Screen::Kernel | Screen::Firmware if app.platform_menuconfig_visible() => {
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
    if app.editor_settings.draft.is_some() {
        return footer_shortcuts(app).split(" | ").map(str::to_owned).collect();
    }
    if app.menu.is_open() {
        return ["↑/↓ select", "Enter open", "Esc close"].into_iter().map(str::to_owned).collect();
    }
    if app.screen == Screen::RawMode && !app.menu.is_open() && !app.command_palette_open
        && (app.focus == FocusTarget::Workspace || app.raw_mode.modal_active()
            || matches!(app.raw_mode.view, yoctui_model::RawModeView::Form | yoctui_model::RawModeView::Preview)) {
        return raw_mode_shortcuts(app).split(" | ").map(str::to_owned).collect();
    }
    if app.terminal_browser_visible() && app.focus == FocusTarget::Workspace
        && app.terminal.mode == yoctui_model::TerminalWorkbenchMode::Live
        && app.active_dialog().is_none() && !app.menu.is_open() && !app.command_palette_open {
        return ["Up/Down sessions", "PgUp/PgDn", "Home/End", "Enter open", "/ global search", "n new shell", "Esc navigator"].into_iter().map(str::to_owned).collect();
    }
    if app.screen == Screen::DiffTool && app.focus == FocusTarget::Workspace
        && app.active_dialog().is_none() && !app.command_palette_open && !app.menu.is_open()
    {
        return if app.file_diff.comparison.is_some() {
            ["Up/Down scroll", "PgUp/PgDn", "Home/End", "Left/Right pan", "Esc file trees"].into_iter().map(str::to_owned).collect()
        } else {
            ["Tab side", "Space mark", "F5 compare", "Enter expand", "Backspace parent", "Esc navigator"].into_iter().map(str::to_owned).collect()
        };
    }
    if app.screen == Screen::Help && app.active_dialog().is_none()
        && !app.command_palette_open && !app.menu.is_open() && !app.onboarding.open
    {
        if app.help_about {
            return ["F1 keyboard help", "g operator guide", "Esc dashboard"].into_iter().map(str::to_owned).collect();
        }
        return ["Up/Down scroll", "PgUp/PgDn", "Home/End", "g operator guide", "Esc dashboard"].into_iter().map(str::to_owned).collect();
    }
    if app.screen == Screen::Daemons && !app.menu.is_open() && !app.command_palette_open
        && (app.focus == FocusTarget::Workspace || app.daemon_manager.editing || app.daemon_manager.review.is_some()) {
        return if app.daemon_manager.editing { vec!["Tab field".into(), "Ctrl+V paste".into(), "Enter review".into(), "Esc cancel".into()] }
        else if app.daemon_manager.review.is_some() { vec!["Enter confirm".into(), "Esc cancel".into()] }
        else if app.focus == FocusTarget::Workspace { vec!["1 health".into(), "2 logs".into(), "r refresh".into(), "s start".into(), "x stop".into(), "t restart".into(), "c configure".into(), "PgUp/PgDn scroll".into()] }
        else { vec!["Enter open".into(), "Tab focus".into(), "F12 Menu".into()] };
    }
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
    if matches!(app.active_dialog(), Some(Dialog::KernelDebug(d)) if d.draft.tool.configuration_prep()) {
        return if matches!(app.kernel_debug.pending, Some(yoctui_model::KernelDebugOperation::ExportInstrumentation { .. })) {
            vec!["Exporting confirmed file; wait for result".into()]
        } else if app.kernel_debug.instrumentation_preview.is_some() {
            vec!["Enter export NEW .cfg".into(), "PgUp/PgDn review".into(), "Esc edit".into()]
        } else {
            vec!["Tab field".into(), "Enter inspect".into(), "Esc cancel".into()]
        };
    }
    if app.active_dialog().is_some() || app.focus == FocusTarget::Dialog {
        return ["Enter select/confirm", "Esc cancel"]
            .into_iter()
            .map(str::to_owned)
            .collect();
    }
    if current_search_state(app).is_some_and(|(editing, _)| editing) {
        // Search keeps the lease while operating on the displayed output.
        if app.screen == Screen::TerminalSessions {
            return ["Type query", "↑/↓ history", "PgUp/PgDn history", "Home/End", "Ctrl+U clear", "Enter/Esc done"]
                .into_iter().map(str::to_owned).collect();
        }
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

    if app.screen == Screen::TerminalSessions && app.focus == FocusTarget::Workspace {
        use yoctui_model::TerminalWorkbenchMode as Mode;
        let hints: &[&str] = match app.terminal.mode {
            Mode::Rename => &["Type name", "Backspace delete", "Enter save", "Esc cancel"],
            Mode::PasteReview => &["Enter send paste", "Esc cancel"],
            Mode::KillConfirmation => &["Enter stop session", "Esc cancel"],
            Mode::Help => &["Esc close shortcuts"],
            Mode::Copy => &["Up/Down line", "Enter copy", "/ find output", "Esc back"],
            Mode::Live | Mode::Search => &[],
        };
        if !hints.is_empty() {
            return hints.iter().map(|hint| (*hint).to_owned()).collect();
        }
    }

    if app.terminal_console_fullscreen() && app.focus == FocusTarget::Workspace
        && app.terminal.mode == yoctui_model::TerminalWorkbenchMode::Live
    {
        return footer_shortcuts(app).split(" | ").map(str::to_owned).collect();
    }

    let responsive_override = app.focus == FocusTarget::Workspace
        && app.screen == Screen::Images
        && width <= 129;
    let source = if responsive_override {
        responsive_footer_shortcuts(app, width)
    } else {
        footer_shortcuts(app)
    };
    let mut items = source.split(" | ").map(str::to_owned).collect::<Vec<_>>();
    if current_search_state(app).is_some_and(|(_, filtered)| filtered) {
        items.insert(0, "Ctrl+U clear".into());
    }
    if !responsive_override {
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
    use yoctui_model::TerminalWorkbenchMode as Mode;

    let owner = app.input_owner();
    let modal = matches!(owner, FocusTarget::Dialog | FocusTarget::CommandPalette);
    let console = !modal && owner == FocusTarget::Workspace
        && (app.terminal_console_fullscreen() || app.platform_menuconfig_visible());
    let terminal_mode = !modal && yoctui_app::terminal_owns_input(app);
    let mut items = footer_context_items(app, width);
    let mut essentials = Vec::new();

    if console && app.terminal.mode == Mode::Live {
        let label = if app.selected_terminal_is_writer() {
            "Terminal input"
        } else {
            "Read only"
        };
        if !app.selected_terminal_is_writer()
            && app.selected_terminal_details().is_some_and(|details| details.writer.is_some())
        {
            items.retain(|item| item != "o type in terminal");
            items.insert(0, "Writer in use".into());
        }
        items.insert(0, label.into());
    } else if !modal && !terminal_mode && app.editor_settings.draft.is_none() {
        if width < 100 {
            essentials.push("q Quit".into());
            essentials.push("? Help · F12 Menu".into());
        } else {
            essentials.push("q Quit".into());
            essentials.push(format!(
                "{} · {}",
                function_footer_item(FunctionKey::F1),
                function_footer_item(FunctionKey::F12),
            ));
        }
    }

    items.retain(|item| !essentials.iter().any(|entry| entry == item)
        && (essentials.is_empty() || !item.to_ascii_lowercase().starts_with("f12 menu")));
    let budget = usize::from(width);
    let reserved = footer_items_width(&essentials);
    let mut visible = Vec::new();
    items.truncate(7_usize.saturating_sub(essentials.len()).min(6));
    push_footer_items_that_fit(
        &mut visible,
        &items,
        budget.saturating_sub(reserved + if essentials.is_empty() { 0 } else { 3 }),
    );
    push_footer_items_that_fit(&mut visible, &essentials, budget);
    visible.join(" | ")
}
