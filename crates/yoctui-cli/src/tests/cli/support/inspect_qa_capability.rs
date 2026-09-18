use super::*;

pub(crate) async fn inspect_qa_capability(
    fixture: &QaCliFixture,
    app: &mut App,
    coordinator: &mut QaCliCoordinator,
) {
    let effect = update(app, Action::Qa(QaAction::InspectCapability)).unwrap();
    assert!(coordinator.handle_effect(app, effect).await);
    poll_qa_until(coordinator, app, |app, _| {
        app.qa.capability.snapshot().is_some()
    })
    .await;
    assert_eq!(
        app.qa
            .selected_check()
            .and_then(|check| check.task.as_deref()),
        None,
        "the first kernel-only check stays disabled for a non-kernel recipe"
    );
    assert_eq!(app.qa.scope.as_ref().unwrap().recipe.file, fixture.provider);
}
