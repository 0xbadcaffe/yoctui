use super::*;

#[test]
fn ux_dependency_graph_renders_typed_partial_paths_and_responsive_modes() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Dependencies;
    app.focus = FocusTarget::Workspace;
    let root = DependencyNodeId::recipe("image");
    let task = DependencyNodeId::task("busybox", "do_compile");
    let orphan = DependencyNodeId::recipe("orphan");
    let (graph, _) = DependencyGraph::normalize(
        root.clone(),
        vec![
            yoctui_model::DependencyNode {
                id: task.clone(),
                provider: Some("/layers/meta/busybox.bb".into()),
                log: Some("/build/tmp/log.do_compile".into()),
            },
            yoctui_model::DependencyNode::identity(orphan.clone()),
        ],
        vec![
            yoctui_model::DependencyEdge {
                from: root.clone(),
                to: task.clone(),
                kind: DependencyEdgeKind::Task,
            },
            yoctui_model::DependencyEdge {
                from: task.clone(),
                to: root.clone(),
                kind: DependencyEdgeKind::Task,
            },
        ],
        100,
        100,
    );
    app.dependency_graph = DependencyGraphState::Partial {
        graph,
        limitations: vec!["runtime edges unavailable".into()],
    };
    app.dependency_graph_selection = Some(task);
    let output = rendered_text(&app, 160, 36);
    assert!(output.contains("Dependency topology"));
    assert!(output.contains("busybox:do_compile"));
    assert!(output.contains("runtime edges unavailable"));
    assert!(output.contains("--task-->"));
    assert!(output.contains("/layers/meta/busybox.bb"));
    assert!(output.contains("Reverse / incoming"));
    assert!(rendered_text(&app, 110, 24).contains("Dependency tree"));
    assert!(rendered_text(&app, 80, 24).contains("Dependency table"));

    app.focus = FocusTarget::Workspace;
    app.dependency_graph_selection = Some(orphan);
    let output = rendered_text(&app, 160, 36);
    assert!(output.contains("unreachable from root"));

    app.focus = FocusTarget::Workspace;
    app.dependency_graph = DependencyGraphState::NotLoaded;
    assert!(rendered_text(&app, 80, 24).contains("not loaded"));
    app.dependency_graph = DependencyGraphState::Loading { root: root.clone() };
    assert!(rendered_text(&app, 80, 24).contains("Stale rows are hidden"));
    app.dependency_graph = DependencyGraphState::AvailableEmpty { root: root.clone() };
    assert!(rendered_text(&app, 80, 24).contains("No dependency edges reported"));
    app.dependency_graph = DependencyGraphState::Failed {
        root,
        message: "server unavailable".into(),
    };
    assert!(rendered_text(&app, 80, 24).contains("server unavailable"));
}
#[test]
fn ux_dependency_graph_reports_path_bounds_without_panicking() {
    let root = DependencyNodeId::recipe("node-0");
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for index in 1..=66 {
        let previous = DependencyNodeId::recipe(format!("node-{}", index - 1));
        let current = DependencyNodeId::recipe(format!("node-{index}"));
        nodes.push(yoctui_model::DependencyNode::identity(current.clone()));
        edges.push(yoctui_model::DependencyEdge {
            from: previous,
            to: current,
            kind: DependencyEdgeKind::Build,
        });
    }
    let selected = DependencyNodeId::recipe("node-66");
    let (graph, _) = DependencyGraph::normalize(root, nodes, edges, 100, 100);
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Dependencies;
    app.focus = FocusTarget::Inspector;
    app.dependency_graph = DependencyGraphState::Available(graph);
    app.dependency_graph_selection = Some(selected);
    assert!(rendered_text(&app, 80, 24).contains("path limit reached"));
}
#[test]
fn ux_dependency_graph_ascii_and_unicode_rows_preserve_relationship_and_position_text() {
    let row = yoctui_model::DependencyProjectionRow {
        id: DependencyNodeId::task("busybox", "do_compile"),
        parent: Some(DependencyNodeId::recipe("image")),
        edge_kind: Some(DependencyEdgeKind::Task),
        depth: 2,
        source_index: 3,
        has_children: true,
        collapsed: false,
    };
    let unicode = dependency_tree_label(&row, true, false);
    let ascii = dependency_tree_label(&row, true, true);
    for text in [&unicode, &ascii] {
        assert!(text.contains("[task]"));
        assert!(text.contains("busybox:do_compile"));
        assert!(text.starts_with("> "));
    }
    assert!(unicode.contains("└─"));
    assert!(ascii.contains("`-"));
}
#[test]
fn dashboard_renders_colored_task_progress_labels() {
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.tasks.insert(
        yoctui_model::TaskId("busybox:do_compile".into()),
        yoctui_model::TaskInfo {
            id: yoctui_model::TaskId("busybox:do_compile".into()),
            recipe: "busybox".into(),
            task: "do_compile".into(),
            progress: Some(42),
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
    assert!(output.contains("do_compile"));
    assert!(output.contains("busybox"));
    assert!(output.contains("42%"));
}

#[test]
fn task_progress_renders_determinate_bar_in_tasks_workspace() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.tasks.insert(
        yoctui_model::TaskId("busybox:do_compile".into()),
        yoctui_model::TaskInfo {
            id: yoctui_model::TaskId("busybox:do_compile".into()),
            recipe: "busybox".into(),
            task: "do_compile".into(),
            state: yoctui_model::TaskState::Active,
            progress: Some(42),
            ..yoctui_model::TaskInfo::default()
        },
    );
    let output = rendered_text(&app, 120, 30);
    assert!(output.contains("████▏░░░░░ 42%"), "{output}");
}

#[test]
fn task_table_bounds_row_rendering_to_the_selected_viewport() {
    let mut app = App::new(256, 64_000);
    app.screen = Screen::Tasks;
    for index in 0..120 {
        let id = yoctui_model::TaskId(format!("recipe-{index:03}:do_compile"));
        app.tasks.insert(
            id.clone(),
            yoctui_model::TaskInfo {
                id,
                recipe: format!("recipe-{index:03}"),
                task: format!("compile-{index:03}"),
                state: yoctui_model::TaskState::Active,
                ..yoctui_model::TaskInfo::default()
            },
        );
    }
    app.task_progress_scroll = 119;
    let rows = app.visible_task_row_refs_at(SystemTime::now());
    let mut terminal = Terminal::new(TestBackend::new(100, 15)).unwrap();
    terminal
        .draw(|frame| render_task_table(frame, &app, frame.area(), &rows, SystemTime::now()))
        .unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("compile-119"), "{output}");
    assert!(!output.contains("compile-000"), "{output}");
}

#[test]
fn task_progress_renders_average_velocity_and_eta_when_authoritative() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.build.started = Some(SystemTime::now() - Duration::from_secs(120));
    app.build.completed = 20;
    app.build.total = Some(40);
    let output = rendered_text(&app, 180, 32);
    assert!(output.contains("avg 10.0/m"), "{output}");
    assert!(output.contains("ETA 00:02:00"), "{output}");
}
#[test]
fn dashboard_renders_completed_and_failed_package_tasks() {
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.screen = Screen::Tasks;
    app.completed_tasks.push_back(yoctui_model::CompletedTask {
        task: yoctui_model::TaskInfo {
            id: yoctui_model::TaskId("busybox:do_compile".into()),
            recipe: "busybox".into(),
            task: "do_compile".into(),
            progress: Some(100),
            ..yoctui_model::TaskInfo::default()
        },
        success: true,
    });
    app.completed_tasks.push_back(yoctui_model::CompletedTask {
        task: yoctui_model::TaskInfo {
            id: yoctui_model::TaskId("bash:do_install".into()),
            recipe: "bash".into(),
            task: "do_install".into(),
            progress: Some(100),
            ..yoctui_model::TaskInfo::default()
        },
        success: false,
    });
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("busybox"));
    assert!(output.contains("✓ Succeeded"));
    assert!(output.contains("bash"));
    assert!(output.contains("✕ Failed"));
}
#[test]
fn renders_build_target_editor() {
    let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
    let mut app = App::new(10, 1_000);
    let mut editor = yoctui_model::PopupEditor::new("target = \"core-image-minimal\"\n".into());
    editor.select_toml_value("target").unwrap();
    app.dialogs.push_back(Dialog::BuildTarget {
        editor,
        task: Some("menuconfig".into()),
    });
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Build target.toml"));
    assert!(output.contains("requested task: menuconfig"));
    assert!(output.contains("⟦core-image-minimal⟧▏"));
    assert!(output.contains("Ctrl+V paste"));
}
#[test]
fn renders_machine_aware_build_options() {
    let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
    let mut app = App::new(10, 1_000);
    app.dialogs.push_back(Dialog::BuildOptions);
    app.build.target = Some("core-image-minimal".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemuarm".into());
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Image build options"));
    assert!(output.contains("qemuarm"));
    assert!(output.contains("Clean image"));
}
#[test]
fn logs_identify_evicted_warnings_and_errors() {
    let mut terminal = Terminal::new(TestBackend::new(300, 30)).unwrap();
    let mut app = App::new(1, 1_000);
    app.screen = Screen::Logs;
    app.logs.dropped = 3;
    app.logs.dropped_warnings = 1;
    app.logs.dropped_errors = 2;
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("3 evicted [W 1 E 2]"));
}
#[test]
fn renders_recipe_task_confirmation() {
    let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
    let mut app = App::new(10, 1_000);
    app.dialogs
        .push_back(Dialog::RecipeTaskConfirmation(BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("cleansstate".into()),
            force: false,
        }));
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Confirm recipe task"));
    assert!(output.contains("cleansstate"));
}
#[test]
fn devtool_target_reset_renders_exact_destructive_identity_confirmation() {
    let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
    let mut app = App::new(10, 1_000);
    app.dialogs.push_back(Dialog::DevtoolResetConfirmation(
        yoctui_model::DevtoolResetPlan {
            identity: yoctui_model::RecipeIdentity {
                name: "busybox".into(),
                file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
            },
            source_path: "/build/workspace/sources/busybox".into(),
        },
    ));
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Confirm Devtool reset"));
    assert!(output.contains("devtool reset busybox"));
    assert!(output.contains("busybox.bb"));
    assert!(output.contains("/build/workspace/sources/busybox"));
}
#[test]
fn devtool_publish_update_renders_exact_identity_confirmation() {
    let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
    let mut app = App::new(10, 1_000);
    app.dialogs.push_back(Dialog::DevtoolUpdateConfirmation(
        yoctui_model::RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
        },
    ));
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Confirm Devtool update-recipe"));
    assert!(output.contains("devtool update-recipe busybox"));
    assert!(output.contains("busybox.bb"));
}
#[test]
fn devtool_publish_finish_renders_configured_picker_and_exact_confirmation() {
    let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
    let mut app = App::new(10, 1_000);
    let identity = yoctui_model::RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
    };
    let layer = yoctui_model::Layer {
        name: "meta-demo".into(),
        path: "/layers/meta-demo".into(),
        priority: Some(7),
    };
    app.dialogs.push_back(Dialog::DevtoolFinishPicker(
        yoctui_model::DevtoolFinishPicker {
            identity: identity.clone(),
            layers: vec![layer.clone()],
            selection: 0,
        },
    ));
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Configured layer"));
    assert!(output.contains("meta-demo"));
    assert!(output.contains("/layers/meta-demo"));

    app.dialogs.clear();
    app.dialogs.push_back(Dialog::DevtoolFinishConfirmation(
        yoctui_model::DevtoolFinishPlan { identity, layer },
    ));
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Confirm Devtool finish"));
    assert!(output.contains("devtool finish busybox /layers/meta-demo"));
    assert!(output.contains("busybox.bb"));
    assert!(output.contains("Configured layer: meta-demo"));
}
