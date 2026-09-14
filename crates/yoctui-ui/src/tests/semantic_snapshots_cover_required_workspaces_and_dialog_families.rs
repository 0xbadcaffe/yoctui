//! Regression tests grouped around semantic_snapshots_cover_required_workspaces_and_dialog_families.
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

#[test]
fn ux_responsive_all_screens_and_dialogs_render_at_boundary_sizes() {
    let screens = [
        Screen::Dashboard,
        Screen::Tasks,
        Screen::BuildHistory,
        Screen::Dependencies,
        Screen::LayerRelationships,
        Screen::Recipes,
        Screen::Images,
        Screen::Sdk,
        Screen::Testing,
        Screen::Security,
        Screen::Layers,
        Screen::Configuration,
        Screen::Bbmask,
        Screen::Maintenance,
        Screen::Logs,
        Screen::Errors,
        Screen::Help,
        Screen::BuildEnvironment,
        Screen::Compatibility,
        Screen::Settings,
    ];
    for screen in screens {
        for (width, height) in [
            (200, 60),
            (160, 50),
            (130, 40),
            (100, 30),
            (80, 24),
            (79, 23),
        ] {
            let mut app = App::new(10, 1_000);
            app.screen = screen;
            let output = rendered_text(&app, width, height);
            assert!(
                !output.contains('\u{fffd}'),
                "{screen:?} at {width}x{height}"
            );
        }
    }

    let mut build_options = App::new(10, 1_000);
    build_options.dialogs.push_back(Dialog::BuildOptions);
    build_options.focus = FocusTarget::Dialog;
    let _ = rendered_text(&build_options, 80, 24);

    let mut palette = App::new(10, 1_000);
    palette.command_palette_open = true;
    palette.focus = FocusTarget::CommandPalette;
    let _ = rendered_text(&palette, 80, 24);

    let mut confirmation = App::new(10, 1_000);
    confirmation
        .dialogs
        .push_back(Dialog::RecipeTaskConfirmation(BuildRequest {
            targets: vec!["base-files".into()],
            task: Some("listtasks".into()),
            force: false,
        }));
    confirmation.focus = FocusTarget::Dialog;
    let _ = rendered_text(&confirmation, 80, 24);
}
#[test]
fn dialog_families_render_on_narrow_supported_terminals() {
    let dialogs = vec![
        (Dialog::BuildOptions, "Image build options"),
        (Dialog::BuildCompletion, "Build finished"),
        (
            Dialog::BuildTarget {
                editor: yoctui_model::PopupEditor::new("target = \"busybox\"\n".into()),
                task: None,
            },
            "Build target",
        ),
        (
            Dialog::ImagePicker(yoctui_model::ImagePicker {
                images: vec!["core-image-minimal".into()],
                selection: 0,
            }),
            "Available image targets",
        ),
        (
            Dialog::RecipeTaskConfirmation(BuildRequest {
                targets: vec!["busybox".into()],
                task: None,
                force: false,
            }),
            "Confirm recipe task",
        ),
        (
            Dialog::DevtoolModifyConfirmation(yoctui_model::RecipeIdentity {
                name: "busybox".into(),
                file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
            }),
            "Confirm Devtool modify",
        ),
        (
            Dialog::DevtoolResetConfirmation(yoctui_model::DevtoolResetPlan {
                identity: yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                },
                source_path: "/build/workspace/sources/busybox".into(),
            }),
            "Confirm Devtool reset",
        ),
        (
            Dialog::DevtoolUpdateConfirmation(yoctui_model::RecipeIdentity {
                name: "busybox".into(),
                file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
            }),
            "Confirm Devtool update-recipe",
        ),
        (
            Dialog::DevtoolFinishPicker(yoctui_model::DevtoolFinishPicker {
                identity: yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                },
                layers: vec![yoctui_model::Layer {
                    name: "meta".into(),
                    path: "/layers/meta".into(),
                    priority: Some(5),
                }],
                selection: 0,
            }),
            "Devtool finish busybox",
        ),
        (
            Dialog::DevtoolFinishConfirmation(yoctui_model::DevtoolFinishPlan {
                identity: yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                },
                layer: yoctui_model::Layer {
                    name: "meta".into(),
                    path: "/layers/meta".into(),
                    priority: Some(5),
                },
            }),
            "Confirm Devtool finish",
        ),
        (
            Dialog::DevtoolDeploy(yoctui_model::DevtoolDeployDraft {
                identity: yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                },
                target: "qemu".into(),
            }),
            "Devtool deploy target",
        ),
        (
            Dialog::DevtoolDeployConfirmation(yoctui_model::DevtoolDeployPlan {
                identity: yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                },
                target: "qemu".into(),
            }),
            "Confirm Devtool deploy-target",
        ),
        (
            Dialog::BbmaskEdit(yoctui_model::PopupEditor::new(
                "bbmask = \"meta-old/.*\"\n".into(),
            )),
            "BBMASK.toml",
        ),
        (
            Dialog::BbmaskConfirmation("meta-old/.*".into()),
            "Confirm BBMASK change",
        ),
        (
            Dialog::RecipeEditor(RecipeEditor {
                recipe: "busybox".into(),
                root: "/workspace/busybox".into(),
                files: vec!["main.c".into()],
                selection: 0,
                focus: yoctui_model::RecipeEditorFocus::Files,
                language: yoctui_model::SourceLanguage::C,
                document: yoctui_model::TextAreaState::new("int main() {}".into()),
                searching: false,
            }),
            "Workspace file tree",
        ),
        (
            Dialog::BuildCancellationConfirmation,
            "Confirm build cancellation",
        ),
        (Dialog::QuitConfirmation, "Confirm exit"),
    ];

    for (dialog, title) in dialogs {
        let mut app = App::new(10, 1_000);
        app.build.status = yoctui_model::BuildStatus::Completed;
        app.dialogs.push_back(dialog);
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains(title), "missing {title} in narrow dialog");
        assert!(output.contains("modal ·"), "missing modal shell: {output}");
    }
}

#[test]
fn next_generation_dialogs_name_standard_confirmation_destructive_and_result_shells() {
    let reset = yoctui_model::DevtoolResetPlan {
        identity: yoctui_model::RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
        },
        source_path: "/build/workspace/sources/busybox".into(),
    };
    for (dialog, expected, control) in [
        (Dialog::BuildOptions, "modal · Image build options", "Esc"),
        (
            Dialog::RecipeTaskConfirmation(BuildRequest {
                targets: vec!["busybox".into()],
                task: Some("compile".into()),
                force: false,
            }),
            "confirm modal · Confirm recipe task",
            "Esc",
        ),
        (
            Dialog::DevtoolResetConfirmation(reset),
            "destructive modal · Confirm Devtool reset",
            "Esc",
        ),
        (
            Dialog::BuildCompletion,
            "result modal · Build finished",
            "any key",
        ),
    ] {
        let mut app = App::new(32, 8192);
        app.focus = FocusTarget::Dialog;
        app.build.status = BuildStatus::Completed;
        app.dialogs.push_front(dialog);
        for (width, height) in [(160, 40), (100, 30), (80, 24)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains(expected), "{width}x{height}: {output}");
            assert!(output.contains(control), "{width}x{height}: {output}");
        }
    }
}

#[test]
fn next_generation_dialogs_reserve_validation_fields_and_controls() {
    let mut app = qemu_workspace_app();
    app.focus = FocusTarget::Dialog;
    let artifact = app.selected_image_artifact().unwrap().identity.clone();
    let mut launch = QemuLaunchDialog::new(yoctui_model::QemuLaunchDraft::for_artifact(
        artifact,
        yoctui_model::ImageArtifactKind::Wic,
    ));
    launch.selected_field = QemuLaunchField::Kernel;
    launch.editing = true;
    launch.draft.kernel = "relative/kernel".into();
    launch.validation_error = Some("kernel path must be absolute".repeat(20));
    app.dialogs.push_front(Dialog::QemuLaunch(launch));
    for (width, height) in [(160, 40), (100, 30), (80, 24)] {
        let output = rendered_text(&app, width, height);
        for expected in [
            "modal · Launch runqemu",
            "▶ Kernel [editing]",
            "✕ Validation: kernel path must be absolute",
            "[p] Preview",
            "[Esc] Close",
        ] {
            assert!(output.contains(expected), "{width}x{height}: {output}");
        }
        assert_eq!(output.matches("▶ Kernel").count(), 1, "{output}");
    }

    app.dialogs.clear();
    let mut editor =
        yoctui_model::PopupEditor::new("destination = \"⟦/exports/result.xml⟧\"\n".into());
    editor.cursor = editor.text.len();
    app.dialogs.push_front(Dialog::SdkPublishTomlEditor(editor));
    let output = rendered_text(&app, 80, 24);
    assert!(
        output.contains("modal · SDK publish.toml — NORMAL"),
        "{output}"
    );
    assert!(output.contains("[Enter] Save/preview"), "{output}");
    assert!(output.contains("Home/End line"), "{output}");
    assert!(output.contains("Ctrl+V paste"), "{output}");
}

#[test]
fn next_generation_dialogs_keep_accessible_focus_and_bounded_geometry() {
    for (theme, color_enabled) in [
        (Theme::HighContrast, true),
        (Theme::Monochrome, true),
        (Theme::DarkPro, false),
    ] {
        let mut app = App::new(32, 8192);
        app.theme = theme;
        app.color_enabled = color_enabled;
        app.reduced_motion = true;
        app.focus = FocusTarget::Dialog;
        app.dialogs.push_front(Dialog::QuitConfirmation);
        let output = rendered_text(&app, 80, 24);
        assert!(
            output.contains("destructive modal · Confirm exit"),
            "{output}"
        );
        assert!(
            output.contains("Are you sure you want to exit yoctui?"),
            "{output}"
        );
        assert!(output.contains("[y/Enter] Exit yoctui"), "{output}");
        assert!(output.contains("[n/Esc] Stay"), "{output}");
    }

    let mut cancellation = App::new(32, 8192);
    cancellation.focus = FocusTarget::Dialog;
    cancellation.build.status = BuildStatus::Running;
    cancellation
        .dialogs
        .push_front(Dialog::BuildCancellationConfirmation);
    let output = rendered_text(&cancellation, 80, 24);
    assert!(
        output.contains("Are you sure you want to cancel the build?"),
        "{output}"
    );
    assert!(output.contains("[y/Enter] Cancel build"), "{output}");
    assert!(output.contains("[n/Esc] Keep building"), "{output}");

    for area in [
        Rect::new(0, 0, 200, 60),
        Rect::new(0, 0, 100, 30),
        Rect::new(0, 0, 80, 24),
        Rect::new(7, 11, 80, 24),
    ] {
        let popup = dialog_popup_rect(area, 110, 30);
        assert!(popup.x >= area.x && popup.y >= area.y);
        assert!(popup.right() <= area.right() && popup.bottom() <= area.bottom());
        assert!(popup.width <= area.width && popup.height <= area.height);
        if area.width >= 2 && area.height >= 2 {
            assert!(popup.x > area.x && popup.y > area.y);
        }
    }
}

#[test]
fn command_palette_renders_search_results_and_disabled_explanations() {
    let mut app = App::new(10, 1_000);
    app.command_palette_open = true;
    app.command_palette_query = "build".into();
    let output = rendered_text(&app, 100, 25);
    assert!(output.contains("[EDITING] Query: build▏"));
    assert!(output.contains("Ctrl+U clear"));
    assert!(output.contains("Build image"));
    assert!(output.contains("Cannot run:"));
    assert!(output.contains("Load a Yocto workspace first"));
    assert!(output.contains("Type search"));

    app.command_palette_query = "nothing matches this".into();
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("No commands match"));
}

#[test]
fn global_regex_search_renders_results_and_inline_errors() {
    let mut app = App::new(10, 1_000);
    app.command_palette_open = true;
    app.command_palette_mode = CommandPaletteMode::GlobalRegexSearch;
    app.command_palette_query = "^open (packages|sdk)$".into();
    let output = rendered_text(&app, 100, 25);
    assert!(output.contains("Global Regex Search"), "{output}");
    assert!(output.contains("Open Packages"), "{output}");
    assert!(output.contains("Open SDK"), "{output}");
    assert!(output.contains("Unified regex"), "{output}");

    app.command_palette_query = "[".into();
    let output = rendered_text(&app, 100, 25);
    assert!(output.contains("Invalid regular expression"), "{output}");
    assert!(output.contains("unclosed character class"), "{output}");
}

#[test]
fn global_search_renders_generated_image_service_with_provenance() {
    let mut app = App::new(10, 1_000);
    app.command_palette_open = true;
    app.command_palette_mode = CommandPaletteMode::GlobalRegexSearch;
    app.command_palette_query = "watchdog".into();
    app.global_search_content = yoctui_model::GlobalSearchContentState::Ready {
            generation: 1,
            query: "watchdog".into(),
            hits: vec![yoctui_model::GlobalSearchHit {
                kind: yoctui_model::GlobalSearchContentKind::ImageRootfs,
                path: "/build/tmp/work/qemux/core-image-demo/1.0/rootfs/usr/lib/systemd/system/watchdog.service".into(),
                line: 4,
                column: 13,
                preview: "Description=watchdog service".into(),
                image: Some("core-image-demo".into()),
            }],
            truncated: false,
            searched_scopes: vec!["generated-work=/build/tmp/work".into()],
        };
    let output = rendered_text(&app, 120, 28);
    for expected in [
        "Generated image rootfs",
        "watchdog.service",
        "line 4 · column 13",
        "Description=watchdog service",
        "Generated image: core-image-demo",
    ] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }
}

#[test]
fn ux_action_catalog_projects_alias_search_palette_details_and_help() {
    let mut app = App::new(10, 1_000);
    app.command_palette_open = true;
    app.command_palette_query = "capabilities".into();
    let output = rendered_text(&app, 100, 25);
    assert!(output.contains("Open Compatibility"), "{output}");
    assert!(output.contains("environment identity"), "{output}");

    app.command_palette_open = false;
    app.command_palette_query.clear();
    app.screen = Screen::Help;
    let output = rendered_text(&app, 160, 50);
    assert!(output.contains("Action catalog"), "{output}");
    assert!(output.contains("Build > Build image"), "{output}");
    assert!(output.contains("[Navigate]"), "{output}");
    assert!(output.contains("Open Tasks"), "{output}");
    assert!(output.contains("Enter/Right open or expand"), "{output}");
    assert!(output.contains("Right expands a group"), "{output}");
}
#[test]
fn command_palette_selection_description_and_shortcut_render_in_all_themes() {
    for theme in [
        Theme::DarkPro,
        Theme::WhiteClassic,
        Theme::MatrixGreen,
        Theme::HighContrast,
        Theme::Monochrome,
    ] {
        let mut app = App::new(10, 1_000);
        app.theme = theme;
        app.command_palette_open = true;
        app.command_palette_query = "Open Settings".into();
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains("Open Settings"));
        assert!(output.contains("persistent visual"));
        assert!(output.contains("none"));
    }
}

#[test]
fn next_generation_palette_uses_documented_responsive_geometry() {
    for (area, expected) in [
        (Rect::new(0, 0, 200, 60), Rect::new(44, 15, 112, 30)),
        (Rect::new(0, 0, 160, 50), Rect::new(24, 10, 112, 30)),
        (Rect::new(0, 0, 130, 40), Rect::new(9, 5, 112, 30)),
        (Rect::new(0, 0, 100, 30), Rect::new(3, 2, 94, 26)),
        (Rect::new(0, 0, 80, 24), Rect::new(1, 1, 78, 22)),
    ] {
        assert_eq!(command_palette_rect(area), expected);
    }
    assert_eq!(
        command_palette_rect(Rect::new(7, 11, 100, 30)),
        Rect::new(10, 13, 94, 26)
    );
}
