use super::*;
use crate::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityImplementation, CapabilityImplementationKind, CapabilityReason, CapabilityRecord,
    CapabilitySnapshot, IdentityAuthority, YoctoEnvironmentIdentity,
};

fn category(value: &str) -> RawCategoryId {
    RawCategoryId::new(value).unwrap()
}

fn command(value: &str) -> RawCommandId {
    RawCommandId::new(value).unwrap()
}

fn parameter(value: &str) -> RawParameterId {
    RawParameterId::new(value).unwrap()
}

fn executable(id: &str, label: &str, description: &str, target: bool) -> RawCommand {
    let parameters = target.then(|| RawParameter {
        id: parameter("target"),
        label: "Target".into(),
        placeholder: "<target>".into(),
        kind: RawParameterKind::Target,
        presence: RawParameterPresence::Required,
    });
    RawCommand {
        id: command(id),
        category: category("build"),
        label: label.into(),
        description: description.into(),
        reference: RawReference {
            id: RawReferenceId::new(format!("{id}.reference")).unwrap(),
            heading: "Build".into(),
            command: if target {
                "bitbake <target>".into()
            } else {
                "bitbake --version".into()
            },
            description: description.into(),
        },
        parameters: parameters.iter().cloned().collect(),
        execution: RawExecutionPolicy::Executable {
            template: RawExecutableTemplate {
                executable: RawExecutable::BitBake,
                arguments: if target {
                    vec![RawArgument::Parameter {
                        parameter: parameter("target"),
                    }]
                } else {
                    vec![RawArgument::Literal {
                        value: "--version".into(),
                    }]
                },
                capabilities: RawCapabilityRequirement::All {
                    capabilities: vec![CapabilityId::BitBakeRawCli],
                },
                interaction: RawInteractionMode::NoninteractiveJob,
                safety: RawSafetyClass::Inspection,
            },
        },
    }
}

fn catalog(version: u16) -> RawCatalog {
    RawCatalog {
        version,
        categories: vec![
            RawCategory {
                id: category("favorites"),
                label: "Favorites".into(),
                reference_heading: "Favorites".into(),
                kind: RawCategoryKind::Favorites,
            },
            RawCategory {
                id: category("build"),
                label: "Build commands".into(),
                reference_heading: "Build commands".into(),
                kind: RawCategoryKind::Executable,
            },
            RawCategory {
                id: category("reference"),
                label: "Reference material".into(),
                reference_heading: "Reference material".into(),
                kind: RawCategoryKind::ReferenceOnly,
            },
        ],
        commands: vec![
            executable(
                "build.target",
                "Build target",
                "Build one exact target.",
                true,
            ),
            executable(
                "build.version",
                "Show BitBake version",
                "Inspect the exact BitBake version.",
                false,
            ),
            RawCommand {
                id: command("reference.pipeline"),
                category: category("reference"),
                label: "Pipeline example".into(),
                description: "Reference-only pipeline explanation.".into(),
                reference: RawReference {
                    id: RawReferenceId::new("reference.pipeline.source").unwrap(),
                    heading: "Reference material".into(),
                    command: "bitbake target | tee log".into(),
                    description: "Reference-only pipeline explanation.".into(),
                },
                parameters: Vec::new(),
                execution: RawExecutionPolicy::ReferenceOnly {
                    kind: RawReferenceKind::ShellPipeline,
                    reason: "Shell pipelines are inert reference material.".into(),
                },
            },
        ],
    }
    .normalize()
    .unwrap()
}

fn authority(generation: u64, available: bool) -> DaemonCompatibilitySnapshot {
    let state = if available {
        CapabilityState::Available
    } else {
        CapabilityState::Unavailable {
            reason: CapabilityReason::new(
                "raw.test.unavailable",
                "Raw CLI probe is unavailable.",
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
                    detail: "Raw mode reducer fixture.".into(),
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

fn select_build_category(state: &mut RawModeState, catalog: &RawCatalog) {
    reduce_raw_mode(
        state,
        catalog,
        None,
        RawModeAction::SelectCategory { delta: 1 },
    );
    reduce_raw_mode(state, catalog, None, RawModeAction::FocusCommands);
}

mod raw_search_browsing_and_help_follow_exact_stable_selection;

mod raw_mode_form_preview_and_back_restore_exact_typed_state;

mod raw_form_editor_is_typed_bounded_and_invalidates_stale_values;

mod raw_mode_capability_replacement_closes_stale_preview_or_unsafe_form;

fn favorite_fixture(catalog: &RawCatalog, command_id: &str, order: u16) -> RawFavorite {
    let command = catalog.command(&command(command_id)).unwrap();
    let defaults = command
        .parameters
        .first()
        .map(|parameter| {
            BTreeMap::from([(
                parameter.id.clone(),
                RawParameterValue::Target("core-image-minimal".into()),
            )])
        })
        .unwrap_or_default();
    RawFavorite::new(
        command,
        format!("Favorite {command_id}"),
        defaults,
        RawAdditionalArguments::from_vec(vec![
            "--dry-run".into(),
            "value with space".into(),
            "quote'and\\slash".into(),
        ])
        .unwrap(),
        order,
    )
    .unwrap()
}

mod raw_favorite_validates_version_name_defaults_and_excludes_runtime_authority_by_shape;

mod raw_favorite_add_update_remove_reorder_and_bounds_are_atomic;

mod raw_favorite_projection_preserves_stale_and_reopens_only_a_fresh_form;

mod raw_history_catalog_replacement_retains_stale_records_without_replay;

mod raw_mode_empty_replacement_and_large_indices_never_panic;

mod raw_category_browser_pins_favorites_before_reference_order;
