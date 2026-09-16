//! Footer.
use super::*;

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

pub(crate) fn footer_shortcuts(app: &App) -> String {
    if app.screen == Screen::Signatures {
        return with_compatibility_footer(
            app,
            WorkspaceDestination::Signatures,
            with_focus_shortcuts(
                app,
                "↑/↓ select | 1/2 sides | c compare | r refresh | e provider | Esc back/cancel",
            ),
        );
    }
    if app.focus == FocusTarget::Navigator {
        return with_compatibility_footer(
            app,
            app.navigator_compatibility_destination(),
            with_focus_shortcuts(
                app,
                "↑/↓ select | h/l groups | Enter open | Ctrl+B prefix | q quit",
            ),
        );
    }
    if app.focus == FocusTarget::Inspector {
        return with_compatibility_footer(
            app,
            yoctui_model::workspace_screen_destination(app.screen),
            with_focus_shortcuts(app, "↑/↓ scroll inspector | q quit"),
        );
    }
    if app.layer_browser.is_some() {
        return with_compatibility_footer(
            app,
            WorkspaceDestination::Layers,
            with_focus_shortcuts(
                app,
                "↑/↓ select | PgUp/PgDn page | →/l expand or focus preview | preview arrows scroll | ← tree | e editor | i info | r refresh | . hidden | / search",
            ),
        );
    }
    let shortcuts = match app.screen {
        Screen::Dashboard => {
            "B build | f favorites | t terminals | F2 Tasks | e errors | Ctrl+B prefix | F8 artifacts | l logs | F3 work | E environment | M sstate | Ctrl+P commands | Tab focus | c cancel | ? help | q quit"
        }
        Screen::Insights => "1-8 view | [/] previous/next | Tab focus | Esc dashboard",
        Screen::Tasks => {
            "↑/↓ select | f state | F field | / edit filter | d duration | c cancel | Tab focus"
        }
        Screen::BuildHistory => {
            "↑/↓ select | Enter details | ←/→ view | PgUp/PgDn scroll | r refresh | l live/saved | Esc back"
        }
        Screen::Dependencies => {
            "↑/↓ or j/k select | Enter recipe | o provider | L task log | r refresh | Tab focus | Esc dashboard"
        }
        Screen::Signatures => {
            "↑/↓ select | 1/2 sides | c compare | r refresh | e provider | Esc back/cancel"
        }
        Screen::LayerRelationships => "Esc dashboard | y layers | ? help | q quit",
        Screen::Recipes => {
            "↑/↓ select | [/] preview scroll | e provider | o logs | p patches | b/f tasks | v devshell | s workspace shell | E edit-recipe | V CVE | X SPDX | d modify | u update | F finish | P deploy | D reset | / search"
        }
        Screen::Packages => {
            "↑/↓ select | Enter detail | / search | R refresh | D dep kind | [/] dep | d follow | u back | o recipe | e provider | c cancel"
        }
        Screen::Images => match app.images_view {
            ImagesView::Artifacts => {
                "↑/↓ select | Enter/p rootfs | Tab view | Q QEMU | W create Wic | D write device | x cancel | [/] output | O open output | / search | R refresh | b build | o artifact | m manifest | l license | s SPDX | w Wic"
            }
            ImagesView::RootfsPackages => {
                "h/l group | j/k package | PgUp/PgDn page | r refresh | Tab filesystem | Shift+Tab artifacts"
            }
            ImagesView::RootfsFilesystem => {
                "j/k select path | Enter/→ explore actual IMAGE_ROOTFS | r refresh | Tab view"
            }
            ImagesView::SystemdServices => {
                "j/k service | e edit unit | Enter/→ explore rootfs | r refresh | Tab view"
            }
            ImagesView::SystemDbus => {
                "j/k bus name | e edit activation file | Enter/→ explore rootfs | r refresh | Tab view"
            }
            ImagesView::UdevRules => {
                "↑/↓ rule | PgUp/PgDn | [/] preview | Enter explore rootfs | r refresh | Tab view"
            }
        },
        Screen::Kernel => {
            "Tab view | ↑/↓ select | m menuconfig | Enter view | e edit | o explore | c compile DTS | d decompile DTB | r refresh"
        }
        Screen::Firmware => {
            "Tab view | ↑/↓ select | m menuconfig | Enter view | e edit | o explore | c compile DTS | d decompile DTB | r refresh"
        }
        Screen::Sdk => {
            "↑/↓ select | i image | s standard | E extensible | t testsdk | T testsdkext | R refresh | P publish | n native | o open | c cancel"
        }
        Screen::Testing => {
            "Tab view | ↑/↓ select | Enter open | r run | i image | / search | I import | R refresh | c compare | J JUnit | o result | l log | x cancel"
        }
        Screen::Security => {
            "Tab view | ↑/↓ select | s scope | i image | / search | f status | V CVE check | M map | X SBOM | I import | R refresh | Enter details | o report | e recipe | v advisory | c cancel"
        }
        Screen::Qa => {
            "Tab view | ↑/↓ select | s scope | / search | f status | r run | I import | R refresh | Enter details | o report | e provider | l source | c cancel"
        }
        Screen::RawMode => {
            if app.raw_mode.view == yoctui_model::RawModeView::Execution {
                "↑/↓ Scroll | ←/→ Horizontal | 1/2 Stream | f Follow | / Search | c Cancel | d Detach | r Reattach | Esc Back"
            } else {
                "←/→ Pane | ↑/↓ Select | Enter Open | / Search | f Favorite | H History | Tab Focus | F1 Help | F10 Menu | q Quit"
            }
        }
        Screen::TerminalSessions => {
            "Ctrl+B prefix | [ copy | / search | r rename | O release | K confirmed kill | z zoom | paste review | o take (viewer)"
        }
        Screen::Layers => {
            "↑/↓ select | Enter browse | i image | R relationships | e in-TUI edit | o external editor | / search | Esc dashboard | ? help | q quit"
        }
        Screen::Configuration => {
            "↑/↓ select | Enter inspect | s scope | c compare | C copy effective | U copy unexpanded | o source | E edit | / search | x BBMASK | Esc dashboard | ? help | q quit"
        }
        Screen::Bbmask => {
            "e edit BBMASK | Enter preview/confirm | Esc cancel/dashboard | v configuration | ? help | q quit"
        }
        Screen::Maintenance => match app.maintenance.view {
            MaintenanceView::Sstate => {
                "[ ] view | r refresh | Enter inspect | x cancel | o open evidence | S signatures | c check | d cleanup"
            }
            MaintenanceView::Services => {
                "[ ] view | r refresh | Enter inspect | x cancel | o open evidence | S signatures | e PR export | m PR import"
            }
            MaintenanceView::Release => {
                "[ ] view | r refresh | Enter inspect | x cancel | o open evidence | S signatures | l locked cache | h compare | a archive"
            }
            MaintenanceView::Integrations => {
                "[ ] view | r refresh | Enter inspect | x cancel | o open evidence | S signatures | detection/inspection only"
            }
        },
        Screen::Compatibility => {
            "↑/↓ or j/k select | 1 All | 2 Available | 3 Limited | 4 Unavailable | 5 Attention | / search | Tab focus"
        }
        Screen::Logs => match app.log_workspace_view {
            LogWorkspaceView::BitBake if app.logs.follow => {
                "v diagnostics | ↑/↓ select | ←/→ horizontal | f pause | w wrap | s/R/T/B/S/I filters | / search | m bookmark | C copy | E export"
            }
            LogWorkspaceView::BitBake => {
                "v diagnostics | ↑/↓ select | ←/→ horizontal | f follow | w wrap | s/R/T/B/S/I filters | / search | m bookmark | C copy | E export"
            }
            LogWorkspaceView::Yoctui if app.internal_logs.follow => {
                "v BitBake | ↑/↓ select | f pause | s level | T target | / search | E export | c clear"
            }
            LogWorkspaceView::Yoctui => {
                "v BitBake | ↑/↓ select | f follow | s level | T target | / search | E export | c clear"
            }
        },
        Screen::Errors => {
            "↑/↓ select | Enter matching log | o source | s severity filter | f pause/follow | B rebuild options"
        }
        Screen::Help => "Esc dashboard | q quit",
        Screen::Settings => {
            "↑/↓ select | ←/→ change | r retry save | Ctrl+P commands | Tab focus | q quit"
        }
        Screen::BuildEnvironment => {
            "e configure | b browse paths | A advanced | V verify | Tab focus | q quit"
        }
    };
    with_compatibility_footer(
        app,
        yoctui_model::workspace_screen_destination(app.screen),
        with_focus_shortcuts(app, shortcuts),
    )
}

pub(crate) fn responsive_footer_shortcuts(app: &App, width: u16) -> String {
    if app.screen == Screen::Images && width <= 129 {
        match app.images_view {
            ImagesView::Artifacts => {
                "↑↓ select | R refresh | Enter/p rootfs | Tab view | Q QEMU | W Wic | D write"
                    .into()
            }
            ImagesView::RootfsPackages => {
                "h/l group | j/k package | PgUp/PgDn | r refresh | Tab view".into()
            }
            ImagesView::RootfsFilesystem => "j/k path | PgUp/PgDn | r refresh | Tab view".into(),
            ImagesView::SystemdServices => "j/k service | e edit | Enter explore | Tab view".into(),
            ImagesView::SystemDbus => "j/k bus | e edit | Enter explore | Tab view".into(),
            ImagesView::UdevRules => {
                "↑↓ rule | [/] preview | Enter explore | r refresh | Tab view".into()
            }
        }
    } else if app.screen == Screen::Sdk && width <= 90 {
        "↑↓ i:image s/E:SDK t/T:test R:scan P:publish n:native o:open c:cancel".into()
    } else if app.screen == Screen::Testing && width <= 90 {
        "Tab:view ↑↓ Enter r:run i:image /:find I/R:results c:compare J:JUnit o/l:open x:cancel"
            .into()
    } else if app.screen == Screen::Security && width <= 90 {
        "Tab:view ↑↓ s:scope i:image /:find f:status V:check M:map X:SBOM I/R:data Enter o/e/v:open c:cancel"
            .into()
    } else if app.screen == Screen::Qa && width <= 90 {
        "Tab:view ↑↓ s:scope /:find f:status r:run I/R:data Enter o/e/l:open c:cancel".into()
    } else {
        let shortcuts = footer_shortcuts(app);
        if width < 100 && pane_focus_shortcuts(app).is_some() {
            shortcuts
                .split(" | ")
                .skip(3)
                .collect::<Vec<_>>()
                .join(" | ")
        } else {
            shortcuts
        }
    }
}

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
