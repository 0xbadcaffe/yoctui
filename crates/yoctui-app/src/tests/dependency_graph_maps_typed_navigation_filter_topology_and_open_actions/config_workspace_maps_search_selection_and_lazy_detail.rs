use super::*;

#[test]
fn config_workspace_maps_search_selection_and_lazy_detail() {
    assert_eq!(
        config_workspace_action(false, Input::Down),
        Some(Action::SelectConfigVariable { delta: 1 })
    );
    assert_eq!(
        config_workspace_action(false, Input::Enter),
        Some(Action::BeginSelectedConfigDetail)
    );
    assert_eq!(
        config_workspace_action(false, Input::Char('/')),
        Some(Action::BeginMetadataSearch)
    );
    assert_eq!(
        config_workspace_action(true, Input::Char('M')),
        Some(Action::AppendMetadataQuery('M'))
    );
}
