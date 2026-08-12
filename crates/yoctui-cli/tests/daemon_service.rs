#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::Path,
    process::Command,
};

#[test]
fn daemon_service_installs_manages_and_removes_user_unit_without_root() {
    let binary = Path::new(env!("CARGO_BIN_EXE_yoctui"));
    let root = std::env::temp_dir().join(format!("yoctui-daemon-service-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let bin = root.join("bin");
    let config = root.join("config");
    fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    fs::create_dir(&bin).unwrap();
    let systemctl = bin.join("systemctl");
    fs::write(
        &systemctl,
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$YOCTUI_SYSTEMCTL_LOG\"\nexit \"${YOCTUI_SYSTEMCTL_EXIT:-0}\"\n",
    )
    .unwrap();
    fs::set_permissions(&systemctl, fs::Permissions::from_mode(0o700)).unwrap();
    let log = root.join("systemctl.log");

    let invoke = |action: &str, exit: Option<&str>| {
        let mut command = Command::new(binary);
        command
            .args(["daemon", "service", action])
            .env("PATH", &bin)
            .env("XDG_CONFIG_HOME", &config)
            .env("YOCTUI_SYSTEMCTL_LOG", &log);
        if let Some(exit) = exit {
            command.env("YOCTUI_SYSTEMCTL_EXIT", exit);
        }
        command.output().unwrap()
    };

    let install = invoke("install", None);
    assert!(
        install.status.success(),
        "{}",
        String::from_utf8_lossy(&install.stderr)
    );
    let unit = config.join("systemd/user/yoctui.service");
    let text = fs::read_to_string(&unit).unwrap();
    assert!(text.contains("ExecStart="));
    assert!(text.contains(" daemon foreground"));
    assert!(text.contains("NoNewPrivileges=true"));
    assert!(!text.contains("sudo"));

    for action in ["start", "status", "restart", "stop"] {
        let output = invoke(action, None);
        assert!(output.status.success(), "{action}");
    }
    let calls = fs::read_to_string(&log).unwrap();
    assert!(calls.contains("--user start yoctui.service"));
    assert!(calls.contains("--user status --no-pager --lines=20 yoctui.service"));

    let unavailable = invoke("status", Some("1"));
    assert!(!unavailable.status.success());
    assert!(String::from_utf8_lossy(&unavailable.stderr).contains("yoctui daemon start"));

    let uninstall = invoke("uninstall", None);
    assert!(uninstall.status.success());
    assert!(!unit.exists());
    fs::remove_dir_all(root).unwrap();
}
