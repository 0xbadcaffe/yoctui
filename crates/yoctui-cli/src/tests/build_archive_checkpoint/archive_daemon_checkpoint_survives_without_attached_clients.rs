use super::*;

#[tokio::test]
async fn archive_daemon_checkpoint_survives_without_attached_clients() {
    let root =
        std::env::temp_dir().join(format!("yoctui-archive-checkpoint-{}", std::process::id()));
    let mut snapshot = DaemonSnapshot {
        daemon_instance_id: DaemonInstanceId([2; 16]),
        sequence: 1,
        generation: 1,
        workspace: None,
        project_profile: ProjectProfileSummary::Absent,
        bitbake: BitBakeState {
            lifecycle: LifecycleState::Running,
            version: None,
            capabilities: Vec::new(),
            diagnostic: None,
        },
        compatibility: None,
        jobs: vec![JobSummary {
            id: JobId(4),
            kind: JobKind::BitBakeBuild,
            label: "image".into(),
            lifecycle: LifecycleState::Running,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        }],
        raw_executions: Vec::new(),
        raw_history: Vec::new(),
        pty_sessions: Vec::new(),
        pty_screens: Vec::new(),
        clients: Vec::new(),
        recent_logs: Vec::new(),
        build_progress: None,
        recovery_warnings: Vec::new(),
        build_events: vec![
            DaemonBuildEvent::Reset {
                targets: vec!["image".into()],
            },
            DaemonBuildEvent::Started {
                started_unix_ms: Some(100),
            },
        ],
    };
    let mut recorder = Recorder::new(root.clone());
    recorder.poll(&snapshot, 200).await;
    recorder.finish().await;
    assert_eq!(
        read(&root).unwrap().builds[0].outcome,
        yoctui_model::SavedBuildOutcome::Incomplete
    );
    snapshot.jobs[0].lifecycle = LifecycleState::Exited;
    snapshot.jobs[0].exit_code = Some(0);
    snapshot.build_events.push(DaemonBuildEvent::Completed {
        success: true,
        exit_code: Some(0),
        finished_unix_ms: Some(300),
    });
    recorder.poll(&snapshot, 300).await;
    recorder.finish().await;
    let archive = read(&root).unwrap();
    assert_eq!(archive.builds.len(), 1);
    assert_eq!(
        archive.builds[0].outcome,
        yoctui_model::SavedBuildOutcome::Succeeded
    );
    assert!(
        archive.builds[0]
            .limitations
            .iter()
            .any(|s| s.contains("unavailable"))
    );
    fs::remove_dir_all(root).unwrap();
}
