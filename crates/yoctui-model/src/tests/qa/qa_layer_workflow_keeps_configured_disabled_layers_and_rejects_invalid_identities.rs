use super::*;

#[test]
fn qa_layer_workflow_keeps_configured_disabled_layers_and_rejects_invalid_identities() {
    assert!(QaLayerIdentity::new("meta".into(), "../meta".into()).is_err());
    let identity = layer("meta");
    assert!(
        QaConfiguredLayerCapability::new(
            QaCheckId::new("yocto-check-layer".into()).unwrap(),
            identity,
            vec![],
            QaLayerRunCapability::Available {
                executable: layer_executable(),
                arguments: vec!["--layer".into(), "/arbitrary/not-configured".into()],
                report_roots: vec![],
            },
            vec![],
        )
        .is_err()
    );

    let mut state = QaState::default();
    assert!(matches!(
        state.layer_capability,
        QaLayerCapability::NotInspected
    ));
    assert_eq!(
        update_qa(&mut state, QaAction::CycleView).effect,
        Some(QaEffect::InspectLayerCapability)
    );
    assert_eq!(state.view, QaView::LayerQa);
    assert!(matches!(
        state.layer_capability,
        QaLayerCapability::Inspecting
    ));
    load_layer(&mut state);
    assert_eq!(state.visible_layers().len(), 2);
    let _ = update_qa(&mut state, QaAction::SelectLayer(1));
    let transition = update_qa(&mut state, QaAction::BeginSelectedLayerCheck);
    assert_eq!(
        transition.notification.as_deref(),
        Some("yocto-check-layer is unavailable")
    );

    let mut partial = QaState::default();
    let _ = update_qa(
        &mut partial,
        QaAction::LayerCapabilityPartial {
            snapshot: layer_capability(),
            limitations: vec!["one compatibility series was unavailable".into()],
        },
    );
    assert!(matches!(
        partial.layer_capability,
        QaLayerCapability::Partial { .. }
    ));
    let _ = update_qa(
        &mut partial,
        QaAction::LayerCapabilityFailed("inspection failed".into()),
    );
    assert!(matches!(
        partial.layer_capability,
        QaLayerCapability::Failed(_)
    ));
}
