//! Regression tests grouped around snapshot_timing_matches_live_batch_replacement_and_terminal_replay.
use super::*;

#[test]
fn snapshot_timing_matches_live_batch_replacement_and_terminal_replay() {
    use std::time::{Duration, UNIX_EPOCH};
    use yoctui_protocol::daemon::{
        DaemonBuildEvent as B, DaemonEvent, DaemonSnapshotJournal, DaemonSnapshotLimits,
    };
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        1,
        "boot".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut journal = DaemonSnapshotJournal::new(
        daemon_protocol_snapshot(&state),
        DaemonSnapshotLimits::default(),
    )
    .unwrap();
    let mut live = yoctui_model::App::new(64, 64 * 1024);
    let mut replica = DaemonClientSnapshot::default();
    replica.replace_app(&mut live, journal.snapshot().clone());
    for event in [
        B::Reset {
            targets: vec!["image".into()],
        },
        B::Started {
            started_unix_ms: Some(1000),
        },
    ] {
        let event = journal.publish(DaemonEvent::Build(event)).unwrap();
        replica.apply_event_to_app(&mut live, &event).unwrap();
    }
    let mut batch = yoctui_model::App::new(64, 64 * 1024);
    let mut batch_replica = DaemonClientSnapshot::default();
    batch_replica.replace_app(&mut batch, journal.snapshot().clone());
    let mut events = Vec::new();
    for recipe in ["llvm-native", "cli11"] {
        for event in [
            B::TaskStarted {
                recipe: recipe.into(),
                task: "do_compile".into(),
                pid: Some(42),
                worker: None,
                log_path: None,
                stats: None,
                started_unix_ms: Some(2000),
            },
            B::TaskCompleted {
                recipe: recipe.into(),
                task: "do_compile".into(),
                success: true,
                started_unix_ms: None,
                finished_unix_ms: Some(5000),
            },
            B::TaskCompleted {
                recipe: recipe.into(),
                task: "do_compile".into(),
                success: true,
                started_unix_ms: None,
                finished_unix_ms: Some(9000),
            },
        ] {
            let event = journal.publish(DaemonEvent::Build(event)).unwrap();
            replica.apply_event_to_app(&mut live, &event).unwrap();
            events.push(event);
        }
    }
    batch_replica
        .apply_task_events_to_app(&mut batch, &events)
        .unwrap();
    let mut fresh = yoctui_model::App::new(64, 64 * 1024);
    fresh.screen = Screen::Recipes;
    fresh.focus = FocusTarget::Inspector;
    DaemonClientSnapshot::default().replace_app(&mut fresh, journal.snapshot().clone());
    let now = UNIX_EPOCH + Duration::from_secs(100);
    for app in [&live, &batch, &fresh] {
        assert_eq!(
            app.build_summary_at(now).elapsed,
            Some(Duration::from_secs(99))
        );
        assert_eq!(app.completed_tasks.len(), 2);
        for row in &app.completed_tasks {
            assert_eq!(row.task.elapsed_at(now), Some(Duration::from_secs(3)));
        }
    }
    assert_eq!(fresh.screen, Screen::Recipes);
    assert_eq!(fresh.focus, FocusTarget::Inspector);
    let previous_record = yoctui_model::BuildRecord {
        target: Some("previous-image".into()),
        success: true,
        exit_code: Some(0),
        elapsed: Some(Duration::from_secs(12)),
        completed_tasks: 1,
        warnings: 0,
        errors: 0,
    };
    fresh.build_history.push_back(previous_record.clone());
    for finished in [6000, 99000] {
        let event = journal
            .publish(DaemonEvent::Build(B::Completed {
                success: true,
                exit_code: Some(0),
                finished_unix_ms: Some(finished),
            }))
            .unwrap();
        replica.apply_event_to_app(&mut live, &event).unwrap();
    }
    DaemonClientSnapshot::default().replace_app(&mut fresh, journal.snapshot().clone());
    DaemonClientSnapshot::default().replace_app(&mut fresh, journal.snapshot().clone());
    assert_eq!(fresh.build_history.len(), 2);
    assert_eq!(fresh.build_history[0], previous_record);
    replica.replace_app(&mut live, journal.snapshot().clone());
    for app in [&live, &fresh] {
        assert_eq!(
            app.build_summary_at(now).elapsed,
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            app.build_summary_at(now + Duration::from_secs(900)).elapsed,
            Some(Duration::from_secs(5))
        );
    }
    let event = journal
        .publish(DaemonEvent::Build(B::Reset {
            targets: vec!["next".into()],
        }))
        .unwrap();
    replica.apply_event_to_app(&mut live, &event).unwrap();
    assert_eq!(live.build.started, None);
    assert!(live.completed_tasks.is_empty());
}

#[test]
fn snapshot_timing_invalid_reversed_and_missing_times_are_unavailable() {
    use std::time::{Duration, UNIX_EPOCH};
    use yoctui_protocol::daemon::DaemonBuildEvent as B;
    for (start, end) in [
        (None, Some(5000)),
        (Some(2000), None),
        (Some(6000), Some(5000)),
        (Some(u64::MAX), Some(u64::MAX)),
    ] {
        let mut app = yoctui_model::App::new(64, 64 * 1024);
        for event in [
            B::Started {
                started_unix_ms: start,
            },
            B::TaskCompleted {
                recipe: "llvm-native".into(),
                task: "do_compile".into(),
                success: true,
                started_unix_ms: start,
                finished_unix_ms: end,
            },
            B::Completed {
                success: true,
                exit_code: Some(0),
                finished_unix_ms: end,
            },
        ] {
            apply_daemon_build_event(&mut app, event);
        }
        let now = UNIX_EPOCH + Duration::from_secs(100);
        assert_eq!(app.build_summary_at(now).elapsed, None);
        assert_eq!(app.completed_tasks[0].task.elapsed_at(now), None);
    }
    let mut app = yoctui_model::App::new(64, 64 * 1024);
    apply_daemon_build_event(
        &mut app,
        B::Started {
            started_unix_ms: Some(2000),
        },
    );
    assert_eq!(app.build_summary_at(UNIX_EPOCH).elapsed, None);
}

#[test]
fn daemon_attach_build_restores_and_updates_typed_progress_without_replacing_presentation() {
    use std::collections::HashMap;
    use yoctui_protocol::daemon::{DaemonBuildEvent, DaemonEvent, SequencedEvent};
    use yoctui_protocol::{TaskStatsData, WorkspaceData};

    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    snapshot.build_events = vec![
        DaemonBuildEvent::Reset {
            targets: vec!["core-image-minimal".into()],
        },
        DaemonBuildEvent::Workspace {
            data: WorkspaceData {
                build_dir: Some("/work/build".into()),
                source_dir: Some("/work/poky".into()),
                variables: HashMap::from([
                    ("MACHINE".into(), "qemux86-64".into()),
                    ("DISTRO".into(), "poky".into()),
                ]),
                variable_provenance: HashMap::new(),
                variable_provenance_chain: HashMap::new(),
                bitbake_version: Some("2.8.1".into()),
                release: Some("5.0.19".into()),
                layers: Vec::new(),
                recipes: Vec::new(),
            },
        },
        DaemonBuildEvent::Started {
            started_unix_ms: None,
        },
        DaemonBuildEvent::TaskStarted {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            pid: Some(42),
            worker: Some("worker-1".into()),
            log_path: None,
            stats: Some(TaskStatsData {
                completed: 102,
                total: 4090,
                active: 8,
                failed: 0,
            }),
            started_unix_ms: None,
        },
        DaemonBuildEvent::TaskProgress {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            progress: Some(77),
        },
    ];
    let mut app = yoctui_model::App::new(64, 64 * 1024);
    app.screen = Screen::Recipes;
    app.focus = FocusTarget::Inspector;
    app.theme = yoctui_model::Theme::MatrixGreen;
    app.dialogs
        .push_back(yoctui_model::Dialog::QuitConfirmation);
    let mut replica = DaemonClientSnapshot::default();
    replica.replace_app(&mut app, snapshot);

    assert_eq!(app.backend, "bridge");
    assert!(matches!(
        app.build_environment,
        yoctui_model::BuildEnvironmentState::Connected(ref profile)
            if profile.source_dir == std::path::Path::new("/work/poky")
                && profile.build_dir == std::path::Path::new("/work/build")
    ));
    assert_eq!(app.build.status, yoctui_model::BuildStatus::Running);
    assert_eq!(app.build.target.as_deref(), Some("core-image-minimal"));
    assert_eq!((app.build.completed, app.build.total), (102, Some(4090)));
    assert_eq!(
        app.tasks[&yoctui_model::TaskId("busybox:do_compile".into())].progress,
        Some(77)
    );
    assert_eq!(app.workspace.release.as_deref(), Some("5.0.19"));
    assert_eq!(app.screen, Screen::Recipes);
    assert_eq!(app.focus, FocusTarget::Inspector);
    assert_eq!(app.theme, yoctui_model::Theme::MatrixGreen);
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::QuitConfirmation)
    ));

    replica
        .apply_event_to_app(
            &mut app,
            &SequencedEvent {
                sequence: 1,
                generation: 1,
                event: DaemonEvent::Build(DaemonBuildEvent::TaskProgress {
                    recipe: "busybox".into(),
                    task: "do_compile".into(),
                    progress: Some(88),
                }),
            },
        )
        .unwrap();
    assert_eq!(
        app.tasks[&yoctui_model::TaskId("busybox:do_compile".into())].progress,
        Some(88)
    );
}

#[test]
fn daemon_attach_uses_top_level_workspace_identity_without_build_event() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([7; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    snapshot.workspace = Some(yoctui_protocol::daemon::WorkspaceIdentity {
        canonical_source: "/srv/yocto/poky".into(),
        canonical_build: "/srv/yocto/build".into(),
        identity_hash: "workspace-identity".into(),
    });
    assert!(snapshot.build_events.is_empty());

    let mut app = yoctui_model::App::new(64, 64 * 1024);
    app.workspace.build_dir = Some("/client/stale-build".into());
    app.screen = Screen::Layers;
    app.focus = FocusTarget::Inspector;
    let mut replica = DaemonClientSnapshot::default();
    replica.replace_app(&mut app, snapshot);

    assert_eq!(
        app.workspace.source_dir.as_deref(),
        Some(std::path::Path::new("/srv/yocto/poky"))
    );
    assert_eq!(
        app.workspace.build_dir.as_deref(),
        Some(std::path::Path::new("/srv/yocto/build"))
    );
    assert!(matches!(
        app.build_environment,
        yoctui_model::BuildEnvironmentState::Connected(ref profile)
            if profile.source_dir == std::path::Path::new("/srv/yocto/poky")
                && profile.build_dir == std::path::Path::new("/srv/yocto/build")
    ));
    assert_eq!(app.screen, Screen::Layers);
    assert_eq!(app.focus, FocusTarget::Inspector);
}

#[test]
fn security_workflow_maps_typed_mapper_events_without_parsing_output() {
    let id = yoctui_model::SecuritySessionId(7);
    assert_eq!(
        security_actions_for_mapper_event(
            SecurityMapperRunnerEvent::Started { id },
            SystemTime::UNIX_EPOCH,
        ),
        [Action::Security(SecurityAction::SessionRunning(id))]
    );
    assert_eq!(
        security_actions_for_mapper_event(
            SecurityMapperRunnerEvent::Output {
                id,
                stream: SecurityOutputStream::Stderr,
                line: "package=busybox product=busybox".into(),
                truncated: true,
            },
            SystemTime::UNIX_EPOCH,
        ),
        [Action::Security(SecurityAction::SessionOutput {
            id,
            stream: SecurityOutputStream::Stderr,
            line: "package=busybox product=busybox".into(),
            truncated: true,
        })]
    );
    assert!(matches!(
        security_actions_for_mapper_event(
            SecurityMapperRunnerEvent::TimedOut {
                id,
                forced: true,
                exit_code: None,
            },
            SystemTime::UNIX_EPOCH,
        )
        .as_slice(),
        [
            Action::Security(SecurityAction::SessionOutput { id: output_id, .. }),
            Action::Security(SecurityAction::TimeoutSession { id: timeout_id, .. }),
        ] if *output_id == id && *timeout_id == id
    ));
    assert_eq!(
        security_actions_for_mapper_event(
            SecurityMapperRunnerEvent::Lost {
                id,
                message: "worker channel closed".into(),
            },
            SystemTime::UNIX_EPOCH,
        ),
        [Action::Security(SecurityAction::LoseSession {
            id,
            message: "worker channel closed".into(),
            finished_at: SystemTime::UNIX_EPOCH,
        })]
    );
}

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

#[test]
fn raw_selector_app_projection_replaces_workspace_and_target_inventories() {
    let mut app = yoctui_model::App::new(10, 1_000);
    let absent = raw_selector_authority(&app, None);
    assert!(matches!(
        absent.recipes,
        yoctui_model::RawSelectorInventory::Unavailable { .. }
    ));

    app.workspace.build_dir = Some("/build".into());
    app.workspace.recipes = vec![yoctui_model::Recipe {
        name: "busybox".into(),
        file: Some("/layers/meta/recipes-core/busybox/busybox.bb".into()),
        ..yoctui_model::Recipe::default()
    }];
    app.available_images = vec!["core-image-minimal".into()];
    app.workspace
        .variables
        .insert("BBMULTICONFIG".into(), "lib32 board1".into());
    app.build.target = Some("core-image-minimal".into());
    app.build_history.push_back(yoctui_model::BuildRecord {
        target: Some("busybox".into()),
        success: true,
        exit_code: Some(0),
        elapsed: Some(std::time::Duration::from_secs(1)),
        completed_tasks: 1,
        warnings: 0,
        errors: 0,
    });

    let initial = raw_selector_authority(&app, None);
    assert_eq!(initial.recipes.choices().unwrap().len(), 1);
    assert_eq!(initial.images.choices().unwrap().len(), 1);
    assert_eq!(initial.targets.choices().unwrap().len(), 2);
    assert_eq!(initial.multiconfigs.choices().unwrap().len(), 2);

    app.workspace.recipes = vec![yoctui_model::Recipe {
        name: "bash".into(),
        file: Some("/layers/meta/recipes-extended/bash/bash.bb".into()),
        ..yoctui_model::Recipe::default()
    }];
    app.available_images.clear();
    app.build.target = Some("bash".into());
    app.build_history.clear();
    let replaced = raw_selector_authority(&app, None);
    assert_eq!(
        replaced.recipes.choices().unwrap()[0].value,
        yoctui_model::RawParameterValue::Recipe("bash".into())
    );
    assert_eq!(replaced.images.choices().unwrap().len(), 0);
    assert_eq!(replaced.targets.choices().unwrap().len(), 1);
}

#[test]
fn raw_selector_app_projection_rejects_stale_tasks_but_keeps_valid_manual_entry() {
    let mut app = yoctui_model::App::new(10, 1_000);
    app.recipe_metadata.insert(
        "alpha".into(),
        yoctui_model::RecipeMetadata {
            recipe: "alpha".into(),
            tasks: Some(vec!["do_compile".into()]),
            ..yoctui_model::RecipeMetadata::default()
        },
    );
    let (command, parameter) = raw_selector_command(yoctui_model::RawParameterKind::Task);

    let alpha = raw_selector_for_command(&app, &command, &parameter, Some("alpha")).unwrap();
    assert_eq!(alpha.inventory.choices().unwrap().len(), 1);

    let beta = raw_selector_for_command(&app, &command, &parameter, Some("beta")).unwrap();
    assert!(matches!(
        beta.inventory,
        yoctui_model::RawSelectorInventory::Unavailable { .. }
    ));
    assert!(beta.manual_entry);
    assert_eq!(
        beta.parse_manual("do_install").unwrap(),
        Some(yoctui_model::RawParameterValue::Task("do_install".into()))
    );
    assert!(beta.parse_manual("do_install;touch").is_err());

    app.recipe_metadata.insert(
        "beta".into(),
        yoctui_model::RecipeMetadata {
            recipe: "beta".into(),
            tasks: Some(Vec::new()),
            ..yoctui_model::RecipeMetadata::default()
        },
    );
    let empty = raw_selector_for_command(&app, &command, &parameter, Some("beta")).unwrap();
    assert_eq!(empty.inventory.choices().unwrap().len(), 0);

    app.recipe_metadata_loading.insert("beta".into());
    let loading = raw_selector_for_command(&app, &command, &parameter, Some("beta")).unwrap();
    assert!(matches!(
        loading.inventory,
        yoctui_model::RawSelectorInventory::Unavailable { .. }
    ));
}

#[test]
fn raw_argv_app_mapping_uses_shared_editor_commands_and_explicit_validation() {
    assert_eq!(
        raw_argv_editor_action(false, Input::Char('i')),
        Some(RawArgvEditorAction::Edit(
            yoctui_model::PopupEditorCommand::ToggleInsert
        ))
    );
    assert_eq!(
        raw_argv_editor_action(true, Input::Char('x')),
        Some(RawArgvEditorAction::Edit(
            yoctui_model::PopupEditorCommand::Insert('x')
        ))
    );
    assert_eq!(
        raw_argv_editor_action(true, Input::Esc),
        Some(RawArgvEditorAction::Edit(
            yoctui_model::PopupEditorCommand::ToggleInsert
        ))
    );
    assert_eq!(
        raw_argv_editor_action(false, Input::Enter),
        Some(RawArgvEditorAction::Validate)
    );
    assert_eq!(
        raw_argv_editor_action(false, Input::Char('x')),
        Some(RawArgvEditorAction::Edit(
            yoctui_model::PopupEditorCommand::Delete
        ))
    );
}

#[test]
fn raw_argv_app_validation_returns_native_elements_and_replaces_stale_results() {
    let mut editor = yoctui_model::RawArgvEditor::new("--flag 'two words'").unwrap();
    assert_eq!(
        validate_raw_argv_editor(&mut editor).unwrap(),
        ["--flag", "two words"]
    );

    editor.replace_input("left|right").unwrap();
    assert!(validate_raw_argv_editor(&mut editor).is_err());
    assert!(editor.validated.is_none());

    editor.replace_input("--next escaped\\ value").unwrap();
    assert_eq!(
        validate_raw_argv_editor(&mut editor).unwrap(),
        ["--next", "escaped value"]
    );
}

#[test]
fn raw_search_app_maps_browser_history_and_favorite_input_mechanically() {
    let catalog = yoctui_model::RawCatalog::builtin();
    let mut state = yoctui_model::RawModeState::new(&catalog);
    assert_eq!(
        raw_mode_input(&state, Input::Right),
        Some(yoctui_model::RawModeAction::FocusCommands)
    );
    assert_eq!(
        raw_mode_input(&state, Input::Down),
        Some(yoctui_model::RawModeAction::SelectCategory { delta: 1 })
    );
    assert_eq!(
        raw_mode_input(&state, Input::Char('/')),
        Some(yoctui_model::RawModeAction::BeginSearch)
    );
    reduce_raw_mode_state(
        &mut state,
        &catalog,
        None,
        yoctui_model::RawModeAction::BeginSearch,
    );
    assert_eq!(
        raw_mode_input(&state, Input::Char('x')),
        Some(yoctui_model::RawModeAction::AppendSearch('x'))
    );
    assert_eq!(
        raw_mode_input(&state, Input::Esc),
        Some(yoctui_model::RawModeAction::FinishSearch)
    );

    state.search.editing = false;
    state.view = yoctui_model::RawModeView::History;
    assert_eq!(
        raw_mode_input(&state, Input::Enter),
        Some(yoctui_model::RawModeAction::ActivateHistory)
    );
    state.view = yoctui_model::RawModeView::Favorites;
    assert_eq!(
        raw_mode_input(&state, Input::Down),
        Some(yoctui_model::RawModeAction::SelectFavorite { delta: 1 })
    );
    state.favorite_confirmation = Some(yoctui_model::RawFavoriteConfirmation {
        command: catalog.commands[0].id.clone(),
        return_focus: yoctui_model::RawModeFocus::Favorites,
    });
    assert_eq!(
        raw_mode_input(&state, Input::Enter),
        Some(yoctui_model::RawModeAction::ConfirmFavorite)
    );
    assert_eq!(
        raw_mode_input(&state, Input::Esc),
        Some(yoctui_model::RawModeAction::CancelFavorite)
    );
    assert_eq!(raw_mode_input(&state, Input::Down), None);
}
