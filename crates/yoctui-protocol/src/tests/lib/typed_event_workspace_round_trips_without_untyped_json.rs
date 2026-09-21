use super::*;

#[test]
fn typed_event_workspace_round_trips_without_untyped_json() {
    let event = Event::Workspace {
        data: WorkspaceData {
            build_dir: Some("/build".into()),
            source_dir: Some("/poky".into()),
            variables: HashMap::from([("MACHINE".into(), "qemux86-64".into())]),
            variable_provenance: HashMap::new(),
            variable_provenance_chain: HashMap::new(),
            bitbake_version: Some("2.19.0".into()),
            release: Some("6.0".into()),
            layers: vec![LayerData {
                name: "core".into(),
                path: "/poky/meta".into(),
                priority: Some(5),
            }],
            recipes: vec![RecipeData {
                name: "base-files".into(),
                version: Some("3.0".into()),
                layer: Some("core".into()),
                preferred_version: None,
                file: Some("/poky/meta/recipes-core/base-files/base-files.bb".into()),
                append_count: Some(0),
            }],
        },
    };
    let envelope = Envelope {
        protocol_version: VERSION,
        sequence: 4,
        correlation_id: None,
        message: event.clone(),
    };
    assert_eq!(
        decode_line::<Event>(&encode_line(&envelope).unwrap(), None)
            .unwrap()
            .message,
        event
    );
}
