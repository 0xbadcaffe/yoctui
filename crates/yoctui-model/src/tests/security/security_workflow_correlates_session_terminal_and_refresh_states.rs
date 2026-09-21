use super::*;

#[test]
fn security_workflow_correlates_session_terminal_and_refresh_states() {
    let mut state = SecurityState::default();
    let _ = update_security(&mut state, SecurityAction::CapabilityLoaded(capability()));
    let transition = update_security(&mut state, SecurityAction::BeginCveCheck);
    let SecurityDialogUpdate::Open(SecurityDialog::Operation(preview)) = transition.dialog else {
        panic!("preview");
    };
    let id = preview.id;
    let transition = update_security(&mut state, SecurityAction::ConfirmOperation(preview));
    assert!(matches!(
        transition.effect,
        Some(SecurityEffect::StartBuild { id: started, .. }) if started == id
    ));
    let _ = update_security(&mut state, SecurityAction::SessionRunning(id));
    for index in 0..=MAX_SECURITY_SESSION_OUTPUT {
        let _ = update_security(
            &mut state,
            SecurityAction::SessionOutput {
                id,
                stream: SecurityOutputStream::Stdout,
                line: format!("mapped package {index}"),
                truncated: index == MAX_SECURITY_SESSION_OUTPUT,
            },
        );
    }
    assert_eq!(
        state.active_session().unwrap().output.len(),
        MAX_SECURITY_SESSION_OUTPUT
    );
    assert_eq!(
        state.active_session().unwrap().output[0].line,
        "mapped package 1"
    );
    let _ = update_security(&mut state, SecurityAction::BeginCancellation);
    let transition = update_security(&mut state, SecurityAction::ConfirmCancellation(id));
    assert_eq!(transition.effect, Some(SecurityEffect::CancelSession(id)));
    let _ = update_security(
        &mut state,
        SecurityAction::RejectCancellation {
            id,
            message: "busy".into(),
        },
    );
    assert_eq!(
        state.active_session().unwrap().status,
        SecuritySessionStatus::Running
    );
    let transition = update_security(
        &mut state,
        SecurityAction::CompleteSession {
            id,
            result_paths: vec!["/reports/cve.json".into()],
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert!(matches!(
        transition.effect,
        Some(SecurityEffect::ImportReports(_))
    ));
    assert_eq!(state.sessions[0].status, SecuritySessionStatus::Succeeded);
    let request = state.inventory.request().unwrap().clone();
    let _ = update_security(&mut state, SecurityAction::ReportsTimedOut(request));
    assert!(matches!(
        state.inventory,
        SecurityInventoryState::TimedOut { .. }
    ));
}
