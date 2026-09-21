use super::*;

#[test]
fn recipe_metadata_round_trips_and_old_recipe_payloads_default_safely() {
    let event = Event::RecipeMetadata {
        data: RecipeMetadataData {
            recipe: "busybox".into(),
            workspace_status: Some(RecipeWorkspaceStatusData::Modified),
            build_status: None,
            tasks: Some(vec!["do_build".into(), "do_compile".into()]),
            sources: Some(vec!["/layers/meta/recipes-core/busybox/busybox.bb".into()]),
            patches: Some(vec!["file://fix.patch".into()]),
            packages: Some(vec!["busybox".into(), "busybox-src".into()]),
            history: None,
        },
    };
    let envelope = Envelope {
        protocol_version: VERSION,
        sequence: 7,
        correlation_id: Some("recipe-metadata".into()),
        message: event.clone(),
    };
    assert_eq!(
        decode_line::<Event>(&encode_line(&envelope).unwrap(), None)
            .unwrap()
            .message,
        event
    );

    let old = br#"{"protocol_version":1,"sequence":8,"message":{"type":"recipes","recipes":[{"name":"busybox","version":"1.36","layer":"core"}]}}"#;
    let Event::Recipes { recipes } = decode_line::<Event>(old, None).unwrap().message else {
        panic!("old recipe payload changed type");
    };
    assert_eq!(recipes[0].preferred_version, None);
    assert_eq!(recipes[0].file, None);
    assert_eq!(recipes[0].append_count, None);
}
