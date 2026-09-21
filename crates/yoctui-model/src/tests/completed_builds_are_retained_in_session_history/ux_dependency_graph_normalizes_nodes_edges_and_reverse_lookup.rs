use super::*;

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
