use super::*;

#[tokio::test]
async fn daemon_task_capabilities_require_matching_recipe_metadata_and_preserve_unknowns() {
    let fixture = RuntimeFixture::new();
    let current = DaemonCompatibilityCoordinator::default()
        .startup_from_environment(&fixture.environment)
        .await
        .unwrap()
        .unwrap();
    assert!(!current.snapshot.allows(CapabilityId::SdkPopulate));
    let mut metadata = crate::daemon_startup::StartupWorkspace {
        workspace: yoctui_model::Workspace {
            build_dir: Some(fixture.build.clone()),
            ..Default::default()
        },
        task_metadata: vec![(
            yoctui_model::RecipeIdentity {
                name: "core-image-minimal".into(),
                file: fixture.root.join("image.bb"),
            },
            vec!["do_populate_sdk".into(), "do_package_qa".into()],
        )],
    };
    let recovered =
        crate::daemon_compatibility::metadata_capabilities::recover(current.clone(), &metadata)
            .unwrap()
            .unwrap();
    assert!(recovered.snapshot.allows(CapabilityId::SdkPopulate));
    assert!(recovered.snapshot.allows(CapabilityId::QaTask));
    assert!(!recovered.snapshot.allows(CapabilityId::TestImage));
    assert_eq!(
        recovered.snapshot.generation,
        current.snapshot.generation + 1
    );
    assert!(
        crate::daemon_compatibility::metadata_capabilities::recover(recovered, &metadata)
            .unwrap()
            .is_none()
    );
    metadata.workspace.build_dir = Some(fixture.root.clone());
    assert!(
        crate::daemon_compatibility::metadata_capabilities::recover(current, &metadata).is_err()
    );
}
