use super::*;

#[test]
fn maintenance_workflow_covers_success_failure_timeout_cancel_and_loss() {
    let terminal = [
        MaintenanceSessionStatus::Succeeded,
        MaintenanceSessionStatus::Failed,
        MaintenanceSessionStatus::TimedOut,
        MaintenanceSessionStatus::Cancelled,
        MaintenanceSessionStatus::Lost,
    ];
    for (index, expected) in terminal.into_iter().enumerate() {
        let mut state = ready_state();
        let preview = readiness_preview(index as u64 + 1);
        update_maintenance(
            &mut state,
            MaintenanceAction::BeginOperation(preview.clone()),
        );
        let transition = update_maintenance(
            &mut state,
            MaintenanceAction::ConfirmOperation(preview.clone()),
        );
        let id = MaintenanceSessionId(preview.id);
        assert!(matches!(
            transition.effect,
            Some(MaintenanceEffect::StartOperation { .. })
        ));
        update_maintenance(
            &mut state,
            MaintenanceAction::SessionRunning {
                id,
                started_at: UNIX_EPOCH,
            },
        );
        match expected {
            MaintenanceSessionStatus::Succeeded => {
                update_maintenance(
                    &mut state,
                    MaintenanceAction::CompleteSession {
                        id,
                        exit_code: 0,
                        evidence: vec![
                            MaintenanceEvidence::new(identity("/reports/result"), "result".into())
                                .unwrap(),
                        ],
                        finished_at: UNIX_EPOCH + Duration::from_secs(1),
                    },
                );
            }
            MaintenanceSessionStatus::Failed => {
                update_maintenance(
                    &mut state,
                    MaintenanceAction::FailSession {
                        id,
                        message: "failed".into(),
                        exit_code: Some(2),
                        finished_at: UNIX_EPOCH,
                    },
                );
            }
            MaintenanceSessionStatus::TimedOut => {
                update_maintenance(
                    &mut state,
                    MaintenanceAction::TimeoutSession {
                        id,
                        finished_at: UNIX_EPOCH,
                    },
                );
            }
            MaintenanceSessionStatus::Cancelled => {
                update_maintenance(
                    &mut state,
                    MaintenanceAction::CancelSession {
                        id,
                        finished_at: UNIX_EPOCH,
                    },
                );
            }
            MaintenanceSessionStatus::Lost => {
                update_maintenance(
                    &mut state,
                    MaintenanceAction::LoseSession {
                        id,
                        message: "runner lost".into(),
                        finished_at: UNIX_EPOCH,
                    },
                );
            }
            _ => unreachable!(),
        }
        assert_eq!(state.sessions.back().unwrap().status, expected);
    }
}
