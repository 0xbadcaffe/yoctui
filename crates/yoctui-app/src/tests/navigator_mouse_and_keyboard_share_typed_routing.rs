//! Regression tests grouped around next_generation_navigator_mouse_and_keyboard_share_typed_routing.
use super::*;

#[test]
fn next_generation_navigator_mouse_and_keyboard_share_typed_routing() {
    let mut app = yoctui_model::App::new(16, 4096);
    let click_layers = MouseInput {
        kind: MouseKind::Down,
        column: 5,
        row: 7,
    };
    let select = mouse_action_for_app(click_layers, &app, 180, 40);
    assert_eq!(select, Some(Action::SelectNavigatorAt { index: 2 }));
    let _ = yoctui_model::update(&mut app, select.unwrap());
    assert_eq!(app.navigator_selection, 2);
    assert_eq!(app.focus, FocusTarget::Navigator);
    assert_eq!(
        mouse_action_for_app(click_layers, &app, 180, 40),
        Some(Action::ActivateNavigator)
    );

    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Left),
        Some(Action::CollapseNavigatorGroup)
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Right),
        Some(Action::ExpandNavigatorGroup)
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Char('h')),
        Some(Action::CollapseNavigatorGroup)
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Char('l')),
        Some(Action::ExpandNavigatorGroup)
    );

    let content_heading = MouseInput {
        kind: MouseKind::Down,
        column: 5,
        row: 6,
    };
    assert_eq!(
        mouse_action_for_app(content_heading, &app, 180, 40),
        Some(Action::ToggleNavigatorGroup { group: 1 })
    );
    let collapse = mouse_action_for_app(content_heading, &app, 180, 40).unwrap();
    let _ = yoctui_model::update(&mut app, collapse);
    assert!(!app.navigator_groups_expanded[1]);
    assert_eq!(app.navigator_selection, 2);
    assert_eq!(
        mouse_action_for_app(content_heading, &app, 180, 40),
        Some(Action::ToggleNavigatorGroup { group: 1 })
    );
    let reopen = mouse_action_for_app(content_heading, &app, 180, 40).unwrap();
    let _ = yoctui_model::update(&mut app, reopen);
    assert!(app.navigator_groups_expanded[1]);
}

#[test]
fn ux_responsive_mouse_regions_keyboard_scroll_and_minimum_are_exact() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::Recipes;

    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 100,
                row: 10,
            },
            &app,
            160,
            48,
        ),
        Some(Action::Focus(FocusTarget::Workspace))
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::ScrollUp,
                column: 30,
                row: 10,
            },
            &app,
            160,
            48,
        ),
        recipes_workspace_action(false, Input::Up)
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 8,
                row: 2,
            },
            &app,
            90,
            30,
        ),
        Some(Action::Focus(FocusTarget::Navigator))
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 32,
                row: 2,
            },
            &app,
            90,
            30,
        ),
        Some(Action::Focus(FocusTarget::Inspector))
    );
    for inert in [
        MouseInput {
            kind: MouseKind::Down,
            column: 30,
            row: 0,
        },
        MouseInput {
            kind: MouseKind::Down,
            column: 30,
            row: 29,
        },
    ] {
        assert_eq!(mouse_action_for_app(inert, &app, 90, 30), None);
    }
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 5,
                row: 5,
            },
            &app,
            79,
            23,
        ),
        None,
        "the below-minimum resize screen is inert"
    );
}

#[test]
fn compact_resource_meters_mouse_geometry_excludes_logs_and_meters() {
    let mut app = yoctui_model::App::new(100, 4096);
    for index in 0..100 {
        let id = TaskId(format!("task-{index:03}"));
        app.tasks.insert(
            id.clone(),
            TaskInfo::active(id, format!("recipe-{index}"), "do_compile".into()),
        );
    }
    app.task_progress_scroll = 50;
    for screen in [Screen::Dashboard, Screen::Tasks] {
        app.screen = screen;
        for (width, height) in [
            (101, 39),
            (89, 44),
            (76, 36),
            (78, 26),
            (80, 19),
            (86, 42),
            (116, 56),
        ] {
            let panels = task_workspace_panel_heights(&app, width, height);
            assert_eq!(panels.iter().sum::<u16>(), height);
            assert!(panels[3] >= 4);
            let area = MouseRect {
                x: 0,
                y: 2,
                width,
                height,
            };
            let summary = if screen == Screen::Dashboard { 4 } else { 2 };
            let count = panels[0] - summary - 3;
            let first = area.y + summary + 2;
            for offset in 0..count {
                assert_eq!(
                    task_row_click(
                        &app,
                        area,
                        MouseInput {
                            kind: MouseKind::Down,
                            column: 1,
                            row: first + offset
                        }
                    ),
                    Some(Action::ScrollBuildTasks {
                        delta: isize::try_from(offset).unwrap()
                            - isize::try_from(count / 2).unwrap()
                    })
                );
            }
            for row in [first - 1, first + count, area.y + height - 2] {
                assert_eq!(
                    task_row_click(
                        &app,
                        area,
                        MouseInput {
                            kind: MouseKind::Down,
                            column: 1,
                            row
                        }
                    ),
                    None
                );
            }
        }
    }
}

#[test]
fn next_generation_mouse_selects_exact_tasks_and_tabs() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::Tasks;
    for index in 0..3 {
        let id = TaskId(format!("task-{index}"));
        app.tasks.insert(
            id.clone(),
            TaskInfo::active(id, format!("recipe-{index}"), "do_compile".into()),
        );
    }
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 40,
                row: 7,
            },
            &app,
            160,
            48,
        ),
        Some(Action::ScrollBuildTasks { delta: 1 })
    );

    app.screen = Screen::Testing;
    let comparison = mouse_action_for_app(
        MouseInput {
            kind: MouseKind::Down,
            column: 48,
            row: 3,
        },
        &app,
        160,
        48,
    );
    assert_eq!(
        comparison,
        Some(Action::SelectTestView(TestWorkspaceView::Comparison))
    );
    let _ = yoctui_model::update(&mut app, comparison.unwrap());
    assert_eq!(app.test_view, TestWorkspaceView::Comparison);
    app.screen = Screen::Security;
    app.security.view = SecurityView::Cves;
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 32,
                row: 3,
            },
            &app,
            160,
            48,
        ),
        Some(Action::Security(SecurityAction::CycleView))
    );
}

#[test]
fn next_generation_mouse_traps_dialogs_and_resizes_exact_terminal_axis() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::TerminalSessions;
    let second = app
        .pane_layout
        .split(yoctui_model::PaneId(1), SplitAxis::Horizontal)
        .unwrap();
    for id in 1..=2 {
        app.daemon
            .pty_sessions
            .push(yoctui_model::ClientDaemonPtySummary {
                id,
                name: format!("shell-{id}"),
                lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
                viewers: 1,
            });
    }
    let select_first = mouse_action_for_app(
        MouseInput {
            kind: MouseKind::Down,
            column: 10,
            row: 10,
        },
        &app,
        120,
        30,
    );
    assert_eq!(
        select_first,
        Some(Action::SelectPtyPane {
            pane: yoctui_model::PaneId(1),
            index: 0,
        })
    );
    let _ = yoctui_model::update(&mut app, select_first.unwrap());
    assert_eq!(app.pane_layout.focused, yoctui_model::PaneId(1));

    let select_second = mouse_action_for_app(
        MouseInput {
            kind: MouseKind::Down,
            column: 90,
            row: 10,
        },
        &app,
        120,
        30,
    );
    assert_eq!(
        select_second,
        Some(Action::SelectPtyPane {
            pane: second,
            index: 1,
        })
    );
    let _ = yoctui_model::update(&mut app, select_second.unwrap());
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Drag,
                column: 90,
                row: 10,
            },
            &app,
            120,
            30,
        ),
        Some(Action::ResizeFocusedPane {
            delta_per_mille: 250,
        })
    );

    app.dialogs.push_back(yoctui_model::Dialog::ThemePicker {
        selection: 0,
        original_theme: app.theme,
        original_color_enabled: app.color_enabled,
        original_settings_dirty: app.settings_dirty,
    });
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::ScrollDown,
                column: 90,
                row: 10,
            },
            &app,
            120,
            30,
        ),
        Some(Action::SelectTheme { delta: 1 })
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Drag,
                column: 10,
                row: 10,
            },
            &app,
            120,
            30,
        ),
        None,
        "a modal traps drag input instead of leaking to terminal resizing"
    );
}

#[test]
fn keymap_function_key_catalog_maps_every_named_route() {
    let inputs = [
        Input::F1,
        Input::F2,
        Input::F3,
        Input::F4,
        Input::F5,
        Input::F6,
        Input::F7,
        Input::F8,
        Input::F9,
        Input::F10,
    ];
    let labels = [
        ("F1", "Help"),
        ("F2", "Tasks"),
        ("F3", "History"),
        ("F4", "Dashboard"),
        ("F5", "Logs"),
        ("F6", "Layers"),
        ("F7", "Recipes"),
        ("F8", "Images"),
        ("F9", "Commands"),
        ("F10", "Menu"),
    ];
    for ((input, shortcut), label) in inputs
        .into_iter()
        .zip(yoctui_model::FUNCTION_SHORTCUTS)
        .zip(labels)
    {
        assert_eq!((shortcut.key_label, shortcut.action_label), label);
        assert_eq!(
            key_action(input),
            Some(yoctui_model::function_shortcut_action(shortcut.key))
        );
    }
}

#[test]
fn compatibility_ui_actions_keep_task_shortcuts_authoritative() {
    let definitions = yoctui_model::compatibility_ui_workspace_action_definitions(
        yoctui_model::WorkspaceDestination::Tasks,
    );
    let shortcut = |id: &str| {
        definitions
            .iter()
            .find(|action| action.id == id)
            .map(|action| action.shortcut)
    };
    assert_eq!(shortcut("tasks.build"), Some("B"));
    assert_eq!(key_action(Input::Char('B')), Some(Action::OpenBuildOptions));
    assert_eq!(shortcut("tasks.cancel"), Some("c"));
    assert_eq!(key_action(Input::Char('c')), Some(Action::Cancel));
    assert_eq!(shortcut("tasks.logs"), Some("l"));
    assert_eq!(
        key_action(Input::Char('l')),
        Some(Action::Open(Screen::Logs))
    );
    assert_eq!(shortcut("tasks.history"), Some("h"));
    assert_eq!(
        key_action(Input::Char('h')),
        Some(Action::Open(Screen::BuildHistory))
    );
}

#[test]
fn ux_dashboard_operational_shortcuts_match_the_typed_action_catalog() {
    let definitions = yoctui_model::compatibility_ui_workspace_action_definitions(
        yoctui_model::WorkspaceDestination::Dashboard,
    );
    let shortcut = |id: &str| {
        definitions
            .iter()
            .find(|action| action.id == id)
            .map(|action| action.shortcut)
    };
    for (id, expected, input, action) in [
        (
            "dashboard.errors",
            "e",
            Input::Char('e'),
            Action::Open(Screen::Errors),
        ),
        (
            "dashboard.environment",
            "E",
            Input::Char('E'),
            Action::Open(Screen::BuildEnvironment),
        ),
        (
            "dashboard.maintenance",
            "M",
            Input::Char('M'),
            Action::Open(Screen::Maintenance),
        ),
    ] {
        assert_eq!(shortcut(id), Some(expected));
        assert_eq!(key_action(input), Some(action));
    }
    assert_eq!(shortcut("dashboard.tasks"), Some("F2"));
    assert_eq!(key_action(Input::F2), Some(Action::Open(Screen::Tasks)));
    assert_eq!(shortcut("dashboard.history"), Some("F3"));
    assert_eq!(
        key_action(Input::F3),
        Some(Action::Open(Screen::BuildHistory))
    );
    assert_eq!(shortcut("dashboard.artifacts"), Some("F8"));
    assert_eq!(key_action(Input::F8), Some(Action::Open(Screen::Images)));
}

#[test]
fn ux_command_center_routes_to_existing_typed_workspaces_and_terminal_command() {
    let mut app = yoctui_model::App::new(16, 4_096);
    for (input, screen) in [
        (Input::F2, Screen::Tasks),
        (Input::F3, Screen::BuildHistory),
        (Input::F8, Screen::Images),
    ] {
        assert_eq!(key_action(input), Some(Action::Open(screen)));
    }
    assert_eq!(
        dashboard_workspace_action(Input::Char('f')),
        Some(Action::OpenRawFavorites)
    );
    let _ = yoctui_model::update(&mut app, Action::OpenRawFavorites);
    assert_eq!(app.screen, Screen::RawMode);
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Favorites);
    assert_eq!(
        dashboard_workspace_action(Input::Char('t')),
        Some(Action::Open(Screen::TerminalSessions))
    );
    assert_eq!(
        context_menu_activation_input("dashboard.errors"),
        Some(Input::Char('e'))
    );
    assert_eq!(
        context_menu_activation_input("dashboard.artifacts"),
        Some(Input::F8)
    );
    assert_eq!(
        context_menu_activation_input("dashboard.favorites"),
        Some(Input::Char('f'))
    );
    assert_eq!(
        context_menu_activation_input("dashboard.terminals"),
        Some(Input::Char('t'))
    );
}

#[test]
fn persistent_pane_focus_preserves_global_quit() {
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Char('q')),
        Some(Action::Quit)
    );
    assert_eq!(
        focus_action(FocusTarget::Workspace, Input::Char('q')),
        Some(Action::Quit)
    );
    assert_eq!(
        focus_action(FocusTarget::Workspace, Input::CtrlC),
        Some(Action::Quit)
    );
    assert_eq!(
        focus_action(FocusTarget::Inspector, Input::CtrlC),
        Some(Action::Quit)
    );
}
#[test]
fn maps_log_controls() {
    assert_eq!(key_action(Input::Char('f')), Some(Action::ToggleLogFollow));
    assert_eq!(key_action(Input::Char('w')), Some(Action::ToggleLogWrap));
    assert_eq!(key_action(Input::Up), Some(Action::ScrollLogs { delta: 1 }));
}
#[test]
fn log_workspace_maps_selection_search_filters_and_selected_actions() {
    assert_eq!(
        logs_action(false, Input::Up),
        Some(Action::ScrollLogs { delta: 1 })
    );
    assert_eq!(
        logs_action(false, Input::Char('B')),
        Some(Action::CycleLogBuildFilter)
    );
    assert_eq!(
        logs_action(false, Input::Char('o')),
        Some(Action::OpenSelectedLogSource)
    );
    assert_eq!(
        logs_action(false, Input::Char('C')),
        Some(Action::CopySelectedLog)
    );
    assert_eq!(
        logs_action(true, Input::Char('x')),
        Some(Action::AppendLogQuery('x'))
    );
    assert_eq!(logs_action(true, Input::Esc), Some(Action::FinishLogSearch));
}
#[test]
fn ux_logs_keyboard_routes_source_time_bookmarks_and_bounded_export() {
    for (input, expected) in [
        (Input::Char('S'), Action::CycleLogSourceFilter),
        (Input::Char('I'), Action::CycleLogTimeRange),
        (Input::Char('m'), Action::ToggleSelectedLogBookmark),
        (Input::Char(']'), Action::NextLogBookmark),
        (Input::Char('['), Action::PreviousLogBookmark),
        (Input::Char('E'), Action::ExportFilteredLogs),
    ] {
        assert_eq!(logs_action(false, input), Some(expected));
    }
    assert_eq!(
        logs_action(true, Input::Char('m')),
        Some(Action::AppendLogQuery('m'))
    );
}
#[test]
fn ux_internal_log_keyboard_routes_only_the_separate_diagnostic_authority() {
    for (input, expected) in [
        (Input::Char('v'), Action::CycleLogWorkspaceView),
        (Input::Char('f'), Action::ToggleInternalLogFollow),
        (Input::Char('s'), Action::CycleInternalLogLevelFilter),
        (Input::Char('T'), Action::CycleInternalLogTargetFilter),
        (Input::Char('/'), Action::BeginInternalLogSearch),
        (Input::Char('c'), Action::ClearInternalLogs),
        (Input::Char('E'), Action::ExportInternalLogs),
    ] {
        assert_eq!(internal_logs_action(false, input), Some(expected));
    }
    assert_eq!(
        internal_logs_action(true, Input::Char('x')),
        Some(Action::AppendInternalLogQuery('x'))
    );
    assert_eq!(internal_logs_action(false, Input::Char('B')), None);
}
#[test]
fn search_clear_shortcut_maps_to_every_typed_domain() {
    for searching in [false, true] {
        assert_eq!(
            compatibility_ui_inspector_action(searching, Input::CtrlU),
            Some(Action::ClearCompatibilityQuery)
        );
        assert_eq!(
            logs_action(searching, Input::CtrlU),
            Some(Action::ClearLogQuery)
        );
        assert_eq!(
            package_workspace_action(searching, Input::CtrlU),
            Some(Action::ClearPackageQuery)
        );
        assert_eq!(
            images_workspace_action(searching, Input::CtrlU),
            Some(Action::ClearImageArtifactQuery)
        );
        assert_eq!(
            sdk_workspace_action(searching, Input::CtrlU),
            Some(Action::ClearSdkArtifactQuery)
        );
        assert_eq!(
            test_results_workspace_action(searching, false, Input::CtrlU),
            Some(Action::ClearTestResultQuery)
        );
        assert_eq!(
            layer_tree_action(searching, Input::CtrlU),
            Some(Action::ClearMetadataQuery)
        );
        assert_eq!(
            recipes_workspace_action(searching, Input::CtrlU),
            Some(Action::ClearMetadataQuery)
        );
        assert_eq!(
            config_workspace_action(searching, Input::CtrlU),
            Some(Action::ClearMetadataQuery)
        );
        assert_eq!(
            security_workspace_action(SecurityView::Cves, false, searching, Input::CtrlU),
            Some(Action::Security(SecurityAction::ClearQuery))
        );
        assert_eq!(
            qa_workspace_action(QaView::RecipeKernel, false, searching, Input::CtrlU),
            Some(Action::Qa(QaAction::ClearQuery))
        );
    }
}
#[test]
fn enter_activates_contextual_notification() {
    assert_eq!(key_action(Input::Enter), Some(Action::ActivateNotification));
}

#[test]
fn visible_guidance_popup_owns_its_advertised_enter_and_escape_controls() {
    let mut app = yoctui_model::App::new(10, 1_000);
    app.screen = Screen::Images;
    app.notification = Some("Select an image first with i.".into());

    assert_eq!(
        notification_popup_action(&app, Input::Esc),
        Some(Action::DismissNotification)
    );
    assert_eq!(
        notification_popup_action(&app, Input::Enter),
        Some(Action::ActivateNotification)
    );
    assert_eq!(notification_popup_action(&app, Input::Down), None);

    app.notification = Some("Package inventory refreshed.".into());
    assert_eq!(notification_popup_action(&app, Input::Esc), None);
}
#[test]
fn maps_severity_filter_control() {
    assert_eq!(key_action(Input::Char('s')), Some(Action::CycleLogSeverity));
}
#[test]
fn error_workspace_maps_selection_log_jump_and_source_open() {
    assert_eq!(
        errors_action(Input::Up),
        Some(Action::SelectError { delta: -1 })
    );
    assert_eq!(
        errors_action(Input::Enter),
        Some(Action::JumpToSelectedError)
    );
    assert_eq!(
        errors_action(Input::Char('o')),
        Some(Action::OpenSelectedErrorSource)
    );
}
#[test]
fn layer_tree_maps_lazy_navigation_hidden_refresh_and_inspector_modes() {
    assert_eq!(
        layer_tree_action(false, Input::Right),
        Some(Action::LayerBrowserExpand)
    );
    assert_eq!(
        layer_tree_action(false, Input::Left),
        Some(Action::LayerBrowserUp)
    );
    assert_eq!(
        layer_tree_action(false, Input::Char('.')),
        Some(Action::ToggleLayerBrowserHidden)
    );
    assert_eq!(
        layer_tree_action(false, Input::Char('i')),
        Some(Action::SetLayerInspectorMode(LayerInspectorMode::Metadata))
    );
    assert_eq!(
        layer_tree_action(false, Input::PageUp),
        Some(Action::SelectLayerBrowserEntry { delta: -10 })
    );
    assert_eq!(
        layer_tree_action(false, Input::PageDown),
        Some(Action::SelectLayerBrowserEntry { delta: 10 })
    );
    assert_eq!(
        layer_tree_action(true, Input::Char('b')),
        Some(Action::AppendMetadataQuery('b'))
    );
}

#[test]
fn layers_workspace_owns_horizontal_hierarchy_keys_before_pane_focus() {
    let mut app = yoctui_model::App::new(10, 1_000);
    app.screen = Screen::Layers;
    app.focus = FocusTarget::Workspace;
    assert_eq!(focus_action_for_app(&app, Input::Right), None);
    assert_eq!(
        focus_action_for_app(&app, Input::Left),
        Some(Action::CycleFocus { backwards: true })
    );

    let _ = yoctui_model::update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "meta-test".into(),
            root: "/tmp/meta-test".into(),
            directory: "/tmp/meta-test".into(),
            entries: Vec::new(),
        },
    );
    assert_eq!(focus_action_for_app(&app, Input::Right), None);
    assert_eq!(focus_action_for_app(&app, Input::Left), None);
    assert_eq!(
        focus_action_for_app(&app, Input::Tab),
        Some(Action::CycleFocus { backwards: false })
    );
}

#[test]
fn navigator_arrows_expand_and_collapse_groups_without_changing_panes() {
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Right),
        Some(Action::ExpandNavigatorGroup)
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Left),
        Some(Action::CollapseNavigatorGroup)
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Enter),
        Some(Action::ActivateNavigator)
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Tab),
        Some(Action::CycleFocus { backwards: false })
    );
}

#[test]
fn ux_list_tree_keyboard_routes_expansion_pages_edges_and_search_without_widget_state() {
    assert_eq!(
        layer_tree_action(false, Input::Right),
        Some(Action::LayerBrowserExpand)
    );
    assert_eq!(
        layer_tree_action(false, Input::PageDown),
        Some(Action::SelectLayerBrowserEntry {
            delta: DEFAULT_COLLECTION_PAGE_ROWS
        })
    );
    assert_eq!(
        layer_tree_action(false, Input::End),
        Some(Action::SelectLayerBrowserEntry { delta: isize::MAX })
    );
    assert_eq!(
        layer_tree_action(false, Input::Char('/')),
        Some(Action::BeginMetadataSearch)
    );
}
#[test]
fn recipes_workspace_maps_search_selection_detail_and_dependencies() {
    assert_eq!(
        recipes_workspace_action(false, Input::Down),
        Some(Action::SelectRecipe { delta: 1 })
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Enter),
        Some(Action::BeginSelectedRecipeMetadata)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('A')),
        Some(Action::BeginSelectedRecipeDependencies)
    );
    assert_eq!(
        recipes_workspace_action(true, Input::Char('b')),
        Some(Action::AppendMetadataQuery('b'))
    );
    assert_eq!(
        recipes_workspace_action(true, Input::Backspace),
        Some(Action::BackspaceMetadataQuery)
    );
}

#[test]
fn concept_chrome_and_pane_mouse_boundaries_agree_across_resize() {
    let mut app = yoctui_model::App::new(16, 4096);
    for screen in [
        Screen::Tasks,
        Screen::Errors,
        Screen::Images,
        Screen::TerminalSessions,
        Screen::Recipes,
    ] {
        app.screen = screen;
        for (width, height) in [(150, 50), (160, 50), (180, 55), (200, 60), (160, 48)] {
            let [header, footer] = workbench_chrome_heights(&app, width, height);
            let widths = workbench_pane_widths(&app, width, height);
            assert_eq!(widths.iter().sum::<u16>(), width);
            let shell = super::super::mouse::workbench_shell(&app, width, height).unwrap();
            assert_eq!(shell.y, header);
            assert_eq!(shell.bottom(), height - footer);
            for (column, target) in [
                (0, FocusTarget::Navigator),
                (widths[0], FocusTarget::Workspace),
                (
                    width - 1,
                    if widths[2] == 0 {
                        FocusTarget::Workspace
                    } else {
                        FocusTarget::Inspector
                    },
                ),
            ] {
                let mouse = MouseInput {
                    kind: MouseKind::Down,
                    column,
                    row: header,
                };
                let region =
                    super::super::mouse::workbench_mouse_region(mouse, &app, shell).unwrap();
                assert_eq!(region.target, target, "{screen:?} {width}x{height}");
            }
            for row in [header - 1, height - footer] {
                let mouse = MouseInput {
                    kind: MouseKind::Down,
                    column: width / 2,
                    row,
                };
                assert!(super::super::mouse::workbench_mouse_region(mouse, &app, shell).is_none());
            }
        }
    }
}
