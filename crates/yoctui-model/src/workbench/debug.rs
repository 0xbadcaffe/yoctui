use super::*;

pub const WORKBENCH_MAX_BREAKPOINTS: usize = 128;
pub const WORKBENCH_DEBUG_FIELDS: [&str; 6] = [
    "Mode (Left/Right)",
    "GDB / cross-GDB executable",
    "Unstripped executable / symbols",
    "Target sysroot (optional)",
    "GDB server host:port",
    "Compiled source prefix (optional)",
];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WorkbenchDebugConfig {
    pub local: bool,
    pub gdb: String,
    pub executable: String,
    pub sysroot: String,
    pub endpoint: String,
    pub source_prefix: String,
}
impl Default for WorkbenchDebugConfig {
    fn default() -> Self {
        Self {
            local: false,
            gdb: "gdb".into(),
            executable: String::new(),
            sysroot: String::new(),
            endpoint: "127.0.0.1:2345".into(),
            source_prefix: String::new(),
        }
    }
}
impl WorkbenchDebugConfig {
    pub fn fields(&self) -> [&str; 6] {
        [
            if self.local {
                "Local program"
            } else {
                "Remote / QEMU gdbserver"
            },
            &self.gdb,
            &self.executable,
            &self.sysroot,
            &self.endpoint,
            &self.source_prefix,
        ]
    }
    pub fn field_mut(&mut self, index: usize) -> Option<&mut String> {
        match index {
            1 => Some(&mut self.gdb),
            2 => Some(&mut self.executable),
            3 => Some(&mut self.sysroot),
            4 => Some(&mut self.endpoint),
            5 => Some(&mut self.source_prefix),
            _ => None,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self
            .fields()
            .iter()
            .any(|v| v.len() > 4096 || v.chars().any(char::is_control))
        {
            return Err("Debug settings must be single-line values of at most 4096 bytes.".into());
        }
        if self.gdb.trim().is_empty() {
            return Err("Choose a GDB or cross-GDB executable.".into());
        }
        if !yoctui_utils::is_absolute_normal_path(Path::new(&self.executable))
            || [&self.sysroot, &self.source_prefix].iter().any(|value| {
                !value.is_empty()
                    && value.as_str() != "/"
                    && !yoctui_utils::is_absolute_normal_path(Path::new(value))
            })
        {
            return Err("Executable, sysroot and source prefix must be absolute paths.".into());
        }
        if !self.local {
            let (host, port) = self
                .endpoint
                .rsplit_once(':')
                .ok_or("Use a GDB server host:port.")?;
            if host.is_empty()
                || !host
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".-:[]".contains(&b))
                || port.parse::<u16>().ok().filter(|p| *p != 0).is_none()
            {
                return Err("Use a valid GDB server host:port.".into());
            }
        }
        Ok(())
    }
    /// Paths are argv values or quoted GDB tokens, never shell commands.
    pub fn request(
        &self,
        root: PathBuf,
        gdb: PathBuf,
        session: u64,
        breakpoints: &[(PathBuf, usize)],
    ) -> Result<TerminalLaunchRequest, String> {
        self.validate()?;
        if !root.is_absolute() || !gdb.is_absolute() {
            return Err("GDB and source folder must use absolute paths.".into());
        }
        if breakpoints.len() > WORKBENCH_MAX_BREAKPOINTS
            || breakpoints.iter().any(|(path, line)| {
                *line == 0
                    || !path.is_absolute()
                    || path.to_string_lossy().chars().any(char::is_control)
            })
        {
            return Err("Invalid or excessive source breakpoints.".into());
        }
        let quote =
            |value: &str| format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""));
        let mut arguments = vec![
            "-nx".into(),
            "-nh".into(),
            "-q".into(),
            "-iex".into(),
            "set auto-load off".into(),
            "-iex".into(),
            "set debuginfod enabled off".into(),
            format!("--se={}", self.executable),
            format!("--directory={}", root.display()),
        ];
        if !self.sysroot.is_empty() {
            arguments.extend(["-ex".into(), format!("set sysroot {}", self.sysroot)]);
        }
        if !self.source_prefix.is_empty() {
            arguments.extend([
                "-ex".into(),
                format!(
                    "set substitute-path {} {}",
                    quote(&self.source_prefix),
                    quote(&root.to_string_lossy())
                ),
            ]);
        }
        arguments.extend(["-ex".into(), "set breakpoint pending on".into()]);
        for (path, line) in breakpoints {
            let mapped = if !self.source_prefix.is_empty() {
                path.strip_prefix(&root)
                    .ok()
                    .map(|relative| Path::new(&self.source_prefix).join(relative))
            } else {
                None
            };
            let path = mapped.as_deref().unwrap_or(path);
            arguments.extend([
                "-ex".into(),
                format!(
                    "break -source {} -line {line}",
                    quote(&path.to_string_lossy())
                ),
            ]);
        }
        if !self.local {
            arguments.extend([
                "-iex".into(),
                "set auto-connect-native-target off".into(),
                "-ex".into(),
                "set remotetimeout 10".into(),
                "-ex".into(),
                format!("target remote {}", self.endpoint),
            ]);
        }
        Ok(TerminalLaunchRequest {
            name: format!("Workbench GDB · {session}"),
            kind: TerminalCreationKind::Utility,
            cwd: root,
            program: gdb,
            arguments,
            completion: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchDebugDialog {
    pub config: WorkbenchDebugConfig,
    pub field: usize,
    pub error: Option<String>,
    pub loading: bool,
}
