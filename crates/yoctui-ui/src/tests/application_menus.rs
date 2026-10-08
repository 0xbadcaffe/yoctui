use super::*;

#[test]
fn application_menu_border_matches_centered_mouse_bounds() {
    let mut app = App::new(10, 1024);
    update(&mut app, Action::OpenApplicationMenu);
    for (width, height) in [(80, 24), (101, 31), (160, 50), (240, 80)] {
        let (left, top, menu_width, menu_height) =
            yoctui_app::application_menu_bounds(&app, width, height, app.active_menu_items().len())
                .unwrap();
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render_at(frame, &app, literal_now()))
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(left, top)].symbol(), "╔");
        assert_eq!(buffer[(left + menu_width - 1, top)].symbol(), "╗");
        assert_eq!(buffer[(left, top + menu_height - 1)].symbol(), "╚");
        assert_eq!(
            buffer[(left + menu_width - 1, top + menu_height - 1)].symbol(),
            "╝"
        );
    }
}

#[test]
fn every_application_menu_group_and_selected_action_is_visible_on_laptop_terminals() {
    let mut app = App::new(10, 1024);
    update(&mut app, Action::OpenApplicationMenu);
    for (width, height) in [(80, 24), (100, 30), (160, 50)] {
        for (index, group) in yoctui_model::ApplicationMenuGroup::ALL.iter().enumerate() {
            app.menu.group_selection = index;
            let count = app.active_menu_items().len();
            for selected in [0, count.saturating_sub(1)] {
                app.menu.item_selection = selected;
                let output = rendered_text(&app, width, height);
                for visible_group in yoctui_model::ApplicationMenuGroup::ALL {
                    assert!(
                        output.contains(visible_group.label()),
                        "missing {} at {width}x{height}: {output}",
                        visible_group.label()
                    );
                }
                if let Some(item) = app.selected_menu_item() {
                    assert!(
                        output.contains(item.label),
                        "{} / {} at {width}x{height}: {output}",
                        group.label(),
                        item.label
                    );
                }
                assert!(!output.contains('�'));
            }
        }
    }
}
