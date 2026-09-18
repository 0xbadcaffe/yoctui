use super::*;

#[tokio::test]
async fn qa_workflow_cli_revalidates_exact_report_provider_source_and_layer_opens() {
    let fixture = QaCliFixture::new("#!/bin/sh\nexit 0\n");
    let report = fixture.write_report();
    let mut app = fixture.app();
    let mut coordinator = QaCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
    inspect_qa_capability(&fixture, &mut app, &mut coordinator).await;
    app.qa.check_selection = Some(QaCheckId::new("recipe-package".into()).unwrap());
    let request = QaReportRequest::new(1, vec![report.clone()]).unwrap();
    app.qa.inventory = yoctui_model::QaReportInventoryState::Loading {
        request: request.clone(),
    };
    coordinator.begin_report_scan(&app, request);
    poll_qa_until(&mut coordinator, &mut app, |app, coordinator| {
        coordinator.report.is_none()
            && app
                .qa
                .inventory
                .reports()
                .is_some_and(|reports| !reports.is_empty())
    })
    .await;
    let identity = app.qa.inventory.reports().unwrap()[0].identity.clone();
    let source = app.qa.inventory.reports().unwrap()[0].findings[0]
        .source
        .clone()
        .unwrap();
    assert!(coordinator.revalidate_report(&app, &identity).is_ok());
    assert!(
        coordinator
            .revalidate_provider(
                &app,
                &RecipeIdentity {
                    name: "busybox".into(),
                    file: fixture.provider.clone(),
                }
            )
            .is_ok()
    );
    assert!(coordinator.revalidate_source(&app, &source).is_ok());

    let effect = update(&mut app, Action::Qa(QaAction::CycleView)).unwrap();
    assert!(coordinator.handle_effect(&mut app, effect).await);
    poll_qa_until(&mut coordinator, &mut app, |app, _| {
        app.qa.layer_capability.snapshot().is_some()
    })
    .await;
    let layer = app.qa.selected_layer().unwrap().identity.clone();
    assert!(coordinator.revalidate_layer(&app, &layer).is_ok());
    fs::write(&report, "{}").unwrap();
    assert!(coordinator.revalidate_report(&app, &identity).is_err());
    fs::remove_file(&fixture.source).unwrap();
    assert!(coordinator.revalidate_source(&app, &source).is_err());
}
