use super::*;

#[test]
fn raw_capability_missing_record_is_unknown_without_version_inference() {
    let authority = authority(Vec::new());
    let command = with_requirement(RawCapabilityRequirement::All {
        capabilities: vec![CapabilityId::BitBakeBuild],
    });
    let availability = command.availability(Some(&authority));
    assert_eq!(availability.state, RawAvailabilityState::Unknown);
    assert_eq!(
        availability.issues[0].reason,
        "bitbake.build has no capability evidence."
    );
}
