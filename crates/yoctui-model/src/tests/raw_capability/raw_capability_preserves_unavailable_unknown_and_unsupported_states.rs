use super::*;

#[test]
fn raw_capability_preserves_unavailable_unknown_and_unsupported_states() {
    for (state, expected, message) in [
        (
            CapabilityState::Unavailable {
                reason: reason("Required option was not found."),
            },
            RawAvailabilityState::Unavailable,
            "Required option was not found.",
        ),
        (
            CapabilityState::Unknown {
                reason: reason("Probe did not complete."),
            },
            RawAvailabilityState::Unknown,
            "Probe did not complete.",
        ),
        (
            CapabilityState::Unsupported {
                reason: reason("Backend cannot expose this operation."),
            },
            RawAvailabilityState::Unsupported,
            "Backend cannot expose this operation.",
        ),
    ] {
        let authority = authority(vec![(CapabilityId::BitBakeBuild, state, None)]);
        let command = with_requirement(RawCapabilityRequirement::All {
            capabilities: vec![CapabilityId::BitBakeBuild],
        });
        let availability = command.availability(Some(&authority));
        assert_eq!(availability.state, expected);
        assert_eq!(availability.issues[0].reason, message);
        assert!(!availability.is_enabled());
    }
}
