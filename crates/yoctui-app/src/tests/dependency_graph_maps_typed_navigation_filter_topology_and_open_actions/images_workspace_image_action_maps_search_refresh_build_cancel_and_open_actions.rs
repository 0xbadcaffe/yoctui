use super::*;

#[test]
fn images_workspace_image_action_maps_search_refresh_build_cancel_and_open_actions() {
    assert_eq!(
        images_workspace_action(false, Input::Up),
        Some(Action::SelectImageArtifact { delta: -1 })
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('R')),
        Some(Action::RefreshImageArtifactInventory)
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('b')),
        Some(Action::BeginSelectedImageArtifactBuild)
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('T')),
        Some(Action::BeginSelectedImageConsole)
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('c')),
        Some(Action::CancelImageArtifactOperation)
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('o')),
        Some(Action::OpenSelectedImageArtifact)
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('m')),
        Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Manifest
        ))
    );
    assert_eq!(
        images_workspace_action(false, Input::Char('/')),
        Some(Action::BeginImageArtifactSearch)
    );
    assert_eq!(
        images_workspace_action(true, Input::Char('w')),
        Some(Action::AppendImageArtifactQuery('w'))
    );
    assert_eq!(
        images_workspace_action(true, Input::Esc),
        Some(Action::FinishImageArtifactSearch)
    );
}
