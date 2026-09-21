use super::*;

#[test]
fn dependency_graph_round_trips_and_legacy_dependencies_remain_compatible() {
    let root = DependencyNodeIdData {
        recipe: "core-image-minimal".into(),
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
                log: None,
            }],
            edges: vec![DependencyEdgeData {
                from: root.clone(),
                to: task,
                kind: DependencyEdgeKindData::Task,
            }],
            limitations: vec!["runtime edges unavailable".into()],
        },
    };
    let envelope = Envelope {
        protocol_version: VERSION,
        sequence: 12,
        correlation_id: Some("dependency-graph".into()),
        message: event.clone(),
    };
    assert_eq!(
        decode_line::<Event>(&encode_line(&envelope).unwrap(), None)
            .unwrap()
            .message,
        event
    );
    let command = Envelope {
        protocol_version: VERSION,
        sequence: 13,
        correlation_id: None,
        message: Command::GetDependencyGraph {
            recipe: "busybox".into(),
        },
    };
    assert_eq!(
        decode_line::<Command>(&encode_line(&command).unwrap(), None).unwrap(),
        command
    );
    let legacy = br#"{"protocol_version":1,"sequence":14,"message":{"type":"dependencies","recipe":"busybox","build":["zlib"],"runtime":[]}}"#;
    assert!(matches!(
        decode_line::<Event>(legacy, None).unwrap().message,
        Event::Dependencies { recipe, build, runtime }
            if recipe == "busybox" && build == ["zlib"] && runtime.is_empty()
    ));
}
