use super::*;
use crate::{KgdbSerialSpec, QemuDebugBootMode, QemuDebugSpec};

#[test]
fn all_kernel_gdb_routes_load_elf_architecture_but_disable_implicit_host_execution() {
    let mut tcp = KernelDebugDraft::new(KernelDebugTool::GdbRemote);
    tcp.symbols = "/work/vmlinux".into();
    let managed = QemuDebugSpec {
        boot_mode: QemuDebugBootMode::DirectKernel,
        runqemu: "/tools/runqemu".into(),
        gdb: "/tools/gdb".into(),
        build_dir: "/work".into(),
        qemuboot: "/work/image.qemuboot.conf".into(),
        kernel: "/work/bzImage".into(),
        rootfs: "/work/image.ext4".into(),
        symbols: "/work/vmlinux".into(),
        memory_mib: 1024,
        ssh_forward_port: None,
    };
    let serial = KgdbSerialSpec {
        gdb: "/tools/gdb".into(),
        cwd: "/work".into(),
        symbols: "/work/vmlinux".into(),
        config: "/work/.config".into(),
        device: "/dev/ttyUSB0".into(),
        baud: 115200,
        target_uart: "ttyS2".into(),
        ready: true,
    };
    for arguments in [
        tcp.plan(&tools()).unwrap().arguments,
        managed.gdb_arguments(std::path::Path::new("/tmp/private/gdb.sock")),
        serial.gdb_arguments(),
    ] {
        let guard = arguments
            .iter()
            .position(|a| a == "set auto-connect-native-target off")
            .unwrap();
        let elf = arguments
            .iter()
            .position(|a| a == "--se=/work/vmlinux")
            .unwrap();
        assert!(guard < elf);
        assert_eq!(arguments[guard - 1], "-iex");
        assert!(arguments.contains(&"set auto-load off".into()));
        assert!(!arguments.iter().any(|a| a.starts_with("--symbols=")
            || matches!(a.as_str(), "target native" | "run" | "start" | "starti")));
        assert!(arguments.iter().any(|a| a.starts_with("target remote ")));
    }
}
