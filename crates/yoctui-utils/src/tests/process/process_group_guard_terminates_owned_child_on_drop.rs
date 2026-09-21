use super::*;

#[test]
fn process_group_guard_terminates_owned_child_on_drop() {
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    let mut child = std::process::Command::new("sleep")
        .arg("30")
        .process_group(0)
        .spawn()
        .unwrap();
    drop(ProcessGroupGuard::new(child.id()));
    assert_eq!(child.wait().unwrap().signal(), Some(libc::SIGKILL));
    drop(ProcessGroupGuard::new(0));
    drop(ProcessGroupGuard::new(u32::MAX));
}
