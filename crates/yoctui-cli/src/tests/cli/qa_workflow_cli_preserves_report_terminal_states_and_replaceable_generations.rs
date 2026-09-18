use super::*;

#[tokio::test]
async fn qa_workflow_cli_preserves_report_terminal_states_and_replaceable_generations() {
    let fixture = QaCliFixture::new("#!/bin/sh\nexit 0\n");
    let empty = fixture.build.join("empty-qa");
    fs::create_dir(&empty).unwrap();
    let empty = fs::canonicalize(empty).unwrap();
    let mut app = fixture.app();
    let mut coordinator = QaCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
    inspect_qa_capability(&fixture, &mut app, &mut coordinator).await;
    app.qa.check_selection = Some(QaCheckId::new("recipe-package".into()).unwrap());

    for (generation, error) in [
        (1, QaReportAdapterError::Cancelled),
        (2, QaReportAdapterError::Timeout(30)),
        (
            3,
            QaReportAdapterError::WorkerLost("worker channel closed".into()),
        ),
        (4, QaReportAdapterError::PermissionDenied(empty.clone())),
    ] {
        let request = QaReportRequest::new(generation, vec![empty.clone()]).unwrap();
        app.qa.inventory = yoctui_model::QaReportInventoryState::Loading {
            request: request.clone(),
        };
        coordinator.report = Some(QaReportCliOperation {
            request,
            cancellation: QaReportCancellation::default(),
            handle: tokio::spawn(async move { Err(error) }),
        });
        tokio::task::yield_now().await;
        coordinator.poll(&mut app).await;
        assert!(
            matches!(
                (&app.qa.inventory, generation),
                (yoctui_model::QaReportInventoryState::Cancelled { .. }, 1)
                    | (yoctui_model::QaReportInventoryState::TimedOut { .. }, 2)
                    | (yoctui_model::QaReportInventoryState::Lost { .. }, 3)
                    | (yoctui_model::QaReportInventoryState::Failed { .. }, 4)
            ),
            "generation {generation}: {:?}",
            app.qa.inventory
        );
    }

    let first = QaReportRequest::new(5, vec![empty.clone()]).unwrap();
    app.qa.inventory = yoctui_model::QaReportInventoryState::Loading {
        request: first.clone(),
    };
    coordinator.begin_report_scan(&app, first);
    let replacement = QaReportRequest::new(6, vec![empty]).unwrap();
    app.qa.inventory = yoctui_model::QaReportInventoryState::Loading {
        request: replacement.clone(),
    };
    coordinator.begin_report_scan(&app, replacement);
    poll_qa_until(&mut coordinator, &mut app, |app, coordinator| {
        coordinator.report.is_none()
            && app
                .qa
                .inventory
                .request()
                .is_some_and(|request| request.generation == 6)
    })
    .await;
    assert!(matches!(
        app.qa.inventory,
        yoctui_model::QaReportInventoryState::AvailableEmpty { .. }
    ));
}
