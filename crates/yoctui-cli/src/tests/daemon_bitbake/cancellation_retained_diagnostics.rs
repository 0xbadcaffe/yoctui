use super::*;

#[test]
fn daemon_cancellation_publishers_keep_explicit_outcomes_and_build_target() {
    use yoctui_model::{DaemonGlobalState, DaemonModelInstanceId, DaemonStateLimits};
    use yoctui_protocol::daemon::*;
    let state = DaemonGlobalState::new(
        DaemonModelInstanceId([9; 16]),
        1,
        "test-boot".into(),
        DaemonStateLimits::default(),
    )
    .unwrap();
    let mut journal = DaemonSnapshotJournal::new(
        crate::daemon_protocol_snapshot(&state),
        DaemonSnapshotLimits::default(),
    )
    .unwrap();
    journal
        .publish(DaemonEvent::JobChanged(JobSummary {
            id: JobId(1),
            kind: JobKind::BitBakeBuild,
            label: "BitBake build core-image-minimal".into(),
            lifecycle: LifecycleState::Running,
            exit_code: None,
            progress_current: None,
            progress_total: None,
        }))
        .unwrap();
    crate::publish_daemon_bitbake_event(
        &mut journal,
        DaemonBitBakeEvent::Cancelled {
            job_id: JobId(1),
            exit_code: Some(1),
        },
    )
    .unwrap();
    assert_eq!(
        journal.snapshot().jobs[0].lifecycle,
        LifecycleState::Cancelled
    );
    assert_eq!(
        journal.snapshot().jobs[0].label,
        "BitBake build core-image-minimal"
    );
    assert!(matches!(
        journal.snapshot().build_events.last(),
        Some(DaemonBuildEvent::Cancelled {
            exit_code: Some(1),
            finished_unix_ms: Some(_)
        })
    ));
    assert_eq!(
        yoctui_app::capture_saved_build(journal.snapshot(), 1000)
            .unwrap()
            .outcome,
        yoctui_model::SavedBuildOutcome::Cancelled
    );
    crate::publish_daemon_devtool_event(
        &mut journal,
        crate::daemon_devtool::DaemonDevtoolEvent::Cancelled {
            job_id: JobId(2),
            forced: true,
            exit_code: Some(130),
        },
    )
    .unwrap();
    crate::publish_daemon_qemu_event(
        &mut journal,
        crate::daemon_qemu::DaemonQemuEvent::Cancelled {
            job_id: JobId(3),
            session_id: 3,
            forced: true,
            exit_code: Some(130),
        },
    )
    .unwrap();
    assert!(
        journal
            .snapshot()
            .jobs
            .iter()
            .all(|job| job.lifecycle == LifecycleState::Cancelled)
    );
    let decoded =
        decode_frame::<DaemonSnapshot>(&encode_frame(journal.snapshot()).unwrap()).unwrap();
    assert!(
        decoded
            .jobs
            .iter()
            .all(|job| job.lifecycle == LifecycleState::Cancelled)
    );
}

#[test]
fn daemon_cancellation_retains_errors_as_logs_not_stale_failure_terminals() {
    let mut supervisor = DaemonBitBakeSupervisor::new(Default::default());
    supervisor
        .reliable_tx
        .try_send(DaemonBitBakeEvent::Failed {
            job_id: JobId(1),
            message: "bridge failure during shutdown".into(),
        })
        .unwrap();
    supervisor
        .reliable_tx
        .try_send(DaemonBitBakeEvent::Backend {
            job_id: JobId(1),
            event: Box::new(BackendEvent::CommandFailed {
                code: "task_failure".into(),
                message: "retained diagnostic".into(),
            }),
        })
        .unwrap();
    supervisor
        .cancellation_terminal_tx
        .try_send(DaemonBitBakeEvent::Cancelled {
            job_id: JobId(1),
            exit_code: Some(1),
        })
        .unwrap();
    assert!(matches!(
        supervisor.try_event(),
        Some(DaemonBitBakeEvent::Cancelled { .. })
    ));
    for text in ["bridge failure", "retained diagnostic"] {
        let Some(DaemonBitBakeEvent::Backend { event, .. }) = supervisor.try_event() else {
            panic!("diagnostic was lost")
        };
        let BackendEvent::Log(entry) = *event else {
            panic!("stale terminal survived cancellation")
        };
        assert!(entry.message.contains(text));
        assert_eq!(entry.severity, yoctui_model::Severity::Error);
        assert!(entry.protected);
    }
    assert!(supervisor.try_event().is_none());
}
