use super::*;

#[tokio::test]
async fn security_workflow_cli_revalidates_exact_report_provider_and_advisory_opens() {
    let fixture = SecurityCliFixture::new("#!/bin/sh\nexit 0\n");
    let report = fixture.write_cve();
    let marker = fixture.root.join("opened-url");
    let opener = fixture.bin.join("xdg-open");
    fs::write(
        &opener,
        format!("#!/bin/sh\nprintf opened > '{}'\n", marker.display()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&opener).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&opener, permissions).unwrap();
    }
    let mut app = fixture.app();
    let mut coordinator =
        SecurityCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
    let request = SecurityReportRequest::new(1, vec![report.clone()]).unwrap();
    app.security.inventory = yoctui_model::SecurityInventoryState::Loading {
        request: request.clone(),
    };
    coordinator.begin_report_scan(request);
    poll_security_until(&mut coordinator, &mut app, |app, coordinator| {
        coordinator.report.is_none()
            && app
                .security
                .inventory
                .reports()
                .is_some_and(|reports| !reports.is_empty())
    })
    .await;
    assert!(
        coordinator
            .revalidate_open_path(&app, &report)
            .await
            .is_ok()
    );
    assert!(
        coordinator
            .revalidate_open_path(&app, &fixture.provider)
            .await
            .is_err(),
        "a provider is authorized only by the selected exact Security scope"
    );

    let input =
        security_capability_input(&app, fixture.build.clone(), vec![fixture.bin.clone()]).unwrap();
    let capability = SecurityCapabilityInspector::new(input).inspect().unwrap();
    let _ = update(
        &mut app,
        Action::Security(SecurityAction::CapabilityLoaded(capability)),
    );
    assert!(
        coordinator
            .revalidate_open_path(&app, &fixture.provider)
            .await
            .is_ok()
    );
    fs::write(&report, "{}").unwrap();
    assert!(
        coordinator
            .revalidate_open_path(&app, &report)
            .await
            .is_err(),
        "changed report identity must fail closed"
    );

    open_security_url(
        &mut app,
        coordinator.url_opener(),
        "https://example.invalid/CVE-2026-0001".into(),
    )
    .await;
    assert_eq!(fs::read_to_string(marker).unwrap(), "opened");
    open_security_url(
        &mut app,
        coordinator.url_opener(),
        "http://example.invalid/not-allowed".into(),
    )
    .await;
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("invalid"))
    );
}
