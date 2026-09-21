use super::*;

#[test]
fn qa_task_capability_accepts_alternate_tasks_only_as_explicit_data() {
    let fixture = Fixture::new();
    let mut input = input(&fixture);
    let kernel = &mut input.scopes[0];
    kernel.reported_tasks.push("do_vendor_uri_audit".into());
    kernel
        .family_tasks
        .retain(|binding| binding.family != QaCheckFamily::UriFetch);
    kernel
        .family_tasks
        .push(binding(QaCheckFamily::UriFetch, "do_vendor_uri_audit"));
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
    assert_eq!(uri.task.as_deref(), Some("do_vendor_uri_audit"));
}
