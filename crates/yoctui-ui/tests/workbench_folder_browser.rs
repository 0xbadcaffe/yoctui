use ratatui::{Terminal, backend::TestBackend};
use yoctui_app::{MouseInput, MouseKind, mouse_action_for_app};
use yoctui_model::*;

fn frame(app: &App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| yoctui_ui::render(frame, app))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .chunks(width as usize)
        .map(|line| line.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn folder_browser_last_directory_and_mouse_rows_match_after_resize() {
    let mut app = App::new(32, 8192);
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::OpenRecipe {
            recipe: "busybox".into(),
            root: "/source".into(),
        }),
    );
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::Form(WorkbenchFormKind::AddFolder)),
    );
    let generation = app.workbench.folder_browser.as_ref().unwrap().generation;
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::FoldersBrowsed {
            generation,
            result: Ok((
                "/source".into(),
                (0..100)
                    .map(|i| format!("/source/folder-{i:03}").into())
                    .collect(),
            )),
        }),
    );
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::BrowseSelect(isize::MAX)),
    );
    for (width, height) in [(80, 24), (100, 30), (160, 48), (240, 80)] {
        let text = frame(&app, width, height);
        assert!(text.contains("browse filesystem"), "{text}");
        let (row, line) = text
            .lines()
            .enumerate()
            .find(|(_, line)| line.contains("▶ folder-099/"))
            .unwrap();
        let column = line.chars().position(|c| c == '▶').unwrap();
        let mouse = MouseInput {
            kind: MouseKind::Down,
            column: column as u16,
            row: row as u16,
        };
        assert_eq!(
            mouse_action_for_app(mouse, &app, width, height),
            Some(Action::Workbench(WorkbenchAction::BrowseOpen)),
            "{width}x{height}\n{text}"
        );
        assert_eq!(
            mouse_action_for_app(
                MouseInput {
                    kind: MouseKind::ScrollUp,
                    ..mouse
                },
                &app,
                width,
                height
            ),
            Some(Action::Workbench(WorkbenchAction::BrowseSelect(-1)))
        );
        assert!(
            mouse_action_for_app(
                MouseInput {
                    kind: MouseKind::Down,
                    column: 2,
                    row: 8
                },
                &app,
                width,
                height
            )
            .is_none()
        );
    }
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::BrowseSelect(isize::MIN)),
    );
    assert!(frame(&app, 80, 24).contains("▶ ./"));
}
