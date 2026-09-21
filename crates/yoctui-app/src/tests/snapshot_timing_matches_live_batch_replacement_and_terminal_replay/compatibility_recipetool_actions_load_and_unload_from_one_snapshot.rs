use super::*;

#[test]
fn compatibility_recipetool_actions_load_and_unload_from_one_snapshot() {
    let record = |id, state, outcome| yoctui_model::CapabilityRecord {
        id,
        state,
        evidence: vec![yoctui_model::CapabilityEvidence {
            kind: yoctui_model::CapabilityEvidenceKind::DirectProbe,
            outcome,
            subject: format!("{} fixture probe", id.as_str()),
            detail: "The fixture reports this exact Recipetool behavior.".into(),
            argv: vec!["/work/poky/scripts/recipetool".into(), "--help".into()],
        }],
    };
    let snapshot = yoctui_model::DaemonCompatibilitySnapshot {
        snapshot: yoctui_model::CapabilitySnapshot {
            generation: 3,
            environment: yoctui_model::YoctoEnvironmentIdentity::default(),
            capabilities: vec![
                record(
                    yoctui_model::CapabilityId::RecipetoolCreateOutfile,
                    yoctui_model::CapabilityState::Unavailable {
                        reason: yoctui_model::CapabilityReason::new(
                            "recipetool.option_missing",
                            "Current Recipetool create does not expose --outfile.",
                            Some("Required option: --outfile".into()),
                        )
                        .unwrap(),
                    },
                    yoctui_model::CapabilityEvidenceOutcome::Negative,
                ),
                record(
                    yoctui_model::CapabilityId::RecipetoolAppendFile,
                    yoctui_model::CapabilityState::Available,
                    yoctui_model::CapabilityEvidenceOutcome::Positive,
                ),
            ],
        },
        implementations: std::collections::BTreeMap::from([(
            yoctui_model::CapabilityId::RecipetoolAppendFile,
            yoctui_model::CapabilityImplementation {
                id: yoctui_bitbake::RECIPETOOL_APPEND_FILE_IMPLEMENTATION.into(),
                kind: yoctui_model::CapabilityImplementationKind::Command,
            },
        )]),
    }
    .normalize()
    .unwrap();
    let actions = compatibility_recipetool_actions(Some(&snapshot));
    assert!(
        actions
            .iter()
            .find(|action| {
                action.capability == yoctui_model::CapabilityId::RecipetoolCreateOutfile
            })
            .unwrap()
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("does not expose --outfile"))
    );
    assert!(
        actions
            .iter()
            .find(|action| {
                action.capability == yoctui_model::CapabilityId::RecipetoolAppendFile
            })
            .unwrap()
            .available
    );
    assert!(
        compatibility_recipetool_actions(None)
            .iter()
            .all(|action| !action.available && action.reason.is_some())
    );
}
