use super::*;

#[test]
fn compatibility_workspace_model_projects_full_limited_all_and_any_requirements() {
    let authority = authority(
        1,
        vec![
            (
                CapabilityId::BitBakeBuild,
                CapabilityState::Available,
                Some("tinfoil.build"),
            ),
            (
                CapabilityId::SdkExtensible,
                CapabilityState::AvailableWithLimitations {
                    reason: reason("legacy extensible SDK task is selected"),
                    limitations: vec!["legacy task adapter".into()],
                },
                Some("bitbake.populate_sdk_ext"),
            ),
            (
                CapabilityId::RunQemu,
                CapabilityState::Unavailable {
                    reason: reason("runqemu was not detected"),
                },
                None,
            ),
            (
                CapabilityId::WicCreate,
                CapabilityState::Available,
                Some("wic.create.argv"),
            ),
        ],
    );
    let limited = workspace_requirement_availability(
        Some(&authority),
        &WorkspaceEffectRequirement::all(&[
            CapabilityId::BitBakeBuild,
            CapabilityId::SdkExtensible,
        ]),
    );
    assert_eq!(
        limited.state,
        WorkspaceAvailabilityState::AvailableWithLimitations
    );
    assert!(limited.issues[0].reason.contains("legacy extensible"));
    assert_eq!(limited.implementations.len(), 2);

    let alternative = workspace_requirement_availability(
        Some(&authority),
        &WorkspaceEffectRequirement::all_and_any(
            &[],
            &[CapabilityId::RunQemu, CapabilityId::WicCreate],
        ),
    );
    assert_eq!(alternative.state, WorkspaceAvailabilityState::Available);
    assert_eq!(
        alternative.implementations,
        vec![(CapabilityId::WicCreate, "wic.create.argv".into())]
    );
}
