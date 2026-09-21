use super::*;

#[test]
fn qa_workflow_maps_workspace_search_and_drill_keys_without_leakage() {
    assert_eq!(
        qa_workspace_action(QaView::RecipeKernel, false, false, Input::Char('r')),
        Some(Action::Qa(QaAction::BeginSelectedCheck))
    );
    assert_eq!(
        qa_workspace_action(QaView::RecipeKernel, false, false, Input::Down),
        Some(Action::Qa(QaAction::SelectCheck(1)))
    );
    assert_eq!(
        qa_workspace_action(QaView::RecipeKernel, true, false, Input::Down),
        Some(Action::Qa(QaAction::SelectFinding(1)))
    );
    assert_eq!(
        qa_workspace_action(QaView::RecipeKernel, false, true, Input::Char('r')),
        Some(Action::Qa(QaAction::AppendQuery('r'))),
        "search editing consumes QA run shortcuts"
    );
    assert_eq!(
        qa_workspace_action(QaView::RecipeKernel, true, false, Input::Esc),
        Some(Action::Qa(QaAction::LeaveDrill))
    );
    assert_eq!(
        qa_workspace_action(QaView::RecipeKernel, false, false, Input::Char('l')),
        Some(Action::Qa(QaAction::OpenSelectedSource))
    );
    assert_eq!(
        qa_workspace_action(QaView::LayerQa, false, false, Input::Tab),
        Some(Action::Qa(QaAction::CycleView))
    );
    assert_eq!(
        qa_workspace_action(QaView::LayerQa, false, false, Input::Down),
        Some(Action::Qa(QaAction::SelectLayer(1)))
    );
    assert_eq!(
        qa_workspace_action(QaView::LayerQa, false, false, Input::Char('r')),
        Some(Action::Qa(QaAction::BeginSelectedLayerCheck))
    );
    assert_eq!(
        qa_workspace_action(QaView::LayerQa, false, false, Input::Char('c')),
        Some(Action::Qa(QaAction::BeginLayerCancellation))
    );
    assert_eq!(
        qa_workspace_action(QaView::LayerQa, false, false, Input::Char('e')),
        Some(Action::Qa(QaAction::OpenSelectedLayerRoot))
    );
}
