use super::*;

#[test]
fn oversized_build_update_keeps_daemon_and_existing_client_alive() {
    let fixture = Fixture::new(
        "oversized-update",
        "#!/bin/sh\necho 'BitBake Build Tool Core version 2.18.0'\n",
        true,
    );
    // Each bridge message is valid (<1 MiB), but their combined checkpoint
    // exceeds 4 MiB. These non-completed task rows cannot be evicted as history.
    let bridge = include_str!("../../../../scripts/fixtures/bitbake-ipc-latency-bridge.py")
        .replace(
            "emit({\"type\": \"build_started\"}, build_correlation)",
            "emit({\"type\": \"build_started\"}, build_correlation)\n        active = False\n        for index in range(6):\n            emit({\"type\": \"task_queued\", \"recipe\": str(index) + 'x' * 800000, \"task\": \"do_build\", \"worker\": None, \"stats\": None}, build_correlation)\n        emit({\"type\": \"build_completed\", \"success\": True, \"exit_code\": 0}, build_correlation)",
        );
    assert!(bridge.contains("range(6)"));
    fs::write(fixture.root.join("bridge.py"), bridge).unwrap();
    assert!(fixture.run("start").status.success());
    fixture.wait_for(|| {
        String::from_utf8_lossy(&fixture.run("status").stdout)
            .contains("Initial workspace and recipe inventory ready")
    });
    let (mut connection, snapshot) = fixture.attach();
    connection
        .set_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    let instance = snapshot.daemon_instance_id;
    let mut sequence = snapshot.sequence;
    connection
        .send(&ClientMessage::Command(CommandRequest {
            request_id: RequestId(1),
            expected_generation: None,
            command: DaemonCommand::StartBuild {
                targets: vec!["fixture-image".into()],
                task: None,
                force: false,
            },
        }))
        .unwrap();
    let mut rejected_size = false;
    let mut accepted = false;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        assert!(
            Instant::now() < deadline,
            "build did not complete on the original socket"
        );
        match connection.receive::<ServerMessage>().unwrap() {
            ServerMessage::CommandResult(result) => {
                assert_eq!(result.request_id, RequestId(1));
                assert!(matches!(result.outcome, CommandOutcome::Accepted));
                accepted = true;
            }
            ServerMessage::Event(event) => {
                assert_eq!(event.sequence, sequence + 1, "no replica sequence gaps");
                sequence = event.sequence;
                match event.event {
                    DaemonEvent::Log(log) if log.source == "daemon-budget" => rejected_size = true,
                    DaemonEvent::Build(DaemonBuildEvent::Completed {
                        success: true,
                        exit_code: Some(0),
                        ..
                    }) => break,
                    _ => {}
                }
            }
            ServerMessage::Ping { nonce, .. } => {
                connection.send(&ClientMessage::Pong { nonce }).unwrap()
            }
            ServerMessage::Snapshot(snapshot) => {
                assert_eq!(snapshot.daemon_instance_id, instance);
                sequence = snapshot.sequence;
            }
            other => panic!("unexpected response while daemon must stay connected: {other:?}"),
        }
    }
    assert!(accepted && rejected_size);
    connection
        .send(&ClientMessage::Attach {
            workspace: None,
            resume: None,
            subscription: Subscription {
                state: true,
                jobs: true,
                logs: true,
                pty_sessions: vec![],
            },
        })
        .unwrap();
    loop {
        match connection.receive::<ServerMessage>().unwrap() {
            ServerMessage::Attached { snapshot, .. } => {
                assert_eq!(
                    snapshot.daemon_instance_id, instance,
                    "daemon must not restart"
                );
                assert!(snapshot.build_events.iter().any(|event| matches!(
                    event,
                    DaemonBuildEvent::Completed {
                        success: true,
                        exit_code: Some(0),
                        ..
                    }
                )));
                assert!(
                    snapshot
                        .jobs
                        .iter()
                        .any(|job| job.kind == JobKind::BitBakeBuild
                            && job.lifecycle == LifecycleState::Exited
                            && job.exit_code == Some(0))
                );
                break;
            }
            ServerMessage::Event(_) => {}
            ServerMessage::Snapshot(snapshot) => assert_eq!(snapshot.daemon_instance_id, instance),
            ServerMessage::Ping { nonce, .. } => {
                connection.send(&ClientMessage::Pong { nonce }).unwrap()
            }
            other => panic!("original client could not refresh after oversized update: {other:?}"),
        }
    }
    assert!(fixture.run("status").status.success());
    fixture.stop();
}
