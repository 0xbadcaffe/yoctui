use super::*;

#[test]
fn pkgdata_workspace_maps_search_navigation_refresh_and_context_actions() {
    assert_eq!(
        package_workspace_action(false, Input::Up),
        Some(Action::SelectPackage { delta: -1 })
    );
    assert_eq!(
        package_workspace_action(false, Input::Enter),
        Some(Action::BeginSelectedPackageDetail)
    );
    assert_eq!(
        package_workspace_action(false, Input::Char('R')),
        Some(Action::RefreshPackageInventory)
    );
    assert_eq!(
        package_workspace_action(false, Input::Char('D')),
        Some(Action::TogglePackageDependencyKind)
    );
    assert_eq!(
        package_workspace_action(false, Input::Char(']')),
        Some(Action::SelectPackageDependency { delta: 1 })
    );
    assert_eq!(
        package_workspace_action(false, Input::Char('d')),
        Some(Action::OpenSelectedPackageDependency)
    );
    assert_eq!(
        package_workspace_action(false, Input::Char('o')),
        Some(Action::OpenSelectedPackageRecipe)
    );
    assert_eq!(
        package_workspace_action(false, Input::Char('e')),
        Some(Action::OpenSelectedPackageProvider)
    );
    assert_eq!(
        package_workspace_action(false, Input::Char('c')),
        Some(Action::CancelPackageOperation)
    );
    assert_eq!(
        package_workspace_action(false, Input::Char('/')),
        Some(Action::BeginPackageSearch)
    );
    assert_eq!(
        package_workspace_action(true, Input::Char('b')),
        Some(Action::AppendPackageQuery('b'))
    );
    assert_eq!(
        package_workspace_action(true, Input::Backspace),
        Some(Action::BackspacePackageQuery)
    );
    assert_eq!(
        package_workspace_action(true, Input::Esc),
        Some(Action::FinishPackageSearch)
    );
    assert_eq!(package_workspace_action(false, Input::Char('x')), None);
}
