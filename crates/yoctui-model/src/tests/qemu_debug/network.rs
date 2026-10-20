use super::*;

#[test]
fn private_ssh_is_opt_in_and_legacy_specs_remain_network_isolated() {
    let spec = spec();
    let encoded = serde_json::to_value(&spec).unwrap();
    assert!(encoded.get("ssh_forward_port").is_none());
    let restored: QemuDebugSpec = serde_json::from_value(encoded).unwrap();
    assert_eq!(restored.ssh_forward_port, None);
    assert_eq!(
        restored.qemu_arguments(Path::new(QEMU_DEBUG_SOCKET_TEMPLATE)),
        spec.qemu_arguments(Path::new(QEMU_DEBUG_SOCKET_TEMPLATE))
    );
    assert!(
        !restored
            .qemu_arguments(Path::new(QEMU_DEBUG_SOCKET_TEMPLATE))
            .join(" ")
            .contains("hostfwd")
    );
}

#[test]
fn private_ssh_plan_exposes_only_one_loopback_forward_and_keeps_gdb_private() {
    let mut spec = spec();
    spec.ssh_forward_port = Some(2245);
    spec.validate().unwrap();
    let arguments = spec.qemu_arguments(Path::new(QEMU_DEBUG_SOCKET_TEMPLATE));
    assert!(arguments.iter().any(|arg| arg == "nonetwork"));
    assert!(arguments.iter().any(|arg| arg == "snapshot"));
    assert!(
        arguments
            .iter()
            .any(|arg| arg == "bootparams=nokaslr ip=dhcp")
    );
    let parameters = arguments
        .iter()
        .find(|arg| arg.starts_with("qemuparams="))
        .unwrap();
    assert!(parameters.contains("restrict=on,hostfwd=tcp:127.0.0.1:2245-:22"));
    assert!(parameters.contains("-gdb chardev:yoctui_gdb"));
    assert!(parameters.contains("-device virtio-net-pci,netdev=yoctui_debug_net"));
    assert_eq!(parameters.matches("hostfwd=").count(), 1);
    for forbidden in [
        "0.0.0.0",
        "tftp=",
        "smb=",
        "guestfwd=",
        "-gdb tcp:",
        "-netdev tap",
    ] {
        assert!(!arguments.join(" ").contains(forbidden), "{forbidden}");
    }
    let restored: QemuDebugSpec =
        serde_json::from_str(&serde_json::to_string(&spec).unwrap()).unwrap();
    assert_eq!(restored, spec);
}

#[test]
fn private_ssh_rejects_privileged_ports_flash_and_untyped_inputs() {
    let mut spec = spec();
    for port in [0, 22, 1023] {
        spec.ssh_forward_port = Some(port);
        assert!(spec.validate().is_err());
    }
    for port in [1024, 65535] {
        spec.ssh_forward_port = Some(port);
        spec.validate().unwrap();
    }
    spec.rootfs = "/build/image.static.mtd".into();
    spec.boot_mode = QemuDebugBootMode::OpenBmcRomulusFlash;
    assert!(spec.validate().is_err());
    let mut encoded = serde_json::to_value(&spec).unwrap();
    for value in [
        serde_json::json!("2245 -netdev tap"),
        serde_json::json!(65536),
        serde_json::json!(-1),
    ] {
        encoded["ssh_forward_port"] = value;
        assert!(serde_json::from_value::<QemuDebugSpec>(encoded.clone()).is_err());
    }
}
