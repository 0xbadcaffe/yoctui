use super::*;

#[test]
fn pty_session_validates_identity_context_and_dimensions() {
    let session = PtySession::new(spec(), 431).unwrap();
    assert_eq!(session.lifecycle, PtySessionLifecycle::Starting);
    let mut invalid = spec();
    invalid.cwd = "/tmp".into();
    assert_eq!(
        PtySession::new(invalid, 431),
        Err(PtySessionError::CwdOutsideWorkspace)
    );
    let mut invalid = spec();
    invalid.dimensions.columns = 0;
    assert!(matches!(
        PtySession::new(invalid, 431),
        Err(PtySessionError::InvalidDimensions(_))
    ));
}
