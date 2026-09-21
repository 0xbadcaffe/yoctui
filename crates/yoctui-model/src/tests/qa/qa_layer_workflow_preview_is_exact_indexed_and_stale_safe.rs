use super::*;

#[test]
fn qa_layer_workflow_preview_is_exact_indexed_and_stale_safe() {
    let mut state = QaState::default();
    load_layer(&mut state);
    let preview = begin_layer(&mut state);
    assert_eq!(
        preview.indexed_arguments,
        [
            "0: /poky/scripts/yocto-check-layer",
            "1: --layer",
            "2: /layers/meta"
        ]
    );
    let mut stale_layer = preview.clone();
    stale_layer.layer = layer("meta-custom");
    let transition = update_qa(&mut state, QaAction::ConfirmLayerOperation(stale_layer));
    assert!(transition.effect.is_none());
    assert!(state.layer_sessions.is_empty());
    let mut stale_tool = preview.clone();
    stale_tool.executable.byte_size += 1;
    let transition = update_qa(&mut state, QaAction::ConfirmLayerOperation(stale_tool));
    assert!(transition.effect.is_none());
    assert!(state.layer_sessions.is_empty());

    let transition = update_qa(&mut state, QaAction::ConfirmLayerOperation(preview.clone()));
    assert!(matches!(
        transition.effect,
        Some(QaEffect::StartLayerCheck {
            layer: QaLayerIdentity { ref name, .. },
            ref executable,
            ref arguments,
            ..
        }) if name == "meta"
            && executable == &layer_executable()
            && arguments == &["--layer", "/layers/meta"]
    ));
    assert!(matches!(transition.dialog, QaDialogUpdate::Close));
    assert!(
        update_qa(&mut state, QaAction::ConfirmLayerOperation(preview))
            .effect
            .is_none()
    );
}
