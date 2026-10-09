use super::*;

#[test]
fn qemu_adapter_builds_exact_shell_free_arguments_and_rejects_tampering() {
    let (directory, mut preview, command) = fixture_preview("command", "printf '%s\\n' \"$@\"");
    assert_eq!(command.executable(), directory.join("runqemu"));
    assert_eq!(
        command.arguments(),
        [
            OsString::from("qemux86-64"),
            directory
                .join("qemux86-64/core-image-minimal.wic")
                .as_os_str()
                .to_owned(),
            OsString::from("qemuparams=-m 1024"),
            OsString::from("slirp"),
            OsString::from("sdl"),
            OsString::from("serialstdio"),
        ]
    );
    preview.argv.push("--help".into());
    assert_eq!(
        QemuCommandSpec::from_preview(&preview),
        Err(QemuAdapterError::PreviewMismatch)
    );
    preview.argv.pop();
    preview.request.extra_arguments = vec!["--help".into()];
    assert!(matches!(
        QemuCommandSpec::from_preview(&preview),
        Err(QemuAdapterError::InvalidRequest(_))
    ));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn qemu_adapter_openbmc_flash_preview_matches_pinned_snapshot_command() {
    let directory = fixture_dir("flash");
    let program = directory.join("runqemu");
    executable(&program, "exit 0");
    let deploy = directory.join("romulus");
    fs::create_dir(&deploy).unwrap();
    let path = deploy.join("obmc-phosphor-image-romulus-20261009022649.static.mtd");
    fs::write(&path, b"flash").unwrap();
    let mut image = artifact(path, ImageArtifactKind::RootFilesystem);
    image.identity.machine = "romulus".into();
    image.identity.image = "obmc-phosphor-image".into();
    let capability =
        QemuCapabilityInspector::with_executable(program).inspect(std::slice::from_ref(&image));
    let draft = QemuLaunchDraft::for_artifact(image.identity, image.kind);
    let mut preview = draft.preview(&capability).unwrap();
    let command = QemuCommandSpec::from_preview(&preview).unwrap();
    assert_eq!(command.arguments()[2], OsString::from("snapshot"));
    assert!(
        command.arguments()[1]
            .to_string_lossy()
            .ends_with("20261009022649.qemuboot.conf")
    );
    preview.argv.remove(3);
    assert_eq!(
        QemuCommandSpec::from_preview(&preview),
        Err(QemuAdapterError::PreviewMismatch)
    );
    fs::remove_dir_all(directory).unwrap();
}
