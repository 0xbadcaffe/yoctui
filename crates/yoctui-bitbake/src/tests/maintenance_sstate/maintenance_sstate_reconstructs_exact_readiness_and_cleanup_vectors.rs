use super::*;

#[test]
fn maintenance_sstate_reconstructs_exact_readiness_and_cleanup_vectors() {
    let (root, snapshot) = fixture(
        "vectors",
        "sstate-cache-management.py",
        "#!/bin/sh\nexit 0\n",
    );
    let output = root.0.join("output/readiness.txt");
    let (preview, command) = MaintenanceSstateCommandSpec::readiness(
        MaintenanceSessionId(1),
        3,
        &snapshot,
        9,
        SstateReadinessRequest::new(
            vec!["busybox".into(), "core-image-minimal".into()],
            SstateReadinessMode::SameTmpdir,
            Some(output.clone()),
            None,
            60,
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        command.arguments(),
        &[
            OsString::from("--outfile"),
            output.into_os_string(),
            OsString::from("--same-tmpdir"),
            OsString::from("busybox"),
            OsString::from("core-image-minimal"),
        ]
    );
    assert_eq!(
        command.environment().get(OsStr::new("BB_SETSCENE_ENFORCE")),
        Some(&OsString::from("1"))
    );
    assert_eq!(
        preview.arguments[0],
        format!(
            "0: {}",
            snapshot
                .capability(MaintenanceTool::OeCheckSstate)
                .and_then(|capability| match capability {
                    MaintenanceToolCapability::Available { executable, .. } =>
                        Some(executable.path.display().to_string()),
                    _ => None,
                })
                .unwrap()
        )
    );

    let cleanup = MaintenanceSstateCommandSpec::cleanup_preview(
        MaintenanceSessionId(2),
        &snapshot,
        cleanup_request(&root),
    )
    .unwrap();
    let arguments = cleanup
        .arguments()
        .iter()
        .map(|value| value.to_string_lossy())
        .collect::<Vec<_>>();
    assert!(arguments.contains(&std::borrow::Cow::Borrowed("--remove-duplicated")));
    assert!(arguments.contains(&std::borrow::Cow::Borrowed("--remove-orphans")));
    assert!(arguments.contains(&std::borrow::Cow::Borrowed("--stamps-dir")));
    assert_eq!(
        arguments.last().map(|value| value.as_ref()),
        Some("--debug")
    );
}
