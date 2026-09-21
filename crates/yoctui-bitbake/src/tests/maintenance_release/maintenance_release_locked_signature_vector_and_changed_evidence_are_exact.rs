use super::*;

#[test]
fn maintenance_release_locked_signature_vector_and_changed_evidence_are_exact() {
    let fixture = TestDirectory::new("locked");
    prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
    let snapshot = inspect(&fixture);
    let request = locked_request(&fixture);
    let (preview, command, before) =
        locked_signature_command(MaintenanceSessionId(1), 2, &snapshot, 3, request.clone())
            .unwrap();
    assert_eq!(
        command.kind(),
        MaintenanceSstateCommandKind::LockedSignatureCache
    );
    assert_eq!(
        command.arguments(),
        [
            request.locked_signatures.as_os_str().to_owned(),
            request.input_cache.as_os_str().to_owned(),
            request.output_cache.as_os_str().to_owned(),
            OsString::from("ubuntu-24.04"),
            request.filter.unwrap().as_os_str().to_owned(),
        ]
    );
    assert!(
        preview
            .limitations
            .iter()
            .any(|line| line.contains("may be replaced"))
    );
    let created = fixture.join("output-cache/aa/new.siginfo");
    fs::create_dir_all(created.parent().unwrap()).unwrap();
    fs::write(&created, "sig\n").unwrap();
    let evidence = before.changed_evidence().unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].identity.path, created);
}
