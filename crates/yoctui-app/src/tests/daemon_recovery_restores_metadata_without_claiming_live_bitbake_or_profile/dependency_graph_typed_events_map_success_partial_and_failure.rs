use super::*;

#[test]
fn dependency_graph_typed_events_map_success_partial_and_failure() {
    let root = DependencyNodeId::recipe("core-image-minimal");
    let (graph, _) = DependencyGraph::normalize(
        root.clone(),
        Vec::new(),
        vec![DependencyEdge {
            from: root.clone(),
            to: DependencyNodeId::recipe("busybox"),
            kind: DependencyEdgeKind::Runtime,
        }],
        10,
        10,
    );
    assert_eq!(
        model_action_from_backend_event(BackendEvent::DependencyGraph {
            graph: graph.clone(),
            limitations: Vec::new(),
        }),
        Some(Action::DependencyGraphLoaded(graph.clone()))
    );
    let mut compatibility = App::new(10, 1_000);
    let action = model_action_from_backend_event(BackendEvent::DependencyGraph {
        graph: graph.clone(),
        limitations: Vec::new(),
    })
    .unwrap();
    let _ = update(&mut compatibility, action);
    assert_eq!(
        compatibility.dependencies.as_ref().unwrap().runtime,
        ["busybox"]
    );
    assert_eq!(
        model_action_from_backend_event(BackendEvent::DependencyGraph {
            graph: graph.clone(),
            limitations: vec!["task graph unavailable".into()],
        }),
        Some(Action::DependencyGraphPartial {
            graph,
            limitations: vec!["task graph unavailable".into()],
        })
    );
    assert_eq!(
        model_action_from_backend_event(BackendEvent::DependencyGraphFailed {
            root: root.clone(),
            message: "query failed".into(),
        }),
        Some(Action::DependencyGraphFailed {
            root,
            message: "query failed".into(),
        })
    );

    let mut app = App::new(10, 1_000);
    let action = model_action_from_backend_event(BackendEvent::DependencyGraphFailed {
        root: DependencyNodeId::recipe("image"),
        message: "offline".into(),
    })
    .unwrap();
    let _ = update(&mut app, action);
    assert!(matches!(
        app.dependency_graph,
        DependencyGraphState::Failed { .. }
    ));
}
