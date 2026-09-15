//! Regression tests grouped around ux_dependency_graph_renders_typed_partial_paths_and_responsive_modes.
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
    assert!(output.contains("▪▪▪▪▪▫▫▫▫▫ 42%"), "{output}");
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
#[test]
fn devtool_target_deploy_renders_identity_entry_and_exact_confirmation() {
    let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
    let mut app = App::new(10, 1_000);
    let identity = yoctui_model::RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
    };
    app.dialogs
        .push_back(Dialog::DevtoolDeploy(yoctui_model::DevtoolDeployDraft {
            identity: identity.clone(),
            target: "qemuarm".into(),
        }));
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Devtool deploy target"));
    assert!(output.contains("busybox.bb"));
    assert!(output.contains("qemuarm"));

    app.dialogs.clear();
    app.dialogs.push_back(Dialog::DevtoolDeployConfirmation(
        yoctui_model::DevtoolDeployPlan {
            identity,
            target: "qemuarm".into(),
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
    assert!(output.contains("Confirm Devtool deploy-target"));
    assert!(output.contains("devtool deploy-target busybox qemuarm"));
    assert!(output.contains("busybox.bb"));
}
#[test]
fn devwork_editor_renders_confirmation_and_workspace_editor_build_shortcut() {
    let mut confirmation = App::new(10, 1_000);
    confirmation
        .dialogs
        .push_back(Dialog::DevtoolModifyConfirmation(
            yoctui_model::RecipeIdentity {
                name: "busybox".into(),
                file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
            },
        ));
    let output = rendered_text(&confirmation, 120, 30);
    assert!(output.contains("Confirm Devtool modify"), "{output}");
    assert!(output.contains("devtool modify busybox"), "{output}");
    assert!(output.contains("busybox.bb"), "{output}");

    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let mut app = App::new(10, 1_000);
    app.dialogs.push_back(Dialog::RecipeEditor(RecipeEditor {
        recipe: "busybox".into(),
        root: "/build/workspace/sources/busybox".into(),
        files: vec!["main.c".into()],
        selection: 0,
        focus: yoctui_model::RecipeEditorFocus::Files,
        language: yoctui_model::SourceLanguage::C,
        document: yoctui_model::TextAreaState::new("int main() {}".into()),
        searching: false,
    }));
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Workspace file tree: busybox"));
    assert!(output.contains("int main() {}"));
    assert!(output.contains("Ctrl+B build recipe"));
}
#[test]
fn devwork_terminal_renders_destination_authority_and_zero_spawn_cancel_hint() {
    let mut app = App::new(10, 1_000);
    app.detached_terminal = yoctui_model::DetachedTerminalAvailability::Available {
        launcher: "x-terminal-emulator".into(),
    };
    app.dialogs
        .push_back(Dialog::TerminalLaunch(yoctui_model::TerminalLaunchDialog {
            request: yoctui_model::TerminalLaunchRequest {
                name: "devshell:busybox".into(),
                kind: yoctui_model::TerminalCreationKind::Devshell,
                cwd: "/work/build".into(),
                program: "/usr/bin/env".into(),
                arguments: vec![
                    "bitbake".into(),
                    "busybox".into(),
                    "-c".into(),
                    "devshell".into(),
                ],
            },
            destination: yoctui_model::TerminalLaunchDestination::Embedded,
        }));
    let output = rendered_text(&app, 120, 30);
    assert!(output.contains("Choose terminal destination"), "{output}");
    assert!(output.contains("Embedded in Yoctui"), "{output}");
    assert!(output.contains("x-terminal-emulator"), "{output}");
    assert!(output.contains("bitbake busybox -c devshell"), "{output}");
    assert!(output.contains("cancel without spawning"), "{output}");
}
#[test]
fn ux_list_tree_layer_browser_renders_external_state_and_numbered_preview() {
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Layers;
    app.workspace.layers.push(yoctui_model::Layer {
        name: "meta-demo".into(),
        path: "/layers/meta-demo".into(),
        priority: Some(7),
    });
    app.layer_relationships = Some(yoctui_model::LayerRelationships {
        layers: vec![yoctui_model::LayerRelationship {
            name: "meta-demo".into(),
            compatible: vec!["scarthgap".into()],
            ..yoctui_model::LayerRelationship::default()
        }],
    });
    let mut browser = LayerBrowser::new("meta-demo".into(), "/layers/meta-demo".into());
    browser.entries = vec![yoctui_model::LayerBrowserEntry {
        path: "/layers/meta-demo/conf/layer.conf".into(),
        is_dir: false,
        size: Some(31),
        git: yoctui_model::GitFileState::Modified,
        ..yoctui_model::LayerBrowserEntry::default()
    }];
    browser.preview = "BBFILE_COLLECTIONS += \\\"demo\\\"".into();
    browser.preview_kind = yoctui_model::PreviewKind::Text;
    app.layer_browser = Some(browser);
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Configured layers"));
    assert!(output.contains("meta-demo"));
    assert!(output.contains("hidden off"));
    assert!(output.contains("layer.conf"));
    assert!(output.contains("BBFILE_COLLECTIONS"));
    assert!(output.contains("M"));
    assert!(output.contains("1"));
}

#[test]
fn layer_browser_gives_unused_tree_width_to_the_file_preview() {
    let mut browser = LayerBrowser::new("meta-demo".into(), "/layers/meta-demo".into());
    browser.entries.push(LayerBrowserEntry {
        path: "/layers/meta-demo/conf/layer.conf".into(),
        depth: 1,
        ..LayerBrowserEntry::default()
    });

    let compact = layer_browser_left_width(&browser, 140);
    assert_eq!(compact, 38);
    assert_eq!(140 - compact, 102);

    browser.entries[0].path =
        "/layers/meta-demo/recipes-core/example/a-very-long-recipe-filename.bb".into();
    let expanded = layer_browser_left_width(&browser, 140);
    assert!(expanded > compact);
    assert!(expanded <= 54);
    assert!(140 - expanded >= 86);
}

#[test]
fn ux_layer_browser_tui_tree_widget_preserves_model_identity_viewport_and_ascii() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Layers;
    app.workspace.layers.push(yoctui_model::Layer {
        name: "meta-demo".into(),
        path: "/layers/meta-demo".into(),
        priority: Some(7),
    });
    let mut browser = LayerBrowser::new("meta-demo".into(), "/layers/meta-demo".into());
    let conf = PathBuf::from("/layers/meta-demo/conf");
    browser.expanded.insert(conf.clone());
    browser.entries = vec![
        LayerBrowserEntry {
            path: conf.clone(),
            is_dir: true,
            git: GitFileState::Clean,
            ..LayerBrowserEntry::default()
        },
        LayerBrowserEntry {
            path: conf.join("layer.conf"),
            depth: 1,
            git: GitFileState::Modified,
            ..LayerBrowserEntry::default()
        },
        LayerBrowserEntry {
            path: "/layers/meta-demo/recipes-core".into(),
            is_dir: true,
            git: GitFileState::Untracked,
            ..LayerBrowserEntry::default()
        },
    ];
    browser.selection = 1;

    let indexed = browser.entries.iter().enumerate().collect::<Vec<_>>();
    let projection = layer_tree_widget_projection(&browser, &indexed, true, false).unwrap();
    let mut state = TreeState::default();
    for path in &projection.opened {
        state.open(path.clone());
    }
    state.select(projection.selected.clone().unwrap());
    let flattened = state.flatten(&projection.items);
    assert_eq!(flattened.len(), browser.entries.len());
    assert_eq!(
        state.selected().last(),
        Some(&PathBuf::from("/layers/meta-demo/conf/layer.conf"))
    );
    assert!(
        flattened[1]
            .identifier
            .starts_with(&[conf.clone(), conf.join("layer.conf")])
    );

    app.layer_browser = Some(browser.clone());
    let unicode = rendered_text(&app, 120, 30);
    assert!(unicode.contains("▾ conf/"), "{unicode}");
    assert!(unicode.contains("layer.conf M"), "{unicode}");
    assert!(unicode.contains("▸ recipes-core/ ?"), "{unicode}");

    app.preferences.symbols = SymbolPreference::Ascii;
    app.color_enabled = false;
    let ascii = rendered_text(&app, 120, 30);
    assert!(ascii.contains("- conf/"), "{ascii}");
    assert!(ascii.contains("+ recipes-core/ ?"), "{ascii}");

    let browser = app.layer_browser.as_mut().unwrap();
    browser.entries = (0..40)
        .map(|index| LayerBrowserEntry {
            path: format!("/layers/meta-demo/file-{index:02}.bb").into(),
            ..LayerBrowserEntry::default()
        })
        .collect();
    browser.selection = 39;
    let bottom = rendered_text(&app, 120, 30);
    assert!(bottom.contains("file-39.bb"), "{bottom}");
    assert!(!bottom.contains("file-00.bb"), "{bottom}");

    let browser = app.layer_browser.as_mut().unwrap();
    browser.entries = vec![
        LayerBrowserEntry {
            path: "/layers/meta-demo/duplicate".into(),
            ..LayerBrowserEntry::default()
        },
        LayerBrowserEntry {
            path: "/layers/meta-demo/duplicate".into(),
            ..LayerBrowserEntry::default()
        },
    ];
    browser.selection = 0;
    let malformed = rendered_text(&app, 120, 30);
    assert!(malformed.contains("Layer tree unavailable"), "{malformed}");
}

#[test]
fn layer_tree_binary_preview_and_responsive_modes_never_render_bytes() {
    for (width, height) in [(160, 30), (110, 28), (90, 25), (70, 20)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Layers;
        app.focus = FocusTarget::Workspace;
        let mut browser = LayerBrowser::new("meta-binary".into(), "/layers/meta-binary".into());
        browser.entries.push(yoctui_model::LayerBrowserEntry {
            path: "/layers/meta-binary/image.bin".into(),
            size: Some(100_000),
            git: yoctui_model::GitFileState::Unavailable,
            ..yoctui_model::LayerBrowserEntry::default()
        });
        browser.preview = "\0secret".into();
        browser.preview_kind = yoctui_model::PreviewKind::Binary;
        browser.preview_truncated = true;
        app.layer_browser = Some(browser);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(!output.contains("secret"));
        if width >= 80 && height >= 24 {
            assert!(output.contains("Binary preview unavailable"));
        }
    }
}
#[test]
fn bitbake_preview_highlights_assignments_and_comments() {
    let app = App::new(10, 1_000);
    let preview = source_preview("SUMMARY = \"demo\" # explanation", "demo.bb", &app);
    assert_eq!(
        preview.lines[0].spans[0].style.fg,
        Some(Color::Rgb(226, 170, 0))
    );
    assert_eq!(
        preview.lines[0].spans[1].style.fg,
        Some(Color::Rgb(176, 126, 214))
    );
    assert_eq!(
        preview.lines[0].spans[2].style.fg,
        Some(Color::Rgb(139, 211, 0))
    );
    assert_eq!(
        preview.lines[0].spans[3].style.fg,
        Some(Color::Rgb(155, 166, 172))
    );
}

#[test]
fn device_tree_preview_highlights_directives_nodes_properties_values_and_comments() {
    let app = App::new(10, 1_000);
    let preview = source_preview(
        "#include \"soc.dtsi\"\n/dts-v1/;\nuart0: serial@1000 {\n  compatible = \"http://vendor/\\\"device\"; // UART\n  interrupt-controller;\n  /* retained */ status = \"okay\";\n};",
        "board.dts",
        &app,
    );
    let styled = preview
        .lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .filter_map(|span| span.style.fg.map(|color| (span.content.as_ref(), color)))
        .collect::<Vec<_>>();

    assert!(styled.iter().any(|(text, _)| *text == "/dts-v1/"));
    assert!(styled.iter().any(|(text, _)| *text == "#include"));
    assert!(styled.iter().any(|(text, _)| *text == "uart0"));
    assert!(styled.iter().any(|(text, _)| *text == "compatible"));
    assert!(
        styled
            .iter()
            .any(|(text, _)| *text == "\"http://vendor/\\\"device\"")
    );
    assert!(
        styled
            .iter()
            .any(|(text, _)| *text == "interrupt-controller")
    );
    assert!(
        styled
            .iter()
            .any(|(text, color)| { *text == "// UART" && *color == Color::Rgb(155, 166, 172) })
    );
    assert!(
        styled.iter().any(|(text, color)| {
            *text == "/* retained */" && *color == Color::Rgb(155, 166, 172)
        })
    );
}

#[test]
fn device_tree_compile_dialog_renders_typed_options_and_derived_paths() {
    let mut app = App::new(10, 1_000);
    app.dialogs
        .push_back(Dialog::DtcCompile(yoctui_model::DtcCompileDialog::new(
            yoctui_model::PlatformComponent::Kernel,
            &yoctui_model::PlatformFile {
                path: "/workspace/kernel/board.dts".into(),
                root: "/workspace/kernel".into(),
                kind: yoctui_model::PlatformFileKind::Dts,
                size_bytes: 64,
            },
            "/toolchain/bin/dtc".into(),
        )));

    let output = rendered_text(&app, 120, 30);
    for expected in [
        "Compile device tree",
        "board.dts",
        "board.yoctui.dtb",
        "Generate symbols (-@)",
        "Stable sort (-s)",
        "Output padding (-p)",
        "Reserve entries (-R)",
        "Enter review launch",
    ] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }
    assert!(rendered_text(&app, 80, 24).contains("Compile device tree"));
    if let Some(Dialog::DtcCompile(dialog)) = app.dialogs.front_mut() {
        dialog.source = PathBuf::from(format!("/workspace/{}/board.dts", "nested/".repeat(30)));
        dialog.output = PathBuf::from(format!(
            "/workspace/{}/board.yoctui.dtb",
            "nested/".repeat(30)
        ));
    }
    assert!(rendered_text(&app, 80, 24).contains("Enter review launch"));
    let _ = rendered_text(&app, 40, 10);
}
#[test]
fn renders_image_picker_for_active_machine() {
    let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
    let mut app = App::new(10, 1_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.dialogs
        .push_back(Dialog::ImagePicker(yoctui_model::ImagePicker {
            images: vec!["core-image-minimal".into()],
            selection: 0,
        }));
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Available image targets"));
    assert!(output.contains("qemux86-64"));
    assert!(output.contains("core-image-minimal"));
}
#[test]
fn inspector_reflects_selected_recipe_and_layer_preview() {
    let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Recipes;
    app.workspace.recipes.push(yoctui_model::Recipe {
        name: "busybox".into(),
        version: Some("1.36".into()),
        layer: Some("meta".into()),
        ..yoctui_model::Recipe::default()
    });
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Recipe: busybox"));
    assert!(output.contains("Resolved version: 1.36"));

    app.screen = Screen::Layers;
    let mut browser = LayerBrowser::new("meta".into(), "/layers/meta".into());
    browser.directory = "/layers/meta/conf".into();
    browser.entries.push(yoctui_model::LayerBrowserEntry {
        path: "/layers/meta/conf/layer.conf".into(),
        ..yoctui_model::LayerBrowserEntry::default()
    });
    browser.preview = "BBFILE_COLLECTIONS += \"meta\"".into();
    browser.preview_kind = yoctui_model::PreviewKind::Text;
    app.layer_browser = Some(browser);
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("/layers/meta/conf/layer.conf"));
    assert!(output.contains("BBFILE_COLLECTIONS"));
}
