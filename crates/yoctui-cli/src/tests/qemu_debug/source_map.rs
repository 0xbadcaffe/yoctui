use super::*;

#[test]
fn qemu_debug_source_mapping_includes_matching_generated_headers_and_compilation_directory() {
    let (_root, mut spec) = fixture();
    let deploy = spec.build_dir.join("tmp/deploy/images/qemux86-64");
    let source = spec
        .build_dir
        .join("tmp/work-shared/qemux86-64/kernel-source");
    let build = spec.build_dir.join(
        "tmp/work/qemux86_64-poky-linux/linux-yocto/7.2.8+git/linux-qemux86_64-standard-build",
    );
    fs::create_dir_all(&deploy).unwrap();
    fs::create_dir_all(source.join("init")).unwrap();
    fs::create_dir_all(build.join("include/generated")).unwrap();
    fs::write(source.join("init/main.c"), "void start_kernel(void) {}\n").unwrap();
    fs::write(
        build.join("include/generated/autoconf.h"),
        "#define CONFIG_DEBUG_INFO 1\n",
    )
    .unwrap();
    fs::copy(&spec.symbols, build.join("vmlinux")).unwrap();
    fs::copy(&spec.qemuboot, deploy.join("image.qemuboot.conf")).unwrap();
    spec.qemuboot = deploy.join("image.qemuboot.conf");
    spec.symbols = build.join("vmlinux");
    let arguments =
        crate::qemu_debug::source_map::arguments(&spec, &spec.build_dir.join("gdb.sock"));
    assert!(arguments.contains(&format!(
        "set substitute-path /usr/src/debug/linux-yocto/7.2.8+git {}",
        build.display()
    )));
    assert!(arguments.contains(&format!("directory {}", source.display())));
    assert!(arguments.contains(&format!("directory {}", build.display())));
    fs::remove_file(build.join("include/generated/autoconf.h")).unwrap();
    assert!(
        !crate::qemu_debug::source_map::arguments(&spec, &spec.build_dir.join("gdb.sock"))
            .iter()
            .any(|arg| arg.starts_with("set substitute-path /usr/src/debug/"))
    );
}

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
