use super::*;

#[test]
fn compatibility_snapshot_accepts_case_preserving_distro_identity() {
    let mut snapshot = compatibility_snapshot_fixture(1);
    snapshot.environment.distro = CompatibilityDetected::Detected {
        value: CompatibilityDistroIdentity {
            name: "Poky+custom".into(),
            version: Some("5.2.4".into()),
        },
        authority: CompatibilityIdentityAuthority::BitBakeDatastore,
    };
    snapshot.validate().unwrap();
}
