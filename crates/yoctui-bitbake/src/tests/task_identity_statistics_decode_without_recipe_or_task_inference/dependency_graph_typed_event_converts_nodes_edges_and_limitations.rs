use super::*;

#[test]
fn dependency_graph_typed_event_converts_nodes_edges_and_limitations() {
    let root = DependencyNodeIdData {
        recipe: "image".into(),
        task: None,
    };
    let task = DependencyNodeIdData {
        recipe: "busybox".into(),
        task: Some("do_compile".into()),
    };
    let event = Event::DependencyGraph {
        data: DependencyGraphData {
            root: root.clone(),
            nodes: vec![DependencyNodeData {
                id: task.clone(),
                provider: Some("/layers/meta/busybox.bb".into()),
                log: Some("tmp/log.do_compile".into()),
            }],
            edges: vec![DependencyEdgeData {
                from: root,
                to: task.clone(),
                kind: DependencyEdgeKindData::Task,
            }],
            limitations: vec!["runtime unavailable".into()],
        },
    };
    let BackendEvent::DependencyGraph { graph, limitations } = BridgeBackend::event(event).unwrap()
    else {
        panic!("dependency graph event was not preserved");
    };
    assert_eq!(graph.root, DependencyNodeId::recipe("image"));
    assert_eq!(
        graph
            .nodes
            .iter()
            .find(|node| node.id == DependencyNodeId::task("busybox", "do_compile"))
            .and_then(|node| node.provider.as_deref()),
        Some(Path::new("/layers/meta/busybox.bb"))
    );
    assert_eq!(graph.edges[0].kind, DependencyEdgeKind::Task);
    assert_eq!(
        graph
            .nodes
            .iter()
            .find(|node| node.id == DependencyNodeId::task("busybox", "do_compile"))
            .and_then(|node| node.log.as_ref()),
        None
    );
    assert!(
        limitations
            .iter()
            .any(|value| value == "runtime unavailable")
    );
    assert!(
        limitations
            .iter()
            .any(|value| value.contains("non-absolute"))
    );
}
