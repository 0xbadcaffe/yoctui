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

#[test]
fn raw_recipe_picker_filters_selects_returns_and_keeps_manual_entry() {
    let (command, parameter) = raw_selector_command(yoctui_model::RawParameterKind::Recipe);
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = yoctui_model::Screen::RawMode;
    app.workspace.build_dir = Some("/work/build".into());
    app.workspace.recipes = ["busybox", "bash", "linux-aspeed"]
        .into_iter()
        .map(|name| yoctui_model::Recipe {
            name: name.into(),
            file: Some(format!("/layers/{name}.bb").into()),
            ..yoctui_model::Recipe::default()
        })
        .collect();
    app.raw_mode.view = yoctui_model::RawModeView::Form;
    app.raw_mode.focus = yoctui_model::RawModeFocus::Form;
    app.raw_mode.form = Some(yoctui_model::RawCommandForm {
        command: command.id,
        fields: std::collections::BTreeMap::from([(
            parameter.clone(),
            yoctui_model::RawFormField {
                parameter: parameter.clone(),
                editor: yoctui_model::PopupEditor::new("manual-recipe".into()),
                value: Some(yoctui_model::RawParameterValue::Recipe(
                    "manual-recipe".into(),
                )),
                validation_error: None,
            },
        )]),
        field_order: vec![parameter.clone()],
        field_selection: 0,
        additional_arguments: yoctui_model::RawArgvEditor::new("").unwrap(),
        capability_generation: 1,
        build_directory: "/work/build".into(),
    });

    let open = raw_mode_input(&app, Input::Char('r')).unwrap();
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::RawMode(open));
    assert_eq!(
        app.raw_mode.recipe_picker.as_ref().unwrap().recipes,
        ["bash", "busybox", "linux-aspeed"]
    );
    for character in "linux".chars() {
        let action = raw_mode_input(&app, Input::Char(character)).unwrap();
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::RawMode(action));
    }
    let choose = raw_mode_input(&app, Input::Enter).unwrap();
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::RawMode(choose));
    assert!(app.raw_mode.recipe_picker.is_none());
    let field = &app.raw_mode.form.as_ref().unwrap().fields[&parameter];
    assert_eq!(field.editor.text, "linux-aspeed");
    assert_eq!(
        field.value,
        Some(yoctui_model::RawParameterValue::Recipe(
            "linux-aspeed".into()
        ))
    );

    let edit = raw_mode_input(&app, Input::Char('i')).unwrap();
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::RawMode(edit));
    let typed = raw_mode_input(&app, Input::Char('2')).unwrap();
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::RawMode(typed));
    assert_eq!(
        app.raw_mode.form.as_ref().unwrap().fields[&parameter]
            .editor
            .text,
        "linux-aspeed2"
    );

    let back = raw_mode_input(&app, Input::BackTab).unwrap();
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::RawMode(back));
    assert_eq!(app.raw_mode.form.as_ref().unwrap().field_selection, 1);
    let wrap = raw_mode_input(&app, Input::Tab).unwrap();
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::RawMode(wrap));
    assert_eq!(app.raw_mode.form.as_ref().unwrap().field_selection, 0);
}
