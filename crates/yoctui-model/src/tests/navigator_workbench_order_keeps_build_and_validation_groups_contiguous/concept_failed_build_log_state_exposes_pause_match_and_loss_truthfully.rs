use super::*;

#[test]
fn concept_failed_build_log_state_exposes_pause_match_and_loss_truthfully() {
    let mut logs = LogState::new(2, 1_000);
    logs.insert(tagged_log(
        "bash_5.2.21-2",
        "do_compile",
        Severity::Warning,
        "WARNING: bash:do_compile mismatch",
    ));
    logs.insert(tagged_log(
        "bash_5.2.21-2",
        "do_compile",
        Severity::Error,
        "ERROR: bash:do_compile failed",
    ));
    logs.insert(tagged_log(
        "bash_5.2.21-2",
        "do_compile",
        Severity::Error,
        "ERROR: bash:do_compile exited 1",
    ));

    logs.follow = false;
    logs.paused_len = Some(logs.entries.len());
    logs.query = "do_compile".into();
    logs.selection = logs.visible_count().saturating_sub(1);

    assert_eq!(logs.dropped, 1);
    assert_eq!(logs.dropped_warnings, 1);
    assert_eq!(logs.dropped_errors, 0);
    assert_eq!(logs.diagnostics().count(), 2);
    assert_eq!(logs.visible_count(), 2);
    assert_eq!(logs.match_position(), Some((2, 2)));
    assert_eq!(logs.paused_len, Some(2));
    assert_eq!(
        logs.selected().and_then(|entry| entry.task.as_deref()),
        Some("do_compile")
    );
}
