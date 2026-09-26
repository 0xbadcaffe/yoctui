use super::*;
use yoctui_model::{HardwareAction, HardwareCategory, HardwareDocument, HardwareDocumentKind};

fn hardware_app() -> App {
    let mut app = App::new(100, 100_000);
    app.screen = Screen::Hardware;
    app.hardware.documents.push(HardwareDocument {
        path: PathBuf::from("/tmp/board.pdf"),
        category: HardwareCategory::Board,
        kind: HardwareDocumentKind::Pdf,
    });
    app
}

#[test]
fn hardware_library_browser_and_viewer_have_typed_controls() {
    let mut app = hardware_app();
    assert_eq!(
        hardware_workspace_action(&app, Input::Right),
        Some(Action::Hardware(HardwareAction::SelectCategory {
            delta: 1
        }))
    );
    assert!(matches!(
        hardware_workspace_action(&app, Input::Char('a')),
        Some(Action::Hardware(HardwareAction::OpenBrowser { .. }))
    ));
    let _ = update(
        &mut app,
        Action::Hardware(HardwareAction::OpenBrowser {
            directory: PathBuf::from("/tmp"),
        }),
    );
    assert_eq!(
        hardware_workspace_action(&app, Input::Esc),
        Some(Action::Hardware(HardwareAction::CancelBrowser))
    );
    let _ = update(&mut app, Action::Hardware(HardwareAction::CancelBrowser));
    let _ = update(&mut app, Action::Hardware(HardwareAction::OpenSelected));
    assert_eq!(
        hardware_workspace_action(&app, Input::Char('+')),
        Some(Action::Hardware(HardwareAction::Zoom { delta: 25 }))
    );
    assert_eq!(
        hardware_workspace_action(&app, Input::Char('/')),
        Some(Action::Hardware(HardwareAction::BeginSearch))
    );
}

#[test]
fn hardware_remove_confirmation_traps_unrelated_input() {
    let mut app = hardware_app();
    let _ = update(&mut app, Action::Hardware(HardwareAction::BeginRemove));
    assert_eq!(hardware_workspace_action(&app, Input::Down), None);
    assert_eq!(
        hardware_workspace_action(&app, Input::Esc),
        Some(Action::Hardware(HardwareAction::CancelRemove))
    );
}
