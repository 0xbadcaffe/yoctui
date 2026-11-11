use super::*;
#[test]
fn explicit_profiles_preserve_custom_scripts_and_allow_reviewing_new_builds_without_sourcing() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-explicit-env-{}-{}",
        std::process::id(),
        yoctui_utils::unix_ms()
    ));
    let source = root.join("source");
    fs::create_dir_all(&source).unwrap();
    let script = source.join("custom-init.sh");
    let marker = root.join("must-not-source");
    fs::write(&script, format!("touch '{}'\n", marker.display())).unwrap();
    let profile = BuildEnvironmentProfile {
        source_dir: source,
        build_dir: root.join("new-build"),
        init_script: script.clone(),
    };
    let plan = SavedEnvironmentPlan {
        edited_profile: true,
        profile: profile.clone(),
        target: String::new(),
        machine: None,
        mode: SavedEnvironmentMode::Start,
    };
    revalidate_plan(&plan).unwrap();
    assert!(!marker.exists());
    assert!(!profile.build_dir.exists());
    let archived = SavedEnvironmentPlan {
        edited_profile: false,
        ..plan.clone()
    };
    assert!(
        revalidate_plan(&archived).is_err(),
        "saved archives must retain their missing-build guard"
    );
    fs::remove_file(script).unwrap();
    assert!(revalidate_plan(&plan).is_err());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn explicit_profile_refresh_restarts_only_idle_daemons_even_for_the_same_build() {
    let mut s = snapshot();
    s.compatibility = Some(CompatibilitySnapshotData {
        schema_version: COMPATIBILITY_SCHEMA_VERSION,
        generation: 1,
        environment: CompatibilityEnvironmentIdentity {
            build_directory: CompatibilityDetected::Detected {
                value: "/build".into(),
                authority: CompatibilityIdentityAuthority::InitializedEnvironment,
            },
            source_roots: CompatibilityDetected::Unknown,
            bitbake_version: CompatibilityDetected::Unknown,
            oe_core: CompatibilityDetected::Unknown,
            poky: CompatibilityDetected::Unknown,
            distro: CompatibilityDetected::Unknown,
            machine: CompatibilityDetected::Unknown,
            layer_series: CompatibilityDetected::Unknown,
            available_tools: CompatibilityDetected::Unknown,
            backend: CompatibilityDetected::Unknown,
            protocol: CompatibilityDetected::Unknown,
        },
        capabilities: Vec::new(),
    });
    assert!(matches!(
        profile_mode(None).unwrap(),
        SavedEnvironmentMode::Start
    ));
    assert!(
        matches!(profile_mode(Some(s.clone())).unwrap(),SavedEnvironmentMode::Restart {instance} if instance == [1;16])
    );
    s.jobs.push(JobSummary {
        id: JobId(1),
        kind: JobKind::BitBakeBuild,
        label: "keep running".into(),
        lifecycle: LifecycleState::Running,
        progress_current: None,
        progress_total: None,
        exit_code: None,
    });
    assert!(
        profile_mode(Some(s.clone()))
            .unwrap_err()
            .to_string()
            .contains("active job")
    );
    s.jobs.clear();
    s.pty_sessions.push(PtySessionSummary {
        id: PtySessionId(1),
        name: "keep terminal".into(),
        kind: PtyKind::BuildShell,
        cwd: "/build".into(),
        lifecycle: LifecycleState::Running,
        dimensions: TerminalDimensions {
            rows: 24,
            columns: 80,
        },
        writer: None,
        writer_epoch: 0,
        viewers: 0,
        exit_code: None,
        restartable: true,
    });
    assert!(
        profile_mode(Some(s))
            .unwrap_err()
            .to_string()
            .contains("active terminal")
    );
}
