use super::*;

#[test]
fn compatibility_probe_aggregation_requires_every_direct_requirement() {
    let map = VersionFallbackMap;
    let capability = entry(CapabilityId::DevtoolUpgrade);
    assert!(capability.probes.len() > 1);

    let complete = vec![direct(CapabilityProbeStatus::Positive); capability.probes.len()];
    assert_eq!(
        map.resolve_bitbake(&capability, Some("2.18"), &complete),
        VersionFallbackResolution::Direct {
            outcome: CapabilityEvidenceOutcome::Positive,
        }
    );
    for incomplete in [
        vec![direct(CapabilityProbeStatus::Positive)],
        vec![
            direct(CapabilityProbeStatus::Positive),
            direct(CapabilityProbeStatus::Inconclusive),
        ],
    ] {
        let resolution = map.resolve_bitbake(&capability, Some("2.18"), &incomplete);
        assert!(matches!(
            resolution,
            VersionFallbackResolution::Unknown {
                state: CapabilityState::Unknown { ref reason },
                ..
            } if reason.code.as_str() == "evidence.incomplete"
        ));
    }
    assert!(matches!(
        map.resolve_bitbake(&capability, Some("2.18"), &[]),
        VersionFallbackResolution::Unknown {
            state: CapabilityState::Unknown { ref reason },
            ..
        } if reason.code.as_str() == "evidence.missing"
    ));
    assert_eq!(
        map.resolve_bitbake(
            &capability,
            Some("2.18"),
            &[
                direct(CapabilityProbeStatus::Negative),
                direct(CapabilityProbeStatus::Inconclusive),
            ],
        ),
        VersionFallbackResolution::Direct {
            outcome: CapabilityEvidenceOutcome::Negative,
        }
    );
}
