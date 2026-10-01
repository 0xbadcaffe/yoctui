use super::*;

const CONFIG: &str =
    "CONFIG_KGDB=y\nCONFIG_KGDB_SERIAL_CONSOLE=y\nCONFIG_DEBUG_INFO=y\nCONFIG_KGDB_KDB=y\n";

fn elf() -> Vec<u8> {
    let mut bytes = vec![0; 64 + 4 * 64];
    bytes[..6].copy_from_slice(b"\x7fELF\x02\x01");
    bytes[40..48].copy_from_slice(&64_u64.to_le_bytes());
    bytes[58..60].copy_from_slice(&64_u16.to_le_bytes());
    bytes[60..62].copy_from_slice(&4_u16.to_le_bytes());
    bytes[62..64].copy_from_slice(&1_u16.to_le_bytes());
    let strings = b"\0.shstrtab\0.symtab\0.debug_info\0";
    bytes[128..132].copy_from_slice(&1_u32.to_le_bytes());
    bytes[152..160].copy_from_slice(&320_u64.to_le_bytes());
    bytes[160..168].copy_from_slice(&(strings.len() as u64).to_le_bytes());
    bytes[192..196].copy_from_slice(&11_u32.to_le_bytes());
    bytes[256..260].copy_from_slice(&19_u32.to_le_bytes());
    bytes.extend(strings);
    bytes
}

#[cfg(target_os = "linux")]
struct Fixture {
    root: tempfile::TempDir,
    spec: KgdbSerialSpec,
    master: File,
    _slave: File,
}

#[cfg(target_os = "linux")]
fn fixture() -> Fixture {
    use std::os::fd::FromRawFd;
    let root = tempfile::tempdir().unwrap();
    let (mut master, mut slave) = (-1, -1);
    let mut name = [0_i8; 128];
    // The fixture owns both ends; no real serial hardware is opened.
    assert_eq!(
        unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                name.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
            )
        },
        0
    );
    let device = unsafe { std::ffi::CStr::from_ptr(name.as_ptr()) }
        .to_str()
        .unwrap()
        .into();
    let spec = KgdbSerialSpec {
        gdb: std::env::current_exe().unwrap(),
        cwd: root.path().into(),
        symbols: root.path().join("vmlinux"),
        config: root.path().join(".config"),
        device,
        baud: 115200,
        target_uart: "ttyS0".into(),
        ready: true,
    };
    fs::write(&spec.config, CONFIG).unwrap();
    fs::write(&spec.symbols, elf()).unwrap();
    Fixture {
        root,
        spec,
        master: unsafe { File::from_raw_fd(master) },
        _slave: unsafe { File::from_raw_fd(slave) },
    }
}

#[cfg(target_os = "linux")]
#[test]
fn kgdb_serial_preflight_reads_inputs_without_opening_serial_or_starting_gdb() {
    use std::os::fd::AsRawFd;
    let mut fixture = fixture();
    let report = validate_files(&fixture.spec).unwrap();
    assert_eq!(report.options["CONFIG_KGDB"].as_deref(), Some("y"));
    assert_eq!(fs::read_to_string(&fixture.spec.config).unwrap(), CONFIG);
    assert_eq!(fs::read(&fixture.spec.symbols).unwrap(), elf());
    unsafe {
        libc::fcntl(fixture.master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK);
    }
    assert_eq!(
        fixture.master.read(&mut [0; 16]).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    fs::write(&fixture.spec.config, "CONFIG_KGDB=n\n").unwrap();
    assert!(validate_files(&fixture.spec).is_err());
    fs::write(&fixture.spec.config, CONFIG).unwrap();
    fs::write(&fixture.spec.symbols, "bzImage").unwrap();
    assert!(validate_files(&fixture.spec).is_err());
    fs::write(&fixture.spec.symbols, elf()).unwrap();
    fixture.spec.device = "/dev/ttyYOCTUI_MISSING_DEVICE".into();
    assert!(
        validate_files(&fixture.spec)
            .unwrap_err()
            .to_string()
            .contains("serial device unavailable")
    );
}

#[cfg(target_os = "linux")]
#[test]
fn kgdb_serial_preflight_rejects_symlink_special_oversize_missing_and_changed_files() {
    use std::os::unix::fs::symlink;
    let mut fixture = fixture();
    let original = fixture.spec.config.clone();
    let link = fixture.root.path().join("link");
    symlink(&original, &link).unwrap();
    fixture.spec.config = link;
    assert!(validate_files(&fixture.spec).is_err());
    fixture.spec.config = "/dev/null".into();
    assert!(validate_files(&fixture.spec).is_err());
    fixture.spec.config = fixture.root.path().join("missing");
    assert!(validate_files(&fixture.spec).is_err());
    fixture.spec.config = original;
    fs::write(&fixture.spec.config, vec![b'x'; MAX_KGDB_CONFIG_BYTES + 1]).unwrap();
    assert!(validate_files(&fixture.spec).is_err());
    fs::write(&fixture.spec.config, CONFIG).unwrap();
    fixture.spec.gdb = fixture.root.path().join("missing-gdb");
    assert!(validate_files(&fixture.spec).is_err());
    assert!(validate_device(&fixture.spec.config).is_err());
    let link = fixture.root.path().join("device-link");
    symlink(&fixture.spec.device, &link).unwrap();
    assert!(validate_device(&link).is_err());
}

#[test]
fn kgdb_serial_decode_rejects_unknown_fields_and_oversized_payloads() {
    assert!(decode("{\"shell\":\"reboot\"}").is_err());
    assert!(decode(&" ".repeat(32 * 1024 + 1)).is_err());
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "private subprocess entrypoint, exercised by parent test"]
fn kgdb_serial_helper_child() {
    let encoded = std::env::var("YOCTUI_TEST_KGDB_SPEC").expect("only parent test invokes this");
    if let Err(error) = run(&encoded) {
        eprintln!("{error:#}");
        std::process::exit(3);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn kgdb_serial_confirmed_helper_revalidates_then_execs_only_fixed_gdb_argv() {
    use std::os::unix::fs::PermissionsExt;
    let mut fixture = fixture();
    fixture.spec.gdb = fixture.root.path().join("fake-gdb");
    let marker = fixture.root.path().join("started");
    fs::write(
        &fixture.spec.gdb,
        "#!/bin/sh\nprintf 'ARG:%s\\n' \"$@\"\nprintf started > started\n",
    )
    .unwrap();
    fs::set_permissions(&fixture.spec.gdb, fs::Permissions::from_mode(0o700)).unwrap();
    let invoke = |spec: &KgdbSerialSpec| {
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "kgdb_serial::tests::kgdb_serial_helper_child",
                "--ignored",
                "--nocapture",
            ])
            .env(
                "YOCTUI_TEST_KGDB_SPEC",
                serde_json::to_string(spec).unwrap(),
            )
            .output()
            .unwrap()
    };
    fs::write(&fixture.spec.config, "CONFIG_KGDB=n\n").unwrap();
    let output = invoke(&fixture.spec);
    assert!(!output.status.success());
    assert!(!marker.exists());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires"));
    fs::write(&fixture.spec.config, CONFIG).unwrap();
    let output = invoke(&fixture.spec);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout
            .lines()
            .filter_map(|line| line.strip_prefix("ARG:"))
            .collect::<Vec<_>>(),
        fixture
            .spec
            .gdb_arguments()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );
    assert!(marker.exists());
    assert_eq!(fs::read_to_string(&fixture.spec.config).unwrap(), CONFIG);
}
