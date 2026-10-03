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
#[path = "../../tests/qemu_debug_staging.rs"]
mod tests;
