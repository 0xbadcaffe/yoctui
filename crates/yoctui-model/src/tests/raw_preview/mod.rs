use super::*;
use crate::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityImplementation, CapabilityImplementationKind, CapabilityReason, CapabilityRecord,
    CapabilitySnapshot, IdentityAuthority, YoctoEnvironmentIdentity,
};

fn id(value: &str) -> RawParameterId {
    RawParameterId::new(value).unwrap()
}

pub(super) fn catalog() -> RawCatalog {
    RawCatalog {
        version: 7,
        categories: vec![RawCategory {
            id: RawCategoryId::new("preview").unwrap(),
            label: "Preview".into(),
            reference_heading: "Preview".into(),
            kind: RawCategoryKind::Executable,
        }],
        commands: vec![RawCommand {
            id: RawCommandId::new("preview.run").unwrap(),
            category: RawCategoryId::new("preview").unwrap(),
            label: "bitbake -c <task> --ui=<ui> mc:<config>:<target> ''".into(),
            description: "Preview fixture command.".into(),
            reference: RawReference {
                id: RawReferenceId::new("preview.reference").unwrap(),
                heading: "Preview".into(),
                command: "bitbake -c <task> --ui=<ui> mc:<config>:<target> ''".into(),
                description: "Preview fixture command.".into(),
            },
            parameters: vec![
                RawParameter {
                    id: id("task"),
                    label: "Task".into(),
                    placeholder: "<task>".into(),
                    kind: RawParameterKind::Task,
                    presence: RawParameterPresence::Required,
                },
                RawParameter {
                    id: id("ui"),
                    label: "UI".into(),
                    placeholder: "<ui>".into(),
                    kind: RawParameterKind::UserInterface,
                    presence: RawParameterPresence::Optional,
                },
                RawParameter {
                    id: id("config"),
                    label: "Config".into(),
                    placeholder: "<config>".into(),
                    kind: RawParameterKind::Multiconfig,
                    presence: RawParameterPresence::Required,
                },
                RawParameter {
                    id: id("target"),
                    label: "Target".into(),
                    placeholder: "<target>".into(),
                    kind: RawParameterKind::Target,
                    presence: RawParameterPresence::Required,
                },
            ],
            execution: RawExecutionPolicy::Executable {
                template: RawExecutableTemplate {
                    executable: RawExecutable::BitBake,
                    arguments: vec![
                        RawArgument::Literal { value: "-c".into() },
                        RawArgument::Parameter {
                            parameter: id("task"),
                        },
                        RawArgument::JoinedParameter {
                            prefix: "--ui=".into(),
                            parameter: id("ui"),
                        },
                        RawArgument::Composed {
                            segments: vec![
                                RawArgumentSegment::Literal {
                                    value: "mc:".into(),
                                },
                                RawArgumentSegment::Parameter {
                                    parameter: id("config"),
                                },
                                RawArgumentSegment::Literal { value: ":".into() },
                                RawArgumentSegment::Parameter {
                                    parameter: id("target"),
                                },
                            ],
                        },
                        RawArgument::Empty,
                    ],
                    capabilities: RawCapabilityRequirement::All {
                        capabilities: vec![CapabilityId::BitBakeRawCli],
                    },
                    interaction: RawInteractionMode::NoninteractiveJob,
                    safety: RawSafetyClass::Build,
                },
            },
        }],
    }
    .normalize()
    .unwrap()
}

pub(super) fn authority(generation: u64, available: bool) -> DaemonCompatibilitySnapshot {
    let state = if available {
        CapabilityState::AvailableWithLimitations {
            reason: CapabilityReason::new("preview.limited", "Preview fixture is limited.", None)
                .unwrap(),
            limitations: vec!["Exact fixture limitation.".into()],
        }
    } else {
        CapabilityState::Unavailable {
            reason: CapabilityReason::new(
                "preview.unavailable",
                "Preview fixture is unavailable.",
                None,
            )
            .unwrap(),
        }
    };
    DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation,
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    "/work/build".into(),
                    IdentityAuthority::InitializedEnvironment,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities: vec![CapabilityRecord {
                id: CapabilityId::BitBakeRawCli,
                state,
                evidence: vec![CapabilityEvidence {
                    kind: CapabilityEvidenceKind::DirectProbe,
                    outcome: if available {
                        CapabilityEvidenceOutcome::Positive
                    } else {
                        CapabilityEvidenceOutcome::Negative
                    },
                    subject: "bitbake --help".into(),
                    detail: "Exact fixture evidence.".into(),
                    argv: vec!["bitbake".into(), "--help".into()],
                }],
            }],
        },
        implementations: if available {
            BTreeMap::from([(
                CapabilityId::BitBakeRawCli,
                CapabilityImplementation {
                    id: "bitbake.raw.argv".into(),
                    kind: CapabilityImplementationKind::Command,
                },
            )])
        } else {
            BTreeMap::new()
        },
    }
    .normalize()
    .unwrap()
}

pub(super) fn request() -> RawPreviewRequest {
    RawPreviewRequest {
        catalog_version: 7,
        command: RawCommandId::new("preview.run").unwrap(),
        parameters: BTreeMap::from([
            (id("task"), RawParameterValue::Task("do_compile".into())),
            (id("config"), RawParameterValue::Multiconfig("lib32".into())),
            (
                id("target"),
                RawParameterValue::Target("virtual/kernel".into()),
            ),
        ]),
        additional_arguments: RawAdditionalArguments::parse("--verbose 'café value'").unwrap(),
        capability_generation: 9,
        build_directory: "/work/build".into(),
    }
}

mod raw_preview_reconstructs_exact_indexed_native_arguments_and_metadata;

mod raw_preview_includes_optional_joined_value_and_preserves_explicit_empty;

mod raw_preview_rejects_missing_extra_and_mismatched_parameters;

mod raw_preview_fails_closed_for_missing_stale_or_unavailable_authority;
