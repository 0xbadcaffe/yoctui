use super::*;

fn mapping() -> crate::KernelDebugSourceMap {
    let build = std::path::PathBuf::from(
        "/work/tmp/work/qemux86_64-poky-linux/linux-yocto/7.2.8+git/linux-qemux86_64-standard-build",
    );
    crate::KernelDebugSourceMap {
        build_dir: "/work".into(),
        machine: "qemux86-64".into(),
        symbols: build.join("vmlinux"),
        kernel_source: "/work/tmp/work-shared/qemux86-64/kernel-source".into(),
        kernel_build: build,
    }
}

#[test]
fn remote_gdb_source_mapping_is_typed_reviewable_and_bound_to_exact_symbols() {
    let map = mapping();
    let mut draft = KernelDebugDraft::new(KernelDebugTool::GdbRemote);
    draft.symbols = map.symbols.display().to_string();
    draft.source_map = Some(Box::new(map.clone()));
    let request = draft.plan(&tools()).unwrap();
    assert!(request.arguments.contains(&format!(
        "set substitute-path /usr/src/kernel {}",
        map.kernel_source.display()
    )));
    assert!(request.arguments.contains(&format!(
        "set substitute-path /usr/src/debug/linux-yocto/7.2.8+git {}",
        map.kernel_build.display()
    )));
    draft.symbols = "/different-build/vmlinux".into();
    assert!(
        !draft
            .plan(&tools())
            .unwrap()
            .arguments
            .iter()
            .any(|value| value.starts_with("set substitute-path"))
    );
}

#[test]
fn remote_gdb_source_mapping_rejects_gdb_command_syntax_and_build_escape() {
    let mut map = mapping();
    let symbols = map.symbols.clone();
    for bad in [
        "/work/source with spaces",
        "/work/source\nquit",
        "/work/../escape",
        "/outside/source",
    ] {
        map.kernel_source = bad.into();
        assert!(map.arguments(&symbols).is_err(), "{bad}");
    }
    let mut map = mapping();
    map.kernel_build = "/outside/build".into();
    assert!(map.arguments(&symbols).is_err());
}
