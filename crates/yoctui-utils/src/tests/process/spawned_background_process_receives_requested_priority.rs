use super::*;

#[test]
fn spawned_background_process_receives_requested_priority() {
    let mut child = std::process::Command::new("sleep")
        .arg("30")
        .spawn()
        .unwrap();
    lower_process_priority(child.id(), 10).unwrap();
    // SAFETY: getpriority reads kernel state for the live child PID.
    let nice = unsafe { libc::getpriority(libc::PRIO_PROCESS, child.id()) };
    let _ = child.kill();
    let _ = child.wait();
    assert_eq!(nice, 10);
}
