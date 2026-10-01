use super::*;
use std::fs;
struct TestDir(std::path::PathBuf);
impl TestDir {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "yoctui-qemu-debug-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> (TestDir, QemuDebugSpec) {
    let root = TestDir::new();
    let path = root.path();
    let spec = QemuDebugSpec {
        runqemu: std::env::current_exe().unwrap(),
        gdb: std::env::current_exe().unwrap(),
        build_dir: path.into(),
        qemuboot: path.join("image.qemuboot.conf"),
        kernel: path.join("bzImage"),
        rootfs: path.join("image.ext4"),
        symbols: path.join("vmlinux"),
        memory_mib: 1024,
    };
    fs::write(
        &spec.qemuboot,
        "[config_bsp]\nqb_system_name=qemu-system-x86_64\nqb_default_kernel=bzImage\n",
    )
    .unwrap();
    fs::write(&spec.kernel, "kernel").unwrap();
    fs::write(&spec.rootfs, "rootfs").unwrap();
    fs::write(&spec.symbols, elf()).unwrap();
    (root, spec)
}

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

#[test]
fn qemu_debug_preflight_checks_files_and_debug_sections_without_spawning() {
    let (_root, spec) = fixture();
    validate_files(&spec).unwrap();
    fs::write(&spec.symbols, b"compressed image").unwrap();
    assert!(validate_files(&spec).is_err());
    fs::write(&spec.symbols, elf()).unwrap();
    fs::write(
        &spec.qemuboot,
        "[config_bsp]\nqb_system_name=qemu-system-arm\nqb_default_kernel=none\n",
    )
    .unwrap();
    assert!(
        validate_files(&spec)
            .unwrap_err()
            .to_string()
            .contains("flash")
    );
    fs::remove_file(&spec.rootfs).unwrap();
    assert!(validate_files(&spec).is_err());
}

#[test]
fn qemu_debug_malformed_elf_and_symlink_are_rejected() {
    let (_root, spec) = fixture();
    for offset in [40, 58, 60, 62, 152, 160, 256] {
        let mut bytes = elf();
        bytes[offset..offset + 4].fill(255);
        fs::write(&spec.symbols, bytes).unwrap();
        assert!(validate_files(&spec).is_err(), "{offset}");
    }
    #[cfg(unix)]
    {
        fs::remove_file(&spec.rootfs).unwrap();
        std::os::unix::fs::symlink(&spec.kernel, &spec.rootfs).unwrap();
        assert!(validate_files(&spec).is_err());
    }
}

#[cfg(unix)]
#[tokio::test]
async fn qemu_debug_readiness_covers_socket_timeout_and_early_exit() {
    use std::{os::unix::net::UnixListener, time::Duration};
    use tokio::process::Command;
    let root = TestDir::new();
    let socket = root.path().join("debug.sock");
    let mut child = Command::new("/bin/sleep")
        .arg("10")
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    assert!(
        runtime::wait_ready(&mut child, &socket, Duration::from_millis(20))
            .await
            .is_err()
    );
    let _listener = UnixListener::bind(&socket).unwrap();
    runtime::wait_ready(&mut child, &socket, Duration::from_millis(20))
        .await
        .unwrap();
    child.kill().await.unwrap();
    let mut exited = Command::new("/bin/false").spawn().unwrap();
    exited.wait().await.unwrap();
    assert!(
        runtime::wait_ready(&mut exited, &socket, Duration::from_millis(20))
            .await
            .is_err()
    );
}

#[cfg(target_os = "linux")]
fn fake_tools(spec: &mut QemuDebugSpec, gdb_body: &str) {
    use std::os::unix::fs::PermissionsExt;
    spec.runqemu = spec.build_dir.join("runqemu");
    spec.gdb = spec.build_dir.join("gdb");
    fs::write(&spec.runqemu, "#!/usr/bin/python3\nimport os,sys,socket,time\np=[x for x in sys.argv if x.startswith('qemuparams=')][0]\npath=p.split('path=')[1].split(',')[0]\nopen('owned.pid','w').write(str(os.getpid()))\ns=socket.socket(socket.AF_UNIX);s.bind(path);s.listen()\nsys.stdout.write('x'*(5*1024*1024));sys.stdout.flush()\ntime.sleep(30)\n").unwrap();
    fs::write(&spec.gdb, format!("#!/bin/sh\ncase \"$*\" in *--version*) echo 'GNU gdb 17.1'; exit 0;; esac\n{gdb_body}\n")).unwrap();
    for tool in [&spec.runqemu, &spec.gdb] {
        fs::set_permissions(tool, fs::Permissions::from_mode(0o700)).unwrap();
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn qemu_debug_fake_session_success_failure_cancel_cleanup_and_log_cap() {
    for body in ["sleep 0.2; exit 0", "exit 7", "sleep 30"] {
        let (_root, mut spec) = fixture();
        fake_tools(&mut spec, body);
        if body == "sleep 30" {
            let plan = spec.clone();
            let task = tokio::spawn(async move { runtime::run(&plan).await });
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            task.abort();
            let _ = task.await;
        } else {
            let result = runtime::run(&spec).await;
            assert_eq!(result.is_ok(), body.ends_with("exit 0"));
        }
        let pid: i32 = fs::read_to_string(spec.build_dir.join("owned.pid"))
            .unwrap()
            .parse()
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while unsafe { libc::kill(pid, 0) } == 0 {
                // A killed child may briefly remain a zombie until Tokio reaps it.
                if fs::read_to_string(format!("/proc/{pid}/stat"))
                    .is_ok_and(|text| text.contains(") Z "))
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn qemu_debug_log_capture_drains_after_four_mib_cap() {
    use tokio::process::Command;
    let root = TestDir::new();
    let log = root.path().join("console.log");
    let mut child = Command::new("/usr/bin/python3")
        .args([
            "-c",
            "import sys;sys.stdout.write('x'*(5*1024*1024));sys.stderr.write('y'*1000)",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let readers = runtime::logging::capture(&mut child, fs::File::create(&log).unwrap());
    assert!(child.wait().await.unwrap().success());
    for reader in readers {
        reader.await.unwrap();
    }
    assert_eq!(fs::metadata(log).unwrap().len(), 4 * 1024 * 1024);
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn qemu_debug_watchdog_worker() {
    if let Ok(encoded) = std::env::var("YOCTUI_QGDB_TEST_SPEC") {
        let spec = serde_json::from_str(&encoded).unwrap();
        let _ = runtime::run(&spec).await;
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn qemu_debug_watchdog_stops_guest_on_helper_sigkill() {
    use std::time::Duration;
    let (_root, mut spec) = fixture();
    fake_tools(&mut spec, "sleep 30");
    let mut helper = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "qemu_debug::tests::qemu_debug_watchdog_worker",
            "--nocapture",
        ])
        .env(
            "YOCTUI_QGDB_TEST_SPEC",
            serde_json::to_string(&spec).unwrap(),
        )
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let pid_file = spec.build_dir.join("owned.pid");
    tokio::time::timeout(Duration::from_secs(3), async {
        while !pid_file.exists() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let pid: i32 = fs::read_to_string(pid_file).unwrap().parse().unwrap();
    helper.kill().await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while unsafe { libc::kill(pid, 0) } == 0 {
            if fs::read_to_string(format!("/proc/{pid}/stat"))
                .is_ok_and(|text| text.contains(") Z "))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
