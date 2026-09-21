use super::*;

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
    assert_eq!(app.focus, yoctui_model::FocusTarget::Navigator);
}
