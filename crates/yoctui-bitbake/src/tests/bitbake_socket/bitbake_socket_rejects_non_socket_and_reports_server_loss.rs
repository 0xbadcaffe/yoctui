use super::*;

#[tokio::test]
async fn bitbake_socket_rejects_non_socket_and_reports_server_loss() {
    let (root, script, context) = fixture();
    fs::write(context.build_dir.join("bitbake.sock"), "not a socket").unwrap();
    let adapter = test_adapter(script.clone(), &context);
    let mut controller =
        BitBakeServerController::new(adapter, context.clone(), Duration::from_secs(2)).unwrap();
    assert!(controller.detect().await.is_err());
    assert_eq!(controller.state().lifecycle, BitBakeServerLifecycle::Failed);

    fs::remove_file(context.build_dir.join("bitbake.sock")).unwrap();
    let body = fs::read_to_string(&script)
        .unwrap()
        .replace("request.get(\"correlation_id\")", "\"unknown-correlation\"");
    fs::write(&script, body).unwrap();
    let adapter = test_adapter(script, &context);
    let mut controller =
        BitBakeServerController::new(adapter, context, Duration::from_secs(2)).unwrap();
    assert!(controller.start().await.is_err());
    assert_eq!(controller.state().lifecycle, BitBakeServerLifecycle::Failed);
    assert!(
        controller
            .state()
            .diagnostic
            .as_deref()
            .is_some_and(|message| message.contains("unknown correlation"))
    );
    fs::remove_dir_all(root).unwrap();
}
