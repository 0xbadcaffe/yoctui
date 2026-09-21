use super::*;

#[test]
fn compatibility_workspace_catalog_classifies_local_single_all_and_alternative_effects() {
    assert_eq!(
        workspace_effect_requirement(&Effect::CopyToClipboard("value".into())),
        WorkspaceEffectRequirement::ClientLocal
    );
    assert_eq!(
        workspace_effect_requirement(&Effect::GetVariable(VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        })),
        WorkspaceEffectRequirement::one(CapabilityId::BitBakeGetVar)
    );
    assert_eq!(
        workspace_effect_requirement(&Effect::Start(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: Some("populate_sdk_ext".into()),
            force: false,
        })),
        WorkspaceEffectRequirement::all(
            &[CapabilityId::BitBakeBuild, CapabilityId::SdkExtensible,]
        )
    );
    assert_eq!(
        workspace_destination_requirement(WorkspaceDestination::QemuWic),
        WorkspaceEffectRequirement::all_and_any(
            &[],
            &[CapabilityId::RunQemu, CapabilityId::WicCreate],
        )
    );
}
