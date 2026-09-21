use super::*;

#[test]
fn compatibility_workspace_model_absent_unknown_unsupported_and_missing_all_fail_closed() {
    let absent = workspace_requirement_availability(
        None,
        &WorkspaceEffectRequirement::all(&[CapabilityId::BitBakeBuild, CapabilityId::SdkPopulate]),
    );
    assert_eq!(absent.state, WorkspaceAvailabilityState::Unknown);
    assert_eq!(absent.issues.len(), 2);

    let authority = authority(
        2,
        vec![
            (
                CapabilityId::BitBakeBuild,
                CapabilityState::Unavailable {
                    reason: reason("build API was rejected"),
                },
                None,
            ),
            (
                CapabilityId::SdkPopulate,
                CapabilityState::Unsupported {
                    reason: reason("standard SDK workflow is intentionally unsupported"),
                },
                None,
            ),
            (
                CapabilityId::RunQemu,
                CapabilityState::Unknown {
                    reason: reason("runqemu probe timed out"),
                },
                None,
            ),
        ],
    );
    let all = workspace_requirement_availability(
        Some(&authority),
        &WorkspaceEffectRequirement::all(&[CapabilityId::BitBakeBuild, CapabilityId::SdkPopulate]),
    );
    assert_eq!(all.state, WorkspaceAvailabilityState::Unavailable);
    assert_eq!(all.issues.len(), 2);
    assert!(
        all.issues
            .iter()
            .any(|issue| issue.reason.contains("build API"))
    );
    let unknown = workspace_requirement_availability(
        Some(&authority),
        &WorkspaceEffectRequirement::one(CapabilityId::RunQemu),
    );
    assert_eq!(unknown.state, WorkspaceAvailabilityState::Unknown);
    assert!(unknown.issues[0].reason.contains("timed out"));
    let unsupported = workspace_requirement_availability(
        Some(&authority),
        &WorkspaceEffectRequirement::one(CapabilityId::SdkPopulate),
    );
    assert_eq!(unsupported.state, WorkspaceAvailabilityState::Unsupported);
}
