use super::*;
use yoctui_utils::is_absolute_normal_path;

pub(super) fn request(
    draft: &KernelDebugDraft,
    tools: &KernelDebugTools,
) -> Result<TerminalLaunchRequest, String> {
    for field in [
        KernelDebugField::Host,
        KernelDebugField::User,
        KernelDebugField::Port,
        KernelDebugField::Pid,
        KernelDebugField::Symbols,
        KernelDebugField::Data,
        KernelDebugField::Endpoint,
        KernelDebugField::Event,
    ] {
        let value = draft.value(field);
        if value.len() > MAX_KERNEL_DEBUG_FIELD_BYTES || value.chars().any(char::is_control) {
            return Err(format!(
                "{} contains control characters or exceeds 4096 bytes",
                field.label()
            ));
        }
    }
    if !is_absolute_normal_path(&tools.cwd) {
        return Err("Host working directory is unavailable; refresh tools".into());
    }
    let name = draft
        .tool
        .program()
        .ok_or("This technique is guidance only; no command is launched")?;
    let arguments = arguments(draft)?;
    let (program, arguments, scope) = if draft.tool.runtime_target() && draft.ssh {
        host(&draft.host)?;
        if draft.user.is_empty()
            || draft.user.len() > 64
            || draft.user.starts_with('-')
            || !draft
                .user
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        {
            return Err("SSH user must be a simple non-option account name".into());
        }
        let port = port(&draft.port)?;
        let remote = std::iter::once(name.to_owned())
            .chain(arguments)
            .map(|argument| format!("'{}'", argument.replace('\'', "'\\''")))
            .collect::<Vec<_>>()
            .join(" ");
        (
            tools.program("ssh")?,
            vec![
                "-tt".into(),
                "-p".into(),
                port.to_string(),
                "-l".into(),
                draft.user.clone(),
                "--".into(),
                draft.host.clone(),
                format!("exec {remote}"),
            ],
            "SSH TARGET",
        )
    } else {
        (tools.program(name)?, arguments, "HOST, NOT TARGET")
    };
    Ok(TerminalLaunchRequest {
        name: format!("Kernel debug · {} · {scope}", draft.tool.label()),
        kind: TerminalCreationKind::Utility,
        cwd: tools.cwd.clone(),
        program,
        arguments,
        completion: None,
    })
}

fn arguments(draft: &KernelDebugDraft) -> Result<Vec<String>, String> {
    use KernelDebugTool as T;
    let pid = || {
        draft
            .pid
            .parse::<u32>()
            .ok()
            .filter(|pid| *pid > 0 && *pid <= i32::MAX as u32)
            .map(|pid| pid.to_string())
            .ok_or_else(|| "PID must be a positive process ID on the selected system".to_owned())
    };
    let gdb = || {
        vec![
            "-nx".into(),
            "-nh".into(),
            "-q".into(),
            "-iex".into(),
            "set auto-load off".into(),
            "-iex".into(),
            "set debuginfod enabled off".into(),
        ]
    };
    Ok(match draft.tool {
        T::GdbRemote => {
            path(&draft.symbols)?;
            let (hostname, port_number) = draft
                .endpoint
                .rsplit_once(':')
                .ok_or("GDB endpoint must be a TCP DNS/IPv4 host:port")?;
            host(hostname)?;
            let endpoint = format!("{hostname}:{}", port(port_number)?);
            let mut arguments = gdb();
            arguments.extend([
                "-iex".into(),
                "set auto-connect-native-target off".into(),
                format!("--symbols={}", draft.symbols),
                "-ex".into(),
                format!("target remote {endpoint}"),
            ]);
            arguments
        }
        T::GdbCore => {
            path(&draft.symbols)?;
            path(&draft.data)?;
            let mut arguments = gdb();
            arguments.extend([
                format!("--se={}", draft.symbols),
                format!("--core={}", draft.data),
            ]);
            arguments
        }
        T::Strace => vec!["-f".into(), "-tt".into(), "-T".into(), "-p".into(), pid()?],
        T::Perf => vec!["top".into(), "-p".into(), pid()?],
        T::TraceCmd => {
            path(&draft.data)?;
            vec![
                "report".into(),
                "-N".into(),
                "-i".into(),
                draft.data.clone(),
            ]
        }
        T::Ftrace => vec!["--".into(), "/sys/kernel/tracing/trace".into()],
        T::Dmesg => vec!["-w".into()],
        T::DynamicDebug => vec![
            "--".into(),
            "/sys/kernel/debug/dynamic_debug/control".into(),
        ],
        T::Kmemleak => vec!["--".into(), "/sys/kernel/debug/kmemleak".into()],
        T::Bpftrace => {
            let parts = draft.event.split(':').collect::<Vec<_>>();
            if parts.len() != 2
                || parts[0] != "syscalls"
                || !parts[1].starts_with("sys_enter_")
                || parts[1].len() <= "sys_enter_".len()
                || draft.event.len() > 128
                || !parts
                    .iter()
                    .all(|part| part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
            {
                return Err("Tracepoint must be syscalls:sys_enter_NAME (letters, digits, underscores only)".into());
            }
            vec![
                "-e".into(),
                format!("tracepoint:{} {{ @[comm] = count(); }}", draft.event),
            ]
        }
        T::Lttng => vec!["list".into(), "--kernel".into()],
        T::Crash => {
            path(&draft.symbols)?;
            path(&draft.data)?;
            vec![
                "--no_crashrc".into(),
                draft.symbols.clone(),
                draft.data.clone(),
            ]
        }
        _ => return Err("Guidance-only technique".into()),
    })
}

fn path(value: &str) -> Result<(), String> {
    if value.is_empty() || !is_absolute_normal_path(std::path::Path::new(value)) {
        return Err("File paths must be explicit normalized absolute paths; ~ and traversal are not expanded".into());
    }
    Ok(())
}

fn host(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 253
        || value.starts_with('-')
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
    {
        return Err("Host must be a DNS/IPv4 name, not an option, command or URI".into());
    }
    Ok(())
}

fn port(value: &str) -> Result<u16, String> {
    value
        .parse::<u16>()
        .ok()
        .filter(|port| *port > 0)
        .ok_or_else(|| "Port must be an integer from 1 through 65535".into())
}
