#[test]
fn daemon_startup_unready_child_is_terminated_and_reaped() {
    let child = std::process::Command::new("sleep")
        .arg("30")
        .spawn()
        .unwrap();
    let pid = child.id();
    drop(crate::DaemonStartupChild {
        child,
        ready: false,
    });
    // The child was reaped by the guard; waitpid must report ECHILD.
    let result = unsafe { libc::waitpid(pid as i32, std::ptr::null_mut(), libc::WNOHANG) };
    assert_eq!(result, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}
