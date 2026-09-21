use super::*;

#[test]
fn raw_capability_any_of_prefers_fully_available_implementation() {
    let command = with_requirement(RawCapabilityRequirement::Any {
        capabilities: vec![
            CapabilityId::BitBakeEnvironmentDump,
            CapabilityId::BitBakeBuild,
        ],
    });
    let authority = authority(vec![
        (
            CapabilityId::BitBakeEnvironmentDump,
            CapabilityState::AvailableWithLimitations {
                reason: reason("Fallback output is limited."),
                limitations: vec!["Recipe scope only.".into()],
            },
            Some("bitbake.environment.fallback"),
        ),
        (
            CapabilityId::BitBakeBuild,
            CapabilityState::Available,
            Some("bitbake.argv"),
        ),
    ]);
    let availability = command.availability(Some(&authority));
    assert_eq!(availability.state, RawAvailabilityState::Available);
    assert!(availability.issues.is_empty());
    assert_eq!(
        availability.implementations,
        vec![(CapabilityId::BitBakeBuild, "bitbake.argv".into())]
    );
}
