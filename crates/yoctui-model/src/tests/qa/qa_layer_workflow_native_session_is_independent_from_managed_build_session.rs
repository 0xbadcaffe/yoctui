use super::*;

#[test]
fn qa_layer_workflow_native_session_is_independent_from_managed_build_session() {
    let mut state = QaState::default();
    load(&mut state);
    let recipe_session = start(&mut state);
    load_layer(&mut state);
    let layer_session = start_layer(&mut state);
    assert_eq!(state.active_session().unwrap().id, recipe_session);
    assert_eq!(state.active_layer_session().unwrap().id, layer_session);
    assert!(matches!(
        update_qa(&mut state, QaAction::BeginLayerCancellation).dialog,
        QaDialogUpdate::Open(dialog)
            if matches!(*dialog, QaDialog::LayerCancellation(id) if id == layer_session)
    ));
    assert_eq!(
        update_qa(
            &mut state,
            QaAction::AttachBackgroundJob {
                session: recipe_session,
                background_job: BackgroundJobId(77),
            }
        )
        .effect,
        None
    );
    assert!(matches!(
        update_qa(&mut state, QaAction::BeginCancellation).dialog,
        QaDialogUpdate::Open(dialog)
            if matches!(
                *dialog,
                QaDialog::Cancellation {
                    session,
                    background_job
                } if session == recipe_session && background_job == BackgroundJobId(77)
            )
    ));
}
