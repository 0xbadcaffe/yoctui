use super::*;

#[tokio::test]
async fn maintenance_workflow_runner_maps_nonzero_and_timeout_distinctly() {
    for (script, id, timeout, expected) in [
        (
            "#!/bin/sh\nexit 7\n",
            MaintenanceSessionId(92),
            30,
            MaintenanceSessionStatus::Failed,
        ),
        (
            "#!/bin/sh\nsleep 5\n",
            MaintenanceSessionId(93),
            1,
            MaintenanceSessionStatus::TimedOut,
        ),
    ] {
        let fixture = Fixture::new(script);
        let (mut app, mut coordinator, request) = refreshed_coordinator(&fixture).await;
        begin_readiness(&mut app, &mut coordinator, request, id, timeout).await;
        poll_until(&mut coordinator, &mut app, |app, coordinator| {
            !coordinator.operation_active()
                && app
                    .maintenance
                    .sessions
                    .back()
                    .is_some_and(|session| session.status.is_terminal())
        })
        .await;
        assert_eq!(app.maintenance.sessions.back().unwrap().status, expected);
        coordinator.shutdown().await;
    }
}
