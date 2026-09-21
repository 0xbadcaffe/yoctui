use super::*;

#[test]
fn typed_event_workspace_converts_wire_paths_and_metadata() {
    let event = Event::Workspace {
        data: yoctui_protocol::WorkspaceData {
            build_dir: Some("/build".into()),
            source_dir: Some("/poky".into()),
            variables: std::collections::HashMap::from([("MACHINE".into(), "qemux86-64".into())]),
            variable_provenance: std::collections::HashMap::new(),
            variable_provenance_chain: std::collections::HashMap::new(),
            bitbake_version: Some("2.19.0".into()),
            release: Some("6.0".into()),
            layers: vec![LayerData {
                name: "core".into(),
                path: "/poky/meta".into(),
                priority: Some(5),
            }],
            recipes: vec![RecipeData {
                name: "base-files".into(),
                version: None,
                layer: Some("core".into()),
                preferred_version: None,
                file: Some("/poky/meta/recipes-core/base-files/base-files.bb".into()),
                append_count: Some(0),
            }],
        },
    };
    let BackendEvent::Workspace(workspace) = BridgeBackend::event(event).unwrap() else {
        panic!("workspace event was not preserved");
    };
    assert_eq!(workspace.build_dir, Some(PathBuf::from("/build")));
    assert_eq!(workspace.layers[0].path, PathBuf::from("/poky/meta"));
    assert_eq!(workspace.recipes[0].name, "base-files");
}
