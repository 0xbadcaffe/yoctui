use super::*;

#[test]
fn daemon_build_job_updates_preserve_the_requested_targets() {
    let mut job = yoctui_protocol::daemon::JobSummary {
        id: yoctui_protocol::daemon::JobId(7),
        kind: yoctui_protocol::daemon::JobKind::BitBakeBuild,
        label: "BitBake build".into(),
        lifecycle: yoctui_protocol::daemon::LifecycleState::Failed,
        progress_current: None,
        progress_total: None,
        exit_code: Some(1),
    };

    preserve_bitbake_target_label(Some("BitBake build core-image-full-cmdline"), &mut job);

    assert_eq!(job.label, "BitBake build core-image-full-cmdline");
}
