use super::*;

#[test]
fn request_validation() {
    assert!(
        BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        }
        .validate()
        .is_ok()
    );
    assert!(
        BuildRequest {
            targets: vec!["bad target".into()],
            task: None,
            force: false,
        }
        .validate()
        .is_err()
    );
    assert!(
        BuildRequest {
            targets: vec!["..".into()],
            task: None,
            force: false,
        }
        .validate()
        .is_err()
    );
}
