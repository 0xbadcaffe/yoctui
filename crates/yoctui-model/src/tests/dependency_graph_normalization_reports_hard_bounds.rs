//! Regression tests grouped around ux_dependency_graph_normalization_reports_hard_bounds.
use super::*;

#[test]
fn ux_dependency_graph_normalization_reports_hard_bounds() {
    let root = DependencyNodeId::recipe("root");
    let (graph, report) = DependencyGraph::normalize(
        root.clone(),
        vec![
            DependencyNode::identity(DependencyNodeId::recipe("a")),
            DependencyNode::identity(DependencyNodeId::recipe("b")),
        ],
        vec![DependencyEdge {
            from: root.clone(),
            to: DependencyNodeId::recipe("missing"),
            kind: DependencyEdgeKind::Runtime,
        }],
        1,
        1,
    );
    assert_eq!(graph.nodes, [DependencyNode::identity(root)]);
    assert!(graph.edges.is_empty());
    assert!(report.truncated_nodes >= 2);
    assert_eq!(report.truncated_edges, 1);
    assert!(report.is_partial());
}
#[test]
fn ux_dependency_graph_projection_bounds_cycles_filters_reverse_and_collapse() {
    let root = DependencyNodeId::recipe("root");
    let a = DependencyNodeId::recipe("a");
    let b = DependencyNodeId::recipe("b");
    let detached = DependencyNodeId::recipe("detached");
    let edge = |from: &DependencyNodeId, to: &DependencyNodeId| DependencyEdge {
        from: from.clone(),
        to: to.clone(),
        kind: DependencyEdgeKind::Build,
    };
    let (graph, _) = DependencyGraph::normalize(
        root.clone(),
        vec![DependencyNode::identity(detached.clone())],
        vec![edge(&root, &a), edge(&a, &b), edge(&b, &a)],
        20,
        20,
    );

    let projection = graph.project(&root, false, "", &BTreeSet::new(), 64, 20);
    assert_eq!(
        projection
            .rows
            .iter()
            .map(|row| row.id.clone())
            .collect::<Vec<_>>(),
        [root.clone(), a.clone(), b.clone(), detached.clone()]
    );
    assert!(projection.cycle_edges >= 1);
    assert_eq!(projection.source_total, 4);
    assert_eq!(projection.rows[2].depth, 2);

    let collapsed = BTreeSet::from([a.clone()]);
    let projection = graph.project(&root, false, "", &collapsed, 64, 20);
    assert!(!projection.rows.iter().any(|row| row.id == b));
    assert!(projection.hidden_by_collapse >= 1);
    assert!(projection.rows.iter().any(|row| row.id == detached));

    let filtered = graph.project(&root, false, "b", &BTreeSet::new(), 64, 20);
    assert_eq!(
        filtered
            .rows
            .iter()
            .map(|row| row.id.clone())
            .collect::<Vec<_>>(),
        [root.clone(), b.clone()]
    );
    assert_eq!(filtered.rows[1].source_index, 1);

    let reverse = graph.project(&b, true, "", &BTreeSet::new(), 64, 2);
    assert_eq!(reverse.anchor, b);
    assert!(reverse.reverse);
    assert_eq!(reverse.rows.len(), 2);
    assert!(reverse.truncated_rows > 0);
}
#[test]
fn ux_dependency_graph_reducer_owns_filter_reverse_expansion_and_exact_selection() {
    let root = DependencyNodeId::recipe("root");
    let child = DependencyNodeId::recipe("child");
    let (graph, _) = DependencyGraph::normalize(
        root.clone(),
        Vec::new(),
        vec![DependencyEdge {
            from: root.clone(),
            to: child.clone(),
            kind: DependencyEdgeKind::Runtime,
        }],
        10,
        10,
    );
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::DependencyGraphLoaded(graph));
    let _ = update(
        &mut app,
        Action::SelectDependencyGraphNodeAt {
            identity: child.clone(),
        },
    );
    let _ = update(&mut app, Action::ToggleDependencyGraphReverse);
    assert!(app.dependency_graph_reverse);
    assert_eq!(app.dependency_graph_anchor, Some(child.clone()));

    let _ = update(&mut app, Action::CollapseSelectedDependencyGraphNode);
    assert!(app.dependency_graph_collapsed.contains(&child));
    let _ = update(&mut app, Action::ExpandSelectedDependencyGraphNode);
    assert!(!app.dependency_graph_collapsed.contains(&child));
    let _ = update(&mut app, Action::BeginDependencyGraphSearch);
    let _ = update(&mut app, Action::AppendDependencyGraphQuery('c'));
    let _ = update(&mut app, Action::AppendDependencyGraphQuery('h'));
    assert_eq!(app.dependency_graph_query, "ch");
    for _ in 0..DEPENDENCY_GRAPH_MAX_QUERY_BYTES {
        let _ = update(&mut app, Action::AppendDependencyGraphQuery('x'));
    }
    assert_eq!(
        app.dependency_graph_query.len(),
        DEPENDENCY_GRAPH_MAX_QUERY_BYTES
    );
    assert!(app.dependency_graph_searching);
    let _ = update(&mut app, Action::FinishDependencyGraphSearch);
    assert!(!app.dependency_graph_searching);
    assert_eq!(app.dependency_graph_selection, Some(child));
}
#[test]
fn ux_dependency_graph_routes_only_typed_identity_provider_and_task_log() {
    let root = DependencyNodeId::recipe("image");
    let task = DependencyNodeId::task("busybox", "do_compile");
    let (graph, _) = DependencyGraph::normalize(
        root.clone(),
        vec![DependencyNode {
            id: task.clone(),
            provider: Some(PathBuf::from("/layers/meta/busybox.bb")),
            log: Some(PathBuf::from("/build/tmp/log.do_compile")),
        }],
        vec![DependencyEdge {
            from: root.clone(),
            to: task.clone(),
            kind: DependencyEdgeKind::Task,
        }],
        10,
        10,
    );
    let mut app = App::new(10, 1_000);
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        ..Recipe::default()
    }];
    let _ = update(&mut app, Action::DependencyGraphLoaded(graph));
    app.dependency_graph_selection = Some(task);

    assert_eq!(
        update(&mut app, Action::OpenSelectedDependencyProvider),
        Some(Effect::OpenInEditor(PathBuf::from(
            "/layers/meta/busybox.bb"
        )))
    );
    assert_eq!(
        update(&mut app, Action::OpenSelectedDependencyTaskLog),
        Some(Effect::OpenInEditor(PathBuf::from(
            "/build/tmp/log.do_compile"
        )))
    );
    let _ = update(&mut app, Action::OpenSelectedDependencyRecipe);
    assert_eq!(app.screen, Screen::Recipes);
    assert_eq!(app.recipe_selection, 0);

    let _ = update(&mut app, Action::Open(Screen::Dependencies));
    assert_eq!(
        update(&mut app, Action::RefreshDependencyGraph),
        Some(Effect::GetDependencies("image".into()))
    );
    assert_eq!(
        app.dependency_graph,
        DependencyGraphState::Loading { root: root.clone() }
    );
    assert_eq!(app.dependency_graph_selection, Some(root));
    let _ = update(&mut app, Action::OpenSelectedDependencyTaskLog);
    assert_eq!(
        app.notification.as_deref(),
        Some("Task logs are available only for typed task dependency nodes.")
    );
}
#[test]
fn signature_model_validates_normalizes_duplicates_and_bounds() {
    let target = SignatureTarget {
        recipe: "busybox".into(),
        task: "do_compile".into(),
    };
    let mut preferred = signature_record("busybox", "do_compile", "aaa", "/tmp/aaa.sigdata");
    preferred.variables = vec![
        SignatureValue {
            name: "Z".into(),
            value: Some("last".into()),
        },
        SignatureValue {
            name: "A".into(),
            value: Some("first".into()),
        },
        SignatureValue {
            name: "A".into(),
            value: Some("second".into()),
        },
    ];
    preferred.dependencies = vec!["z".into(), "a".into(), "a".into()];
    let mut duplicate = preferred.clone();
    duplicate.base_hash = Some("zzz".into());
    let invalid = signature_record("other", "do_compile", "bad", "/tmp/bad.sigdata");
    let overflow = signature_record("busybox", "do_compile", "ccc", "/tmp/ccc.sigdata");

    let (records, report) =
        normalize_signature_records(&target, vec![duplicate, invalid, overflow, preferred], 1);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].base_hash.as_deref(), Some("base-aaa"));
    assert_eq!(
        records[0].variables,
        [
            SignatureValue {
                name: "A".into(),
                value: Some("first".into())
            },
            SignatureValue {
                name: "Z".into(),
                value: Some("last".into())
            }
        ]
    );
    assert_eq!(records[0].dependencies, ["a", "z"]);
    assert_eq!(report.duplicate_records, 1);
    assert_eq!(report.invalid_records, 1);
    assert_eq!(report.truncated_records, 1);
    assert!(report.is_partial());

    let relative = SignatureIdentity {
        target,
        hash: Some("abc".into()),
        path: Some(PathBuf::from("relative.sigdata")),
    };
    assert_eq!(relative.validate(), Err("signature paths must be absolute"));
}
#[test]
fn signature_model_derives_deterministic_typed_differences() {
    let mut left = signature_record("busybox", "do_compile", "left", "/tmp/left.sigdata");
    left.base_hash = Some("base-left".into());
    left.task_hash = Some("task-left".into());
    left.variables = vec![
        SignatureValue {
            name: "CC".into(),
            value: Some("gcc".into()),
        },
        SignatureValue {
            name: "ONLY_LEFT".into(),
            value: Some("yes".into()),
        },
    ];
    left.dependencies = vec!["dep-left".into(), "dep-shared".into()];
    let mut right = signature_record("busybox", "do_compile", "right", "/tmp/right.sigdata");
    right.base_hash = Some("base-right".into());
    right.task_hash = None;
    right.variables = vec![SignatureValue {
        name: "CC".into(),
        value: Some("clang".into()),
    }];
    right.dependencies = vec!["dep-right".into(), "dep-shared".into()];

    let (differences, report) = compare_signature_records(&left, &right, 20);
    assert!(!report.is_partial());
    assert!(differences.iter().any(|difference| {
        difference.category == SignatureDifferenceCategory::BaseHash
            && difference.key == "base_hash"
    }));
    assert!(differences.iter().any(|difference| {
        difference.category == SignatureDifferenceCategory::ChangedValue && difference.key == "CC"
    }));
    assert!(differences.iter().any(|difference| {
        difference.category == SignatureDifferenceCategory::Unavailable
            && difference.key == "ONLY_LEFT"
    }));
    assert_eq!(
        differences
            .iter()
            .filter(|difference| { difference.category == SignatureDifferenceCategory::Dependency })
            .count(),
        2
    );
    let (_, bounded) = compare_signature_records(&left, &right, 2);
    assert!(bounded.is_partial());
}
#[test]
fn signature_model_reducer_correlates_states_selection_and_comparison() {
    let target = SignatureTarget {
        recipe: "busybox".into(),
        task: "do_compile".into(),
    };
    let left = signature_record("busybox", "do_compile", "aaa", "/tmp/aaa.sigdata");
    let right = signature_record("busybox", "do_compile", "bbb", "/tmp/bbb.sigdata");
    let mut app = App::new(10, 1_000);
    assert_eq!(
        update(&mut app, Action::BeginSignatureDump(target.clone())),
        Some(Effect::GetSignatureDump(target.clone()))
    );
    let stale_target = SignatureTarget {
        recipe: "other".into(),
        task: "do_compile".into(),
    };
    let _ = update(
        &mut app,
        Action::SignatureDumpLoaded {
            target: stale_target,
            records: vec![left.clone()],
        },
    );
    assert!(matches!(
        app.signature_dump,
        SignatureDumpState::Loading { .. }
    ));
    let _ = update(
        &mut app,
        Action::SignatureDumpLoaded {
            target: target.clone(),
            records: vec![right.clone(), left.clone()],
        },
    );
    assert_eq!(app.signature_selection, Some(left.identity.clone()));
    assert!(matches!(
        app.signature_dump,
        SignatureDumpState::Available { .. }
    ));

    let _ = update(
        &mut app,
        Action::SetSelectedSignatureComparisonSide(SignatureComparisonSide::Left),
    );
    let _ = update(&mut app, Action::SelectSignatureRecord { delta: 1 });
    assert_eq!(app.signature_selection, Some(right.identity.clone()));
    let _ = update(
        &mut app,
        Action::SetSelectedSignatureComparisonSide(SignatureComparisonSide::Right),
    );
    let request = SignatureComparisonRequest {
        left: left.identity.clone(),
        right: right.identity.clone(),
    };
    assert_eq!(
        update(&mut app, Action::BeginSignatureComparison),
        Some(Effect::CompareSignatures(request.clone()))
    );
    let stale_request = SignatureComparisonRequest {
        left: right.identity.clone(),
        right: left.identity.clone(),
    };
    let _ = update(
        &mut app,
        Action::SignatureComparisonLoaded {
            request: stale_request,
            differences: Vec::new(),
        },
    );
    assert!(matches!(
        app.signature_comparison,
        SignatureComparisonState::Loading { .. }
    ));
    let _ = update(
        &mut app,
        Action::SignatureComparisonLoaded {
            request: request.clone(),
            differences: vec![SignatureDifference {
                category: SignatureDifferenceCategory::ChangedValue,
                key: "CC".into(),
                left: Some("gcc".into()),
                right: Some("clang".into()),
            }],
        },
    );
    assert!(matches!(
        app.signature_comparison,
        SignatureComparisonState::Available { .. }
    ));
    let _ = update(
        &mut app,
        Action::SetSelectedSignatureComparisonSide(SignatureComparisonSide::Left),
    );
    assert!(matches!(
        app.signature_comparison,
        SignatureComparisonState::Ready { .. }
    ));

    let _ = update(&mut app, Action::BeginSignatureDump(target.clone()));
    let _ = update(
        &mut app,
        Action::SignatureDumpPartial {
            target: target.clone(),
            records: vec![right.clone()],
            limitations: vec!["one artifact unreadable".into()],
        },
    );
    assert_eq!(app.signature_selection, Some(right.identity));
    assert!(matches!(
        app.signature_dump,
        SignatureDumpState::Partial { .. }
    ));
    let _ = update(&mut app, Action::BeginSignatureDump(target.clone()));
    let _ = update(
        &mut app,
        Action::SignatureDumpLoaded {
            target: target.clone(),
            records: Vec::new(),
        },
    );
    assert_eq!(
        app.signature_dump,
        SignatureDumpState::AvailableEmpty {
            target: target.clone()
        }
    );
    let _ = update(&mut app, Action::BeginSignatureDump(target.clone()));
    let _ = update(
        &mut app,
        Action::SignatureDumpFailed {
            target: target.clone(),
            message: "tool unavailable".into(),
        },
    );
    assert_eq!(
        app.signature_dump,
        SignatureDumpState::Failed {
            target,
            message: "tool unavailable".into()
        }
    );
}
#[test]
fn devtool_target_reset_requires_authoritative_removable_source_and_confirmation() {
    let mut app = App::new(10, 1_000);
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
    };
    let source_path = PathBuf::from("/build/workspace/sources/busybox");
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        file: Some(identity.file.clone()),
        ..Recipe::default()
    }];
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolReset);
    assert_eq!(
        app.notification.as_deref(),
        Some("Refresh authoritative Devtool status with t before reset.")
    );
    app.devtool_statuses.insert(
        identity.clone(),
        DevtoolStatus {
            identity: identity.clone(),
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: source_path.clone(),
                recipe_file: Some(identity.file.clone()),
            },
            git: DevtoolGitState::NotRepository,
            error: None,
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolReset);
    let plan = DevtoolResetPlan {
        identity: identity.clone(),
        source_path: source_path.clone(),
    };
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::DevtoolResetConfirmation(plan.clone()))
    );
    app.devtool_statuses.get_mut(&identity).unwrap().workspace =
        DevtoolWorkspace::MissingDirectory {
            source_path: PathBuf::from("/build/workspace/sources/moved"),
        };
    assert_eq!(update(&mut app, Action::ConfirmDevtoolReset), None);
    assert_eq!(
        app.notification.as_deref(),
        Some("The authoritative Devtool reset source changed; refresh with t.")
    );
    app.devtool_statuses.get_mut(&identity).unwrap().workspace =
        DevtoolWorkspace::MissingDirectory { source_path };
    assert_eq!(
        update(&mut app, Action::ConfirmDevtoolReset),
        Some(Effect::DevtoolReset(plan))
    );
}
#[test]
fn devtool_publish_update_requires_authoritative_workspace_and_confirmation() {
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
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolUpdateRecipe);
    assert_eq!(
        app.notification.as_deref(),
        Some("Refresh authoritative Devtool status with t before update-recipe.")
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
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolUpdateRecipe);
    assert_eq!(
        app.notification.as_deref(),
        Some("Recipe is not in the Devtool workspace.")
    );
    app.devtool_statuses.insert(
        identity.clone(),
        DevtoolStatus {
            identity: identity.clone(),
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: PathBuf::from("/build/workspace/sources/busybox"),
                recipe_file: Some(identity.file.clone()),
            },
            git: DevtoolGitState::NotRepository,
            error: None,
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolUpdateRecipe);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::DevtoolUpdateConfirmation(identity.clone()))
    );
    assert_eq!(
        update(&mut app, Action::ConfirmDevtoolUpdateRecipe),
        Some(Effect::DevtoolUpdateRecipe(identity))
    );
}
#[test]
fn devtool_publish_finish_requires_clean_status_and_configured_layer_confirmation() {
    let mut app = App::new(10, 1_000);
    let destination = Layer {
        name: "meta-demo".into(),
        path: PathBuf::from("/layers/meta-demo"),
        priority: Some(7),
    };
    app.workspace.layers = vec![
        Layer {
            name: "meta-core".into(),
            path: PathBuf::from("/layers/meta-core"),
            priority: Some(5),
        },
        destination.clone(),
        Layer {
            name: "relative".into(),
            path: PathBuf::from("layers/relative"),
            priority: None,
        },
    ];
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: PathBuf::from("/layers/meta-core/recipes-core/busybox/busybox.bb"),
    };
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        layer: Some("meta-demo".into()),
        file: Some(identity.file.clone()),
        ..Recipe::default()
    }];
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolFinish);
    assert_eq!(
        app.notification.as_deref(),
        Some("Refresh authoritative Devtool status with t before finish.")
    );
    app.devtool_statuses.insert(
        identity.clone(),
        DevtoolStatus {
            identity: identity.clone(),
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: PathBuf::from("/build/workspace/sources/busybox"),
                recipe_file: Some(identity.file.clone()),
            },
            git: DevtoolGitState::Available {
                branch: Some("devtool".into()),
                head: Some("abc123".into()),
                modified: 1,
                untracked: 0,
                conflicted: 0,
            },
            error: None,
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolFinish);
    assert_eq!(
        app.notification.as_deref(),
        Some("Commit all workspace changes before Devtool finish.")
    );
    app.devtool_statuses.get_mut(&identity).unwrap().git = DevtoolGitState::Available {
        branch: Some("devtool".into()),
        head: Some("abc123".into()),
        modified: 0,
        untracked: 0,
        conflicted: 0,
    };
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolFinish);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::DevtoolFinishPicker(picker))
            if picker.identity == identity
                && picker.layers.len() == 2
                && picker.layers[picker.selection] == destination
    ));
    let _ = update(&mut app, Action::PreviewDevtoolFinish);
    let plan = DevtoolFinishPlan {
        identity: identity.clone(),
        layer: destination.clone(),
    };
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::DevtoolFinishConfirmation(plan.clone()))
    );
    assert_eq!(
        update(&mut app, Action::ConfirmDevtoolFinish),
        Some(Effect::DevtoolFinish(plan))
    );

    app.dialogs
        .push_back(Dialog::DevtoolFinishConfirmation(DevtoolFinishPlan {
            identity,
            layer: Layer {
                name: "meta-rogue".into(),
                path: PathBuf::from("/tmp/meta-rogue"),
                priority: None,
            },
        }));
    assert_eq!(update(&mut app, Action::ConfirmDevtoolFinish), None);
    assert_eq!(
        app.notification.as_deref(),
        Some("The selected finish layer is no longer configured.")
    );
}
#[test]
fn devtool_target_deploy_requires_authoritative_workspace_and_validated_confirmation() {
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
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolDeploy);
    assert_eq!(
        app.notification.as_deref(),
        Some("Refresh authoritative Devtool status with t before deploy-target.")
    );
    app.devtool_statuses.insert(
        identity.clone(),
        DevtoolStatus {
            identity: identity.clone(),
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: PathBuf::from("/build/workspace/sources/busybox"),
                recipe_file: Some(identity.file.clone()),
            },
            git: DevtoolGitState::NotRepository,
            error: None,
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolDeploy);
    let _ = update(&mut app, Action::AppendDevtoolDeployTarget('q'));
    let _ = update(&mut app, Action::AppendDevtoolDeployTarget('e'));
    let _ = update(&mut app, Action::AppendDevtoolDeployTarget('m'));
    let _ = update(&mut app, Action::AppendDevtoolDeployTarget('u'));
    let _ = update(&mut app, Action::PreviewDevtoolDeploy);
    let plan = DevtoolDeployPlan {
        identity: identity.clone(),
        target: "qemu".into(),
    };
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::DevtoolDeployConfirmation(plan.clone()))
    );
    assert_eq!(
        update(&mut app, Action::ConfirmDevtoolDeploy),
        Some(Effect::DevtoolDeploy(plan))
    );

    app.dialogs
        .push_back(Dialog::DevtoolDeploy(DevtoolDeployDraft {
            identity,
            target: "--help".into(),
        }));
    assert_eq!(update(&mut app, Action::PreviewDevtoolDeploy), None);
    assert_eq!(
        app.notification.as_deref(),
        Some(
            "Devtool target must be one non-option value without whitespace or control characters"
        )
    );
}
#[test]
fn devtool_modify_editor_loads_saves_and_builds_selected_recipe() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        ..Recipe::default()
    });
    let root = PathBuf::from("/build/workspace/sources/busybox");
    assert_eq!(
        update(
            &mut app,
            Action::OpenRecipeEditor {
                recipe: "busybox".into(),
                root: root.clone(),
                files: vec![PathBuf::from("main.c")],
            },
        ),
        Some(Effect::LoadRecipeEditorFile(root.join("main.c")))
    );
    let _ = update(
        &mut app,
        Action::LoadRecipeEditorContent("int main() {}".into()),
    );
    let _ = update(&mut app, Action::ToggleRecipeEditorEditing);
    let _ = update(
        &mut app,
        Action::EditRecipeEditor(PopupEditorCommand::Newline),
    );
    let editor = match app.active_dialog() {
        Some(Dialog::RecipeEditor(editor)) => editor,
        other => panic!("expected recipe editor, got {other:?}"),
    };
    assert_eq!(
        editor.document.base_revision(),
        TextAreaRevision::of("int main() {}")
    );
    assert!(editor.is_dirty());
    assert!(editor.local_validation().is_empty());
    let _ = update(&mut app, Action::BeginRecipeEditorBuild);
    assert_eq!(
        app.notification.as_deref(),
        Some("Save workspace changes before starting the recipe build.")
    );
    assert!(matches!(app.active_dialog(), Some(Dialog::RecipeEditor(_))));
    assert_eq!(
        update(&mut app, Action::SaveRecipeEditor),
        Some(Effect::SaveRecipeEditorFile {
            root: root.clone(),
            path: root.join("main.c"),
            content: "int main() {}\n".into(),
            expected: TextAreaRevision::of("int main() {}"),
        })
    );
    let _ = update(&mut app, Action::RecipeEditorSaved);
    let editor = match app.active_dialog() {
        Some(Dialog::RecipeEditor(editor)) => editor,
        other => panic!("expected recipe editor, got {other:?}"),
    };
    assert!(!editor.document.is_modified());
    assert!(editor.diff_preview(2).is_empty());
    let _ = update(&mut app, Action::BeginRecipeEditorBuild);
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
fn external_recipe_edit_keeps_diff_and_allows_ctrl_b_build() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        ..Recipe::default()
    });
    let _ = update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "busybox".into(),
            root: "/workspace/busybox".into(),
            files: vec!["main.c".into()],
        },
    );
    let _ = update(
        &mut app,
        Action::LoadRecipeEditorContent("int value = 1;\n".into()),
    );
    let _ = update(
        &mut app,
        Action::LoadRecipeEditorExternalContent("int value = 2;\n".into()),
    );
    let editor = match app.active_dialog() {
        Some(Dialog::RecipeEditor(editor)) => editor,
        other => panic!("expected recipe editor, got {other:?}"),
    };
    assert!(!editor.is_dirty());
    assert!(!editor.diff_preview(8).is_empty());

    let _ = update(&mut app, Action::BeginRecipeEditorBuild);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::RecipeTaskConfirmation(BuildRequest {
            targets: vec!["busybox".into()],
            task: None,
            force: false,
        }))
    );
}
