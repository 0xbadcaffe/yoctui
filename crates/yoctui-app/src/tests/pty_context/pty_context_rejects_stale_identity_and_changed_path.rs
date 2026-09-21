use super::*;

#[test]
fn pty_context_rejects_stale_identity_and_changed_path() {
    let (root, authority) = fixture();
    assert!(matches!(
        authority.resolve(PtyContextAction::SelectedLayer {
            identity: "removed".into()
        }),
        Err(PtyContextError::StaleIdentity { kind: "layer", .. })
    ));
    fs::remove_dir_all(root.join("layer")).unwrap();
    assert!(matches!(
        authority.resolve(PtyContextAction::SelectedLayer {
            identity: "meta-test".into()
        }),
        Err(PtyContextError::Path { .. })
    ));
    fs::remove_dir_all(root).unwrap();
}
