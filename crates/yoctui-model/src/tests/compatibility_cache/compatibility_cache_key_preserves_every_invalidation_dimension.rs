use super::*;

#[test]
fn compatibility_cache_key_preserves_every_invalidation_dimension() {
    let original = key().normalize().unwrap();
    let mut variants = Vec::new();

    let mut changed = key();
    changed.workspace_identity.push_str("-other");
    variants.push(changed);

    let mut changed = key();
    changed.environment.build_directory = AuthoritativeValue::detected(
        "/workspace/other-build".into(),
        IdentityAuthority::InitializedEnvironment,
    );
    variants.push(changed);

    let mut changed = key();
    changed.environment.bitbake_version =
        AuthoritativeValue::detected("2.10.0".into(), IdentityAuthority::BitBakeVersionProbe);
    variants.push(changed);

    let mut changed = key();
    changed.environment.source_roots = AuthoritativeValue::detected(
        vec![SourceRootIdentity {
            kind: SourceRootKind::CoreBase,
            path: "/workspace/other-source".into(),
        }],
        IdentityAuthority::InitializedEnvironment,
    );
    variants.push(changed);

    let mut changed = key();
    changed.environment.available_tools = AuthoritativeValue::detected(
        vec![ToolIdentity {
            id: "bitbake".into(),
            executable: "/workspace/bitbake/bin/bitbake".into(),
            version: Some("2.8.2".into()),
        }],
        IdentityAuthority::ExecutableProbe,
    );
    variants.push(changed);

    let mut changed = key();
    changed.environment.layer_series = AuthoritativeValue::detected(
        vec![LayerSeriesIdentity {
            layer: "core".into(),
            root: "/workspace/meta".into(),
            compatible_series: vec!["scarthgap".into()],
        }],
        IdentityAuthority::ConfiguredLayerMetadata,
    );
    variants.push(changed);

    let mut changed = key();
    changed.initialized_environment_digest = digest('d');
    variants.push(changed);

    let mut changed = key();
    changed.layer_configuration_digest = digest('d');
    variants.push(changed);

    let mut changed = key();
    changed.build_configuration_digest = digest('d');
    variants.push(changed);

    let mut changed = key();
    changed.daemon_workspace_identity = "daemon-workspace-two".into();
    variants.push(changed);

    for variant in variants {
        assert_ne!(original, variant.normalize().unwrap());
    }
}
