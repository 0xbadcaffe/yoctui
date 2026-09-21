use super::*;

#[test]
fn utility_runner_preview_is_indexed_and_cwd_bounded() {
    let spec = UtilityCommandSpec::new(
        "/usr/bin/bitbake",
        "/tmp/build",
        vec!["core-image-minimal".into()],
        UtilityRisk::ReadOnly,
    )
    .unwrap();
    assert_eq!(spec.indexed_argv()[1], "[1] core-image-minimal");
    assert!(spec.validate_cwd(Path::new("/tmp/build")).is_ok());
    assert!(spec.validate_cwd(Path::new("/tmp/other")).is_err());
}
