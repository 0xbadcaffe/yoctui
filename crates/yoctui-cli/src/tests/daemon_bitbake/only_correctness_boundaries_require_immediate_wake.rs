use super::*;

#[test]
fn only_correctness_boundaries_require_immediate_wake() {
    assert!(!bitbake_event_requires_immediate_wake(&log_event(
        yoctui_model::Severity::Warning,
        "retained but batchable warning",
    )));
    assert!(bitbake_event_requires_immediate_wake(
        &DaemonBitBakeEvent::Failed {
            job_id: JobId(1),
            message: "bridge failed".into(),
        }
    ));
    assert!(bitbake_event_requires_immediate_wake(
        &DaemonBitBakeEvent::Backend {
            job_id: JobId(1),
            event: Box::new(BackendEvent::TaskCompleted {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                success: false,
            }),
        }
    ));
    assert!(bitbake_event_requires_immediate_wake(
        &DaemonBitBakeEvent::Backend {
            job_id: JobId(1),
            event: Box::new(BackendEvent::BuildCompleted {
                success: true,
                exit_code: Some(0),
            }),
        }
    ));
}
