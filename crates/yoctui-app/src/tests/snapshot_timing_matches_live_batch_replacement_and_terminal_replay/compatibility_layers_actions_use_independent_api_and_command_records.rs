use super::*;

#[test]
fn compatibility_layers_actions_use_independent_api_and_command_records() {
    let evidence = |id: yoctui_model::CapabilityId,
                    outcome: yoctui_model::CapabilityEvidenceOutcome| {
        yoctui_model::CapabilityEvidence {
            kind: yoctui_model::CapabilityEvidenceKind::DirectProbe,
            outcome,
            subject: format!("{} fixture probe", id.as_str()),
            detail: "The fixture reports this exact Layers behavior.".into(),
            argv: vec![
                "/work/poky/bitbake/bin/bitbake-layers".into(),
                "--help".into(),
            ],
        }
    };
    let available = yoctui_model::CapabilityId::BitBakeLayersShowLayers;
    let unavailable = yoctui_model::CapabilityId::BitBakeLayersRemoveLayer;
    let snapshot = yoctui_model::DaemonCompatibilitySnapshot {
        snapshot: yoctui_model::CapabilitySnapshot {
            generation: 4,
            environment: yoctui_model::YoctoEnvironmentIdentity::default(),
            capabilities: vec![
                yoctui_model::CapabilityRecord {
                    id: available,
                    state: yoctui_model::CapabilityState::Available,
                    evidence: vec![evidence(
                        available,
                        yoctui_model::CapabilityEvidenceOutcome::Positive,
                    )],
                },
                yoctui_model::CapabilityRecord {
                    id: unavailable,
                    state: yoctui_model::CapabilityState::Unavailable {
                        reason: yoctui_model::CapabilityReason::new(
                            "bitbake_layers.subcommand_missing",
                            "Current bitbake-layers does not expose remove-layer.",
                            Some("Required capability: bitbake_layers.remove_layer".into()),
                        )
                        .unwrap(),
                    },
                    evidence: vec![evidence(
                        unavailable,
                        yoctui_model::CapabilityEvidenceOutcome::Negative,
                    )],
                },
            ],
        },
        implementations: std::collections::BTreeMap::from([(
            available,
            yoctui_model::CapabilityImplementation {
                id: yoctui_bitbake::BITBAKE_LAYERS_SHOW_IMPLEMENTATION.into(),
                kind: yoctui_model::CapabilityImplementationKind::Command,
            },
        )]),
    }
    .normalize()
    .unwrap();
    let actions = compatibility_layer_actions(Some(&snapshot));
    assert!(
        actions
            .iter()
            .find(|action| action.capability == available)
            .unwrap()
            .available
    );
    assert!(
        actions
            .iter()
            .find(|action| action.capability == unavailable)
            .unwrap()
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("does not expose remove-layer"))
    );
    assert!(
        actions
            .iter()
            .find(|action| {
                action.capability == yoctui_model::CapabilityId::BitBakeLayerInventory
            })
            .unwrap()
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("current environment capability snapshot"))
    );
}
