use super::*;

#[test]
fn capability_snapshot_normalizes_all_states_and_supports_fail_closed_lookup() {
    let snapshot = CapabilitySnapshot {
        generation: 7,
        environment: environment(),
        capabilities: vec![
            record(
                CapabilityId::DevtoolUpgrade,
                CapabilityState::Unavailable {
                    reason: reason("command.missing"),
                },
            ),
            record(CapabilityId::BitBakeGetVar, CapabilityState::Available),
            record(
                CapabilityId::SpdxCreate,
                CapabilityState::AvailableWithLimitations {
                    reason: reason("fallback.legacy_spdx"),
                    limitations: vec!["Legacy SPDX task does not emit the newest schema".into()],
                },
            ),
            record(
                CapabilityId::ResultTool,
                CapabilityState::Unknown {
                    reason: reason("probe.not_run"),
                },
            ),
            record(
                CapabilityId::RecipetoolAppendFile,
                CapabilityState::Unsupported {
                    reason: reason("yoctui.no_safe_implementation"),
                },
            ),
        ],
    }
    .normalize()
    .unwrap();

    assert_eq!(snapshot.generation, 7);
    assert!(snapshot.allows(CapabilityId::BitBakeGetVar));
    assert!(snapshot.allows(CapabilityId::SpdxCreate));
    assert!(!snapshot.allows(CapabilityId::DevtoolUpgrade));
    assert!(!snapshot.allows(CapabilityId::ResultTool));
    assert!(!snapshot.allows(CapabilityId::RecipetoolAppendFile));
    assert!(!snapshot.allows(CapabilityId::RunQemu));
    assert_eq!(
        snapshot
            .capability(CapabilityId::DevtoolUpgrade)
            .unwrap()
            .state
            .reason()
            .unwrap()
            .code
            .as_str(),
        "command.missing"
    );
    assert!(
        snapshot
            .capabilities
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id)
    );
}
