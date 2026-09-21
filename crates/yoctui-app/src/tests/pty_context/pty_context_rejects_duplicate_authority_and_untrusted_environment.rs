use super::*;

#[test]
fn pty_context_rejects_duplicate_authority_and_untrusted_environment() {
    let (root, _) = fixture();
    let shell = fs::canonicalize("/bin/sh").unwrap();
    let environment = VerifiedPtyEnvironment {
        identity: "build".into(),
        shell,
        environment: BTreeMap::from([("BAD=NAME".into(), "value".into())]),
    };
    let entry = PtyContextEntry {
        identity: "same".into(),
        directory: root.join("layer"),
    };
    assert!(matches!(
        PtyContextAuthority::new(
            "workspace".into(),
            root.join("source"),
            root.join("build"),
            environment,
            vec![entry.clone(), entry],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new()
        ),
        Err(PtyContextError::InvalidEnvironment(_))
    ));
    let valid_environment = VerifiedPtyEnvironment {
        identity: "build".into(),
        shell: fs::canonicalize("/bin/sh").unwrap(),
        environment: BTreeMap::new(),
    };
    let entry = PtyContextEntry {
        identity: "same".into(),
        directory: root.join("layer"),
    };
    assert!(matches!(
        PtyContextAuthority::new(
            "workspace".into(),
            root.join("source"),
            root.join("build"),
            valid_environment,
            vec![entry.clone(), entry],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new()
        ),
        Err(PtyContextError::DuplicateIdentity(_))
    ));
    fs::remove_dir_all(root).unwrap();
}
