use super::*;

#[test]
fn security_capability_is_partial_for_unsafe_optional_inputs() {
    let (directory, mut input) = fixture(&["do_cve_check"]);
    input.cve_roots.push(directory.path().join("missing"));
    input
        .path_directories
        .push(directory.path().join("missing-bin"));
    input.reported_tasks.push("do_unrelated".into());
    let snapshot = SecurityCapabilityInspector::new(input).inspect().unwrap();
    assert_eq!(snapshot.cve_task.as_deref(), Some("cve_check"));
    assert!(snapshot.recipe_sbom_task.is_none());
    assert_eq!(snapshot.limitations.len(), 2);
}
