use super::*;

#[test]
fn yocto_utility_dialog_renders_typed_fields_validation_and_controls() {
    let mut app = App::new(16, 4096);
    let mut dialog =
        yoctui_model::YoctoUtilityDialog::new(yoctui_model::YoctoUtilityCommand::LayersCreateLayer);
    dialog.validation_error = Some("layer directory is required".into());
    app.dialogs.push_back(Dialog::YoctoUtility(dialog));
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| render_at(frame, &app, UNIX_EPOCH))
        .unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Create layer"), "{output}");
    assert!(output.contains("bitbake-layers"), "{output}");
    assert!(output.contains("Layer directory"), "{output}");
    assert!(output.contains("Add to bblayers.conf"), "{output}");
    assert!(output.contains("layer directory is required"), "{output}");
    assert!(output.contains("Enter reviews exact command"), "{output}");
}

#[test]
fn complete_layer_forms_render_all_command_specific_options() {
    let mut app = App::new(16, 4096);
    app.dialogs
        .push_back(Dialog::YoctoUtility(yoctui_model::YoctoUtilityDialog::new(
            yoctui_model::YoctoUtilityCommand::LayersShowRecipes,
        )));
    let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
    terminal
        .draw(|frame| render_at(frame, &app, UNIX_EPOCH))
        .unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    for expected in [
        "Recipe patterns",
        "Show filenames",
        "Recipes only",
        "Multiple providers only",
        "Inherited classes",
        "Bare names",
        "Show variants",
        "Multiconfig",
    ] {
        assert!(output.contains(expected), "{expected}: {output}");
    }
}

#[test]
fn complete_devtool_form_renders_command_specific_options() {
    let mut app = App::new(16, 4096);
    app.dialogs
        .push_back(Dialog::YoctoUtility(yoctui_model::YoctoUtilityDialog::new(
            yoctui_model::YoctoUtilityCommand::Devtool(yoctui_model::DevtoolUtilityCommand::Add),
        )));
    let mut terminal = Terminal::new(TestBackend::new(110, 36)).unwrap();
    terminal
        .draw(|frame| render_at(frame, &app, UNIX_EPOCH))
        .unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    for expected in [
        "Tool: devtool",
        "Fetch URI syntax",
        "Build directory",
        "Source revision",
        "Also add native variant",
        "Provides alias",
    ] {
        assert!(output.contains(expected), "{expected}: {output}");
    }
}

#[test]
fn application_menu_renders_the_dedicated_devtool_group() {
    let mut app = App::new(16, 4096);
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::OpenApplicationMenu);
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal
        .draw(|frame| render_at(frame, &app, UNIX_EPOCH))
        .unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Devtool"), "{output}");
    assert!(output.contains("Workspace"), "{output}");
    assert!(output.contains("Tools"), "{output}");
}
