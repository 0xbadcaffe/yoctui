use super::*;

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
