#[test]
fn raw_form_routes_manual_selector_and_argv_edits_to_exact_preview() {
    let mut app = raw_command_list_app();
    set_raw_command_query(&mut app, "--continue <target>");
    let _ = update(
        &mut app,
        Action::RawMode(yoctui_model::RawModeAction::OpenSelected),
    );

    let request = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Enter).unwrap();
    assert_eq!(update(&mut app, Action::RawMode(request)), None);
    let invalid = rendered_text(&app, 100, 30);
    assert!(
        invalid.contains("ERROR: Raw parameter target is required"),
        "{invalid}"
    );
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Form);

    let choose = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Right).unwrap();
    assert!(matches!(
        choose,
        yoctui_model::RawModeAction::ChooseParameter { .. }
    ));
    let _ = update(&mut app, Action::RawMode(choose));
    let edit = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Char('e')).unwrap();
    let _ = update(&mut app, Action::RawMode(edit));
    for character in "vé".chars() {
        let action = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Char(character)).unwrap();
        let _ = update(&mut app, Action::RawMode(action));
    }
    let unicode_error = rendered_text(&app, 100, 30);
    assert!(unicode_error.contains("ERROR:"), "{unicode_error}");
    assert!(!unicode_error.contains('�'), "{unicode_error}");
    let normal = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Esc).unwrap();
    let _ = update(&mut app, Action::RawMode(normal));
    let choose = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Right).unwrap();
    let _ = update(&mut app, Action::RawMode(choose));

    let next = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Tab).unwrap();
    let _ = update(&mut app, Action::RawMode(next));
    let insert = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Char('i')).unwrap();
    let _ = update(&mut app, Action::RawMode(insert));
    for character in "--verbose".chars() {
        let action = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Char(character)).unwrap();
        let _ = update(&mut app, Action::RawMode(action));
    }
    let preview = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Enter).unwrap();
    assert_eq!(update(&mut app, Action::RawMode(preview)), None);
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Preview);
    assert_eq!(app.focus, FocusTarget::Dialog);
    let output = rendered_text(&app, 160, 50);
    for expected in [
        "Exact indexed native argv:",
        "[0] bitbake",
        "[1] --continue",
        "[2] busybox",
        "[3] --verbose",
    ] {
        assert!(output.contains(expected), "missing {expected:?}: {output}");
    }

    let back = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Esc).unwrap();
    let _ = update(&mut app, Action::RawMode(back));
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Form);
    let normal = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Esc).unwrap();
    let _ = update(&mut app, Action::RawMode(normal));
    let close = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Char('q')).unwrap();
    let _ = update(&mut app, Action::RawMode(close));
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Browser);
    assert_eq!(app.focus, FocusTarget::Workspace);
}

#[test]
fn raw_recipe_picker_renders_searchable_inventory_above_the_form() {
    let mut app = raw_command_list_app();
    let command = yoctui_model::builtin_raw_catalog()
        .commands
        .iter()
        .find(|command| {
            command.parameters.iter().any(|parameter| {
                parameter.kind == yoctui_model::RawParameterKind::Recipe
            })
        })
        .unwrap();
    let parameter = command.parameters[0].id.clone();
    app.workspace.recipes = ["bash", "busybox", "linux-aspeed"]
        .into_iter()
        .map(|name| Recipe {
            name: name.into(),
            file: Some(format!("/layers/{name}.bb").into()),
            ..Recipe::default()
        })
        .collect();
    app.raw_mode.view = yoctui_model::RawModeView::Form;
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
        field_order: vec![parameter],
        field_selection: 0,
        additional_arguments: yoctui_model::RawArgvEditor::new("").unwrap(),
        capability_generation: 19,
        build_directory: "/work/build".into(),
    });

    let open = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Char('r')).unwrap();
    let _ = update(&mut app, Action::RawMode(open));
    for character in "linux".chars() {
        let filter = yoctui_app::raw_mode_input(&app, yoctui_app::Input::Char(character)).unwrap();
        let _ = update(&mut app, Action::RawMode(filter));
    }
    let output = rendered_text(&app, 120, 36);
    for expected in [
        "Choose recipe",
        "Search: linux",
        "linux-aspeed",
        "Enter use recipe",
    ] {
        assert!(output.contains(expected), "missing {expected:?}: {output}");
    }
    assert!(!output.contains(" busybox\n"), "filtered list leaked: {output}");
}
