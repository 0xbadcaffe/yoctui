use super::*;
use yoctui_model::{TerminalLaunchRequest, WorkbenchDebugConfig};

pub(super) fn prepare(
    session: u64,
    root: PathBuf,
    config: WorkbenchDebugConfig,
    breakpoints: Vec<(PathBuf, usize)>,
) -> anyhow::Result<TerminalLaunchRequest> {
    config.validate().map_err(anyhow::Error::msg)?;
    let root = root.canonicalize()?;
    anyhow::ensure!(root.is_dir(), "source folder is unavailable");
    let executable = Path::new(&config.executable);
    anyhow::ensure!(
        fs::metadata(executable)?.is_file(),
        "choose a regular ELF executable with debug symbols"
    );
    let mut magic = [0u8; 4];
    fs::File::open(executable)?.read_exact(&mut magic)?;
    anyhow::ensure!(
        magic == *b"\x7fELF",
        "choose an unstripped ELF executable, not a source file or script"
    );
    if !config.sysroot.is_empty() {
        anyhow::ensure!(
            Path::new(&config.sysroot).is_dir(),
            "the target sysroot directory is unavailable"
        );
    }
    let program = PathBuf::from(&config.gdb);
    let program = if program.is_absolute() {
        program
    } else {
        anyhow::ensure!(
            program.components().count() == 1,
            "use a GDB command name or absolute path"
        );
        std::env::var_os("PATH")
            .and_then(|path| {
                std::env::split_paths(&path)
                    .take(64)
                    .filter(|p| p.is_absolute())
                    .map(|p| p.join(&program))
                    .find(|p| p.is_file())
            })
            .ok_or_else(|| {
                anyhow::anyhow!("GDB not found; choose the SDK cross-GDB or install GDB")
            })?
    }
    .canonicalize()?;
    anyhow::ensure!(program.is_file(), "GDB executable is unavailable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        anyhow::ensure!(
            fs::metadata(&program)?.permissions().mode() & 0o111 != 0,
            "GDB is not executable"
        );
    }
    config
        .request(root, program, session, &breakpoints)
        .map_err(anyhow::Error::msg)
}
