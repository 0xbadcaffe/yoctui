use super::*;

#[test]
fn daemon_compatibility_snapshot_is_identical_on_attach_reconnect_and_update() {
    fn compatibility(generation: u64) -> yoctui_model::DaemonCompatibilitySnapshot {
        let environment = yoctui_model::YoctoEnvironmentIdentity {
            build_directory: yoctui_model::AuthoritativeValue::detected(
                "/work/poky/build".into(),
                yoctui_model::IdentityAuthority::InitializedEnvironment,
            ),
            bitbake_version: yoctui_model::AuthoritativeValue::detected(
                "2.18.0".into(),
                yoctui_model::IdentityAuthority::BitBakeVersionProbe,
            ),
            ..yoctui_model::YoctoEnvironmentIdentity::default()
        };
        yoctui_model::DaemonCompatibilitySnapshot {
            snapshot: yoctui_model::CapabilitySnapshot {
                generation,
                environment,
                capabilities: vec![yoctui_model::CapabilityRecord {
                    id: yoctui_model::CapabilityId::BitBakeBuild,
                    state: yoctui_model::CapabilityState::Available,
                    evidence: vec![yoctui_model::CapabilityEvidence {
                        kind: yoctui_model::CapabilityEvidenceKind::DirectProbe,
                        outcome: yoctui_model::CapabilityEvidenceOutcome::Positive,
                        subject: "bitbake executable".into(),
                        detail: "The initialized environment exposes BitBake.".into(),
                        argv: Vec::new(),
                    }],
                }],
            },
            implementations: std::collections::BTreeMap::from([(
                yoctui_model::CapabilityId::BitBakeBuild,
                yoctui_model::CapabilityImplementation {
                    id: "bitbake.build.command".into(),
                    kind: yoctui_model::CapabilityImplementationKind::Command,
                },
            )]),
        }
    }

    let mut state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([6; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    reduce_daemon_state(
        &mut state,
        yoctui_model::DaemonStateAction::ReplaceCompatibility(Box::new(compatibility(1))),
    )
    .unwrap();
    let initial = daemon_protocol_snapshot(&state);
    initial.compatibility.as_ref().unwrap().validate().unwrap();

    let mut first_client = DaemonClientSnapshot::default();
    let mut reconnecting_client = DaemonClientSnapshot::default();
    first_client.replace(initial.clone());
    reconnecting_client.replace(initial.clone());
    assert_eq!(
        first_client.snapshot.as_ref().unwrap().compatibility,
        reconnecting_client.snapshot.as_ref().unwrap().compatibility
    );

    let replacement = daemon_compatibility_protocol(&compatibility(2));
    let event = yoctui_protocol::daemon::SequencedEvent {
        sequence: initial.sequence + 1,
        generation: initial.generation + 1,
        event: yoctui_protocol::daemon::DaemonEvent::CompatibilityChanged(Box::new(replacement)),
    };
    first_client.apply_event(&event).unwrap();
    assert_eq!(
        first_client
            .snapshot
            .as_ref()
            .unwrap()
            .compatibility
            .as_ref()
            .unwrap()
            .generation,
        2
    );
    assert_eq!(
        reconnecting_client
            .snapshot
            .as_ref()
            .unwrap()
            .compatibility
            .as_ref()
            .unwrap()
            .generation,
        1
    );
}
