use super::*;

#[test]
fn compatibility_probe_aggregation_keeps_partial_compound_evidence_disabled() {
    let catalog = CapabilityCatalog::builtin();
    let entry = catalog.entry(CapabilityId::DevtoolUpgrade).unwrap();
    let resolver = CapabilityResolver::default();

    let complete = complete_observations(
        &catalog,
        entry.id,
        CapabilityProbeStatus::Positive,
        "devtool upgrade",
    );
    let available = resolver.resolve(entry, Some("2.18"), &complete);
    assert_eq!(available.record.state, CapabilityState::Available);
    assert_eq!(available.implementation, Some(entry.preferred.clone()));

    let partial = vec![
        observation(CapabilityProbeStatus::Positive, "devtool executable"),
        observation(
            CapabilityProbeStatus::Inconclusive,
            "upgrade --help timeout",
        ),
    ];
    let unknown = resolver.resolve(entry, Some("2.18"), &partial);
    assert!(matches!(
        unknown.record.state,
        CapabilityState::Unknown { ref reason }
            if reason.code.as_str() == "evidence.incomplete"
    ));
    assert!(unknown.implementation.is_none());

    let unavailable = resolver.resolve(
        entry,
        Some("2.18"),
        &[
            observation(CapabilityProbeStatus::Negative, "upgrade absent"),
            observation(CapabilityProbeStatus::Inconclusive, "help timeout"),
        ],
    );
    assert!(matches!(
        unavailable.record.state,
        CapabilityState::Unavailable { .. }
    ));
    assert!(unavailable.implementation.is_none());

    let conflict = resolver.resolve(
        entry,
        Some("2.18"),
        &[
            observation(CapabilityProbeStatus::Positive, "upgrade present"),
            observation(CapabilityProbeStatus::Negative, "upgrade absent"),
        ],
    );
    assert!(matches!(
        conflict.record.state,
        CapabilityState::Unknown { ref reason }
            if reason.code.as_str() == "evidence.conflict"
    ));
    assert!(conflict.implementation.is_none());
}
