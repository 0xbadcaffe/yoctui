use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "yoctui-debug-defaults-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn context(&self) -> KernelDebugDefaultContext {
        KernelDebugDefaultContext {
            build_dir: self.0.clone(),
            machine: "romulus".into(),
            image: Some("demo".into()),
        }
    }
    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
const DEPLOY: &str = "tmp/deploy/images/romulus/";
const CONFIG: &str = "[config_bsp]\nmachine=romulus\nimage_name=demo-romulus-123\nqb_system_name=qemu-system-arm\nqb_default_kernel=none\nkernel_imagetype=zImage\nqb_machine=-machine romulus-bmc\nqb_default_fstype=static.mtd\nqb_rootfs_opt=-drive file=@ROOTFS@,if=mtd,format=raw\nqb_mem=-m 512\n";

#[test]
fn kernel_debug_defaults_find_split_dwarf_symbols_and_remote_does_not_require_qemuboot() {
    let f = Fixture::new();
    f.write(
        "tmp/work-shared/romulus/kernel-build-artifacts/kernel-abiversion",
        "6.18.1\n",
    );
    f.write(
        "yoctui-debug-artifacts-romulus/package/boot/vmlinux-6.18.1",
        "stripped",
    );
    let symbols = f.write(
        "yoctui-debug-artifacts-romulus/package/boot/.debug/vmlinux-6.18.1",
        "",
    );
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
    fs::write(&symbols, bytes).unwrap();
    f.write(
        &format!("{DEPLOY}demo-romulus-123.qemuboot.conf"),
        "x".repeat(128 * 1024 + 1).as_str(),
    );
    let defaults =
        discover_for_tool(&f.context(), yoctui_model::KernelDebugTool::GdbRemote).unwrap();
    assert!(
        defaults
            .values
            .contains(&(F::Symbols, symbols.display().to_string()))
    );
    assert!(
        !defaults
            .values
            .iter()
            .any(|(field, _)| *field == F::Qemuboot)
    );
    assert!(discover(&f.context()).is_err());
}

#[test]
fn kernel_debug_defaults_select_image_boot_mode_memory_and_contained_aliases() {
    let f = Fixture::new();
    let config = f.write(&format!("{DEPLOY}demo-romulus-123.qemuboot.conf"), CONFIG);
    let rootfs = f.write(&format!("{DEPLOY}demo-romulus-123.static.mtd"), "flash");
    File::options()
        .write(true)
        .open(&rootfs)
        .unwrap()
        .set_len(32 * 1024 * 1024)
        .unwrap();
    let kernel = f.write(&format!("{DEPLOY}zImage-version.bin"), "kernel");
    #[cfg(unix)]
    std::os::unix::fs::symlink("zImage-version.bin", f.0.join(format!("{DEPLOY}zImage"))).unwrap();
    f.write(&format!("{DEPLOY}other-romulus-123.qemuboot.conf"), CONFIG);
    let defaults = discover(&f.context()).unwrap();
    for (field, path) in [
        (F::Qemuboot, config),
        (F::RootfsImage, rootfs),
        (F::KernelImage, kernel),
    ] {
        assert!(
            defaults
                .values
                .contains(&(field, path.display().to_string()))
        );
    }
    assert_eq!(
        defaults.boot_mode,
        Some(QemuDebugBootMode::OpenBmcRomulusFlash)
    );
    assert!(defaults.values.contains(&(F::Memory, "512".into())));
    assert!(defaults.note.contains("vmlinux symbols"));
    assert!(!defaults.values.iter().any(|(f, _)| *f == F::Symbols));
}

#[test]
fn kernel_debug_defaults_reject_ambiguity_escape_bad_identity_metadata_and_bounds() {
    let f = Fixture::new();
    let config = f.write(&format!("{DEPLOY}demo-romulus-123.qemuboot.conf"), CONFIG);
    let second = f.write(&format!("{DEPLOY}demo-romulus-456.qemuboot.conf"), CONFIG);
    assert!(
        !discover(&f.context())
            .unwrap()
            .values
            .iter()
            .any(|(f, _)| *f == F::Qemuboot)
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            config.file_name().unwrap(),
            f.0.join(format!("{DEPLOY}demo-romulus.qemuboot.conf")),
        )
        .unwrap();
        assert!(
            discover(&f.context())
                .unwrap()
                .values
                .iter()
                .any(|(f, _)| *f == F::Qemuboot)
        );
        let outside = Fixture::new();
        let escaped = outside.write("zImage", "other build kernel");
        std::os::unix::fs::symlink(escaped, f.0.join(format!("{DEPLOY}zImage"))).unwrap();
        assert!(
            !discover(&f.context())
                .unwrap()
                .values
                .iter()
                .any(|(f, _)| *f == F::KernelImage)
        );
    }
    fs::remove_file(second).unwrap();
    for text in [
        CONFIG.replace("machine=romulus", "machine=other"),
        format!("{CONFIG}qb_mem=-m 256\n"),
        "x".repeat(128 * 1024 + 1),
    ] {
        fs::write(&config, text).unwrap();
        assert!(discover(&f.context()).is_err());
    }
    let mut context = f.context();
    context.machine = "../other".into();
    assert!(discover(&context).is_err());
    assert!(
        discover(&Fixture::new().context())
            .unwrap()
            .values
            .is_empty()
    );
}

#[tokio::test]
async fn kernel_debug_defaults_worker_preserves_live_edits_and_does_not_launch() {
    use yoctui_model::{Action, App, Dialog, KernelDebugAction as A, KernelDebugTool, Screen};
    let f = Fixture::new();
    f.write(&format!("{DEPLOY}demo-romulus-123.qemuboot.conf"), CONFIG);
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    app.workspace.build_dir = Some(f.0.clone());
    app.workspace
        .variables
        .insert("MACHINE".into(), "romulus".into());
    app.build.target = Some("demo".into());
    app.kernel_debug.selection = KernelDebugTool::ALL
        .iter()
        .position(|t| *t == KernelDebugTool::QemuGdb)
        .unwrap();
    let effect = yoctui_model::update(&mut app, Action::KernelDebug(A::OpenSelected)).unwrap();
    let mut worker = super::super::worker::KernelDebugIo::default();
    worker.submit(effect);
    let Some(Dialog::KernelDebug(d)) = app.active_dialog_mut() else {
        panic!()
    };
    d.selection = 6;
    yoctui_model::update(&mut app, Action::KernelDebug(A::Clear));
    yoctui_model::update(&mut app, Action::KernelDebug(A::Insert("2048".into())));
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !worker.poll(&mut app).await {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let Some(Dialog::KernelDebug(d)) = app.active_dialog() else {
        panic!()
    };
    assert_eq!(d.draft.qemu.memory, "2048");
    assert!(d.draft.qemu.qemuboot.ends_with("123.qemuboot.conf"));
    assert!(app.kernel_debug.pending.is_none());
    assert!(app.kernel_debug.prepared.is_none());
}

#[test]
#[ignore = "explicit read-only local OpenBMC artifact smoke, no guest launch"]
fn kernel_debug_defaults_actual_openbmc_read_only_smoke() {
    for build in [
        "/home/bspguy-dev/src/openbmc/build/romulus",
        "/home/bspguy-dev/src/build-openbmc-romulus",
    ] {
        let defaults = discover(&KernelDebugDefaultContext {
            build_dir: build.into(),
            machine: "romulus".into(),
            image: Some("obmc-phosphor-image".into()),
        })
        .unwrap();
        assert_eq!(
            defaults.boot_mode,
            Some(QemuDebugBootMode::OpenBmcRomulusFlash)
        );
        for field in [F::Qemuboot, F::KernelImage, F::RootfsImage, F::Memory] {
            assert!(
                defaults.values.iter().any(|(f, _)| *f == field),
                "{build}: {defaults:?}"
            );
        }
        assert_eq!(
            defaults.values.iter().any(|(f, _)| *f == F::Symbols),
            build.ends_with("build-openbmc-romulus"),
            "{defaults:?}"
        );
        println!("Read-only {build}: {defaults:?}");
    }
}
