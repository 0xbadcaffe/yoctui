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
            OsString::from("qemumemory=1024"),
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
