//! Regression tests grouped around daemon_recovery_restores_metadata_without_claiming_live_bitbake_or_profile.
use super::*;

#[cfg(unix)]
#[test]
fn daemon_recovery_restores_metadata_without_claiming_live_bitbake_or_profile() {
    let mut state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([5; 16]),
        123,
        "current-boot".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    snapshot.workspace = Some(yoctui_protocol::daemon::WorkspaceIdentity {
        canonical_source: "/work/poky".into(),
        canonical_build: "/work/poky/build".into(),
        identity_hash: "identity".into(),
    });
    snapshot.project_profile =
        yoctui_protocol::daemon::ProjectProfileSummary::Loaded { schema_version: 1 };
    snapshot.bitbake.version = Some("2.8.1".into());
    snapshot.bitbake.capabilities =
        vec![yoctui_protocol::daemon::BitBakeCapability::WorkspaceInspection];
    let persisted = yoctui_protocol::daemon_persist::DaemonPersistedState::capture(
        &snapshot,
        1,
        "previous-boot".into(),
        Vec::new(),
        yoctui_protocol::daemon_persist::PersistedPreferences::default(),
    );

    recover_daemon_model_metadata(&mut state, &persisted, "current-boot").unwrap();
    assert_eq!(
        state.workspace.build_dir.as_deref(),
        Some(std::path::Path::new("/work/poky/build"))
    );
    assert_eq!(
        state.project_profile,
        yoctui_model::ProjectProfileState::NotLoaded
    );
    assert_eq!(
        state.bitbake.lifecycle,
        yoctui_model::DaemonBitBakeLifecycle::Disconnected
    );
    assert_eq!(
        state.session.recovery,
        yoctui_model::DaemonRecoveryState::Degraded
    );
    assert!(
        state
            .session
            .recovery_warnings
            .iter()
            .any(|warning| warning.contains("must be reloaded"))
    );
}

#[test]
fn project_profile_generation_crosses_app_boundary_as_typed_effect() {
    let profile = yoctui_model::ProjectProfile {
        schema_version: yoctui_model::PROJECT_PROFILE_SCHEMA_VERSION,
        favorites: yoctui_model::ProjectFavorites::default(),
        build_presets: Vec::new(),
        workflows: Vec::new(),
    };
    let mut app = App::new(8, 512);
    assert_eq!(
        update(
            &mut app,
            Action::PreviewProjectProfileGeneration(profile.clone())
        ),
        None
    );
    assert_eq!(
        update(
            &mut app,
            Action::ConfirmProjectProfileGeneration { replace: false }
        ),
        Some(yoctui_model::Effect::GenerateProjectProfile {
            profile: profile.clone(),
            replace: false,
        })
    );
    let _ = update(&mut app, Action::ProjectProfileGenerated(profile.clone()));
    assert_eq!(
        app.project_profile,
        yoctui_model::ProjectProfileState::Loaded(profile)
    );
}

#[test]
fn project_profile_input_selects_and_previews_without_starting() {
    assert_eq!(
        build_environment_action(Input::Char('n')),
        Some(Action::SelectProjectProfileItem { delta: 1 })
    );
    assert_eq!(
        build_environment_action(Input::Char('p')),
        Some(Action::ActivateProjectProfileItem)
    );
    let profile = yoctui_model::ProjectProfile {
        schema_version: yoctui_model::PROJECT_PROFILE_SCHEMA_VERSION,
        favorites: yoctui_model::ProjectFavorites::default(),
        build_presets: vec![yoctui_model::ProjectBuildPreset {
            name: "minimal".into(),
            targets: vec!["core-image-minimal".into()],
            machine: None,
            distro: None,
            options: yoctui_model::ProjectBuildOptions::default(),
        }],
        workflows: Vec::new(),
    };
    let mut app = App::new(8, 512);
    app.workspace.recipes.push(yoctui_model::Recipe {
        name: "core-image-minimal".into(),
        ..yoctui_model::Recipe::default()
    });
    let _ = update(&mut app, Action::ProjectProfileLoaded(profile));
    assert_eq!(update(&mut app, Action::ActivateProjectProfileItem), None);
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::RecipeTaskConfirmation(request))
            if request.targets == ["core-image-minimal"]
    ));
    assert!(matches!(app.build.status, yoctui_model::BuildStatus::Idle));
}

#[test]
fn maintenance_workflow_maps_first_class_screen_and_typed_keys() {
    let mut app = App::new(10, 1_000);
    assert_eq!(
        update(&mut app, Action::Open(Screen::Maintenance)),
        Some(yoctui_model::Effect::Maintenance(
            yoctui_model::MaintenanceEffect::InspectCapability { request: 1 }
        ))
    );
    assert_eq!(app.screen, Screen::Maintenance);
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Sstate, 4, Input::Char(']')),
        Some(Action::Maintenance(MaintenanceAction::CycleView {
            backwards: false
        }))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Sstate, 4, Input::Down),
        Some(Action::Maintenance(MaintenanceAction::Select {
            delta: 1,
            row_count: 4
        }))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Sstate, 4, Input::Char('S')),
        Some(Action::Maintenance(MaintenanceAction::OpenSignatures))
    );
}

#[test]
fn maintenance_workflow_dialog_mapping_traps_typed_input() {
    let preview = maintenance_preview();
    assert_eq!(
        maintenance_dialog_action(&MaintenanceDialog::Confirm(preview.clone()), Input::Enter),
        Some(Action::Maintenance(MaintenanceAction::ConfirmOperation(
            preview.clone()
        )))
    );
    assert_eq!(
        maintenance_dialog_action(
            &MaintenanceDialog::CleanupPhrase {
                preview: preview.clone(),
                input: "DELETE".into()
            },
            Input::Char(' ')
        ),
        Some(Action::Maintenance(
            MaintenanceAction::UpdateCleanupPhrase {
                preview: preview.clone(),
                input: "DELETE ".into()
            }
        ))
    );
    assert_eq!(
        maintenance_dialog_action(&MaintenanceDialog::ConfirmNetworkPush(preview), Input::Esc),
        Some(Action::Maintenance(MaintenanceAction::CancelDialog))
    );
}

#[test]
fn maintenance_sstate_workspace_maps_only_typed_form_input() {
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Sstate, 2, Input::Char('c')),
        Some(Action::Maintenance(MaintenanceAction::OpenReadinessForm))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Sstate, 2, Input::Char('d')),
        Some(Action::Maintenance(MaintenanceAction::OpenCleanupForm))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Release, 4, Input::Char('c')),
        None
    );

    let mut readiness = yoctui_model::PopupEditor::new(
        "targets = \"\"\nmode = \"isolated_tmpdir\"\ntimeout = 3600\n".into(),
    );
    readiness.select_range(11, 11);
    readiness.editing = true;
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::ReadinessToml {
                editor: readiness.clone(),
                validation_error: None,
            },
            Input::Char('c'),
        ),
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('c')))
    ));
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::ReadinessToml {
                editor: readiness.clone(),
                validation_error: None,
            },
            Input::Enter,
        ),
        Some(Action::Maintenance(
            MaintenanceAction::ConfirmReadinessToml(document)
        )) if document == readiness.text
    ));

    let cleanup = yoctui_model::PopupEditor::new(
        "duplicates = true\norphans = false\nunreferenced_by_stamps = false\njobs = 1\n".into(),
    );
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::CleanupToml {
                editor: cleanup.clone(),
                validation_error: None,
            },
            Input::Char('e'),
        ),
        Some(Action::EditActivePopup(PopupEditorCommand::SelectValue))
    ));
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::CleanupToml {
                editor: cleanup.clone(),
                validation_error: None,
            },
            Input::Enter,
        ),
        Some(Action::Maintenance(MaintenanceAction::ConfirmCleanupToml(document)))
            if document == cleanup.text
    ));
    assert_eq!(
        maintenance_dialog_action(
            &MaintenanceDialog::CleanupToml {
                editor: cleanup,
                validation_error: None,
            },
            Input::Esc,
        ),
        Some(Action::Maintenance(MaintenanceAction::CancelDialog))
    );
}

#[test]
fn maintenance_service_workspace_maps_distinct_export_and_import_forms() {
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Services, 1, Input::Char('e')),
        Some(Action::Maintenance(MaintenanceAction::OpenPrServiceForm(
            yoctui_model::PrServiceOperation::Export,
        )))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Services, 1, Input::Char('m')),
        Some(Action::Maintenance(MaintenanceAction::OpenPrServiceForm(
            yoctui_model::PrServiceOperation::Import,
        )))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Sstate, 2, Input::Char('e')),
        None
    );
    let mut editor = yoctui_model::PopupEditor::new("file = \"\"\n".into());
    editor.select_range(8, 8);
    editor.editing = true;
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::PrServiceToml {
                operation: yoctui_model::PrServiceOperation::Import,
                editor: editor.clone(),
                validation_error: None,
            },
            Input::Char('/'),
        ),
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('/')))
    ));
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::PrServiceToml {
                operation: yoctui_model::PrServiceOperation::Import,
                editor: editor.clone(),
                validation_error: None,
            },
            Input::Enter,
        ),
        Some(Action::Maintenance(MaintenanceAction::ConfirmPrServiceToml {
            operation: yoctui_model::PrServiceOperation::Import,
            document,
        })) if document == editor.text
    ));
}

#[test]
fn maintenance_release_locked_workspace_maps_only_typed_form_input() {
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Release, 3, Input::Char('l')),
        Some(Action::Maintenance(MaintenanceAction::OpenLockedCacheForm))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Services, 3, Input::Char('l')),
        None
    );
    let mut editor = yoctui_model::PopupEditor::new(
        "locked_signatures = \"\"\ninput_cache = \"\"\noutput_cache = \"\"\nfilter = \"\"\n".into(),
    );
    editor.select_range(21, 21);
    editor.editing = true;
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::LockedCacheToml {
                editor: editor.clone(),
                validation_error: None,
            },
            Input::Char('/'),
        ),
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('/')))
    ));
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::LockedCacheToml {
                editor: editor.clone(),
                validation_error: None,
            },
            Input::Enter,
        ),
        Some(Action::Maintenance(MaintenanceAction::ConfirmLockedCacheToml(document)))
            if document == editor.text
    ));
    editor.editing = false;
    assert_eq!(
        maintenance_dialog_action(
            &MaintenanceDialog::LockedCacheToml {
                editor,
                validation_error: None,
            },
            Input::Esc,
        ),
        Some(Action::Maintenance(MaintenanceAction::CancelDialog))
    );
}

#[test]
fn maintenance_release_history_workspace_maps_fields_and_toggles_without_leakage() {
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Release, 3, Input::Char('h')),
        Some(Action::Maintenance(MaintenanceAction::OpenBuildHistoryForm))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Sstate, 3, Input::Char('h')),
        None
    );
    let mut editor =
        yoctui_model::PopupEditor::new("from_revision = \"\"\nreport_version = false\n".into());
    editor.editing = true;
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::BuildHistoryToml {
                editor: editor.clone(),
                validation_error: None
            },
            Input::Char('H'),
        ),
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('H')))
    ));
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::BuildHistoryToml {
                editor: editor.clone(),
                validation_error: None
            },
            Input::Enter,
        ),
        Some(Action::Maintenance(
            MaintenanceAction::ConfirmBuildHistoryToml(_)
        ))
    ));
    editor.editing = false;
    assert_eq!(
        maintenance_dialog_action(
            &MaintenanceDialog::BuildHistoryToml {
                editor,
                validation_error: None
            },
            Input::Esc,
        ),
        Some(Action::Maintenance(MaintenanceAction::CancelDialog))
    );
}

#[test]
fn maintenance_release_archive_workspace_maps_text_toggles_and_cancel() {
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Release, 3, Input::Char('a')),
        Some(Action::Maintenance(MaintenanceAction::OpenGitArchiveForm))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Services, 3, Input::Char('a')),
        None
    );
    let mut editor = yoctui_model::PopupEditor::new(
        "data_dir = \"\"\ncreate = true\npush_remote = \"\"\n".into(),
    );
    editor.editing = true;
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::GitArchiveToml {
                editor: editor.clone(),
                validation_error: None,
            },
            Input::Char('/'),
        ),
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('/')))
    ));
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::GitArchiveToml {
                editor: editor.clone(),
                validation_error: None,
            },
            Input::Enter,
        ),
        Some(Action::Maintenance(MaintenanceAction::ConfirmGitArchiveToml(document)))
            if document == editor.text
    ));
    editor.editing = false;
    assert_eq!(
        maintenance_dialog_action(
            &MaintenanceDialog::GitArchiveToml {
                editor,
                validation_error: None,
            },
            Input::Esc,
        ),
        Some(Action::Maintenance(MaintenanceAction::CancelDialog))
    );
}

#[test]
fn background_job_build_events_survive_navigation_and_complete() {
    let mut coordinator = BuildJobCoordinator::default();
    let mut app = App::new(10, 1_000);
    apply_actions(
        &mut app,
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .unwrap(),
    );
    let id = coordinator.active_job_id().unwrap();
    assert_eq!(
        app.background_jobs.get(id).unwrap().status,
        BackgroundJobStatus::Starting
    );
    apply_actions(
        &mut app,
        coordinator.actions_for_backend_event(
            BackendEvent::BuildStarted,
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
        ),
    );
    let _ = update(&mut app, Action::Open(Screen::Layers));
    let log = yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Warning,
        message: "cache miss".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: None,
        timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
        build: None,
        protected: false,
        diagnostic: None,
    };
    apply_actions(
        &mut app,
        coordinator.actions_for_backend_event(
            BackendEvent::Log(log),
            SystemTime::UNIX_EPOCH + Duration::from_secs(2),
        ),
    );
    apply_actions(
        &mut app,
        coordinator.actions_for_backend_event(
            BackendEvent::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
            SystemTime::UNIX_EPOCH + Duration::from_secs(3),
        ),
    );

    let job = app.background_jobs.get(id).unwrap();
    assert_eq!(app.screen, Screen::Layers);
    assert_eq!(app.build.status, BuildStatus::Completed);
    assert_eq!(job.status, BackgroundJobStatus::Succeeded);
    assert_eq!(job.output.len(), 1);
    assert_eq!(job.warnings, 1);
    assert_eq!(coordinator.active_job_id(), None);
}

#[test]
fn typed_event_maps_every_metadata_family_and_ignores_future_events() {
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::Recipes(vec![])),
        Some(Action::RecipesLoaded(_))
    ));
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::Layers(vec![])),
        Some(Action::LayersLoaded(_))
    ));
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::Variable {
            name: "MACHINE".into(),
            recipe: None,
            value: Some("qemux86-64".into()),
            provenance: Some("conf/local.conf:1".into()),
            unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
            operations: vec![],
            active_overrides: vec![],
        }),
        Some(Action::VariableLoaded(_))
    ));
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::Dependencies {
            recipe: "busybox".into(),
            build: vec![],
            runtime: vec![],
        }),
        Some(Action::DependenciesLoaded(_))
    ));
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::RecipeSources {
            recipe: "busybox".into(),
            paths: vec!["/workspace/busybox".into()],
        }),
        Some(Action::RecipeSourcesLoaded { .. })
    ));
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::LayerRelationships(vec![])),
        Some(Action::LayerRelationshipsLoaded(_))
    ));
    assert_eq!(model_action_from_backend_event(BackendEvent::Ignored), None);
}
#[test]
fn dependency_graph_typed_events_map_success_partial_and_failure() {
    let root = DependencyNodeId::recipe("core-image-minimal");
    let (graph, _) = DependencyGraph::normalize(
        root.clone(),
        Vec::new(),
        vec![DependencyEdge {
            from: root.clone(),
            to: DependencyNodeId::recipe("busybox"),
            kind: DependencyEdgeKind::Runtime,
        }],
        10,
        10,
    );
    assert_eq!(
        model_action_from_backend_event(BackendEvent::DependencyGraph {
            graph: graph.clone(),
            limitations: Vec::new(),
        }),
        Some(Action::DependencyGraphLoaded(graph.clone()))
    );
    let mut compatibility = App::new(10, 1_000);
    let action = model_action_from_backend_event(BackendEvent::DependencyGraph {
        graph: graph.clone(),
        limitations: Vec::new(),
    })
    .unwrap();
    let _ = update(&mut compatibility, action);
    assert_eq!(
        compatibility.dependencies.as_ref().unwrap().runtime,
        ["busybox"]
    );
    assert_eq!(
        model_action_from_backend_event(BackendEvent::DependencyGraph {
            graph: graph.clone(),
            limitations: vec!["task graph unavailable".into()],
        }),
        Some(Action::DependencyGraphPartial {
            graph,
            limitations: vec!["task graph unavailable".into()],
        })
    );
    assert_eq!(
        model_action_from_backend_event(BackendEvent::DependencyGraphFailed {
            root: root.clone(),
            message: "query failed".into(),
        }),
        Some(Action::DependencyGraphFailed {
            root,
            message: "query failed".into(),
        })
    );

    let mut app = App::new(10, 1_000);
    let action = model_action_from_backend_event(BackendEvent::DependencyGraphFailed {
        root: DependencyNodeId::recipe("image"),
        message: "offline".into(),
    })
    .unwrap();
    let _ = update(&mut app, action);
    assert!(matches!(
        app.dependency_graph,
        DependencyGraphState::Failed { .. }
    ));
}
#[test]
fn signature_model_typed_events_map_dump_comparison_partial_and_failure() {
    let target = SignatureTarget {
        recipe: "busybox".into(),
        task: "do_compile".into(),
    };
    let identity = SignatureIdentity {
        target: target.clone(),
        hash: Some("abc".into()),
        path: Some("/tmp/busybox.sigdata".into()),
    };
    let record = SignatureRecord {
        identity: identity.clone(),
        base_hash: Some("base".into()),
        task_hash: Some("task".into()),
        variables: Vec::new(),
        dependencies: Vec::new(),
    };
    assert_eq!(
        model_action_from_backend_event(BackendEvent::SignatureDump {
            target: target.clone(),
            records: vec![record.clone()],
            limitations: Vec::new(),
        }),
        Some(Action::SignatureDumpLoaded {
            target: target.clone(),
            records: vec![record.clone()],
        })
    );
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::SignatureDump {
            target: target.clone(),
            records: vec![record],
            limitations: vec!["partial".into()],
        }),
        Some(Action::SignatureDumpPartial { .. })
    ));
    let request = SignatureComparisonRequest {
        left: identity.clone(),
        right: SignatureIdentity {
            hash: Some("def".into()),
            path: Some("/tmp/busybox-old.sigdata".into()),
            ..identity
        },
    };
    let difference = SignatureDifference {
        category: SignatureDifferenceCategory::ChangedValue,
        key: "CC".into(),
        left: Some("gcc".into()),
        right: Some("clang".into()),
    };
    assert_eq!(
        model_action_from_backend_event(BackendEvent::SignatureComparison {
            request: request.clone(),
            differences: vec![difference.clone()],
            limitations: Vec::new(),
        }),
        Some(Action::SignatureComparisonLoaded {
            request: request.clone(),
            differences: vec![difference],
        })
    );
    assert_eq!(
        model_action_from_backend_event(BackendEvent::SignatureComparisonFailed {
            request: request.clone(),
            message: "tool failed".into(),
        }),
        Some(Action::SignatureComparisonFailed {
            request,
            message: "tool failed".into(),
        })
    );
}

#[test]
fn pkgdata_model_typed_events_map_inventory_detail_partial_and_failure() {
    let inventory_request = PackageInventoryRequest { generation: 7 };
    let package = PackageSummary {
        identity: PackageIdentity::new("busybox"),
        recipe: PackageField::Available("busybox".into()),
        provider: PackageField::Available("/layers/core/recipes-core/busybox.bb".into()),
        version: PackageField::Available("1.37.0".into()),
        installed_size_bytes: PackageField::Available(1_024),
        license: PackageField::Available("GPL-2.0-only".into()),
        image_membership: PackageField::Available(vec!["core-image-minimal".into()]),
    };
    assert_eq!(
        model_action_from_backend_event(BackendEvent::PackageInventory {
            request: inventory_request,
            packages: vec![package.clone()],
            limitations: Vec::new(),
        }),
        Some(Action::PackageInventoryLoaded {
            request: inventory_request,
            packages: vec![package],
        })
    );
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::PackageInventory {
            request: inventory_request,
            packages: Vec::new(),
            limitations: vec!["pkgdata directory is incomplete".into()],
        }),
        Some(Action::PackageInventoryPartial { .. })
    ));
    assert_eq!(
        model_action_from_backend_event(BackendEvent::PackageInventoryFailed {
            request: inventory_request,
            message: "pkgdata directory is missing".into(),
        }),
        Some(Action::PackageInventoryFailed {
            request: inventory_request,
            message: "pkgdata directory is missing".into(),
        })
    );

    let detail_request = PackageDetailRequest {
        identity: PackageIdentity::new("busybox"),
        generation: 3,
    };
    let detail = PackageDetail {
        identity: detail_request.identity.clone(),
        files: PackageField::Available(vec!["/bin/busybox".into()]),
        runtime_dependencies: PackageField::Available(vec![PackageIdentity::new("libc6")]),
        reverse_dependencies: PackageField::Unavailable,
    };
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::PackageDetail {
            request: detail_request.clone(),
            detail: detail.clone(),
            limitations: vec!["reverse dependencies unavailable".into()],
        }),
        Some(Action::PackageDetailPartial { .. })
    ));
    assert_eq!(
        model_action_from_backend_event(BackendEvent::PackageDetailFailed {
            request: detail_request.clone(),
            message: "package was not found".into(),
        }),
        Some(Action::PackageDetailFailed {
            request: detail_request,
            message: "package was not found".into(),
        })
    );
}

#[test]
fn image_artifact_model_typed_events_map_success_partial_and_failure() {
    let request = ImageArtifactRequest {
        generation: 9,
        machine: "qemux86-64".into(),
    };
    let artifact = ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: request.machine.clone(),
            image: "core-image-minimal".into(),
            path: "/build/tmp/deploy/images/qemux86-64/core-image-minimal.wic".into(),
        },
        kind: ImageArtifactKind::Wic,
        size_bytes: ImageArtifactField::Available(8_192),
        modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
        checksums: ImageArtifactField::Unavailable,
        manifests: ImageArtifactField::Available(Vec::new()),
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Unavailable,
        wic_files: ImageArtifactField::Available(Vec::new()),
    };
    let inventory = ImageArtifactInventory {
        machine: request.machine.clone(),
        deploy_directory: ImageArtifactField::Available(
            "/build/tmp/deploy/images/qemux86-64".into(),
        ),
        artifacts: vec![artifact],
    };
    assert_eq!(
        model_action_from_backend_event(BackendEvent::ImageArtifacts {
            request: request.clone(),
            inventory: inventory.clone(),
            limitations: Vec::new(),
        }),
        Some(Action::ImageArtifactInventoryLoaded {
            request: request.clone(),
            inventory: inventory.clone(),
        })
    );
    assert_eq!(
        model_action_from_backend_event(BackendEvent::ImageArtifacts {
            request: request.clone(),
            inventory: inventory.clone(),
            limitations: vec!["license metadata unavailable".into()],
        }),
        Some(Action::ImageArtifactInventoryPartial {
            request: request.clone(),
            inventory,
            limitations: vec!["license metadata unavailable".into()],
        })
    );
    assert_eq!(
        model_action_from_backend_event(BackendEvent::ImageArtifactsFailed {
            request: request.clone(),
            message: "deploy directory is missing".into(),
        }),
        Some(Action::ImageArtifactInventoryFailed {
            request,
            message: "deploy directory is missing".into(),
        })
    );
}
