use super::*;
use yoctui_app::{MouseInput, MouseKind, mouse_action_for_app};

#[test]
fn tasks_navigator_clicks_actual_rendered_destinations_across_geometries() {
    for (width, height, compact) in [
        (160, 50, false),
        (160, 48, false),
        (160, 50, true),
        (120, 40, false),
        (80, 24, false),
    ] {
        for (index, label, expected) in [
            (20, "Devtool", Screen::Devtool),
            (21, "QEMU / Wic", Screen::Images),
        ] {
            let mut app = App::new(10, 1024);
            app.screen = Screen::Tasks;
            app.focus = FocusTarget::Navigator;
            app.preferences.mouse_enabled = true;
            app.navigator_selection = index;
            if compact {
                app.preferences.density = yoctui_model::UiDensity::Compact;
            }
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| render_at(frame, &app, literal_now()))
                .unwrap();
            let buffer = terminal.backend().buffer();
            let literal = !compact && width == 160 && height == 48;
            let label = if literal && index == 21 { "Wic" } else { label };
            let row = (0..height)
                .find(|row| {
                    let text = (0..width.min(26))
                        .map(|col| buffer[(col, *row)].symbol())
                        .collect::<String>();
                    text.contains(label)
                })
                .unwrap_or_else(|| panic!("missing {label} at {width}x{height}"));
            let mouse = MouseInput {
                kind: MouseKind::Down,
                column: 10,
                row,
            };
            // If the literal row selected a different destination, the first
            // click selects it and the second must activate that exact row.
            let action = mouse_action_for_app(mouse, &app, width, height).unwrap();
            update(&mut app, action);
            if app.screen == Screen::Tasks && expected != Screen::Tasks {
                let action = mouse_action_for_app(mouse, &app, width, height).unwrap();
                update(&mut app, action);
            }
            assert_eq!(app.screen, expected, "{label} at {width}x{height}");
            assert!(app.daemon.jobs.is_empty());
            assert!(app.daemon.pty_sessions.is_empty());
        }
    }
}
