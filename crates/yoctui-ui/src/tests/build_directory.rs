use super::*;

#[test]
fn incomplete_directory_is_not_presented_as_an_empty_filesystem() {
    let app = App::new(8, 1024);
    let mut browser = LayerBrowser::new("Build directory".into(), "/build".into());
    browser.incomplete.insert(browser.root.clone());
    let text = crate::package_render::layer_inspector_text(&app, &browser);
    assert_eq!(
        text,
        Text::from("Directory listing is loading or incomplete; reopen or refresh to retry.")
    );
}

#[test]
fn build_directory_renders_real_root_and_tree_without_configured_layer_list() {
    let mut app = App::new(20, 1000);
    app.screen = Screen::Layers;
    app.focus = FocusTarget::Workspace;
    let mut browser = LayerBrowser::new("Build directory".into(), "/work/poky-build".into());
    browser.nodes.insert(
        browser.root.clone(),
        vec![LayerBrowserEntry {
            path: "/work/poky-build/conf".into(),
            is_dir: true,
            ..Default::default()
        }],
    );
    browser.entries = browser.nodes[&browser.root].clone();
    for (width, height) in [(160, 40), (100, 30), (80, 24)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| layer_browser(frame, &app, &browser, frame.area()))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains("Build directory: /work/poky-build"), "{text}");
        assert!(text.contains("conf"), "{text}");
        assert!(!text.contains("Configured layers"), "{text}");
    }
}
