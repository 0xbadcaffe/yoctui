//! Regression tests grouped around navigator_workbench_order_keeps_build_and_validation_groups_contiguous.
use super::*;

#[test]
fn navigator_workbench_order_keeps_build_and_validation_groups_contiguous() {
    assert_eq!(
        NAVIGATOR_SCREENS,
        [
            Screen::Dashboard,
            Screen::Insights,
            Screen::Layers,
            Screen::Recipes,
            Screen::Packages,
            Screen::Images,
            Screen::Kernel,
            Screen::Firmware,
            Screen::Sdk,
            Screen::Tasks,
            Screen::Logs,
            Screen::Errors,
            Screen::Configuration,
            Screen::Dependencies,
            Screen::Testing,
            Screen::Security,
            Screen::Qa,
            Screen::RawMode,
            Screen::TerminalSessions,
            Screen::Recipes,
            Screen::Images,
            Screen::Maintenance,
            Screen::BuildEnvironment,
            Screen::Compatibility,
            Screen::Settings,
        ]
    );
}

#[test]
fn navigator_groups_collapse_without_exposing_hidden_destinations() {
    let mut app = App::new(10, 1_000);
    app.navigator_selection = 9;
    assert_eq!(app.navigator_group_index(), 2);
    assert_eq!(app.navigator_visual_row(), 12);

    let _ = update(&mut app, Action::CollapseNavigatorGroup);
    assert!(!app.navigator_groups_expanded[2]);
    assert_eq!(app.navigator_visual_row(), 11);
    assert_eq!(app.navigator_group_at_visual_row(11), Some(2));
    assert_eq!(app.navigator_selection_at_visual_row(11), None);

    let _ = update(&mut app, Action::SelectNavigator { delta: 1 });
    assert_eq!(app.navigator_selection, 14);
    let _ = update(&mut app, Action::SelectNavigatorAt { index: 10 });
    assert_eq!(
        app.navigator_selection, 14,
        "hidden rows cannot be selected"
    );

    app.navigator_selection = 9;
    let _ = update(&mut app, Action::ActivateNavigator);
    assert!(app.navigator_groups_expanded[2]);
    assert_eq!(app.screen, Screen::Dashboard, "expansion does not navigate");
}

#[test]
fn collapsed_navigator_root_remains_selected_and_can_reopen() {
    let mut app = App::new(10, 1_000);
    app.navigator_selection = NAVIGATOR_GROUPS[4].start + 3;

    let _ = update(&mut app, Action::ToggleNavigatorGroup { group: 1 });
    assert!(!app.navigator_groups_expanded[1]);
    assert_eq!(app.navigator_selection, NAVIGATOR_GROUPS[1].start);
    assert_eq!(app.navigator_group_index(), 1);

    let _ = update(&mut app, Action::ExpandNavigatorGroup);
    assert!(app.navigator_groups_expanded[1]);

    let _ = update(&mut app, Action::ToggleNavigatorGroup { group: 1 });
    assert!(!app.navigator_groups_expanded[1]);
    let _ = update(&mut app, Action::ActivateNavigator);
    assert!(app.navigator_groups_expanded[1]);
    assert_eq!(app.screen, Screen::Dashboard, "reopening does not navigate");
}

#[test]
fn collapsed_navigator_root_can_be_reselected_and_reopened() {
    let mut app = App::new(10, 1_000);
    app.navigator_selection = NAVIGATOR_GROUPS[2].start;
    let _ = update(&mut app, Action::CollapseNavigatorGroup);

    let _ = update(&mut app, Action::SelectNavigator { delta: 1 });
    assert_eq!(app.navigator_group_index(), 3);
    let _ = update(&mut app, Action::SelectNavigator { delta: -1 });
    assert_eq!(app.navigator_selection, NAVIGATOR_GROUPS[2].start);
    assert_eq!(app.navigator_visual_row(), 11);

    let _ = update(&mut app, Action::ExpandNavigatorGroup);
    assert!(app.navigator_groups_expanded[2]);
    assert_eq!(app.screen, Screen::Dashboard, "reopening must not navigate");
}

#[test]
fn responsive_pane_focus_cycle_cannot_escape_modal_focus() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Dialog;
    let _ = update(&mut app, Action::CycleFocus { backwards: false });
    assert_eq!(app.focus, FocusTarget::Dialog);

    app.focus = FocusTarget::CommandPalette;
    let _ = update(&mut app, Action::CycleFocus { backwards: true });
    assert_eq!(app.focus, FocusTarget::CommandPalette);

    app.focus = FocusTarget::Workspace;
    let _ = update(&mut app, Action::CycleFocus { backwards: false });
    assert_eq!(app.focus, FocusTarget::Inspector);
    let _ = update(&mut app, Action::CycleFocus { backwards: false });
    assert_eq!(app.focus, FocusTarget::Navigator);
}
#[test]
fn focus_restores_exact_pane_after_nested_dialog_transitions() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Inspector;

    let _ = update(&mut app, Action::OpenBuildOptions);
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert_eq!(app.focus_return, Some(FocusTarget::Inspector));

    let _ = update(&mut app, Action::BeginBuildTargetEdit);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::BuildTarget { .. })
    ));
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert_eq!(app.focus_return, Some(FocusTarget::Inspector));

    let _ = update(&mut app, Action::CancelBuildTargetEdit);
    assert_eq!(app.focus, FocusTarget::Inspector);
    assert_eq!(app.focus_return, None);
}
#[test]
fn focus_command_palette_restores_or_transitions_without_leaking_input() {
    let mut app = App::new(10, 1_000);
    app.focus = FocusTarget::Navigator;
    app.workspace.build_dir = Some(PathBuf::from("/build"));

    let _ = update(&mut app, Action::OpenCommandPalette);
    assert_eq!(app.focus, FocusTarget::CommandPalette);
    assert_eq!(app.focus_return, Some(FocusTarget::Navigator));
    for character in "Choose theme".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }

    let original_screen = app.screen;
    let original_selection = app.navigator_selection;
    let _ = update(&mut app, Action::Open(Screen::Logs));
    let _ = update(&mut app, Action::SelectNavigator { delta: 1 });
    let _ = update(&mut app, Action::Focus(FocusTarget::Workspace));
    assert_eq!(app.screen, original_screen);
    assert_eq!(app.navigator_selection, original_selection);
    assert_eq!(app.focus, FocusTarget::CommandPalette);

    let _ = update(&mut app, Action::ActivateCommandPalette);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::ThemePicker { .. })
    ));
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert_eq!(app.focus_return, Some(FocusTarget::Navigator));
    let _ = update(&mut app, Action::CloseThemePicker);
    assert_eq!(app.focus, FocusTarget::Navigator);
}
#[test]
fn command_palette_search_is_case_insensitive_and_selection_is_bounded() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenCommandPalette);
    for character in "PROVENANCE".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }
    let commands = app.filtered_command_palette_commands();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, CommandId::OpenConfiguration);

    let _ = update(&mut app, Action::SelectCommandPalette { delta: 99 });
    assert_eq!(app.command_palette_selection, 0);
    for _ in 0.."PROVENANCE".len() {
        let _ = update(&mut app, Action::BackspaceCommandPaletteQuery);
    }
    assert!(app.command_palette_query.is_empty());
    assert!(app.filtered_command_palette_commands().len() > 6);
}
#[test]
fn global_search_uses_case_insensitive_regex_and_reports_invalid_patterns() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenGlobalSearch);
    assert_eq!(
        app.command_palette_mode,
        CommandPaletteMode::GlobalRegexSearch
    );
    for character in "^open (packages|sdk)$".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }
    let ids = app
        .filtered_command_palette_commands()
        .into_iter()
        .map(|command| command.id)
        .collect::<Vec<_>>();
    assert_eq!(ids, [CommandId::OpenPackages, CommandId::OpenSdk]);
    assert_eq!(app.command_palette_regex_error(), None);

    let _ = update(&mut app, Action::ClearCommandPaletteQuery);
    let _ = update(&mut app, Action::AppendCommandPaletteQuery('['));
    assert!(app.filtered_command_palette_commands().is_empty());
    assert!(app.command_palette_regex_error().is_some());

    let _ = update(&mut app, Action::CloseCommandPalette);
    let _ = update(&mut app, Action::OpenCommandPalette);
    assert_eq!(app.command_palette_mode, CommandPaletteMode::Commands);
    assert_eq!(app.command_palette_regex_error(), None);
}
#[test]
fn global_search_query_is_bounded() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenGlobalSearch);
    for _ in 0..MAX_COMMAND_PALETTE_QUERY_CHARS + 20 {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery('x'));
    }
    assert_eq!(
        app.command_palette_query.chars().count(),
        MAX_COMMAND_PALETTE_QUERY_CHARS
    );
}
#[test]
fn global_content_search_rejects_stale_results_and_opens_selected_file() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenGlobalSearch);
    for character in "service token".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }
    let _ = update(&mut app, Action::BeginGlobalContentSearch);
    let generation = app.global_search_generation;
    let query = app.command_palette_query.clone();
    let hit = GlobalSearchHit {
        kind: GlobalSearchContentKind::ImageRootfs,
        path:
            "/build/tmp/work/machine/core-image-demo/1.0/rootfs/usr/lib/systemd/system/demo.service"
                .into(),
        line: 3,
        column: 13,
        preview: "Description=service token".into(),
        image: Some("core-image-demo".into()),
    };
    let _ = update(
        &mut app,
        Action::GlobalContentSearchLoaded {
            generation: generation.saturating_sub(1),
            query: query.clone(),
            hits: vec![hit.clone()],
            truncated: false,
            searched_scopes: vec!["stale".into()],
        },
    );
    assert!(app.global_search_content.hits().is_empty());
    let _ = update(
        &mut app,
        Action::GlobalContentSearchLoaded {
            generation,
            query,
            hits: vec![hit.clone()],
            truncated: false,
            searched_scopes: vec!["generated image".into()],
        },
    );
    assert_eq!(app.global_search_content.hits(), std::slice::from_ref(&hit));
    app.command_palette_selection = app.filtered_command_palette_commands().len();
    assert_eq!(
        update(&mut app, Action::ActivateCommandPalette),
        Some(Effect::OpenInEditor(hit.path))
    );
    assert!(!app.command_palette_open);
}
#[test]
fn command_palette_empty_and_disabled_activation_are_inert() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenCommandPalette);
    let original = app.clone();
    assert_eq!(update(&mut app, Action::ActivateCommandPalette), None);
    assert_eq!(app, original, "disabled Build image must remain open");

    for character in "no such command".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }
    let no_results = app.clone();
    assert!(app.filtered_command_palette_commands().is_empty());
    assert_eq!(update(&mut app, Action::ActivateCommandPalette), None);
    assert_eq!(app, no_results);
}
#[test]
fn command_palette_available_entry_dispatches_existing_typed_action() {
    let mut app = App::new(10, 1_000);
    app.focus = FocusTarget::Inspector;
    let _ = update(&mut app, Action::OpenCommandPalette);
    for character in "Open Settings".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }

    assert_eq!(update(&mut app, Action::ActivateCommandPalette), None);
    assert_eq!(app.screen, Screen::Settings);
    assert!(!app.command_palette_open);
    assert_eq!(app.focus, FocusTarget::Workspace);
}
#[test]
fn theme_command_palette_entry_opens_named_picker() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Inspector;
    let _ = update(&mut app, Action::OpenCommandPalette);
    for character in "Choose theme".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }

    assert_eq!(update(&mut app, Action::ActivateCommandPalette), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::ThemePicker { .. })
    ));
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert_eq!(app.focus_return, Some(FocusTarget::Inspector));
}
#[test]
fn focus_async_dialog_waits_behind_palette_then_restores() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Inspector;
    let _ = update(&mut app, Action::OpenCommandPalette);
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert!(matches!(app.active_dialog(), Some(Dialog::BuildCompletion)));
    assert_eq!(app.focus, FocusTarget::CommandPalette);

    let _ = update(&mut app, Action::CloseCommandPalette);
    assert_eq!(app.focus, FocusTarget::Dialog);
    let _ = update(&mut app, Action::DismissBuildCompletion);
    assert_eq!(app.focus, FocusTarget::Inspector);
}
#[test]
fn dialog_completion_queues_behind_active_dialog_and_restores_focus_after_both_close() {
    let mut app = App::new(10, 1_000);
    app.focus = FocusTarget::Navigator;
    let _ = update(&mut app, Action::OpenBuildOptions);
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );

    assert_eq!(
        app.dialogs.iter().collect::<Vec<_>>(),
        vec![&Dialog::BuildOptions, &Dialog::BuildCompletion]
    );
    assert_eq!(app.focus, FocusTarget::Dialog);
    let _ = update(&mut app, Action::DismissBuildCompletion);
    assert_eq!(app.dialogs.len(), 2, "only the active dialog may dismiss");

    let _ = update(&mut app, Action::CloseBuildOptions);
    assert!(matches!(app.active_dialog(), Some(Dialog::BuildCompletion)));
    assert_eq!(app.focus, FocusTarget::Dialog);
    let _ = update(&mut app, Action::DismissBuildCompletion);
    assert!(app.dialogs.is_empty());
    assert_eq!(app.focus, FocusTarget::Navigator);
}
#[test]
fn dialog_invalid_actions_leave_active_state_unchanged() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenBuildOptions);
    let original = app.clone();

    assert_eq!(update(&mut app, Action::ConfirmDevtoolReset), None);
    let _ = update(&mut app, Action::AppendBbmask('x'));
    let _ = update(&mut app, Action::CancelImagePicker);

    assert_eq!(app, original);
}
#[test]
fn focus_quit_confirmation_traps_and_restores() {
    let mut app = App::new(10, 1_000);
    app.focus = FocusTarget::Navigator;
    app.build.status = BuildStatus::Running;
    let _ = update(&mut app, Action::Quit);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QuitConfirmation)
    ));
    assert_eq!(app.focus, FocusTarget::Dialog);

    let _ = update(&mut app, Action::Open(Screen::Logs));
    assert_eq!(app.screen, Screen::Dashboard);
    let _ = update(&mut app, Action::CancelQuit);
    assert!(app.active_dialog().is_none());
    assert_eq!(app.focus, FocusTarget::Navigator);
}
#[test]
fn parse_progress_tracks_current_and_total() {
    let mut app = App::new(10, 1_000);
    app.build.status = BuildStatus::LoadingWorkspace;
    let _ = update(
        &mut app,
        Action::ParseProgress {
            current: Some(8),
            total: Some(20),
        },
    );
    assert_eq!(app.build.status, BuildStatus::Parsing);
    assert_eq!(app.build.parse_current, Some(8));
    assert_eq!(app.build.parse_total, Some(20));
    let _ = update(&mut app, Action::BuildStarted);
    assert_eq!(app.build.parse_current, None);
    assert_eq!(app.build.parse_total, None);
}
#[test]
fn task_activity_transitions_parsing_to_running_without_regression() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::BuildRequested {
            target: Some("core-image-minimal".into()),
        },
    );
    let _ = update(&mut app, Action::BuildStarted);
    let _ = update(
        &mut app,
        Action::ParseProgress {
            current: Some(59),
            total: Some(100),
        },
    );
    assert_eq!(app.build.status, BuildStatus::Parsing);

    let task = TaskInfo {
        id: TaskId("glibc:do_compile".into()),
        recipe: "glibc".into(),
        task: "do_compile".into(),
        stats: Some(TaskStats {
            completed: 59,
            total: 100,
            active: 1,
            failed: 0,
        }),
        ..TaskInfo::default()
    };
    let _ = update(&mut app, Action::TaskQueued(task));
    assert_eq!(app.build.status, BuildStatus::Running);
    assert_eq!(app.build.parse_current, None);
    assert_eq!(app.build.parse_total, None);

    let _ = update(
        &mut app,
        Action::ParseProgress {
            current: Some(100),
            total: Some(100),
        },
    );
    assert_eq!(app.build.status, BuildStatus::Running);
    assert_eq!(app.build.parse_current, None);
    assert_eq!(app.build.parse_total, None);
}
#[test]
fn eviction_counts_dropped_warnings_and_errors() {
    let mut logs = LogState::new(1, 100);
    logs.insert(tagged_log(
        "busybox",
        "do_compile",
        Severity::Warning,
        "warning",
    ));
    logs.insert(tagged_log(
        "busybox",
        "do_compile",
        Severity::Error,
        "error",
    ));
    logs.insert(log("latest"));
    assert_eq!(logs.dropped, 2);
    assert_eq!(logs.dropped_warnings, 1);
    assert_eq!(logs.dropped_errors, 0);
    assert_eq!(
        logs.entries.front().map(|entry| entry.severity),
        Some(Severity::Error)
    );
}

#[test]
fn concept_failed_build_log_state_exposes_pause_match_and_loss_truthfully() {
    let mut logs = LogState::new(2, 1_000);
    logs.insert(tagged_log(
        "bash_5.2.21-2",
        "do_compile",
        Severity::Warning,
        "WARNING: bash:do_compile mismatch",
    ));
    logs.insert(tagged_log(
        "bash_5.2.21-2",
        "do_compile",
        Severity::Error,
        "ERROR: bash:do_compile failed",
    ));
    logs.insert(tagged_log(
        "bash_5.2.21-2",
        "do_compile",
        Severity::Error,
        "ERROR: bash:do_compile exited 1",
    ));

    logs.follow = false;
    logs.paused_len = Some(logs.entries.len());
    logs.query = "do_compile".into();
    logs.selection = logs.visible_count().saturating_sub(1);

    assert_eq!(logs.dropped, 1);
    assert_eq!(logs.dropped_warnings, 1);
    assert_eq!(logs.dropped_errors, 0);
    assert_eq!(logs.diagnostics().count(), 2);
    assert_eq!(logs.visible_count(), 2);
    assert_eq!(logs.match_position(), Some((2, 2)));
    assert_eq!(logs.paused_len, Some(2));
    assert_eq!(
        logs.selected().and_then(|entry| entry.task.as_deref()),
        Some("do_compile")
    );
}
#[test]
fn ux_scroll_log_eviction_and_filtering_retain_selected_identity_when_present() {
    let mut navigator = App::new(8, 1_000);
    let _ = update(
        &mut navigator,
        Action::SelectNavigator { delta: isize::MAX },
    );
    assert_eq!(
        navigator.navigator_selection,
        NAVIGATOR_SCREENS.len() - 1,
        "edge navigation must not iterate once per signed delta"
    );
    let _ = update(
        &mut navigator,
        Action::SelectNavigator { delta: isize::MIN },
    );
    assert_eq!(navigator.navigator_selection, 0);

    let mut tasks = App::new(8, 1_000);
    tasks.screen = Screen::Tasks;
    for index in 0..15 {
        let id = TaskId(format!("scroll-task-{index}"));
        tasks.tasks.insert(
            id.clone(),
            TaskInfo::active(id, "busybox".into(), "do_compile".into()),
        );
    }
    let _ = update(&mut tasks, Action::ScrollCurrent { to_end: true });
    assert_eq!(tasks.task_progress_scroll, 14);
    let _ = update(&mut tasks, Action::ScrollCurrent { to_end: false });
    assert_eq!(tasks.task_progress_scroll, 0);

    let mut logs = LogState::new(3, 1_000);
    logs.insert(log("alpha"));
    logs.insert(log("beta"));
    logs.insert(log("gamma"));
    logs.follow = false;
    logs.paused_len = Some(3);
    logs.selection = 1;
    let beta_id = logs.selected().unwrap().id;

    logs.insert(log("delta"));
    assert_eq!(logs.selected().map(|entry| entry.id), Some(beta_id));
    assert_eq!(
        logs.selection, 0,
        "eviction before the row shifts its index"
    );

    let mut app = App::new(8, 1_000);
    app.logs.insert(log("alpha"));
    app.logs.insert(log("beta match"));
    app.logs.follow = false;
    app.logs.paused_len = Some(2);
    app.logs.selection = 1;
    let beta_id = app.logs.selected().unwrap().id;
    let _ = update(&mut app, Action::BeginLogSearch);
    let _ = update(&mut app, Action::AppendLogQuery('b'));
    assert_eq!(app.logs.selected().map(|entry| entry.id), Some(beta_id));
    assert_eq!(
        app.logs.selection, 0,
        "filtering recomputes the stable row index"
    );
}
#[test]
fn high_volume_logs_remain_within_retention_limits() {
    let mut logs = LogState::new(128, 4_096);
    for index in 0..20_000 {
        logs.insert(log(&format!("line {index}: {}", "x".repeat(index % 80))));
    }
    assert!(logs.entries.len() <= 128);
    assert!(logs.retained_bytes <= 4_096);
    assert_eq!(
        logs.retained_bytes,
        logs.entries.iter().map(|entry| entry.message.len()).sum()
    );
    assert!(logs.dropped > 0);
}
#[test]
fn task_batches_coalesce_progress_and_preserve_terminal_failures() {
    let mut app = App::new(16, 4096);
    let failed = TaskId("busybox:do_compile".into());
    let succeeded = TaskId("base-files:do_install".into());
    let task = |id: &TaskId| TaskInfo {
        id: id.clone(),
        recipe: id.0.split(':').next().unwrap().into(),
        task: id.0.split(':').nth(1).unwrap().into(),
        ..TaskInfo::default()
    };
    let _ = update(
        &mut app,
        Action::TaskEvents(vec![
            TaskEvent::Started(task(&failed)),
            TaskEvent::Progress {
                id: failed.clone(),
                progress: Some(10),
            },
            TaskEvent::Progress {
                id: failed.clone(),
                progress: Some(90),
            },
            TaskEvent::Started(task(&succeeded)),
            TaskEvent::Progress {
                id: succeeded.clone(),
                progress: Some(40),
            },
            TaskEvent::Completed {
                id: failed,
                success: false,
            },
            TaskEvent::Completed {
                id: succeeded,
                success: true,
            },
        ]),
    );
    assert_eq!(app.task_progress_events, 3);
    assert_eq!(app.task_progress_coalesced, 1);
    assert!(app.tasks.is_empty());
    assert_eq!(app.completed_tasks.len(), 2);
    assert!(!app.completed_tasks[0].success);
    assert_eq!(app.completed_tasks[0].task.state, TaskState::Failed);
    assert!(app.completed_tasks[1].success);
    assert_eq!(app.build.completed, 2);
}
#[test]
fn task_event_flood_bounds_active_and_completed_state_without_losing_terminal_failure() {
    let mut app = App::new(16, 4096);
    let mut events = (0..MAX_ACTIVE_TASKS + 128)
        .map(|index| {
            let id = TaskId(format!("recipe-{index}:do_compile"));
            TaskEvent::Started(TaskInfo {
                id,
                recipe: format!("recipe-{index}"),
                task: "do_compile".into(),
                ..TaskInfo::default()
            })
        })
        .collect::<Vec<_>>();
    let overflow_id = TaskId(format!("recipe-{}:do_compile", MAX_ACTIVE_TASKS + 127));
    events.push(TaskEvent::Completed {
        id: overflow_id.clone(),
        success: false,
    });
    let _ = update(&mut app, Action::TaskEvents(events));
    assert_eq!(app.tasks.len(), MAX_ACTIVE_TASKS);
    assert_eq!(app.task_active_overflow, 128);
    assert!(app.completed_tasks.len() <= MAX_COMPLETED_TASKS);
    assert!(app.completed_tasks.iter().any(|completed| {
        completed.task.id == overflow_id
            && !completed.success
            && completed.task.state == TaskState::Failed
    }));
}
#[test]
fn unchanged_task_projection_reuses_sorted_identity_cache() {
    let mut app = App::new(16, 4096);
    let id = TaskId("busybox:do_compile".into());
    let _ = update(
        &mut app,
        Action::TaskStarted(TaskInfo {
            id: id.clone(),
            recipe: "busybox".into(),
            task: "do_compile".into(),
            ..TaskInfo::default()
        }),
    );
    let rebuilds = app.task_projection_rebuilds();
    assert_eq!(app.visible_task_row_refs_at(SystemTime::now()).len(), 1);
    assert_eq!(app.task_projection_rebuilds(), rebuilds);
    let _ = update(
        &mut app,
        Action::TaskProgress {
            id,
            progress: Some(75),
        },
    );
    let rows = app.visible_task_row_refs_at(SystemTime::now());
    assert!(matches!(rows[0], TaskRowRef::Task { task, .. } if task.progress == Some(75)));
    assert_eq!(app.task_projection_rebuilds(), rebuilds);
    let _ = update(&mut app, Action::CycleTaskStateFilter);
    assert!(app.task_projection_rebuilds() > rebuilds);
}
#[test]
fn reducer_covers_build_lifecycle_and_log_controls() {
    let mut app = App::new(10, 1_000);
    assert!(
        update(
            &mut app,
            Action::Start(BuildRequest {
                targets: vec!["bad target".into()],
                task: None,
                force: false,
            }),
        )
        .is_none()
    );
    assert!(app.notification.is_some());
    let request = BuildRequest {
        targets: vec!["busybox".into()],
        task: Some("compile".into()),
        force: false,
    };
    assert_eq!(
        update(&mut app, Action::Start(request.clone())),
        Some(Effect::Start(request))
    );
    let _ = update(&mut app, Action::BuildStarted);
    let id = TaskId("busybox:do_compile".into());
    let _ = update(
        &mut app,
        Action::TaskStarted(TaskInfo {
            id: id.clone(),
            recipe: "busybox".into(),
            task: "do_compile".into(),
            progress: None,
            ..TaskInfo::default()
        }),
    );
    let _ = update(
        &mut app,
        Action::TaskProgress {
            id: id.clone(),
            progress: Some(50),
        },
    );
    let _ = update(&mut app, Action::TaskCompleted { id, success: true });
    assert_eq!(update(&mut app, Action::Cancel), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::BuildCancellationConfirmation)
    ));
    assert_eq!(
        update(&mut app, Action::ConfirmBuildCancellation),
        Some(Effect::Cancel)
    );
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: false,
            exit_code: Some(1),
        },
    );
    assert_eq!(app.build.status, BuildStatus::Failed);
    assert_eq!(app.build.exit_code, Some(1));
    let _ = update(&mut app, Action::Open(Screen::Logs));
    let _ = update(&mut app, Action::BeginLogSearch);
    let _ = update(&mut app, Action::AppendLogQuery('x'));
    let _ = update(&mut app, Action::BackspaceLogQuery);
    let _ = update(&mut app, Action::FinishLogSearch);
    let _ = update(&mut app, Action::ScrollLogsHorizontally { delta: 5 });
    let _ = update(&mut app, Action::ScrollLogsHorizontally { delta: -5 });
    let _ = update(
        &mut app,
        Action::Failure(AppError::new("test", "failure", "retry")),
    );
    let _ = update(&mut app, Action::DismissNotification);
    assert!(app.notification.is_none());
}
#[test]
fn beginning_a_build_clears_stale_build_state() {
    let mut app = App::new(10, 1_000);
    app.build.completed = 7;
    app.build.total = Some(10);
    app.build.parse_current = Some(3);
    app.build.parse_total = Some(4);
    app.build.warnings = 2;
    app.build.errors = 1;
    app.build.exit_code = Some(1);
    app.build.started = Some(SystemTime::now());
    app.tasks.insert(
        TaskId("old:task".into()),
        TaskInfo {
            id: TaskId("old:task".into()),
            recipe: "old".into(),
            task: "task".into(),
            progress: Some(50),
            ..TaskInfo::default()
        },
    );
    let request = BuildRequest {
        targets: vec!["busybox".into()],
        task: None,
        force: false,
    };
    assert_eq!(
        update(&mut app, Action::Start(request.clone())),
        Some(Effect::Start(request))
    );
    assert_eq!(app.build.status, BuildStatus::LoadingWorkspace);
    assert_eq!(app.build.target.as_deref(), Some("busybox"));
    assert_eq!(app.build.completed, 0);
    assert_eq!(app.build.total, None);
    assert_eq!(app.build.parse_current, None);
    assert_eq!(app.build.parse_total, None);
    assert_eq!(app.build.warnings, 0);
    assert_eq!(app.build.errors, 0);
    assert_eq!(app.build.exit_code, None);
    assert_eq!(app.build.started, None);
    assert!(app.tasks.is_empty());
}
