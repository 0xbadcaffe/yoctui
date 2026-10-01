//! Closed serial attachment inputs and read-only kernel prerequisite evidence.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

pub const MAX_KGDB_CONFIG_BYTES: usize = 512 * 1024;
pub const KGDB_CONFIG_OPTIONS: [&str; 7] = [
    "CONFIG_KGDB",
    "CONFIG_KGDB_SERIAL_CONSOLE",
    "CONFIG_DEBUG_INFO",
    "CONFIG_KGDB_KDB",
    "CONFIG_FRAME_POINTER",
    "CONFIG_MAGIC_SYSRQ",
    "CONFIG_STRICT_KERNEL_RWX",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KgdbConfigReport {
    pub options: BTreeMap<String, Option<String>>,
}

impl KgdbConfigReport {
    pub fn inspect(text: &str) -> Result<Self, String> {
        if text.len() > MAX_KGDB_CONFIG_BYTES
            || text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        {
            return Err("Kernel config exceeds 512 KiB or contains invalid control bytes".into());
        }
        let mut options: BTreeMap<String, Option<String>> = KGDB_CONFIG_OPTIONS
            .into_iter()
            .map(|name| (name.into(), None))
            .collect();
        for line in text.lines().map(str::trim) {
            let entry = line.split_once('=').or_else(|| {
                line.strip_prefix("# ")?
                    .strip_suffix(" is not set")
                    .map(|name| (name, "n"))
            });
            if let Some((name, value)) = entry
                && let Some(observed) = options.get_mut(name)
            {
                if observed.is_some() || !matches!(value, "y" | "m" | "n") {
                    return Err(format!(
                        "Ambiguous or invalid kernel config value for {name}"
                    ));
                }
                *observed = Some(value.into());
            }
        }
        let missing = KGDB_CONFIG_OPTIONS[..3]
            .iter()
            .filter(|name| options[**name].as_deref() != Some("y"))
            .copied()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(format!(
                "Serial KGDB requires built-in/debug options set to y: {}. This check never edits the kernel config.",
                missing.join(", ")
            ));
        }
        Ok(Self { options })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KgdbSerialSpec {
    pub gdb: PathBuf,
    pub cwd: PathBuf,
    pub symbols: PathBuf,
    pub config: PathBuf,
    pub device: PathBuf,
    pub baud: u32,
    pub target_uart: String,
    pub ready: bool,
}

impl KgdbSerialSpec {
    pub fn validate(&self) -> Result<(), String> {
        for path in [
            &self.gdb,
            &self.cwd,
            &self.symbols,
            &self.config,
            &self.device,
        ] {
            let text = path.to_str().ok_or("KGDB paths must be UTF-8")?;
            if !yoctui_utils::is_absolute_normal_path(path)
                || text.len() > 4096
                || text.chars().any(char::is_control)
            {
                return Err("KGDB paths must be bounded normalized absolute paths".into());
            }
        }
        let device = self.device.to_str().unwrap();
        if !device
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
            || !(device
                .strip_prefix("/dev/tty")
                .is_some_and(|name| !name.is_empty() && !name.contains('/'))
                || device.strip_prefix("/dev/pts/").is_some_and(|name| {
                    !name.is_empty() && name.bytes().all(|b| b.is_ascii_digit())
                }))
        {
            return Err("Select an explicit Linux /dev/ttyNAME or /dev/pts/NUMBER device; symlink aliases and command syntax are unsupported".into());
        }
        if ![
            1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600,
        ]
        .contains(&self.baud)
        {
            return Err(
                "Select a supported serial baud rate (1200 through 921600) matching the target"
                    .into(),
            );
        }
        if self.target_uart.is_empty()
            || self.target_uart.len() > 64
            || !self
                .target_uart
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err("Supply the target UART name (for example ttyS0), not the host USB device or boot arguments".into());
        }
        if !self.ready {
            return Err("Confirm readiness with yes only after the target is already configured/halted and serial-console clients are closed".into());
        }
        Ok(())
    }

    pub fn boot_guidance(&self) -> String {
        format!(
            "Manual target setup: kgdboc={},{} nokaslr; optional kgdbwait AFTER kgdboc. NOT applied by Yoctui.",
            self.target_uart, self.baud
        )
    }

    pub fn gdb_arguments(&self) -> Vec<String> {
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
            format!("set serial baud {}", self.baud),
            "-ex".into(),
            "set remotetimeout 10".into(),
            "-ex".into(),
            format!("target remote {}", self.device.display()),
        ]
    }

    pub fn terminal_request(
        &self,
        helper: PathBuf,
    ) -> Result<crate::TerminalLaunchRequest, String> {
        self.validate()?;
        if !yoctui_utils::is_absolute_normal_path(&helper) {
            return Err("KGDB launch helper must be an absolute executable".into());
        }
        let encoded = serde_json::to_string(self).map_err(|error| error.to_string())?;
        if encoded.len() > 32 * 1024 {
            return Err("KGDB specification exceeds 32 KiB".into());
        }
        Ok(crate::TerminalLaunchRequest {
            name: format!(
                "Kernel debug · KGDB serial BOARD TARGET · {} @ {}",
                self.device.display(),
                self.baud
            ),
            kind: crate::TerminalCreationKind::Utility,
            cwd: self.cwd.clone(),
            program: helper,
            arguments: vec!["__kgdb-serial-session".into(), "--spec".into(), encoded],
            completion: None,
        })
    }
}

#[cfg(test)]
#[path = "tests/kgdb_serial.rs"]
mod tests;
