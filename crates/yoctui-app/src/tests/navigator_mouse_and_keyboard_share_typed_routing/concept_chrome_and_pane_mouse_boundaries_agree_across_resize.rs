use super::*;

#[test]
fn concept_chrome_and_pane_mouse_boundaries_agree_across_resize() {
    let mut app = yoctui_model::App::new(16, 4096);
    for screen in [
        Screen::Tasks,
        Screen::Errors,
        Screen::Images,
        Screen::TerminalSessions,
        Screen::Recipes,
    ] {
        app.screen = screen;
        for (width, height) in [(150, 50), (160, 50), (180, 55), (200, 60), (160, 48)] {
            let [header, footer] = workbench_chrome_heights(&app, width, height);
            let widths = workbench_pane_widths(&app, width, height);
            assert_eq!(widths.iter().sum::<u16>(), width);
            let shell = super::super::mouse::workbench_shell(&app, width, height).unwrap();
            assert_eq!(shell.y, header);
            assert_eq!(shell.bottom(), height - footer);
            for (column, target) in [
                (0, FocusTarget::Navigator),
                (widths[0], FocusTarget::Workspace),
                (
                    width - 1,
                    if widths[2] == 0 {
                        FocusTarget::Workspace
                    } else {
                        FocusTarget::Inspector
                    },
                ),
            ] {
                let mouse = MouseInput {
                    kind: MouseKind::Down,
                    column,
                    row: header,
                };
                let region =
                    super::super::mouse::workbench_mouse_region(mouse, &app, shell).unwrap();
                assert_eq!(region.target, target, "{screen:?} {width}x{height}");
            }
            for row in [header - 1, height - footer] {
                let mouse = MouseInput {
                    kind: MouseKind::Down,
                    column: width / 2,
                    row,
                };
                assert!(super::super::mouse::workbench_mouse_region(mouse, &app, shell).is_none());
            }
        }
    }
}
