use super::*;

#[test]
fn wic_device_write_and_creation_map_distinct_modal_and_artifact_keys() {
    assert_eq!(
        images_workspace_action(false, Input::Char('W')),
        Some(Action::BeginSelectedWicCreate)
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('w')),
        Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Wic
        ))
    );
    assert_eq!(
        wic_create_dialog_action(false, Input::Down),
        Some(Action::SelectWicCreateField { delta: 1 })
    );
    assert_eq!(
        wic_create_dialog_action(false, Input::Left),
        Some(Action::CycleWicCreateChoice { backwards: true })
    );
    assert_eq!(
        wic_create_dialog_action(true, Input::Char('/')),
        Some(Action::AppendWicCreateField('/'))
    );
    assert_eq!(
        wic_create_dialog_action(false, Input::Char('p')),
        Some(Action::PreviewWicCreate)
    );
    assert_eq!(
        wic_create_confirmation_action(Input::Enter),
        Some(Action::ConfirmWicCreate)
    );
    assert_eq!(
        wic_cancellation_confirmation_action(WicSessionId(3), false, Input::Enter),
        Some(Action::ConfirmWicSessionCancellation {
            id: WicSessionId(3),
            acknowledge_incomplete_device: false,
        })
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('D')),
        Some(Action::BeginSelectedWicDeviceWrite)
    );
    assert_eq!(
        wic_device_picker_action(Input::Down),
        Some(Action::SelectWicDevice { delta: 1 })
    );
    assert_eq!(
        wic_device_picker_action(Input::Enter),
        Some(Action::ConfirmWicDeviceSelection)
    );
    assert_eq!(
        wic_write_phrase_action(Input::Char('W')),
        Some(Action::AppendWicWritePhrase('W'))
    );
    assert_eq!(
        wic_write_phrase_action(Input::Enter),
        Some(Action::PreviewWicDeviceWrite)
    );
    assert_eq!(
        wic_write_confirmation_action(Input::Enter),
        Some(Action::ConfirmWicDeviceWrite)
    );
    assert_eq!(
        wic_cancellation_confirmation_action(WicSessionId(4), true, Input::Enter),
        Some(Action::ConfirmWicSessionCancellation {
            id: WicSessionId(4),
            acknowledge_incomplete_device: true,
        })
    );
}
