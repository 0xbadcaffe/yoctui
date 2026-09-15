//! Regression tests grouped around compatibility_dynamic_app_converts_installs_updates_and_invalidates_authority.
use super::*;

#[test]
fn compatibility_dynamic_app_converts_installs_updates_and_invalidates_authority() {
    let first = compatibility_workspace_authority(1).normalize().unwrap();
    let wire = daemon_compatibility_protocol(&first);
    assert_eq!(compatibility_model_snapshot(&wire).unwrap(), first);

    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::Layers;
    app.focus = FocusTarget::Inspector;
    let mut client = DaemonClientSnapshot::default();
    let snapshot = compatibility_workspace_daemon_snapshot(&first);
    let next_sequence = snapshot.sequence + 1;
    let next_generation = snapshot.generation + 1;
    client.replace_app(&mut app, snapshot);
    assert_eq!(
        app.workspace_compatibility
            .authority()
            .unwrap()
            .snapshot
            .generation,
        1
    );
    assert_eq!(app.screen, Screen::Layers);
    assert_eq!(app.focus, FocusTarget::Inspector);

    let second = compatibility_workspace_authority(2).normalize().unwrap();
    client
        .apply_event_to_app(
            &mut app,
            &yoctui_protocol::daemon::SequencedEvent {
                sequence: next_sequence,
                generation: next_generation,
                event: yoctui_protocol::daemon::DaemonEvent::CompatibilityChanged(Box::new(
                    daemon_compatibility_protocol(&second),
                )),
            },
        )
        .unwrap();
    assert_eq!(
        app.workspace_compatibility
            .authority()
            .unwrap()
            .snapshot
            .generation,
        2
    );
    assert_eq!(app.screen, Screen::Layers);

    client.disconnect_app(&mut app);
    assert!(app.workspace_compatibility.authority().is_none());
    assert_eq!(app.screen, Screen::Layers);
    assert_eq!(app.focus, FocusTarget::Inspector);
}

#[test]
fn compatibility_dynamic_app_rejects_unknown_wire_data_and_retains_newer_authority() {
    let first = compatibility_workspace_authority(1).normalize().unwrap();
    let mut unknown = daemon_compatibility_protocol(&first);
    unknown.capabilities[0].id = "future.unregistered.capability".into();
    assert!(compatibility_model_snapshot(&unknown).is_err());

    let mut app = yoctui_model::App::new(16, 4096);
    let mut client = DaemonClientSnapshot::default();
    let mut malformed_snapshot = compatibility_workspace_daemon_snapshot(&first);
    malformed_snapshot.compatibility = Some(unknown);
    client.replace_app(&mut app, malformed_snapshot);
    assert!(app.workspace_compatibility.authority().is_none());
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("unknown capability ID")
    );

    let second = compatibility_workspace_authority(2).normalize().unwrap();
    client.replace_app(&mut app, compatibility_workspace_daemon_snapshot(&second));
    client.replace_app(&mut app, compatibility_workspace_daemon_snapshot(&first));
    assert_eq!(
        app.workspace_compatibility
            .authority()
            .unwrap()
            .snapshot
            .generation,
        2
    );
    assert!(app.notification.as_deref().unwrap().contains("stale"));
}

#[test]
fn compatibility_decode_failure_is_not_repeated_for_unrelated_events() {
    let authority = compatibility_workspace_authority(1).normalize().unwrap();
    let mut unknown = daemon_compatibility_protocol(&authority);
    unknown.capabilities[0].id = "future.unregistered.capability".into();
    let mut snapshot = compatibility_workspace_daemon_snapshot(&authority);
    snapshot.compatibility = Some(unknown);
    let next_sequence = snapshot.sequence + 1;
    let next_generation = snapshot.generation + 1;

    let mut app = yoctui_model::App::new(16, 4096);
    let mut client = DaemonClientSnapshot::default();
    client.replace_app(&mut app, snapshot);
    assert!(
        app.notification
            .take()
            .unwrap()
            .contains("unknown capability ID")
    );

    client
        .apply_event_to_app(
            &mut app,
            &yoctui_protocol::daemon::SequencedEvent {
                sequence: next_sequence,
                generation: next_generation,
                event: yoctui_protocol::daemon::DaemonEvent::Telemetry(
                    yoctui_protocol::daemon::DaemonTelemetry {
                        uptime_seconds: 1,
                        bitbake: yoctui_protocol::daemon::LifecycleState::Running,
                        connected_clients: 1,
                        active_jobs: 1,
                        pty_sessions: 0,
                        queue_depth: 0,
                        pressure: yoctui_protocol::daemon::DaemonPressureCounters::default(),
                        memory_bytes: None,
                        recovery: yoctui_protocol::daemon::DaemonRecoveryState::CleanStart,
                    },
                ),
            },
        )
        .unwrap();
    assert!(app.notification.is_none());
}

#[test]
fn compatibility_workspace_app_routes_local_effects_and_denies_unavailable_effects() {
    let mut app = yoctui_model::App::new(16, 4096);
    assert_eq!(
        compatibility_workspace_action(
            &mut app,
            yoctui_model::Action::ChangeSelectedSetting { backwards: false },
        ),
        Some(yoctui_model::Effect::PersistSettings)
    );

    let before = app.package_inventory.clone();
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::BeginPackageInventory,),
        None
    );
    assert_eq!(app.package_inventory, before);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("No current environment capability snapshot")
    );
}

#[test]
fn compatibility_ui_model_wire_lifecycle_reconciles_filter_search_and_selection() {
    let first = compatibility_workspace_authority(1).normalize().unwrap();
    let mut app = yoctui_model::App::new(16, 4096);
    let mut client = DaemonClientSnapshot::default();
    client.replace_app(&mut app, compatibility_workspace_daemon_snapshot(&first));

    let projection = app
        .compatibility_ui
        .project(&app.workspace_compatibility, app.daemon.status);
    assert_eq!(projection.total_capabilities, 4);
    assert_eq!(projection.summary.available, 3);
    assert_eq!(projection.summary.unavailable, 1);
    let _ = compatibility_workspace_action(
        &mut app,
        yoctui_model::Action::SetCompatibilityFilter(
            yoctui_model::CompatibilityUiFilter::Unavailable,
        ),
    );
    assert_eq!(
        app.compatibility_ui.selected(),
        Some(yoctui_model::CapabilityId::DevtoolUpgrade)
    );
    let _ =
        compatibility_workspace_action(&mut app, yoctui_model::Action::BeginCompatibilitySearch);
    for character in "upgrade".chars() {
        let _ = compatibility_workspace_action(
            &mut app,
            yoctui_model::Action::AppendCompatibilityQuery(character),
        );
    }

    let second = compatibility_workspace_authority(2).normalize().unwrap();
    client.replace_app(&mut app, compatibility_workspace_daemon_snapshot(&second));
    assert_eq!(
        app.compatibility_ui.selected(),
        Some(yoctui_model::CapabilityId::DevtoolUpgrade)
    );
    assert_eq!(app.compatibility_ui.query, "upgrade");

    client.disconnect_app(&mut app);
    let projection = app
        .compatibility_ui
        .project(&app.workspace_compatibility, app.daemon.status);
    assert!(matches!(
        projection.authority,
        yoctui_model::CompatibilityUiAuthorityStatus::Unavailable { .. }
    ));
    assert!(projection.rows.is_empty());
    assert_eq!(app.compatibility_ui.selected(), None);
    assert_eq!(
        app.compatibility_ui.filter,
        yoctui_model::CompatibilityUiFilter::Unavailable
    );
    assert_eq!(app.compatibility_ui.query, "upgrade");
}

#[test]
fn compatibility_ui_inspector_keys_are_typed_modal_and_do_not_leak() {
    assert_eq!(
        compatibility_ui_inspector_action(false, Input::Char('1')),
        Some(Action::SetCompatibilityFilter(
            yoctui_model::CompatibilityUiFilter::All
        ))
    );
    assert_eq!(
        compatibility_ui_inspector_action(false, Input::Char('5')),
        Some(Action::SetCompatibilityFilter(
            yoctui_model::CompatibilityUiFilter::Attention
        ))
    );
    assert_eq!(
        compatibility_ui_inspector_action(false, Input::Char('/')),
        Some(Action::BeginCompatibilitySearch)
    );
    assert_eq!(
        compatibility_ui_inspector_action(true, Input::Char('x')),
        Some(Action::AppendCompatibilityQuery('x'))
    );
    assert_eq!(
        compatibility_ui_inspector_action(true, Input::Backspace),
        Some(Action::BackspaceCompatibilityQuery)
    );
    assert_eq!(
        compatibility_ui_inspector_action(true, Input::Esc),
        Some(Action::FinishCompatibilitySearch)
    );
    assert_eq!(compatibility_ui_inspector_action(false, Input::Tab), None);
    assert_eq!(
        compatibility_ui_inspector_action(false, Input::Char('q')),
        None
    );
}

#[test]
fn compatibility_ui_nav_actions_load_unload_and_reject_before_activation() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.workspace.build_dir = Some("/work/poky/build".into());

    let commands = app.command_palette_commands();
    let build = commands
        .iter()
        .find(|command| command.id == yoctui_model::CommandId::BuildImage)
        .unwrap();
    assert!(!build.enabled());
    assert_eq!(
        build.compatibility_state,
        yoctui_model::WorkspaceAvailabilityState::Unknown
    );
    assert!(
        build
            .disabled_reason
            .as_deref()
            .unwrap()
            .contains("current environment capability snapshot")
    );
    let layers = commands
        .iter()
        .find(|command| command.id == yoctui_model::CommandId::OpenLayers)
        .unwrap();
    assert!(layers.enabled(), "navigation must remain discoverable");
    assert_eq!(
        layers.compatibility_state,
        yoctui_model::WorkspaceAvailabilityState::Unknown
    );

    yoctui_model::install_workspace_compatibility(&mut app, compatibility_workspace_authority(1))
        .unwrap();
    let build = app
        .command_palette_commands()
        .into_iter()
        .find(|command| command.id == yoctui_model::CommandId::BuildImage)
        .unwrap();
    assert!(build.enabled());
    assert_eq!(
        build.compatibility_state,
        yoctui_model::WorkspaceAvailabilityState::Available
    );
    assert_eq!(
        build.implementations,
        [(
            yoctui_model::CapabilityId::BitBakeBuild,
            "bitbake.build.command".into()
        )]
    );

    app.command_palette_open = true;
    app.focus = yoctui_model::FocusTarget::CommandPalette;
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::ActivateCommandPalette,),
        None
    );
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::BuildOptions)
    ));
    let _ = compatibility_workspace_action(&mut app, yoctui_model::Action::CloseBuildOptions);

    yoctui_model::invalidate_workspace_compatibility(&mut app);
    app.command_palette_open = true;
    app.focus = yoctui_model::FocusTarget::CommandPalette;
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::ActivateCommandPalette,),
        None
    );
    assert!(app.active_dialog().is_none());
    assert!(app.command_palette_open);
}

#[test]
fn ux_action_catalog_drives_palette_metadata_search_and_workspace_projection() {
    yoctui_model::validate_operator_action_catalog().unwrap();
    let mut app = yoctui_model::App::new(16, 4096);
    app.command_palette_query = "capabilities".into();
    let commands = app.filtered_command_palette_commands();
    assert_eq!(commands.len(), 1);
    let command = &commands[0];
    assert_eq!(command.action_id.as_str(), "navigate.compatibility");
    assert_eq!(command.id, yoctui_model::CommandId::OpenCompatibility);
    assert_eq!(command.menu_path, ["Navigate", "Open Compatibility"]);
    assert!(command.aliases.contains(&"capabilities"));
    assert_eq!(
        command.help_group,
        yoctui_model::OperatorActionHelpGroup::Navigate
    );

    let build = app
        .command_palette_commands()
        .into_iter()
        .find(|command| command.id == yoctui_model::CommandId::BuildImage)
        .unwrap();
    assert_eq!(build.shortcut, "B");
    assert_eq!(key_action(Input::Char('B')), Some(Action::OpenBuildOptions));

    let catalog = yoctui_model::workspace_operator_action_definitions(
        yoctui_model::WorkspaceDestination::Images,
    );
    let presentations = yoctui_model::compatibility_ui_workspace_action_presentations(
        &app.workspace_compatibility,
        yoctui_model::WorkspaceDestination::Images,
    );
    assert_eq!(catalog.len(), presentations.len());
    for (definition, presentation) in catalog.iter().zip(&presentations) {
        assert_eq!(definition.id.as_str(), presentation.id);
        assert_eq!(definition.label, presentation.label);
        assert_eq!(definition.description, presentation.description);
        assert_eq!(definition.menu_path, presentation.menu_path);
        assert_eq!(definition.safety, presentation.safety);
        assert_eq!(definition.footer_priority, presentation.footer_priority);
        assert_eq!(definition.help_group, presentation.help_group);
    }
}

#[test]
fn ux_keymap_routes_scoped_defaults_custom_chords_and_traps_dialogs() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = yoctui_model::Screen::Images;
    assert!(matches!(
        keymap_action_for_app(&mut app, Input::Char('i')),
        KeymapInputResult::Action(action) if matches!(*action, Action::OpenImagePicker(_))
    ));

    app.install_keymap(yoctui_model::KeymapPreferences {
        schema_version: yoctui_model::KEYMAP_SCHEMA_VERSION,
        overrides: vec![yoctui_model::KeymapOverride {
            action_id: "navigate.logs".into(),
            scope: yoctui_model::KeymapScope::Global,
            sequences: vec!["z".parse().unwrap(), "g l".parse().unwrap()],
        }],
    })
    .unwrap();
    app.screen = yoctui_model::Screen::Dashboard;
    assert_eq!(
        keymap_action_for_app(&mut app, Input::Char('l')),
        KeymapInputResult::Unmatched,
        "an overridden default must not leak through the legacy router"
    );
    assert_eq!(
        keymap_action_for_app(&mut app, Input::Char('z')),
        KeymapInputResult::Action(Box::new(Action::Open(yoctui_model::Screen::Logs)))
    );
    assert_eq!(
        keymap_action_for_app(&mut app, Input::Char('g')),
        KeymapInputResult::Pending
    );
    assert_eq!(
        keymap_action_for_app(&mut app, Input::Char('l')),
        KeymapInputResult::Action(Box::new(Action::Open(yoctui_model::Screen::Logs)))
    );

    app.dialogs
        .push_back(yoctui_model::Dialog::QuitConfirmation);
    app.focus = yoctui_model::FocusTarget::Dialog;
    assert_eq!(
        keymap_action_for_app(&mut app, Input::Char('z')),
        KeymapInputResult::Unmatched
    );
    assert!(!app.keymap_chord.is_pending());
}

#[test]
fn global_slash_search_routes_without_stealing_owned_text_input() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = yoctui_model::Screen::Recipes;
    assert_eq!(
        global_search_action(&app, Input::Char('/')),
        Some(Action::OpenGlobalSearch)
    );

    app.metadata_searching = true;
    assert_eq!(global_search_action(&app, Input::Char('/')), None);
    app.metadata_searching = false;
    app.screen = yoctui_model::Screen::Dashboard;
    app.logs.searching = true;
    assert_eq!(
        global_search_action(&app, Input::Char('/')),
        Some(Action::OpenGlobalSearch),
        "a retained search owned by another screen must not suppress global search"
    );
    app.logs.searching = false;
    app.screen = yoctui_model::Screen::TerminalSessions;
    assert_eq!(global_search_action(&app, Input::Char('/')), None);

    app.screen = yoctui_model::Screen::Dashboard;
    app.dialogs
        .push_back(yoctui_model::Dialog::QuitConfirmation);
    assert_eq!(global_search_action(&app, Input::Char('/')), None);
    assert_eq!(global_search_action(&app, Input::Char('x')), None);
}

#[test]
fn ux_keymap_preferences_capture_validate_reset_export_and_trap_focus() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = yoctui_model::Screen::Settings;
    app.settings_selection = yoctui_model::SETTINGS.len() - 1;
    let action = settings_action(Input::Enter).unwrap();
    assert_eq!(compatibility_workspace_action(&mut app, action), None);
    assert!(app.keymap_preferences_ui.open);
    assert_eq!(app.focus, yoctui_model::FocusTarget::Dialog);
    assert_eq!(
        keymap_action_for_app(&mut app, Input::Char('l')),
        KeymapInputResult::Unmatched
    );

    let action = keymap_preferences_action(&app, Input::Char('/')).unwrap();
    let _ = compatibility_workspace_action(&mut app, action);
    for character in "navigate.logs".chars() {
        let action = keymap_preferences_action(&app, Input::Char(character)).unwrap();
        let _ = compatibility_workspace_action(&mut app, action);
    }
    let action = keymap_preferences_action(&app, Input::Enter).unwrap();
    let _ = compatibility_workspace_action(&mut app, action);
    assert_eq!(
        yoctui_model::keymap_preference_rows(
            &app.keymap_preferences,
            &app.effective_keymap,
            &app.keymap_preferences_ui.query,
        )
        .len(),
        1
    );

    let action = keymap_preferences_action(&app, Input::Enter).unwrap();
    let _ = compatibility_workspace_action(&mut app, action);
    let action = keymap_preferences_action(&app, Input::Char('e')).unwrap();
    let _ = compatibility_workspace_action(&mut app, action);
    let action = keymap_preferences_action(&app, Input::CtrlS).unwrap();
    assert_eq!(compatibility_workspace_action(&mut app, action), None);
    assert!(
        app.keymap_preferences_ui
            .validation_error
            .as_deref()
            .is_some_and(|error| error.contains("collision"))
    );

    let action = keymap_preferences_action(&app, Input::Backspace).unwrap();
    let _ = compatibility_workspace_action(&mut app, action);
    let action = keymap_preferences_action(&app, Input::Char('z')).unwrap();
    let _ = compatibility_workspace_action(&mut app, action);
    let action = keymap_preferences_action(&app, Input::CtrlS).unwrap();
    assert_eq!(
        compatibility_workspace_action(&mut app, action),
        Some(yoctui_model::Effect::PersistSettings)
    );
    assert!(app.settings_dirty);
    assert!(app.keymap_preferences_ui.capture.is_none());
    assert!(app.effective_keymap.bindings().iter().any(|binding| {
        binding.action_id.as_str() == "navigate.logs"
            && binding.sequence.to_string() == "z"
            && !binding.is_default
    }));

    let action = keymap_preferences_action(&app, Input::Char('e')).unwrap();
    let Some(yoctui_model::Effect::CopyToClipboard(report)) =
        compatibility_workspace_action(&mut app, action)
    else {
        panic!("export should use the bounded clipboard effect")
    };
    assert!(report.contains("navigate.logs\tz\tcustom"));

    let _ = compatibility_workspace_action(
        &mut app,
        Action::SettingsPersistenceFailed("read-only filesystem".into()),
    );
    let action = keymap_preferences_action(&app, Input::Char('p')).unwrap();
    assert_eq!(
        compatibility_workspace_action(&mut app, action),
        Some(yoctui_model::Effect::PersistSettings)
    );

    let _ = compatibility_workspace_action(&mut app, Action::ClearKeymapPreferenceQuery);
    let _ = compatibility_workspace_action(&mut app, Action::BeginKeymapPreferenceSearch);
    for character in "help.open".chars() {
        let _ = compatibility_workspace_action(
            &mut app,
            Action::AppendKeymapPreferenceQuery(character),
        );
    }
    let _ = compatibility_workspace_action(&mut app, Action::FinishKeymapPreferenceSearch);
    assert_eq!(
        compatibility_workspace_action(&mut app, Action::RemoveKeymapBinding),
        None
    );
    assert!(
        app.keymap_preferences_ui
            .validation_error
            .as_deref()
            .is_some_and(|error| error.contains("critical action help.open"))
    );

    assert_eq!(
        compatibility_workspace_action(&mut app, Action::ResetAllKeymapBindings),
        Some(yoctui_model::Effect::PersistSettings)
    );
    assert!(app.keymap_preferences.overrides.is_empty());
    let _ = compatibility_workspace_action(&mut app, Action::CloseKeymapPreferences);
    assert!(!app.keymap_preferences_ui.open);
    assert_eq!(app.focus, yoctui_model::FocusTarget::Navigator);
}

#[test]
fn compatibility_dynamic_app_actions_follow_installed_authority_and_runtime_gate() {
    let mut app = yoctui_model::App::new(16, 4096);
    yoctui_model::install_workspace_compatibility(&mut app, compatibility_workspace_authority(3))
        .unwrap();

    let dashboard = yoctui_model::compatibility_ui_workspace_action_presentations(
        &app.workspace_compatibility,
        yoctui_model::WorkspaceDestination::Dashboard,
    );
    let build = dashboard
        .iter()
        .find(|action| action.id == "dashboard.build")
        .unwrap();
    assert!(build.availability.enabled);
    assert_eq!(
        build.availability.implementations,
        [(
            yoctui_model::CapabilityId::BitBakeBuild,
            "bitbake.build.command".into()
        )]
    );
    let cancel = dashboard
        .iter()
        .find(|action| action.id == "dashboard.cancel")
        .unwrap();
    assert!(!cancel.availability.enabled);
    assert_eq!(
        cancel.availability.state,
        yoctui_model::WorkspaceAvailabilityState::Unknown
    );

    app.build.status = yoctui_model::BuildStatus::Running;
    let before = app.build.clone();
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::Cancel),
        None
    );
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::BuildCancellationConfirmation)
    ));
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::ConfirmBuildCancellation,),
        None
    );
    assert_eq!(app.build, before);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("bitbake.cancellation")
    );

    let images = yoctui_model::compatibility_ui_workspace_action_presentations(
        &app.workspace_compatibility,
        yoctui_model::WorkspaceDestination::Images,
    );
    assert!(
        images
            .iter()
            .find(|action| action.id == "images.device_write")
            .unwrap()
            .availability
            .enabled
    );
    yoctui_model::invalidate_workspace_compatibility(&mut app);
    let unloaded = yoctui_model::compatibility_ui_workspace_action_presentations(
        &app.workspace_compatibility,
        yoctui_model::WorkspaceDestination::Dashboard,
    );
    assert_eq!(
        unloaded
            .iter()
            .find(|action| action.id == "dashboard.build")
            .unwrap()
            .availability
            .state,
        yoctui_model::WorkspaceAvailabilityState::Unknown
    );
}

#[test]
fn compatibility_dynamic_app_dialogs_reject_confirm_and_revalidate_snapshot_changes() {
    let request = yoctui_model::BuildRequest {
        targets: vec!["core-image-minimal".into()],
        task: None,
        force: false,
    };
    let mut app = yoctui_model::App::new(16, 4096);
    app.dialogs
        .push_front(yoctui_model::Dialog::RecipeTaskConfirmation(
            request.clone(),
        ));
    app.focus = yoctui_model::FocusTarget::Dialog;
    let presentation = yoctui_model::compatibility_ui_dialog_action_availability(
        &app.workspace_compatibility,
        app.active_dialog().unwrap(),
    );
    assert!(!presentation.enabled);
    assert_eq!(
        presentation.state,
        yoctui_model::WorkspaceAvailabilityState::Unknown
    );
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::ConfirmRecipeTask,),
        None
    );
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::RecipeTaskConfirmation(_))
    ));

    yoctui_model::install_workspace_compatibility(&mut app, compatibility_workspace_authority(4))
        .unwrap();
    let presentation = yoctui_model::compatibility_ui_dialog_action_availability(
        &app.workspace_compatibility,
        app.active_dialog().unwrap(),
    );
    assert!(presentation.enabled);
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::ConfirmRecipeTask,),
        Some(yoctui_model::Effect::Start(request))
    );
    assert!(app.active_dialog().is_none());

    app.dialogs.push_front(yoctui_model::Dialog::BuildOptions);
    app.focus = yoctui_model::FocusTarget::Dialog;
    app.focus_return = Some(yoctui_model::FocusTarget::Inspector);
    let revalidated = yoctui_model::invalidate_workspace_compatibility(&mut app);
    assert!(revalidated.closed_dialog);
    assert!(app.active_dialog().is_none());
    assert_eq!(
        app.focus,
        yoctui_model::FocusTarget::Navigator,
        "Dashboard cannot restore a passive Inspector focus"
    );
    assert!(
        revalidated
            .reason
            .unwrap()
            .contains("current environment capability snapshot")
    );

    app.dialogs
        .push_front(yoctui_model::Dialog::QuitConfirmation);
    let local = yoctui_model::compatibility_ui_dialog_action_availability(
        &app.workspace_compatibility,
        app.active_dialog().unwrap(),
    );
    assert!(local.enabled);
    assert_eq!(
        local.state,
        yoctui_model::WorkspaceAvailabilityState::Available
    );
}

#[test]
fn compatibility_dynamic_app_parent_gate_enforces_one_runtime_authority() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.workspace.build_dir = Some("/work/poky/build".into());
    yoctui_model::install_workspace_compatibility(&mut app, compatibility_workspace_authority(9))
        .unwrap();

    let command = app
        .command_palette_commands()
        .into_iter()
        .find(|command| command.id == yoctui_model::CommandId::BuildImage)
        .unwrap();
    let workspace = yoctui_model::compatibility_ui_workspace_action_presentations(
        &app.workspace_compatibility,
        yoctui_model::WorkspaceDestination::Dashboard,
    )
    .into_iter()
    .find(|action| action.id == "dashboard.build")
    .unwrap();
    app.dialogs.push_front(yoctui_model::Dialog::BuildOptions);
    let dialog = yoctui_model::compatibility_ui_dialog_action_availability(
        &app.workspace_compatibility,
        app.active_dialog().unwrap(),
    );
    assert!(command.enabled());
    assert!(workspace.availability.enabled);
    assert!(dialog.enabled);
    assert_eq!(
        command.implementations,
        workspace.availability.implementations
    );
    assert_eq!(command.implementations, dialog.implementations);

    yoctui_model::invalidate_workspace_compatibility(&mut app);
    assert!(app.active_dialog().is_none());
    app.command_palette_open = true;
    app.command_palette_query = "Build image".into();
    app.command_palette_selection = 0;
    app.focus = yoctui_model::FocusTarget::CommandPalette;
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::ActivateCommandPalette),
        None
    );
    assert!(app.active_dialog().is_none());
    assert!(app.command_palette_open);

    app.dialogs
        .push_front(yoctui_model::Dialog::QuitConfirmation);
    let local = yoctui_model::compatibility_ui_dialog_action_availability(
        &app.workspace_compatibility,
        app.active_dialog().unwrap(),
    );
    assert!(local.enabled);
}

#[test]
fn mouse_routes_focus_and_scroll_semantically() {
    assert_eq!(
        mouse_action(
            MouseInput {
                kind: MouseKind::Down,
                column: 3,
                row: 4
            },
            140
        ),
        Some(Action::Focus(FocusTarget::Navigator))
    );
    assert_eq!(
        mouse_action(
            MouseInput {
                kind: MouseKind::Down,
                column: 130,
                row: 4
            },
            140
        ),
        Some(Action::Focus(FocusTarget::Inspector))
    );
    assert_eq!(
        mouse_action(
            MouseInput {
                kind: MouseKind::ScrollDown,
                column: 3,
                row: 4
            },
            80
        ),
        Some(Action::SelectNavigator { delta: 1 })
    );
}
