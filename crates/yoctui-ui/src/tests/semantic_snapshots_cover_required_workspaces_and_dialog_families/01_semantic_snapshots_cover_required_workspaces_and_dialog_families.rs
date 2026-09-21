use super::*;

#[test]
fn semantic_snapshots_cover_required_workspaces_and_dialog_families() {
    let mut app = literal_reference_app();
    app.daemon.pty_sessions.clear();
    app.recipe_selection = 2;
    app.layer_selection = 4;
    app.settings_selection = 0;
    let catalog = [
        SemanticSnapshot {
            name: "dashboard",
            screen: Screen::Dashboard,
            anchors: &[
                "Build Overview",
                "Target: core-image-minimal",
                "Quick Actions",
                "Job History",
                "Resource Telemetry",
            ],
            selected: None,
        },
        SemanticSnapshot {
            name: "tasks",
            screen: Screen::Tasks,
            anchors: &[
                "Tasks: core-image-minimal",
                "do_compile",
                "Log Viewer",
                "Job History",
                "Inspector: Task",
            ],
            selected: Some("do_compile"),
        },
        SemanticSnapshot {
            name: "logs",
            screen: Screen::Logs,
            anchors: &[
                "Log activity",
                "following",
                "Log Viewer",
                "[ 72%] Linking bash",
            ],
            selected: Some("[ 72%] Linking bash"),
        },
        SemanticSnapshot {
            name: "jobs",
            screen: Screen::BuildHistory,
            anchors: &[
                "Job History / Build history",
                "core-image-minimal",
                "Selected job detail",
                "Operation:",
            ],
            selected: Some("core-image-m"),
        },
        SemanticSnapshot {
            name: "recipes",
            screen: Screen::Recipes,
            anchors: &[
                "Recipes (shown: 3 of 3)",
                "core-image-minimal",
                "Recipe preview",
            ],
            selected: Some("core-image-m"),
        },
        SemanticSnapshot {
            name: "layers",
            screen: Screen::Layers,
            anchors: &["Active layer tree", "meta-oe", "Recipes: meta-oe"],
            selected: Some("meta-oe"),
        },
        SemanticSnapshot {
            name: "images",
            screen: Screen::Images,
            anchors: &[
                "Images",
                "MACHINE qemux86-64",
                "core-image-minimal",
                "Artifacts not loaded. Press R to scan.",
            ],
            selected: None,
        },
        SemanticSnapshot {
            name: "settings",
            screen: Screen::Settings,
            anchors: &["Settings", "Theme", "Dark blue", "Settings controls"],
            selected: Some("Theme"),
        },
        SemanticSnapshot {
            name: "build-environment",
            screen: Screen::BuildEnvironment,
            anchors: &[
                "Build environment",
                "connected",
                "available images:",
                "b Browse directories",
                "V Initialize and verify",
            ],
            selected: None,
        },
    ];
    for snapshot in &catalog {
        assert_semantic_snapshot(&app, snapshot);
    }

    let terminal_app = literal_reference_app();
    assert_semantic_snapshot(
        &terminal_app,
        &SemanticSnapshot {
            name: "terminal-session",
            screen: Screen::TerminalSessions,
            anchors: &["terminal #1 Running", "1 viewer(s)", "Screen unavailable"],
            selected: None,
        },
    );

    let dialog = |value: Dialog| {
        let mut app = literal_reference_app();
        app.focus = FocusTarget::Dialog;
        app.dialogs.push_back(value);
        app
    };
    assert_dialog_semantic_snapshot(
        "standard",
        &dialog(Dialog::BuildOptions),
        &[
            "modal · Image build options",
            "Machine: qemux86-64",
            "Esc closes",
        ],
    );
    assert_dialog_semantic_snapshot(
        "confirmation",
        &dialog(Dialog::RecipeTaskConfirmation(BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("compile".into()),
            force: false,
        })),
        &[
            "confirm modal · Confirm recipe task",
            "bitbake busybox -c compile",
            "Enter to continue or Esc to cancel",
        ],
    );
    assert_dialog_semantic_snapshot(
        "destructive",
        &dialog(Dialog::DevtoolResetConfirmation(
            yoctui_model::DevtoolResetPlan {
                identity: yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/work/meta/recipes-core/busybox/busybox.bb".into(),
                },
                source_path: "/work/build/workspace/sources/busybox".into(),
            },
        )),
        &[
            "destructive modal · Confirm Devtool reset",
            "devtool reset busybox",
            "This removes the Devtool workspace",
            "Esc cancels",
        ],
    );
    let mut result = dialog(Dialog::BuildCompletion);
    result.build.status = BuildStatus::Completed;
    assert_dialog_semantic_snapshot(
        "result",
        &result,
        &[
            "result modal · Build finished",
            "completed successfully",
            "Tasks completed: 4",
            "Press any key",
        ],
    );
    assert_dialog_semantic_snapshot(
        "editor",
        &dialog(Dialog::BuildTarget {
            editor: yoctui_model::PopupEditor::new("target = \"core-image-minimal\"\n".into()),
            task: Some("build".into()),
        }),
        &[
            "modal · Build target.toml",
            "requested task: build",
            "core-image-minimal",
            "[Enter] Save/preview",
            "[Esc] Normal",
        ],
    );
}

#[test]
fn style_invariants_enforce_focus_titles_status_progress_and_disabled_actions() {
    let focused_corner_count = |app: &App, width: u16, height: u16| {
        let palette = ThemePalette::for_app(app);
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render_at(frame, app, literal_now()))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .filter(|cell| cell.symbol() == "┌" && cell.fg == palette.focused_border)
            .count()
    };

    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        for focus in [
            FocusTarget::Navigator,
            FocusTarget::Workspace,
            FocusTarget::Inspector,
        ] {
            let mut app = literal_reference_app();
            app.focus = focus;
            assert_eq!(
                focused_corner_count(&app, width, height),
                1,
                "{width}x{height} {focus:?} must expose exactly one focused border"
            );
        }
    }
    let mut dialog = literal_reference_app();
    dialog.focus = FocusTarget::Dialog;
    dialog.dialogs.push_back(Dialog::BuildOptions);
    assert_eq!(focused_corner_count(&dialog, 100, 30), 1);

    let titled = literal_reference_app();
    let mut terminal = Terminal::new(TestBackend::new(160, 50)).unwrap();
    terminal
        .draw(|frame| render_at(frame, &titled, literal_now()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    for y in 5..47 {
        for x in 0..160 {
            if buffer[(x, y)].symbol() != "┌" {
                continue;
            }
            let right = (x + 1..160)
                .find(|right| buffer[(*right, y)].symbol() == "┐")
                .expect("every body border has a bounded right corner");
            let title = (x + 1..right)
                .map(|column| buffer[(column, y)].symbol())
                .filter(|symbol| !matches!(*symbol, "─" | " "))
                .collect::<String>();
            assert!(
                !title.is_empty(),
                "body pane at ({x},{y}) has no semantic section title"
            );
        }
    }

    let palette = ThemePalette::for_app(&titled);
    for (tone, expected) in [
        (StatusTone::Success, palette.success),
        (StatusTone::Warning, palette.warning),
        (StatusTone::Error, palette.error),
        (StatusTone::Running, palette.running),
        (StatusTone::Pending, palette.pending),
        (StatusTone::Accent, palette.accent),
        (StatusTone::Muted, palette.muted),
        (StatusTone::Info, palette.informational),
        (StatusTone::Disabled, palette.disabled),
    ] {
        assert_eq!(status_tone_style(&palette, tone).fg, Some(expected));
        assert!(!tone.marker().is_empty());
    }

    let mut state_app = App::new(32, 8192);
    state_app.screen = Screen::Tasks;
    state_app.reduced_motion = true;
    let states = [
        (TaskState::Queued, None),
        (TaskState::Waiting, None),
        (TaskState::Active, Some(42)),
        (TaskState::Active, None),
        (TaskState::Completed, Some(100)),
        (TaskState::Failed, None),
        (TaskState::Cancelled, None),
        (TaskState::Lost, None),
    ];
    let tasks = states
        .iter()
        .enumerate()
        .map(|(index, (state, progress))| yoctui_model::TaskInfo {
            id: yoctui_model::TaskId(format!("recipe-{index}:do_state")),
            recipe: format!("recipe-{index}"),
            task: format!("do_state_{index}"),
            state: *state,
            progress: *progress,
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let rows = tasks
        .iter()
        .zip(states)
        .map(|(task, (state, _))| TaskRowRef::Task { task, state })
        .collect::<Vec<_>>();
    let mut task_terminal = Terminal::new(TestBackend::new(120, 16)).unwrap();
    task_terminal
        .draw(|frame| render_task_table(frame, &state_app, frame.area(), &rows, literal_now()))
        .unwrap();
    let task_output = task_terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    for expected in [
        "· Queued",
        "▫ Waiting",
        "▶ Running",
        "progress unknown",
        "42%",
        "✓ Succeeded",
        "100%",
        "✕ Failed",
        "■ Cancelled",
        "? Lost",
    ] {
        assert!(
            task_output.contains(expected),
            "missing {expected}: {task_output}"
        );
    }
    assert_eq!(
        task_output.matches('%').count(),
        3,
        "only authoritative active/completed percentages may render: {task_output}"
    );

    let items = vec![
        ActionListItem {
            marker: "✓",
            label: "Enabled action".into(),
            shortcut: "e".into(),
            state: "Local".into(),
            enabled: true,
            details: Vec::new(),
        },
        ActionListItem {
            marker: "×",
            label: "Disabled action".into(),
            shortcut: "d".into(),
            state: "Disabled".into(),
            enabled: false,
            details: Vec::new(),
        },
    ];
    let mut action_terminal = Terminal::new(TestBackend::new(60, 2)).unwrap();
    action_terminal
        .draw(|frame| {
            frame.render_widget(
                Paragraph::new(action_list(&items, 60, inspector_action_styles(&titled))),
                frame.area(),
            )
        })
        .unwrap();
    let disabled_row = &action_terminal.backend().buffer().content[60..120];
    assert!(disabled_row.iter().any(|cell| cell.symbol() == "×"));
    assert!(
        disabled_row
            .iter()
            .filter(|cell| cell.symbol() != " ")
            .all(|cell| {
                cell.fg == palette.disabled
                    && cell.bg != palette.selection_background
                    && cell.fg != palette.accent
            })
    );
}
