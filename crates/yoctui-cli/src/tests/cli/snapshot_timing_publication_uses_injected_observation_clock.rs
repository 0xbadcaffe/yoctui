use super::*;

#[test]
fn snapshot_timing_publication_uses_injected_observation_clock() {
    use yoctui_bitbake::BackendEvent as B;
    use yoctui_protocol::daemon::{DaemonBuildEvent as D, JobId};
    let events = [
        B::BuildStarted,
        B::TaskStarted {
            recipe: "llvm-native".into(),
            task: "do_compile".into(),
            pid: Some(42),
            worker: None,
            log_path: None,
            stats: None,
        },
        B::TaskCompleted {
            recipe: "llvm-native".into(),
            task: "do_compile".into(),
            success: true,
        },
        B::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    ];
    for event in events {
        let (event, _) = daemon_build_event_at(event, JobId(1), 123456);
        match event.unwrap() {
            D::Started { started_unix_ms }
            | D::TaskStarted {
                started_unix_ms, ..
            } => assert_eq!(started_unix_ms, Some(123456)),
            D::TaskCompleted {
                started_unix_ms,
                finished_unix_ms,
                ..
            } => {
                assert_eq!(started_unix_ms, None);
                assert_eq!(finished_unix_ms, Some(123456));
            }
            D::Completed {
                finished_unix_ms, ..
            } => assert_eq!(finished_unix_ms, Some(123456)),
            event => panic!("unexpected {event:?}"),
        }
    }
}
