use super::*;

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
