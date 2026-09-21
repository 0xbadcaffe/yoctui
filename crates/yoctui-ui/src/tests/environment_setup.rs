use super::*;
use ratatui::{Terminal, backend::TestBackend};
#[test]
fn environment_setup_popup_keeps_bottom_selection_and_controls_visible() {
    let mut app = App::new(20, 2000);
    let mut setup = EnvironmentSetup {
        values: Default::default(),
        field: 0,
        editor: None,
        error: None,
        browser: Some(yoctui_model::EnvironmentBrowser {
            request: 1,
            loading: false,
            selection: 99,
            directory: Some(yoctui_model::EnvironmentDirectory {
                path: "/src".into(),
                children: (0..100)
                    .map(|i| format!("/src/folder-{i:03}").into())
                    .collect(),
                init_script: None,
                notice: None,
            }),
        }),
    };
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| environment_setup_popup(frame, &app, &setup, frame.area()))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains("> folder-099/"));
        assert!(text.contains("s use this directory"));
        assert!(text.contains("Esc back"));
    }
    setup.browser = None;
    app.dialogs
        .push_front(Dialog::EnvironmentSetup(Box::new(setup)));
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains("Configure build environment"));
    assert!(text.contains("b browse"));
    assert!(text.contains("s save profile"));
}
