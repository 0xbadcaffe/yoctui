use super::*;

#[tokio::test]
async fn daemon_compatibility_runtime_doctor_compatibility_publishes_initialized_authority() {
    let fixture = RuntimeFixture::new();
    let mut coordinator = DaemonCompatibilityCoordinator::default();
    let compatibility = coordinator
        .startup_from_environment(&fixture.environment)
        .await
        .unwrap()
        .expect("initialized environment must produce authority");
    assert_eq!(
        compatibility.snapshot.environment.build_directory.value(),
        Some(&fixture.build.canonicalize().unwrap())
    );
    assert_eq!(
        compatibility.snapshot.environment.bitbake_version.value(),
        Some(&"2.18.0".to_owned())
    );
    assert_eq!(
        compatibility.snapshot.environment.machine.value(),
        Some(&"qemux86-64".to_owned())
    );
    assert_eq!(
        compatibility
            .snapshot
            .environment
            .layer_series
            .value()
            .unwrap(),
        &[LayerSeriesIdentity {
            layer: "core".into(),
            root: fixture.root.join("layers/meta").canonicalize().unwrap(),
            compatible_series: vec!["wrynose".into()],
        }]
    );
    assert_eq!(
        compatibility
            .snapshot
            .environment
            .poky
            .value()
            .and_then(|release| release.name.as_deref()),
        Some("wrynose")
    );
    assert_eq!(
        compatibility
            .snapshot
            .environment
            .available_tools
            .value()
            .unwrap()
            .iter()
            .find(|tool| tool.id == "bitbake-getvar")
            .unwrap()
            .executable,
        fixture.bin.join("bitbake-getvar").canonicalize().unwrap()
    );
    assert!(compatibility.snapshot.allows(CapabilityId::BitBakeGetVar));
    assert_eq!(
        compatibility
            .implementations
            .get(&CapabilityId::BitBakeGetVar)
            .unwrap()
            .id,
        "bitbake_getvar.argv"
    );

    let mut state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        1,
        "boot".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    yoctui_app::reduce_daemon_state(
        &mut state,
        yoctui_model::DaemonStateAction::ReplaceCompatibility(Box::new(compatibility.clone())),
    )
    .unwrap();
    let wire = yoctui_app::daemon_protocol_snapshot(&state)
        .compatibility
        .expect("journal snapshot must publish compatibility");
    wire.validate().unwrap();
    assert_eq!(wire.generation, compatibility.snapshot.generation);
    assert_eq!(
        wire.environment.bitbake_version,
        yoctui_protocol::daemon::CompatibilityDetected::Detected {
            value: "2.18.0".into(),
            authority: yoctui_protocol::daemon::CompatibilityIdentityAuthority::BitBakeVersionProbe,
        }
    );
    let report = crate::doctor_compatibility_report(Some(&wire), None);
    assert_eq!(
        report.authority,
        crate::DoctorCompatibilityAuthority::Current
    );
    assert!(matches!(
        report
            .environment
            .as_ref()
            .map(|environment| &environment.bitbake_version),
        Some(yoctui_protocol::daemon::CompatibilityDetected::Detected {
            value,
            authority:
                yoctui_protocol::daemon::CompatibilityIdentityAuthority::BitBakeVersionProbe,
        }) if value == "2.18.0"
    ));
}
