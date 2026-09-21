use super::*;

#[test]
fn compact_resource_meters_mouse_geometry_excludes_logs_and_meters() {
    let mut app = yoctui_model::App::new(100, 4096);
    for index in 0..100 {
        let id = TaskId(format!("task-{index:03}"));
        app.tasks.insert(
            id.clone(),
            TaskInfo::active(id, format!("recipe-{index}"), "do_compile".into()),
        );
    }
    app.task_progress_scroll = 50;
    for screen in [Screen::Dashboard, Screen::Tasks] {
        app.screen = screen;
        for (width, height) in [
            (101, 39),
            (89, 44),
            (76, 36),
            (78, 26),
            (80, 19),
            (86, 42),
            (116, 56),
        ] {
            let panels = task_workspace_panel_heights(&app, width, height);
            assert_eq!(panels.iter().sum::<u16>(), height);
            assert!(panels[3] >= 4);
            let area = MouseRect {
                x: 0,
                y: 2,
                width,
                height,
            };
            let summary = if screen == Screen::Dashboard { 4 } else { 2 };
            let count = panels[0] - summary - 3;
            let first = area.y + summary + 2;
            for offset in 0..count {
                assert_eq!(
                    task_row_click(
                        &app,
                        area,
                        MouseInput {
                            kind: MouseKind::Down,
                            column: 1,
                            row: first + offset
                        }
                    ),
                    Some(Action::ScrollBuildTasks {
                        delta: isize::try_from(offset).unwrap()
                            - isize::try_from(count / 2).unwrap()
                    })
                );
            }
            for row in [first - 1, first + count, area.y + height - 2] {
                assert_eq!(
                    task_row_click(
                        &app,
                        area,
                        MouseInput {
                            kind: MouseKind::Down,
                            column: 1,
                            row
                        }
                    ),
                    None
                );
            }
        }
    }
}
