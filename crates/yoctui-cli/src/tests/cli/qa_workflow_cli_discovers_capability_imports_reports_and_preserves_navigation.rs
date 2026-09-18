use super::*;

#[tokio::test]
async fn qa_workflow_cli_discovers_capability_imports_reports_and_preserves_navigation() {
    let fixture = QaCliFixture::new("#!/bin/sh\nexit 0\n");
    let report = fixture.write_report();
    let mut app = fixture.app();
    let mut coordinator = QaCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
    inspect_qa_capability(&fixture, &mut app, &mut coordinator).await;
    app.qa.check_selection = Some(QaCheckId::new("recipe-package".into()).unwrap());

    let _ = update(&mut app, Action::Qa(QaAction::BeginImport));
    let effect = update(
        &mut app,
        Action::Qa(QaAction::ConfirmImport(format!(
            "root = \"{}\"\n",
            report.display()
        ))),
    )
    .unwrap();
    assert!(coordinator.handle_effect(&mut app, effect).await);
    app.screen = Screen::Layers;
    poll_qa_until(&mut coordinator, &mut app, |app, coordinator| {
        coordinator.report.is_none()
            && app
                .qa
                .inventory
                .reports()
                .is_some_and(|reports| !reports.is_empty())
    })
    .await;
    assert_eq!(app.screen, Screen::Layers);
    assert_eq!(app.qa.visible_findings().len(), 1);
    assert_eq!(
        app.qa.visible_findings()[0].message,
        "license checksum needs review"
    );
}
