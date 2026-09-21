use super::*;

#[tokio::test]
async fn backend_probe_process_ignores_stderr_and_fails_closed() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    let report = serde_json::json!({
        "schema": "yoctui.bridge-capability-probe.v1",
        "build_directory": fixture.root,
        "bitbake_version": "99.0",
        "capabilities": ["build", "native_events"],
    });
    let environment = BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]);
    write_executable(
        &fixture.tool,
        &format!("#!/bin/sh\nprintf '%s\\n' '{report}'\nprintf 'probe diagnostic\\n' >&2\n"),
    );
    let capabilities =
        probe_bundled_backend_capabilities(&fixture.tool, &fixture.root, &environment, "99.0")
            .await
            .unwrap();
    assert!(capabilities.contains("build"));
    assert!(
        probe_bundled_backend_capabilities(&fixture.tool, &fixture.root, &environment, "2.19.0")
            .await
            .is_err()
    );
    write_executable(
        &fixture.tool,
        &format!("#!/bin/sh\nprintf '%s\\n' '{report}' >&2\n"),
    );
    assert!(
        probe_bundled_backend_capabilities(&fixture.tool, &fixture.root, &environment, "99.0")
            .await
            .is_err()
    );
    write_executable(&fixture.tool, "#!/bin/sh\nprintf '%70000s' x\n");
    assert!(
        probe_bundled_backend_capabilities(&fixture.tool, &fixture.root, &environment, "99.0")
            .await
            .is_err()
    );
    let overridden = BTreeMap::from([(
        "YOCTUI_BRIDGE_PATH".into(),
        fixture.tool.to_string_lossy().into_owned(),
    )]);
    assert!(
        probe_bundled_backend_capabilities(&fixture.tool, &fixture.root, &overridden, "99.0")
            .await
            .unwrap_err()
            .contains("custom bridge")
    );
}
