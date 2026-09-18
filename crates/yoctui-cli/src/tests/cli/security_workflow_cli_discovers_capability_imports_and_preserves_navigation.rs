use super::*;

#[tokio::test]
async fn security_workflow_cli_discovers_capability_imports_and_preserves_navigation() {
    let fixture = SecurityCliFixture::new("#!/bin/sh\nexit 0\n");
    let report = fixture.write_cve();
    let mut app = fixture.app();
    let mut coordinator =
        SecurityCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);

    let effect = update(
        &mut app,
        Action::Security(SecurityAction::InspectCapability),
    )
    .unwrap();
    assert!(coordinator.handle_effect(&mut app, effect).await);
    app.screen = Screen::Dashboard;
    poll_security_until(&mut coordinator, &mut app, |app, _| {
        matches!(
            app.security.capability,
            yoctui_model::SecurityCapability::Available(_)
        )
    })
    .await;
    let yoctui_model::SecurityCapability::Available(capability) = &app.security.capability else {
        unreachable!()
    };
    assert_eq!(capability.release.as_deref(), Some("6.0"));
    assert_eq!(capability.cve_task.as_deref(), Some("cve_check"));
    assert_eq!(
        capability.recipe_sbom_task.as_deref(),
        Some("create_recipe_sbom")
    );
    assert_eq!(
        capability.mapper.as_ref().unwrap().executable,
        fixture.bin.join("cve-check-map-pkgs")
    );

    let _ = update(&mut app, Action::Security(SecurityAction::BeginImport));
    let effect = update(
        &mut app,
        Action::Security(SecurityAction::ConfirmImport(format!(
            "root = \"{}\"\n",
            report.display()
        ))),
    )
    .unwrap();
    assert!(coordinator.handle_effect(&mut app, effect).await);
    poll_security_until(&mut coordinator, &mut app, |app, coordinator| {
        coordinator.report.is_none()
            && app
                .security
                .inventory
                .reports()
                .is_some_and(|reports| !reports.is_empty())
    })
    .await;
    assert_eq!(app.screen, Screen::Dashboard);
    assert_eq!(
        app.security.visible_findings()[0].identity.cve,
        "CVE-2026-0001"
    );
}
