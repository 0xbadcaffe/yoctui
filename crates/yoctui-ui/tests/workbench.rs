use ratatui::{Terminal, backend::TestBackend};
use yoctui_model::*;

fn fixture() -> App {
    let mut app = App::new(32, 8192);
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::OpenRecipe {
            recipe: "busybox".into(),
            root: "/sources/busybox".into(),
        }),
    );
    app.notification = None;
    app
}
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
fn workbench_renders_session_browser_explorer_editor_and_actions_at_all_sizes() {
    for (width, height) in [(80, 24), (100, 30), (160, 48), (240, 80)] {
        let mut app = fixture();
        let text = frame(&app, width, height);
        assert!(text.contains("Explorer"), "{width}x{height}\n{text}");
        assert!(text.contains("Editor"), "{width}x{height}\n{text}");
        update(&mut app, Action::Workbench(WorkbenchAction::Menu));
        assert!(frame(&app, width, height).contains("Devtool"));
        update(&mut app, Action::Workbench(WorkbenchAction::Cancel));
        update(&mut app, Action::Workbench(WorkbenchAction::Sessions));
        assert!(frame(&app, width, height).contains("Workbench · Sessions"));
    }
}
#[test]
fn workbench_last_menu_item_stays_visible_on_small_terminals() {
    let mut app = fixture();
    update(&mut app, Action::Workbench(WorkbenchAction::Menu));
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::MenuSelect(isize::MAX)),
    );
    assert!(frame(&app, 80, 24).contains("Back to sessions"));
}
#[test]
fn workbench_panel_zoom_and_below_minimum_resize_are_safe() {
    let mut app = fixture();
    for zoom in [true, false] {
        app.workbench.current_mut().unwrap().panel_zoom = zoom;
        for (w, h) in [(79, 23), (80, 24), (160, 48)] {
            let text = frame(&app, w, h);
            assert!(!text.is_empty());
        }
    }
}
