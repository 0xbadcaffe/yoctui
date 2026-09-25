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
