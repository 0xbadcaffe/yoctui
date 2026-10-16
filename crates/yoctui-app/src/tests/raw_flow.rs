use super::*;
use yoctui_model::{
    FocusTarget, RawModeAction as Raw, RawModeView as View, Screen, TextAreaPasteSource,
};

fn raw_app() -> App {
    let mut app = App::new(16, 4096);
    app.screen = Screen::RawMode;
    app.focus = FocusTarget::Workspace;
    app
}

#[test]
fn raw_local_search_enter_finishes_without_opening_or_running_and_escape_unwinds() {
    let mut app = raw_app();
    assert_eq!(global_search_action(&app, Input::Char('/')), None);
    assert_eq!(
        raw_mode_input(&app, Input::Char('/')),
        Some(Raw::BeginSearch)
    );
    yoctui_model::update(&mut app, Action::RawMode(Raw::BeginSearch));
    let paste = text_paste_actions(
        &app,
        "--version".into(),
        TextAreaPasteSource::BracketedPaste,
    )
    .unwrap()
    .unwrap();
    apply_actions(&mut app, paste);
    assert_eq!(app.raw_mode.search.query, "--version");
    assert_eq!(raw_mode_input(&app, Input::Enter), Some(Raw::FinishSearch));
    yoctui_model::update(&mut app, Action::RawMode(Raw::FinishSearch));
    assert_eq!(app.raw_mode.view, View::Browser);
    assert!(app.raw_mode.form.is_none());
    assert_eq!(focus_action_for_app(&app, Input::Esc), None);
    assert_eq!(raw_mode_input(&app, Input::Esc), Some(Raw::ClearSearch));
    yoctui_model::update(&mut app, Action::RawMode(Raw::ClearSearch));
    assert_eq!(raw_mode_input(&app, Input::Esc), Some(Raw::Back));
    yoctui_model::update(&mut app, Action::RawMode(Raw::Back));
    assert_eq!(
        app.raw_mode.browser_column,
        yoctui_model::RawBrowserColumn::Categories
    );
    assert_eq!(
        focus_action_for_app(&app, Input::Esc),
        Some(Action::Focus(FocusTarget::Navigator))
    );
    app.focus = FocusTarget::Navigator;
    assert_eq!(
        global_search_action(&app, Input::Char('/')),
        Some(Action::OpenGlobalSearch)
    );
}

#[test]
fn raw_form_arrows_navigate_fields_and_insert_mode_keeps_literal_jk() {
    let mut app = raw_app();
    let (command, parameter) = raw_selector_command(yoctui_model::RawParameterKind::Recipe);
    app.raw_mode.view = View::Form;
    app.raw_mode.form = Some(yoctui_model::RawCommandForm {
        command: command.id,
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
    for (key, delta) in [
        (Input::Down, 1),
        (Input::Up, -1),
        (Input::Char('j'), 1),
        (Input::Char('k'), -1),
        (Input::Tab, 1),
        (Input::BackTab, -1),
    ] {
        assert_eq!(
            raw_mode_input(&app, key),
            Some(Raw::SelectFormField { delta })
        );
    }
    app.raw_mode
        .form
        .as_mut()
        .unwrap()
        .fields
        .get_mut(&parameter)
        .unwrap()
        .editor
        .editing = true;
    for character in ['j', 'k'] {
        assert_eq!(
            raw_mode_input(&app, Input::Char(character)),
            Some(Raw::EditParameterInput {
                parameter: parameter.clone(),
                command: yoctui_model::PopupEditorCommand::Insert(character),
            })
        );
    }
    app.raw_mode.recipe_picker = Some(yoctui_model::RawRecipePicker {
        parameter,
        recipes: vec!["jack".into(), "kernel".into()],
        query: String::new(),
        selection: 0,
    });
    for character in ['j', 'k'] {
        assert_eq!(
            raw_mode_input(&app, Input::Char(character)),
            Some(Raw::AppendRecipePickerQuery(character))
        );
    }
    assert_eq!(
        raw_mode_input(&app, Input::CtrlU),
        Some(Raw::ClearRecipePickerQuery)
    );
    assert_eq!(
        raw_mode_input(&app, Input::End),
        Some(Raw::SelectRecipePicker { delta: isize::MAX })
    );
}

#[test]
fn raw_favorite_modals_trap_literal_input_paste_and_mouse_without_shortcut_theft() {
    let mut app = raw_app();
    app.raw_mode.favorite_rename = Some(String::new());
    assert!(workspace_text_input_active(&app));
    assert_eq!(global_search_action(&app, Input::Char('/')), None);
    assert_eq!(
        raw_mode_input(&app, Input::Char('q')),
        Some(Raw::AppendFavoriteName('q'))
    );
    assert_eq!(raw_mode_input(&app, Input::F12), None);
    let actions = text_paste_actions(&app, "Poky café".into(), TextAreaPasteSource::Clipboard)
        .unwrap()
        .unwrap();
    apply_actions(&mut app, actions);
    assert_eq!(app.raw_mode.favorite_rename.as_deref(), Some("Poky café"));
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 1,
                row: 10
            },
            &app,
            160,
            48
        ),
        None
    );
    assert_eq!(
        raw_mode_input(&app, Input::Esc),
        Some(Raw::CancelFavoriteRename)
    );
    app.raw_mode.favorite_rename = None;
    app.raw_mode.favorite_confirmation = Some(yoctui_model::RawFavoriteConfirmation {
        command: yoctui_model::RawCommandId::new("fixture.favorite").unwrap(),
        return_focus: yoctui_model::RawModeFocus::Favorites,
    });
    assert_eq!(raw_mode_input(&app, Input::Char('q')), None);
    assert_eq!(raw_mode_input(&app, Input::Esc), Some(Raw::CancelFavorite));
    assert_eq!(
        raw_mode_input(&app, Input::Enter),
        Some(Raw::ConfirmFavorite)
    );
    assert!(!text_paste_active(&app));
}

#[test]
fn raw_output_search_and_review_have_distinct_enter_and_navigation_routes() {
    let mut app = raw_app();
    app.raw_mode.view = View::Execution;
    app.raw_mode.output.searching = true;
    assert_eq!(
        raw_mode_input(&app, Input::Enter),
        Some(Raw::FinishOutputSearch)
    );
    assert_eq!(
        raw_mode_input(&app, Input::CtrlU),
        Some(Raw::ClearOutputSearch)
    );
    app.raw_mode.output.searching = false;
    assert_eq!(
        raw_mode_input(&app, Input::End),
        Some(Raw::ScrollOutput {
            vertical: isize::MIN,
            horizontal: 0
        })
    );
    app.raw_mode.view = View::Preview;
    assert_eq!(
        raw_mode_input(&app, Input::PageDown),
        Some(Raw::ScrollPreview { delta: 10 })
    );
    assert_eq!(
        raw_mode_input(&app, Input::Enter),
        Some(Raw::ConfirmPreview)
    );
    assert_eq!(raw_mode_input(&app, Input::Esc), Some(Raw::Back));
    app.raw_mode.view = View::History;
    assert_eq!(
        raw_mode_input(&app, Input::Enter),
        Some(Raw::ActivateHistory)
    );
    assert_eq!(
        raw_mode_input(&app, Input::Char('r')),
        Some(Raw::ReviewHistory)
    );
}

#[test]
fn raw_interactive_open_uses_real_session_and_never_steals_another_writer() {
    for writer in [None, Some([8; 16])] {
        let mut app = raw_app();
        app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
        app.terminal.client_id = Some([7; 16]);
        let mut execution = raw_execution_state_fixture();
        execution.request.interaction = yoctui_model::RawInteractionMode::InteractivePty;
        let session = yoctui_model::RawSessionId::new("raw-session:daemon-3").unwrap();
        let id = session.daemon_pty_id().unwrap();
        execution.owner = Some(yoctui_model::RawExecutionOwner::Pty(session));
        app.raw_mode.output.request = Some(execution.request.id.clone());
        app.raw_mode
            .execution_states
            .insert(execution.request.id.clone(), execution);
        app.daemon
            .pty_sessions
            .push(yoctui_model::ClientDaemonPtySummary {
                id,
                name: "Raw interactive".into(),
                lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
                viewers: 1,
            });
        app.daemon
            .pty_details
            .push(yoctui_model::ClientDaemonPtyDetails {
                id,
                kind: yoctui_model::ClientDaemonPtyKind::BuildShell,
                cwd: "/work/build".into(),
                columns: 80,
                rows: 24,
                writer,
                writer_epoch: 1,
                exit_code: None,
                restartable: false,
            });
        let effect = yoctui_model::update(&mut app, Action::RawMode(Raw::OpenExecutionTerminal));
        assert_eq!(app.screen, Screen::TerminalSessions);
        assert_eq!(app.navigator_screen(), Screen::TerminalSessions);
        assert!(!app.terminal.browser_open);
        assert_eq!(app.selected_terminal_session().unwrap().id, id);
        if writer.is_none() {
            assert!(matches!(effect, Some(yoctui_model::Effect::Terminal(_))));
        } else {
            assert!(effect.is_none());
            assert!(app.notification.as_deref().unwrap().contains("writer"));
        }
    }
}

#[test]
fn raw_local_input_precedes_global_image_picker_shell_and_metadata_routes() {
    let mut app = raw_app();
    app.onboarding.open = false;
    app.raw_mode.view = View::Form;
    for key in [
        Input::Char('i'),
        Input::Char('!'),
        Input::Char('q'),
        Input::Char('/'),
        Input::F12,
        Input::CtrlP,
    ] {
        assert!(
            raw_workspace_owns_input(&app, key),
            "Raw forms trap {key:?}"
        );
    }
    app.raw_mode.view = View::Browser;
    app.raw_mode.search.editing = true;
    for character in ['i', '!', 'j', 'k', 'q'] {
        assert!(raw_workspace_owns_input(&app, Input::Char(character)));
        assert_eq!(
            raw_mode_input(&app, Input::Char(character)),
            Some(Raw::AppendSearch(character))
        );
    }
    app.raw_mode.search.editing = false;
    app.raw_mode.view = View::Favorites;
    assert!(
        raw_workspace_owns_input(&app, Input::Char('i')),
        "i inspects the favorite, not image selection"
    );
    assert!(!raw_workspace_owns_input(&app, Input::F12));
    // Unhandled global keys must fall through, not be consumed by Raw's branch.
    for view in [
        View::Browser,
        View::Favorites,
        View::History,
        View::Execution,
    ] {
        app.raw_mode.view = view;
        for key in [Input::F1, Input::F4, Input::F12, Input::CtrlP] {
            assert!(
                !raw_workspace_owns_input(&app, key),
                "{view:?} swallowed {key:?}"
            );
        }
    }
    app.raw_mode.view = View::Execution;
    for key in [Input::Char('c'), Input::Char('d'), Input::Char('r')] {
        assert!(
            raw_workspace_owns_input(&app, key),
            "Pending Raw work must not cancel an unrelated build"
        );
    }
    app.raw_mode.view = View::Browser;
    app.focus = FocusTarget::Navigator;
    assert!(!raw_workspace_owns_input(&app, Input::Char('/')));
    assert!(!raw_workspace_owns_input(&app, Input::Down));
    app.focus = FocusTarget::Workspace;
    app.raw_mode.browser_column = yoctui_model::RawBrowserColumn::Categories;
    app.raw_mode.search.query.clear();
    assert!(
        !raw_workspace_owns_input(&app, Input::Esc),
        "Outermost Esc returns to the navigator"
    );
    assert_eq!(
        focus_action_for_app(&app, Input::Esc),
        Some(Action::Focus(FocusTarget::Navigator))
    );
    app.focus = FocusTarget::Navigator;
    // A mouse focus change must not strand an unfinished local text search.
    app.raw_mode.search.editing = true;
    assert!(raw_workspace_owns_input(&app, Input::Esc));
    assert!(raw_workspace_owns_input(&app, Input::Char('i')));
    app.raw_mode.search.editing = false;
    app.raw_mode.view = View::Execution;
    app.raw_mode.output.searching = true;
    assert!(raw_workspace_owns_input(&app, Input::Esc));
    app.raw_mode.output.searching = false;
    app.raw_mode.view = View::Form;
    app.command_palette_open = true;
    assert!(!raw_workspace_owns_input(&app, Input::Esc));
}
