use super::*;
use yoctui_model::{HardwareAction, HardwareCategory, HardwareDocument, HardwareDocumentKind};

#[test]
fn hardware_project_controls_trap_forms_and_keep_legacy_library_keys() {
    use yoctui_model::{HardwareProject, HardwareProjectAction as A, HardwareProjectForm};
    let mut app = hardware_app();
    assert_eq!(
        hardware_workspace_action(&app, Input::Char('p')),
        Some(Action::Hardware(HardwareAction::Project(A::Toggle)))
    );
    app.hardware.projects.visible = true;
    app.focus = yoctui_model::FocusTarget::Workspace;
    assert!(!hardware_project_owns_input(&app, Input::Char('a'))); // No project yet.
    assert_eq!(
        hardware_workspace_action(&app, Input::Char('n')),
        Some(Action::Hardware(HardwareAction::Project(A::NewName)))
    );
    app.hardware.projects.form = Some(HardwareProjectForm::Name {
        value: String::new(),
    });
    assert_eq!(
        hardware_workspace_action(&app, Input::Char('p')),
        Some(Action::Hardware(HardwareAction::Project(A::EditName('p'))))
    );
    assert_eq!(hardware_workspace_action(&app, Input::Tab), None);
    app.hardware.projects.project = Some(HardwareProject {
        name: "board".into(),
        root: "/tmp/board".into(),
        progress: [0; 6],
    });
    app.hardware.projects.form = None;
    assert!(hardware_project_owns_input(&app, Input::Char('a')));
    app.hardware.projects.form = Some(HardwareProjectForm::Progress {
        values: [0; 6],
        stage: 0,
        digits: String::new(),
    });
    assert_eq!(
        hardware_workspace_action(&app, Input::Tab),
        Some(Action::Hardware(HardwareAction::Project(A::SelectStage {
            delta: 1
        })))
    );
    assert_eq!(
        hardware_workspace_action(&app, Input::Char('5')),
        Some(Action::Hardware(HardwareAction::Project(A::ProgressDigit(
            '5'
        ))))
    );
    assert_eq!(hardware_workspace_action(&app, Input::Char('p')), None);
    app.hardware.projects.form = None;
    app.hardware.projects.import_browser = Some(("/tmp".into(), Vec::new(), 0));
    assert_eq!(
        hardware_workspace_action(&app, Input::Enter),
        Some(Action::Hardware(HardwareAction::Project(A::ImportOpen)))
    );
    assert_eq!(hardware_workspace_action(&app, Input::Char('n')), None);
    app.hardware.projects.import_browser = None;
    let _ = update(&mut app, Action::Hardware(HardwareAction::OpenSelected));
    app.focus = yoctui_model::FocusTarget::Navigator;
    assert!(hardware_project_owns_input(&app, Input::Esc));
    assert!(hardware_project_owns_input(&app, Input::Backspace));
    assert!(!hardware_project_owns_input(&app, Input::Enter));
}

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
        hardware_workspace_action(&app, Input::Char('=')),
        Some(Action::Hardware(HardwareAction::Zoom { delta: 25 }))
    );
    assert_eq!(
        hardware_workspace_action(&app, Input::Char('/')),
        Some(Action::Hardware(HardwareAction::BeginSearch))
    );
    assert_eq!(
        hardware_workspace_action(&app, Input::Char('v')),
        Some(Action::Hardware(HardwareAction::TogglePresentation))
    );
    assert_eq!(
        hardware_workspace_action(&app, Input::Backspace),
        Some(Action::Hardware(HardwareAction::CloseViewer))
    );
    assert_eq!(
        focus_action_for_app(&app, Input::Tab),
        Some(Action::CycleFocus { backwards: false })
    );
    assert_eq!(workbench_pane_widths(&app, 160, 50), [22, 138, 0]);
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

#[test]
fn hardware_pdf_mouse_wheel_selects_pages_instead_of_panning() {
    let mut app = hardware_app();
    app.focus = FocusTarget::Workspace;
    let _ = update(&mut app, Action::Hardware(HardwareAction::OpenSelected));
    let _ = update(
        &mut app,
        Action::Hardware(HardwareAction::PreviewLoaded {
            generation: 1,
            page_count: 3,
            preview: yoctui_model::HardwarePreview::Text {
                lines: vec!["page one".into()],
                limitation: None,
            },
            searchable_text: Vec::new(),
        }),
    );

    let wheel_down = mouse_action_for_app(
        MouseInput {
            kind: MouseKind::ScrollDown,
            column: 40,
            row: 12,
        },
        &app,
        160,
        50,
    );
    let wheel_up = mouse_action_for_app(
        MouseInput {
            kind: MouseKind::ScrollUp,
            column: 40,
            row: 12,
        },
        &app,
        160,
        50,
    );
    assert_eq!(
        wheel_down,
        Some(Action::Hardware(HardwareAction::ChangePage { delta: 1 }))
    );
    assert_eq!(
        wheel_up,
        Some(Action::Hardware(HardwareAction::ChangePage { delta: -1 }))
    );
}
