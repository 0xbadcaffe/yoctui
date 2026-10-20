//! Per-launch BitBake overrides: an existing server must not fall back to a GUI.
use super::*;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt, symlink},
};

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
) -> Result<PathBuf> {
    let custom_command = environment
        .get("OE_TERMINAL_CUSTOMCMD")
        .context("relay has no custom terminal command")?;
    let mut identity = DefaultHasher::new();
    custom_command.hash(&mut identity);
    // Reuse the exact post-read path and unchanged file timestamps. A fresh
    // -R pathname (or a socket embedded in its content) reparses every recipe.
    let directory = socket
        .parent()
        .context("relay has no runtime directory")?
        .join(format!("embedded-terminal-{:016x}", identity.finish()));
    private_directory(&directory)?;
    let tools = directory.join("gnu-hosttools");
    private_directory(&tools)?;
    // Ubuntu's uutils tools cannot run reliably inside Yocto's pseudo devshell.
    // Prefer installed GNU variants only in this devshell; leave HOSTTOOLS alone.
    for name in [
        "ls", "pwd", "cat", "cp", "mv", "rm", "mkdir", "ln", "readlink", "realpath", "stat", "env",
        "head", "tail", "sort", "uniq", "cut", "wc", "chmod", "chown", "touch", "date", "basename",
        "dirname", "install",
    ] {
        let program = Path::new("/usr/bin").join(format!("gnu{name}"));
        if program.is_file() {
            let link = tools.join(name);
            match symlink(&program, &link) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    anyhow::ensure!(
                        fs::read_link(&link)? == program,
                        "unsafe GNU host tool link"
                    );
                }
                Err(error) => return Err(error.into()),
            }
        }
    }
    let custom = conf_value(custom_command);
    let path = conf_value(&format!("{}:", tools.display()));
    let contents = format!(
        "OE_TERMINAL = \"custom\"\nBB_NUMBER_THREADS = \"2\"\nBB_NUMBER_PARSE_THREADS = \"2\"\nPARALLEL_MAKE = \"-j2\"\nOE_TERMINAL_CUSTOMCMD = {custom}\nPATH:prepend:task-devshell = {path}\n"
    );
    let config = directory.join("terminal.conf");
    match fs::symlink_metadata(&config) {
        Ok(metadata) => {
            anyhow::ensure!(
                metadata.is_file()
                    && metadata.uid() == effective_uid()
                    && metadata.len() <= 64 * 1024,
                "unsafe terminal configuration file"
            );
            if fs::read_to_string(&config)? == contents {
                return Ok(config);
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let mut temporary = tempfile::NamedTempFile::new_in(&directory)?;
    temporary.write_all(contents.as_bytes())?;
    temporary.persist(&config)?;
    Ok(config)
}

fn effective_uid() -> u32 {
    // SAFETY: geteuid only reads the process identity.
    unsafe { libc::geteuid() }
}

fn private_directory(path: &Path) -> Result<()> {
    match fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    let metadata = fs::symlink_metadata(path)?;
    anyhow::ensure!(
        metadata.is_dir()
            && metadata.uid() == effective_uid()
            && metadata.permissions().mode() & 0o077 == 0,
        "unsafe terminal configuration directory"
    );
    Ok(())
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
