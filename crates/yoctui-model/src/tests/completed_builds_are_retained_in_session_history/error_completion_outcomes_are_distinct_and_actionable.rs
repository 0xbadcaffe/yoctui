use super::*;

#[test]
fn error_completion_outcomes_are_distinct_and_actionable() {
    let mut success = App::new(10, 1_000);
    let _ = update(
        &mut success,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert!(
        success
            .notification
            .as_deref()
            .is_some_and(|message| message.contains("successfully"))
    );

    let mut warning = App::new(10, 1_000);
    let _ = update(
        &mut warning,
        Action::Log(tagged_log(
            "busybox",
            "do_compile",
            Severity::Warning,
            "deprecated option",
        )),
    );
    let _ = update(
        &mut warning,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert!(
        warning
            .notification
            .as_deref()
            .is_some_and(|message| message.contains("warning"))
    );

    let mut failed = App::new(10, 1_000);
    let _ = update(
        &mut failed,
        Action::Log(tagged_log(
            "busybox",
            "do_compile",
            Severity::Error,
            "compile failed",
        )),
    );
    let _ = update(
        &mut failed,
        Action::BuildCompleted {
            success: false,
            exit_code: Some(1),
        },
    );
    assert!(
        failed
            .notification
            .as_deref()
            .is_some_and(|message| message.contains("Press Enter"))
    );
    let _ = update(&mut failed, Action::OpenBuildCompletionErrors);
    assert_eq!(failed.screen, Screen::Errors);
    assert!(failed.active_dialog().is_none());

    let mut cancelled = App::new(10, 1_000);
    let _ = update(
        &mut cancelled,
        Action::BuildCancelled {
            exit_code: Some(130),
        },
    );
    assert!(
        cancelled
            .notification
            .as_deref()
            .is_some_and(|message| message.contains("distinct"))
    );
}
