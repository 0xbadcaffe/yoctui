//! Closed, reviewed managed QEMU/GDB inputs and child argv templates.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QemuDebugSpec {
    pub runqemu: PathBuf,
    pub gdb: PathBuf,
    pub build_dir: PathBuf,
    pub qemuboot: PathBuf,
    pub kernel: PathBuf,
    pub rootfs: PathBuf,
    pub symbols: PathBuf,
    pub memory_mib: u32,
}

pub const QEMU_DEBUG_SOCKET_TEMPLATE: &str = "/PRIVATE_SESSION/gdb.sock";

impl QemuDebugSpec {
    pub fn validate(&self) -> Result<(), String> {
        for path in [
            &self.runqemu,
            &self.gdb,
            &self.build_dir,
            &self.qemuboot,
            &self.kernel,
            &self.rootfs,
            &self.symbols,
        ] {
            let text = path.to_str().ok_or("Debug paths must be UTF-8")?;
            if !yoctui_utils::is_absolute_normal_path(path)
                || text.len() > 4096
                || text.chars().any(char::is_control)
            {
                return Err("Debug paths must be bounded normalized absolute paths".into());
            }
        }
        for path in [&self.build_dir, &self.qemuboot, &self.kernel, &self.rootfs] {
            if !path
                .to_str()
                .unwrap()
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
            {
                return Err("runqemu input paths support ASCII letters/digits and /._- only; spaces, commas and shell syntax are unsupported".into());
            }
        }
        if !self.qemuboot.to_string_lossy().ends_with(".qemuboot.conf") {
            return Err("Select the exact deployed .qemuboot.conf, not a guessed machine".into());
        }
        if !(crate::MIN_QEMU_MEMORY_MIB..=crate::MAX_QEMU_MEMORY_MIB).contains(&self.memory_mib) {
            return Err("QEMU memory must be 128..=262144 MiB".into());
        }
        Ok(())
    }

    pub fn qemu_arguments(&self, socket: &Path) -> Vec<String> {
        vec![
            self.qemuboot.display().to_string(),
            self.kernel.display().to_string(),
            self.rootfs.display().to_string(),
            "nonetwork".into(),
            "snapshot".into(),
            "nographic".into(),
            "serialstdio".into(),
            format!("qemumemory={}", self.memory_mib),
            format!(
                "qemuparams=-S -chardev socket,path={},server=on,wait=off,id=yoctui_gdb -gdb chardev:yoctui_gdb",
                socket.display()
            ),
            "bootparams=nokaslr".into(),
        ]
    }

    pub fn gdb_arguments(&self, socket: &Path) -> Vec<String> {
        vec![
            "-nx".into(),
            "-nh".into(),
            "-q".into(),
            "-iex".into(),
            "set auto-load off".into(),
            "-iex".into(),
            "set debuginfod enabled off".into(),
            "-iex".into(),
            "set auto-connect-native-target off".into(),
            format!("--symbols={}", self.symbols.display()),
            "-ex".into(),
            "set remotetimeout 10".into(),
            "-ex".into(),
            format!("target remote {}", socket.display()),
        ]
    }

    pub fn terminal_request(
        &self,
        helper: PathBuf,
    ) -> Result<crate::TerminalLaunchRequest, String> {
        self.validate()?;
        if !yoctui_utils::is_absolute_normal_path(&helper) {
            return Err("Yoctui session helper must be an absolute executable".into());
        }
        Ok(crate::TerminalLaunchRequest {
            name: "Kernel debug · QEMU → GDB · MANAGED HOST GUEST".into(),
            kind: crate::TerminalCreationKind::Utility,
            cwd: self.build_dir.clone(),
            program: helper,
            arguments: vec![
                "__qemu-gdb-session".into(),
                "--spec".into(),
                serde_json::to_string(self).map_err(|error| error.to_string())?,
            ],
            completion: None,
        })
    }
}

#[cfg(test)]
#[path = "tests/qemu_debug.rs"]
mod tests;
