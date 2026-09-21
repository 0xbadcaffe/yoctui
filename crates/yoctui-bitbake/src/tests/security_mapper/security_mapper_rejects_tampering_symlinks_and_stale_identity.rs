use super::*;

#[tokio::test]
async fn security_mapper_rejects_tampering_symlinks_and_stale_identity() {
    let directory = TestDirectory::new("validation");
    let mut tampered = preview(&directory);
    tampered.indexed_arguments.push("2: injected".into());
    assert_eq!(
        SecurityMapperCommandSpec::from_preview(&tampered),
        Err(SecurityMapperAdapterError::PreviewMismatch)
    );

    let safe = preview(&directory);
    let SecurityOperation::PackageMap {
        executable: tool, ..
    } = &safe.operation
    else {
        unreachable!();
    };
    let linked = directory.path().join("linked");
    symlink(tool, &linked).unwrap();
    let mut linked_preview = safe.clone();
    let SecurityOperation::PackageMap { executable, .. } = &mut linked_preview.operation else {
        unreachable!();
    };
    *executable = linked.clone();
    linked_preview.indexed_arguments[0] = format!("0: {}", linked.display());
    assert!(matches!(
        SecurityMapperCommandSpec::from_preview(&linked_preview),
        Err(SecurityMapperAdapterError::UnsafeExecutable(_))
    ));

    let linked_reports = directory.path().join("linked-reports");
    symlink(&safe.report_roots[0], &linked_reports).unwrap();
    let mut linked_input = safe.clone();
    linked_input.report_roots = vec![linked_reports.clone()];
    let SecurityOperation::PackageMap { arguments, .. } = &mut linked_input.operation else {
        unreachable!();
    };
    *arguments = vec![linked_reports.display().to_string()];
    let SecurityOperation::PackageMap {
        executable,
        arguments,
    } = &linked_input.operation
    else {
        unreachable!();
    };
    linked_input.indexed_arguments = indexed_arguments(executable, arguments);
    assert!(matches!(
        SecurityMapperCommandSpec::from_preview(&linked_input),
        Err(SecurityMapperAdapterError::UnsafeInput(_))
    ));

    let command = SecurityMapperCommandSpec::from_preview(&safe).unwrap();
    write_executable(tool, "#!/bin/sh\nprintf 'changed\\n'\n");
    assert!(matches!(
        SecurityMapperJobRunner::new().start(command).await,
        Err(SecurityMapperAdapterError::StaleIdentity(_))
    ));
}
