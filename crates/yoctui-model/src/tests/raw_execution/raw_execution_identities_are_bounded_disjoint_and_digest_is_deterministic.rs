use super::*;

#[test]
fn raw_execution_identities_are_bounded_disjoint_and_digest_is_deterministic() {
    assert!(RawRequestId::new("raw-request:one").is_ok());
    assert!(RawRequestId::new("raw-job:one").is_err());
    assert!(RawJobId::new("raw-request:one").is_err());
    assert!(RawSessionId::new("raw-session:").is_err());
    assert!(RawStreamId::new("raw-stream:bad/token").is_err());
    assert!(
        RawDurableReferenceId::new(format!(
            "raw-durable:{}",
            "x".repeat(MAX_RAW_EXECUTION_ID_BYTES)
        ))
        .is_err()
    );

    let (catalog, request, preview) = request_and_preview(RawInteractionMode::NoninteractiveJob);
    let first = RawConfirmedExecutionRequest::from_reviewed_preview(
        RawRequestId::new("raw-request:one").unwrap(),
        &catalog,
        &request,
        &preview,
    )
    .unwrap();
    let second = RawPreviewDigest::from_preview(&preview);
    assert_eq!(first.preview_digest, second);
    assert_eq!(
        RawPreviewDigest::from_hex(&second.to_hex()).unwrap(),
        second
    );
    assert_eq!(second.to_hex().len(), 64);
    assert!(!second.to_hex().contains("bitbake"));

    let mut forged = preview.clone();
    forged.arguments[0] = "forged".into();
    assert_eq!(
        RawConfirmedExecutionRequest::from_reviewed_preview(
            RawRequestId::new("raw-request:forged").unwrap(),
            &catalog,
            &request,
            &forged,
        ),
        Err(RawExecutionError::PreviewRequestMismatch)
    );

    let mut mismatched = preview;
    mismatched.capability_generation += 1;
    assert_eq!(
        RawConfirmedExecutionRequest::from_reviewed_preview(
            RawRequestId::new("raw-request:two").unwrap(),
            &catalog,
            &request,
            &mismatched,
        ),
        Err(RawExecutionError::PreviewRequestMismatch)
    );
}
