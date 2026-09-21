use super::*;

#[test]
fn qa_check_workflow_preview_is_indexed_exact_and_stale_safe() {
    let mut state = QaState::default();
    load(&mut state);
    let preview = begin(&mut state);
    assert_eq!(
        preview.indexed_arguments,
        [
            "0: bitbake",
            "1: linux-yocto",
            "2: -c",
            "3: kernel_configcheck"
        ]
    );
    let mut stale = preview.clone();
    stale.request.task = Some("guessed_task".into());
    let transition = update_qa(&mut state, QaAction::ConfirmOperation(stale));
    assert!(transition.effect.is_none());
    assert!(state.sessions.is_empty());

    let transition = update_qa(&mut state, QaAction::ConfirmOperation(preview.clone()));
    assert!(matches!(
        transition.effect,
        Some(QaEffect::StartBuild {
            request: BuildRequest {
                ref targets,
                task: Some(ref task),
                force: false,
            },
            ..
        }) if targets == &["linux-yocto"] && task == "kernel_configcheck"
    ));
    assert!(matches!(transition.dialog, QaDialogUpdate::Close));
    assert_eq!(state.sessions.len(), 1);

    let duplicate = update_qa(&mut state, QaAction::ConfirmOperation(preview));
    assert!(duplicate.effect.is_none());
}
