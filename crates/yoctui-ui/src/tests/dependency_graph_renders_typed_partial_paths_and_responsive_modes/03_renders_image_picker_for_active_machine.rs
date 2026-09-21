#[test]
fn renders_image_picker_for_active_machine() {
    let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
    let mut app = App::new(10, 1_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.dialogs
        .push_back(Dialog::ImagePicker(yoctui_model::ImagePicker {
            images: vec!["core-image-minimal".into()],
            selection: 0,
        }));
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Available image targets"));
    assert!(output.contains("qemux86-64"));
    assert!(output.contains("core-image-minimal"));
}
#[test]
fn inspector_reflects_selected_recipe_and_layer_preview() {
    let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Recipes;
    app.workspace.recipes.push(yoctui_model::Recipe {
        name: "busybox".into(),
        version: Some("1.36".into()),
        layer: Some("meta".into()),
        ..yoctui_model::Recipe::default()
    });
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("Recipe: busybox"));
    assert!(output.contains("Resolved version: 1.36"));

    app.screen = Screen::Layers;
    let mut browser = LayerBrowser::new("meta".into(), "/layers/meta".into());
    browser.directory = "/layers/meta/conf".into();
    browser.entries.push(yoctui_model::LayerBrowserEntry {
        path: "/layers/meta/conf/layer.conf".into(),
        ..yoctui_model::LayerBrowserEntry::default()
    });
    browser.preview = "BBFILE_COLLECTIONS += \"meta\"".into();
    browser.preview_kind = yoctui_model::PreviewKind::Text;
    app.layer_browser = Some(browser);
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("/layers/meta/conf/layer.conf"));
    assert!(output.contains("BBFILE_COLLECTIONS"));
}
