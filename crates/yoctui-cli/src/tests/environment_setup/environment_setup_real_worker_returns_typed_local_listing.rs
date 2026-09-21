use super::*;

#[tokio::test]
async fn environment_setup_real_worker_returns_typed_local_listing() {
    let mut app = App::new(20, 2000);
    let effect = yoctui_model::update(
        &mut app,
        Action::EnvironmentSetup(EnvironmentSetupAction::Open { browse: true }),
    )
    .unwrap();
    let mut io = EnvironmentBrowserIo::default();
    io.submit(effect);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !io.poll(&mut app).await {
        assert!(std::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let Some(Dialog::EnvironmentSetup(setup)) = app.active_dialog() else {
        panic!("setup missing")
    };
    assert!(setup.browser.as_ref().unwrap().directory.is_some());
}
