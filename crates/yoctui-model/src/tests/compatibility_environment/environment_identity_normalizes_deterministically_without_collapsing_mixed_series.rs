use super::*;

#[test]
fn environment_identity_normalizes_deterministically_without_collapsing_mixed_series() {
    let normalized = full_identity().normalize().unwrap();
    let roots = normalized.source_roots.value().unwrap();
    assert_eq!(roots[0].kind, SourceRootKind::CoreBase);
    let layers = normalized.layer_series.value().unwrap();
    assert_eq!(layers[0].layer, "core");
    assert_eq!(
        layers[0].compatible_series,
        ["nanbield".to_string(), "scarthgap".to_string()]
    );
    assert_ne!(layers[0].compatible_series, layers[1].compatible_series);
}
