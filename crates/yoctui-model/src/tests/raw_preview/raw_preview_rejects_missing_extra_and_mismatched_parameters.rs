use super::*;

#[test]
fn raw_preview_rejects_missing_extra_and_mismatched_parameters() {
    let catalog = catalog();
    let authority = authority(9, true);

    let mut missing = request();
    missing.parameters.remove(&id("task"));
    assert_eq!(
        catalog.preview(&missing, Some(&authority)),
        Err(RawPreviewError::MissingParameter(id("task")))
    );

    let mut extra = request();
    extra
        .parameters
        .insert(id("other"), RawParameterValue::Text("ordinary".into()));
    assert_eq!(
        catalog.preview(&extra, Some(&authority)),
        Err(RawPreviewError::UnknownParameter(id("other")))
    );

    let mut mismatch = request();
    mismatch
        .parameters
        .insert(id("task"), RawParameterValue::Target("busybox".into()));
    assert!(matches!(
        catalog.preview(&mismatch, Some(&authority)),
        Err(RawPreviewError::InvalidParameter(
            RawParameterError::KindMismatch { .. }
        ))
    ));
}
