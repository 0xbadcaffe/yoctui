use super::*;

pub(super) struct Rootfs(Option<PathBuf>);
impl Rootfs {
    pub(super) fn prepare(spec: &QemuDebugSpec, socket: &Path) -> Result<Self> {
        let target = spec.session_rootfs(socket);
        if target == spec.rootfs {
            return Ok(Self(None));
        }
        // Isolate decompression and flash writes/cleanup from deployed inputs,
        // including forced death. Flash is copied even with QEMU snapshot enabled.
        let mut source = fs::File::open(&spec.rootfs)?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&target)?;
        let staging = Self(Some(target.clone()));
        if spec.boot_mode == yoctui_model::QemuDebugBootMode::OpenBmcRomulusFlash {
            const FLASH_BYTES: u64 = 32 * 1024 * 1024;
            let copied = std::io::copy(&mut source.take(FLASH_BYTES + 1), &mut output)
                .context("Could not stage flash privately")?;
            if copied != FLASH_BYTES {
                bail!("Romulus flash size changed while staging; guest was not started");
            }
        } else {
            std::io::copy(&mut source, &mut output).context("Could not stage rootfs privately")?;
        }
        Ok(staging)
    }
}

impl Drop for Rootfs {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = fs::remove_file(path);
            let _ = fs::remove_file(path.with_extension(""));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn qemu_debug_flash_staging_is_private_bounded_and_cleans_on_error() {
        let directory = super::super::private_directory().unwrap();
        let source_dir = directory.join("source");
        fs::create_dir(&source_dir).unwrap();
        let source = source_dir.join("romulus.static.mtd");
        let file = fs::File::create(&source).unwrap();
        file.set_len(32 * 1024 * 1024).unwrap();
        let spec = QemuDebugSpec {
            boot_mode: yoctui_model::QemuDebugBootMode::OpenBmcRomulusFlash,
            runqemu: "/tools/runqemu".into(),
            gdb: "/tools/gdb".into(),
            build_dir: "/build".into(),
            qemuboot: "/build/image.qemuboot.conf".into(),
            kernel: "/build/zImage".into(),
            rootfs: source.clone(),
            symbols: "/build/vmlinux".into(),
            memory_mib: 512,
        };
        let socket = directory.join("gdb.sock");
        let staged = spec.session_rootfs(&socket);
        {
            let _guard = Rootfs::prepare(&spec, &socket).unwrap();
            assert_ne!(staged, source);
            assert_eq!(fs::metadata(&staged).unwrap().len(), 32 * 1024 * 1024);
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&staged).unwrap().permissions().mode() & 0o777,
                0o600
            );
            fs::write(&staged, "guest writes").unwrap();
            assert_eq!(fs::metadata(&source).unwrap().len(), 32 * 1024 * 1024);
        }
        assert!(!staged.exists());
        for size in [4, 32 * 1024 * 1024 + 1] {
            file.set_len(size).unwrap();
            assert!(Rootfs::prepare(&spec, &socket).is_err());
            assert!(!staged.exists());
            assert_eq!(file.metadata().unwrap().len(), size);
        }
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn qemu_debug_compressed_staging_preserves_source_and_cleans_only_owned_files() {
        let directory = super::super::private_directory().unwrap();
        let source_dir = directory.join("source");
        fs::create_dir(&source_dir).unwrap();
        let source = source_dir.join("test-image.ext4.zst");
        fs::write(&source, "compressed fixture").unwrap();
        let spec = QemuDebugSpec {
            boot_mode: yoctui_model::QemuDebugBootMode::DirectKernel,
            runqemu: "/tools/runqemu".into(),
            gdb: "/tools/gdb".into(),
            build_dir: "/build".into(),
            qemuboot: "/build/image.qemuboot.conf".into(),
            kernel: "/build/bzImage".into(),
            rootfs: source.clone(),
            symbols: "/build/vmlinux".into(),
            memory_mib: 1024,
        };
        let socket = directory.join("gdb.sock");
        let staged = spec.session_rootfs(&socket);
        {
            let _guard = Rootfs::prepare(&spec, &socket).unwrap();
            assert_eq!(fs::read(&staged).unwrap(), fs::read(&source).unwrap());
            fs::write(staged.with_extension(""), "decompressed fixture").unwrap();
            assert!(Rootfs::prepare(&spec, &socket).is_err());
            assert!(staged.exists());
        }
        assert!(!staged.exists());
        assert!(!staged.with_extension("").exists());
        assert_eq!(fs::read_to_string(source).unwrap(), "compressed fixture");
        fs::remove_dir_all(directory).unwrap();
    }
}
