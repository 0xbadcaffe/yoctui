use super::*;

#[test]
fn qemu_adapter_rejects_symlinked_artifacts() {
    use std::os::unix::fs::symlink;
    let directory = fixture_dir("symlink");
    let program = directory.join("runqemu");
    executable(&program, "exit 0");
    let deploy = directory.join("qemux86-64");
    fs::create_dir(&deploy).unwrap();
    let target = deploy.join("target.wic");
    let link = deploy.join("core-image-minimal.wic");
    fs::write(&target, b"wic").unwrap();
    symlink(&target, &link).unwrap();
    let image = artifact(link, ImageArtifactKind::Wic);
    assert!(matches!(
        QemuCapabilityInspector::with_executable(program).inspect(&[image]),
        QemuCapability::Failed { message } if message.contains("non-symlink")
    ));
    fs::remove_dir_all(directory).unwrap();
}
