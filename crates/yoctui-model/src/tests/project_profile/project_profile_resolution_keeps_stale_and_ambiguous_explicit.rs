use super::*;

#[test]
fn project_profile_resolution_keeps_stale_and_ambiguous_explicit() {
    let stale: ProjectIdentityResolution<String> = ProjectIdentityResolution::Stale {
        identity: "old-image".into(),
        reason: "not reported by BitBake".into(),
    };
    assert!(matches!(stale, ProjectIdentityResolution::Stale { .. }));
    let ambiguous: ProjectIdentityResolution<String> = ProjectIdentityResolution::Ambiguous {
        identity: "virtual/kernel".into(),
        candidates: vec!["linux-yocto".into(), "linux-vendor".into()],
    };
    assert!(matches!(
        ambiguous,
        ProjectIdentityResolution::Ambiguous { .. }
    ));
}
