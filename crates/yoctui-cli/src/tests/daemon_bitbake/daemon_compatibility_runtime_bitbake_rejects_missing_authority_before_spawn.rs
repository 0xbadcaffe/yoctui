use super::*;

#[test]
fn daemon_compatibility_runtime_bitbake_rejects_missing_authority_before_spawn() {
    let mut supervisor = DaemonBitBakeSupervisor::new(Default::default());
    let error = supervisor
        .start(
            "/work/build".into(),
            BuildRequest {
                targets: vec!["base-files".into()],
                task: Some("listtasks".into()),
                force: false,
            },
        )
        .unwrap_err();
    assert!(error.contains("requires current environment capability authority"));
}
