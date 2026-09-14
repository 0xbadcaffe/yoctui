//! Regression tests grouped around completed_builds_are_retained_in_session_history.
use super::*;

#[test]
fn completed_builds_are_retained_in_session_history() {
    let mut app = App::new(10, 1_000);
    app.build.target = Some("core-image-minimal".into());
    app.build.completed = 12;
    app.build.warnings = 2;
    app.build.errors = 1;
    app.build.started = Some(SystemTime::now());
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: false,
            exit_code: Some(1),
        },
    );
    assert_eq!(app.build_history.len(), 1);
    assert_eq!(
        app.build_history[0].target.as_deref(),
        Some("core-image-minimal")
    );
    assert!(!app.build_history[0].success);
    assert_eq!(app.build_history[0].completed_tasks, 12);
    assert_eq!(app.build_history[0].errors, 1);
}
#[test]
fn selected_error_jumps_to_exact_log_without_replacing_user_filters() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::Log(tagged_log(
            "busybox",
            "do_compile",
            Severity::Error,
            "compile failed",
        )),
    );
    app.logs.query = "user query".into();
    app.logs.filter = Some(Severity::Warning);
    let _ = update(&mut app, Action::Open(Screen::Errors));
    let _ = update(&mut app, Action::JumpToSelectedError);
    assert_eq!(app.screen, Screen::Logs);
    assert_eq!(app.logs.query, "user query");
    assert_eq!(app.logs.filter, Some(Severity::Warning));
    assert_eq!(
        app.logs.selected().map(|entry| entry.message.as_str()),
        Some("compile failed")
    );
}
#[test]
fn selected_error_opens_its_source_path() {
    let mut app = App::new(10, 1_000);
    let mut entry = tagged_log("busybox", "do_compile", Severity::Error, "compile failed");
    entry.path = Some(PathBuf::from("/tmp/log.do_compile"));
    let _ = update(&mut app, Action::Log(entry));

    assert_eq!(
        update(&mut app, Action::OpenSelectedErrorSource),
        Some(Effect::OpenInEditor(PathBuf::from("/tmp/log.do_compile")))
    );
}
#[test]
fn error_entries_gain_typed_category_summary_metadata_and_suggestions() {
    let mut app = App::new(10, 1_000);
    app.build.target = Some("core-image-minimal".into());
    let mut entry = tagged_log(
        "busybox",
        "do_compile",
        Severity::Error,
        "compile failed\nfull compiler context",
    );
    entry.path = Some(PathBuf::from("/tmp/log.do_compile"));
    let _ = update(&mut app, Action::Log(entry));
    let retained = app.logs.diagnostics().next().unwrap();
    let diagnostic = retained.diagnostic.as_ref().unwrap();
    assert_eq!(diagnostic.category, "BitBake error");
    assert_eq!(diagnostic.summary, "compile failed");
    assert!(
        diagnostic
            .event_metadata
            .iter()
            .any(|(name, value)| name == "build" && value == "core-image-minimal")
    );
    assert!(diagnostic.suggestions.len() >= 2);
    assert_eq!(retained.build.as_deref(), Some("core-image-minimal"));
}
#[test]
fn error_completion_outcomes_are_distinct_and_actionable() {
    let mut success = App::new(10, 1_000);
    let _ = update(
        &mut success,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert!(
        success
            .notification
            .as_deref()
            .is_some_and(|message| message.contains("successfully"))
    );

    let mut warning = App::new(10, 1_000);
    let _ = update(
        &mut warning,
        Action::Log(tagged_log(
            "busybox",
            "do_compile",
            Severity::Warning,
            "deprecated option",
        )),
    );
    let _ = update(
        &mut warning,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert!(
        warning
            .notification
            .as_deref()
            .is_some_and(|message| message.contains("warning"))
    );

    let mut failed = App::new(10, 1_000);
    let _ = update(
        &mut failed,
        Action::Log(tagged_log(
            "busybox",
            "do_compile",
            Severity::Error,
            "compile failed",
        )),
    );
    let _ = update(
        &mut failed,
        Action::BuildCompleted {
            success: false,
            exit_code: Some(1),
        },
    );
    assert!(
        failed
            .notification
            .as_deref()
            .is_some_and(|message| message.contains("Press Enter"))
    );
    let _ = update(&mut failed, Action::OpenBuildCompletionErrors);
    assert_eq!(failed.screen, Screen::Errors);
    assert!(failed.active_dialog().is_none());

    let mut cancelled = App::new(10, 1_000);
    let _ = update(
        &mut cancelled,
        Action::BuildCancelled {
            exit_code: Some(130),
        },
    );
    assert!(
        cancelled
            .notification
            .as_deref()
            .is_some_and(|message| message.contains("distinct"))
    );
}
#[test]
fn recipe_selection_stays_in_workspace_bounds() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes = vec![
        Recipe {
            name: "alpha".into(),
            version: None,
            layer: None,
            ..Recipe::default()
        },
        Recipe {
            name: "beta".into(),
            version: None,
            layer: None,
            ..Recipe::default()
        },
    ];
    let _ = update(&mut app, Action::SelectRecipe { delta: 8 });
    assert_eq!(app.recipe_selection, 1);
    let _ = update(&mut app, Action::SelectRecipe { delta: -8 });
    assert_eq!(app.recipe_selection, 0);
}
#[test]
fn recipe_metadata_refresh_is_typed_and_replaces_stale_detail() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        version: Some("1.36".into()),
        layer: Some("core".into()),
        preferred_version: None,
        file: Some("/layers/meta/recipes-core/busybox/busybox.bb".into()),
        append_count: Some(2),
    });
    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipeMetadata),
        Some(Effect::GetRecipeMetadata("busybox".into()))
    );
    let _ = update(
        &mut app,
        Action::RecipeMetadataLoaded(RecipeMetadata {
            recipe: "busybox".into(),
            workspace_status: None,
            build_status: None,
            tasks: Some(vec!["do_build".into()]),
            sources: Some(vec!["/layers/meta/busybox.bb".into()]),
            patches: Some(vec![]),
            packages: Some(vec!["busybox".into()]),
            history: None,
        }),
    );
    let _ = update(
        &mut app,
        Action::RecipeMetadataLoaded(RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(vec!["do_compile".into()]),
            sources: None,
            ..RecipeMetadata::default()
        }),
    );
    let metadata = &app.recipe_metadata["busybox"];
    assert_eq!(metadata.tasks, Some(vec!["do_compile".into()]));
    assert_eq!(metadata.sources, None);
    assert!(!app.recipe_sources.contains_key("busybox"));
    assert_eq!(metadata.workspace_status, None);
    assert_eq!(metadata.history, None);
}
#[test]
fn recipes_workspace_filter_selection_refresh_and_failure_are_identity_stable() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::RecipesLoaded(vec![
            Recipe {
                name: "alpha".into(),
                version: Some("1".into()),
                layer: Some("core".into()),
                ..Recipe::default()
            },
            Recipe {
                name: "busybox".into(),
                version: Some("1.36".into()),
                layer: Some("base".into()),
                file: Some("/layers/base/recipes-core/busybox.bb".into()),
                ..Recipe::default()
            },
            Recipe {
                name: "zlib".into(),
                version: Some("1.3".into()),
                layer: Some("core".into()),
                ..Recipe::default()
            },
        ]),
    );
    let _ = update(&mut app, Action::BeginMetadataSearch);
    for character in "base".chars() {
        let _ = update(&mut app, Action::AppendMetadataQuery(character));
    }
    assert_eq!(app.recipe_selection, 1);
    let _ = update(&mut app, Action::SelectRecipe { delta: isize::MAX });
    assert_eq!(app.recipe_selection, 1);
    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipeMetadata),
        Some(Effect::GetRecipeMetadata("busybox".into()))
    );
    assert!(app.recipe_metadata_loading.contains("busybox"));
    let _ = update(
        &mut app,
        Action::RecipeMetadataFailed {
            recipe: "busybox".into(),
            message: "server unavailable".into(),
        },
    );
    assert!(!app.recipe_metadata_loading.contains("busybox"));
    assert_eq!(
        app.recipe_metadata_errors
            .get("busybox")
            .map(String::as_str),
        Some("server unavailable")
    );
    app.recipe_metadata_errors
        .insert("alpha".into(), "stale".into());

    let _ = update(
        &mut app,
        Action::RecipesLoaded(vec![
            Recipe {
                name: "busybox".into(),
                version: Some("1.37".into()),
                layer: Some("base".into()),
                ..Recipe::default()
            },
            Recipe {
                name: "new".into(),
                ..Recipe::default()
            },
        ]),
    );
    assert_eq!(app.workspace.recipes[app.recipe_selection].name, "busybox");
    assert_eq!(
        app.recipe_metadata_errors
            .get("busybox")
            .map(String::as_str),
        Some("server unavailable")
    );
    assert!(!app.recipe_metadata_errors.contains_key("alpha"));
}
#[test]
fn recipe_bitbake_action_uses_authoritative_tasks_picker_and_confirmation() {
    let mut app = App::new(10, 1_000);
    app.daemon.status = ClientReplicaStatus::Current;
    app.workspace.build_dir = Some("/work/build".into());
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        ..Recipe::default()
    });
    app.recipe_metadata.insert(
        "busybox".into(),
        RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(
                [
                    "do_clean",
                    "do_cleansstate",
                    "do_devshell",
                    "do_diffconfig",
                    "do_diffsigs",
                    "do_menuconfig",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            ),
            ..RecipeMetadata::default()
        },
    );

    let actions = [
        (Action::BeginSelectedRecipeClean, "clean"),
        (Action::BeginSelectedRecipeCleanState, "cleansstate"),
        (Action::BeginSelectedRecipeDiffconfig, "diffconfig"),
        (Action::BeginSelectedRecipeDiffsigs, "diffsigs"),
    ];
    for (action, expected) in actions {
        app.dialogs.clear();
        let _ = update(&mut app, action);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::RecipeTaskConfirmation(BuildRequest {
                targets,
                task: Some(task),
                force: false,
            })) if targets == &vec!["busybox".to_owned()] && task == expected
        ));
    }
    for (action, expected) in [
        (Action::BeginSelectedRecipeMenuConfig, "menuconfig"),
        (Action::BeginSelectedRecipeDevshell, "devshell"),
    ] {
        app.dialogs.clear();
        assert_eq!(update(&mut app, action), None);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::TerminalLaunch(TerminalLaunchDialog { request, destination: TerminalLaunchDestination::Embedded }))
                if request.name == format!("{expected}:busybox")
        ));
    }

    app.dialogs.clear();
    let _ = update(&mut app, Action::BeginSelectedRecipeForceTask);
    let Some(Dialog::RecipeTaskPicker(picker)) = app.active_dialog() else {
        panic!("authoritative task picker did not open");
    };
    assert!(picker.force);
    assert_eq!(picker.tasks[0], "clean");
    let _ = update(&mut app, Action::SelectRecipeTask { delta: 3 });
    let _ = update(&mut app, Action::PreviewSelectedRecipeTask);
    let Some(Dialog::RecipeTaskConfirmation(request)) = app.active_dialog() else {
        panic!("forced task was not previewed");
    };
    assert!(request.force);
    assert_eq!(request.targets, vec!["busybox"]);
    let request = request.clone();
    assert_eq!(
        update(&mut app, Action::ConfirmRecipeTask),
        Some(Effect::Start(request))
    );
}

#[test]
fn recipe_bitbake_action_rejects_unavailable_and_malformed_tasks() {
    let mut app = App::new(10, 1_000);
    app.daemon.status = ClientReplicaStatus::Current;
    app.workspace.build_dir = Some("/work/build".into());
    app.workspace.recipes.push(Recipe {
        name: "demo".into(),
        ..Recipe::default()
    });
    let _ = update(&mut app, Action::BeginSelectedRecipeDevshell);
    assert_eq!(
        app.notification.as_deref(),
        Some("Load authoritative recipe tasks with Enter before opening an interactive task.")
    );
    app.recipe_metadata.insert(
        "demo".into(),
        RecipeMetadata {
            recipe: "demo".into(),
            tasks: Some(vec!["do_build".into(), "bad task".into()]),
            ..RecipeMetadata::default()
        },
    );
    app.notification = None;
    let _ = update(&mut app, Action::BeginSelectedRecipeMenuConfig);
    assert_eq!(
        app.notification.as_deref(),
        Some("Task menuconfig is not reported for recipe demo.")
    );
    app.notification = None;
    let _ = update(
        &mut app,
        Action::BeginSelectedRecipeTask {
            task: Some("bad task".into()),
            force: true,
        },
    );
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("invalid build target"))
    );
    assert!(app.active_dialog().is_none());
}
#[test]
fn selected_recipe_build_requires_confirmation() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        version: None,
        layer: None,
        ..Recipe::default()
    }];
    let _ = update(&mut app, Action::BeginSelectedRecipeBuild);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::RecipeTaskConfirmation(BuildRequest {
            targets: vec!["busybox".into()],
            task: None,
            force: false,
        }))
    );
}
#[test]
fn selected_recipe_clean_prefills_the_clean_task() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        version: None,
        layer: None,
        ..Recipe::default()
    }];
    app.recipe_metadata.insert(
        "busybox".into(),
        RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(vec!["do_clean".into()]),
            ..RecipeMetadata::default()
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeClean);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskConfirmation(BuildRequest {
            targets,
            task: Some(task),
            force: false,
        })) if targets == &vec!["busybox".to_owned()] && task == "clean"
    ));
}
#[test]
fn selected_recipe_menuconfig_opens_the_embedded_terminal_chooser() {
    let mut app = App::new(10, 1_000);
    app.daemon.status = ClientReplicaStatus::Current;
    app.workspace.build_dir = Some("/work/build".into());
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        version: None,
        layer: None,
        ..Recipe::default()
    }];
    app.recipe_metadata.insert(
        "busybox".into(),
        RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(vec!["do_menuconfig".into()]),
            ..RecipeMetadata::default()
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeMenuConfig);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request: TerminalLaunchRequest {
                name,
                kind: TerminalCreationKind::Menuconfig,
                ..
            },
            destination: TerminalLaunchDestination::Embedded,
        })) if name == "menuconfig:busybox"
    ));
}
#[test]
fn kernel_menuconfig_uses_virtual_provider_and_requires_reported_task() {
    let mut app = App::new(10, 1_000);
    app.daemon.status = ClientReplicaStatus::Current;
    app.workspace.build_dir = Some("/work/build".into());
    app.kernel.inventory = PlatformInventoryState::Available(PlatformInventory {
        component: PlatformComponent::Kernel,
        target: "virtual/kernel".into(),
        provider: Some("/layers/linux.bb".into()),
        tasks: vec!["do_menuconfig".into()],
        roots: vec![],
        files: vec![],
        dtc: None,
        limitations: vec![],
    });
    let _ = update(&mut app, Action::LaunchKernelMenuconfig);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request: TerminalLaunchRequest { arguments, .. },
            destination: TerminalLaunchDestination::Embedded,
        })) if arguments == &vec!["bitbake", "virtual/kernel", "-c", "menuconfig"]
    ));
}
#[test]
fn firmware_menuconfig_uses_detected_provider_and_requires_reported_task() {
    let mut app = App::new(10, 1_000);
    app.daemon.status = ClientReplicaStatus::Current;
    app.workspace.build_dir = Some("/work/build".into());
    app.firmware.inventory = PlatformInventoryState::Available(PlatformInventory {
        component: PlatformComponent::UBoot,
        target: "u-boot-fslc".into(),
        provider: Some("/layers/u-boot-fslc.bb".into()),
        tasks: vec!["do_menuconfig".into()],
        roots: vec![],
        files: vec![],
        dtc: None,
        limitations: vec![],
    });
    let _ = update(&mut app, Action::LaunchFirmwareMenuconfig);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request: TerminalLaunchRequest { arguments, .. },
            destination: TerminalLaunchDestination::Embedded,
        })) if arguments == &vec!["bitbake", "u-boot-fslc", "-c", "menuconfig"]
    ));
}
#[test]
fn devtool_modify_requires_authoritative_status_and_confirmation() {
    let mut app = App::new(10, 1_000);
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
    };
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        file: Some(identity.file.clone()),
        ..Recipe::default()
    }];
    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipeDevtoolModify),
        None
    );
    assert_eq!(
        app.notification.as_deref(),
        Some("Refresh authoritative Devtool status with t before modifying.")
    );
    app.devtool_statuses.insert(
        identity.clone(),
        DevtoolStatus {
            identity: identity.clone(),
            capability: DevtoolCapability::MissingExecutable,
            workspace: DevtoolWorkspace::NotMember,
            git: DevtoolGitState::NotApplicable,
            error: None,
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolModify);
    assert_eq!(
        app.notification.as_deref(),
        Some("Devtool executable is missing.")
    );
    app.devtool_statuses.insert(
        identity.clone(),
        DevtoolStatus {
            identity: identity.clone(),
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::NotMember,
            git: DevtoolGitState::NotApplicable,
            error: None,
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolModify);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::DevtoolModifyConfirmation(identity.clone()))
    );
    assert_eq!(
        update(&mut app, Action::ConfirmDevtoolModify),
        Some(Effect::DevtoolModify(identity.clone()))
    );
    app.devtool_statuses.insert(
        identity.clone(),
        DevtoolStatus {
            identity,
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: PathBuf::from("/build/workspace/sources/busybox"),
                recipe_file: None,
            },
            git: DevtoolGitState::NotRepository,
            error: None,
        },
    );
    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipeDevtoolModify),
        Some(Effect::OpenWorkspaceEditor {
            label: "busybox".into(),
            root: PathBuf::from("/build/workspace/sources/busybox"),
        })
    );
}
#[test]
fn selected_recipe_requests_authoritative_dependencies() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        version: None,
        layer: None,
        ..Recipe::default()
    }];
    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipeDependencies),
        Some(Effect::GetDependencies("busybox".into()))
    );
    let _ = update(
        &mut app,
        Action::DependenciesLoaded(RecipeDependencies {
            recipe: "busybox".into(),
            build: vec!["virtual/libc".into()],
            runtime: vec!["base-files".into()],
        }),
    );
    assert_eq!(app.screen, Screen::Dependencies);
    assert_eq!(app.dependencies.as_ref().unwrap().build, ["virtual/libc"]);
    app.workspace.recipes.push(Recipe {
        name: "base-files".into(),
        version: None,
        layer: None,
        ..Recipe::default()
    });
    let _ = update(&mut app, Action::SelectDependency { delta: 1 });
    let _ = update(&mut app, Action::OpenSelectedDependency);
    assert_eq!(app.screen, Screen::Recipes);
    assert_eq!(app.recipe_selection, 1);
}
#[test]
fn ux_dependency_graph_normalizes_nodes_edges_and_reverse_lookup() {
    let root = DependencyNodeId::recipe("image");
    let library = DependencyNodeId::recipe("library");
    let compile = DependencyNodeId::task("library", "do_compile");
    let duplicate_library = DependencyNode {
        id: library.clone(),
        provider: Some(PathBuf::from("/z/library.bb")),
        log: None,
    };
    let preferred_library = DependencyNode {
        id: library.clone(),
        provider: Some(PathBuf::from("/a/library.bb")),
        log: None,
    };
    let build_edge = DependencyEdge {
        from: root.clone(),
        to: library.clone(),
        kind: DependencyEdgeKind::Build,
    };
    let (graph, report) = DependencyGraph::normalize(
        root.clone(),
        vec![duplicate_library, preferred_library],
        vec![
            build_edge.clone(),
            build_edge,
            DependencyEdge {
                from: library.clone(),
                to: library.clone(),
                kind: DependencyEdgeKind::Runtime,
            },
            DependencyEdge {
                from: library.clone(),
                to: compile.clone(),
                kind: DependencyEdgeKind::Task,
            },
        ],
        10,
        10,
    );

    assert_eq!(report.duplicate_nodes, 1);
    assert_eq!(report.duplicate_edges, 1);
    assert_eq!(report.self_edges, 1);
    assert_eq!(report.synthesized_nodes, 1);
    assert!(!report.is_partial());
    assert_eq!(
        graph
            .nodes
            .iter()
            .find(|node| node.id == library)
            .and_then(|node| node.provider.as_deref()),
        Some(Path::new("/a/library.bb"))
    );
    assert_eq!(graph.incoming(&compile).len(), 1);
    assert_eq!(graph.incoming(&compile)[0].from, library);
}
#[test]
fn ux_dependency_graph_finds_deterministic_bounded_paths_through_cycles() {
    let root = DependencyNodeId::recipe("root");
    let a = DependencyNodeId::recipe("a");
    let b = DependencyNodeId::recipe("b");
    let target = DependencyNodeId::recipe("target");
    let isolated = DependencyNodeId::recipe("isolated");
    let edge = |from: &DependencyNodeId, to: &DependencyNodeId| DependencyEdge {
        from: from.clone(),
        to: to.clone(),
        kind: DependencyEdgeKind::Build,
    };
    let (graph, _) = DependencyGraph::normalize(
        root.clone(),
        vec![DependencyNode::identity(isolated.clone())],
        vec![
            edge(&root, &b),
            edge(&b, &target),
            edge(&root, &a),
            edge(&a, &root),
            edge(&a, &target),
        ],
        20,
        20,
    );

    assert_eq!(
        graph.why_built(&target, 4, 20),
        DependencyPathResult::Found(vec![root.clone(), a, target.clone()])
    );
    assert_eq!(
        graph.why_built(&target, 1, 20),
        DependencyPathResult::LimitReached
    );
    assert_eq!(
        graph.why_built(&target, 4, 1),
        DependencyPathResult::LimitReached
    );
    assert_eq!(
        graph.why_built(&isolated, 4, 20),
        DependencyPathResult::Unreachable
    );
}
#[test]
fn ux_dependency_graph_reducer_preserves_identity_and_explicit_states() {
    let root = DependencyNodeId::recipe("image");
    let selected = DependencyNodeId::recipe("library");
    let edge = DependencyEdge {
        from: root.clone(),
        to: selected.clone(),
        kind: DependencyEdgeKind::Build,
    };
    let (graph, _) = DependencyGraph::normalize(root.clone(), Vec::new(), vec![edge], 10, 10);
    let mut app = App::new(10, 1_000);

    assert_eq!(
        update(
            &mut app,
            Action::BeginDependencyGraph { root: root.clone() }
        ),
        Some(Effect::GetDependencies("image".into()))
    );
    assert_eq!(
        app.dependency_graph,
        DependencyGraphState::Loading { root: root.clone() }
    );
    let _ = update(&mut app, Action::DependencyGraphLoaded(graph.clone()));
    app.dependency_graph_selection = Some(selected.clone());
    let _ = update(
        &mut app,
        Action::DependencyGraphPartial {
            graph: graph.clone(),
            limitations: vec!["task edges unavailable".into()],
        },
    );
    assert_eq!(app.dependency_graph_selection, Some(selected.clone()));
    assert!(matches!(
        app.dependency_graph,
        DependencyGraphState::Partial { .. }
    ));

    let (without_selected, _) =
        DependencyGraph::normalize(root.clone(), Vec::new(), Vec::new(), 10, 10);
    let _ = update(&mut app, Action::DependencyGraphLoaded(without_selected));
    assert_eq!(app.dependency_graph_selection, Some(root.clone()));
    assert_eq!(
        app.dependency_graph,
        DependencyGraphState::AvailableEmpty { root: root.clone() }
    );

    let _ = update(
        &mut app,
        Action::DependencyGraphFailed {
            root: root.clone(),
            message: "backend failed".into(),
        },
    );
    assert_eq!(
        app.dependency_graph,
        DependencyGraphState::Failed {
            root,
            message: "backend failed".into()
        }
    );
}
