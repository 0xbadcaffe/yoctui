use super::*;

#[test]
fn environment_identity_rejects_conflicting_duplicate_tools_and_layers() {
    let mut tools = full_identity();
    let AuthoritativeValue::Detected { value, .. } = &mut tools.available_tools else {
        unreachable!();
    };
    value.push(ToolIdentity {
        id: "bitbake".into(),
        executable: "/other/bin/bitbake".into(),
        version: Some("2.8.1".into()),
    });
    assert!(matches!(
        tools.normalize(),
        Err(EnvironmentIdentityError::ConflictingDuplicate {
            field: "available_tools",
            ..
        })
    ));

    let mut layers = full_identity();
    let AuthoritativeValue::Detected { value, .. } = &mut layers.layer_series else {
        unreachable!();
    };
    value.push(LayerSeriesIdentity {
        layer: "core".into(),
        root: "/different/meta".into(),
        compatible_series: vec!["scarthgap".into()],
    });
    assert!(matches!(
        layers.normalize(),
        Err(EnvironmentIdentityError::ConflictingDuplicate {
            field: "layer_series",
            ..
        })
    ));
}
