use super::*;

#[test]
fn maintenance_optional_bounds_candidates_and_process_records() {
    let fixture = TestDirectory::new("bounds");
    let mut input = complete_fixture(&fixture);
    input.git_worktree_candidates = vec![fixture.join("missing"); MAX_MAINTENANCE_PATHS + 1];
    for pid in 1..=(MAX_MAINTENANCE_OUTPUT as u32 + 1) {
        process(&fixture.join("proc"), pid + 100, "toaster", b"toaster\0");
    }
    let inspection = MaintenanceOptionalCapabilityInspector::inspect(input).unwrap();
    assert_eq!(
        inspection.toaster.observed_processes.len(),
        MAX_MAINTENANCE_OUTPUT
    );
    assert!(
        inspection
            .limitations
            .iter()
            .any(|value| value.contains("Git worktree candidates reached"))
    );
}
