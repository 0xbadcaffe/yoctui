use super::*;

fn spec() -> QemuDebugSpec {
    QemuDebugSpec {
        boot_mode: QemuDebugBootMode::DirectKernel,
        runqemu: "/tools/runqemu".into(),
        gdb: "/tools/gdb".into(),
        build_dir: "/build".into(),
        qemuboot: "/build/image.qemuboot.conf".into(),
        kernel: "/build/bzImage".into(),
        rootfs: "/build/image.ext4".into(),
        symbols: "/build/vmlinux".into(),
        memory_mib: 1024,
    }
}

#[test]
fn qemu_debug_flash_plan_omits_kernel_and_bootparams_and_stages_image() {
    let mut spec = spec();
    let mut legacy = serde_json::to_value(&spec).unwrap();
    legacy.as_object_mut().unwrap().remove("boot_mode");
    assert_eq!(
        serde_json::from_value::<QemuDebugSpec>(legacy)
            .unwrap()
            .boot_mode,
        QemuDebugBootMode::DirectKernel
    );
    spec.boot_mode = QemuDebugBootMode::OpenBmcRomulusFlash;
    assert!(spec.validate().is_err());
    spec.rootfs = "/build/romulus.static.mtd".into();
    spec.validate().unwrap();
    let args = spec.qemu_arguments(Path::new(QEMU_DEBUG_SOCKET_TEMPLATE));
    assert_eq!(args[0], "/PRIVATE_SESSION/romulus.static.mtd");
    assert_eq!(args[1], "/build/image.qemuboot.conf");
    assert!(
        !args
            .iter()
            .any(|arg| arg == "/build/bzImage" || arg.starts_with("bootparams="))
    );
    for arg in ["nonetwork", "snapshot", "nographic", "serialstdio"] {
        assert!(args.iter().any(|value| value == arg));
    }
    assert!(args.last().unwrap().contains("-S -m 1024"));
    assert!(!args.join(" ").contains("tcp:"));
    let mut encoded = serde_json::to_value(&spec).unwrap();
    encoded["boot_mode"] = serde_json::json!("UnreviewedFirmware");
    assert!(serde_json::from_value::<QemuDebugSpec>(encoded).is_err());
}

#[test]
fn qemu_debug_closed_plan_pauses_and_isolates_guest_without_public_listener() {
    let spec = spec();
    let args = spec.qemu_arguments(Path::new(QEMU_DEBUG_SOCKET_TEMPLATE));
    for value in [
        "snapshot",
        "nonetwork",
        "nographic",
        "serialstdio",
        "bootparams=nokaslr",
    ] {
        assert!(args.iter().any(|arg| arg == value));
    }
    assert!(args[7].starts_with("qemuparams=-S -m 1024 -chardev socket,path=/PRIVATE_SESSION/"));
    assert_eq!(args[2], spec.qemuboot.display().to_string());
    assert!(!args.iter().any(|arg| arg.starts_with("qemumemory=")));
    assert!(!args.join(" ").contains("tcp:"));
    let gdb = spec.gdb_arguments(Path::new(QEMU_DEBUG_SOCKET_TEMPLATE));
    assert!(gdb.iter().any(|arg| arg == "set debuginfod enabled off"));
    assert!(
        gdb.iter()
            .any(|arg| arg == "set auto-connect-native-target off")
    );
    assert_eq!(
        gdb.last().unwrap(),
        "target remote /PRIVATE_SESSION/gdb.sock"
    );
    assert!(!gdb.iter().any(|arg| arg == "continue" || arg == "run"));
    let request = spec.terminal_request("/tools/yoctui".into()).unwrap();
    assert_eq!(request.arguments[0], "__qemu-gdb-session");
    assert_eq!(
        serde_json::from_str::<QemuDebugSpec>(&request.arguments[2]).unwrap(),
        spec
    );
}

#[test]
fn qemu_debug_rejects_ambiguous_paths_memory_and_unstructured_arguments() {
    let mut spec = spec();
    for value in [
        "relative",
        "/build/../other",
        "/build/image;reboot",
        "/build/image space",
        "/build/image,foo",
    ] {
        spec.rootfs = value.into();
        assert!(spec.validate().is_err(), "{value}");
    }
    spec.rootfs = "/build/image.ext4".into();
    for value in [0, 127, 262145] {
        spec.memory_mib = value;
        assert!(spec.validate().is_err());
    }
    spec.memory_mib = 1024;
    spec.symbols = "/build/symbols with spaces".into();
    assert!(spec.validate().is_ok()); // passed as one GDB argv, not runqemu syntax
    spec.rootfs = "/build/core-image.rootfs.ext4.zst".into();
    assert_eq!(
        spec.qemu_arguments(Path::new(QEMU_DEBUG_SOCKET_TEMPLATE))[1],
        "/PRIVATE_SESSION/core-image.rootfs.ext4.zst"
    );
    let mut json = serde_json::to_value(&spec).unwrap();
    json["extra_arguments"] = serde_json::json!(["-daemonize"]);
    assert!(serde_json::from_value::<QemuDebugSpec>(json).is_err());
}
