//! Daemon identity.
use super::*;

#[cfg(unix)]
pub(crate) fn daemon_state_root() -> Result<PathBuf> {
    yoctui_utils::state_dir()
        .context("XDG_STATE_HOME or HOME is required for daemon state persistence")
}

#[cfg(unix)]
pub(crate) struct DaemonRuntimeGuard {
    pub(crate) paths: yoctui_protocol::daemon_ipc::RuntimePaths,
    pub(crate) instance: yoctui_protocol::daemon::DaemonInstanceId,
}

#[cfg(unix)]
impl Drop for DaemonRuntimeGuard {
    fn drop(&mut self) {
        let _ =
            yoctui_protocol::daemon_lifecycle::remove_runtime_record(&self.paths, self.instance);
    }
}

#[cfg(unix)]
pub(crate) fn random_instance_id() -> Result<yoctui_protocol::daemon::DaemonInstanceId> {
    let mut bytes = [0_u8; 16];
    fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(yoctui_protocol::daemon::DaemonInstanceId(bytes))
}

#[cfg(unix)]
pub(crate) fn process_memory_bytes() -> Option<u64> {
    let fields = fs::read_to_string("/proc/self/statm").ok()?;
    let resident_pages = fields.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    // sysconf reports the actual page size on hosts with 4, 16 or 64 KiB pages.
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    resident_pages.checked_mul(u64::try_from(page_size).ok().filter(|size| *size > 0)?)
}

#[cfg(unix)]
pub(crate) fn format_instance(instance: yoctui_protocol::daemon::DaemonInstanceId) -> String {
    instance
        .0
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
