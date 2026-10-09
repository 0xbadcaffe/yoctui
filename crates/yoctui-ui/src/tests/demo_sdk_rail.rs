use super::*;

#[test]
fn demo_sdk_rail_keeps_all_modifier_routes_and_cancel_as_complete_tokens() {
    let mut app = App::new(32, 8192);
    app.screen = Screen::Sdk;
    app.focus = FocusTarget::Workspace;
    for width in 80..=200 {
        let mut terminal = Terminal::new(TestBackend::new(width, 24)).unwrap();
        terminal
            .draw(|frame| render_at(frame, &app, literal_now()))
            .unwrap();
        let row: String = (0..width)
            .map(|x| terminal.backend().buffer()[(x, 22)].symbol())
            .collect();
        assert!(
            row.contains(if width < 102 { "c:cancel" } else { "c cancel" }),
            "{width}: {row}"
        );
        if width < 102 {
            for token in [
                "i:img",
                "s/Alt+e:SDK",
                "t/Alt+t:tst",
                "Alt+r:scan",
                "Alt+p:pub",
                "n:native",
                "o:open",
            ] {
                assert!(row.contains(token), "{width} lost {token}: {row}");
            }
        }
    }
    for width in 0..100 {
        let rail = responsive_footer_shortcuts(&app, width);
        assert!(Line::from(rail.as_str()).width() <= usize::from(width));
        for token in rail.split_whitespace() {
            assert!(
                [
                    "↑↓",
                    "i:img",
                    "s/Alt+e:SDK",
                    "t/Alt+t:tst",
                    "c:cancel",
                    "Alt+r:scan",
                    "Alt+p:pub",
                    "n:native",
                    "o:open"
                ]
                .contains(&token),
                "partial token {token}"
            );
        }
    }
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert!(app.sdk_sessions.is_empty());
}

#[test]
fn demo_sdk_rail_respects_navigator_and_dialog_focus() {
    let mut app = App::new(32, 8192);
    app.screen = Screen::Sdk;
    app.focus = FocusTarget::Navigator;
    app.navigator_selection = 10;
    let output = rendered_text(&app, 80, 24);
    assert!(!output.contains("s/Alt+e:SDK"), "{output}");
    assert!(!output.contains("c:cancel"), "{output}");
    assert!(output.contains("Tab Focus"), "{output}");
    assert!(output.contains("Ctrl+P Menu"), "{output}");
    app.dialogs.push_back(Dialog::BuildOptions);
    app.focus = FocusTarget::Dialog;
    let output = rendered_text(&app, 80, 24);
    assert!(output.contains("Esc cancel"), "{output}");
    assert!(!output.contains("s/Alt+e:SDK"), "{output}");
}
