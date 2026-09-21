use super::*;

#[tokio::test]
async fn maintenance_service_shared_runner_streams_success_and_nonzero_without_a_shell() {
    for (name, body, success) in [
        (
            "success",
            "#!/bin/sh\nprintf 'service:%s\\n' \"$1\"\nprintf 'warning\\n' >&2\nexit 0\n",
            true,
        ),
        ("nonzero", "#!/bin/sh\necho failed >&2\nexit 7\n", false),
    ] {
        let fixture = TestDirectory::new(name);
        prepare_fixture(&fixture, Some(body));
        let inspection = MaintenanceServiceCapabilityInspector::inspect(fixture_input(
            &fixture,
            Some("localhost:0".into()),
            None,
            None,
        ))
        .unwrap();
        let (_, command) = pr_service_command(
            MaintenanceSessionId(1),
            1,
            &inspection.capability,
            1,
            export_request(&inspection, &fixture),
        )
        .unwrap();
        let mut runner = MaintenanceSstateJobRunner::new();
        runner.start(command).await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Started { .. }
        ));
        let terminal = terminal_event(&mut runner).await;
        assert_eq!(
            matches!(terminal, MaintenanceSstateRunnerEvent::Completed { .. }),
            success
        );
        if !success {
            assert!(matches!(
                terminal,
                MaintenanceSstateRunnerEvent::Failed {
                    exit_code: Some(7),
                    ..
                }
            ));
        }
    }
}
