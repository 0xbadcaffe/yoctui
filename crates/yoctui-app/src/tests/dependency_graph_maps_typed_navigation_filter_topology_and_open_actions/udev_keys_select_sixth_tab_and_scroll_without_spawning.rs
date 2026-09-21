use super::*;

#[test]
fn udev_keys_select_sixth_tab_and_scroll_without_spawning() {
    use yoctui_model::ImagesView;
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::Artifacts, Input::Char('6')),
        Some(Action::ShiftImagesView { delta: 5 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::UdevRules, Input::End),
        Some(Action::SelectRootfsUdevRule { delta: isize::MAX })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::UdevRules, Input::PageDown),
        Some(Action::SelectRootfsUdevRule { delta: 10 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::UdevRules, Input::Char(']')),
        Some(Action::ScrollRootfsUdevPreview { delta: 1 })
    );
    assert_eq!(
        images_workspace_action_for_view(false, ImagesView::UdevRules, Input::Char('e')),
        None
    );
}
