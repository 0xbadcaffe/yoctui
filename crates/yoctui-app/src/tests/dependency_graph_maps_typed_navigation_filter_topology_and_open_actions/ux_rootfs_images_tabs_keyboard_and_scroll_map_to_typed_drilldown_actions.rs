use super::*;

#[test]
fn ux_rootfs_images_tabs_keyboard_and_scroll_map_to_typed_drilldown_actions() {
    use yoctui_model::ImagesView;

    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::Artifacts, Input::Enter),
        Some(Action::BeginSelectedRootfsComposition)
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::Artifacts, Input::Tab),
        Some(Action::ShiftImagesView { delta: 1 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::RootfsPackages, Input::Left),
        Some(Action::SelectRootfsGroup { delta: -1 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::RootfsPackages, Input::PageDown),
        Some(Action::SelectRootfsPackage { delta: 10 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::RootfsFilesystem, Input::Down),
        Some(Action::SelectRootfsEntry { delta: 1 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::RootfsFilesystem, Input::Right),
        Some(Action::BrowseRootfsFilesystem)
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::SystemdServices, Input::Char('e')),
        Some(Action::EditSelectedRootfsSystemFile)
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::SystemDbus, Input::Down),
        Some(Action::SelectRootfsDbusService { delta: 1 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::RootfsFilesystem, Input::BackTab),
        Some(Action::ShiftImagesView { delta: -1 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::Artifacts, Input::Char('3')),
        Some(Action::ShiftImagesView { delta: 2 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::RootfsFilesystem, Input::Char('1')),
        Some(Action::ShiftImagesView { delta: -2 })
    );
    assert_eq!(
        images_workspace_action_for_view(true, ImagesView::Artifacts, Input::Char('2')),
        Some(Action::AppendImageArtifactQuery('2'))
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::Artifacts, Input::Char('5')),
        Some(Action::ShiftImagesView { delta: 4 })
    );

    let mut app = App::new(10, 1_000);
    app.screen = Screen::Images;
    app.images_view = ImagesView::RootfsFilesystem;
    assert_eq!(
        workspace_collection_action(&app, Input::PageUp),
        Some(Action::SelectRootfsEntry { delta: -10 })
    );
}
