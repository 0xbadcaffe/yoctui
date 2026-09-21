use super::*;

#[test]
fn compatibility_older_release_preserves_mixed_state_without_global_failure() {
    let snapshot = CapabilitySnapshot {
        generation: 8,
        environment: environment(),
        capabilities: vec![
            record(
                CapabilityId::BitBakeWorkspaceInspection,
                CapabilityState::Available,
            ),
            record(
                CapabilityId::BitBakeNativeEvents,
                CapabilityState::AvailableWithLimitations {
                    reason: reason("fallback.version_inference"),
                    limitations: vec!["Legacy Tinfoil adapter is selected".into()],
                },
            ),
            record(
                CapabilityId::DevtoolUpgrade,
                CapabilityState::Unavailable {
                    reason: reason("command.missing"),
                },
            ),
            record(
                CapabilityId::ResultTool,
                CapabilityState::Unknown {
                    reason: reason("probe.not_run"),
                },
            ),
            record(
                CapabilityId::SpdxCreate,
                CapabilityState::Unsupported {
                    reason: reason("yoctui.no_safe_implementation"),
                },
            ),
        ],
    }
    .normalize()
    .unwrap();
    assert_eq!(
        snapshot.operating_mode(),
        EnvironmentOperatingMode::Degraded
    );
    assert_eq!(
        snapshot.availability_summary(),
        CapabilityAvailabilitySummary {
            available: 1,
            limited: 1,
            unavailable: 1,
            unknown: 1,
            unsupported: 1,
        }
    );
    assert!(snapshot.allows(CapabilityId::BitBakeWorkspaceInspection));
    assert!(snapshot.allows(CapabilityId::BitBakeNativeEvents));
    assert!(!snapshot.allows(CapabilityId::DevtoolUpgrade));

    let diagnostic = CapabilitySnapshot {
        generation: 9,
        environment: environment(),
        capabilities: vec![record(
            CapabilityId::BitBakeBuild,
            CapabilityState::Unknown {
                reason: reason("probe.not_run"),
            },
        )],
    }
    .normalize()
    .unwrap();
    assert_eq!(
        diagnostic.operating_mode(),
        EnvironmentOperatingMode::Diagnostic
    );
}
