use super::*;

#[test]
fn doctor_compatibility_reports_bounded_authority_and_exact_degradation() {
    let snapshot = doctor_compatibility_fixture();
    snapshot.validate().unwrap();
    let report = doctor_compatibility_report(Some(&snapshot), None);
    assert_eq!(report.authority, DoctorCompatibilityAuthority::Current);
    assert_eq!(
        report.operating_mode,
        Some(DoctorCompatibilityMode::Degraded)
    );
    assert_eq!(
        report.summary,
        DoctorCompatibilitySummary {
            available: 1,
            limited: 1,
            unavailable: 1,
            unknown: 1,
            unsupported: 1,
        }
    );
    assert_eq!(report.missing_tools[0].tool, "devtool");
    assert_eq!(report.limited_features[0].id, "bitbake.getvar");
    assert_eq!(
        report.limited_features[0].implementation.as_deref(),
        Some("bitbake.getvar.environment_fallback")
    );
    assert_eq!(report.unsupported_features[0].id, "git_archive");
    let human = render_doctor_compatibility(&report);
    for expected in [
        "snapshot generation: 12",
        "BitBake: 2.18.0",
        "Poky: wrynose 6.0",
        "DISTRO: poky 6.0",
        "MACHINE: qemux86-64",
        "backend: tinfoil 2.18",
        "protocol: yoctui-daemon 1.0",
        "missing tool: devtool",
        "limited: bitbake.getvar",
        "unsupported: git_archive",
        "unknown: resulttool",
    ] {
        assert!(human.contains(expected), "missing {expected}: {human}");
    }

    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["schema"], "yoctui.doctor.compatibility.v1");
    assert_eq!(json["authority"], "current");
    assert_eq!(json["summary"]["limited"], 1);
    assert_eq!(json["capabilities"].as_array().unwrap().len(), 5);
}
