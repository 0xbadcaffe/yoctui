//! Relays BitBake's generated menuconfig wrapper into the requesting PTY.

use anyhow::{Context, Result, bail};
use std::{
    ffi::{OsStr, OsString},
    fs,
    io::{ErrorKind, Read, Write},
    os::unix::{
        ffi::OsStringExt,
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

mod configuration;
mod diagnostics;

const MAX_HANDOFF_PATH_BYTES: usize = 16 * 1024;

pub(crate) fn command(
    bitbake: &Path,
    arguments: &[String],
) -> std::io::Result<(PathBuf, Vec<String>)> {
    let executable = std::env::current_exe()?.canonicalize()?;
    let mut relay_arguments = vec![
        "__menuconfig-relay".into(),
        "--bitbake".into(),
        bitbake.display().to_string(),
        "--".into(),
    ];
    relay_arguments.extend(arguments.iter().cloned());
    Ok((executable, relay_arguments))
}

pub(crate) fn run(bitbake: &Path, arguments: &[String]) -> Result<()> {
    let cwd = std::env::current_dir().context("cannot resolve the menuconfig build directory")?;
    let executable = std::env::current_exe()
        .context("cannot resolve the Yoctui executable")?
        .canonicalize()
        .context("cannot resolve the installed Yoctui executable")?;
    let socket = socket_path()?;
    remove_stale_socket(&socket)?;
    let _cleanup = Cleanup::new(socket.clone());
    let listener = UnixListener::bind(&socket)
        .with_context(|| format!("cannot create menuconfig relay {}", socket.display()))?;
    listener.set_nonblocking(true)?;

    let mut environment = std::env::vars().collect();
    configure_environment(&mut environment, &executable, &socket);
    let task = configuration::task_label(arguments);
    let (_configuration, config) = configuration::create(&environment, &socket)?;
    println!("Preparing {task} with BitBake… (Ctrl+B returns to Yoctui controls)");
    std::io::stdout().flush()?;
    let mut bitbake_child = Command::new(bitbake);
    bitbake_child
        .args(arguments)
        .arg("-R")
        .arg(&config)
        .current_dir(&cwd)
        .env_clear()
        .envs(environment);
    isolate_outer_bitbake_console(&mut bitbake_child);
    let mut bitbake_child = bitbake_child
        .spawn()
        .with_context(|| format!("cannot start {}", bitbake.display()))?;

    let diagnostics = diagnostics::Diagnostics::start(&mut bitbake_child)?;

    let (mut stream, _) = loop {
        match listener.accept() {
            Ok(connection) => break connection,
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                if let Some(status) = bitbake_child.try_wait()? {
                    diagnostics.report();
                    if status.success() {
                        bail!("BitBake finished without opening embedded {task}");
                    }
                    bail!("BitBake {task} failed with {status}");
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) => return Err(error).context("menuconfig relay failed"),
        }
    };
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let command = read_command(&mut stream)?;
    let (command_cwd, program, arguments) = validate_command(&cwd, &command)?;
    let wrapper_status = Command::new(&program)
        .args(arguments)
        .current_dir(command_cwd)
        .status()
        .with_context(|| format!("cannot start menuconfig handoff {}", program.display()))?;
    stream.write_all(&[u8::from(!wrapper_status.success())])?;
    drop(stream);

    let bitbake_status = bitbake_child.wait()?;
    if !wrapper_status.success() || !bitbake_status.success() {
        diagnostics.report();
    }
    if !wrapper_status.success() {
        bail!("{task} exited with {wrapper_status}");
    }
    if !bitbake_status.success() {
        bail!("BitBake {task} failed with {bitbake_status}");
    }
    Ok(())
}

fn isolate_outer_bitbake_console(command: &mut Command) {
    // BitBake's Knotty UI otherwise treats the relay PTY as its own terminal.
    // Its footer then overwrites ncurses and its terminal mode competes with
    // the validated wrapper started below. BitBake retains its normal logs;
    // only its transient client console is detached from this PTY.
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
}

pub(crate) fn handoff(socket: &Path, command: &[PathBuf]) -> Result<()> {
    let mut stream = UnixStream::connect(socket)
        .with_context(|| format!("cannot connect to menuconfig relay {}", socket.display()))?;
    stream.set_read_timeout(Some(Duration::from_secs(60 * 60)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let cwd = std::env::current_dir().context("cannot resolve menuconfig task directory")?;
    write_command(&mut stream, &cwd, command)?;
    let mut result = [1_u8];
    stream.read_exact(&mut result)?;
    if result[0] != 0 {
        bail!("menuconfig wrapper failed");
    }
    Ok(())
}

fn write_command(stream: &mut UnixStream, cwd: &Path, command: &[PathBuf]) -> Result<()> {
    if command.len() != 3 {
        bail!("menuconfig handoff command is invalid");
    }
    stream.write_all(&4_u32.to_be_bytes())?;
    for argument in std::iter::once(cwd).chain(command.iter().map(PathBuf::as_path)) {
        let bytes = argument.as_os_str().as_encoded_bytes();
        if bytes.is_empty() || bytes.len() > MAX_HANDOFF_PATH_BYTES {
            bail!("menuconfig handoff argument is invalid");
        }
        stream.write_all(&u32::try_from(bytes.len())?.to_be_bytes())?;
        stream.write_all(bytes)?;
    }
    Ok(())
}

fn read_command(stream: &mut UnixStream) -> Result<Vec<PathBuf>> {
    let count = read_length(stream)?;
    if count != 4 {
        bail!("menuconfig handoff command is invalid");
    }
    (0..count)
        .map(|_| {
            let length = read_length(stream)?;
            if length == 0 || length > MAX_HANDOFF_PATH_BYTES {
                bail!("menuconfig handoff argument is invalid");
            }
            let mut bytes = vec![0_u8; length];
            stream.read_exact(&mut bytes)?;
            Ok(PathBuf::from(OsString::from_vec(bytes)))
        })
        .collect()
}

fn read_length(stream: &mut UnixStream) -> Result<usize> {
    let mut length = [0_u8; 4];
    stream.read_exact(&mut length)?;
    Ok(usize::try_from(u32::from_be_bytes(length))?)
}

fn validate_command(
    build_dir: &Path,
    command: &[PathBuf],
) -> Result<(PathBuf, PathBuf, Vec<PathBuf>)> {
    if command.len() != 4 || command.iter().any(|argument| !argument.is_absolute()) {
        bail!("menuconfig handoff command is invalid");
    }
    let build_dir = build_dir.canonicalize()?;
    let command_cwd = command[0].canonicalize()?;
    if !command_cwd.starts_with(&build_dir) || !command_cwd.is_dir() {
        bail!("menuconfig task directory is outside the active build directory");
    }
    let program = command[1].canonicalize()?;
    if !program.is_file() || program.file_name() != Some(OsStr::new("oe-gnome-terminal-phonehome"))
    {
        bail!("menuconfig handoff executable is invalid");
    }
    let pidfile_parent = command[2]
        .parent()
        .and_then(|parent| parent.canonicalize().ok());
    if pidfile_parent.as_deref() != Some(std::env::temp_dir().as_path()) {
        bail!("menuconfig handoff pidfile is outside the temporary directory");
    }
    let wrapper = command[3]
        .canonicalize()
        .with_context(|| format!("cannot resolve menuconfig wrapper {}", command[3].display()))?;
    if !wrapper.starts_with(&build_dir) || !wrapper.is_file() {
        bail!("menuconfig wrapper is outside the active build directory");
    }
    Ok((command_cwd, program, vec![command[2].clone(), wrapper]))
}

fn shell_word(value: &OsStr) -> String {
    let value = value.to_string_lossy();
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub(crate) fn socket_path() -> Result<PathBuf> {
    Ok(socket_path_in(
        &yoctui_protocol::daemon_ipc::runtime_paths()?.directory,
        std::process::id(),
    ))
}

fn socket_path_in(runtime_directory: &Path, process_id: u32) -> PathBuf {
    runtime_directory.join(format!("menuconfig-{process_id}.sock"))
}

pub(crate) fn configure_environment(
    environment: &mut std::collections::BTreeMap<String, String>,
    executable: &Path,
    socket: &Path,
) {
    let additions = environment
        .entry("BB_ENV_PASSTHROUGH_ADDITIONS".into())
        .or_default();
    for variable in ["OE_TERMINAL", "OE_TERMINAL_CUSTOMCMD"] {
        if !additions.split_whitespace().any(|entry| entry == variable) {
            if !additions.is_empty() && !additions.ends_with(' ') {
                additions.push(' ');
            }
            additions.push_str(variable);
        }
    }
    environment.insert("OE_TERMINAL".into(), "custom".into());
    environment.insert(
        "OE_TERMINAL_CUSTOMCMD".into(),
        format!(
            "{} __menuconfig-handoff --socket {} -- {{command}}",
            shell_word(executable.as_os_str()),
            shell_word(socket.as_os_str())
        ),
    );
}

fn remove_stale_socket(socket: &Path) -> Result<()> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};

    if !socket.exists() {
        return Ok(());
    }
    if UnixStream::connect(socket).is_ok() {
        bail!("another menuconfig relay is already active");
    }
    let metadata = fs::symlink_metadata(socket)?;
    // SAFETY: `geteuid` has no preconditions and only reads the process identity.
    let effective_uid = unsafe { libc::geteuid() };
    if !metadata.file_type().is_socket() || metadata.uid() != effective_uid {
        bail!("refusing to replace an unsafe menuconfig relay path");
    }
    fs::remove_file(socket)?;
    Ok(())
}

struct Cleanup {
    socket: PathBuf,
}

impl Cleanup {
    fn new(socket: PathBuf) -> Self {
        Self { socket }
    }
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket);
    }
}

#[cfg(test)]
#[path = "tests/menuconfig_relay/mod.rs"]
mod tests;
