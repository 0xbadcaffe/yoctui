use super::*;

#[test]
fn qa_task_capability_uses_only_exact_family_bindings_and_reported_tasks() {
    let fixture = Fixture::new();
    let response = QaTaskCapabilityInspector::new(input(&fixture))
        .inspect()
        .unwrap();
    assert!(!response.is_partial());
    let snapshot = response.snapshot();
    assert_eq!(snapshot.scopes.len(), 2);
    assert_eq!(snapshot.checks.len(), 10);
    let kernel = snapshot
        .checks
        .iter()
        .find(|check| {
            check.scope.recipe.name == "linux-yocto"
                && check.family == QaCheckFamily::KernelConfiguration
        })
        .unwrap();
    assert_eq!(kernel.task.as_deref(), Some("do_kernel_configcheck"));
    assert!(matches!(
        kernel.availability,
        QaCheckAvailability::Available
    ));
    assert_eq!(kernel.report_roots.len(), 1);
    let request = BuildRequest {
        targets: vec![kernel.scope.recipe.name.clone()],
        task: kernel.task.clone(),
        force: false,
    };
    request.validate().unwrap();
    assert_eq!(request.task.as_deref(), Some("do_kernel_configcheck"));

    let non_kernel = snapshot
        .checks
        .iter()
        .find(|check| {
            check.scope.recipe.name == "busybox"
                && check.family == QaCheckFamily::KernelConfiguration
        })
        .unwrap();
    assert!(non_kernel.task.is_none());
    assert_eq!(
        non_kernel.availability.disabled_reason(),
        Some("kernel configuration checks require authoritative kernel classification")
    );
}
