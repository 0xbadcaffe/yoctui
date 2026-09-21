use super::*;

#[test]
fn compatibility_pkgdata_actions_distinguish_artifact_command_and_unknown_state() {
    let generated = yoctui_model::CapabilityId::PkgDataGenerated;
    let command = yoctui_model::CapabilityId::PkgDataListPackages;
    let snapshot = yoctui_model::DaemonCompatibilitySnapshot {
        snapshot: yoctui_model::CapabilitySnapshot {
            generation: 5,
            environment: yoctui_model::YoctoEnvironmentIdentity::default(),
            capabilities: vec![
                yoctui_model::CapabilityRecord {
                    id: generated,
                    state: yoctui_model::CapabilityState::Unavailable {
                        reason: yoctui_model::CapabilityReason::new(
                            "pkgdata.not_generated",
                            "Generated pkgdata is absent; build through do_package first.",
                            Some("Required artifact: tmp/pkgdata".into()),
                        )
                        .unwrap(),
                    },
                    evidence: vec![yoctui_model::CapabilityEvidence {
                        kind: yoctui_model::CapabilityEvidenceKind::Metadata,
                        outcome: yoctui_model::CapabilityEvidenceOutcome::Negative,
                        subject: "pkgdata artifact".into(),
                        detail: "No generated pkgdata directory was observed.".into(),
                        argv: Vec::new(),
                    }],
                },
                yoctui_model::CapabilityRecord {
                    id: command,
                    state: yoctui_model::CapabilityState::Available,
                    evidence: vec![yoctui_model::CapabilityEvidence {
                        kind: yoctui_model::CapabilityEvidenceKind::DirectProbe,
                        outcome: yoctui_model::CapabilityEvidenceOutcome::Positive,
                        subject: "oe-pkgdata-util list-pkgs --help".into(),
                        detail: "The command is available.".into(),
                        argv: vec!["/work/scripts/oe-pkgdata-util".into(), "--help".into()],
                    }],
                },
            ],
        },
        implementations: std::collections::BTreeMap::from([(
            command,
            yoctui_model::CapabilityImplementation {
                id: yoctui_bitbake::PKGDATA_LIST_PACKAGES_IMPLEMENTATION.into(),
                kind: yoctui_model::CapabilityImplementationKind::Command,
            },
        )]),
    }
    .normalize()
    .unwrap();
    let actions = compatibility_pkgdata_actions(Some(&snapshot));
    assert!(
        !actions
            .iter()
            .find(|action| action.capability == generated)
            .unwrap()
            .available
    );
    assert!(
        actions
            .iter()
            .find(|action| action.capability == command)
            .unwrap()
            .available
    );
    assert!(
        actions
            .iter()
            .find(|action| action.capability == yoctui_model::CapabilityId::PkgDataFindPath)
            .unwrap()
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("current environment capability snapshot"))
    );
}
