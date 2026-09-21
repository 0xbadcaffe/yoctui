use super::*;

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
