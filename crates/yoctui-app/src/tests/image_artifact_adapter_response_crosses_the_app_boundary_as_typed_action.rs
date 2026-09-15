//! Regression tests grouped around image_artifact_adapter_response_crosses_the_app_boundary_as_typed_action.
use super::*;

#[test]
fn image_artifact_adapter_response_crosses_the_app_boundary_as_typed_action() {
    let request = ImageArtifactRequest {
        generation: 12,
        machine: "qemux86-64".into(),
    };
    let inventory = ImageArtifactInventory {
        machine: request.machine.clone(),
        deploy_directory: ImageArtifactField::Available(
            "/build/tmp/deploy/images/qemux86-64".into(),
        ),
        artifacts: Vec::new(),
    };
    let event: BackendEvent = yoctui_bitbake::ImageArtifactResponse {
        request: request.clone(),
        inventory: inventory.clone(),
        limitations: vec!["one symlink was not followed".into()],
    }
    .into();
    assert_eq!(
        model_action_from_backend_event(event),
        Some(Action::ImageArtifactInventoryPartial {
            request,
            inventory,
            limitations: vec!["one symlink was not followed".into()],
        })
    );
}

#[test]
fn ux_rootfs_protocol_crosses_app_boundary_as_typed_correlated_action() {
    use yoctui_protocol::rootfs::{
        ROOTFS_COMPOSITION_SCHEMA_VERSION, RootfsAuthorityData, RootfsCompositionData,
        RootfsCompositionRequestData, RootfsEntryData, RootfsEntryKindData,
        RootfsImageIdentityData, RootfsInstalledPackageData,
    };

    let data = RootfsCompositionData {
        schema_version: ROOTFS_COMPOSITION_SCHEMA_VERSION,
        request: RootfsCompositionRequestData {
            generation: 4,
            image: RootfsImageIdentityData {
                machine: "qemux86-64".into(),
                image: "core-image-minimal".into(),
                path: "/build/tmp/deploy/images/qemux86-64/image.ext4".into(),
            },
        },
        installed_packages: RootfsAuthorityData::Available {
            records: vec![RootfsInstalledPackageData {
                name: "busybox".into(),
                recipe: Some("busybox".into()),
                category: "base".into(),
                installed_size_bytes: 1_024,
                file_count: 12,
            }],
        },
        filesystem_entries: RootfsAuthorityData::Partial {
            records: vec![RootfsEntryData {
                path: "/bin/busybox".into(),
                kind: RootfsEntryKindData::RegularFile,
                size_bytes: 1_024,
                package: Some("busybox".into()),
            }],
            limitations: vec!["hard-link count unavailable".into()],
        },
        limitations: vec!["manifest versions unavailable".into()],
    };
    let event = backend_event_from_rootfs_data(data).unwrap();
    let action = model_action_from_backend_event(event).unwrap();
    let Action::RootfsCompositionPartial {
        request,
        composition,
        limitations,
    } = action
    else {
        panic!("expected typed partial rootfs composition action")
    };
    assert_eq!(request.generation, 4);
    assert_eq!(request.image.image, "core-image-minimal");
    assert_eq!(
        composition.package_inventory().unwrap().packages[0].identity,
        yoctui_model::PackageIdentity::new("busybox")
    );
    assert_eq!(
        composition.filesystem_tree().unwrap().entries[0].kind,
        yoctui_model::RootfsEntryKind::RegularFile
    );
    assert_eq!(limitations, ["manifest versions unavailable"]);
}

#[test]
fn pkgdata_adapter_responses_cross_the_app_boundary_as_typed_actions() {
    let inventory_request = PackageInventoryRequest { generation: 11 };
    let package = PackageSummary {
        identity: PackageIdentity::new("busybox"),
        recipe: PackageField::Available("busybox".into()),
        provider: PackageField::Unavailable,
        version: PackageField::Available("1.37.0-r0".into()),
        installed_size_bytes: PackageField::Available(1_024),
        license: PackageField::Available("GPL-2.0-only".into()),
        image_membership: PackageField::Unavailable,
    };
    let event: BackendEvent = yoctui_bitbake::PackageInventoryResponse {
        request: inventory_request,
        packages: vec![package.clone()],
        limitations: vec!["provider path unavailable".into()],
    }
    .into();
    assert_eq!(
        model_action_from_backend_event(event),
        Some(Action::PackageInventoryPartial {
            request: inventory_request,
            packages: vec![package],
            limitations: vec!["provider path unavailable".into()],
        })
    );

    let request = PackageDetailRequest {
        identity: PackageIdentity::new("busybox"),
        generation: 4,
    };
    let detail = PackageDetail {
        identity: request.identity.clone(),
        files: PackageField::Available(vec!["/bin/busybox".into()]),
        runtime_dependencies: PackageField::Available(vec![PackageIdentity::new("libc6")]),
        reverse_dependencies: PackageField::Available(Vec::new()),
    };
    let event: BackendEvent = yoctui_bitbake::PackageDetailResponse {
        request: request.clone(),
        detail: detail.clone(),
        limitations: Vec::new(),
    }
    .into();
    assert_eq!(
        model_action_from_backend_event(event),
        Some(Action::PackageDetailLoaded { request, detail })
    );
}

#[test]
fn pkgdata_workspace_maps_search_navigation_refresh_and_context_actions() {
    assert_eq!(
        package_workspace_action(false, Input::Up),
        Some(Action::SelectPackage { delta: -1 })
    );
    assert_eq!(
        package_workspace_action(false, Input::Enter),
        Some(Action::BeginSelectedPackageDetail)
    );
    assert_eq!(
        package_workspace_action(false, Input::Char('R')),
        Some(Action::RefreshPackageInventory)
    );
    assert_eq!(
        package_workspace_action(false, Input::Char('D')),
        Some(Action::TogglePackageDependencyKind)
    );
    assert_eq!(
        package_workspace_action(false, Input::Char(']')),
        Some(Action::SelectPackageDependency { delta: 1 })
    );
    assert_eq!(
        package_workspace_action(false, Input::Char('d')),
        Some(Action::OpenSelectedPackageDependency)
    );
    assert_eq!(
        package_workspace_action(false, Input::Char('o')),
        Some(Action::OpenSelectedPackageRecipe)
    );
    assert_eq!(
        package_workspace_action(false, Input::Char('e')),
        Some(Action::OpenSelectedPackageProvider)
    );
    assert_eq!(
        package_workspace_action(false, Input::Char('c')),
        Some(Action::CancelPackageOperation)
    );
    assert_eq!(
        package_workspace_action(false, Input::Char('/')),
        Some(Action::BeginPackageSearch)
    );
    assert_eq!(
        package_workspace_action(true, Input::Char('b')),
        Some(Action::AppendPackageQuery('b'))
    );
    assert_eq!(
        package_workspace_action(true, Input::Backspace),
        Some(Action::BackspacePackageQuery)
    );
    assert_eq!(
        package_workspace_action(true, Input::Esc),
        Some(Action::FinishPackageSearch)
    );
    assert_eq!(package_workspace_action(false, Input::Char('x')), None);
}

#[test]
fn signature_adapter_responses_cross_the_app_boundary_as_typed_actions() {
    let target = SignatureTarget {
        recipe: "busybox".into(),
        task: "do_compile".into(),
    };
    let identity = SignatureIdentity {
        target: target.clone(),
        hash: Some("aaa".into()),
        path: Some("/build/tmp/stamps/busybox/do_compile.sigdata.aaa".into()),
    };
    let record = SignatureRecord {
        identity: identity.clone(),
        base_hash: Some("base-aaa".into()),
        task_hash: Some("aaa".into()),
        variables: Vec::new(),
        dependencies: Vec::new(),
    };
    let event: BackendEvent = yoctui_bitbake::SignatureDumpResponse {
        target: target.clone(),
        records: vec![record.clone()],
        limitations: vec!["one malformed historical signature was omitted".into()],
    }
    .into();
    assert_eq!(
        model_action_from_backend_event(event),
        Some(Action::SignatureDumpPartial {
            target,
            records: vec![record],
            limitations: vec!["one malformed historical signature was omitted".into()],
        })
    );

    let request = SignatureComparisonRequest {
        left: identity.clone(),
        right: SignatureIdentity {
            hash: Some("bbb".into()),
            ..identity
        },
    };
    let difference = SignatureDifference {
        category: SignatureDifferenceCategory::BaseHash,
        key: "base_hash".into(),
        left: Some("base-aaa".into()),
        right: Some("base-bbb".into()),
    };
    let event: BackendEvent = yoctui_bitbake::SignatureComparisonResponse {
        request: request.clone(),
        differences: vec![difference.clone()],
        limitations: Vec::new(),
    }
    .into();
    assert_eq!(
        model_action_from_backend_event(event),
        Some(Action::SignatureComparisonLoaded {
            request,
            differences: vec![difference],
        })
    );
}

#[test]
fn typed_event_terminal_events_emit_primary_and_job_actions_once() {
    let mut coordinator = BuildJobCoordinator::default();
    coordinator
        .queue_build(&request(), SystemTime::UNIX_EPOCH)
        .unwrap();
    let completed = coordinator.actions_for_backend_event(
        BackendEvent::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
        SystemTime::UNIX_EPOCH,
    );
    assert_eq!(
        completed
            .iter()
            .filter(|action| matches!(action, Action::BuildCompleted { .. }))
            .count(),
        1
    );
    assert_eq!(
        completed
            .iter()
            .filter(|action| matches!(action, Action::SucceedBackgroundJob { .. }))
            .count(),
        1
    );
    assert_eq!(completed.len(), 2);

    let mut coordinator = BuildJobCoordinator::default();
    coordinator
        .queue_build(&request(), SystemTime::UNIX_EPOCH)
        .unwrap();
    let failed = coordinator.actions_for_backend_event(
        BackendEvent::CommandFailed {
            code: "parse".into(),
            message: "bad metadata".into(),
        },
        SystemTime::UNIX_EPOCH,
    );
    assert_eq!(
        failed
            .iter()
            .filter(|action| matches!(action, Action::Failure(_)))
            .count(),
        1
    );
    assert_eq!(
        failed
            .iter()
            .filter(|action| matches!(action, Action::FailBackgroundJob { .. }))
            .count(),
        1
    );
    assert_eq!(failed.len(), 2);
}

#[test]
fn background_job_command_failure_and_disconnect_are_terminal() {
    let mut coordinator = BuildJobCoordinator::default();
    let mut app = App::new(10, 1_000);
    apply_actions(
        &mut app,
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .unwrap(),
    );
    let failed_id = coordinator.active_job_id().unwrap();
    apply_actions(
        &mut app,
        coordinator.actions_for_backend_event(
            BackendEvent::CommandFailed {
                code: "start_failed".into(),
                message: "server rejected build".into(),
            },
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
        ),
    );
    assert_eq!(
        app.background_jobs.get(failed_id).unwrap().status,
        BackgroundJobStatus::Failed
    );

    apply_actions(
        &mut app,
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .unwrap(),
    );
    let lost_id = coordinator.active_job_id().unwrap();
    apply_actions(
        &mut app,
        coordinator.actions_for_backend_event(
            BackendEvent::Disconnected,
            SystemTime::UNIX_EPOCH + Duration::from_secs(2),
        ),
    );
    assert_eq!(
        app.background_jobs.get(lost_id).unwrap().status,
        BackgroundJobStatus::Lost
    );
}

#[test]
fn background_job_start_failure_finishes_the_queued_job() {
    let mut coordinator = BuildJobCoordinator::default();
    let mut app = App::new(10, 1_000);
    apply_actions(
        &mut app,
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .unwrap(),
    );
    let id = coordinator.active_job_id().unwrap();
    apply_actions(
        &mut app,
        coordinator.start_failed(
            "executable not found".into(),
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
        ),
    );
    let job = app.background_jobs.get(id).unwrap();
    assert_eq!(job.status, BackgroundJobStatus::Failed);
    assert_eq!(
        job.error.as_ref().and_then(|error| error.detail.as_deref()),
        Some("executable not found")
    );
    assert_eq!(coordinator.active_job_id(), None);
}

#[test]
fn background_job_backend_error_marks_the_active_job_lost() {
    let mut coordinator = BuildJobCoordinator::default();
    let mut app = App::new(10, 1_000);
    apply_actions(
        &mut app,
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .unwrap(),
    );
    let id = coordinator.active_job_id().unwrap();
    apply_actions(
        &mut app,
        coordinator.backend_lost(
            "protocol framing failed".into(),
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
        ),
    );
    let job = app.background_jobs.get(id).unwrap();
    assert_eq!(job.status, BackgroundJobStatus::Lost);
    assert_eq!(
        job.error.as_ref().and_then(|error| error.detail.as_deref()),
        Some("protocol framing failed")
    );
    assert_eq!(coordinator.active_job_id(), None);
}

#[test]
fn background_job_cancellation_failure_recovers_then_acknowledges() {
    let mut coordinator = BuildJobCoordinator::default();
    let mut app = App::new(10, 1_000);
    apply_actions(
        &mut app,
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .unwrap(),
    );
    let id = coordinator.active_job_id().unwrap();
    apply_actions(
        &mut app,
        coordinator.actions_for_backend_event(BackendEvent::BuildStarted, SystemTime::UNIX_EPOCH),
    );
    assert_eq!(update(&mut app, Action::Cancel), None);
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::BuildCancellationConfirmation)
    ));
    assert!(matches!(
        update(&mut app, Action::ConfirmBuildCancellation),
        Some(yoctui_model::Effect::Cancel)
    ));
    apply_actions(&mut app, vec![coordinator.request_cancellation().unwrap()]);
    apply_actions(
        &mut app,
        coordinator.cancellation_failed(
            "backend refused".into(),
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
        ),
    );
    assert_eq!(
        app.background_jobs.get(id).unwrap().status,
        BackgroundJobStatus::Running
    );
    assert_eq!(app.build.status, BuildStatus::Running);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("may still be running")
    );

    assert_eq!(update(&mut app, Action::Cancel), None);
    assert!(matches!(
        update(&mut app, Action::ConfirmBuildCancellation),
        Some(yoctui_model::Effect::Cancel)
    ));
    apply_actions(&mut app, vec![coordinator.request_cancellation().unwrap()]);
    apply_actions(
        &mut app,
        coordinator.actions_for_backend_event(
            BackendEvent::BuildCompleted {
                success: false,
                exit_code: None,
            },
            SystemTime::UNIX_EPOCH + Duration::from_secs(2),
        ),
    );
    assert_eq!(app.build.status, BuildStatus::Cancelled);
    assert_eq!(
        app.background_jobs.get(id).unwrap().status,
        BackgroundJobStatus::Cancelled
    );
}

#[test]
fn background_job_coordinator_prevents_duplicate_active_builds() {
    let mut coordinator = BuildJobCoordinator::default();
    assert!(
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .is_some()
    );
    assert!(
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .is_none()
    );
    assert_eq!(coordinator.active_job_id(), Some(BackgroundJobId(1)));
}

#[test]
fn maps_navigation() {
    assert_eq!(
        key_action(Input::Char('l')),
        Some(Action::Open(Screen::Logs))
    );
    assert_eq!(
        key_action(Input::Tab),
        Some(Action::CycleFocus { backwards: false })
    );
    assert_eq!(key_action(Input::F5), Some(Action::Open(Screen::Logs)));
    assert_eq!(
        key_action(Input::Char('x')),
        Some(Action::Open(Screen::Bbmask))
    );
}
#[test]
fn ux_responsive_pane_shortcuts_map_to_focus_cycle() {
    assert_eq!(
        key_action(Input::Tab),
        Some(Action::CycleFocus { backwards: false })
    );
    assert_eq!(
        key_action(Input::BackTab),
        Some(Action::CycleFocus { backwards: true })
    );
}
#[test]
fn exit_and_build_cancel_confirmations_trap_yes_no_and_enter() {
    assert_eq!(
        build_cancellation_confirmation_action(Input::Enter),
        Some(Action::ConfirmBuildCancellation)
    );
    assert_eq!(
        build_cancellation_confirmation_action(Input::Char('n')),
        Some(Action::CancelBuildCancellation)
    );
    assert_eq!(
        quit_confirmation_action(Input::Char('y')),
        Some(Action::ConfirmQuit)
    );
    assert_eq!(
        quit_confirmation_action(Input::Esc),
        Some(Action::CancelQuit)
    );
    assert_eq!(quit_confirmation_action(Input::Char('q')), None);
}
#[test]
fn settings_input_maps_selection_and_typed_changes() {
    assert_eq!(
        settings_action(Input::Up),
        Some(Action::SelectSetting { delta: -1 })
    );
    assert_eq!(
        settings_action(Input::Down),
        Some(Action::SelectSetting { delta: 1 })
    );
    assert_eq!(
        settings_action(Input::Left),
        Some(Action::ChangeSelectedSetting { backwards: true })
    );
    assert_eq!(
        settings_action(Input::Enter),
        Some(Action::ChangeSelectedSetting { backwards: false })
    );
    assert_eq!(
        settings_action(Input::Char('r')),
        Some(Action::RetrySettingsPersistence)
    );
    assert_eq!(settings_action(Input::Esc), None);
}
#[test]
fn build_environment_input_verifies_and_returns_to_dashboard() {
    assert_eq!(
        build_environment_action(Input::Char('V')),
        Some(Action::BeginBuildEnvironmentVerification)
    );
    assert_eq!(
        build_environment_action(Input::Esc),
        Some(Action::Open(Screen::Dashboard))
    );
}
#[test]
fn ux_textarea_popup_input_maps_normal_insert_and_visual_commands() {
    assert_eq!(
        popup_editor_action(false, Input::Char('e')),
        Some(Action::EditActivePopup(PopupEditorCommand::SelectValue))
    );
    assert_eq!(
        popup_editor_action(false, Input::Char('j')),
        Some(Action::EditActivePopup(PopupEditorCommand::Down))
    );
    assert_eq!(
        popup_editor_action(false, Input::Char('x')),
        Some(Action::EditActivePopup(PopupEditorCommand::Delete))
    );
    assert_eq!(
        popup_editor_action(false, Input::Char('v')),
        Some(Action::EditActivePopup(PopupEditorCommand::ToggleVisual))
    );
    assert_eq!(
        popup_editor_action(false, Input::Char('u')),
        Some(Action::EditActivePopup(PopupEditorCommand::Undo))
    );
    assert_eq!(
        popup_editor_action(false, Input::PageDown),
        Some(Action::EditActivePopup(PopupEditorCommand::PageDown))
    );
    assert_eq!(
        popup_editor_action(true, Input::Char('k')),
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('k')))
    );
    assert_eq!(
        popup_editor_action(true, Input::Home),
        Some(Action::EditActivePopup(PopupEditorCommand::Home))
    );
    assert_eq!(
        popup_editor_action(true, Input::CtrlV),
        Some(Action::EditActivePopup(PopupEditorCommand::Paste))
    );
    assert_eq!(
        popup_editor_action(true, Input::Enter),
        Some(Action::EditActivePopup(PopupEditorCommand::Newline))
    );
}

#[test]
fn ux_textarea_mouse_down_and_drag_map_to_typed_cursor_selection() {
    let mut app = yoctui_model::App::new(10, 1_000);
    app.dialogs
        .push_front(yoctui_model::Dialog::BuildEnvironmentEditor(
            yoctui_model::PopupEditor::new("alpha\nbeta\n".into()),
        ));
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 8,
                row: 4,
            },
            &app,
            100,
            30,
        ),
        Some(Action::EditActivePopup(
            PopupEditorCommand::SelectPosition {
                line: 0,
                column: 2,
                extend: false,
            }
        ))
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Drag,
                column: 9,
                row: 5,
            },
            &app,
            100,
            30,
        ),
        Some(Action::EditActivePopup(
            PopupEditorCommand::SelectPosition {
                line: 1,
                column: 3,
                extend: true,
            }
        ))
    );
}

#[test]
fn ux_checkbox_space_toggles_but_enter_remains_primary() {
    assert_eq!(
        checkbox_input_action(Input::Char(' ')),
        Some(CheckboxInputAction::Toggle)
    );
    assert_eq!(
        checkbox_input_action(Input::Enter),
        Some(CheckboxInputAction::Primary)
    );
    assert_eq!(
        checkbox_input_action(Input::Down),
        Some(CheckboxInputAction::Move(1))
    );
}
#[test]
fn live_tasks_input_maps_selection_and_filter_controls() {
    assert_eq!(
        tasks_action(false, Input::Down),
        Some(Action::ScrollBuildTasks { delta: 1 })
    );
    assert_eq!(
        tasks_action(false, Input::Char('f')),
        Some(Action::CycleTaskStateFilter)
    );
    assert_eq!(
        tasks_action(false, Input::Char('F')),
        Some(Action::CycleTaskFilterField)
    );
    assert_eq!(
        tasks_action(false, Input::Char('/')),
        Some(Action::BeginTaskFilterEdit)
    );
    assert_eq!(
        tasks_action(true, Input::Char('x')),
        Some(Action::AppendTaskFilter('x'))
    );
    assert_eq!(
        tasks_action(true, Input::Esc),
        Some(Action::FinishTaskFilterEdit)
    );
    let action = model_action_from_backend_event(BackendEvent::TaskQueued {
        recipe: "busybox".into(),
        task: "do_compile".into(),
        worker: Some("worker-1".into()),
        stats: Some(yoctui_model::TaskStats {
            completed: 3,
            total: 10,
            active: 2,
            failed: 0,
        }),
    });
    assert!(matches!(
        action,
        Some(Action::TaskQueued(TaskInfo {
            worker: Some(worker),
            stats: Some(yoctui_model::TaskStats { total: 10, .. }),
            ..
        })) if worker == "worker-1"
    ));
}
#[test]
fn config_metadata_normalizes_typed_scope_and_history_once() {
    let action = model_action_from_backend_event(BackendEvent::Variable {
        name: "PACKAGE_ARCH".into(),
        recipe: Some("base-files".into()),
        value: Some("qemux86_64".into()),
        provenance: Some("/layers/meta/conf/machine/qemux86-64.conf:5".into()),
        unexpanded_value: Some("${MACHINE_ARCH}".into()),
        operations: vec![yoctui_model::VariableOperation {
            operation: "set".into(),
            file: Some("/layers/meta/conf/machine/qemux86-64.conf".into()),
            line: Some(5),
            value: Some("${MACHINE_ARCH}".into()),
        }],
        active_overrides: vec!["qemux86-64".into()],
    });
    assert!(matches!(
        action,
        Some(Action::VariableLoaded(VariableDetail {
            identity: VariableIdentity {
                name,
                recipe: Some(recipe),
            },
            unexpanded_value: Some(unexpanded),
            operations,
            ..
        })) if name == "PACKAGE_ARCH"
            && recipe == "base-files"
            && unexpanded == "${MACHINE_ARCH}"
            && operations.len() == 1
    ));
}
#[test]
fn command_palette_global_shortcut_is_typed() {
    assert_eq!(key_action(Input::CtrlP), Some(Action::OpenCommandPalette));
    assert_eq!(focus_action(FocusTarget::CommandPalette, Input::Tab), None);
}
#[test]
fn dialog_focus_navigation_keys_are_typed_before_cli_routing() {
    assert_eq!(
        key_action(Input::Tab),
        Some(Action::CycleFocus { backwards: false })
    );
    assert_eq!(
        key_action(Input::BackTab),
        Some(Action::CycleFocus { backwards: true })
    );
    assert_eq!(
        key_action(Input::Esc),
        Some(Action::Open(Screen::Dashboard))
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Up),
        Some(Action::SelectNavigator { delta: -1 })
    );
    assert_eq!(
        focus_action(FocusTarget::Inspector, Input::Up),
        None,
        "inspector arrows must not leak into workspace actions"
    );
    for focus in [FocusTarget::Workspace, FocusTarget::Inspector] {
        assert_eq!(focus_action(focus, Input::Left), None);
        assert_eq!(focus_action(focus, Input::Right), None);
    }
    assert_eq!(
        focus_action(FocusTarget::Dialog, Input::Tab),
        None,
        "modal input is handled only by the active dialog"
    );
}

#[test]
fn navigator_focus_maps_selection_and_activation_without_swallowing_globals() {
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Down),
        Some(Action::SelectNavigator { delta: 1 })
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Enter),
        Some(Action::ActivateNavigator)
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::CtrlP),
        None,
        "unmapped global input must continue to the shared key route"
    );
}
