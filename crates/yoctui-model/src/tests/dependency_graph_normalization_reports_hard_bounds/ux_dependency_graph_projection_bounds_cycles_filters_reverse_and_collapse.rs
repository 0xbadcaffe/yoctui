use super::*;

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
