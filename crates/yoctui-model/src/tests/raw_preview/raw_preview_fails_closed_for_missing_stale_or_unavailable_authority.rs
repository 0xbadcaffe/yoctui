use super::*;

#[test]
fn raw_preview_fails_closed_for_missing_stale_or_unavailable_authority() {
    let catalog = catalog();
    let request = request();
    assert_eq!(
        catalog.preview(&request, None),
        Err(RawPreviewError::MissingAuthority)
    );
    assert_eq!(
        catalog.preview(&request, Some(&authority(10, true))),
        Err(RawPreviewError::StaleCapabilityGeneration {
            current: 10,
            received: 9,
        })
    );
    assert!(matches!(
        catalog.preview(&request, Some(&authority(9, false))),
        Err(RawPreviewError::CapabilityUnavailable {
            state: RawAvailabilityState::Unavailable,
            ..
        })
    ));

    let mut stale_directory = request;
    stale_directory.build_directory = "/other/build".into();
    assert!(matches!(
        catalog.preview(&stale_directory, Some(&authority(9, true))),
        Err(RawPreviewError::StaleBuildDirectory { .. })
    ));
}
