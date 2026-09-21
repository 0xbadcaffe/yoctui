use super::*;

#[test]
fn qa_check_workflow_scope_cycle_preserves_exact_provider_identity() {
    let mut state = QaState::default();
    load(&mut state);
    let first = state.scope.clone().unwrap();
    let _ = update_qa(&mut state, QaAction::CycleScope);
    assert_ne!(state.scope.as_ref(), Some(&first));
    assert_eq!(
        state.scope.as_ref().unwrap().recipe.file,
        PathBuf::from("/layers/meta/recipes/busybox/busybox_1.0.bb")
    );
    assert_eq!(state.visible_checks().len(), 2);
}
