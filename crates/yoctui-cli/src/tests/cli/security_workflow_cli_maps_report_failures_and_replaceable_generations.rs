use super::*;

#[tokio::test]
async fn security_workflow_cli_maps_report_failures_and_replaceable_generations() {
    let fixture = SecurityCliFixture::new("#!/bin/sh\nexit 0\n");
    let empty = fixture.root.join("empty");
    fs::create_dir(&empty).unwrap();
    let mut app = fixture.app();
    let mut coordinator =
        SecurityCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);

    let request = SecurityReportRequest::new(1, vec![empty.clone()]).unwrap();
    app.security.inventory = yoctui_model::SecurityInventoryState::Loading {
        request: request.clone(),
    };
    coordinator.begin_report_scan(request);
    poll_security_until(&mut coordinator, &mut app, |app, coordinator| {
        coordinator.report.is_none()
            && matches!(
                app.security.inventory,
                yoctui_model::SecurityInventoryState::AvailableEmpty { .. }
            )
    })
    .await;

    for (generation, error) in [
        (2, SecurityReportAdapterError::Cancelled),
        (3, SecurityReportAdapterError::Timeout(30)),
        (
            4,
            SecurityReportAdapterError::WorkerLost("worker channel closed".into()),
        ),
        (
            5,
            SecurityReportAdapterError::PermissionDenied(empty.clone()),
        ),
    ] {
        let request = SecurityReportRequest::new(generation, vec![empty.clone()]).unwrap();
        app.security.inventory = yoctui_model::SecurityInventoryState::Loading {
            request: request.clone(),
        };
        let cancellation = SecurityReportCancellation::default();
        coordinator.report = Some(SecurityReportCliOperation {
            request,
            cancellation,
            handle: tokio::spawn(async move { Err(error) }),
        });
        tokio::task::yield_now().await;
        coordinator.poll(&mut app).await;
        assert!(
            matches!(
                (&app.security.inventory, generation),
                (yoctui_model::SecurityInventoryState::Cancelled { .. }, 2)
                    | (yoctui_model::SecurityInventoryState::TimedOut { .. }, 3)
                    | (yoctui_model::SecurityInventoryState::Lost { .. }, 4)
                    | (yoctui_model::SecurityInventoryState::Failed { .. }, 5)
            ),
            "generation {generation}: {:?}",
            app.security.inventory
        );
    }

    let first = SecurityReportRequest::new(6, vec![empty.clone()]).unwrap();
    app.security.inventory = yoctui_model::SecurityInventoryState::Loading {
        request: first.clone(),
    };
    coordinator.begin_report_scan(first);
    let replacement = SecurityReportRequest::new(7, vec![empty]).unwrap();
    app.security.inventory = yoctui_model::SecurityInventoryState::Loading {
        request: replacement.clone(),
    };
    coordinator.begin_report_scan(replacement);
    poll_security_until(&mut coordinator, &mut app, |app, coordinator| {
        coordinator.report.is_none()
            && app
                .security
                .inventory
                .request()
                .is_some_and(|request| request.generation == 7)
    })
    .await;
    assert_eq!(app.security.inventory.request().unwrap().generation, 7);
}
