#[test]
fn config_scope_renders_picker_active_identity_and_global_fallback() {
    for (width, height) in [(140, 32), (100, 28), (90, 24)] {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Configuration;
        app.focus = FocusTarget::Workspace;
        app.workspace
            .variables
            .insert("MACHINE".into(), "global-summary".into());
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "base-files".into(),
            ..yoctui_model::Recipe::default()
        });
        app.config_scope = Some("base-files".into());
        let identity = yoctui_model::VariableIdentity {
            name: "MACHINE".into(),
            recipe: Some("base-files".into()),
        };
        app.variable_detail_errors
            .insert(identity, "scoped Tinfoil failure".into());
        let output = rendered_text(&app, width, height);
        if width >= 80 && height >= 24 {
            assert!(output.contains("scoped Tinfoil failure"), "{output}");
            assert!(output.contains("active base-files"), "{output}");
        }

        app.dialogs
            .push_back(Dialog::ConfigScopePicker(yoctui_model::ConfigScopePicker {
                variable: "MACHINE".into(),
                scopes: vec![None, Some("base-files".into())],
                selection: 1,
            }));
        let output = rendered_text(&app, width, height);
        assert!(output.contains("MACHINE scope"), "{output}");
        assert!(output.contains("(global)"), "{output}");
        assert!(output.contains("base-files"), "{output}");
    }

    let mut app = App::new(10, 1_000);
    app.screen = Screen::Configuration;
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    let output = rendered_text(&app, 120, 28);
    assert!(output.contains("global only"), "{output}");
    assert!(output.contains("no recipes reported"), "{output}");
}
