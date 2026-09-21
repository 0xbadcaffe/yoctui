use super::*;

#[test]
fn compatibility_utilities_preserves_partial_and_unavailable_reasons() {
    let mut snapshot = CapabilitySnapshot {
        generation: 7,
        environment: YoctoEnvironmentIdentity::default(),
        capabilities: vec![
            CapabilityRecord {
                id: CapabilityId::RunQemu,
                state: CapabilityState::Available,
                evidence: vec![],
            },
            CapabilityRecord {
                id: CapabilityId::WicCreate,
                state: CapabilityState::Unavailable {
                    reason: reason("wic create is absent"),
                },
                evidence: vec![],
            },
            CapabilityRecord {
                id: CapabilityId::ResultTool,
                state: CapabilityState::Unavailable {
                    reason: reason("resulttool executable was not detected"),
                },
                evidence: vec![],
            },
        ],
    };
    snapshot.capabilities.sort_by_key(|record| record.id);
    let statuses = utility_compatibility_statuses(Some(&snapshot));
    let image = statuses
        .iter()
        .find(|status| status.id == "image-runtime")
        .unwrap();
    assert_eq!(
        image.state,
        UtilityCompatibilityState::AvailableWithLimitations
    );
    assert!(image.reason.contains("wic create is absent"));
    let resulttool = statuses
        .iter()
        .find(|status| status.id == "resulttool")
        .unwrap();
    assert_eq!(resulttool.state, UtilityCompatibilityState::Unavailable);
    assert!(resulttool.reason.contains("executable was not detected"));
}
