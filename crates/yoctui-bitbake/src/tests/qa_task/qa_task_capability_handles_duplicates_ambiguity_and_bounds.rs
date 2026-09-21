use super::*;

#[test]
fn qa_task_capability_handles_duplicates_ambiguity_and_bounds() {
    let fixture = Fixture::new();
    let mut capability_input = input(&fixture);
    capability_input
        .scopes
        .push(capability_input.scopes[1].clone());
    capability_input.scopes[0]
        .family_tasks
        .push(binding(QaCheckFamily::License, "do_license_alt"));
    capability_input.scopes[0]
        .reported_tasks
        .push("do_license_alt".into());
    let response = QaTaskCapabilityInspector::new(capability_input)
        .inspect()
        .unwrap();
    assert!(response.is_partial());
    let license = response
        .snapshot()
        .checks
        .iter()
        .find(|check| {
            check.scope.recipe.name == "linux-yocto" && check.family == QaCheckFamily::License
        })
        .unwrap();
    assert!(license.task.is_none());
    assert_eq!(
        license.availability.disabled_reason(),
        Some("multiple authoritative tasks are bound to this QA family")
    );
    assert!(
        response
            .snapshot()
            .limitations
            .iter()
            .any(|value| { value.contains("duplicate QA provider scope") })
    );

    let mut oversized = input(&fixture);
    oversized.scopes[0].reported_tasks = (0..=MAX_QA_TASK_INPUTS)
        .map(|index| format!("task{index}"))
        .collect();
    assert_eq!(
        QaTaskCapabilityInspector::new(oversized).inspect(),
        Err(QaTaskCapabilityError::TooManyInputs)
    );
}
