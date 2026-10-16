use super::*;
use yoctui_model::{RawBrowserColumn, RawModeAction as Raw, RawModeView as View};

#[test]
fn raw_mouse_commands_use_workspace_geometry_and_current_scrolled_viewport() {
    for (width, height) in [(80, 24), (100, 30), (160, 48), (240, 80)] {
        for inspector in [false, true] {
            let mut app = yoctui_model::App::new(16, 4096);
            app.screen = Screen::RawMode;
            app.focus = FocusTarget::Workspace;
            app.inspector_visible = inspector;
            app.raw_mode.search.query = "bitbake".into();
            yoctui_model::update(
                &mut app,
                Action::RawMode(Raw::SelectCommand { delta: isize::MAX }),
            );
            let shell = workbench_shell(&app, width, height).unwrap();
            let region = (0..width)
                .find_map(|column| {
                    workbench_mouse_region(
                        MouseInput {
                            kind: MouseKind::Down,
                            column,
                            row: shell.y + 2,
                        },
                        &app,
                        shell,
                    )
                    .filter(|region| region.target == FocusTarget::Workspace)
                })
                .unwrap();
            let mut area = region.area;
            area.y += app.raw_mode.workspace_status_rows();
            area.height -= app.raw_mode.workspace_status_rows();
            let commands = app
                .raw_mode
                .visible_commands(yoctui_model::builtin_raw_catalog());
            let current = commands.len() - 1;
            let visible = (usize::from(area.height.saturating_sub(2)) / 2).max(1);
            let range =
                yoctui_model::centered_viewport_range(Some(current), commands.len(), visible);
            let column = area.x
                + if width >= 100 {
                    yoctui_model::raw_category_column_width(area.width) + 1
                } else {
                    1
                };
            let mouse = MouseInput {
                kind: MouseKind::Down,
                column,
                row: area.y + 1,
            };
            assert_eq!(
                raw_mouse_action(mouse, &app, width, height),
                Some(Raw::SelectCommand {
                    delta: range.start as isize - current as isize
                })
            );
            assert_eq!(app.raw_mode.browser_column, RawBrowserColumn::Commands);
            let navigator = workbench_mouse_region(
                MouseInput {
                    column: 1,
                    row: shell.y + 2,
                    ..mouse
                },
                &app,
                shell,
            )
            .filter(|region| region.target == FocusTarget::Navigator);
            if let Some(navigator) = navigator {
                assert_eq!(
                    raw_mouse_action(
                        MouseInput {
                            column: navigator.area.x + 1,
                            row: navigator.area.y + 1,
                            ..mouse
                        },
                        &app,
                        width,
                        height
                    ),
                    None
                );
            }
            app.raw_mode.view = View::Execution;
            assert_eq!(
                raw_mouse_action(
                    MouseInput {
                        kind: MouseKind::ScrollDown,
                        ..mouse
                    },
                    &app,
                    width,
                    height
                ),
                Some(Raw::ScrollOutput {
                    vertical: -1,
                    horizontal: 0
                })
            );
        }
    }
}
