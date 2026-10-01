use super::*;

fn spec() -> KgdbSerialSpec {
    KgdbSerialSpec {
        gdb: "/usr/bin/gdb".into(),
        cwd: "/tmp".into(),
        symbols: "/tmp/kernel build/vmlinux".into(),
        config: "/tmp/kernel build/.config".into(),
        device: "/dev/ttyUSB0".into(),
        baud: 115200,
        target_uart: "ttyAMA0".into(),
        ready: true,
    }
}

#[test]
fn kgdb_serial_closed_plan_is_host_client_with_explicit_board_transport() {
    let spec = spec();
    let request = spec.terminal_request("/opt/yoctui".into()).unwrap();
    assert_eq!(request.arguments[0], "__kgdb-serial-session");
    assert_eq!(
        serde_json::from_str::<KgdbSerialSpec>(&request.arguments[2]).unwrap(),
        spec
    );
    let arguments = spec.gdb_arguments();
    for required in [
        "-nx",
        "-nh",
        "set auto-load off",
        "set debuginfod enabled off",
        "set auto-connect-native-target off",
        "set serial baud 115200",
        "set remotetimeout 10",
        "target remote /dev/ttyUSB0",
        "--symbols=/tmp/kernel build/vmlinux",
    ] {
        assert!(arguments.iter().any(|arg| arg == required), "{required}");
    }
    assert!(
        !arguments
            .iter()
            .any(|arg| matches!(arg.as_str(), "run" | "continue" | "shell" | "monitor reset"))
    );
    assert!(spec.boot_guidance().contains("kgdboc=ttyAMA0,115200"));
    assert!(spec.boot_guidance().contains("NOT applied"));
    let mut json: serde_json::Value = serde_json::from_str(&request.arguments[2]).unwrap();
    json["command"] = "sudo reboot".into();
    assert!(serde_json::from_value::<KgdbSerialSpec>(json).is_err());
}

#[test]
fn kgdb_serial_rejects_unready_ambiguous_injected_and_unbounded_inputs() {
    for device in [
        "/dev/null",
        "/dev/tty",
        "/dev/serial/by-id/device",
        "/dev/pts/../1",
        "/dev/ttyUSB0;reboot",
        "/dev/tty USB0",
        "ttyUSB0",
    ] {
        let mut bad = spec();
        bad.device = device.into();
        assert!(bad.validate().is_err(), "{device}");
    }
    for baud in [0, 1, 12345, u32::MAX] {
        let mut bad = spec();
        bad.baud = baud;
        assert!(bad.validate().is_err());
    }
    for uart in ["", "ttyS0,9600", "$(reboot)", "ttyS0\ncontinue", "-ttyS0"] {
        let mut bad = spec();
        bad.target_uart = uart.into();
        assert!(bad.validate().is_err());
    }
    let mut bad = spec();
    bad.ready = false;
    assert!(
        bad.validate()
            .unwrap_err()
            .contains("already configured/halted")
    );
    bad = spec();
    bad.symbols = "/tmp/../vmlinux".into();
    assert!(bad.validate().is_err());
    bad = spec();
    bad.config = format!("/{}", "x".repeat(4096)).into();
    assert!(bad.validate().is_err());
}

pub(crate) const CONFIG: &str = "CONFIG_KGDB=y\nCONFIG_KGDB_SERIAL_CONSOLE=y\nCONFIG_DEBUG_INFO=y\nCONFIG_KGDB_KDB=y\n# CONFIG_FRAME_POINTER is not set\nCONFIG_STRICT_KERNEL_RWX=y\n";

#[test]
fn kgdb_serial_config_distinguishes_required_optional_disabled_and_unknown() {
    let report = KgdbConfigReport::inspect(CONFIG).unwrap();
    assert_eq!(report.options["CONFIG_KGDB"].as_deref(), Some("y"));
    assert_eq!(report.options["CONFIG_FRAME_POINTER"].as_deref(), Some("n"));
    assert_eq!(report.options["CONFIG_MAGIC_SYSRQ"], None);
    for text in [
        "",
        "CONFIG_KGDB=y",
        "CONFIG_KGDB=m\nCONFIG_KGDB_SERIAL_CONSOLE=y\nCONFIG_DEBUG_INFO=y",
    ] {
        assert!(
            KgdbConfigReport::inspect(text)
                .unwrap_err()
                .contains("requires")
        );
    }
    for suffix in [
        "CONFIG_KGDB=y\n",
        "CONFIG_KGDB=n\n",
        "# CONFIG_KGDB is not set\n",
        "CONFIG_MAGIC_SYSRQ=anything\n",
    ] {
        assert!(KgdbConfigReport::inspect(&format!("{CONFIG}{suffix}")).is_err());
    }
    assert!(KgdbConfigReport::inspect(&"x".repeat(MAX_KGDB_CONFIG_BYTES + 1)).is_err());
    assert!(KgdbConfigReport::inspect(&format!("{CONFIG}\0")).is_err());
}
