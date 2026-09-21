use super::*;

#[test]
fn ux_dependency_graph_maps_typed_navigation_filter_topology_and_open_actions() {
    assert_eq!(
        dependency_workspace_action(false, Input::Up),
        Some(Action::SelectDependencyGraphNode { delta: -1 })
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Char('j')),
        Some(Action::SelectDependencyGraphNode { delta: 1 })
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Enter),
        Some(Action::OpenSelectedDependencyRecipe)
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Char('o')),
        Some(Action::OpenSelectedDependencyProvider)
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Char('L')),
        Some(Action::OpenSelectedDependencyTaskLog)
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Char('r')),
        Some(Action::RefreshDependencyGraph)
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Char('v')),
        Some(Action::ToggleDependencyGraphReverse)
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Left),
        Some(Action::CollapseSelectedDependencyGraphNode)
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Right),
        Some(Action::ExpandSelectedDependencyGraphNode)
    );
    assert_eq!(
        dependency_workspace_action(false, Input::Char('/')),
        Some(Action::BeginDependencyGraphSearch)
    );
    assert_eq!(
        dependency_workspace_action(true, Input::Char('b')),
        Some(Action::AppendDependencyGraphQuery('b'))
    );
    assert_eq!(
        dependency_workspace_action(true, Input::Esc),
        Some(Action::FinishDependencyGraphSearch)
    );
    assert_eq!(dependency_workspace_action(false, Input::Char('x')), None);
}
