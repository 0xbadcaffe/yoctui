use super::*;

#[test]
fn recipe_metadata_converts_typed_statuses_and_paths() {
    let event = Event::RecipeMetadata {
        data: yoctui_protocol::RecipeMetadataData {
            recipe: "busybox".into(),
            workspace_status: Some(RecipeWorkspaceStatusData::Modified),
            build_status: Some(RecipeBuildStatusData::Running),
            tasks: Some(vec!["do_compile".into()]),
            sources: Some(vec!["/layers/meta/busybox.bb".into()]),
            patches: Some(vec!["file://fix.patch".into()]),
            packages: Some(vec!["busybox".into()]),
            history: None,
        },
    };
    let BackendEvent::RecipeMetadata(metadata) = BridgeBackend::event(event).unwrap() else {
        panic!("recipe metadata event was not preserved");
    };
    assert_eq!(
        metadata.workspace_status,
        Some(RecipeWorkspaceStatus::Modified)
    );
    assert_eq!(metadata.build_status, Some(RecipeBuildStatus::Running));
    assert_eq!(
        metadata.sources,
        Some(vec![PathBuf::from("/layers/meta/busybox.bb")])
    );
    assert_eq!(metadata.history, None);
}
