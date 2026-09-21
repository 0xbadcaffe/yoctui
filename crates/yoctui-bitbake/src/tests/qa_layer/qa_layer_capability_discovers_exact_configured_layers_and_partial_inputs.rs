use super::*;

#[test]
fn qa_layer_capability_discovers_exact_configured_layers_and_partial_inputs() {
    let (root, snapshot) = fixture("capability", "#!/bin/sh\nexit 0\n");
    assert_eq!(snapshot.layers.len(), 1);
    assert!(matches!(
        snapshot.layers[0].run,
        QaLayerRunCapability::Available { .. }
    ));
    let missing = root.0.join("missing-bin");
    fs::create_dir(&missing).unwrap();
    let identity = snapshot.selected_layer.clone();
    let response = QaLayerCapabilityInspector::inspect(QaLayerCapabilityInput {
        release: None,
        build_directory: root.0.clone(),
        selected_layer: identity.clone(),
        layers: vec![QaConfiguredLayerInput {
            check: QaCheckId::new("layer-meta-demo".into()).unwrap(),
            identity,
            compatible_series: Vec::new(),
            report_roots: Vec::new(),
        }],
        executable_search_path: vec![missing],
    })
    .unwrap();
    assert!(matches!(response, QaLayerCapabilityResponse::Partial(_)));
}
