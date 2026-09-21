use super::*;

#[test]
fn maintenance_workflow_bounds_output_and_replaces_only_successful_evidence() {
    let mut state = ready_state();
    state.evidence =
        vec![MaintenanceEvidence::new(identity("/reports/old"), "old".into()).unwrap()];
    let preview = readiness_preview(1);
    update_maintenance(
        &mut state,
        MaintenanceAction::BeginOperation(preview.clone()),
    );
    update_maintenance(&mut state, MaintenanceAction::ConfirmOperation(preview));
    let id = MaintenanceSessionId(1);
    for index in 0..MAX_MAINTENANCE_OUTPUT + 10 {
        update_maintenance(
            &mut state,
            MaintenanceAction::SessionOutput {
                id,
                stream: MaintenanceOutputStream::Stdout,
                text: format!("line {index}"),
            },
        );
    }
    assert_eq!(
        state.sessions.back().unwrap().output.len(),
        MAX_MAINTENANCE_OUTPUT
    );
    assert_eq!(state.sessions.back().unwrap().dropped_lines, 10);
    update_maintenance(
        &mut state,
        MaintenanceAction::FailSession {
            id,
            message: "failed".into(),
            exit_code: Some(1),
            finished_at: UNIX_EPOCH,
        },
    );
    assert_eq!(state.evidence[0].identity.path, Path::new("/reports/old"));
}
