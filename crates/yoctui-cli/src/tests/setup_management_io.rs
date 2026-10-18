use super::*;
use std::fs;

#[test]
fn setup_fragments_parse_only_recognized_tabbed_sections() {
    let text = "NOTE: Loading cache...\nAvailable built-in fragments:\nmachine/...\tSets MACHINE\n\nEnabled built-in fragments:\nmachine/qemux86-64\tSets MACHINE x86\n\nAvailable fragments in core layer located in /core:\nEnabled fragments:\ncore/yocto/a\tSummary A\nUnused fragments:\ncore/yocto/b\tSummary B\n";
    let rows = fragments::parse(text).unwrap();
    assert_eq!(rows.len(), 4);
    assert!(rows[0].pattern);
    assert!(rows[1].enabled);
    assert!(rows[2].enabled);
    assert!(!rows[3].enabled);
    assert!(fragments::parse("garbage (not an API response)").is_err());
    assert!(fragments::parse("Unused fragments:\n../bad\tunknown\n").is_err());
}

#[tokio::test]
#[cfg(unix)]
async fn setup_fragment_inspection_sources_exact_environment_and_never_writes_toolcfg() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let build = root.join("build");
    fs::create_dir_all(build.join("conf")).unwrap();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    fs::write(build.join("conf/local.conf"), "").unwrap();
    fs::write(build.join("conf/bblayers.conf"), "").unwrap();
    let script = build.join("init-build-env");
    fs::write(
        &script,
        format!(
            "test \"$#\" = 0 || return 41\nexport PATH=\"{}:$PATH\"\nexport BUILDDIR=\"{}\"\n",
            bin.display(),
            build.display()
        ),
    )
    .unwrap();
    let tool = bin.join("bitbake-config-build");
    fs::write(&tool,"#!/bin/sh\ntest \"$3\" = list-fragments || exit 3\nprintf 'Enabled built-in fragments:\\nmachine/qemux86-64\\tSelected machine\\n'\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    let profile = yoctui_model::BuildEnvironmentProfile {
        source_dir: build.clone(),
        build_dir: build.clone(),
        init_script: script,
    };
    let result = perform(R::Inspect {
        generation: 4,
        profile: profile.clone(),
    })
    .await;
    assert!(
        matches!(result,A::Loaded {generation:4,result:Ok(rows)} if rows.len()==1 && rows[0].enabled)
    );
    assert!(!build.join("conf/toolcfg.conf").exists());
    let d = SetupOperationDialog {
        operation: SetupOperation::Update,
        profile,
        tool: tool.display().to_string(),
        fragment: String::new(),
        editor: None,
        config_policy: 0,
        backup_conflicts: false,
        offline: true,
        preparing: None,
        error: None,
    };
    assert!(
        prepare(&d).is_err(),
        "legacy/non-managed setup must not update"
    );
}

#[tokio::test]
async fn setup_capture_bounds_output_and_times_out_without_blocking_the_ui() {
    let request = TerminalLaunchRequest {
        name: "test".into(),
        kind: yoctui_model::TerminalCreationKind::Utility,
        cwd: std::env::temp_dir(),
        program: "/bin/sh".into(),
        arguments: vec!["-c".into(), "sleep 10".into()],
        completion: None,
    };
    assert!(
        capture(request, Duration::from_millis(40))
            .await
            .unwrap_err()
            .contains("timed out")
    );
    assert!(bounded(&b"12345"[..], 4).await.is_err());
}
