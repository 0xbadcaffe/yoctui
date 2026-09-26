use super::*;

#[test]
fn next_generation_inspector_shell_names_modes_and_orders_typed_sections() {
    let mut app = App::new(10, 1_000);
    app.focus = FocusTarget::Inspector;
    app.screen = Screen::Logs;
    app.logs.insert(yoctui_model::LogEntry {
        id: 1,
        severity: Severity::Warning,
        message: "authoritative warning output".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: Some("/build/temp/log.do_compile".into()),
        timestamp: UNIX_EPOCH + Duration::from_secs(10),
        build: Some("core-image-minimal".into()),
        protected: true,
        diagnostic: None,
    });
    let log = rendered_text_at(&app, 180, 36, UNIX_EPOCH + Duration::from_secs(11));
    for expected in [
        "Inspector: Log",
        "▾ PRIMARY FACTS",
        "▾ RELATED PATHS",
        "/build/temp/log.do_compile",
        "▾ RECENT OUTPUT",
        "authoritative warning output",
    ] {
        assert!(log.contains(expected), "missing {expected}: {log}");
    }

    app.screen = Screen::Layers;
    let mut browser = LayerBrowser::new("meta-demo".into(), "/layers/meta-demo".into());
    browser.entries.push(yoctui_model::LayerBrowserEntry {
        path: "conf/layer.conf".into(),
        ..yoctui_model::LayerBrowserEntry::default()
    });
    app.layer_browser = Some(browser);
    let file = rendered_text_at(&app, 180, 36, UNIX_EPOCH + Duration::from_secs(11));
    assert!(file.contains("File preview"), "{file}");
    assert!(file.contains("/layers/meta-demo/conf/layer.conf"), "{file}");

    app.screen = Screen::Dashboard;
    app.color_enabled = false;
    let narrow = rendered_text_at(&app, 90, 24, UNIX_EPOCH + Duration::from_secs(11));
    assert!(narrow.contains("Project Inspector"), "{narrow}");
    assert!(narrow.contains("Environment"), "{narrow}");
}
#[test]
fn recipes_workspace_renders_authoritative_summary_and_inspector_sections() {
    let mut terminal = Terminal::new(TestBackend::new(180, 44)).unwrap();
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Recipes;
    app.workspace.recipes = vec![
        yoctui_model::Recipe {
            name: "alpha".into(),
            ..yoctui_model::Recipe::default()
        },
        yoctui_model::Recipe {
            name: "busybox".into(),
            version: Some("1.36".into()),
            preferred_version: Some("1.36%".into()),
            layer: Some("core".into()),
            file: Some("/layers/meta/recipes-core/busybox/busybox_1.36.bb".into()),
            append_count: Some(2),
        },
    ];
    app.recipe_selection = 1;
    app.metadata_query = "busy".into();
    app.recipe_metadata.insert(
        "busybox".into(),
        yoctui_model::RecipeMetadata {
            recipe: "busybox".into(),
            workspace_status: Some(yoctui_model::RecipeWorkspaceStatus::Modified),
            build_status: None,
            tasks: Some(vec!["do_build".into(), "do_compile".into()]),
            sources: Some(vec![
                "/layers/meta/recipes-core/busybox/busybox_1.36.bb".into(),
            ]),
            patches: Some(vec!["file://security.patch".into()]),
            packages: Some(vec!["busybox".into(), "busybox-src".into()]),
            history: None,
        },
    );
    let identity = yoctui_model::RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta/recipes-core/busybox/busybox_1.36.bb".into(),
    };
    app.devtool_statuses.insert(
        identity.clone(),
        yoctui_model::DevtoolStatus {
            identity,
            capability: yoctui_model::DevtoolCapability::Available,
            workspace: yoctui_model::DevtoolWorkspace::Present {
                source_path: "/build/workspace/sources/busybox".into(),
                recipe_file: Some("/layers/meta/recipes-core/busybox/busybox_1.36.bb".into()),
            },
            git: yoctui_model::DevtoolGitState::Available {
                repository_root: Some("/build/workspace/sources/busybox".into()),
                branch: Some("devtool".into()),
                upstream: Some("origin/devtool".into()),
                ahead: 0,
                behind: 0,
                head: Some("abc123".into()),
                modified: 1,
                untracked: 0,
                conflicted: 0,
            },
            error: None,
        },
    );
    app.dependencies = Some(yoctui_model::RecipeDependencies {
        recipe: "busybox".into(),
        build: vec!["virtual/libc".into()],
        runtime: vec!["busybox-udhcpc".into()],
    });
    app.tasks.insert(
        yoctui_model::TaskId("busybox:do_compile".into()),
        yoctui_model::TaskInfo {
            id: yoctui_model::TaskId("busybox:do_compile".into()),
            recipe: "busybox".into(),
            task: "do_compile".into(),
            state: TaskState::Active,
            ..yoctui_model::TaskInfo::default()
        },
    );
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Recipes (shown: 1 of 2)"));
    assert!(output.contains("Resolved"));
    assert!(output.contains("Preferred"));
    assert!(output.contains("Provider file"));
    assert!(output.contains("Workspace/Devtool: member at"));
    assert!(output.contains("Git branch devtool"));
    assert!(output.contains("Active tasks: do_compile"));
    assert!(output.contains("virtual/libc"));
    assert!(output.contains("security.patch"));
    assert!(output.contains("busybox-src"));
    assert!(output.contains("History: unavailable"));
}

#[test]
fn render_cache_metadata_viewports_follow_selection_and_query_without_stale_rows() {
    let mut app = App::new(16, 4096);
    app.screen = Screen::Recipes;
    app.focus = FocusTarget::Workspace;
    app.workspace.recipes = (0..4_096)
        .map(|index| yoctui_model::Recipe {
            name: format!("recipe-{index:05}"),
            layer: Some(format!("meta-{:04}", index % 1_024)),
            ..Default::default()
        })
        .collect();
    app.workspace.layers = (0..1_024)
        .map(|index| yoctui_model::Layer {
            name: format!("meta-{index:04}"),
            path: format!("/layers/meta-{index:04}").into(),
            priority: Some(index),
        })
        .collect();

    let render_recipes = |app: &App| {
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal
            .draw(|frame| recipes(frame, app, frame.area()))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    };
    app.recipe_selection = 4_000;
    let selected = render_recipes(&app);
    assert!(selected.contains("recipe-04000"), "{selected}");
    assert!(!selected.contains("recipe-00000"), "{selected}");

    app.metadata_query = "recipe-00042".into();
    app.recipe_selection = 42;
    let filtered = render_recipes(&app);
    assert!(filtered.contains("recipe-00042"), "{filtered}");
    assert!(!filtered.contains("recipe-04000"), "{filtered}");

    app.metadata_query.clear();
    app.layer_selection = 1_000;
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal
        .draw(|frame| layers(frame, &app, frame.area()))
        .unwrap();
    let layer_output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(layer_output.contains("meta-1000"), "{layer_output}");
    assert!(!layer_output.contains("meta-0000"), "{layer_output}");
}

#[test]
fn recipes_workspace_partial_failure_and_all_responsive_modes_are_safe() {
    for (width, height) in [(160, 30), (110, 28), (90, 25), (70, 20)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Recipes;
        app.focus = FocusTarget::Workspace;
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "demo".into(),
            ..yoctui_model::Recipe::default()
        });
        app.recipe_metadata_errors
            .insert("demo".into(), "metadata service unavailable".into());
        terminal.draw(|frame| render(frame, &app)).unwrap();
        if width >= 80 && height >= 24 {
            let output = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(output.contains("demo"));
            assert!(output.contains("unavailable"));
        }
    }
}
#[test]
fn recipe_bitbake_action_renders_task_picker_and_exact_forced_confirmation() {
    for (width, height) in [(120, 30), (90, 25)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut app = App::new(10, 1_000);
        app.dialogs
            .push_back(Dialog::RecipeTaskPicker(yoctui_model::RecipeTaskPicker {
                recipe: "busybox".into(),
                tasks: vec!["clean".into(), "compile".into(), "devshell".into()],
                selection: 1,
                force: true,
            }));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Force task: busybox"));
        assert!(output.contains("Authoritative BitBake tasks"));
        assert!(output.contains("compile"));

        app.dialogs.clear();
        app.dialogs
            .push_back(Dialog::RecipeTaskConfirmation(BuildRequest {
                targets: vec!["busybox".into()],
                task: Some("compile".into()),
                force: true,
            }));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("bitbake -f busybox -c compile"));
    }
}
#[test]
fn recipe_navigation_renders_log_patch_pickers_and_disabled_reasons_responsively() {
    for (width, height) in [(160, 32), (110, 28), (90, 25)] {
        let mut app = App::new(10, 1_000);
        app.dialogs.push_back(Dialog::RecipeTaskLogPicker(
            yoctui_model::RecipeTaskLogPicker {
                recipe: "busybox".into(),
                logs: vec![
                    yoctui_model::RecipeTaskLogChoice {
                        task: "do_compile".into(),
                        state: TaskState::Failed,
                        path: "/tmp/log.do_compile".into(),
                    },
                    yoctui_model::RecipeTaskLogChoice {
                        task: "do_install".into(),
                        state: TaskState::Completed,
                        path: "/tmp/log.do_install".into(),
                    },
                ],
                selection: 1,
            },
        ));
        let output = rendered_text(&app, width, height);
        assert!(output.contains("busybox retained task logs"), "{output}");
        assert!(output.contains("/tmp/log.do_install"), "{output}");

        app.dialogs.clear();
        app.dialogs
            .push_back(Dialog::RecipePatchPicker(yoctui_model::RecipePatchPicker {
                recipe: "busybox".into(),
                patches: vec!["/layers/meta/recipes-core/busybox/files/fix.patch".into()],
                selection: 0,
            }));
        let output = rendered_text(&app, width, height);
        assert!(output.contains("busybox patch review"), "{output}");
        assert!(output.contains("fix.patch"), "{output}");
    }

    let mut app = App::new(10, 1_000);
    app.screen = Screen::Recipes;
    app.workspace.recipes.push(yoctui_model::Recipe {
        name: "demo".into(),
        ..yoctui_model::Recipe::default()
    });
    app.recipe_metadata.insert(
        "demo".into(),
        yoctui_model::RecipeMetadata {
            recipe: "demo".into(),
            patches: Some(vec!["file://unresolved.patch".into()]),
            ..yoctui_model::RecipeMetadata::default()
        },
    );
    let output = rendered_text(&app, 160, 36);
    assert!(
        output.contains("provider unavailable (provider path not reported)"),
        "{output}"
    );
    assert!(
        output.contains("patches unavailable (remote or")
            && output.contains("unresolved")
            && output.contains("paths)"),
        "{output}"
    );
}

#[test]
fn devtool_metadata_renders_typed_partial_and_disabled_states_responsively() {
    for (width, height) in [(160, 34), (110, 28), (90, 25)] {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Recipes;
        app.focus = FocusTarget::Workspace;
        let file = std::path::PathBuf::from("/layers/core/demo.bb");
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "demo".into(),
            file: Some(file.clone()),
            ..yoctui_model::Recipe::default()
        });
        let identity = yoctui_model::RecipeIdentity {
            name: "demo".into(),
            file,
        };
        app.devtool_statuses.insert(
            identity.clone(),
            yoctui_model::DevtoolStatus {
                identity,
                capability: yoctui_model::DevtoolCapability::Available,
                workspace: yoctui_model::DevtoolWorkspace::NotMember,
                git: yoctui_model::DevtoolGitState::NotApplicable,
                error: None,
            },
        );
        let output = rendered_text(&app, width, height);
        if width >= 80 && height >= 24 {
            assert!(output.contains("not in workspace"), "{output}");
            assert!(output.contains("u update: disabled"), "{output}");
        }
    }
}
