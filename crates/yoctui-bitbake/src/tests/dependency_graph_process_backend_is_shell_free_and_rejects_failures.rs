//! Regression tests grouped around dependency_graph_process_backend_is_shell_free_and_rejects_failures.
use super::*;

#[cfg(unix)]
#[tokio::test]
async fn dependency_graph_process_backend_is_shell_free_and_rejects_failures() {
    let root = fixture_script("dependency-graph-build");
    fs::create_dir_all(&root).unwrap();
    let script = root.join("fake-bitbake");
    fs::write(
        &script,
        r#"#!/bin/sh
test "$1" = "-g" || exit 8
test "$2" = "image" || exit 9
printf '%s\n' 'digraph depends {' '"image.do_build" -> "busybox.do_build"' '}' > task-depends.dot
"#,
    )
    .unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions).unwrap();
    let mut backend = ProcessBackend::with_executable(root.clone(), script.clone())
        .with_compatibility(process_compatibility(&root, &script))
        .unwrap();
    let response = backend.get_dependency_graph("image".into()).await.unwrap();
    assert_eq!(response.graph.root, DependencyNodeId::recipe("image"));
    assert!(
        response
            .graph
            .edges
            .iter()
            .any(|edge| edge.kind == DependencyEdgeKind::Task
                && edge.to == DependencyNodeId::task("busybox", "do_build"))
    );

    fs::write(&script, "#!/bin/sh\nexit 7\n").unwrap();
    let error = backend
        .get_dependency_graph("image".into())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("exited with 7"));

    fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
    let error = backend
        .get_dependency_graph("image".into())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("No such file"));

    let missing = root.join("missing-bitbake");
    let mut unavailable = ProcessBackend::with_executable(root.clone(), missing.clone())
        .with_compatibility(process_compatibility(&root, &missing))
        .unwrap();
    let error = unavailable
        .get_dependency_graph("image".into())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("process:"));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn dependency_graph_process_backend_rejects_symlink_output() {
    let root = fixture_script("dependency-graph-symlink");
    fs::create_dir_all(&root).unwrap();
    let outside = fixture_script("dependency-graph-outside");
    fs::write(&outside, "digraph depends {\n}\n").unwrap();
    let script = root.join("fake-bitbake");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\nln -s '{}' task-depends.dot\n",
            outside.display()
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions).unwrap();
    let mut backend = ProcessBackend::with_executable(root.clone(), script.clone())
        .with_compatibility(process_compatibility(&root, &script))
        .unwrap();
    let error = backend
        .get_dependency_graph("image".into())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("not a regular file"));
    fs::remove_dir_all(root).unwrap();
    fs::remove_file(outside).unwrap();
}

#[tokio::test]
async fn recipe_bitbake_action_process_backend_preserves_force_task_and_target_arguments() {
    let script = fixture_script("fake-recipe-task");
    fs::write(&script, "#!/bin/sh\nprintf '%s\\n' \"$*\"\n").unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions).unwrap();
    let mut backend = shell_backend(script.clone());
    backend
        .start_build(BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("compile".into()),
            force: true,
        })
        .await
        .unwrap();
    let output = loop {
        match backend.next_event().await.unwrap() {
            BackendEvent::Log(entry) => break entry.message,
            BackendEvent::BuildCompleted { .. } => {
                panic!("process completed before its arguments were observed")
            }
            _ => {}
        }
    };
    fs::remove_file(script).unwrap();
    assert_eq!(output, "-f -c compile busybox");
}

#[tokio::test]
async fn process_backend_cancellation_acknowledges_a_hung_child() {
    let script = fixture_script("hung-bitbake");
    fs::write(&script, "#!/bin/sh\nsleep 30\n").unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions).unwrap();
    let mut backend = shell_backend(script.clone());
    backend
        .start_build(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), backend.cancel_build())
        .await
        .unwrap()
        .unwrap();
    loop {
        if let BackendEvent::BuildCompleted { success, .. } =
            tokio::time::timeout(Duration::from_secs(2), backend.next_event())
                .await
                .unwrap()
                .unwrap()
        {
            assert!(!success);
            break;
        }
    }
    fs::remove_file(script).unwrap();
}

#[tokio::test]
async fn process_backend_escalates_after_configured_cancellation_timeout() {
    let script = fixture_script("term-ignoring-bitbake");
    fs::write(
        &script,
        "#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions).unwrap();
    let mut backend =
        shell_backend(script.clone()).with_cancellation_timeout(Duration::from_millis(20));
    backend
        .start_build(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), backend.cancel_build())
        .await
        .unwrap()
        .unwrap();
    fs::remove_file(script).unwrap();
}

#[tokio::test]
async fn process_backend_reports_exit_code() {
    let script = fixture_script("failed-bitbake");
    fs::write(
        &script,
        "#!/bin/sh\nprintf 'ERROR: failed build\\n' >&2\nexit 7\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions).unwrap();
    let mut backend = shell_backend(script.clone());
    backend
        .start_build(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        })
        .await
        .unwrap();
    loop {
        if let BackendEvent::BuildCompleted { success, exit_code } =
            backend.next_event().await.unwrap()
        {
            assert!(!success);
            assert_eq!(exit_code, Some(7));
            break;
        }
    }
    fs::remove_file(script).unwrap();
}

#[test]
fn bridge_stderr_tail_is_bounded_and_redacts_sensitive_lines() {
    let mut tail = BridgeStderrTail::default();
    tail.push(&vec![b'x'; MAX_BRIDGE_STDERR_BYTES + 20]);
    tail.push(b"\nAPI_TOKEN=do-not-display\nlast diagnostic\n");
    let diagnostic = tail.diagnostic().unwrap();
    assert!(tail.bytes.len() <= MAX_BRIDGE_STDERR_BYTES);
    assert!(diagnostic.starts_with("[earlier bridge stderr truncated]"));
    assert!(diagnostic.contains("[redacted sensitive diagnostic]"));
    assert!(diagnostic.contains("last diagnostic"));
    assert!(!diagnostic.contains("do-not-display"));
}

#[cfg(unix)]
#[tokio::test]
async fn daemon_startup_metadata_interrupt_runs_python_cleanup_and_reaps() {
    let script = fixture_script("interrupt-metadata");
    fs::write(&script, r#"import json, signal, sys, time
signal.signal(signal.SIGINT, signal.default_int_handler)
request = json.loads(sys.stdin.readline())
try:
    print(json.dumps({'protocol_version':1, 'sequence':1, 'correlation_id':'1', 'message':{'type':'hello_ack','bitbake_version':'fixture'}}), flush=True)
    time.sleep(60)
finally:
    print('metadata cleanup ran', file=sys.stderr, flush=True)
"#).unwrap();
    let mut backend = BridgeBackend::spawn("python3", script.clone(), std::env::temp_dir())
        .await
        .unwrap();
    backend.interrupt_metadata().await;
    assert!(backend.child.try_wait().unwrap().is_some());
    assert!(
        backend
            .stderr_diagnostic()
            .unwrap()
            .contains("metadata cleanup ran")
    );
    fs::remove_file(script).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn bridge_stderr_is_captured_without_affecting_protocol() {
    let script = fixture_script("bridge-stderr-protocol");
    fs::write(
            &script,
            r#"#!/bin/sh
read -r _request
printf '%s\n' 'NOTE: bridge startup diagnostic' >&2
printf '%s\n' '{"protocol_version":1,"sequence":1,"correlation_id":"1","message":{"type":"hello_ack","bitbake_version":"test"}}'
read -r _request
printf '%s\n' '{"protocol_version":1,"sequence":2,"correlation_id":"2","message":{"type":"bridge_shutdown"}}'
"#,
        )
        .unwrap();
    let mut backend = BridgeBackend::spawn("/bin/sh", script.clone(), std::env::temp_dir())
        .await
        .unwrap();
    for _ in 0..20 {
        if backend.stderr_diagnostic().is_some() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(
        backend.stderr_diagnostic().as_deref(),
        Some("NOTE: bridge startup diagnostic")
    );
    backend.shutdown().await.unwrap();
    fs::remove_file(script).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn bridge_stderr_is_attached_to_failed_handshake() {
    let script = fixture_script("bridge-stderr-failure");
    fs::write(
        &script,
        "#!/bin/sh\nprintf '%s\\n' 'fatal bridge fixture marker' >&2\nexit 17\n",
    )
    .unwrap();
    let error = match BridgeBackend::spawn("/bin/sh", script.clone(), std::env::temp_dir()).await {
        Ok(_) => panic!("bridge startup unexpectedly succeeded"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("fatal bridge fixture marker"));
    assert!(error.to_string().contains("bridge stderr:"));
    fs::remove_file(script).unwrap();
}

#[tokio::test]
async fn bridge_backend_requires_compatibility_before_workspace_inspection() {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bridge/yoctui_bridge.py");
    let mut backend = BridgeBackend::spawn("python3", script, std::env::temp_dir())
        .await
        .unwrap();
    assert!(
        backend
            .inspect_workspace()
            .await
            .unwrap_err()
            .to_string()
            .contains("requires a daemon capability snapshot")
    );
    backend.shutdown().await.unwrap();
}

#[tokio::test]
async fn bridge_backend_waits_for_shutdown_acknowledgement() {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bridge/yoctui_bridge.py");
    let mut backend = BridgeBackend::spawn("python3", script, std::env::temp_dir())
        .await
        .unwrap();
    backend.shutdown().await.unwrap();
    assert!(backend.child.try_wait().unwrap().is_some());
}

#[tokio::test]
async fn bridge_backend_rejects_typed_queries_without_compatibility() {
    let mut backend = BridgeBackend::spawn_bundled("python3", std::env::temp_dir())
        .await
        .unwrap();
    assert!(
        backend
            .list_recipes(None)
            .await
            .unwrap_err()
            .to_string()
            .contains("requires a daemon capability snapshot")
    );
    backend.shutdown().await.unwrap();
}

#[tokio::test]
async fn bundled_bridge_starts_without_a_source_checkout_path() {
    assert!(BUNDLED_BRIDGE_SOURCE.contains("class BitBakeAdapter"));
    let mut backend = BridgeBackend::spawn_bundled("python3", std::env::temp_dir())
        .await
        .unwrap();
    assert!(backend.list_recipes(None).await.is_err());
    backend.shutdown().await.unwrap();
}
