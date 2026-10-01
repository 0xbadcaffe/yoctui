#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelDebugTool {
    GdbRemote,
    GdbCore,
    Strace,
    Perf,
    TraceCmd,
    Ftrace,
    Dmesg,
    DynamicDebug,
    Kmemleak,
    Bpftrace,
    Lttng,
    Crash,
    Kgdb,
    Sanitizers,
    Lockdep,
    SysrqKdump,
}

impl KernelDebugTool {
    pub const ALL: [Self; 16] = [
        Self::GdbRemote,
        Self::GdbCore,
        Self::Strace,
        Self::Perf,
        Self::TraceCmd,
        Self::Ftrace,
        Self::Dmesg,
        Self::DynamicDebug,
        Self::Kmemleak,
        Self::Bpftrace,
        Self::Lttng,
        Self::Crash,
        Self::Kgdb,
        Self::Sanitizers,
        Self::Lockdep,
        Self::SysrqKdump,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::GdbRemote => "GDB · remote kernel/QEMU",
            Self::GdbCore => "GDB · userspace core",
            Self::Strace => "strace · syscall attach",
            Self::Perf => "perf · live CPU profile",
            Self::TraceCmd => "trace-cmd · offline report",
            Self::Ftrace => "ftrace · current snapshot",
            Self::Dmesg => "dmesg · kernel log follow",
            Self::DynamicDebug => "dynamic debug · callsite inventory",
            Self::Kmemleak => "kmemleak · existing report",
            Self::Bpftrace => "bpftrace · syscall counts",
            Self::Lttng => "LTTng · kernel-event discovery",
            Self::Crash => "crash · kernel vmcore analysis",
            Self::Kgdb => "KGDB / KDB setup",
            Self::Sanitizers => "KASAN / KCSAN / UBSAN",
            Self::Lockdep => "lockdep / hung tasks",
            Self::SysrqKdump => "SysRq / kdump setup",
        }
    }

    pub const fn program(self) -> Option<&'static str> {
        match self {
            Self::GdbRemote | Self::GdbCore => Some("gdb"),
            Self::Strace => Some("strace"),
            Self::Perf => Some("perf"),
            Self::TraceCmd => Some("trace-cmd"),
            Self::Ftrace | Self::DynamicDebug | Self::Kmemleak => Some("cat"),
            Self::Dmesg => Some("dmesg"),
            Self::Bpftrace => Some("bpftrace"),
            Self::Lttng => Some("lttng"),
            Self::Crash => Some("crash"),
            _ => None,
        }
    }

    pub const fn runtime_target(self) -> bool {
        matches!(
            self,
            Self::Strace
                | Self::Perf
                | Self::Ftrace
                | Self::Dmesg
                | Self::DynamicDebug
                | Self::Kmemleak
                | Self::Bpftrace
                | Self::Lttng
        )
    }

    pub const fn guide(self) -> &'static str {
        match self {
            Self::GdbRemote => {
                "Host GDB connects to an already configured QEMU/KGDB TCP stub. Supply matching uncompressed vmlinux with debug symbols and architecture-capable GDB. Breakpoints can stop the whole target. No guest boot or stub setup is automatic. Useful commands: info threads, bt, break, continue. Init files/auto-loaded scripts are disabled; explicitly trust kernel GDB helpers before loading them manually."
            }
            Self::GdbCore => {
                "Offline userspace core analysis, not a kernel vmcore debugger. Supply the exact executable with symbols and its core. Useful commands: bt full, info threads, thread apply all bt. Init files/auto-loaded scripts are disabled. Core dumps can contain secrets."
            }
            Self::Strace => {
                "Userspace syscall tracing, NOT a kernel source debugger. Attach to a PID on the chosen host/target; follows children and displays timestamps/durations. ptrace access is required. Attachment changes timing and may expose arguments/data. Ctrl+C detaches; do not select a critical service without considering disruption."
            }
            Self::Perf => {
                "Live CPU sampling for an explicit PID. Requires perf plus permitted perf_event access, matching kernel support and symbols. Sampling adds overhead. Kernel symbols/stack unwinding can require debug symbols and kernel configuration. This route does not change perf_event_paranoid or elevate privileges."
            }
            Self::TraceCmd => {
                "Offline trace-cmd report of an existing trace.dat; automatic plugins are disabled. Recording is manual: select relevant events and a bounded capture rather than tracing everything. Enable kernel tracing options and install trace-cmd on the capture system. Report files must match the capture and can contain sensitive workload details."
            }
            Self::Ftrace => {
                "Reads /sys/kernel/tracing/trace without clearing or enabling it. Requires mounted tracefs and read permission. Configure tracers/events manually; this route neither mounts tracefs nor writes controls. A snapshot during active tracing may be inconsistent. Function/function_graph and sched/IRQ events help latency analysis."
            }
            Self::Dmesg => {
                "Follow kernel messages without clearing the ring buffer. Requires dmesg with -w support and permission under dmesg_restrict. Diagnose boot, driver, warning, oops and sanitizer reports. Logs can contain sensitive data; access failure stays in terminal output."
            }
            Self::DynamicDebug => {
                "Read /sys/kernel/debug/dynamic_debug/control callsites. Requires dynamic-debug support, debugfs and permission. Enable selected module/file/function callsites manually, then inspect kernel logs. No write, mount or logging change is performed here."
            }
            Self::Kmemleak => {
                "Read the existing /sys/kernel/debug/kmemleak report. Requires CONFIG_DEBUG_KMEMLEAK, debugfs and permissions. Scanning/clearing is manual; suspected leaks need investigation and may be false positives. This route does not trigger a scan or change detector state."
            }
            Self::Bpftrace => {
                "Count one explicit syscall tracepoint by process name; Ctrl+C prints accumulated counts and removes probes. Requires bpftrace, BPF/tracing support and appropriate privilege. Instrumentation adds overhead; tracepoints vary by kernel. No arbitrary BPF program or privilege elevation is accepted."
            }
            Self::Lttng => {
                "List available kernel events, not a recording. Requires LTTng tools and kernel tracer modules/permissions on the selected system. Configure sessions, channels and bounded recording manually; no tracing session is created here."
            }
            Self::Crash => {
                "Offline kernel dump analysis with crash; crashrc startup files are disabled. Supply matching vmlinux/debug symbols and vmcore, architecture-compatible crash and a previously collected kdump. Useful commands: bt, log, ps, kmem. Collection/reboot is not automatic; vmcore may contain secrets."
            }
            Self::Kgdb => {
                "GUIDE ONLY. Enable KGDB and debug information; optionally KDB and KGDB serial-console support. Use matching vmlinux on the debugger host. Configure the target transport/boot arguments intentionally; kgdbwait/breakpoints stop execution. QEMU can provide its own GDB stub. Yoctui remote GDB uses TCP; serial KGDB transport setup remains manual."
            }
            Self::Sanitizers => {
                "GUIDE ONLY. KASAN detects memory-access bugs, KCSAN samples data races, UBSAN reports undefined behavior. Enable the architecture-supported options in menuconfig, rebuild/deploy deliberately and inspect reports via dmesg. Instrumentation changes memory/performance characteristics; no configuration is enabled automatically."
            }
            Self::Lockdep => {
                "GUIDE ONLY. CONFIG_PROVE_LOCKING/lockdep can expose lock ordering problems; DEBUG_ATOMIC_SLEEP helps sleeping-in-atomic bugs. Hung-task and RCU-stall diagnostics complement stack traces. Enable supported options manually, reproduce under controlled load and inspect dmesg. Debug options add overhead."
            }
            Self::SysrqKdump => {
                "GUIDE ONLY. SysRq stack/task dumps aid hang analysis; some SysRq actions crash or reboot the system. Configure crashkernel reservation, kexec/kdump and dump storage deliberately before collecting vmcore. Yoctui never writes sysrq-trigger, crashes/reboots the target or modifies crash settings."
            }
        }
    }

    pub const fn reference(self) -> &'static str {
        match self {
            Self::GdbRemote | Self::Kgdb => "https://docs.kernel.org/process/debugging/kgdb.html",
            Self::GdbCore => "https://sourceware.org/gdb/current/onlinedocs/gdb/Files.html",
            Self::Strace => "https://strace.io/",
            Self::Perf => {
                "https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/tools/perf/Documentation/perf-top.txt"
            }
            Self::TraceCmd => {
                "https://www.trace-cmd.org/Documentation/trace-cmd/trace-cmd-report.1.html"
            }
            Self::Ftrace => "https://docs.kernel.org/trace/ftrace.html",
            Self::Dmesg => {
                "https://github.com/util-linux/util-linux/blob/master/sys-utils/dmesg.1.adoc"
            }
            Self::DynamicDebug => "https://docs.kernel.org/admin-guide/dynamic-debug-howto.html",
            Self::Kmemleak => "https://docs.kernel.org/dev-tools/kmemleak.html",
            Self::Bpftrace => "https://bpftrace.org/docs/release_024/language",
            Self::Lttng => "https://lttng.org/man/1/lttng-list/v2.15/",
            Self::Crash => "https://crash-utility.github.io/",
            Self::Sanitizers => "https://docs.kernel.org/dev-tools/kasan.html",
            Self::Lockdep => "https://docs.kernel.org/locking/lockdep-design.html",
            Self::SysrqKdump => "https://docs.kernel.org/admin-guide/kdump/kdump.html",
        }
    }
}
