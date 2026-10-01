//! Typed Kernel debugging catalogue and validated command planning.
use std::{collections::BTreeMap, path::PathBuf};

use crate::{TerminalCreationKind, TerminalLaunchRequest};

mod catalogue;
mod plan;
mod state;
pub use catalogue::*;
pub(crate) use state::reduce;
pub use state::*;

pub const MAX_KERNEL_DEBUG_FIELD_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelDebugField {
    Destination,
    Host,
    User,
    Port,
    Pid,
    Symbols,
    Data,
    Endpoint,
    Event,
}

impl KernelDebugField {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Destination => "Execution scope",
            Self::Host => "SSH host",
            Self::User => "SSH user",
            Self::Port => "SSH port",
            Self::Pid => "PID on selected system",
            Self::Symbols => "Absolute executable/vmlinux symbols",
            Self::Data => "Absolute core/vmcore/trace.dat file",
            Self::Endpoint => "GDB TCP host:port (QEMU/KGDB stub)",
            Self::Event => "Syscall tracepoint (category:event)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelDebugDraft {
    pub tool: KernelDebugTool,
    pub ssh: bool,
    pub host: String,
    pub user: String,
    pub port: String,
    pub pid: String,
    pub symbols: String,
    pub data: String,
    pub endpoint: String,
    pub event: String,
}

impl KernelDebugDraft {
    pub fn new(tool: KernelDebugTool) -> Self {
        Self {
            tool,
            ssh: tool.runtime_target(),
            host: String::new(),
            user: "root".into(),
            port: "22".into(),
            pid: String::new(),
            symbols: String::new(),
            data: String::new(),
            endpoint: "127.0.0.1:1234".into(),
            event: "syscalls:sys_enter_openat".into(),
        }
    }

    pub fn fields(&self) -> Vec<KernelDebugField> {
        use KernelDebugField as F;
        let mut fields = Vec::new();
        if self.tool.runtime_target() {
            fields.push(F::Destination);
            if self.ssh {
                fields.extend([F::Host, F::User, F::Port]);
            }
        }
        match self.tool {
            KernelDebugTool::GdbRemote => fields.extend([F::Symbols, F::Endpoint]),
            KernelDebugTool::GdbCore | KernelDebugTool::Crash => {
                fields.extend([F::Symbols, F::Data]);
            }
            KernelDebugTool::Strace | KernelDebugTool::Perf => fields.push(F::Pid),
            KernelDebugTool::TraceCmd => fields.push(F::Data),
            KernelDebugTool::Bpftrace => fields.push(F::Event),
            _ => {}
        }
        fields
    }

    pub fn value(&self, field: KernelDebugField) -> &str {
        match field {
            KernelDebugField::Destination => {
                if self.ssh {
                    "SSH TARGET"
                } else {
                    "YOCTUI HOST, NOT TARGET"
                }
            }
            KernelDebugField::Host => &self.host,
            KernelDebugField::User => &self.user,
            KernelDebugField::Port => &self.port,
            KernelDebugField::Pid => &self.pid,
            KernelDebugField::Symbols => &self.symbols,
            KernelDebugField::Data => &self.data,
            KernelDebugField::Endpoint => &self.endpoint,
            KernelDebugField::Event => &self.event,
        }
    }

    pub fn value_mut(&mut self, field: KernelDebugField) -> Option<&mut String> {
        match field {
            KernelDebugField::Destination => None,
            KernelDebugField::Host => Some(&mut self.host),
            KernelDebugField::User => Some(&mut self.user),
            KernelDebugField::Port => Some(&mut self.port),
            KernelDebugField::Pid => Some(&mut self.pid),
            KernelDebugField::Symbols => Some(&mut self.symbols),
            KernelDebugField::Data => Some(&mut self.data),
            KernelDebugField::Endpoint => Some(&mut self.endpoint),
            KernelDebugField::Event => Some(&mut self.event),
        }
    }

    pub fn plan(&self, tools: &KernelDebugTools) -> Result<TerminalLaunchRequest, String> {
        plan::request(self, tools)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KernelDebugTools {
    pub cwd: PathBuf,
    pub programs: BTreeMap<String, PathBuf>,
}

impl KernelDebugTools {
    pub fn program(&self, name: &str) -> Result<PathBuf, String> {
        self.programs.get(name).filter(|path| {
            yoctui_utils::is_absolute_normal_path(path)
        }).cloned().ok_or_else(|| format!("{name} is missing on the host; install/configure it on PATH, then refresh. Target installation is separate."))
    }
}

#[cfg(test)]
#[path = "tests/kernel_debug.rs"]
mod tests;
