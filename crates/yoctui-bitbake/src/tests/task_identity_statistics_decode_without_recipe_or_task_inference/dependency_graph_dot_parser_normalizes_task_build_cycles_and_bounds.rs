use super::*;

#[test]
fn dependency_graph_dot_parser_normalizes_task_build_cycles_and_bounds() {
    let graph = br#"digraph depends {
"image.do_build" [label="image do_build"]
"busybox.do_build" [label="busybox do_build"]
"image.do_build" -> "busybox.do_build"
"image.do_build" -> "busybox.do_build"
"busybox.do_build" -> "image.do_build"
}
"#;
    let response = parse_task_dependency_dot("image", graph).unwrap();
    assert!(
        response
            .limitations
            .iter()
            .any(|value| value.contains("runtime"))
    );
    assert!(response.graph.edges.contains(&DependencyEdge {
        from: DependencyNodeId::recipe("image"),
        to: DependencyNodeId::recipe("busybox"),
        kind: DependencyEdgeKind::Build,
    }));
    assert!(response.graph.edges.contains(&DependencyEdge {
        from: DependencyNodeId::recipe("image"),
        to: DependencyNodeId::task("image", "do_build"),
        kind: DependencyEdgeKind::Task,
    }));
    assert!(response.graph.edges.contains(&DependencyEdge {
        from: DependencyNodeId::task("image", "do_build"),
        to: DependencyNodeId::task("busybox", "do_build"),
        kind: DependencyEdgeKind::Task,
    }));
    assert_eq!(
        parse_task_dependency_dot("image", b"not a dot graph")
            .unwrap_err()
            .to_string(),
        "bridge: dependency graph has an invalid header"
    );

    let mut bounded = String::from("digraph depends {\n");
    for index in 0..=MAX_DEPENDENCY_EDGES {
        bounded.push_str(&format!(
            "\"image.do_{index}\" -> \"dep-{index}.do_build\"\n"
        ));
    }
    bounded.push_str("}\n");
    let response = parse_task_dependency_dot("image", bounded.as_bytes()).unwrap();
    assert!(
        response
            .limitations
            .iter()
            .any(|value| value.contains("bounds dropped"))
    );
    assert!(response.graph.nodes.len() <= MAX_DEPENDENCY_NODES);
    assert!(response.graph.edges.len() <= MAX_DEPENDENCY_EDGES);
}
