use super::*;

#[test]
fn completed_progress_reducer_keeps_existing_completion_counter_contract() {
    for total in [None, Some(0), Some(7)] {
        for observed in [0, 3] {
            let mut app = App::new(8, 1024);
            app.build.status = BuildStatus::Running;
            app.build.total = total;
            app.build.completed = observed;
            assert!(app.build.completed_progress().is_none());
            update(
                &mut app,
                Action::BuildCompleted {
                    success: true,
                    exit_code: Some(0),
                },
            );
            // Existing successful completion finalizes a reported total;
            // missing totals retain observations. This UI fix does not change it.
            let observed = total.unwrap_or(observed);
            assert_eq!(app.build.completed, observed);
            assert_eq!(app.build.total, total);
            assert_eq!(app.build_history.back().unwrap().completed_tasks, observed);
            let expected = match total {
                None => Some(CompletedBuildProgress::UnknownTotal { observed }),
                Some(0) => Some(CompletedBuildProgress::ZeroTotal { observed }),
                Some(_) => None,
            };
            assert_eq!(app.build.completed_progress(), expected);
            assert!(app.completed_tasks.is_empty());
        }
    }
}

#[test]
fn completed_progress_never_relabels_running_failed_cancelled_lost_or_inconsistent_exit() {
    let mut app = App::new(8, 1024);
    for status in [
        BuildStatus::Idle,
        BuildStatus::Parsing,
        BuildStatus::Running,
        BuildStatus::Cancelling,
        BuildStatus::Cancelled,
        BuildStatus::Failed,
        BuildStatus::Lost,
    ] {
        app.build.status = status;
        app.build.exit_code = Some(0);
        assert!(app.build.completed_progress().is_none(), "{status:?}");
    }
    app.build.status = BuildStatus::Completed;
    app.build.exit_code = Some(1);
    assert!(app.build.completed_progress().is_none());
    app.build.exit_code = None;
    assert_eq!(
        app.build.completed_progress(),
        Some(CompletedBuildProgress::UnknownTotal { observed: 0 })
    );
    update(
        &mut app,
        Action::BuildCompleted {
            success: false,
            exit_code: Some(2),
        },
    );
    assert!(app.build.completed_progress().is_none());
}
