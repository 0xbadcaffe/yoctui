//! Per-launch BitBake overrides: an existing server must not fall back to a GUI.
use super::*;
use std::os::unix::fs::symlink;

pub(super) fn task_label(arguments: &[String]) -> &'static str {
    if arguments
        .windows(2)
        .any(|args| args[0] == "-c" && args[1] == "devshell")
        || arguments.iter().any(|arg| arg == "--cmd=devshell")
    {
        "devshell"
    } else {
        "menuconfig"
    }
}

pub(super) fn create(
    environment: &std::collections::BTreeMap<String, String>,
    socket: &Path,
) -> Result<(tempfile::TempDir, PathBuf)> {
    let directory = tempfile::Builder::new()
        .prefix("embedded-terminal-")
        .tempdir_in(socket.parent().context("relay has no runtime directory")?)?;
    let tools = directory.path().join("gnu-hosttools");
    fs::create_dir(&tools)?;
    // Ubuntu's uutils tools cannot run reliably inside Yocto's pseudo devshell.
    // Prefer installed GNU variants only in this devshell; leave HOSTTOOLS alone.
    for name in [
        "ls", "pwd", "cat", "cp", "mv", "rm", "mkdir", "ln", "readlink", "realpath", "stat", "env",
        "head", "tail", "sort", "uniq", "cut", "wc", "chmod", "chown", "touch", "date", "basename",
        "dirname", "install",
    ] {
        let program = Path::new("/usr/bin").join(format!("gnu{name}"));
        if program.is_file() {
            symlink(program, tools.join(name))?;
        }
    }
    let custom = conf_value(&environment["OE_TERMINAL_CUSTOMCMD"]);
    let path = conf_value(&format!("{}:", tools.display()));
    let contents = format!(
        "OE_TERMINAL = \"custom\"\nBB_NUMBER_THREADS = \"2\"\nBB_NUMBER_PARSE_THREADS = \"2\"\nPARALLEL_MAKE = \"-j2\"\nOE_TERMINAL_CUSTOMCMD = {custom}\nPATH:prepend:task-devshell = {path}\n"
    );
    let config = directory.path().join("terminal.conf");
    fs::write(&config, contents)?;
    Ok((directory, config))
}

fn conf_value(value: &str) -> String {
    // .conf files accept assignments, not anonymous Python function blocks.
    // Encode path/command bytes so quotes, backslashes and newlines cannot
    // become configuration syntax. BitBake expands this to the original UTF-8.
    let hex = value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("\"${{@bytes.fromhex('{hex}').decode('utf-8')}}\"")
}
