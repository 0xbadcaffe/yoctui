use super::*;

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
