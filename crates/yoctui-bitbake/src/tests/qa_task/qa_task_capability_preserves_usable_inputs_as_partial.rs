use super::*;

#[test]
fn qa_task_capability_preserves_usable_inputs_as_partial() {
    let fixture = Fixture::new();
    let mut input = input(&fixture);
    let unsafe_provider = fixture.root.join("missing.bb");
    input.scopes.push(scope_input(
        identity("optional", unsafe_provider),
        false,
        fixture.root.join("missing-reports"),
    ));
    input.scopes[0].report_roots.push(QaReportRootInput {
        family: QaCheckFamily::License,
        path: fixture.directory("outside-license-reports"),
    });
    input.scopes[0].reported_tasks.push("bad/task".into());
    let response = QaTaskCapabilityInspector::new(input).inspect().unwrap();
    assert!(response.is_partial());
    assert_eq!(response.snapshot().scopes.len(), 2);
    assert!(
        response
            .snapshot()
            .limitations
            .iter()
            .any(|value| { value.contains("unsafe QA provider scope") })
    );
    assert!(
        response
            .snapshot()
            .limitations
            .iter()
            .any(|value| { value.contains("unsafe License QA report root") })
    );
}
