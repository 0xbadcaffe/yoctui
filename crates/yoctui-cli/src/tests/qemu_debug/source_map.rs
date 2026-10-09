use super::*;

#[test]
fn qemu_debug_source_mapping_requires_the_selected_deploy_machine_and_retained_source() {
    let (_root, mut spec) = fixture();
    assert!(crate::qemu_debug::source_map::discover(&spec).is_none());
    let deploy = spec.build_dir.join("tmp/deploy/images/romulus");
    let source = spec.build_dir.join("tmp/work-shared/romulus/kernel-source");
    fs::create_dir_all(&deploy).unwrap();
    fs::create_dir_all(source.join("init")).unwrap();
    fs::write(source.join("init/main.c"), "void start_kernel(void) {}\n").unwrap();
    fs::copy(&spec.qemuboot, deploy.join("image.qemuboot.conf")).unwrap();
    spec.qemuboot = deploy.join("image.qemuboot.conf");
    let expected = source.canonicalize().unwrap();
    assert_eq!(
        crate::qemu_debug::source_map::discover(&spec),
        Some(expected.clone())
    );
    let arguments =
        crate::qemu_debug::source_map::arguments(&spec, &spec.build_dir.join("gdb.sock"));
    assert!(arguments.windows(2).any(|pair| pair
        == [
            "-iex",
            &format!("set substitute-path /usr/src/kernel {}", expected.display())
        ]));
    let other = spec.build_dir.join("tmp/deploy/images/another-machine");
    fs::create_dir_all(&other).unwrap();
    fs::copy(&spec.qemuboot, other.join("image.qemuboot.conf")).unwrap();
    spec.qemuboot = other.join("image.qemuboot.conf");
    assert!(crate::qemu_debug::source_map::discover(&spec).is_none());
}

#[cfg(unix)]
#[test]
fn qemu_debug_source_mapping_does_not_follow_sources_outside_the_build() {
    let (_root, mut spec) = fixture();
    let outside = TestDir::new();
    fs::create_dir(outside.path().join("init")).unwrap();
    fs::write(outside.path().join("init/main.c"), "other kernel\n").unwrap();
    let deploy = spec.build_dir.join("tmp/deploy/images/romulus");
    fs::create_dir_all(&deploy).unwrap();
    fs::copy(&spec.qemuboot, deploy.join("image.qemuboot.conf")).unwrap();
    spec.qemuboot = deploy.join("image.qemuboot.conf");
    let shared = spec.build_dir.join("tmp/work-shared/romulus");
    fs::create_dir_all(&shared).unwrap();
    std::os::unix::fs::symlink(outside.path(), shared.join("kernel-source")).unwrap();
    assert!(crate::qemu_debug::source_map::discover(&spec).is_none());
    assert!(
        !crate::qemu_debug::source_map::arguments(&spec, &spec.build_dir.join("gdb.sock"))
            .iter()
            .any(|argument| argument.starts_with("set substitute-path"))
    );
}
