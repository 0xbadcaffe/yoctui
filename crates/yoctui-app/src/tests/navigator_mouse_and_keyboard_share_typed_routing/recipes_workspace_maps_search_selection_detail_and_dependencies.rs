use super::*;

#[test]
fn recipes_workspace_maps_search_selection_detail_and_dependencies() {
    assert_eq!(
        recipes_workspace_action(false, Input::Down),
        Some(Action::SelectRecipe { delta: 1 })
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Enter),
        Some(Action::BeginSelectedRecipeMetadata)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('A')),
        Some(Action::BeginSelectedRecipeDependencies)
    );
    assert_eq!(
        recipes_workspace_action(true, Input::Char('b')),
        Some(Action::AppendMetadataQuery('b'))
    );
    assert_eq!(
        recipes_workspace_action(true, Input::Backspace),
        Some(Action::BackspaceMetadataQuery)
    );
}
