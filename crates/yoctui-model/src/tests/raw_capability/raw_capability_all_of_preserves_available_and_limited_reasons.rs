use super::*;

#[test]
fn raw_capability_all_of_preserves_available_and_limited_reasons() {
    let command = with_requirement(RawCapabilityRequirement::All {
        capabilities: vec![
            CapabilityId::BitBakeBuild,
            CapabilityId::BitBakeEnvironmentDump,
        ],
    });
    let authority = authority(vec![
        (
            CapabilityId::BitBakeBuild,
            CapabilityState::Available,
            Some("bitbake.argv"),
        ),
        (
            CapabilityId::BitBakeEnvironmentDump,
            CapabilityState::AvailableWithLimitations {
                reason: reason("Environment output is truncated."),
                limitations: vec!["Maximum 4 MiB output.".into()],
            },
            Some("bitbake.environment.argv"),
        ),
    ]);
    let availability = command.availability(Some(&authority));
    assert_eq!(availability.state, RawAvailabilityState::Limited);
    assert!(availability.is_enabled());
    assert_eq!(
        availability.issues[0].reason,
        "Environment output is truncated."
    );
    assert_eq!(
        availability.issues[0].limitations,
        vec!["Maximum 4 MiB output."]
    );
    assert_eq!(availability.implementations.len(), 2);
}
