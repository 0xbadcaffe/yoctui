use super::*;
use std::{
    fs,
    io::Read,
    os::unix::fs::{DirBuilderExt, FileTypeExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{
    process::{Child, Command},
    signal::unix::{SignalKind, signal},
    time::{Instant, sleep, timeout},
};
pub(super) mod logging;
mod staging;
mod watchdog;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);

struct OwnedQemu {
    child: Child,
    pid: i32,
}
impl Drop for OwnedQemu {
    fn drop(&mut self) {
        // Only the new process group created by this helper, never a discovered PID.
        unsafe {
            libc::kill(-self.pid, libc::SIGKILL);
        }
    }
}
impl OwnedQemu {
    async fn stop(&mut self) {
        unsafe {
            libc::kill(-self.pid, libc::SIGTERM);
        }
        if timeout(Duration::from_millis(500), self.child.wait())
            .await
            .is_err()
        {
            unsafe {
                libc::kill(-self.pid, libc::SIGKILL);
            }
            let _ = self.child.wait().await;
        }
    }
}

pub(super) async fn run(spec: &QemuDebugSpec) -> Result<()> {
    let mut terminate = signal(SignalKind::terminate())?;
    let mut hangup = signal(SignalKind::hangup())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    verify_gdb(&spec.gdb).await?;
    let directory = private_directory()?;
    let socket = directory.join("gdb.sock");
    let log = directory.join("qemu.log");
    let _staged = staging::Rootfs::prepare(spec, &socket)?;
    println!("Managed QEMU → GDB: snapshot/nonetwork, CPU paused until continue.");
    println!("QEMU serial/startup log: {}", log.display());
    println!("Private debug socket: {}", socket.display());
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&log)?;
    let mut command = Command::new(&spec.runqemu);
    command
        .args(spec.qemu_arguments(&socket))
        .current_dir(&spec.build_dir)
        .env("BUILDDIR", &spec.build_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .process_group(0);
    let (reader, _lifetime) = watchdog::install(&mut command)?;
    let child = command.spawn().context("Could not start runqemu")?;
    drop(reader);
    let pid = i32::try_from(child.id().context("runqemu has no process identity")?)?;
    let mut qemu = OwnedQemu { child, pid };
    let logs = logging::capture(&mut qemu.child, file);
    let result: Result<()> = async {
        tokio::select! {
            result = wait_ready(&mut qemu.child, &socket, STARTUP_TIMEOUT) => result?,
            _ = terminate.recv() => bail!("Debug startup terminated"),
            _ = hangup.recv() => bail!("Debug terminal closed during startup"),
            _ = interrupt.recv() => bail!("Debug startup cancelled"),
        }
        println!("QEMU socket ready; attaching GDB. Use break/bt/continue; quit stops this guest.");
        let mut debugger = Command::new(&spec.gdb).args(spec.gdb_arguments(&socket))
            .current_dir(&spec.build_dir).kill_on_drop(true).spawn().context("Could not start GDB")?;
        // SIGINT belongs to native GDB (interrupting the guest), not VM termination.
        let result: Result<()> = tokio::select! {
            status = debugger.wait() => {
                let status = status?;
                if !status.success() { bail!("GDB exited with {status}"); }
                Ok(())
            },
            status = qemu.child.wait() => Err(anyhow::anyhow!("QEMU exited while debugging: {}", status?)),
            _ = terminate.recv() => Err(anyhow::anyhow!("Debug session terminated")),
            _ = hangup.recv() => Err(anyhow::anyhow!("Debug terminal closed")),
        };
        let _ = debugger.kill().await;
        result
    }.await;
    qemu.stop().await;
    drop(_lifetime);
    for mut task in logs {
        if timeout(Duration::from_millis(250), &mut task)
            .await
            .is_err()
        {
            task.abort();
            // Readers own no processes; the output cap prevents disk growth.
            tracing::debug!("QEMU log reader did not finish promptly");
        }
    }
    let _ = fs::remove_file(&socket);
    println!(
        "Owned QEMU stopped. Console log retained: {}",
        log.display()
    );
    result.with_context(|| format!("QEMU/GDB session failed; inspect {}", log.display()))
}

async fn verify_gdb(program: &Path) -> Result<()> {
    let output = timeout(
        Duration::from_secs(3),
        Command::new(program)
            .args(["-nx", "-nh", "--version"])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .context("GDB version probe timed out")??;
    let text = String::from_utf8_lossy(&output.stdout);
    let major = text
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|word| word.split('.').next()?.parse::<u32>().ok())
        .next_back();
    if !output.status.success() || major.is_none_or(|major| major < 9) {
        bail!("Managed Unix-socket debugging requires GDB 9 or newer");
    }
    Ok(())
}

pub(super) async fn wait_ready(child: &mut Child, socket: &Path, limit: Duration) -> Result<()> {
    let deadline = Instant::now() + limit;
    loop {
        if let Some(status) = child.try_wait()? {
            bail!("runqemu exited before debugger readiness: {status}");
        }
        match fs::symlink_metadata(socket) {
            Ok(metadata) if metadata.file_type().is_socket() => return Ok(()),
            Ok(_) => bail!("QEMU debug endpoint is not a Unix socket"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        if Instant::now() >= deadline {
            bail!("QEMU debug socket startup timed out");
        }
        sleep(Duration::from_millis(50)).await;
    }
}

fn private_directory() -> Result<PathBuf> {
    let mut bytes = [0_u8; 16];
    fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let nonce = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let directory = PathBuf::from(format!("/tmp/yoctui-qgdb-{nonce}"));
    fs::DirBuilder::new().mode(0o700).create(&directory)?;
    Ok(directory)
}
