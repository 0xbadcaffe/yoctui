use super::*;

#[test]
fn ux_dependency_graph_mouse_click_selects_exact_projected_identity() {
    let root = DependencyNodeId::recipe("root");
    let child = DependencyNodeId::recipe("child");
    let (graph, _) = DependencyGraph::normalize(
        root.clone(),
        Vec::new(),
        vec![DependencyEdge {
            from: root,
            to: child.clone(),
            kind: DependencyEdgeKind::Build,
        }],
        10,
        10,
    );
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Dependencies;
    app.focus = FocusTarget::Workspace;
    app.dependency_graph = DependencyGraphState::Available(graph);
    let action = mouse_action_for_app(
        MouseInput {
            kind: MouseKind::Down,
            column: 24,
            row: 5,
        },
        &app,
        160,
        48,
    );
    assert_eq!(
        action,
        Some(Action::SelectDependencyGraphNodeAt { identity: child })
    );
}
