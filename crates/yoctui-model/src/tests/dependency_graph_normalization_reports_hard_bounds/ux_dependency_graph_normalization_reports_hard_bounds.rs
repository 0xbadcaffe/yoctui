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
