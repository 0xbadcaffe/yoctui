use super::*;

#[cfg(unix)]
#[test]
fn reboot_recovery_user_service_can_restart_daemon_after_login_without_root() {
    let unit = daemon_service_unit(Path::new("/opt/yoctui/bin/yoctui")).unwrap();
    assert!(unit.contains("ExecStart=\"/opt/yoctui/bin/yoctui\" daemon foreground"));
    assert!(unit.contains("Restart=on-failure"));
    assert!(unit.contains("WantedBy=default.target"));
    assert!(unit.contains("NoNewPrivileges=true"));
    assert!(!unit.contains("User=root"));
}
