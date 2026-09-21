use super::*;

#[test]
fn compatibility_workspace_catalog_marks_environment_inspection_daemon_owned() {
    assert_eq!(
        workspace_effect_requirement(&Effect::InspectQemuCapability),
        WorkspaceEffectRequirement::probe(&[CapabilityId::RunQemu])
    );
    assert_eq!(
        workspace_effect_requirement(&Effect::InspectSdkTools),
        WorkspaceEffectRequirement::probe(&[
            CapabilityId::SdkPublish,
            CapabilityId::SdkNativeTools,
        ])
    );
}
