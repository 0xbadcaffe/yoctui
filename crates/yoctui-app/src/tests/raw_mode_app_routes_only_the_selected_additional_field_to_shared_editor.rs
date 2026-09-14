//! Regression tests grouped around raw_mode_app_routes_only_the_selected_additional_field_to_shared_editor.
use super::*;

#[test]
fn raw_mode_app_routes_only_the_selected_additional_field_to_shared_editor() {
    let catalog = yoctui_model::RawCatalog::builtin();
    let mut state = yoctui_model::RawModeState::new(&catalog);
    state.view = yoctui_model::RawModeView::Form;
    state.focus = yoctui_model::RawModeFocus::Form;
    state.form = Some(yoctui_model::RawCommandForm {
        command: catalog.commands[0].id.clone(),
        fields: std::collections::BTreeMap::new(),
        field_order: Vec::new(),
        field_selection: 0,
        additional_arguments: yoctui_model::RawArgvEditor::new("").unwrap(),
        capability_generation: 1,
        build_directory: "/work/build".into(),
    });

    assert_eq!(
        raw_mode_input(&state, Input::Char('i')),
        Some(yoctui_model::RawModeAction::EditAdditionalArguments(
            yoctui_model::PopupEditorCommand::ToggleInsert
        ))
    );
    state
        .form
        .as_mut()
        .unwrap()
        .additional_arguments
        .editor
        .editing = true;
    assert_eq!(
        raw_mode_input(&state, Input::Char('v')),
        Some(yoctui_model::RawModeAction::EditAdditionalArguments(
            yoctui_model::PopupEditorCommand::Insert('v')
        ))
    );
    assert_eq!(
        raw_mode_input(&state, Input::Enter),
        Some(yoctui_model::RawModeAction::RequestPreview)
    );
    assert_eq!(raw_mode_input(&state, Input::CtrlS), None);
}

#[test]
fn raw_form_app_routes_selector_editor_fields_and_dialog_focus() {
    let catalog = yoctui_model::RawCatalog::builtin();
    let command = catalog
        .commands
        .iter()
        .find(|command| {
            command
                .parameters
                .iter()
                .any(|parameter| parameter.kind == yoctui_model::RawParameterKind::Target)
        })
        .unwrap();
    let parameter = command
        .parameters
        .iter()
        .find(|parameter| parameter.kind == yoctui_model::RawParameterKind::Target)
        .unwrap()
        .id
        .clone();
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = yoctui_model::Screen::RawMode;
    app.build.target = Some("busybox".into());
    app.raw_mode.view = yoctui_model::RawModeView::Form;
    app.raw_mode.focus = yoctui_model::RawModeFocus::Form;
    app.raw_mode.form = Some(yoctui_model::RawCommandForm {
        command: command.id.clone(),
        fields: std::collections::BTreeMap::from([(
            parameter.clone(),
            yoctui_model::RawFormField {
                parameter: parameter.clone(),
                editor: yoctui_model::PopupEditor::new(String::new()),
                value: None,
                validation_error: None,
            },
        )]),
        field_order: vec![parameter.clone()],
        field_selection: 0,
        additional_arguments: yoctui_model::RawArgvEditor::new("").unwrap(),
        capability_generation: 1,
        build_directory: "/work/build".into(),
    });

    let choice = raw_mode_input(&app, Input::Right).unwrap();
    assert!(matches!(
        choice,
        yoctui_model::RawModeAction::ChooseParameter {
            value: yoctui_model::RawParameterValue::Target(ref value),
            ..
        } if value == "busybox"
    ));
    assert!(matches!(
        raw_mode_input(&app, Input::Char('i')),
        Some(yoctui_model::RawModeAction::EditParameterInput {
            command: yoctui_model::PopupEditorCommand::ToggleInsert,
            ..
        })
    ));
    assert_eq!(
        raw_mode_input(&app, Input::Tab),
        Some(yoctui_model::RawModeAction::SelectFormField { delta: 1 })
    );

    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::RawMode(yoctui_model::RawModeAction::DismissNotification),
    );
    assert_eq!(app.focus, yoctui_model::FocusTarget::Dialog);
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 1,
                row: 3,
            },
            &app,
            160,
            50,
        ),
        Some(yoctui_model::Action::Focus(
            yoctui_model::FocusTarget::Dialog
        ))
    );
    let close = raw_mode_input(&app, Input::Char('q')).unwrap();
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::RawMode(close));
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Browser);
    assert_eq!(app.focus, yoctui_model::FocusTarget::Workspace);
}

#[test]
fn raw_navigation_routes_workspace_and_shared_focus_input() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::RawMode;
    assert_eq!(
        workspace_collection_action(&app, Input::Down),
        Some(Action::RawMode(
            yoctui_model::RawModeAction::SelectCategory { delta: 1 }
        ))
    );
    assert_eq!(
        raw_mode_input(&app.raw_mode, Input::Char('/')),
        Some(yoctui_model::RawModeAction::BeginSearch)
    );
    assert_eq!(
        focus_action(FocusTarget::Workspace, Input::Tab),
        Some(Action::CycleFocus { backwards: false })
    );
    assert_eq!(
        focus_action(FocusTarget::Workspace, Input::BackTab),
        Some(Action::CycleFocus { backwards: true })
    );
}

#[test]
fn raw_category_input_routes_bounded_browser_columns() {
    let state = yoctui_model::RawModeState::new(yoctui_model::builtin_raw_catalog());
    assert_eq!(
        raw_mode_input(&state, Input::Char('j')),
        Some(yoctui_model::RawModeAction::SelectCategory { delta: 1 })
    );
    assert_eq!(
        raw_mode_input(&state, Input::Char('k')),
        Some(yoctui_model::RawModeAction::SelectCategory { delta: -1 })
    );
    assert_eq!(
        raw_mode_input(&state, Input::Char('l')),
        Some(yoctui_model::RawModeAction::FocusCommands)
    );
    assert_eq!(
        raw_mode_input(&state, Input::Enter),
        Some(yoctui_model::RawModeAction::FocusCommands)
    );
    assert_eq!(
        raw_mode_input(&state, Input::Char('h')),
        Some(yoctui_model::RawModeAction::FocusCategories)
    );
}

#[test]
fn raw_job_client_confirmation_and_cancellation_remain_typed() {
    let mut preview_state = yoctui_model::RawModeState::new(yoctui_model::builtin_raw_catalog());
    preview_state.view = yoctui_model::RawModeView::Preview;
    assert_eq!(
        raw_mode_input(&preview_state, Input::Enter),
        Some(yoctui_model::RawModeAction::ConfirmPreview)
    );

    let state = raw_execution_state_fixture();
    let wire = raw_execution_request_to_protocol(&state.request).unwrap();
    assert_eq!(wire.request_id, state.request.id.as_str());
    assert_eq!(wire.command_id, state.request.command.as_str());
    assert_eq!(wire.preview_digest, state.request.preview_digest.to_hex());

    let request_id = state.request.id.clone();
    let mut app = yoctui_model::App::new(16, 4096);
    app.raw_mode
        .execution_states
        .insert(request_id.clone(), state);
    assert_eq!(
        yoctui_model::update(
            &mut app,
            yoctui_model::Action::RawMode(yoctui_model::RawModeAction::CancelExecution(
                request_id.clone()
            ),),
        ),
        Some(yoctui_model::Effect::CancelRaw(request_id))
    );
}

#[test]
fn raw_pty_request_and_owner_round_trip_without_command_string_authority() {
    let mut state = raw_execution_state_fixture();
    state.request.interaction = yoctui_model::RawInteractionMode::InteractivePty;
    let wire = raw_execution_request_to_protocol(&state.request).unwrap();
    assert_eq!(
        wire.interaction,
        yoctui_protocol::daemon::RawInteractionData::InteractivePty
    );
    assert_eq!(
        raw_execution_request_from_protocol(&wire).unwrap(),
        state.request
    );
    assert_eq!(wire.command_id, "build.target");
    assert!(!wire.command_id.contains(' '));
}

#[test]
fn raw_output_keys_route_follow_search_scroll_cancel_and_attachment_as_typed_actions() {
    let state = raw_execution_state_fixture();
    let request = state.request.id.clone();
    let command = state.request.command.clone();
    let mut app = yoctui_model::App::new(16, 4096);
    app.raw_mode.execution_states.insert(request.clone(), state);
    app.raw_mode.execution = Some(command);
    app.raw_mode.view = yoctui_model::RawModeView::Execution;
    app.raw_mode.focus = yoctui_model::RawModeFocus::Execution;

    assert_eq!(
        raw_mode_input(&app, Input::Char('f')),
        Some(yoctui_model::RawModeAction::ToggleOutputFollow)
    );
    assert!(matches!(
        raw_mode_input(&app, Input::Up),
        Some(yoctui_model::RawModeAction::ScrollOutput { vertical: 1, .. })
    ));
    assert_eq!(
        raw_mode_input(&app, Input::Char('c')),
        Some(yoctui_model::RawModeAction::CancelExecution(
            request.clone()
        ))
    );
    let detach = raw_mode_input(&app, Input::Char('d')).unwrap();
    assert_eq!(
        yoctui_model::update(&mut app, yoctui_model::Action::RawMode(detach)),
        Some(yoctui_model::Effect::SetRawAttachment {
            request: request.clone(),
            attached: false,
        })
    );
    assert_eq!(
        raw_mode_input(&app, Input::Char('/')),
        Some(yoctui_model::RawModeAction::BeginOutputSearch)
    );
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::RawMode(yoctui_model::RawModeAction::BeginOutputSearch),
    );
    assert_eq!(
        raw_mode_input(&app, Input::Char('界')),
        Some(yoctui_model::RawModeAction::AppendOutputSearch('界'))
    );
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::RawMode(yoctui_model::RawModeAction::FinishOutputSearch),
    );
    assert_eq!(
        raw_mode_input(&app, Input::Esc),
        Some(yoctui_model::RawModeAction::CloseExecution)
    );
    assert_eq!(
        yoctui_model::update(
            &mut app,
            yoctui_model::Action::RawMode(yoctui_model::RawModeAction::CloseExecution),
        ),
        Some(yoctui_model::Effect::SetRawAttachment {
            request,
            attached: false,
        })
    );
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Browser);
}

#[test]
fn raw_execution_app_mechanically_round_trips_request_event_chunk_result_and_snapshot() {
    let mut state = raw_execution_state_fixture();
    let request = raw_execution_request_to_protocol(&state.request).unwrap();
    assert_eq!(
        raw_execution_request_from_protocol(&request).unwrap(),
        state.request
    );
    let event = apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Starting {
            owner: yoctui_model::RawExecutionOwner::Job(
                yoctui_model::RawJobId::new("raw-job:app-1").unwrap(),
            ),
        },
    );
    let wire_event = raw_execution_event_to_protocol(&event).unwrap();
    assert_eq!(
        raw_execution_event_from_protocol(&wire_event).unwrap(),
        event
    );
    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Running {
            started_unix_ms: 20,
        },
    );
    let chunk = yoctui_model::RawOutputChunk {
        stream_id: state.stdout.stream_id.clone(),
        stream: yoctui_model::RawOutputStream::Stdout,
        sequence: 1,
        text: "unicode 界\n".into(),
        truncated_bytes: 0,
        dropped_lines: 0,
    };
    let wire_chunk = raw_output_chunk_to_protocol(&chunk).unwrap();
    assert_eq!(raw_output_chunk_from_protocol(&wire_chunk).unwrap(), chunk);
    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Output { chunk },
    );
    let result = yoctui_model::RawExecutionResult {
        outcome: yoctui_model::RawExecutionOutcome::Succeeded,
        exit_code: Some(0),
        message: Some("complete".into()),
        elapsed_ms: 50,
        durable_reference: Some(
            yoctui_model::RawDurableReferenceId::new("raw-durable:app-1").unwrap(),
        ),
    };
    let wire_result = raw_execution_result_to_protocol(&result).unwrap();
    assert_eq!(
        raw_execution_result_from_protocol(&wire_result).unwrap(),
        result
    );
    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Finished { result },
    );
    let snapshot = raw_execution_snapshot_to_protocol(&state).unwrap();
    assert_eq!(
        raw_execution_snapshot_from_protocol(&snapshot).unwrap(),
        state
    );
}

#[test]
fn raw_history_app_installs_validated_records_without_recreating_execution_authority() {
    let mut state = raw_execution_state_fixture();
    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Starting {
            owner: yoctui_model::RawExecutionOwner::Job(
                yoctui_model::RawJobId::new("raw-job:history-app").unwrap(),
            ),
        },
    );
    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Running {
            started_unix_ms: 20,
        },
    );
    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Finished {
            result: yoctui_model::RawExecutionResult {
                outcome: yoctui_model::RawExecutionOutcome::Succeeded,
                exit_code: Some(0),
                message: Some("not history".into()),
                elapsed_ms: 50,
                durable_reference: None,
            },
        },
    );
    let execution = raw_execution_snapshot_to_protocol(&state).unwrap();
    let wire = yoctui_protocol::daemon::RawHistoryRecordData::from_terminal(&execution).unwrap();
    let model = raw_history_record_from_protocol(&wire).unwrap();
    assert_eq!(model.command, state.request.command);

    let global = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        10,
        "raw-history-app".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&global);
    snapshot.raw_history = vec![wire];
    let mut app = yoctui_model::App::new(16, 4_096);
    DaemonClientSnapshot::default().replace_app(&mut app, snapshot);
    assert_eq!(app.raw_mode.history, [model]);
    assert!(app.raw_mode.execution_states.is_empty());
}

#[test]
fn raw_execution_app_installs_reconnect_snapshots_and_rejects_stale_replacement() {
    let mut state = raw_execution_state_fixture();
    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Starting {
            owner: yoctui_model::RawExecutionOwner::Job(
                yoctui_model::RawJobId::new("raw-job:app-1").unwrap(),
            ),
        },
    );
    let global = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        10,
        "raw-app".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&global);
    snapshot.raw_executions = vec![raw_execution_snapshot_to_protocol(&state).unwrap()];
    let mut app = yoctui_model::App::new(16, 4_096);
    let mut client = DaemonClientSnapshot::default();
    client.replace_app(&mut app, snapshot.clone());
    assert_eq!(
        app.raw_mode.execution_states.get(&state.request.id),
        Some(&state)
    );

    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Running {
            started_unix_ms: 20,
        },
    );
    client
        .apply_event_to_app(
            &mut app,
            &yoctui_protocol::daemon::SequencedEvent {
                sequence: snapshot.sequence + 1,
                generation: snapshot.generation + 1,
                event: yoctui_protocol::daemon::DaemonEvent::RawExecutionChanged(Box::new(
                    raw_execution_snapshot_to_protocol(&state).unwrap(),
                )),
            },
        )
        .unwrap();
    assert_eq!(
        app.raw_mode.execution_states[&state.request.id].phase,
        yoctui_model::RawExecutionPhase::Running
    );

    let before = app.raw_mode.execution_states.clone();
    let stale = snapshot.raw_executions[0].clone();
    assert!(
        client
            .apply_event_to_app(
                &mut app,
                &yoctui_protocol::daemon::SequencedEvent {
                    sequence: snapshot.sequence + 2,
                    generation: snapshot.generation + 2,
                    event: yoctui_protocol::daemon::DaemonEvent::RawExecutionChanged(Box::new(
                        stale,
                    )),
                },
            )
            .is_err()
    );
    assert_eq!(app.raw_mode.execution_states, before);

    client.disconnect_app(&mut app);
    assert!(app.raw_mode.execution_states.is_empty());
}

#[test]
fn ux_menu_keyboard_context_mouse_and_catalog_activation_share_typed_routes() {
    let mut app = yoctui_model::App::new(16, 4_096);
    assert_eq!(key_action(Input::F10), Some(Action::OpenApplicationMenu));
    let _ = yoctui_model::update(&mut app, Action::OpenApplicationMenu);
    assert_eq!(
        menu_action(&app, Input::Right),
        Some(MenuInputResult::Reduce(Box::new(Action::SelectMenuGroup {
            delta: 1
        })))
    );
    let _ = yoctui_model::update(&mut app, Action::SelectMenuGroup { delta: 1 });
    assert_eq!(app.menu.group(), yoctui_model::ApplicationMenuGroup::Build);
    assert_eq!(
        menu_action(&app, Input::Enter),
        None,
        "build stays disabled without a workspace"
    );

    let _ = yoctui_model::update(&mut app, Action::CloseMenu);
    app.screen = Screen::Recipes;
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::ContextDown,
                column: 40,
                row: 8,
            },
            &app,
            160,
            48,
        ),
        Some(Action::OpenContextMenu)
    );
    assert_eq!(key_action(Input::Char('a')), Some(Action::OpenContextMenu));
    assert_eq!(
        context_menu_activation_input("recipes.dependencies"),
        Some(Input::Char('A'))
    );
    assert!(
        yoctui_model::operator_action_catalog()
            .into_iter()
            .filter_map(|definition| match definition.target {
                yoctui_model::OperatorActionTarget::Workspace { legacy_id, .. } => {
                    Some(legacy_id)
                }
                yoctui_model::OperatorActionTarget::Command(_) => None,
            })
            .all(|id| context_menu_activation_input(id).is_some()),
        "every contextual catalog entry must retain a typed legacy route"
    );
}

#[test]
fn ux_menu_traps_input_over_recipe_editor_and_preserves_editor_state() {
    let mut app = yoctui_model::App::new(16, 4_096);
    app.screen = Screen::Recipes;
    app.workspace.build_dir = Some("/workspace/build".into());
    let root = std::path::PathBuf::from("/workspace/bash");
    let _ = yoctui_model::update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "bash".into(),
            root,
            files: vec!["bash.bb".into()],
        },
    );
    let _ = yoctui_model::update(
        &mut app,
        Action::LoadRecipeEditorContent("SUMMARY = \"bash\"\n".into()),
    );
    let _ = yoctui_model::update(&mut app, Action::ToggleRecipeEditorEditing);
    let _ = yoctui_model::update(&mut app, Action::AppendRecipeEditor('X'));
    let editor_before = app.active_dialog().cloned();

    let _ = yoctui_model::update(&mut app, Action::OpenApplicationMenu);
    let _ = yoctui_model::update(&mut app, Action::SelectMenuGroup { delta: 1 });
    let _ = yoctui_model::update(&mut app, Action::SelectMenuItem { delta: 2 });
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert_eq!(app.menu.group(), yoctui_model::ApplicationMenuGroup::Build);
    assert_eq!(
        app.selected_menu_item()
            .and_then(|item| item.disabled_reason),
        Some("No active build is available to cancel.".into())
    );
    assert_eq!(menu_action(&app, Input::Enter), None);
    assert_eq!(
        menu_action(&app, Input::Char('c')),
        Some(MenuInputResult::Reduce(Box::new(Action::AppendMenuPrefix(
            'c'
        ))))
    );
    assert_eq!(app.active_dialog(), editor_before.as_ref());

    let _ = yoctui_model::update(&mut app, Action::CloseMenu);
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::RecipeEditor(_))
    ));
}

#[test]
fn ux_focus_outward_escape_zoom_and_modal_input_are_typed_without_shortcut_theft() {
    let mut app = yoctui_model::App::new(16, 4_096);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Workspace;
    let _ = yoctui_model::update(&mut app, Action::CyclePaneSubfocus { backwards: false });
    assert_eq!(
        focus_action_for_app(&app, Input::Esc),
        Some(Action::ResetPaneSubfocus)
    );
    let _ = yoctui_model::update(&mut app, Action::ResetPaneSubfocus);
    assert_eq!(focus_action_for_app(&app, Input::Esc), None);
    assert_eq!(focus_action_for_app(&app, Input::Char('z')), None);

    let _ = yoctui_model::update(&mut app, Action::TogglePaneZoom);
    assert_eq!(
        focus_action_for_app(&app, Input::Esc),
        Some(Action::TogglePaneZoom)
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 2,
                row: 8,
            },
            &app,
            160,
            48,
        ),
        Some(Action::Focus(FocusTarget::Workspace)),
        "a zoomed pane owns its whole body instead of leaking clicks"
    );

    let _ = yoctui_model::update(&mut app, Action::OpenApplicationMenu);
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert_eq!(focus_action_for_app(&app, Input::Tab), None);
    assert_eq!(focus_action_for_app(&app, Input::Esc), None);
    assert_eq!(
        menu_action(&app, Input::Esc),
        Some(MenuInputResult::Reduce(Box::new(Action::CloseMenu)))
    );
}

#[test]
fn ux_scroll_common_keys_map_to_typed_bounded_collection_actions() {
    assert_eq!(collection_scroll_delta(Input::Up), Some(-1));
    assert_eq!(collection_scroll_delta(Input::Char('j')), Some(1));
    assert_eq!(collection_scroll_delta(Input::PageUp), Some(-10));
    assert_eq!(collection_scroll_delta(Input::PageDown), Some(10));
    assert_eq!(collection_scroll_delta(Input::Home), Some(isize::MIN));
    assert_eq!(collection_scroll_delta(Input::End), Some(isize::MAX));
    assert_eq!(collection_scroll_delta(Input::Char('G')), Some(isize::MAX));

    let mut chord_app = yoctui_model::App::new(8, 1_000);
    assert_eq!(
        keymap_action_for_app(&mut chord_app, Input::Char('g')),
        KeymapInputResult::Pending
    );
    assert_eq!(
        keymap_action_for_app(&mut chord_app, Input::Char('g')),
        KeymapInputResult::Action(Box::new(Action::ScrollCurrent { to_end: false }))
    );

    assert_eq!(
        tasks_action(false, Input::PageDown),
        Some(Action::ScrollBuildTasks { delta: 10 })
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Home),
        Some(Action::SelectRecipe { delta: isize::MIN })
    );
    assert_eq!(
        logs_action(false, Input::End),
        Some(Action::ScrollLogs { delta: -isize::MAX })
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::PageDown),
        Some(Action::SelectNavigator { delta: 10 })
    );

    let state = yoctui_model::RawModeState::new(yoctui_model::builtin_raw_catalog());
    assert_eq!(
        raw_mode_input(&state, Input::PageDown),
        Some(yoctui_model::RawModeAction::SelectCategory { delta: 10 })
    );
}

#[test]
fn ux_progress_backend_events_feed_distinct_typed_phase_projections() {
    let action = model_action_from_backend_event(BackendEvent::ParseProgress {
        current: Some(8),
        total: Some(20),
    })
    .unwrap();
    assert_eq!(
        action,
        Action::ParseProgress {
            current: Some(8),
            total: Some(20)
        }
    );

    let mut app = yoctui_model::App::new(8, 1_000);
    app.build.status = yoctui_model::BuildStatus::Parsing;
    let _ = yoctui_model::update(&mut app, action);
    let progress = app.progress_hierarchy_at(std::time::SystemTime::UNIX_EPOCH);
    assert_eq!(progress.parse.fraction.unwrap().exact_text(), "8/20 (40%)");
    assert_eq!(
        progress.runqueue.state,
        yoctui_model::WidgetState::Unavailable
    );
    assert_eq!(
        progress.sstate.state,
        yoctui_model::WidgetState::Unavailable
    );
}

#[test]
fn ux_terminal_keys_are_modal_and_never_forward_control_actions() {
    let mut app = yoctui_model::App::new(8, 1_000);
    app.screen = Screen::TerminalSessions;
    assert_eq!(
        terminal_workspace_action(&app, Input::Char('o')),
        Some(Action::TerminalTakeControl)
    );
    assert_eq!(
        terminal_workspace_action(&app, Input::Char('n')),
        Some(Action::TerminalCreateBuildShell)
    );
    assert_eq!(
        terminal_workspace_action(&app, Input::Char('s')),
        Some(Action::TerminalCreateSelectedDevshell)
    );
    assert_eq!(
        terminal_workspace_action(&app, Input::Char('m')),
        Some(Action::TerminalCreateSelectedMenuconfig)
    );
    assert_eq!(terminal_workspace_action(&app, Input::Char('a')), None);

    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.terminal.client_id = Some([3; 16]);
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 1,
            name: "shell".into(),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            viewers: 1,
        });
    app.daemon
        .pty_details
        .push(yoctui_model::ClientDaemonPtyDetails {
            id: 1,
            kind: yoctui_model::ClientDaemonPtyKind::BuildShell,
            cwd: "/build".into(),
            columns: 80,
            rows: 24,
            writer: Some([3; 16]),
            writer_epoch: 1,
            exit_code: None,
            restartable: true,
        });
    for literal in ['n', 's', 'm', 'o', 'r', 'x', '?', '/', 'v'] {
        assert_eq!(
            terminal_workspace_action(&app, Input::Char(literal)),
            None,
            "writer character {literal:?} must remain PTY input"
        );
    }
    assert_eq!(
        terminal_context_action(Input::Char('s')),
        Some(Action::TerminalCreateSelectedDevshell)
    );

    app.terminal.mode = yoctui_model::TerminalWorkbenchMode::Search;
    assert_eq!(
        terminal_workspace_action(&app, Input::Char('a')),
        Some(Action::TerminalAppendSearch('a'))
    );
    assert_eq!(
        terminal_workspace_action(&app, Input::Enter),
        Some(Action::TerminalFinishSearch)
    );

    app.terminal.mode = yoctui_model::TerminalWorkbenchMode::PasteReview;
    assert_eq!(
        terminal_workspace_action(&app, Input::Enter),
        Some(Action::TerminalConfirmPaste)
    );
    assert_eq!(terminal_workspace_action(&app, Input::Char('y')), None);

    app.terminal.mode = yoctui_model::TerminalWorkbenchMode::KillConfirmation;
    assert_eq!(
        terminal_workspace_action(&app, Input::Enter),
        Some(Action::TerminalConfirmKill)
    );
    assert_eq!(
        terminal_workspace_action(&app, Input::Esc),
        Some(Action::TerminalCancelMode)
    );
}

#[test]
fn devwork_terminal_dialog_keys_are_focus_trapped_and_recipe_routes_are_explicit() {
    assert_eq!(
        terminal_launch_dialog_action(Input::Down),
        Some(Action::SelectTerminalLaunchDestination { delta: 1 })
    );
    assert_eq!(
        terminal_launch_dialog_action(Input::Enter),
        Some(Action::ConfirmTerminalLaunch)
    );
    assert_eq!(
        terminal_launch_dialog_action(Input::Esc),
        Some(Action::CancelTerminalLaunch)
    );
    assert_eq!(terminal_launch_dialog_action(Input::Char('x')), None);
    assert_eq!(
        recipes_workspace_action(false, Input::Char('s')),
        Some(Action::BeginSelectedRecipeDevtoolWorkspaceShell)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('E')),
        Some(Action::BeginSelectedRecipeDevtoolEditRecipe)
    );
}

#[test]
fn ux_onboarding_traps_input_and_routes_only_typed_guide_actions() {
    let mut app = yoctui_model::App::new_unconfigured(8, 1_000);
    assert_eq!(onboarding_action(&app, Input::Enter), None);
    let _ = yoctui_model::update(&mut app, Action::OpenOnboarding);

    assert_eq!(
        onboarding_action(&app, Input::Down),
        Some(Action::SelectOnboarding { delta: 1 })
    );
    assert_eq!(
        onboarding_action(&app, Input::Enter),
        Some(Action::ActivateOnboardingStep)
    );
    assert_eq!(
        onboarding_action(&app, Input::Char('n')),
        Some(Action::AdvanceOnboarding)
    );
    assert_eq!(
        onboarding_action(&app, Input::Char('s')),
        Some(Action::SkipOnboardingStep)
    );
    assert_eq!(
        onboarding_action(&app, Input::Esc),
        Some(Action::DismissOnboarding)
    );

    assert!(matches!(
        keymap_action_for_app(&mut app, Input::Char('B')),
        KeymapInputResult::Unmatched
    ));
}
