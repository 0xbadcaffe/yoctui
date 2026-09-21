use super::*;

#[test]
fn compatibility_ui_model_projects_identity_summary_states_reasons_and_evidence() {
    let compatibility = state_with(authority(1));
    let mut state = CompatibilityUiState::default();
    state.reconcile(compatibility.authority());
    let projection = state.project(&compatibility, ClientReplicaStatus::Current);

    assert_eq!(
        projection.authority,
        CompatibilityUiAuthorityStatus::Current {
            generation: 1,
            mode: EnvironmentOperatingMode::Degraded,
        }
    );
    assert_eq!(
        projection.summary,
        CapabilityAvailabilitySummary {
            available: 1,
            limited: 1,
            unavailable: 1,
            unknown: 1,
            unsupported: 1,
        }
    );
    assert_eq!(projection.total_capabilities, 5);
    assert_eq!(projection.rows.len(), 5);
    assert_eq!(
        projection
            .environment
            .as_ref()
            .unwrap()
            .bitbake_version
            .value()
            .map(String::as_str),
        Some("2.18.0")
    );
    let limited = projection
        .rows
        .iter()
        .find(|row| row.id == CapabilityId::BitBakeGetVar)
        .unwrap();
    assert_eq!(limited.state, CompatibilityUiCapabilityState::Limited);
    assert_eq!(
        limited.reason.as_ref().unwrap().code.as_str(),
        "compatibility.fallback"
    );
    assert_eq!(limited.limitations.len(), 1);
    assert_eq!(
        limited.implementation.as_ref().unwrap().id,
        "bitbake.getvar.environment-fallback"
    );
    assert_eq!(limited.evidence[0].argv, ["bitbake -e", "--help"]);
}
