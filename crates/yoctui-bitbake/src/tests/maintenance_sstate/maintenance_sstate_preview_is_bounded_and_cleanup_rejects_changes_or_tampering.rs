use super::*;

#[test]
fn maintenance_sstate_preview_is_bounded_and_cleanup_rejects_changes_or_tampering() {
    let (root, snapshot) = fixture(
        "preview",
        "sstate-cache-management.py",
        "#!/bin/sh\nexit 0\n",
    );
    let candidate = root.0.join("cache/sstate:a");
    fs::write(&candidate, "one").unwrap();
    let request = cleanup_request(&root);
    let confirmed =
        parse_cleanup_preview(request.clone(), &[candidate.display().to_string()]).unwrap();
    assert_eq!(confirmed.candidates.len(), 1);
    assert!(
        parse_cleanup_preview(request.clone(), &[])
            .unwrap()
            .candidates
            .is_empty()
    );
    let (execution_preview, execution) = MaintenanceSstateCommandSpec::cleanup_execution(
        MaintenanceSessionId(3),
        1,
        &snapshot,
        3,
        &confirmed,
        &confirmed,
    )
    .unwrap();
    assert_eq!(execution.arguments().last(), Some(&OsString::from("--yes")));
    assert!(
        !execution
            .arguments()
            .iter()
            .any(|argument| argument == OsStr::new("--debug"))
    );
    assert_eq!(
        execution_preview.operation,
        MaintenanceOperation::SstateCleanup(confirmed.clone())
    );
    let changed = SstateCleanupPreview::new(request.clone(), vec![]).unwrap();
    assert!(matches!(
        MaintenanceSstateCommandSpec::cleanup_execution(
            MaintenanceSessionId(3),
            1,
            &snapshot,
            3,
            &confirmed,
            &changed,
        ),
        Err(MaintenanceSstateAdapterError::CandidateMismatch)
    ));
    fs::write(&candidate, "changed").unwrap();
    assert!(matches!(
        MaintenanceSstateCommandSpec::cleanup_execution(
            MaintenanceSessionId(3),
            1,
            &snapshot,
            3,
            &confirmed,
            &confirmed,
        ),
        Err(MaintenanceSstateAdapterError::StaleIdentity(path)) if path == candidate
    ));
    assert!(
        parse_cleanup_preview(
            request,
            &[format!("/outside/{}", "x".repeat(MAX_PREVIEW_OUTPUT_BYTES))]
        )
        .is_err()
    );
}
