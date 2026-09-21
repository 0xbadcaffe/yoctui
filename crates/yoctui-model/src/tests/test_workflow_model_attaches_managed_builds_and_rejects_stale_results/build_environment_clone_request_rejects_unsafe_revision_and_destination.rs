use super::*;

#[test]
fn build_environment_clone_request_rejects_unsafe_revision_and_destination() {
    let mut request = BuildEnvironmentCloneRequest {
        repository: "https://example.invalid/poky".into(),
        destination: PathBuf::from("/workspace/poky"),
        revision: Some("main;rm -rf".into()),
    };
    assert!(request.validate().is_err());
    request.revision = Some("scarthgap".into());
    request.destination = PathBuf::from("relative/poky");
    assert!(request.validate().is_err());
}
