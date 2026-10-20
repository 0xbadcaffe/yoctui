use super::*;
use yoctui_model::{
    KernelDebugDefaultContext, KernelDebugDraft, KernelDebugTool, KernelDebugTools,
};

#[test]
fn remote_source_map_is_selected_build_only_and_revalidated_before_review() {
    let (_root, spec) = fixture();
    let context = KernelDebugDefaultContext {
        build_dir: spec.build_dir.clone(),
        machine: "qemux86-64".into(),
        image: None,
    };
    let source = spec
        .build_dir
        .join("tmp/work-shared/qemux86-64/kernel-source");
    let build = spec.build_dir.join(
        "tmp/work/qemux86_64-poky-linux/linux-yocto/7.2.8+git/linux-qemux86_64-standard-build",
    );
    fs::create_dir_all(source.join("init")).unwrap();
    fs::create_dir_all(build.join("include/generated")).unwrap();
    fs::write(source.join("init/main.c"), "void start_kernel(void) {}\n").unwrap();
    fs::write(
        build.join("include/generated/autoconf.h"),
        "#define CONFIG_DEBUG_INFO 1\n",
    )
    .unwrap();
    fs::copy(&spec.symbols, build.join("vmlinux")).unwrap();
    let mapping = crate::qemu_debug::remote_source_map(&context, &build.join("vmlinux")).unwrap();
    let mut draft = KernelDebugDraft::new(KernelDebugTool::GdbRemote);
    draft.symbols = mapping.symbols.display().to_string();
    draft.source_map = Some(Box::new(mapping));
    let tools = KernelDebugTools {
        cwd: spec.build_dir.clone(),
        programs: [("gdb".into(), spec.gdb)].into(),
    };
    let request = crate::kernel_debug_io::prepare(&draft, &tools).unwrap();
    assert_eq!(request, draft.plan(&tools).unwrap());
    assert!(
        request
            .arguments
            .contains(&format!("directory {}", source.display()))
    );
    let mut wrong = context.clone();
    wrong.machine = "another-machine".into();
    assert!(crate::qemu_debug::remote_source_map(&wrong, &build.join("vmlinux")).is_none());
    assert!(crate::qemu_debug::remote_source_map(&context, &spec.symbols).is_none());
    fs::remove_file(build.join("include/generated/autoconf.h")).unwrap();
    assert!(
        crate::kernel_debug_io::prepare(&draft, &tools)
            .unwrap_err()
            .contains("mapping changed")
    );
    // An explicitly changed symbols path must not inherit the stale map.
    draft.symbols = spec.symbols.display().to_string();
    assert!(
        !crate::kernel_debug_io::prepare(&draft, &tools)
            .unwrap()
            .arguments
            .iter()
            .any(|value| value.starts_with("set substitute-path"))
    );
}

#[cfg(unix)]
#[test]
fn remote_source_map_rejects_retained_source_symlinks_outside_selected_build() {
    let (_root, spec) = fixture();
    let outside = TestDir::new();
    fs::create_dir(outside.path().join("init")).unwrap();
    fs::write(outside.path().join("init/main.c"), "other source\n").unwrap();
    let shared = spec.build_dir.join("tmp/work-shared/qemux86-64");
    fs::create_dir_all(&shared).unwrap();
    std::os::unix::fs::symlink(outside.path(), shared.join("kernel-source")).unwrap();
    let context = KernelDebugDefaultContext {
        build_dir: spec.build_dir.clone(),
        machine: "qemux86-64".into(),
        image: None,
    };
    assert!(crate::qemu_debug::remote_source_map(&context, &spec.symbols).is_none());
}
