use super::*;

#[test]
fn snapshot_timing_invalid_reversed_and_missing_times_are_unavailable() {
    use std::time::{Duration, UNIX_EPOCH};
    use yoctui_protocol::daemon::DaemonBuildEvent as B;
    for (start, end) in [
        (None, Some(5000)),
        (Some(2000), None),
        (Some(6000), Some(5000)),
        (Some(u64::MAX), Some(u64::MAX)),
    ] {
        let mut app = yoctui_model::App::new(64, 64 * 1024);
        for event in [
            B::Started {
                started_unix_ms: start,
            },
            B::TaskCompleted {
                recipe: "llvm-native".into(),
                task: "do_compile".into(),
                success: true,
                started_unix_ms: start,
                finished_unix_ms: end,
            },
            B::Completed {
                success: true,
                exit_code: Some(0),
                finished_unix_ms: end,
            },
        ] {
            apply_daemon_build_event(&mut app, event);
        }
        let now = UNIX_EPOCH + Duration::from_secs(100);
        assert_eq!(app.build_summary_at(now).elapsed, None);
        assert_eq!(app.completed_tasks[0].task.elapsed_at(now), None);
    }
    let mut app = yoctui_model::App::new(64, 64 * 1024);
    apply_daemon_build_event(
        &mut app,
        B::Started {
            started_unix_ms: Some(2000),
        },
    );
    assert_eq!(app.build_summary_at(UNIX_EPOCH).elapsed, None);
}
