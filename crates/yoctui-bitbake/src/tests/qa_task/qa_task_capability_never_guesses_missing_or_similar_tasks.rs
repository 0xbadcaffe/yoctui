use super::*;

#[test]
fn qa_task_capability_never_guesses_missing_or_similar_tasks() {
    let fixture = Fixture::new();
    let mut input = input(&fixture);
    let kernel = &mut input.scopes[0];
    kernel.reported_tasks.retain(|task| task != "do_checkuri");
    kernel.reported_tasks.push("do_checkuris".into());
    let snapshot = QaTaskCapabilityInspector::new(input)
        .inspect()
        .unwrap()
        .into_snapshot();
    let uri = snapshot
        .checks
        .iter()
        .find(|check| {
            check.scope.recipe.name == "linux-yocto" && check.family == QaCheckFamily::UriFetch
        })
        .unwrap();
    assert!(uri.task.is_none());
    assert_eq!(
        uri.availability.disabled_reason(),
        Some("the bound task is not reported for the exact recipe scope")
    );
}
