use super::*;

#[test]
fn ux_textarea_mouse_down_and_drag_map_to_typed_cursor_selection() {
    let mut app = yoctui_model::App::new(10, 1_000);
    app.dialogs
        .push_front(yoctui_model::Dialog::BuildEnvironmentEditor(
            yoctui_model::PopupEditor::new("alpha\nbeta\n".into()),
        ));
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 8,
                row: 4,
            },
            &app,
            100,
            30,
        ),
        Some(Action::EditActivePopup(
            PopupEditorCommand::SelectPosition {
                line: 0,
                column: 2,
                extend: false,
            }
        ))
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Drag,
                column: 9,
                row: 5,
            },
            &app,
            100,
            30,
        ),
        Some(Action::EditActivePopup(
            PopupEditorCommand::SelectPosition {
                line: 1,
                column: 3,
                extend: true,
            }
        ))
    );
}
