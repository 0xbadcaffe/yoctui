use super::*;

#[test]
fn image_console_input_keeps_text_typing_separate_from_choice_navigation() {
    let identity = yoctui_model::ImageArtifactIdentity {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        path: "/deploy/image.wic".into(),
    };
    let mut dialog =
        yoctui_model::ImageConsoleDialog::new(yoctui_model::ImageConsoleDraft::for_artifact(
            identity,
            yoctui_model::ImageArtifactKind::Wic,
        ));
    assert_eq!(
        image_console_dialog_action(&dialog, Input::Right),
        Some(Action::CycleImageConsoleChoice { backwards: false })
    );
    dialog.draft.mode = yoctui_model::ImageConsoleMode::Ssh;
    dialog.selected_field = yoctui_model::ImageConsoleField::Host;
    assert_eq!(
        image_console_dialog_action(&dialog, Input::Char('j')),
        Some(Action::AppendImageConsoleField('j'))
    );
    assert_eq!(
        image_console_dialog_action(&dialog, Input::Enter),
        Some(Action::ConfirmImageConsole)
    );
    assert_eq!(
        image_console_dialog_action(&dialog, Input::Esc),
        Some(Action::CancelImageConsole)
    );
}
