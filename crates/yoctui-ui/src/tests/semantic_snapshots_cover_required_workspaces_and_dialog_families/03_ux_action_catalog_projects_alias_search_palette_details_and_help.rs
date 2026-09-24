#[test]
fn ux_action_catalog_projects_alias_search_palette_details_and_help() {
    let mut app = App::new(10, 1_000);
    app.command_palette_open = true;
    app.command_palette_query = "capabilities".into();
    let output = rendered_text(&app, 100, 25);
    assert!(output.contains("Open Compatibility"), "{output}");
    assert!(output.contains("environment identity"), "{output}");

    app.command_palette_open = false;
    app.command_palette_query.clear();
    app.screen = Screen::Help;
    let output = rendered_text(&app, 160, 50);
    assert!(output.contains("Operator guide"), "{output}");
    assert!(output.contains("Global action shortcuts"), "{output}");
    assert!(output.contains("Build > Build image"), "{output}");
    assert!(output.contains("[Navigate]"), "{output}");
    assert!(output.contains("Open Tasks"), "{output}");
    assert!(output.contains("About Yoctui"), "{output}");
    assert!(output.contains("Build SHA:"), "{output}");
}
#[test]
fn command_palette_selection_description_and_shortcut_render_in_all_themes() {
    for theme in [
        Theme::DarkPro,
        Theme::WhiteClassic,
        Theme::MatrixGreen,
        Theme::HighContrast,
        Theme::Monochrome,
    ] {
        let mut app = App::new(10, 1_000);
        app.theme = theme;
        app.command_palette_open = true;
        app.command_palette_query = "Open Settings".into();
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains("Open Settings"));
        assert!(output.contains("persistent visual"));
        assert!(output.contains("none"));
    }
}

#[test]
fn next_generation_palette_uses_documented_responsive_geometry() {
    for (area, expected) in [
        (Rect::new(0, 0, 200, 60), Rect::new(44, 15, 112, 30)),
        (Rect::new(0, 0, 160, 50), Rect::new(24, 10, 112, 30)),
        (Rect::new(0, 0, 130, 40), Rect::new(9, 5, 112, 30)),
        (Rect::new(0, 0, 100, 30), Rect::new(3, 2, 94, 26)),
        (Rect::new(0, 0, 80, 24), Rect::new(1, 1, 78, 22)),
    ] {
        assert_eq!(command_palette_rect(area), expected);
    }
    assert_eq!(
        command_palette_rect(Rect::new(7, 11, 100, 30)),
        Rect::new(10, 13, 94, 26)
    );
}
