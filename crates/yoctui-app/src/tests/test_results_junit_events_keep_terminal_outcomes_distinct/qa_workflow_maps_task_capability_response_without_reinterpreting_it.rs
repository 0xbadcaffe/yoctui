use super::*;

#[test]
fn qa_workflow_maps_task_capability_response_without_reinterpreting_it() {
    let scope = yoctui_model::QaScope::new(yoctui_model::RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
    })
    .unwrap();
    let snapshot = yoctui_model::QaCapabilitySnapshot::new(
        Some("6.0".into()),
        "/build".into(),
        scope.clone(),
        vec![scope],
        vec![],
        vec!["one optional report root was unsafe".into()],
    )
    .unwrap();
    assert_eq!(
        qa_task_capability_action(QaTaskCapabilityResponse::Available(snapshot.clone())),
        Action::Qa(QaAction::CapabilityLoaded(snapshot.clone()))
    );
    assert_eq!(
        qa_task_capability_action(QaTaskCapabilityResponse::Partial(snapshot.clone())),
        Action::Qa(QaAction::CapabilityPartial {
            snapshot,
            limitations: vec!["one optional report root was unsafe".into()],
        })
    );
}
