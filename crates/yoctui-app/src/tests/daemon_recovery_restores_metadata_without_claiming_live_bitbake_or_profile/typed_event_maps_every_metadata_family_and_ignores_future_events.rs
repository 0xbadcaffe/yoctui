use super::*;

#[test]
fn typed_event_maps_every_metadata_family_and_ignores_future_events() {
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::Recipes(vec![])),
        Some(Action::RecipesLoaded(_))
    ));
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::Layers(vec![])),
        Some(Action::LayersLoaded(_))
    ));
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::Variable {
            name: "MACHINE".into(),
            recipe: None,
            value: Some("qemux86-64".into()),
            provenance: Some("conf/local.conf:1".into()),
            unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
            operations: vec![],
            active_overrides: vec![],
        }),
        Some(Action::VariableLoaded(_))
    ));
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::Dependencies {
            recipe: "busybox".into(),
            build: vec![],
            runtime: vec![],
        }),
        Some(Action::DependenciesLoaded(_))
    ));
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::RecipeSources {
            recipe: "busybox".into(),
            paths: vec!["/workspace/busybox".into()],
        }),
        Some(Action::RecipeSourcesLoaded { .. })
    ));
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::LayerRelationships(vec![])),
        Some(Action::LayerRelationshipsLoaded(_))
    ));
    assert_eq!(model_action_from_backend_event(BackendEvent::Ignored), None);
}
