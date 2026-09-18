use super::*;

#[cfg(unix)]
#[tokio::test]
async fn signature_workspace_background_operation_reports_success_failure_and_cancellation() {
    use std::os::unix::fs::PermissionsExt;

    let directory = std::env::temp_dir().join(format!(
        "yoctui-signature-workspace-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let stamps = directory.join("tmp/stamps/qemux86_64/busybox");
    fs::create_dir_all(&stamps).unwrap();
    fs::write(
        stamps.join("1.0.do_compile.sigdata.aaa"),
        "fixture artifact",
    )
    .unwrap();
    let dump = directory.join("bitbake-dumpsig");
    let diff = directory.join("bitbake-diffsigs");
    let write_tool = |path: &Path, body: &str| {
        fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(path, permissions).unwrap();
    };
    write_tool(
        &dump,
        "printf '%s\\n' 'basehash_ignore_vars: []' 'taskhash_ignore_tasks: []' 'Task dependencies: []' 'basehash: base-aaa' 'Variable CC value is gcc' 'Tasks this task depends on: []' 'Computed base hash is base-aaa and from file base-aaa' 'Computed task hash is aaa'",
    );
    write_tool(&diff, "exit 0");
    let adapter = SignatureAdapter::with_programs(directory.clone(), dump.clone(), diff.clone())
        .with_compatibility(signature_test_compatibility(&directory))
        .unwrap();
    let target = SignatureTarget {
        recipe: "busybox".into(),
        task: "do_compile".into(),
    };
    let mut app = App::new(10, 1_000);
    let effect = update(&mut app, Action::BeginSignatureDump(target.clone())).unwrap();
    let mut operation = None;
    begin_signature_operation(&mut app, &adapter, &mut operation, effect);
    tokio::time::timeout(Duration::from_secs(2), async {
        while operation.is_some() {
            poll_signature_operation(&mut app, &mut operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        app.signature_dump,
        yoctui_model::SignatureDumpState::Available { .. }
    ));

    write_tool(&dump, "printf 'bad signature\\n' >&2\nexit 7");
    let effect = update(&mut app, Action::RefreshSignatureDump).unwrap();
    begin_signature_operation(&mut app, &adapter, &mut operation, effect);
    tokio::time::timeout(Duration::from_secs(2), async {
        while operation.is_some() {
            poll_signature_operation(&mut app, &mut operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        app.signature_dump,
        yoctui_model::SignatureDumpState::Failed { ref message, .. }
            if message.contains("bad signature")
    ));

    app.notification = None;
    write_tool(&dump, "sleep 30");
    let effect = update(&mut app, Action::BeginSignatureDump(target)).unwrap();
    begin_signature_operation(&mut app, &adapter, &mut operation, effect);
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        operation
            .as_ref()
            .is_some_and(|operation| operation.cancellation.cancel())
    );
    tokio::time::timeout(Duration::from_secs(2), async {
        while operation.is_some() {
            poll_signature_operation(&mut app, &mut operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        app.signature_dump,
        yoctui_model::SignatureDumpState::Failed { ref message, .. }
            if message.contains("cancelled")
    ));
    fs::remove_dir_all(directory).unwrap();
}
