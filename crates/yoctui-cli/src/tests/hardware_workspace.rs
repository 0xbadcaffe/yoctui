use super::*;
use yoctui_model::{
    HardwareDocument, HardwareDocumentKind, HardwarePresentation, HardwareViewerState,
};

#[test]
fn hardware_escape_reaches_open_viewer_while_navigator_owns_focus() {
    let mut app = App::new(10, 1024);
    app.screen = Screen::Hardware;
    app.focus = yoctui_model::FocusTarget::Navigator;
    app.hardware.viewer = Some(HardwareViewerState {
        document: HardwareDocument {
            path: PathBuf::from("/tmp/board.pdf"),
            category: yoctui_model::HardwareCategory::Board,
            kind: HardwareDocumentKind::Pdf,
        },
        generation: 1,
        page: 1,
        page_count: 1,
        zoom_percent: 100,
        fit_width: false,
        presentation: HardwarePresentation::Text,
        pan_x: 0,
        pan_y: 0,
        loading: false,
        preview: None,
        searchable_text: vec!["board".into()],
        query: String::new(),
        searching: false,
        matches: Vec::new(),
        match_selection: 0,
        error: None,
    });
    assert!(hardware_route_owns_input(&app, Input::Esc));
    assert!(hardware_route_owns_input(&app, Input::Backspace));
    assert!(!hardware_route_owns_input(&app, Input::Enter));
    let action = yoctui_app::hardware_workspace_action(&app, Input::Backspace).unwrap();
    let _ = compatibility_workspace_action(&mut app, action);
    assert!(app.hardware.viewer.is_none());
    assert_eq!(app.hardware.selection, 0);
}
